//! Model initialisation: AquaCrop-OSPy `aquacrop/initialize/` and the
//! `AquaCropModel._initialize` sequence.

#![allow(non_snake_case)]

use crate::config::{Co2Spec, FieldMngt, GroundWaterSpec, InitialWaterContent, IrrMngt, IrrigationSpec, ModelInput, Weather};
use crate::crop::Crop;
use crate::date::{Date, MonthDay};
use crate::error::Error;
use crate::numeric::{np_cumsum, np_interp, np_sum};
use crate::round::{np_round, round0};
use crate::soil::{Soil, SoilProfile, NO_VALUE};
use crate::state::{Clock, Co2State, InitCond, Params};
use serde_json::Value;

/// One day of weather: MinTemp, MaxTemp, Precipitation, ReferenceET (the
/// column order `prepare_weather` produces), and the date.
#[derive(Clone, Copy, Debug)]
pub struct WeatherDay {
    pub min_temp: f64,
    pub max_temp: f64,
    pub precipitation: f64,
    pub et0: f64,
    pub date: Date,
}

/// read_clocks_parameters.py
pub fn read_clock_parameters(sim_start: Date, sim_end: Date, off_season: bool) -> Result<Clock, Error> {
    if sim_end.year() - sim_start.year() > 580 {
        return Err(Error::Input("Simulation period must be less than 580 years.".into()));
    }
    if sim_end.0 - sim_start.0 < 1 {
        return Err(Error::Input("the simulation must span at least two days".into()));
    }
    let n_steps = (sim_end.0 - sim_start.0 + 1) as usize;
    Ok(Clock {
        time_step_counter: 0,
        model_is_finished: false,
        simulation_start_date: sim_start,
        simulation_end_date: sim_end,
        n_steps,
        step_start_time: sim_start,
        step_end_time: sim_start.add_days(1),
        evap_time_steps: 20,
        sim_off_season: off_season,
        planting_dates: vec![],
        harvest_dates: vec![],
        n_seasons: 0,
        season_counter: -1,
    })
}

/// read_weather_inputs.py: the weather rows from the start to the end date.
pub fn read_weather_inputs(clock: &Clock, w: &Weather) -> Result<Vec<WeatherDay>, Error> {
    w.check()?;
    if w.is_empty() {
        return Err(Error::Input("no weather data".into()));
    }
    let last = w.start.add_days(w.len() as i64 - 1);
    if w.start > clock.simulation_start_date {
        return Err(Error::Input(
            "The first date of the climate data cannot be longer than the start date of the model.".into(),
        ));
    }
    if last < clock.simulation_end_date {
        return Err(Error::Input("The model end date cannot be longer than the last date of climate data.".into()));
    }
    let from = (clock.simulation_start_date.0 - w.start.0) as usize;
    Ok((0..clock.n_steps)
        .map(|k| {
            let i = from + k;
            let et0 = if w.clip_et0 { w.reference_et[i].max(0.1) } else { w.reference_et[i] };
            WeatherDay {
                min_temp: w.min_temp[i],
                max_temp: w.max_temp[i],
                precipitation: w.precipitation[i],
                et0,
                date: clock.simulation_start_date.add_days(k as i64),
            }
        })
        .collect())
}

fn date_1990(md: MonthDay) -> Result<Date, Error> {
    Date::from_month_day(1990, md).ok_or_else(|| Error::Input(format!("{md} is not a date in 1990 (a non-leap year)")))
}

fn date_in(year: i64, md: MonthDay) -> Result<Date, Error> {
    Date::from_month_day(year, md).ok_or_else(|| Error::Input(format!("{year}/{md} is not a date")))
}

/// read_model_parameters.py
pub fn read_model_parameters(clock: &mut Clock, soil: &mut Soil, crop: &mut Crop, weather: &[WeatherDay]) -> Result<(), Error> {
    soil.fill_nan();
    while soil.zSoil < crop.Zmax + 0.1 {
        let n = soil.profile.len();
        let Some(i) = (0..n).rev().find(|&i| soil.profile.dz[i] < 0.25) else {
            // Deviation: AquaCrop-OSPy loops forever here.
            return Err(Error::Input(format!(
                "the soil profile ({} m) is shallower than the maximum rooting depth plus 0.1 m and no compartment can grow",
                soil.zSoil
            )));
        };
        soil.profile.dz[i] += 0.1;
        soil.fill_nan();
    }

    if crop.harvest_date.is_none() {
        compute_crop_calendar(crop, &clock.planting_dates, clock.simulation_start_date, clock.simulation_end_date, weather)?;
        let mature = (crop.MaturityCD + 30.0).trunc() as i64;
        let harv = date_1990(crop.planting_date)?.add_days(mature);
        crop.harvest_date = Some(MonthDay { month: harv.month(), day: harv.day() });
    }
    let harvest_date = crop.harvest_date.expect("set above");

    let start = clock.simulation_start_date;
    let end = clock.simulation_end_date;
    let (sy, mut ey) = (start.year(), end.year());
    let single_year = date_1990(crop.planting_date)? < date_1990(harvest_date)?;
    let (mut plant_years, mut harvest_years): (Vec<i64>, Vec<i64>);
    if single_year {
        // A 29 February end date does not exist in 1990; pandas would raise.
        let mock_end = Date::from_ymd(1990, end.month(), end.day())
            .ok_or_else(|| Error::Input("the simulation cannot end on 29 February".into()))?;
        let mock_start = date_1990(crop.planting_date)?;
        if mock_end <= mock_start {
            ey -= 1;
        }
        plant_years = (sy..=ey).collect();
        harvest_years = plant_years.clone();
    } else if date_in(ey + 2, harvest_date)? < end {
        plant_years = (sy..=ey).collect();
        harvest_years = (sy + 1..=ey + 1).collect();
    } else {
        plant_years = (sy..ey).collect();
        harvest_years = (sy + 1..=ey).collect();
    }
    if plant_years.is_empty() {
        return Err(Error::Input("the simulation period holds no complete growing season".into()));
    }
    if date_in(plant_years[0], crop.planting_date)? < start {
        plant_years.remove(0);
        harvest_years.remove(0);
    }
    if plant_years.is_empty() {
        return Err(Error::Input("the simulation period holds no complete growing season".into()));
    }
    clock.planting_dates = plant_years.iter().map(|y| date_in(*y, crop.planting_date)).collect::<Result<_, _>>()?;
    clock.harvest_dates = harvest_years.iter().map(|y| date_in(*y, harvest_date)).collect::<Result<_, _>>()?;
    clock.n_seasons = clock.planting_dates.len();
    clock.season_counter = if clock.step_start_time == clock.planting_dates[0] { 0 } else { -1 };
    Ok(())
}

