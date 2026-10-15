//! Model state: ClockStruct, ParamStruct and InitialCondition of AquaCrop-OSPy.

#![allow(non_snake_case)]

use crate::config::{FieldMngt, IrrMngt};
use crate::crop::Crop;
use crate::date::Date;
use crate::soil::{Soil, SoilProfile, NO_VALUE};

/// entities/clockStruct.py
#[derive(Clone, Debug)]
pub struct Clock {
    pub time_step_counter: usize,
    pub model_is_finished: bool,
    pub simulation_start_date: Date,
    pub simulation_end_date: Date,
    pub n_steps: usize,
    pub step_start_time: Date,
    pub step_end_time: Date,
    pub evap_time_steps: usize,
    pub sim_off_season: bool,
    pub planting_dates: Vec<Date>,
    pub harvest_dates: Vec<Date>,
    pub n_seasons: usize,
    pub season_counter: i64,
}

impl Clock {
    /// `time_span[i]`.
    pub fn time_span(&self, i: usize) -> Date {
        self.simulation_start_date.add_days(i as i64)
    }

    /// `time_span.get_loc(date)`.
    pub fn index_of(&self, date: Date) -> Option<usize> {
        let i = date.0 - self.simulation_start_date.0;
        if i >= 0 && (i as usize) < self.n_steps {
            Some(i as usize)
        } else {
            None
        }
    }
}

/// entities/co2.py after compute_variables.
#[derive(Clone, Debug)]
pub struct Co2State {
    pub ref_concentration: f64,
    pub current_concentration: f64,
    pub constant_conc: bool,
    /// (year, ppm) interpolated for every simulated year.
    pub processed: Vec<(i64, f64)>,
}

impl Co2State {
    pub fn for_year(&self, year: i64) -> f64 {
        self.processed.iter().find(|(y, _)| *y == year).map(|(_, c)| *c).unwrap_or(f64::NAN)
    }
}

/// entities/paramStruct.py
#[derive(Clone, Debug)]
pub struct Params {
    pub Soil: Soil,
    pub Profile: SoilProfile,
    pub IrrMngt: IrrMngt,
    pub FallowIrrMngt: IrrMngt,
    pub FieldMngt: FieldMngt,
    pub FallowFieldMngt: FieldMngt,
    pub CO2: Co2State,
    pub water_table: i32,
    /// Water table depth (m) for each simulated day.
    pub z_gw: Vec<f64>,
    pub CropChoices: Vec<String>,
    pub Seasonal_Crop_List: Vec<Crop>,
    pub Fallow_Crop: Crop,
}

/// entities/initParamVariables.py: the state carried from day to day.
///
/// Integers in Python are held as f64 here; every one stays far below 2^53.
#[derive(Clone, Debug)]
pub struct InitCond {
    pub age_days: f64,
    pub age_days_ns: f64,
    pub aer_days: f64,
    pub aer_days_comp: Vec<f64>,
    pub irr_cum: f64,
    pub delayed_gdds: f64,
    pub delayed_cds: f64,
    pub pct_lag_phase: f64,
    pub t_early_sen: f64,
    pub gdd_cum: f64,
    pub day_submerged: f64,
    pub irr_net_cum: f64,
    pub dap: f64,
    pub e_pot: f64,
    pub t_pot: f64,
    pub pre_adj: bool,
    pub crop_mature: bool,
    pub crop_dead: bool,
    pub germination: bool,
    pub premat_senes: bool,
    pub harvest_flag: bool,
    pub growing_season: bool,
    pub yield_form: bool,
    pub stage2: bool,
    pub wt_in_soil: bool,
    pub stage: f64,
    pub f_pre: f64,
    pub f_post: f64,
    pub fpost_dwn: f64,
    pub fpost_upp: f64,
    pub h1_cor_asum: f64,
    pub h1_cor_bsum: f64,
    pub f_pol: f64,
    pub s_cor1: f64,
    pub s_cor2: f64,
    pub hi_ref: f64,
    pub HIfinal: f64,
    pub growth_stage: f64,
    pub tr_ratio: f64,
    pub r_cor: f64,
    pub canopy_cover: f64,
    pub canopy_cover_adj: f64,
    pub canopy_cover_ns: f64,
    pub canopy_cover_adj_ns: f64,
    pub biomass: f64,
    pub biomass_ns: f64,
    pub YieldPot: f64,
    pub harvest_index: f64,
    pub harvest_index_adj: f64,
    pub ccx_act: f64,
    pub ccx_act_ns: f64,
    pub ccx_w: f64,
    pub ccx_w_ns: f64,
    pub ccx_early_sen: f64,
    pub cc_prev: f64,
    pub protected_seed: bool,
    pub DryYield: f64,
    pub FreshYield: f64,
    pub z_root: f64,
    pub cc0_adj: f64,
    pub surface_storage: f64,
    /// Water table depth (m); NaN where AquaCrop-OSPy holds None (no water table).
    pub z_gw: f64,
    pub th_fc_Adj: Vec<f64>,
    pub th: Vec<f64>,
    pub thini: Vec<f64>,
    /// True while `th` and `thini` are the same numpy array in AquaCrop-OSPy
    /// (from the start of a season until the first day's drainage replaces
    /// `th`). A write into `th` then changes `thini` too, and so the soil water
    /// the next season starts from. See [`InitCond::set_th`].
    pub th_is_thini: bool,
    pub time_step_counter: usize,
    pub precipitation: f64,
    pub temp_max: f64,
    pub temp_min: f64,
    pub et0: f64,
    pub sumET0EarlySen: f64,
    pub gdd: f64,
    pub w_surf: f64,
    pub evap_z: f64,
    pub w_stage_2: f64,
    pub depletion: f64,
    pub taw: f64,
}

