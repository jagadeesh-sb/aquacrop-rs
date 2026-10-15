//! solution/harvest_index.py, with its helpers HIadj_pre_anthesis.py,
//! HIadj_pollination.py and HIadj_post_anthesis.py.

#![allow(non_snake_case)]

use crate::crop::Crop;
use crate::soil::SoilProfile;
use crate::solution::root_zone_water::root_zone_water;
use crate::solution::temperature_stress::temperature_stress;
use crate::solution::water_stress::water_stress;
use crate::state::{InitCond, Kst, Ksw};
use std::f64::consts::PI;

/// solution/HIadj_pre_anthesis.py: harvest index adjustment for water stress
/// before flowering.
pub fn HIadj_pre_anthesis(NewCond_B: f64, NewCond_B_NS: f64, NewCond_CC: f64, Crop_dHI_pre: f64) -> f64 {
    let mut NewCond_Fpre;
    if Crop_dHI_pre > 0.0 {
        let Br = NewCond_B / NewCond_B_NS;
        let Br_range = Crop_dHI_pre.ln() / 5.62;
        let Br_upp = 1.0;
        let Br_low = 1.0 - Br_range;
        let Br_top = Br_upp - (Br_range / 3.0);
        let ratio_low = (Br - Br_low) / (Br_top - Br_low);
        let ratio_upp = (Br - Br_top) / (Br_upp - Br_top);
        if (Br >= Br_low) && (Br < Br_top) {
            NewCond_Fpre = 1.0 + (((1.0 + ((1.5 - ratio_low) * PI).sin()) / 2.0) * (Crop_dHI_pre / 100.0));
        } else if (Br > Br_top) && (Br <= Br_upp) {
            NewCond_Fpre = 1.0 + (((1.0 + ((0.5 + ratio_upp) * PI).sin()) / 2.0) * (Crop_dHI_pre / 100.0));
        } else {
            NewCond_Fpre = 1.0;
        }
    } else {
        NewCond_Fpre = 1.0;
    }
    if NewCond_CC <= 0.01 {
        NewCond_Fpre = 0.0;
    }
    NewCond_Fpre
}

/// solution/HIadj_pollination.py: pollination success factor.
pub fn HIadj_pollination(
    NewCond_CC: f64,
    NewCond_Fpol: f64,
    Crop_FloweringCD: f64,
    Crop_CCmin: f64,
    Crop_exc: f64,
    Ksw: &Ksw,
    Kst: &Kst,
    HIt: f64,
) -> f64 {
    // Deviation: HIt < 0 leaves FracFlow unbound in Python; the caller only passes HIt > 0.
    let mut FracFlow = 0.0;
    if HIt == 0.0 {
        FracFlow = 0.0;
    } else if HIt > 0.0 {
        let t1 = HIt - 1.0;
        let mut F1;
        if t1 == 0.0 {
            F1 = 0.0;
        } else {
            let mut t1Pct = 100.0 * (t1 / Crop_FloweringCD);
            if t1Pct > 100.0 {
                t1Pct = 100.0;
            }
            F1 = 0.00558 * (0.63 * t1Pct.ln()).exp() - (0.000969 * t1Pct) - 0.00383;
        }
        if F1 < 0.0 {
            F1 = 0.0;
        }
        let t2 = HIt;
        let mut F2;
        if t2 == 0.0 {
            F2 = 0.0;
        } else {
            let mut t2Pct = 100.0 * (t2 / Crop_FloweringCD);
            if t2Pct > 100.0 {
                t2Pct = 100.0;
            }
            F2 = 0.00558 * (0.63 * t2Pct.ln()).exp() - (0.000969 * t2Pct) - 0.00383;
        }
        if F2 < 0.0 {
            F2 = 0.0;
        }
        let F = if (F1 - F2).abs() < 0.0000001 { 0.0 } else { 100.0 * ((F1 + F2) / 2.0) / Crop_FloweringCD };
        FracFlow = F;
    }
    let dFpol = if NewCond_CC < Crop_CCmin {
        0.0
    } else {
        // min([Ksw.pol, Kst.PolC, Kst.PolH])
        let Ks = Ksw.pol.min(Kst.PolC).min(Kst.PolH);
        Ks * FracFlow * (1.0 + (Crop_exc / 100.0))
    };
    let mut NewCond_Fpol = NewCond_Fpol + dFpol;
    if NewCond_Fpol > 1.0 {
        NewCond_Fpol = 1.0;
    }
    NewCond_Fpol
}