/// The GDD of one day by the crop's method, as compute_crop_calendar.py and
/// reset_initial_conditions.py compute it over a whole series.
pub fn daily_gdd(crop: &Crop, temp_max: f64, temp_min: f64) -> f64 {
    let clip = |x: f64, lo: f64, hi: f64| if x < lo { lo } else if x > hi { hi } else { x };
    if crop.GDDmethod == 1.0 {
        let Tmean = clip((temp_max + temp_min) / 2.0, crop.Tbase, crop.Tupp);
        Tmean - crop.Tbase
    } else if crop.GDDmethod == 2.0 {
        let tmax = clip(temp_max, crop.Tbase, crop.Tupp);
        let tmin = clip(temp_min, crop.Tbase, crop.Tupp);
        (tmax + tmin) / 2.0 - crop.Tbase
    } else {
        let tmax = clip(temp_max, crop.Tbase, crop.Tupp);
        let tmin = if temp_min > crop.Tupp { crop.Tupp } else { temp_min };
        let Tmean = (tmax + tmin) / 2.0;
        let Tmean = if Tmean < crop.Tbase { crop.Tbase } else { Tmean };
        Tmean - crop.Tbase
    }
}

/// `(gdd_cum > x).idxmax() + 1`: the first day the cumulative GDD passes `x`
/// (1 when it never does, as idxmax of an all-False series is 0).
pub fn first_past(gdd_cum: &[f64], x: f64) -> f64 {
    (gdd_cum.iter().position(|&g| g > x).unwrap_or(0) + 1) as f64
}

