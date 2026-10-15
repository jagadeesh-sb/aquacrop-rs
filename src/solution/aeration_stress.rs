//! solution/aeration_stress.py

#![allow(non_snake_case)]

use crate::state::RootZone;

/// Aeration stress coefficient and the updated count of waterlogged days.
pub fn aeration_stress(NewCond_AerDays: f64, Crop_LagAer: f64, thRZ: &RootZone) -> (f64, f64) {
    let mut NewCond_AerDays = NewCond_AerDays;
    let Ksa_Aer;
    if thRZ.thRZ_Act > thRZ.thRZ_Aer {
        if NewCond_AerDays < Crop_LagAer {
            let stress = 1.0 - ((thRZ.thRZ_S - thRZ.thRZ_Act) / (thRZ.thRZ_S - thRZ.thRZ_Aer));
            Ksa_Aer = 1.0 - ((NewCond_AerDays / 3.0) * stress);
        } else {
            // elif NewCond_AerDays >= Crop_LagAer: the only other case for numbers.
            Ksa_Aer = (thRZ.thRZ_S - thRZ.thRZ_Act) / (thRZ.thRZ_S - thRZ.thRZ_Aer);
        }
        NewCond_AerDays = NewCond_AerDays + 1.0;
        if NewCond_AerDays > Crop_LagAer {
            NewCond_AerDays = Crop_LagAer;
        }
    } else {
        Ksa_Aer = 1.0;
        NewCond_AerDays = 0.0;
    }
    (Ksa_Aer, NewCond_AerDays)
}