/// solution/HIadj_post_anthesis.py: harvest index adjustment for water stress
/// after flowering.
///
/// Returns (sCor1, sCor2, fpost_upp, fpost_dwn, Fpost).
pub fn HIadj_post_anthesis(
    NewCond_DelayedCDs: f64,
    NewCond_sCor1: f64,
    NewCond_sCor2: f64,
    NewCond_DAP: f64,
    NewCond_Fpre: f64,
    NewCond_CC: f64,
    NewCond_fpost_upp: f64,
    NewCond_fpost_dwn: f64,
    Crop: &Crop,
    Ksw: &Ksw,
) -> (f64, f64, f64, f64, f64) {
    let mut NewCond_sCor1 = NewCond_sCor1;
    let mut NewCond_sCor2 = NewCond_sCor2;
    let mut NewCond_fpost_upp = NewCond_fpost_upp;
    let mut NewCond_fpost_dwn = NewCond_fpost_dwn;
    let InitCond_DelayedCDs = NewCond_DelayedCDs * 1.0;
    let InitCond_sCor1 = NewCond_sCor1 * 1.0;
    let InitCond_sCor2 = NewCond_sCor2 * 1.0;
    let tmax1 = Crop.CanopyDevEndCD - Crop.HIstartCD;
    let dap = NewCond_DAP - InitCond_DelayedCDs;
    if (dap <= (Crop.CanopyDevEndCD + 1.0))
        && (tmax1 > 0.0)
        && (NewCond_Fpre > 0.99)
        && (NewCond_CC > 0.001)
        && (Crop.a_HI > 0.0)
    {
        let dCor = 1.0 + (1.0 - Ksw.exp) / Crop.a_HI;
        NewCond_sCor1 = InitCond_sCor1 + (dCor / tmax1);
        let DayCor = dap - 1.0 - Crop.HIstartCD;
        NewCond_fpost_upp = (tmax1 / DayCor) * NewCond_sCor1;
    }
    let tmax2 = Crop.YldFormCD;
    let dap = NewCond_DAP - InitCond_DelayedCDs;
    if (dap <= (Crop.HIendCD + 1.0))
        && (tmax2 > 0.0)
        && (NewCond_Fpre > 0.99)
        && (NewCond_CC > 0.001)
        && (Crop.b_HI > 0.0)
    {
        let dCor = Ksw.sto.powf(0.1) * (1.0 - (1.0 - Ksw.sto) / Crop.b_HI);
        NewCond_sCor2 = InitCond_sCor2 + (dCor / tmax2);
        let DayCor = dap - 1.0 - Crop.HIstartCD;
        NewCond_fpost_dwn = (tmax2 / DayCor) * NewCond_sCor2;
    }
    let NewCond_Fpost;
    if (tmax1 == 0.0) && (tmax2 == 0.0) {
        NewCond_Fpost = 1.0;
    } else if tmax2 == 0.0 {
        NewCond_Fpost = NewCond_fpost_upp;
    } else if tmax1 == 0.0 {
        NewCond_Fpost = NewCond_fpost_dwn;
    } else if tmax1 <= tmax2 {
        NewCond_Fpost = NewCond_fpost_dwn * (((tmax1 * NewCond_fpost_upp) + (tmax2 - tmax1)) / tmax2);
    } else {
        NewCond_Fpost = NewCond_fpost_upp * (((tmax2 * NewCond_fpost_dwn) + (tmax1 - tmax2)) / tmax1);
    }
    (NewCond_sCor1, NewCond_sCor2, NewCond_fpost_upp, NewCond_fpost_dwn, NewCond_Fpost)
}

