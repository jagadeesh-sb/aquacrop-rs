//! Model inputs, mirroring the arguments of `AquaCropModel(...)` and of the
//! entity classes it takes. Field names follow AquaCrop-OSPy.

#![allow(non_snake_case)]

use crate::date::{Date, MonthDay};
use crate::error::Error;
use crate::soil::SoilSpec;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Everything one simulation needs.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelInput {
    /// First simulated day.
    pub sim_start: Date,
    /// Last day of the simulation period.
    pub sim_end: Date,
    pub weather: Weather,
    pub soil: SoilSpec,
    pub crop: CropSpec,
    #[serde(default)]
    pub initial_water_content: InitialWaterContent,
    #[serde(default)]
    pub irrigation: IrrigationSpec,
    #[serde(default)]
    pub field_management: FieldMngtSpec,
    #[serde(default)]
    pub fallow_field_management: FieldMngtSpec,
    #[serde(default)]
    pub groundwater: GroundWaterSpec,
    #[serde(default)]
    pub co2: Co2Spec,
    /// Simulate the days between harvest and the next planting.
    #[serde(default)]
    pub off_season: bool,
}

/// Daily weather from `start`, one value per day in each series.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Weather {
    pub start: Date,
    /// Minimum air temperature (degC).
    pub min_temp: Vec<f64>,
    /// Maximum air temperature (degC).
    pub max_temp: Vec<f64>,
    /// Precipitation (mm).
    pub precipitation: Vec<f64>,
    /// Reference evapotranspiration ET0 (mm). `prepare_weather` raises values
    /// below 0.1 to 0.1; supply them already clipped, or set `clip_et0`.
    pub reference_et: Vec<f64>,
    /// Raise ET0 below 0.1 mm to 0.1, as AquaCrop-OSPy's `prepare_weather` does.
    #[serde(default)]
    pub clip_et0: bool,
}

impl Weather {
    pub fn len(&self) -> usize {
        self.min_temp.len()
    }

    pub fn is_empty(&self) -> bool {
        self.min_temp.is_empty()
    }