/// compute_crop_calendar.py
pub fn compute_crop_calendar(
    crop: &mut Crop,
    planting_dates: &[Date],
    sim_start: Date,
    sim_end: Date,
    weather: &[WeatherDay],
) -> Result<(), Error> {
    let pl_date = match planting_dates.first() {
        Some(d) => *d,
        None => {
            let plant_year = sim_start.year();
            if date_in(plant_year, crop.planting_date)? < sim_start {
                date_in(plant_year + 1, crop.planting_date)?
            } else {
                date_in(plant_year, crop.planting_date)?
            }
        }
    };
    let gdd_from_planting = |crop: &Crop| -> Vec<(Date, f64)> {
        weather
            .iter()
            .filter(|w| w.date >= pl_date && w.date <= sim_end)
            .map(|w| (w.date, daily_gdd(crop, w.max_temp, w.min_temp)))
            .collect()
    };
    let Mode = crop.CalendarType;
    if Mode == 1.0 {
        crop.CanopyDevEndCD =
            if crop.Determinant == 1.0 { round0(crop.HIstartCD + (crop.FloweringCD / 2.0)) } else { crop.SenescenceCD };
        crop.Canopy10PctCD = round0(crop.EmergenceCD + ((0.1 / crop.CC0).ln() / crop.CGC_CD));
        crop.MaxCanopyCD = round0(
            crop.EmergenceCD
                + (((0.25 * crop.CCx * crop.CCx / crop.CC0) / (crop.CCx - (0.98 * crop.CCx))).ln() / crop.CGC_CD),
        );
        crop.HIendCD = crop.HIstartCD + crop.YldFormCD;
        crop.Emergence = crop.EmergenceCD;
        crop.Canopy10Pct = crop.Canopy10PctCD;
        crop.MaxRooting = crop.MaxRootingCD;
        crop.Senescence = crop.SenescenceCD;
        crop.Maturity = crop.MaturityCD;
        crop.MaxCanopy = crop.MaxCanopyCD;
        crop.CanopyDevEnd = crop.CanopyDevEndCD;
        crop.HIstart = crop.HIstartCD;
        crop.HIend = crop.HIendCD;
        crop.YldForm = crop.YldFormCD;
        if crop.CropType == 3.0 {
            crop.FloweringEndCD = crop.HIstartCD + crop.FloweringCD;
        } else {
            crop.FloweringEnd = NO_VALUE;
            crop.FloweringEndCD = NO_VALUE;
            crop.FloweringCD = NO_VALUE;
        }
        if crop.SwitchGDD == 1.0 {
            let gdd = gdd_from_planting(crop);
            prepare_gdd(&gdd, sim_start, sim_end, crop)?;
            crop.CGC = ((((0.98 * crop.CCx) - crop.CCx) * crop.CC0) / (-0.25 * (crop.CCx.powf(2.0)))).ln()
                / (-(crop.MaxCanopy - crop.Emergence));
            let mut tCD = crop.MaturityCD - crop.SenescenceCD;
            if tCD <= 0.0 {
                tCD = 1.0;
            }
            let mut CCi = crop.CCx * (1.0 - 0.05 * ((((3.33 * crop.CDC_CD) / (crop.CCx + 2.29)) * tCD).exp() - 1.0));
            if CCi < 0.0 {
                CCi = 0.0;
            }
            let mut tGDD = crop.Maturity - crop.Senescence;
            if tGDD <= 0.0 {
                tGDD = 5.0;
            }
            crop.CDC = ((crop.CCx + 2.29) * ((((CCi / crop.CCx) - 1.0) / -0.05) + 1.0).ln()) / (3.33 * tGDD);
            crop.CalendarType = 2.0;
        } else {
            crop.CDC = crop.CDC_CD;
            crop.CGC = crop.CGC_CD;
        }
    } else if Mode == 2.0 {
        crop.CanopyDevEnd =
            if crop.Determinant == 1.0 { round0(crop.HIstart + (crop.Flowering / 2.0)) } else { crop.Senescence };
        crop.Canopy10Pct = round0(crop.Emergence + ((0.1 / crop.CC0).ln() / crop.CGC));
        crop.MaxCanopy = round0(
            crop.Emergence + (((0.25 * crop.CCx * crop.CCx / crop.CC0) / (crop.CCx - (0.98 * crop.CCx))).ln() / crop.CGC),
        );
        crop.HIend = crop.HIstart + crop.YldForm;
        if crop.CropType == 3.0 {
            crop.FloweringEnd = crop.HIstart + crop.Flowering;
        }
        let gdd: Vec<f64> = gdd_from_planting(crop).iter().map(|(_, g)| *g).collect();
        let gdd_cum = np_cumsum(&gdd);
        let last = gdd_cum.last().copied().unwrap_or(0.0);
        if !(last > crop.Maturity) {
            return Err(Error::Model(format!(
                "not enough growing degree days in simulation ({last}) to reach maturity ({})",
                crop.Maturity
            )));
        }
        crop.MaturityCD = first_past(&gdd_cum, crop.Maturity);
        if !(crop.MaturityCD < 365.0) {
            return Err(Error::Model("crop will take longer than 1 year to mature".into()));
        }
        crop.MaxCanopyCD = first_past(&gdd_cum, crop.MaxCanopy);
        crop.CanopyDevEndCD = first_past(&gdd_cum, crop.CanopyDevEnd);
        crop.HIstartCD = first_past(&gdd_cum, crop.HIstart);
        crop.HIendCD = first_past(&gdd_cum, crop.HIend);
        crop.YldFormCD = crop.HIendCD - crop.HIstartCD;
        if crop.CropType == 3.0 {
            let FloweringEnd = first_past(&gdd_cum, crop.FloweringEnd);
            crop.FloweringCD = FloweringEnd - crop.HIstartCD;
        } else {
            crop.FloweringCD = NO_VALUE;
        }
    }
    Ok(())
}

/// utils/prepare_gdd.py: the GDD at each calendar-day stage, averaged over seasons.
fn prepare_gdd(gdd: &[(Date, f64)], sim_start: Date, sim_end: Date, crop: &mut Crop) -> Result<(), Error> {
    let mut season: Vec<Option<usize>> = vec![None; gdd.len()];
    let mut season_counter = 1usize;
    let mut current_year = sim_start.year();
    let mut first_planting_date = date_in(current_year, crop.planting_date)?;
    if first_planting_date < sim_start {
        current_year += 1;
        first_planting_date = date_in(current_year, crop.planting_date)?;
    }
    while first_planting_date <= sim_end {
        let next_planting_date = date_in(current_year + 1, crop.planting_date)?;
        let mut season_end_date = next_planting_date.add_days(-1);
        if season_end_date > sim_end {
            season_end_date = sim_end;
        }
        for (i, (d, _)) in gdd.iter().enumerate() {
            if *d >= first_planting_date && *d <= season_end_date {
                season[i] = Some(season_counter);
            }
        }
        season_counter += 1;
        current_year += 1;
        first_planting_date = next_planting_date;
    }
    // weather_df['season'].unique(), in order of appearance.
    let mut seasons: Vec<Option<usize>> = vec![];
    for s in &season {
        if !seasons.contains(s) {
            seasons.push(*s);
        }
    }
    let crop_type3 = crop.CropType == 3.0;
    let cd = [
        crop.EmergenceCD,
        crop.Canopy10PctCD,
        crop.MaxRootingCD,
        crop.MaxCanopyCD,
        crop.CanopyDevEndCD,
        crop.SenescenceCD,
        crop.MaturityCD,
        crop.HIstartCD,
        crop.HIendCD,
    ];
    let mut lists: Vec<Vec<f64>> = vec![vec![]; cd.len()];
    let mut flowering_end: Vec<f64> = vec![];
    for s in &seasons {
        if s.is_none() {
            return Err(Error::Input("the weather before the first planting date has no season".into()));
        }
        let g: Vec<f64> = gdd.iter().zip(&season).filter(|(_, ss)| *ss == s).map(|((_, g), _)| *g).collect();
        let gdd_cum = np_cumsum(&g);
        let at = |k: f64| -> Result<f64, Error> {
            gdd_cum.get(k.trunc() as usize).copied().ok_or_else(|| {
                Error::Input("a season in the simulation is shorter than the crop's calendar-day maturity".into())
            })
        };
        for (j, c) in cd.iter().enumerate() {
            lists[j].push(at(*c)?);
        }
        if crop_type3 {
            flowering_end.push(at(crop.FloweringEndCD)?);
        }
    }
    let median = crop.SwitchGDDType == "median";
    let summarise = |v: &[f64]| -> f64 {
        if median {
            let mut s = v.to_vec();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = s.len();
            if n % 2 == 1 {
                s[n / 2]
            } else {
                np_sum(&[s[n / 2 - 1], s[n / 2]]) / 2.0
            }
        } else {
            np_sum(v) / v.len() as f64
        }
    };
    let vals: Vec<f64> = lists.iter().map(|l| summarise(l)).collect();
    crop.Emergence = vals[0];
    crop.Canopy10Pct = vals[1];
    crop.MaxRooting = vals[2];
    crop.MaxCanopy = vals[3];
    crop.CanopyDevEnd = vals[4];
    crop.Senescence = vals[5];
    crop.Maturity = vals[6];
    crop.HIstart = vals[7];
    crop.HIend = vals[8];
    // prepare_gdd also sets 'YieldFormation' and 'FloweringDuration', new
    // attributes nothing reads; only FloweringEnd matters.
    if crop_type3 {
        crop.FloweringEnd = summarise(&flowering_end);
    }
    Ok(())
}

