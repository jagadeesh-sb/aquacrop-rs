//! solution/temperature_stress.py

#![allow(non_snake_case)]

use crate::crop::Crop;
use crate::state::Kst;

/// Heat and cold stress coefficients on pollination.
pub fn temperature_stress(Crop: &Crop, temp_max: f64, temp_min: f64) -> Kst {
    let KsPol_up = 1.0;
    let KsPol_lo = 0.001;
    // Deviation: a stress flag other than 0 or 1 leaves the coefficient unbound
    // in Python (UnboundLocalError); here it means no stress (1).
    let mut Kst_PolH = 1.0;
    if Crop.PolHeatStress == 0.0 {
        Kst_PolH = 1.0;
    } else if Crop.PolHeatStress == 1.0 {
        if temp_max <= Crop.Tmax_lo {
            Kst_PolH = 1.0;
        } else if temp_max >= Crop.Tmax_up {
            Kst_PolH = 0.0;
        } else {
            let Trel = (temp_max - Crop.Tmax_lo) / (Crop.Tmax_up - Crop.Tmax_lo);
            Kst_PolH =
                (KsPol_up * KsPol_lo) / (KsPol_lo + (KsPol_up - KsPol_lo) * (-Crop.fshape_b * (1.0 - Trel)).exp());
        }
    }
    let mut Kst_PolC = 1.0;
    if Crop.PolColdStress == 0.0 {
        Kst_PolC = 1.0;
    } else if Crop.PolColdStress == 1.0 {
        if temp_min >= Crop.Tmin_up {
            Kst_PolC = 1.0;
        } else if temp_min <= Crop.Tmin_lo {
            Kst_PolC = 0.0;
        } else {
            let Trel = (Crop.Tmin_up - temp_min) / (Crop.Tmin_up - Crop.Tmin_lo);
            Kst_PolC =
                (KsPol_up * KsPol_lo) / (KsPol_lo + (KsPol_up - KsPol_lo) * (-Crop.fshape_b * (1.0 - Trel)).exp());
        }
    }
    Kst { PolH: Kst_PolH, PolC: Kst_PolC }
}
