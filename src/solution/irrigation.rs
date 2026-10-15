//! solution/irrigation.py

#![allow(non_snake_case)]

use crate::config::IrrMngt;
use crate::crop::Crop;
use crate::soil::SoilProfile;
use crate::solution::root_zone_water::root_zone_water;

/// Irrigation applied on the day by the chosen strategy.
/// Returns `(Depletion, TAW, IrrCum, Irr)`.
#[allow(clippy::too_many_arguments)]
pub fn irrigation(
    irr: &IrrMngt,
    NewCond_GrowthStage: f64,
    NewCond_IrrCum: f64,
    NewCond_Epot: f64,
    NewCond_Tpot: f64,
    NewCond_Zroot: f64,
    NewCond_th: &[f64],
    NewCond_DAP: f64,
    NewCond_TimeStepCounter: usize,
    crop: &Crop,
    prof: &SoilProfile,
    Soil_zTop: f64,
    growing_season: bool,
    Rain: f64,
    Runoff: f64,
) -> (f64, f64, f64, f64) {
    let IrrMngt_IrrMethod = irr.irrigation_method;
    let mut NewCond_GrowthStage = NewCond_GrowthStage;
    let mut NewCond_IrrCum = NewCond_IrrCum;
    let NewCond_Depletion;
    let NewCond_TAW;
    let mut Irr;
    if growing_season {
        let rz = root_zone_water(prof, NewCond_Zroot, NewCond_th, Soil_zTop, crop.Zmin, crop.Aer);
        let Dr = rz.Dr_Rz;
        let taw = rz.TAW_Rz;
        let AbvFc = if rz.thRZ_Act > rz.thRZ_FC {
            let rootdepth = NewCond_Zroot.max(crop.Zmin);
            (rz.thRZ_Act - rz.thRZ_FC) * 1000.0 * rootdepth
        } else {
            0.0
        };
        let WCadj = NewCond_Tpot + NewCond_Epot - Rain + Runoff - AbvFc;
        NewCond_Depletion = Dr + WCadj;
        NewCond_TAW = taw;
        if NewCond_DAP == 1.0 {
            NewCond_GrowthStage = 1.0;
        }
        if IrrMngt_IrrMethod == 0 {
            Irr = 0.0;
        } else if IrrMngt_IrrMethod == 1 {
            let Dr = NewCond_Depletion / NewCond_TAW;
            // Python's negative index: stage 0 reads SMT[-1], the last target.
            let index = NewCond_GrowthStage as i64 - 1;
            let index = index.rem_euclid(4) as usize;
            if Dr > 1.0 - irr.SMT[index] / 100.0 {
                let mut IrrReq = NewCond_Depletion.max(0.0);
                let EffAdj = ((100.0 - irr.AppEff) + 100.0) / 100.0;
                IrrReq *= EffAdj;
                Irr = irr.MaxIrr.min(IrrReq);
            } else {
                Irr = 0.0;
            }
        } else if IrrMngt_IrrMethod == 2 {
            let Dr = NewCond_Depletion;
            let nDays = NewCond_DAP - 1.0;
            // A zero interval divides by zero in Python; NaN here, so no irrigation.
            if nDays.rem_euclid(irr.IrrInterval) == 0.0 {
                let mut IrrReq = Dr.max(0.0);
                let EffAdj = ((100.0 - irr.AppEff) + 100.0) / 100.0;
                IrrReq *= EffAdj;
                Irr = irr.MaxIrr.min(IrrReq);
            } else {
                Irr = 0.0;
            }
        } else if IrrMngt_IrrMethod == 3 {
            let idx = NewCond_TimeStepCounter;
            Irr = irr.Schedule[idx];
            assert!(Irr >= 0.0, "scheduled irrigation must not be negative");
            Irr = irr.MaxIrr.min(Irr);
        } else if IrrMngt_IrrMethod == 4 {
            Irr = 0.0;
        } else if IrrMngt_IrrMethod == 5 {
            Irr = irr.MaxIrr.min(irr.depth);
        } else {
            // Deviation: Python leaves Irr unset (NameError) for other methods.
            Irr = 0.0;
        }
        // Python's max(0, Irr) gives 0 for NaN, as f64::max does.
        Irr = 0f64.max(Irr);
    } else {
        Irr = 0.0;
        NewCond_IrrCum = 0.0;
        NewCond_Depletion = 0.0;
        NewCond_TAW = 0.0;
    }
    if NewCond_IrrCum + Irr > irr.MaxIrrSeason {
        Irr = 0f64.max(irr.MaxIrrSeason - NewCond_IrrCum);
    }
    NewCond_IrrCum += Irr;
    (NewCond_Depletion, NewCond_TAW, NewCond_IrrCum, Irr)
}
