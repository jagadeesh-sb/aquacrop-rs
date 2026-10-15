//! One simulated day and the step to the next: AquaCrop-OSPy
//! `aquacrop/timestep/`.

#![allow(non_snake_case)]

use crate::crop::Crop;
use crate::init::{adaptive_planting_date, calculate_HIGC, calculate_HI_linear, daily_gdd, f_co2, first_past, WeatherDay};
use crate::numeric::np_cumsum;
use crate::output::FinalStat;
use crate::soil::NO_VALUE;
use crate::solution::{
    biomass_accumulation::biomass_accumulation, canopy_cover::canopy_cover, capillary_rise::capillary_rise,
    check_groundwater_table::check_groundwater_table, drainage::drainage, germination::germination,
    groundwater_inflow::groundwater_inflow, growing_degree_day::growing_degree_day, growth_stage::growth_stage,
    harvest_index::harvest_index, hi_ref_current_day::HIref_current_day, infiltration::infiltration,
    irrigation::irrigation, pre_irrigation::pre_irrigation, rainfall_partition::rainfall_partition,
    root_development::root_development, root_zone_water::root_zone_water, soil_evaporation::soil_evaporation,
    tile_drainage::tile_drainage, transpiration::transpiration,
};
use crate::error::Error;
use crate::state::{Clock, InitCond, Params};

/// The daily tables while the simulation runs (entities/output.py).
#[derive(Clone, Debug)]
pub struct Outputs {
    pub water_flux: Vec<Vec<f64>>,
    pub water_storage: Vec<Vec<f64>>,
    pub crop_growth: Vec<Vec<f64>>,
    pub final_stats: Vec<FinalStat>,
}

impl Outputs {
    pub fn new(n_steps: usize, n_comp: usize) -> Outputs {
        Outputs {
            water_flux: vec![vec![0.0; 17]; n_steps],
            water_storage: vec![vec![0.0; 3 + n_comp]; n_steps],
            crop_growth: vec![vec![0.0; 15]; n_steps],
            final_stats: vec![],
        }
    }
}

fn b(x: bool) -> f64 {
    if x {
        1.0
    } else {
        0.0
    }
}