/// Harvest index for the day, adjusted for water and temperature stress.
pub fn harvest_index(
    prof: &SoilProfile,
    Soil_zTop: f64,
    Crop: &Crop,
    cond: &mut InitCond,
    et0: f64,
    temp_max: f64,
    temp_min: f64,
    growing_season: bool,
) {
    let InitCond_HI = cond.harvest_index;
    let InitCond_HIadj = cond.harvest_index_adj;
    let InitCond_PreAdj = cond.pre_adj;
    let NewCond = cond;
    if growing_season {
        let rz = root_zone_water(prof, NewCond.z_root, &NewCond.th, Soil_zTop, Crop.Zmin, Crop.Aer);
        let (water_root_depletion, taw);
        if (rz.Dr_Rz / rz.TAW_Rz) <= (rz.Dr_Zt / rz.TAW_Zt) {
            water_root_depletion = rz.Dr_Rz;
            taw = rz.TAW_Rz;
        } else {
            water_root_depletion = rz.Dr_Zt;
            taw = rz.TAW_Zt;
        }
        let beta = true;
        let ksw = water_stress(
            &Crop.p_up,
            &Crop.p_lo,
            Crop.ETadj,
            Crop.beta,
            &Crop.fshape_w,
            NewCond.t_early_sen,
            water_root_depletion,
            taw,
            et0,
            beta,
        );
        let kst = temperature_stress(Crop, temp_max, temp_min);
        let mut HIi = NewCond.hi_ref;
        let HIt = NewCond.dap - NewCond.delayed_cds - Crop.HIstartCD - 1.0;
        // Deviation: a CropType other than 1, 2 or 3 leaves harvest_index_adj
        // unbound in Python; the unadjusted index is kept here.
        let mut harvest_index_adj = HIi;
        if NewCond.yield_form && (HIt >= 0.0) {
            if (Crop.CropType == 2.0) || (Crop.CropType == 3.0) {
                if !InitCond_PreAdj {
                    NewCond.pre_adj = true;
                    NewCond.f_pre =
                        HIadj_pre_anthesis(NewCond.biomass, NewCond.biomass_ns, NewCond.canopy_cover, Crop.dHI_pre);
                }
                let HImax;
                if Crop.CropType == 3.0 {
                    if (HIt > 0.0) && (HIt <= Crop.FloweringCD) {
                        NewCond.f_pol = HIadj_pollination(
                            NewCond.canopy_cover,
                            NewCond.f_pol,
                            Crop.FloweringCD,
                            Crop.CCmin,
                            Crop.exc,
                            &ksw,
                            &kst,
                            HIt,
                        );
                    }
                    HImax = NewCond.f_pol * Crop.HI0;
                } else {
                    HImax = Crop.HI0;
                }
                if HIt > 0.0 {
                    let (s_cor1, s_cor2, fpost_upp, fpost_dwn, f_post) = HIadj_post_anthesis(
                        NewCond.delayed_cds,
                        NewCond.s_cor1,
                        NewCond.s_cor2,
                        NewCond.dap,
                        NewCond.f_pre,
                        NewCond.canopy_cover,
                        NewCond.fpost_upp,
                        NewCond.fpost_dwn,
                        Crop,
                        &ksw,
                    );
                    NewCond.s_cor1 = s_cor1;
                    NewCond.s_cor2 = s_cor2;
                    NewCond.fpost_upp = fpost_upp;
                    NewCond.fpost_dwn = fpost_dwn;
                    NewCond.f_post = f_post;
                }
                let mut HImult = NewCond.f_pre * NewCond.f_post;
                if HImult > 1.0 + (Crop.dHI0 / 100.0) {
                    HImult = 1.0 + (Crop.dHI0 / 100.0);
                }
                if HImax >= HIi {
                    harvest_index_adj = HImult * HIi;
                } else {
                    harvest_index_adj = HImult * HImax;
                }
            } else if Crop.CropType == 1.0 {
                harvest_index_adj = HIi;
            }
        } else {
            HIi = InitCond_HI;
            harvest_index_adj = InitCond_HIadj;
        }
        NewCond.harvest_index = HIi;
        NewCond.harvest_index_adj = harvest_index_adj;
    } else {
        NewCond.harvest_index = 0.0;
        NewCond.harvest_index_adj = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_anthesis_no_effect_without_dhi_pre() {
        assert_eq!(HIadj_pre_anthesis(100.0, 120.0, 0.5, 0.0), 1.0);
        assert_eq!(HIadj_pre_anthesis(100.0, 120.0, 0.005, 0.0), 0.0);
    }

    #[test]
    fn pollination_first_day_adds_nothing() {
        // On the first flowering day F1 = 0 and F2 is tiny, so Fpol barely moves.
        let ksw = Ksw { exp: 1.0, sto: 1.0, sen: 1.0, pol: 1.0, sto_lin: 1.0 };
        let kst = Kst { PolH: 1.0, PolC: 1.0 };
        let f = HIadj_pollination(0.8, 0.0, 10.0, 0.05, 50.0, &ksw, &kst, 0.0);
        assert_eq!(f, 0.0);
        let f = HIadj_pollination(0.01, 0.3, 10.0, 0.05, 50.0, &ksw, &kst, 5.0);
        assert_eq!(f, 0.3);
    }
}
