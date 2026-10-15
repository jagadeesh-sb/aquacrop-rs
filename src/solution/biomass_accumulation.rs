//! solution/biomass_accumulation.py

#![allow(non_snake_case)]

use crate::crop::Crop;

/// Above-ground biomass with and without water stress (g/m2).
pub fn biomass_accumulation(
    Crop: &Crop,
    NewCond_DAP: f64,
    NewCond_DelayedCDs: f64,
    NewCond_HIref: f64,
    NewCond_PctLagPhase: f64,
    NewCond_B: f64,
    NewCond_B_NS: f64,
    Tr: f64,
    TrPot: f64,
    et0: f64,
    growing_season: bool,
) -> (f64, f64) {
    let mut NewCond_B = NewCond_B;
    let mut NewCond_B_NS = NewCond_B_NS;
    if growing_season {
        let HIt = NewCond_DAP - NewCond_DelayedCDs - Crop.HIstartCD - 1.0;
        let mut WPadj;
        if ((Crop.CropType == 2.0) || (Crop.CropType == 3.0)) && (NewCond_HIref > 0.0) {
            let fswitch = if Crop.Determinant == 1.0 {
                NewCond_PctLagPhase / 100.0
            } else if HIt < (Crop.YldFormCD / 3.0) {
                HIt / (Crop.YldFormCD / 3.0)
            } else {
                1.0
            };
            WPadj = Crop.WP * (1.0 - (1.0 - Crop.WPy / 100.0) * fswitch);
        } else {
            WPadj = Crop.WP;
        }
        WPadj = WPadj * Crop.fCO2;
        let dB_NS = WPadj * (TrPot / et0);
        let mut dB = WPadj * (Tr / et0);
        if dB.is_nan() {
            dB = 0.0;
        }
        NewCond_B = NewCond_B + dB;
        NewCond_B_NS = NewCond_B_NS + dB_NS;
    } else {
        NewCond_B = 0.0;
        NewCond_B_NS = 0.0;
    }
    (NewCond_B, NewCond_B_NS)
}