/// run_single_timestep.py: `solution_single_time_step`.
pub fn solution_single_time_step(cond: &mut InitCond, params: &mut Params, clock: &Clock, weather_step: &WeatherDay, outputs: &mut Outputs) {
    let precipitation = weather_step.precipitation;
    let temp_max = weather_step.max_temp;
    let temp_min = weather_step.min_temp;
    let et0 = weather_step.et0;

    let GroundWater = if params.water_table == 1 { params.z_gw[clock.time_step_counter] } else { 0.0 };

    let growing_season: bool;
    let Crop_Name: String;
    let fallow = clock.season_counter < 0;
    if !fallow {
        let sc = clock.season_counter as usize;
        let CurrentDate = clock.step_start_time;
        let planting_date = clock.planting_dates[sc];
        let harvest_date = clock.harvest_dates[sc];
        growing_season = (planting_date <= CurrentDate) && (harvest_date >= CurrentDate) && !cond.crop_mature && !cond.crop_dead;
        Crop_Name = params.CropChoices[sc].clone();
    } else {
        growing_season = false;
        Crop_Name = "fallow".into();
        params.Fallow_Crop.Aer = 5.0;
        params.Fallow_Crop.Zmin = 0.3;
    }
    // Disjoint borrows of the parameter set for the rest of the day.
    let Params {
        Soil, Profile, IrrMngt, FallowIrrMngt, FieldMngt, FallowFieldMngt, CO2, water_table, Seasonal_Crop_List, Fallow_Crop, ..
    } = params;
    let water_table = *water_table;
    let crop: &Crop = if fallow { Fallow_Crop } else { &Seasonal_Crop_List[clock.season_counter as usize] };
    let irr_mngt = if fallow { &*FallowIrrMngt } else { &*IrrMngt };
    let field = if !fallow && growing_season { &*FieldMngt } else { &*FallowFieldMngt };

    let gdd;
    if growing_season {
        cond.dap += 1.0;
        gdd = growing_degree_day(crop.GDDmethod, crop.Tupp, crop.Tbase, temp_max, temp_min);
        cond.gdd = gdd;
        cond.gdd_cum += gdd;
        cond.growing_season = true;
    } else {
        cond.growing_season = false;
        cond.dap = 0.0;
        gdd = 0.3;
        cond.gdd_cum = 0.0;
    }
    cond.time_step_counter = clock.time_step_counter;
    cond.precipitation = precipitation;
    cond.temp_max = temp_max;
    cond.temp_min = temp_min;
    cond.et0 = et0;

    let (th_fc_Adj, wt_in_soil, z_gw) = check_groundwater_table(Profile, cond.z_gw, &cond.th_fc_Adj, water_table, GroundWater);
    cond.th_fc_Adj = th_fc_Adj;
    cond.wt_in_soil = wt_in_soil;
    cond.z_gw = z_gw;

    let (z_root, r_cor) = root_development(
        crop,
        Profile,
        cond.dap,
        cond.z_root,
        cond.delayed_cds,
        cond.gdd_cum,
        cond.delayed_gdds,
        cond.tr_ratio,
        &cond.th,
        cond.canopy_cover,
        cond.canopy_cover_ns,
        cond.germination,
        cond.r_cor,
        cond.t_pot,
        cond.z_gw,
        gdd,
        growing_season,
        water_table,
    );
    cond.z_root = z_root;
    cond.r_cor = r_cor;

    let PreIrr = pre_irrigation(Profile, crop, cond, growing_season, irr_mngt);

    let (th, mut DeepPerc, FluxOut) = drainage(Profile, &cond.th, &cond.th_fc_Adj);
    cond.replace_th(th);

    let mut TileDrain = 0.0;
    if Soil.tile_drainage {
        let (th, q) = tile_drainage(Profile, &cond.th, Soil.drain_depth.unwrap_or(f64::INFINITY), Soil.drain_target, Soil.drain_coeff);
        cond.replace_th(th);
        TileDrain = q;
    }

    let (mut Runoff, mut Infl, day_submerged) =
        rainfall_partition(precipitation, &cond.th, cond.day_submerged, field, Soil.cn, Soil.adj_cn, Soil.z_cn, Soil.nComp, Profile);
    cond.day_submerged = day_submerged;

    let (depletion, taw, irr_cum, Irr) = irrigation(
        irr_mngt,
        cond.growth_stage,
        cond.irr_cum,
        cond.e_pot,
        cond.t_pot,
        cond.z_root,
        &cond.th,
        cond.dap,
        cond.time_step_counter,
        crop,
        Profile,
        Soil.z_top,
        growing_season,
        precipitation,
        Runoff,
    );
    cond.depletion = depletion;
    cond.taw = taw;
    cond.irr_cum = irr_cum;

    let (th, surface_storage, dp, ro, infl, FluxOut) = infiltration(
        Profile,
        cond.surface_storage,
        &cond.th_fc_Adj,
        &cond.th,
        Infl,
        Irr,
        irr_mngt.AppEff,
        field.bunds,
        field.z_bund,
        FluxOut,
        DeepPerc,
        Runoff,
        growing_season,
    );
    cond.replace_th(th);
    cond.surface_storage = surface_storage;
    DeepPerc = dp;
    Runoff = ro;
    Infl = infl;

    let CR = capillary_rise(Profile, Soil.nLayer, Soil.fshape_cr, cond, &FluxOut, water_table);

    germination(cond, Soil.z_germ, Profile, crop.GermThr, crop.PlantMethod, gdd, growing_season);
    growth_stage(crop, cond, growing_season);
    canopy_cover(crop, Profile, Soil.z_top, cond, gdd, et0, growing_season);

    let (Es, EsPot) = soil_evaporation(
        clock.evap_time_steps,
        clock.sim_off_season,
        clock.time_step_counter,
        Profile,
        Soil,
        crop.CalendarType,
        crop.Senescence,
        irr_mngt.irrigation_method,
        irr_mngt.WetSurf,
        field,
        cond,
        et0,
        Infl,
        precipitation,
        Irr,
        growing_season,
    );

    let (Tr, TrPot_NS, TrPot, mut IrrNet) =
        transpiration(Profile, Soil.nComp, Soil.z_top, crop, irr_mngt.irrigation_method, irr_mngt.NetIrrSMT, cond, et0, CO2, growing_season, gdd);

    let GwIn = groundwater_inflow(Profile, cond);

    let (hi_ref, yield_form, pct_lag_phase) = HIref_current_day(
        cond.hi_ref,
        cond.HIfinal,
        cond.dap,
        cond.delayed_cds,
        cond.yield_form,
        cond.pct_lag_phase,
        cond.canopy_cover,
        cond.cc_prev,
        cond.ccx_w,
        crop,
        growing_season,
    );
    cond.hi_ref = hi_ref;
    cond.yield_form = yield_form;
    cond.pct_lag_phase = pct_lag_phase;

    let (biomass, biomass_ns) = biomass_accumulation(
        crop,
        cond.dap,
        cond.delayed_cds,
        cond.hi_ref,
        cond.pct_lag_phase,
        cond.biomass,
        cond.biomass_ns,
        Tr,
        TrPot_NS,
        et0,
        growing_season,
    );
    cond.biomass = biomass;
    cond.biomass_ns = biomass_ns;

    harvest_index(Profile, Soil.z_top, crop, cond, et0, temp_max, temp_min, growing_season);

    cond.YieldPot = (cond.biomass_ns / 100.0) * cond.harvest_index;
    if growing_season {
        cond.DryYield = (cond.biomass / 100.0) * cond.harvest_index_adj;
        cond.FreshYield = cond.DryYield / (crop.YldWC / 100.0);
        if ((crop.CalendarType == 1.0) && (cond.dap >= crop.Maturity)) || ((crop.CalendarType == 2.0) && (cond.gdd_cum >= crop.Maturity)) {
            cond.crop_mature = true;
        }
    } else {
        cond.DryYield = 0.0;
        cond.FreshYield = 0.0;
    }

    let rz = root_zone_water(Profile, cond.z_root, &cond.th, Soil.z_top, crop.Zmin, crop.Aer);
    let Wr = rz.WrAct;

    IrrNet += PreIrr;
    cond.irr_net_cum += PreIrr;

    let row_day = clock.time_step_counter;
    let (IrrDay, IrrTot);
    if growing_season {
        if irr_mngt.irrigation_method == 4 {
            IrrDay = IrrNet;
            IrrTot = cond.irr_net_cum;
        } else {
            IrrDay = Irr;
            IrrTot = cond.irr_cum;
        }
    } else {
        IrrDay = 0.0;
        IrrTot = 0.0;
        cond.depletion = rz.Dr_Rz;
        cond.taw = rz.TAW_Rz;
    }

    let counter = clock.time_step_counter as f64;
    let sc = clock.season_counter as f64;
    let mut ws = vec![counter, b(growing_season), cond.dap];
    ws.extend_from_slice(&cond.th);
    outputs.water_storage[row_day] = ws;
    outputs.water_flux[row_day] = vec![
        counter, sc, cond.dap, Wr, cond.z_gw, cond.surface_storage, IrrDay, Infl, Runoff, DeepPerc, TileDrain, CR, GwIn, Es,
        EsPot, Tr, TrPot,
    ];
    outputs.crop_growth[row_day] = vec![
        counter,
        sc,
        cond.dap,
        gdd,
        cond.gdd_cum,
        cond.z_root,
        cond.canopy_cover,
        cond.canopy_cover_ns,
        cond.biomass,
        cond.biomass_ns,
        cond.harvest_index,
        cond.harvest_index_adj,
        cond.DryYield,
        cond.FreshYield,
        cond.YieldPot,
    ];

    if clock.season_counter > -1 {
        let s = clock.season_counter as usize;
        if (cond.crop_mature || cond.crop_dead || clock.harvest_dates[s] == clock.step_end_time) && !cond.harvest_flag {
            outputs.final_stats.push(FinalStat {
                season: s,
                crop_type: Crop_Name,
                harvest_date: clock.step_end_time,
                harvest_step: clock.time_step_counter,
                dry_yield: cond.DryYield,
                fresh_yield: cond.FreshYield,
                yield_potential: cond.YieldPot,
                seasonal_irrigation: IrrTot,
            });
            cond.harvest_flag = true;
        }
    }
}