impl InitCond {
    pub fn new(num_comp: usize) -> InitCond {
        InitCond {
            age_days: 0.0,
            age_days_ns: 0.0,
            aer_days: 0.0,
            aer_days_comp: vec![0.0; num_comp],
            irr_cum: 0.0,
            delayed_gdds: 0.0,
            delayed_cds: 0.0,
            pct_lag_phase: 0.0,
            t_early_sen: 0.0,
            gdd_cum: 0.0,
            day_submerged: 0.0,
            irr_net_cum: 0.0,
            dap: 0.0,
            e_pot: 0.0,
            t_pot: 0.0,
            pre_adj: false,
            crop_mature: false,
            crop_dead: false,
            germination: false,
            premat_senes: false,
            harvest_flag: false,
            growing_season: false,
            yield_form: false,
            stage2: false,
            wt_in_soil: false,
            stage: 1.0,
            f_pre: 1.0,
            f_post: 1.0,
            fpost_dwn: 1.0,
            fpost_upp: 1.0,
            h1_cor_asum: 0.0,
            h1_cor_bsum: 0.0,
            f_pol: 0.0,
            s_cor1: 0.0,
            s_cor2: 0.0,
            hi_ref: 0.0,
            HIfinal: 0.0,
            growth_stage: 0.0,
            tr_ratio: 1.0,
            r_cor: 1.0,
            canopy_cover: 0.0,
            canopy_cover_adj: 0.0,
            canopy_cover_ns: 0.0,
            canopy_cover_adj_ns: 0.0,
            biomass: 0.0,
            biomass_ns: 0.0,
            YieldPot: 0.0,
            harvest_index: 0.0,
            harvest_index_adj: 0.0,
            ccx_act: 0.0,
            ccx_act_ns: 0.0,
            ccx_w: 0.0,
            ccx_w_ns: 0.0,
            ccx_early_sen: 0.0,
            cc_prev: 0.0,
            protected_seed: false,
            DryYield: 0.0,
            FreshYield: 0.0,
            z_root: 0.0,
            cc0_adj: 0.0,
            surface_storage: 0.0,
            z_gw: NO_VALUE,
            th_fc_Adj: vec![0.0; num_comp],
            th: vec![0.0; num_comp],
            thini: vec![0.0; num_comp],
            th_is_thini: false,
            time_step_counter: 0,
            precipitation: 0.0,
            temp_max: 0.0,
            temp_min: 0.0,
            et0: 0.0,
            sumET0EarlySen: 0.0,
            gdd: 0.0,
            w_surf: 0.0,
            evap_z: 0.0,
            w_stage_2: 0.0,
            depletion: 0.0,
            taw: 0.0,
        }
    }

    /// `NewCond.th[i] = v`, carried into `thini` while the two share memory.
    pub fn set_th(&mut self, i: usize, v: f64) {
        self.th[i] = v;
        if self.th_is_thini {
            self.thini[i] = v;
        }
    }

    /// `NewCond.th = new_array`: `th` no longer shares memory with `thini`.
    pub fn replace_th(&mut self, th: Vec<f64>) {
        self.th = th;
        self.th_is_thini = false;
    }
}

/// Root-zone water quantities returned by `root_zone_water`.
#[derive(Clone, Copy, Debug, Default)]
pub struct RootZone {
    pub WrAct: f64,
    pub Dr_Zt: f64,
    pub Dr_Rz: f64,
    pub TAW_Zt: f64,
    pub TAW_Rz: f64,
    pub thRZ_Act: f64,
    pub thRZ_S: f64,
    pub thRZ_FC: f64,
    pub thRZ_WP: f64,
    pub thRZ_Dry: f64,
    pub thRZ_Aer: f64,
}

/// Water stress coefficients (entities/waterStressCoefficients.py).
#[derive(Clone, Copy, Debug)]
pub struct Ksw {
    pub exp: f64,
    pub sto: f64,
    pub sen: f64,
    pub pol: f64,
    pub sto_lin: f64,
}

/// Temperature stress coefficients (entities/temperatureStressCoefficients.py).
#[derive(Clone, Copy, Debug)]
pub struct Kst {
    pub PolH: f64,
    pub PolC: f64,
}
