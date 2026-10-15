//! solution/canopy_cover.py, with its helpers cc_development.py,
//! cc_required_time.py, adjust_CCx.py and update_CCx_CDC.py.

#![allow(non_snake_case)]

use crate::crop::Crop;
use crate::round::round0;
use crate::soil::SoilProfile;
use crate::solution::root_zone_water::root_zone_water;
use crate::solution::water_stress::water_stress;
use crate::state::InitCond;

/// solution/cc_development.py: canopy cover after `dt` of growth or decline.
pub fn cc_development(CCo: f64, CCx: f64, CGC: f64, CDC: f64, dt: f64, Mode: &str, CCx0: f64) -> f64 {
    let mut canopy_cover;
    if Mode == "Growth" {
        canopy_cover = CCo * (CGC * dt).exp();
        if canopy_cover > (CCx / 2.0) {
            canopy_cover = CCx - 0.25 * (CCx / CCo) * CCx * (-CGC * dt).exp();
        }
        if canopy_cover > CCx {
            canopy_cover = CCx;
        }
    } else {
        // Mode == "Decline". Deviation: any other mode raises in Python.
        if CCx < 0.001 {
            canopy_cover = 0.0;
        } else {
            canopy_cover =
                CCx * (1.0 - 0.05 * ((dt * CDC * 3.33 * ((CCx + 2.29) / (CCx0 + 2.29)) / (CCx + 2.29)).exp() - 1.0));
        }
    }
    if canopy_cover > 1.0 {
        canopy_cover = 1.0;
    } else if canopy_cover < 0.0 {
        canopy_cover = 0.0;
    }
    canopy_cover
}

/// solution/cc_required_time.py: time needed to reach `cc_prev`.
pub fn cc_required_time(cc_prev: f64, CCo: f64, CCx: f64, CGC: f64, CDC: f64, Mode: &str) -> f64 {
    if Mode == "CGC" {
        let CGCx = if cc_prev <= (CCx / 2.0) {
            (cc_prev / CCo).ln()
        } else {
            ((0.25 * CCx * CCx / CCo) / (CCx - cc_prev)).ln()
        };
        CGCx / CGC
    } else {
        // Mode == "CDC".
        ((1.0 + (1.0 - cc_prev / CCx) / 0.05).ln()) / (CDC / CCx)
    }
}

/// solution/adjust_CCx.py: maximum canopy cover reachable under water stress.
pub fn adjust_CCx(
    cc_prev: f64,
    CCo: f64,
    CCx: f64,
    CGC: f64,
    CDC: f64,
    dt: f64,
    tSum: f64,
    Crop_CanopyDevEnd: f64,
    Crop_CCx: f64,
) -> f64 {
    let mut tCCtmp = cc_required_time(cc_prev, CCo, CCx, CGC, CDC, "CGC");
    if tCCtmp > 0.0 {
        tCCtmp = tCCtmp + (Crop_CanopyDevEnd - tSum) + dt;
        cc_development(CCo, CCx, CGC, CDC, tCCtmp, "Growth", Crop_CCx)
    } else {
        0.0
    }
}

/// solution/update_CCx_CDC.py: CCx and CDC adjusted for early senescence.
pub fn update_CCx_CDC(cc_prev: f64, CDC: f64, CCx: f64, dt: f64) -> (f64, f64) {
    let CCXadj = cc_prev / (1.0 - 0.05 * ((dt * ((CDC * 3.33) / (CCx + 2.29))).exp() - 1.0));
    let CDCadj = CDC * ((CCXadj + 2.29) / (CCx + 2.29));
    (CCXadj, CDCadj)
}