/// calculate_HIGC.py
pub fn calculate_HIGC(crop_YldFormCD: f64, crop_HI0: f64, crop_HIini: f64) -> f64 {
    let tHI = crop_YldFormCD;
    let mut HIGC = 0.001;
    let mut HIest = 0.0;
    while HIest <= (0.98 * crop_HI0) {
        HIGC += 0.001;
        HIest = (crop_HIini * crop_HI0) / (crop_HIini + (crop_HI0 - crop_HIini) * (-HIGC * tHI).exp());
    }
    if HIest >= crop_HI0 {
        HIGC -= 0.001;
    }
    HIGC
}

/// calculate_HI_linear.py
pub fn calculate_HI_linear(crop_YldFormCD: f64, crop_HIini: f64, crop_HI0: f64, crop_HIGC: f64) -> (f64, f64) {
    let mut ti = 0.0;
    let tmax = crop_YldFormCD;
    let mut HIest = 0.0;
    let mut HIprev = crop_HIini;
    while (HIest <= crop_HI0) && (ti < tmax) {
        ti += 1.0;
        let HInew = (crop_HIini * crop_HI0) / (crop_HIini + (crop_HI0 - crop_HIini) * (-crop_HIGC * ti).exp());
        HIest = HInew + (tmax - ti) * (HInew - HIprev);
        HIprev = HInew;
    }
    let tSwitch = ti - 1.0;
    if tSwitch > 0.0 {
        HIest = (crop_HIini * crop_HI0) / (crop_HIini + (crop_HI0 - crop_HIini) * (-crop_HIGC * tSwitch).exp());
    } else {
        HIest = 0.0;
    }
    let dHILin = (crop_HI0 - HIest) / (tmax - tSwitch);
    (tSwitch, dHILin)
}

const MAUNA_LOA: &str = include_str!("../data/co2/MaunaLoaCO2.txt");

/// The (year, ppm) series of a CO2 setting.
pub fn co2_series(spec: &Co2Spec) -> Result<Vec<(f64, f64)>, Error> {
    if let Some(d) = &spec.co2_data {
        return Ok(d.clone());
    }
    let text = match spec.scenario.as_deref() {
        None => MAUNA_LOA,
        Some(s) => match s.to_lowercase().replace(['-', '_', '.'], "").as_str() {
            "ssp119" => include_str!("../data/co2/CO2_SSP1-1.9.txt"),
            "ssp126" => include_str!("../data/co2/CO2_SSP1-2.6.txt"),
            "ssp245" => include_str!("../data/co2/CO2_SSP2-4.5.txt"),
            "ssp370" => include_str!("../data/co2/CO2_SSP3-7.0.txt"),
            "ssp434" => include_str!("../data/co2/CO2_SSP4-3.4.txt"),
            "ssp460" => include_str!("../data/co2/CO2_SSP4-6.0.txt"),
            "ssp534os" => include_str!("../data/co2/CO2_SSP5-3.4-OS.txt"),
            "ssp585" => include_str!("../data/co2/CO2_SSP5-8.5.txt"),
            _ => {
                return Err(Error::Input(format!(
                    "Unknown CO2 scenario '{s}'. Options: ssp119, ssp126, ssp245, ssp370, ssp434, ssp460, ssp534os, ssp585"
                )))
            }
        },
    };
    // pd.read_csv(header=1, sep=r"\s+"): skip the comment and the header line.
    Ok(text
        .lines()
        .skip(2)
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
        })
        .collect())
}

