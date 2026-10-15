//! solution/HIref_current_day.py

#![allow(non_snake_case)]

use crate::crop::Crop;

/// Reference harvest index for the day.
///
/// Returns (HIref, YieldForm, PctLagPhase). As in AquaCrop-OSPy, a change to
/// `HIfinal` made here stays local to the function.
pub fn HIref_current_day(
    NewCond_HIref: f64,
    NewCond_HIfinal: f64,
    NewCond_DAP: f64,
    NewCond_DelayedCDs: f64,
    NewCond_YieldForm: bool,
    NewCond_PctLagPhase: f64,
    NewCond_CC: f64,
    NewCond_CC_prev: f64,
    NewCond_CCxW: f64,
    Crop: &Crop,
    growing_season: bool,
) -> (f64, bool, f64) {
    let _ = NewCond_CC_prev;
    let mut NewCond_HIref = NewCond_HIref;
    let mut NewCond_HIfinal = NewCond_HIfinal;
    let mut NewCond_YieldForm = NewCond_YieldForm;
    let mut NewCond_PctLagPhase = NewCond_PctLagPhase;
    if growing_season {
        let tAdj = NewCond_DAP - NewCond_DelayedCDs;
        NewCond_YieldForm = tAdj > Crop.HIstartCD;
        let HIt = NewCond_DAP - NewCond_DelayedCDs - Crop.HIstartCD - 1.0;
        if HIt <= 0.0 {
            NewCond_HIref = 0.0;
            NewCond_PctLagPhase = 0.0;
        } else {
            if (Crop.CropType == 1.0) || (Crop.CropType == 2.0) {
                NewCond_PctLagPhase = 100.0;
                NewCond_HIref =
                    (Crop.HIini * Crop.HI0) / (Crop.HIini + (Crop.HI0 - Crop.HIini) * (-Crop.HIGC * HIt).exp());
                if NewCond_HIref >= (0.9799 * Crop.HI0) {
                    NewCond_HIref = Crop.HI0;
                }
            } else if Crop.CropType == 3.0 {
                if HIt < Crop.tLinSwitch {
                    NewCond_PctLagPhase = 100.0 * (HIt / Crop.tLinSwitch);
                    NewCond_HIref =
                        (Crop.HIini * Crop.HI0) / (Crop.HIini + (Crop.HI0 - Crop.HIini) * (-Crop.HIGC * HIt).exp());
                } else {
                    NewCond_PctLagPhase = 100.0;
                    NewCond_HIref = (Crop.HIini * Crop.HI0)
                        / (Crop.HIini + (Crop.HI0 - Crop.HIini) * (-Crop.HIGC * Crop.tLinSwitch).exp());
                    NewCond_HIref = NewCond_HIref + (Crop.dHILinear * (HIt - Crop.tLinSwitch));
                }
            }
            if NewCond_HIref > Crop.HI0 {
                NewCond_HIref = Crop.HI0;
            } else if NewCond_HIref <= (Crop.HIini + 0.004) {
                NewCond_HIref = 0.0;
            } else if (Crop.HI0 - NewCond_HIref) < 0.004 {
                NewCond_HIref = Crop.HI0;
            }
            if (NewCond_HIfinal == Crop.HI0)
                && (HIt <= Crop.YldFormCD)
                && (NewCond_CC <= 0.05)
                && (NewCond_CCxW > 0.0)
                && (NewCond_CC < NewCond_CCxW)
                && (Crop.CropType == 2.0 || Crop.CropType == 3.0)
            {
                NewCond_HIfinal = NewCond_HIref;
            }
            if NewCond_HIref > NewCond_HIfinal {
                NewCond_HIref = NewCond_HIfinal;
            }
        }
    } else {
        NewCond_HIref = 0.0;
    }
    (NewCond_HIref, NewCond_YieldForm, NewCond_PctLagPhase)
}