/// check_if_model_is_finished.py
pub fn check_model_is_finished(clock: &Clock, harvest_flag: bool) -> bool {
    let mut finished = clock.step_end_time >= clock.simulation_end_date;
    if harvest_flag && (clock.season_counter == clock.n_seasons as i64 - 1) {
        finished = true;
    }
    finished
}

/// update_time.py. `crop` is the crop as the model was given it.
pub fn update_time(clock: &mut Clock, cond: &mut InitCond, params: &mut Params, weather: &[WeatherDay], crop: &Crop) -> Result<(), Error> {
    if clock.model_is_finished {
        return Ok(());
    }
    let n_seasons = clock.n_seasons as i64;
    if cond.harvest_flag && !clock.sim_off_season {
        if clock.season_counter < n_seasons - 1 {
            clock.season_counter += 1;
            let sc = clock.season_counter as usize;
            let new_pd = adaptive_planting_date(clock.planting_dates[sc], weather, crop, &cond.th);
            let delta = new_pd.0 - clock.planting_dates[sc].0;
            clock.planting_dates[sc] = new_pd;
            clock.harvest_dates[sc] = clock.harvest_dates[sc].add_days(delta);
            let Some(k) = clock.index_of(clock.planting_dates[sc]).filter(|k| k + 1 < clock.n_steps) else {
                // Deviation: AquaCrop-OSPy fails here (the next planting date is not
                // inside the simulation); the simulation ends instead.
                clock.model_is_finished = true;
                return Ok(());
            };
            clock.time_step_counter = k;
            clock.step_start_time = clock.time_span(k);
            clock.step_end_time = clock.time_span(k + 1);
            reset_initial_conditions(clock, cond, params, weather)?;
        }
    } else {
        clock.time_step_counter += 1;
        clock.step_start_time = clock.time_span(clock.time_step_counter);
        clock.step_end_time = clock.time_span(clock.time_step_counter + 1);
        if clock.season_counter < n_seasons - 1 && clock.step_start_time == clock.planting_dates[(clock.season_counter + 1) as usize] {
            clock.season_counter += 1;
            reset_initial_conditions(clock, cond, params, weather)?;
        }
    }
    Ok(())
}