/// The CO2 effect on water productivity. compute_variables.py and
/// reset_initial_conditions.py compute the same value in two spellings.
pub fn f_co2(crop: &Crop, CO2conc: f64, CO2ref: f64) -> f64 {
    let fw = if CO2conc <= CO2ref {
        0.0
    } else if CO2conc >= 550.0 {
        1.0
    } else {
        1.0 - ((550.0 - CO2conc) / (550.0 - CO2ref))
    };
    let fCO2old = (CO2conc / CO2ref)
        / (1.0
            + (CO2conc - CO2ref)
                * ((1.0 - fw) * crop.bsted + fw * ((crop.bsted * crop.fsink) + (crop.bface * (1.0 - crop.fsink)))));
    let mut fCO2new = f64::NAN;
    if CO2conc > CO2ref {
        let fshape = -4.61824 - 3.43831 * crop.fsink - 5.32587 * crop.fsink * crop.fsink;
        if CO2conc >= 2000.0 {
            fCO2new = 1.58;
        } else {
            let CO2rel = (CO2conc - CO2ref) / (2000.0 - CO2ref);
            fCO2new = 1.0 + 0.58 * (((CO2rel * fshape).exp() - 1.0) / (fshape.exp() - 1.0));
        }
    }
    let fCO2 = if CO2conc <= CO2ref {
        fCO2old
    } else if (CO2conc <= 550.0) && (fCO2old < fCO2new) {
        fCO2old
    } else {
        fCO2new
    };
    let ftype = if crop.WP >= 40.0 {
        0.0
    } else if crop.WP <= 20.0 {
        1.0
    } else {
        (40.0 - crop.WP) / (40.0 - 20.0)
    };
    1.0 + ftype * (fCO2 - 1.0)
}

/// compute_variables.py
pub fn compute_variables(
    soil: &mut Soil,
    crop: &mut Crop,
    water_table: i32,
    co2: &Co2Spec,
    clock: &Clock,
    weather: &[WeatherDay],
) -> Result<Co2State, Error> {
    if water_table == 1 {
        soil.add_capillary_rise_params();
    }
    if soil.adj_rew == 0.0 {
        soil.rew = np_round(1000.0 * (soil.profile.th_fc[0] - soil.profile.th_dry[0]) * soil.evap_z_surf, 2);
    }
    if soil.calc_cn == 1.0 {
        let ksat = soil.profile.Ksat[0];
        if ksat > 864.0 {
            soil.cn = 46.0;
        } else if ksat > 347.0 {
            soil.cn = 61.0;
        } else if ksat > 36.0 {
            soil.cn = 72.0;
        } else if ksat > 0.0 {
            soil.cn = 77.0;
        }
        if !(ksat > 0.0) {
            return Err(Error::Input("the top soil layer's Ksat must be positive to derive a curve number".into()));
        }
    }
    compute_crop_calendar(crop, &clock.planting_dates, clock.simulation_start_date, clock.simulation_end_date, weather)?;
    crop.HIGC = calculate_HIGC(crop.YldFormCD, crop.HI0, crop.HIini);
    if crop.CropType == 3.0 {
        let (t, d) = calculate_HI_linear(crop.YldFormCD, crop.HIini, crop.HI0, crop.HIGC);
        crop.tLinSwitch = t;
        crop.dHILinear = d;
    } else {
        crop.tLinSwitch = 0.0;
        crop.dHILinear = 0.0;
    }

    let data = co2_series(co2)?;
    if data.is_empty() {
        return Err(Error::Input("the CO2 series is empty".into()));
    }
    let xp: Vec<f64> = data.iter().map(|d| d.0).collect();
    let fp: Vec<f64> = data.iter().map(|d| d.1).collect();
    let (y0, y1) = (clock.simulation_start_date.year(), clock.simulation_end_date.year());
    let processed: Vec<(i64, f64)> = (y0..=y1).map(|y| (y, np_interp(y as f64, &xp, &fp))).collect();
    let mut CO2conc = processed[0].1;
    if co2.constant_conc {
        CO2conc = if co2.current_concentration > 0.0 { co2.current_concentration } else { processed[0].1 };
    }
    let state = Co2State {
        ref_concentration: co2.ref_concentration,
        current_concentration: CO2conc,
        constant_conc: co2.constant_conc,
        processed,
    };
    crop.fCO2 = f_co2(crop, CO2conc, co2.ref_concentration);
    Ok(state)
}

/// read_irrigation_management.py, with IrrigationManagement's defaults.
pub fn read_irrigation_management(spec: &IrrigationSpec, clock: &Clock) -> Result<IrrMngt, Error> {
    let m = spec.irrigation_method;
    if !(0..=5).contains(&m) {
        return Err(Error::Input(format!("irrigation_method must be 0 to 5, got {m}")));
    }
    let mut schedule = vec![0.0; clock.n_steps];
    if m == 3 {
        for e in &spec.Schedule {
            if let Some(i) = clock.index_of(e.date) {
                schedule[i] = e.depth;
            }
        }
    }
    let smt = match &spec.SMT {
        Some(v) => {
            if v.len() != 4 {
                return Err(Error::Input("SMT needs four soil-moisture targets, one per growth stage".into()));
            }
            [v[0], v[1], v[2], v[3]]
        }
        None if m == 1 => [100.0; 4],
        None => [0.0; 4],
    };
    let interval = spec.IrrInterval.unwrap_or(if m == 2 { 3.0 } else { 0.0 });
    if m == 2 && !(interval >= 1.0) {
        return Err(Error::Input("IrrInterval must be at least one day".into()));
    }
    Ok(IrrMngt {
        irrigation_method: m,
        WetSurf: spec.WetSurf,
        AppEff: spec.AppEff,
        MaxIrr: spec.MaxIrr,
        MaxIrrSeason: spec.MaxIrrSeason,
        SMT: smt,
        IrrInterval: interval,
        Schedule: schedule,
        NetIrrSMT: spec.NetIrrSMT,
        depth: spec.depth,
    })
}