    pub fn check(&self) -> Result<(), Error> {
        let n = self.min_temp.len();
        if self.max_temp.len() != n || self.precipitation.len() != n || self.reference_et.len() != n {
            return Err(Error::Input("weather series must all have the same length".into()));
        }
        let all = self.min_temp.iter().chain(&self.max_temp).chain(&self.precipitation).chain(&self.reference_et);
        if all.clone().any(|v| !v.is_finite()) {
            return Err(Error::Input("weather contains a missing or non-numeric value".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CropSpec {
    /// A built-in crop name (see [`crate::crop::crop_names`]) or "custom".
    pub name: String,
    /// Planting date, MM/DD.
    pub planting_date: MonthDay,
    /// Latest harvest date, MM/DD; derived from maturity when absent.
    #[serde(default)]
    pub harvest_date: Option<MonthDay>,
    /// Parameter overrides, by AquaCrop-OSPy name (e.g. "CCx", "WP", "Maturity").
    #[serde(default)]
    pub params: Map<String, Value>,
}

/// entities/inititalWaterContent.py
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InitialWaterContent {
    /// "Prop" (values "WP", "FC", "SAT"), "Num" (m3/m3) or "Pct" (% of TAW).
    pub wc_type: String,
    /// "Layer" (one value per soil layer) or "Depth" (interpolated between depths).
    pub method: String,
    pub depth_layer: Vec<f64>,
    pub value: Vec<Value>,
}

impl Default for InitialWaterContent {
    fn default() -> Self {
        InitialWaterContent {
            wc_type: "Prop".into(),
            method: "Layer".into(),
            depth_layer: vec![1.0],
            value: vec![Value::String("FC".into())],
        }
    }
}

/// One scheduled irrigation (method 3).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct IrrigationEvent {
    pub date: Date,
    /// Depth (mm).
    pub depth: f64,
}

/// entities/irrigationManagement.py `IrrigationManagement`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IrrigationSpec {
    /// 0 rainfed, 1 soil-moisture targets, 2 fixed interval, 3 schedule, 4 net irrigation, 5 constant depth.
    pub irrigation_method: i32,
    pub WetSurf: f64,
    pub AppEff: f64,
    pub MaxIrr: f64,
    pub MaxIrrSeason: f64,
    /// Soil-moisture targets (% TAW) for the four growth stages; default 100 for method 1.
    pub SMT: Option<Vec<f64>>,
    /// Days between irrigations; default 3 for method 2.
    pub IrrInterval: Option<f64>,
    pub NetIrrSMT: f64,
    pub depth: f64,
    pub Schedule: Vec<IrrigationEvent>,
}

impl Default for IrrigationSpec {
    fn default() -> Self {
        IrrigationSpec {
            irrigation_method: 0,
            WetSurf: 100.0,
            AppEff: 100.0,
            MaxIrr: 25.0,
            MaxIrrSeason: 10_000.0,
            SMT: None,
            IrrInterval: None,
            NetIrrSMT: 80.0,
            depth: 0.0,
            Schedule: vec![],
        }
    }
}

/// IrrMngtStruct: the irrigation settings the daily solution reads.
#[derive(Clone, Debug)]
pub struct IrrMngt {
    pub irrigation_method: i32,
    pub WetSurf: f64,
    pub AppEff: f64,
    pub MaxIrr: f64,
    pub MaxIrrSeason: f64,
    pub SMT: [f64; 4],
    pub IrrInterval: f64,
    /// Depth (mm) for every simulated day (method 3).
    pub Schedule: Vec<f64>,
    pub NetIrrSMT: f64,
    pub depth: f64,
}

impl IrrMngt {
    /// `IrrMngtStruct(sim_len)`: rainfed defaults.
    pub fn fallow(sim_len: usize) -> IrrMngt {
        IrrMngt {
            irrigation_method: 0,
            WetSurf: 100.0,
            AppEff: 100.0,
            MaxIrr: 25.0,
            MaxIrrSeason: 10_000.0,
            SMT: [0.0; 4],
            IrrInterval: 0.0,
            Schedule: vec![0.0; sim_len],
            NetIrrSMT: 80.0,
            depth: 0.0,
        }
    }
}

/// entities/fieldManagement.py `FieldMngt`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FieldMngtSpec {
    pub mulches: bool,
    pub bunds: bool,
    pub curve_number_adj: bool,
    pub sr_inhb: bool,
    pub mulch_pct: f64,
    pub f_mulch: f64,
    /// Bund height (m).
    pub z_bund: f64,
    /// Initial water held by the bunds (mm).
    pub bund_water: f64,
    pub curve_number_adj_pct: f64,
}

impl Default for FieldMngtSpec {
    fn default() -> Self {
        FieldMngtSpec {
            mulches: false,
            bunds: false,
            curve_number_adj: false,
            sr_inhb: false,
            mulch_pct: 50.0,
            f_mulch: 0.5,
            z_bund: 0.0,
            bund_water: 0.0,
            curve_number_adj_pct: 0.0,
        }
    }
}

/// FieldMngtStruct, bund height in mm.
#[derive(Clone, Debug)]
pub struct FieldMngt {
    pub mulches: bool,
    pub bunds: bool,
    pub curve_number_adj: bool,
    pub sr_inhb: bool,
    pub mulch_pct: f64,
    pub f_mulch: f64,
    pub z_bund: f64,
    pub bund_water: f64,
    pub curve_number_adj_pct: f64,
}

impl From<&FieldMngtSpec> for FieldMngt {
    fn from(s: &FieldMngtSpec) -> Self {
        FieldMngt {
            mulches: s.mulches,
            bunds: s.bunds,
            curve_number_adj: s.curve_number_adj,
            sr_inhb: s.sr_inhb,
            mulch_pct: s.mulch_pct,
            f_mulch: s.f_mulch,
            z_bund: s.z_bund * 1000.0,
            bund_water: s.bund_water,
            curve_number_adj_pct: s.curve_number_adj_pct,
        }
    }
}

/// entities/groundWater.py
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroundWaterSpec {
    pub water_table: bool,
    /// "Constant" (steps) or "Variable" (interpolated).
    pub method: String,
    pub dates: Vec<Date>,
    /// Water table depths (m).
    pub values: Vec<f64>,
}

impl Default for GroundWaterSpec {
    fn default() -> Self {
        GroundWaterSpec { water_table: false, method: "Constant".into(), dates: vec![], values: vec![] }
    }
}

/// entities/co2.py
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Co2Spec {
    pub ref_concentration: f64,
    pub current_concentration: f64,
    pub constant_conc: bool,
    /// A built-in series: "ssp119", "ssp126", "ssp245", "ssp370", "ssp434",
    /// "ssp460", "ssp534os", "ssp585"; Mauna Loa observations when absent.
    pub scenario: Option<String>,
    /// Your own (year, ppm) series, used instead of the built-in ones.
    pub co2_data: Option<Vec<(f64, f64)>>,
}

impl Default for Co2Spec {
    fn default() -> Self {
        Co2Spec { ref_concentration: 369.41, current_concentration: 0.0, constant_conc: false, scenario: None, co2_data: None }
    }
}