/// reset_initial_conditions.py
pub fn reset_initial_conditions(clock: &Clock, c: &mut InitCond, params: &mut Params, weather: &[WeatherDay]) -> Result<(), Error> {
    let sc = clock.season_counter as usize;
    let n_comp = params.Soil.nComp;
    c.age_days = 0.0;
    c.age_days_ns = 0.0;
    c.aer_days = 0.0;
    c.irr_cum = 0.0;
    c.delayed_gdds = 0.0;
    c.delayed_cds = 0.0;
    c.pct_lag_phase = 0.0;
    c.t_early_sen = 0.0;
    c.gdd_cum = 0.0;
    c.day_submerged = 0.0;
    c.irr_net_cum = 0.0;
    c.dap = 0.0;
    c.aer_days_comp = vec![0.0; n_comp];
    c.pre_adj = false;
    c.crop_mature = false;
    c.crop_dead = false;
    c.germination = false;
    c.premat_senes = false;
    c.harvest_flag = false;
    c.stage = 1.0;
    c.f_pre = 1.0;
    c.f_post = 1.0;
    c.fpost_dwn = 1.0;
    c.fpost_upp = 1.0;
    c.h1_cor_asum = 0.0;
    c.h1_cor_bsum = 0.0;
    c.f_pol = 0.0;
    c.s_cor1 = 0.0;
    c.s_cor2 = 0.0;
    c.growth_stage = 0.0;
    c.tr_ratio = 1.0;
    c.r_cor = 1.0;
    c.canopy_cover = 0.0;
    c.canopy_cover_adj = 0.0;
    c.canopy_cover_ns = 0.0;
    c.canopy_cover_adj_ns = 0.0;
    c.biomass = 0.0;
    c.biomass_ns = 0.0;
    c.harvest_index = 0.0;
    c.harvest_index_adj = 0.0;
    c.ccx_act = 0.0;
    c.ccx_act_ns = 0.0;
    c.ccx_w = 0.0;
    c.ccx_w_ns = 0.0;
    c.ccx_early_sen = 0.0;
    c.cc_prev = 0.0;
    c.protected_seed = false;
    c.sumET0EarlySen = 0.0;
    let crop = &mut params.Seasonal_Crop_List[sc];
    c.HIfinal = crop.HI0;
    c.DryYield = 0.0;
    c.FreshYield = 0.0;

    let co2 = &mut params.CO2;
    let CO2conc = if co2.constant_conc {
        if co2.current_concentration > 0.0 {
            co2.current_concentration
        } else {
            co2.processed[0].1
        }
    } else {
        co2.for_year(clock.step_start_time.year())
    };
    co2.current_concentration = CO2conc;
    crop.fCO2 = f_co2(crop, CO2conc, co2.ref_concentration);

    if !clock.sim_off_season {
        // InitCond.th = InitCond.thini: the two names share one array again.
        c.th = c.thini.clone();
        c.th_is_thini = true;
        let fm = &params.FieldMngt;
        c.surface_storage = if fm.bunds && fm.z_bund > 0.001 { fm.bund_water.min(fm.z_bund) } else { 0.0 };
    }

    if crop.CalendarType == 2.0 {
        let pd = clock.planting_dates[sc];
        let gdd: Vec<f64> = weather.iter().filter(|w| w.date >= pd).map(|w| daily_gdd(crop, w.max_temp, w.min_temp)).collect();
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
        crop.HIGC = calculate_HIGC(crop.YldFormCD, crop.HI0, crop.HIini);
        if crop.CropType == 3.0 {
            let (t, d) = calculate_HI_linear(crop.YldFormCD, crop.HIini, crop.HI0, crop.HIGC);
            crop.tLinSwitch = t;
            crop.dHILinear = d;
        } else {
            crop.tLinSwitch = 0.0;
            crop.dHILinear = 0.0;
        }
    }
    Ok(())
}
