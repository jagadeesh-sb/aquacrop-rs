//! Simulation outputs (entities/output.py and outputs_when_model_is_finished.py).

use crate::date::Date;
use serde::{Deserialize, Serialize};

pub const WATER_FLUX_COLUMNS: [&str; 17] = [
    "time_step_counter", "season_counter", "dap", "Wr", "z_gw", "surface_storage", "IrrDay", "Infl", "Runoff", "DeepPerc",
    "TileDrain", "CR", "GwIn", "Es", "EsPot", "Tr", "TrPot",
];

pub const CROP_GROWTH_COLUMNS: [&str; 15] = [
    "time_step_counter", "season_counter", "dap", "gdd", "gdd_cum", "z_root", "canopy_cover", "canopy_cover_ns", "biomass",
    "biomass_ns", "harvest_index", "harvest_index_adj", "DryYield", "FreshYield", "YieldPot",
];

/// A daily table, one array per column. Days the model skips (between a
/// harvest and the next planting, unless the off-season is simulated) and
/// the last day of the period stay zero, as in AquaCrop-OSPy. NaN (JSON
/// null) marks values AquaCrop-OSPy holds as None, such as the water table
/// depth when there is none.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    #[serde(with = "nan_as_null")]
    pub values: Vec<Vec<f64>>,
}

impl Table {
    pub fn new(columns: Vec<String>, rows: &[Vec<f64>]) -> Table {
        let values = (0..columns.len()).map(|c| rows.iter().map(|r| r[c]).collect()).collect();
        Table { columns, values }
    }

    /// The values of one column.
    pub fn column(&self, name: &str) -> Option<&[f64]> {
        self.columns.iter().position(|c| c == name).map(|i| self.values[i].as_slice())
    }
}

/// One harvested season (`final_stats`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FinalStat {
    pub season: usize,
    pub crop_type: String,
    /// The day after the last simulated day of the season.
    pub harvest_date: Date,
    pub harvest_step: usize,
    /// Dry yield (t/ha).
    pub dry_yield: f64,
    /// Fresh yield (t/ha).
    pub fresh_yield: f64,
    /// Yield without water stress (t/ha).
    pub yield_potential: f64,
    /// Seasonal irrigation (mm).
    pub seasonal_irrigation: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Results {
    /// First date of the daily tables.
    pub start: Date,
    pub water_flux: Table,
    /// Water content (m3/m3) of each soil compartment, `th1`, `th2`, ...
    pub water_storage: Table,
    pub crop_growth: Table,
    pub final_stats: Vec<FinalStat>,
    /// The compartment thicknesses (m) the model used, after it deepened the
    /// profile to fit the roots.
    pub compartments: Vec<f64>,
}

mod nan_as_null {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &[Vec<f64>], s: S) -> Result<S::Ok, S::Error> {
        let o: Vec<Vec<Option<f64>>> =
            v.iter().map(|c| c.iter().map(|x| if x.is_finite() { Some(*x) } else { None }).collect()).collect();
        o.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Vec<f64>>, D::Error> {
        let o: Vec<Vec<Option<f64>>> = Vec::deserialize(d)?;
        Ok(o.into_iter().map(|c| c.into_iter().map(|x| x.unwrap_or(f64::NAN)).collect()).collect())
    }
}