/// Canopy cover with and without water stress for one day.
pub fn canopy_cover(Crop: &Crop, prof: &SoilProfile, Soil_zTop: f64, cond: &mut InitCond, gdd: f64, et0: f64, growing_season: bool) {
    let InitCond_CC_NS = cond.canopy_cover_ns;
    let InitCond_CC = cond.canopy_cover;
    let InitCond_ProtectedSeed = cond.protected_seed;
    let InitCond_CCxAct = cond.ccx_act;
    let InitCond_CropDead = cond.crop_dead;
    let InitCond_tEarlySen = cond.t_early_sen;
    let InitCond_CCxW = cond.ccx_w;
    let NewCond = cond;
    NewCond.cc_prev = InitCond_CC;
    if growing_season {
        let rz = root_zone_water(prof, NewCond.z_root, &NewCond.th, Soil_zTop, Crop.Zmin, Crop.Aer);
        let (root_zone_depletion, taw);
        if (rz.Dr_Rz / rz.TAW_Rz) <= (rz.Dr_Zt / rz.TAW_Zt) {
            root_zone_depletion = rz.Dr_Rz;
            taw = rz.TAW_Rz;
        } else {
            root_zone_depletion = rz.Dr_Zt;
            taw = rz.TAW_Zt;
        }
        let beta = true;
        let mut water_stress_coef = water_stress(
            &Crop.p_up,
            &Crop.p_lo,
            Crop.ETadj,
            Crop.beta,
            &Crop.fshape_w,
            NewCond.t_early_sen,
            root_zone_depletion,
            taw,
            et0,
            beta,
        );
        // Deviation: CalendarType other than 1 or 2 leaves these unbound in Python; read as 2.
        let (dtCC, tCCadj) = if Crop.CalendarType == 1.0 {
            (1.0, NewCond.dap - NewCond.delayed_cds)
        } else {
            (gdd, NewCond.gdd_cum - NewCond.delayed_gdds)
        };

        // Canopy cover without water stress.
        if (tCCadj < Crop.Emergence) || (round0(tCCadj) > Crop.Maturity) {
            NewCond.canopy_cover_ns = 0.0;
        } else if tCCadj < Crop.CanopyDevEnd {
            if InitCond_CC_NS <= Crop.CC0 {
                NewCond.canopy_cover_ns = Crop.CC0 * (Crop.CGC * dtCC).exp();
            } else {
                let tmp_tCC = tCCadj - Crop.Emergence;
                NewCond.canopy_cover_ns =
                    cc_development(Crop.CC0, 0.98 * Crop.CCx, Crop.CGC, Crop.CDC, tmp_tCC, "Growth", Crop.CCx);
            }
            NewCond.ccx_act_ns = NewCond.canopy_cover_ns;
        } else if tCCadj > Crop.CanopyDevEnd {
            NewCond.ccx_w_ns = NewCond.ccx_act_ns;
            if tCCadj < Crop.Senescence {
                NewCond.canopy_cover_ns = InitCond_CC_NS;
                NewCond.ccx_act_ns = NewCond.canopy_cover_ns;
            } else {
                let tmp_tCC = tCCadj - Crop.Senescence;
                NewCond.canopy_cover_ns = cc_development(
                    Crop.CC0,
                    NewCond.ccx_act_ns,
                    Crop.CGC,
                    Crop.CDC,
                    tmp_tCC,
                    "Decline",
                    NewCond.ccx_act_ns,
                );
            }
        }

        // Canopy cover with water stress.
        if (tCCadj < Crop.Emergence) || (round0(tCCadj) > Crop.Maturity) {
            NewCond.canopy_cover = 0.0;
            NewCond.cc0_adj = Crop.CC0;
        } else if tCCadj < Crop.CanopyDevEnd {
            if InitCond_CC <= NewCond.cc0_adj
                || (InitCond_ProtectedSeed && (InitCond_CC <= (1.25 * NewCond.cc0_adj)))
            {
                if InitCond_ProtectedSeed {
                    let tmp_tCC = tCCadj - Crop.Emergence;
                    NewCond.canopy_cover =
                        cc_development(Crop.CC0, Crop.CCx, Crop.CGC, Crop.CDC, tmp_tCC, "Growth", Crop.CCx);
                    if NewCond.canopy_cover > (1.25 * NewCond.cc0_adj) {
                        NewCond.protected_seed = false;
                    }
                } else {
                    NewCond.canopy_cover = NewCond.cc0_adj * (Crop.CGC * dtCC).exp();
                }
            } else if InitCond_CC < (0.9799 * Crop.CCx) {
                let CGCadj = Crop.CGC * water_stress_coef.exp;
                if CGCadj > 0.0 {
                    let CCXadj = adjust_CCx(
                        InitCond_CC,
                        NewCond.cc0_adj,
                        Crop.CCx,
                        CGCadj,
                        Crop.CDC,
                        dtCC,
                        tCCadj,
                        Crop.CanopyDevEnd,
                        Crop.CCx,
                    );
                    if CCXadj < 0.0 {
                        NewCond.canopy_cover = InitCond_CC;
                    } else if (InitCond_CC - (0.9799 * Crop.CCx)).abs() < 0.001 {
                        let tmp_tCC = tCCadj - Crop.Emergence;
                        NewCond.canopy_cover =
                            cc_development(Crop.CC0, Crop.CCx, Crop.CGC, Crop.CDC, tmp_tCC, "Growth", Crop.CCx);
                    } else {
                        let tReq = cc_required_time(InitCond_CC, NewCond.cc0_adj, CCXadj, CGCadj, Crop.CDC, "CGC");
                        if tReq > 0.0 {
                            let tmp_tCC = tReq + dtCC;
                            NewCond.canopy_cover =
                                cc_development(NewCond.cc0_adj, CCXadj, CGCadj, Crop.CDC, tmp_tCC, "Growth", Crop.CCx);
                        } else {
                            NewCond.canopy_cover = InitCond_CC;
                        }
                    }
                } else {
                    NewCond.canopy_cover = InitCond_CC;
                    if NewCond.canopy_cover > NewCond.cc0_adj {
                        NewCond.cc0_adj = Crop.CC0;
                    } else {
                        NewCond.cc0_adj = NewCond.canopy_cover;
                    }
                }
            } else {
                let tmp_tCC = tCCadj - Crop.Emergence;
                NewCond.canopy_cover = cc_development(Crop.CC0, Crop.CCx, Crop.CGC, Crop.CDC, tmp_tCC, "Growth", Crop.CCx);
                NewCond.cc0_adj = Crop.CC0;
            }
            if NewCond.canopy_cover > InitCond_CCxAct {
                NewCond.ccx_act = NewCond.canopy_cover;
            }
        } else if tCCadj > Crop.CanopyDevEnd {
            if tCCadj < Crop.Senescence {
                NewCond.canopy_cover = InitCond_CC;
                if NewCond.canopy_cover > InitCond_CCxAct {
                    NewCond.ccx_act = NewCond.canopy_cover;
                }
            } else {
                let CDCadj = Crop.CDC * ((NewCond.ccx_act + 2.29) / (Crop.CCx + 2.29));
                let tmp_tCC = tCCadj - Crop.Senescence;
                NewCond.canopy_cover = cc_development(
                    NewCond.cc0_adj,
                    NewCond.ccx_act,
                    Crop.CGC,
                    CDCadj,
                    tmp_tCC,
                    "Decline",
                    NewCond.ccx_act,
                );
            }
            if (NewCond.canopy_cover < 0.001) && !InitCond_CropDead {
                NewCond.canopy_cover = 0.0;
                NewCond.crop_dead = true;
            }
        }

        // Early (water stress induced) senescence.
        if tCCadj >= Crop.Emergence && ((tCCadj < Crop.Senescence) || (InitCond_tEarlySen > 0.0)) {
            if (water_stress_coef.sen < 1.0) && !InitCond_ProtectedSeed {
                NewCond.premat_senes = true;
                if InitCond_tEarlySen == 0.0 {
                    NewCond.ccx_early_sen = InitCond_CC;
                }
                NewCond.t_early_sen = InitCond_tEarlySen + dtCC;
                let beta = false;
                water_stress_coef = water_stress(
                    &Crop.p_up,
                    &Crop.p_lo,
                    Crop.ETadj,
                    Crop.beta,
                    &Crop.fshape_w,
                    NewCond.t_early_sen,
                    root_zone_depletion,
                    taw,
                    et0,
                    beta,
                );
                let CDCadj = if water_stress_coef.sen > 0.99999 {
                    0.0001
                } else {
                    (1.0 - water_stress_coef.sen.powf(8.0)) * Crop.CDC
                };
                let mut CCsen;
                if NewCond.ccx_early_sen < 0.001 {
                    CCsen = 0.0;
                } else {
                    let tReq = ((1.0 + (1.0 - InitCond_CC / NewCond.ccx_early_sen) / 0.05).ln())
                        / ((CDCadj * 3.33) / (NewCond.ccx_early_sen + 2.29));
                    let tmp_tCC = tReq + dtCC;
                    CCsen = NewCond.ccx_early_sen
                        * (1.0 - 0.05 * ((tmp_tCC * ((CDCadj * 3.33) / (NewCond.ccx_early_sen + 2.29))).exp() - 1.0));
                    if CCsen < 0.0 {
                        CCsen = 0.0;
                    }
                }
                if tCCadj < Crop.Senescence {
                    if CCsen > Crop.CCx {
                        CCsen = Crop.CCx;
                    }
                    NewCond.canopy_cover = CCsen;
                    if NewCond.canopy_cover > InitCond_CC {
                        NewCond.canopy_cover = InitCond_CC;
                    }
                    NewCond.ccx_act = NewCond.canopy_cover;
                    if NewCond.canopy_cover < Crop.CC0 {
                        NewCond.cc0_adj = NewCond.canopy_cover;
                    } else {
                        NewCond.cc0_adj = Crop.CC0;
                    }
                } else if CCsen < NewCond.canopy_cover {
                    NewCond.canopy_cover = CCsen;
                }
                if (NewCond.canopy_cover < 0.001) && !InitCond_CropDead {
                    NewCond.canopy_cover = 0.0;
                    NewCond.crop_dead = true;
                }
            } else {
                NewCond.premat_senes = false;
                if (tCCadj > Crop.Senescence) && (InitCond_tEarlySen > 0.0) {
                    let tmp_tCC = tCCadj - dtCC - Crop.Senescence;
                    let (CCXadj, CDCadj) = update_CCx_CDC(InitCond_CC, Crop.CDC, Crop.CCx, tmp_tCC);
                    NewCond.ccx_act = CCXadj;
                    let tmp_tCC = tCCadj - Crop.Senescence;
                    NewCond.canopy_cover =
                        cc_development(NewCond.cc0_adj, CCXadj, Crop.CGC, CDCadj, tmp_tCC, "Decline", CCXadj);
                    if (NewCond.canopy_cover < 0.001) && !InitCond_CropDead {
                        NewCond.canopy_cover = 0.0;
                        NewCond.crop_dead = true;
                    }
                }
                NewCond.t_early_sen = 0.0;
            }
            if NewCond.canopy_cover > InitCond_CCxW {
                NewCond.ccx_w = NewCond.canopy_cover;
            }
        }

        if NewCond.canopy_cover_ns < NewCond.canopy_cover {
            NewCond.canopy_cover_ns = NewCond.canopy_cover;
            if tCCadj < Crop.CanopyDevEnd {
                NewCond.ccx_act_ns = NewCond.canopy_cover_ns;
            }
        }
        NewCond.canopy_cover_adj = (1.72 * NewCond.canopy_cover) - NewCond.canopy_cover.powf(2.0)
            + (0.3 * NewCond.canopy_cover.powf(3.0));
        NewCond.canopy_cover_adj_ns = (1.72 * NewCond.canopy_cover_ns) - NewCond.canopy_cover_ns.powf(2.0)
            + (0.3 * NewCond.canopy_cover_ns.powf(3.0));
    } else {
        NewCond.canopy_cover = 0.0;
        NewCond.canopy_cover_adj = 0.0;
        NewCond.canopy_cover_ns = 0.0;
        NewCond.canopy_cover_adj_ns = 0.0;
        NewCond.ccx_w = 0.0;
        NewCond.ccx_act = 0.0;
        NewCond.ccx_w_ns = 0.0;
        NewCond.ccx_act_ns = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_from_zero_time_is_cc0() {
        assert_eq!(cc_development(0.05, 0.9, 0.01, 0.01, 0.0, "Growth", 0.9), 0.05);
    }

    #[test]
    fn decline_from_zero_time_is_ccx() {
        assert_eq!(cc_development(0.05, 0.9, 0.01, 0.01, 0.0, "Decline", 0.9), 0.9);
        assert_eq!(cc_development(0.05, 0.0005, 0.01, 0.01, 10.0, "Decline", 0.9), 0.0);
    }

    #[test]
    fn required_time_inverts_growth() {
        let (cco, ccx, cgc) = (0.05, 0.9, 0.012);
        let cc = cc_development(cco, ccx, cgc, 0.01, 40.0, "Growth", ccx);
        let t = cc_required_time(cc, cco, ccx, cgc, 0.01, "CGC");
        assert!((t - 40.0).abs() < 1e-9);
    }

    #[test]
    fn update_ccx_at_zero_time() {
        let (ccx_adj, cdc_adj) = update_CCx_CDC(0.8, 0.01, 0.9, 0.0);
        assert_eq!(ccx_adj, 0.8);
        assert_eq!(cdc_adj, 0.01 * ((0.8 + 2.29) / (0.9 + 2.29)));
    }
}