/// read_groundwater_table.py: (water_table, depth for each day).
pub fn read_groundwater_table(gw: &GroundWaterSpec, clock: &Clock) -> Result<(i32, Vec<f64>), Error> {
    let n = clock.n_steps;
    if !gw.water_table {
        return Ok((0, vec![999.0; n]));
    }
    if gw.dates.is_empty() || gw.dates.len() != gw.values.len() {
        return Err(Error::Input("a water table needs matching dates and depths".into()));
    }
    let z = if gw.dates.len() == 1 {
        vec![gw.values[0]; n]
    } else if gw.method == "Constant" {
        let mut z = vec![f64::NAN; n];
        for (row, (date, depth)) in gw.dates.iter().zip(&gw.values).enumerate() {
            for (i, zi) in z.iter_mut().enumerate() {
                let d = clock.time_span(i);
                if d >= *date || (row == 0 && d <= *date) {
                    *zi = *depth;
                }
            }
        }
        z
    } else if gw.method == "Variable" {
        let mut z = vec![f64::NAN; n];
        for (date, depth) in gw.dates.iter().zip(&gw.values) {
            // Deviation: pandas appends a date outside the simulation as an extra row; it is rejected here.
            let i = clock
                .index_of(*date)
                .ok_or_else(|| Error::Input(format!("water table date {date} is outside the simulation period")))?;
            z[i] = *depth;
        }
        // Series.interpolate() is np.interp over row positions: linear between
        // known values, the last value carried forward, leading gaps left empty.
        let known: Vec<usize> = (0..n).filter(|&i| !z[i].is_nan()).collect();
        let xp: Vec<f64> = known.iter().map(|&i| i as f64).collect();
        let fp: Vec<f64> = known.iter().map(|&i| z[i]).collect();
        if let Some(&first) = known.first() {
            for i in first..n {
                if z[i].is_nan() {
                    z[i] = np_interp(i as f64, &xp, &fp);
                }
            }
        }
        z
    } else {
        return Err(Error::Input(format!("water table method must be Constant or Variable, got {:?}", gw.method)));
    };
    Ok((1, z))
}

/// The water-table adjusted field capacity of each compartment
/// (read_model_initial_conditions.py; check_groundwater_table.py repeats it).
pub fn th_fc_adjusted(th_fc: &[f64], th_s: &[f64], zMid: &[f64], z_gw: f64) -> Vec<f64> {
    let n = th_fc.len();
    let mut thfcAdj = vec![0.0; n];
    let mut compi = n as i64 - 1;
    while compi >= 0 {
        let c = compi as usize;
        let Xmax = if th_fc[c] <= 0.1 {
            1.0
        } else if th_fc[c] >= 0.3 {
            2.0
        } else {
            let pF = 2.0 + 0.3 * (th_fc[c] - 0.1) / 0.2;
            (pF * 10f64.ln()).exp() / 100.0
        };
        if (z_gw < 0.0) || ((z_gw - zMid[c]) >= Xmax) {
            thfcAdj[..=c].copy_from_slice(&th_fc[..=c]);
            compi = -1;
        } else {
            if th_fc[c] >= th_s[c] {
                thfcAdj[c] = th_fc[c];
            } else if zMid[c] >= z_gw {
                thfcAdj[c] = th_s[c];
            } else {
                let dV = th_s[c] - th_fc[c];
                let dFC = (dV / Xmax.powf(2.0)) * ((zMid[c] - (z_gw - Xmax)).powf(2.0));
                thfcAdj[c] = th_fc[c] + dFC;
            }
            compi -= 1;
        }
    }
    thfcAdj
}

/// What read_model_initial_conditions needs from the parameters.
pub struct InitParams {
    pub seasonal_zmin: f64,
    pub seasonal_cc0: f64,
    pub hi0: f64,
    pub field: FieldMngt,
    pub fallow_field: FieldMngt,
    pub water_table: i32,
    pub z_gw: Vec<f64>,
}

