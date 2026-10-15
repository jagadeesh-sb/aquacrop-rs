//! solution/growth_stage.py

#![allow(non_snake_case)]

use crate::crop::Crop;
use crate::state::InitCond;

/// Current growth stage (1 to 4, or 0 outside the growing season).
pub fn growth_stage(Crop: &Crop, cond: &mut InitCond, growing_season: bool) {
    let NewCond = cond;
    if growing_season {
        // Deviation: a CalendarType other than 1 or 2 leaves tAdj unbound in Python; read as 2 here.
        let tAdj = if Crop.CalendarType == 1.0 {
            NewCond.dap - NewCond.delayed_cds
        } else {
            NewCond.gdd_cum - NewCond.delayed_gdds
        };
        if tAdj <= Crop.Canopy10Pct {
            NewCond.growth_stage = 1.0;
        } else if tAdj <= Crop.MaxCanopy {
            NewCond.growth_stage = 2.0;
        } else if tAdj <= Crop.Senescence {
            NewCond.growth_stage = 3.0;
        } else if tAdj > Crop.Senescence {
            NewCond.growth_stage = 4.0;
        }
    } else {
        NewCond.growth_stage = 0.0;
    }
}