/// read_model_initial_conditions.py
pub fn read_model_initial_conditions(
    soil: &mut Soil,
    clock: &Clock,
    iwc: &InitialWaterContent,
    params: &InitParams,
) -> Result<InitCond, Error> {
    let n = soil.profile.len();
    let mut ic = InitCond::new(n);
    if clock.season_counter == 0 {
        ic.z_root = params.seasonal_zmin;
        ic.cc0_adj = params.seasonal_cc0;
    }
    ic.HIfinal = params.hi0;
    let fm = if clock.season_counter == -1 { &params.fallow_field } else { &params.field };
    ic.surface_storage = if fm.bunds && fm.z_bund > 0.001 {
        if fm.bund_water > fm.z_bund {
            fm.z_bund
        } else {
            fm.bund_water
        }
    } else {
        0.0
    };

    if params.water_table == 0 {
        ic.z_gw = NO_VALUE;
        ic.wt_in_soil = false;
        ic.th_fc_Adj = soil.profile.th_fc.clone();
    } else {
        let p = &soil.profile;
        ic.z_gw = params.z_gw[clock.time_step_counter];
        ic.wt_in_soil = ic.z_gw >= 0.0 && p.zMid.iter().any(|&z| z >= ic.z_gw);
        ic.th_fc_Adj = th_fc_adjusted(&p.th_fc, &p.th_s, &p.zMid, ic.z_gw).iter().map(|v| np_round(*v, 3)).collect();
    }
    soil.profile.th_fc_Adj = ic.th_fc_Adj.iter().map(|v| np_round(*v, 3)).collect();

    let typestr = iwc.wc_type.as_str();
    let methodstr = iwc.method.as_str();
    if !matches!(methodstr, "Layer" | "Depth") {
        return Err(Error::Input(format!("initial water content method must be Layer or Depth, got {methodstr:?}")));
    }
    if iwc.depth_layer.len() != iwc.value.len() || iwc.value.is_empty() {
        return Err(Error::Input("initial water content needs one value per depth or layer".into()));
    }
    let soil_ref: &Soil = soil;
    let p = &soil_ref.profile;
    // Soil.Hydrology: the profile averaged per layer.
    let hyd = |layer: f64, col: &str| -> f64 {
        let c = match col {
            "th_wp" => &p.th_wp,
            "th_fc" => &p.th_fc,
            _ => &p.th_s,
        };
        soil_ref.layer_mean(layer, c)
    };
    let layer_at_depth = |depth: f64| -> f64 {
        let last = *p.dzsum.last().expect("non-empty profile");
        if depth < last {
            p.layer[p.dzsum.iter().position(|&d| depth < d).expect("depth above the bottom")]
        } else {
            *p.layer.last().expect("non-empty profile")
        }
    };
    let layers = soil_ref.layers();
    let layer_ok = |layer: f64| -> Result<f64, Error> {
        if layers.contains(&layer) {
            Ok(layer)
        } else {
            Err(Error::Input(format!("initial water content refers to soil layer {layer}, which does not exist")))
        }
    };
    let as_num = |v: &Value| -> Result<f64, Error> {
        v.as_f64().ok_or_else(|| Error::Input(format!("initial water content value {v} must be a number")))
    };
    let depth_layer = iwc.depth_layer.clone();
    let mut values = vec![0.0; iwc.value.len()];
    match typestr {
        "Num" => {
            for (i, v) in iwc.value.iter().enumerate() {
                values[i] = as_num(v)?;
            }
        }
        "Pct" => {
            for ii in 0..values.len() {
                let value = as_num(&iwc.value[ii])?;
                let layer = if methodstr == "Depth" { layer_at_depth(depth_layer[ii]) } else { layer_ok(depth_layer[ii])? };
                let (wp, fc) = (hyd(layer, "th_wp"), hyd(layer, "th_fc"));
                values[ii] = wp + ((value / 100.0) * (fc - wp));
            }
        }
        "Prop" => {
            for ii in 0..values.len() {
                let value = match &iwc.value[ii] {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                let layer = if methodstr == "Depth" { layer_at_depth(depth_layer[ii]) } else { layer_ok(depth_layer[ii])? };
                values[ii] = match value.as_str() {
                    "SAT" => hyd(layer, "th_s"),
                    "FC" => hyd(layer, "th_fc"),
                    "WP" => hyd(layer, "th_wp"),
                    // numpy keeps the zero the array started with.
                    _ => 0.0,
                };
            }
        }
        other => return Err(Error::Input(format!("initial water content type must be Prop, Num or Pct, got {other:?}"))),
    }

    let mut thini = vec![0.0; n];
    if methodstr == "Layer" {
        for ii in 0..values.len() {
            let layer = depth_layer[ii].trunc();
            for i in 0..n {
                if p.layer[i] == layer {
                    thini[i] = values[ii];
                }
            }
        }
    } else {
        let mut depths = depth_layer.clone();
        let mut vals = values.clone();
        if depths[0] > 0.0 {
            depths.insert(0, 0.0);
            vals.insert(0, vals[0]);
        }
        if *depths.last().unwrap() < soil_ref.zSoil {
            depths.push(soil_ref.zSoil);
            vals.push(*vals.last().unwrap());
        }
        for i in 0..n {
            let top = if i == 0 { 0.0 } else { p.dzsum[i - 1] };
            let mid = (top + p.dzsum[i]) / 2.0;
            thini[i] = np_interp(mid, &depths, &vals);
        }
    }
    let mut th = thini;
    if params.water_table == 1 && typestr == "Prop" && matches!(iwc.value.last(), Some(Value::String(s)) if s == "FC") {
        th = ic.th_fc_Adj.clone();
    }
    if ic.wt_in_soil {
        let mids: Vec<f64> = (0..n).map(|i| ((if i == 0 { 0.0 } else { p.dzsum[i - 1] }) + p.dzsum[i]) / 2.0).collect();
        if let Some(idx) = mids.iter().position(|&m| m >= ic.z_gw) {
            for ii in idx..n {
                th[ii] = hyd(p.layer[ii], "th_s");
            }
        }
    }
    // InitCond.thini = InitCond.th: one array under two names.
    ic.thini = th.clone();
    ic.th = th;
    ic.th_is_thini = true;
    Ok(ic)
}

/// create_soil_profile.py
pub fn create_soil_profile(soil: &Soil, water_table: i32) -> SoilProfile {
    let p = &soil.profile;
    let n = p.len();
    SoilProfile {
        Comp: (0..n).map(|i| p.comp[i] as usize).collect(),
        dz: p.dz.clone(),
        Layer: p.layer.iter().map(|l| *l as i64).collect(),
        dzsum: p.dzsum.clone(),
        th_fc: p.th_fc.clone(),
        th_s: p.th_s.clone(),
        th_wp: p.th_wp.clone(),
        Ksat: p.Ksat.clone(),
        Penetrability: p.penetrability.clone(),
        th_dry: p.th_dry.clone(),
        tau: p.tau.clone(),
        zBot: p.zBot.clone(),
        z_top: p.z_top.clone(),
        zMid: p.zMid.clone(),
        th_fc_Adj: p.th_fc_Adj.clone(),
        aCR: if water_table == 1 { p.aCR.clone() } else { vec![0.0; n] },
        bCR: if water_table == 1 { p.bCR.clone() } else { vec![0.0; n] },
    }
}

/// adaptive_planting.py: the first day on or after `nominal` that meets the
/// crop's planting triggers.
pub fn adaptive_planting_date(nominal: Date, weather: &[WeatherDay], crop: &Crop, th: &[f64]) -> Date {
    if !crop.PlantTrigger {
        return nominal;
    }
    let start = weather.iter().position(|w| w.date >= nominal).unwrap_or(0);
    let max_delay = crop.PlantMaxDelay.trunc() as usize;
    let last = (start + max_delay).min(weather.len() - 1);
    let t_win = crop.PlantTWindow.trunc() as i64;
    let rain_win = crop.PlantRainWindow.map(|v| v.trunc() as i64).unwrap_or(t_win);
    let tmean: Vec<f64> = weather.iter().map(|w| (w.min_temp + w.max_temp) / 2.0).collect();
    for i in start..=last {
        let mut ok = true;
        if let Some(t_thr) = crop.PlantTempThr {
            let lo = i as i64 - t_win + 1;
            if lo < start as i64 || tmean[lo as usize..=i].iter().any(|&t| t < t_thr) {
                ok = false;
            }
        }
        if ok {
            if let Some(rain_thr) = crop.PlantRainThr {
                let lo = (start as i64).max(i as i64 - rain_win + 1) as usize;
                let rain: Vec<f64> = weather[lo..=i].iter().map(|w| w.precipitation).collect();
                if np_sum(&rain) < rain_thr {
                    ok = false;
                }
            }
        }
        if ok {
            if let Some(sm_thr) = crop.PlantSMThr {
                if th[0] < sm_thr {
                    ok = false;
                }
            }
        }
        if ok {
            return weather[i].date;
        }
    }
    weather[last].date
}

/// `AquaCropModel._initialize`: (clock, parameters, state, weather, the crop as configured).
pub fn initialize(input: &ModelInput) -> Result<(Clock, Params, InitCond, Vec<WeatherDay>, Crop), Error> {
    let mut clock = read_clock_parameters(input.sim_start, input.sim_end, input.off_season)?;
    let weather = read_weather_inputs(&clock, &input.weather)?;
    let mut soil = Soil::new(&input.soil)?;
    let c = &input.crop;
    let mut crop = Crop::new(&c.name, c.planting_date, c.harvest_date, &c.params)?;
    let missing = crop.missing_parameters();
    if !missing.is_empty() {
        return Err(Error::Input(format!("the custom crop needs: {}", missing.join(", "))));
    }
    if input.soil.tile_drainage && input.soil.drain_depth.is_none() {
        return Err(Error::Input("tile drainage needs a drain depth".into()));
    }
    read_model_parameters(&mut clock, &mut soil, &mut crop, &weather)?;
    let irr = read_irrigation_management(&input.irrigation, &clock)?;
    let field = FieldMngt::from(&input.field_management);
    let fallow_field = FieldMngt::from(&input.fallow_field_management);
    let (water_table, z_gw) = read_groundwater_table(&input.groundwater, &clock)?;
    let co2 = compute_variables(&mut soil, &mut crop, water_table, &input.co2, &clock, &weather)?;
    let seasonal: Vec<Crop> = (0..clock.n_seasons).map(|_| crop.clone()).collect();
    let init_params = InitParams {
        seasonal_zmin: seasonal[0].Zmin,
        seasonal_cc0: seasonal[0].CC0,
        hi0: crop.HI0,
        field: field.clone(),
        fallow_field: fallow_field.clone(),
        water_table,
        z_gw: z_gw.clone(),
    };
    let ic = read_model_initial_conditions(&mut soil, &clock, &input.initial_water_content, &init_params)?;
    let profile = create_soil_profile(&soil, water_table);
    clock.planting_dates[0] = adaptive_planting_date(clock.planting_dates[0], &weather, &crop, &ic.th);
    let params = Params {
        Profile: profile,
        IrrMngt: irr,
        FallowIrrMngt: IrrMngt::fallow(clock.n_steps),
        FieldMngt: field,
        FallowFieldMngt: fallow_field,
        CO2: co2,
        water_table,
        z_gw,
        CropChoices: vec![crop.Name.clone(); clock.n_seasons],
        Seasonal_Crop_List: seasonal,
        Fallow_Crop: crop.clone(),
        Soil: soil,
    };
    Ok((clock, params, ic, weather, crop))
}
