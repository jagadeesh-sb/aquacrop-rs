//! solution/soil_evaporation.py and solution/evap_layer_water_content.py

#![allow(non_snake_case)]

use crate::config::FieldMngt;
use crate::round::np_round;
use crate::soil::{Soil, SoilProfile};
use crate::state::InitCond;

/// Water storage in the evaporation layer (mm): at saturation, field
/// capacity, wilting point, air dry, and actual.
pub fn evap_layer_water_content(InitCond_th: &[f64], InitCond_EvapZ: f64, prof: &SoilProfile) -> (f64, f64, f64, f64, f64) {
    let comp_sto = prof.dzsum.iter().filter(|&&d| d < InitCond_EvapZ).count() + 1;
    // Deviation: Python indexes past the last compartment (IndexError) when the
    // evaporation layer reaches the bottom of the profile; stop at the last one.
    let comp_sto = comp_sto.min(prof.dzsum.len());
    let mut Wevap_Sat = 0.0;
    let mut Wevap_Fc = 0.0;
    let mut Wevap_Wp = 0.0;
    let mut Wevap_Dry = 0.0;
    let mut Wevap_Act = 0.0;
    for ii in 0..comp_sto {
        let factor = if prof.dzsum[ii] > InitCond_EvapZ { 1.0 - ((prof.dzsum[ii] - InitCond_EvapZ) / prof.dz[ii]) } else { 1.0 };
        Wevap_Act += factor * 1000.0 * InitCond_th[ii] * prof.dz[ii];
        Wevap_Sat += factor * 1000.0 * prof.th_s[ii] * prof.dz[ii];
        Wevap_Fc += factor * 1000.0 * prof.th_fc[ii] * prof.dz[ii];
        Wevap_Wp += factor * 1000.0 * prof.th_wp[ii] * prof.dz[ii];
        Wevap_Dry += factor * 1000.0 * prof.th_dry[ii] * prof.dz[ii];
    }
    if Wevap_Act < 0.0 {
        Wevap_Act = 0.0;
    }
    (Wevap_Sat, Wevap_Fc, Wevap_Wp, Wevap_Dry, Wevap_Act)
}

/// Soil evaporation: surface storage first, then stage 1 (limited by the
/// readily evaporable water) and stage 2 (in sub-daily steps, the layer
/// deepening as it dries). Returns `(EsAct, EsPot)`; the new state goes into
/// `cond` (e_pot, th, stage2, w_stage_2, w_surf, surface_storage, evap_z).
#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)] // ToExtract is updated in stage 2 and never read again, as in the Python
pub fn soil_evaporation(
    ClockStruct_EvapTimeSteps: usize,
    ClockStruct_SimOffSeason: bool,
    ClockStruct_TimeStepCounter: usize,
    prof: &SoilProfile,
    soil: &Soil,
    Crop_CalendarType: f64,
    Crop_Senescence: f64,
    IrrMngt_IrrMethod: i32,
    IrrMngt_WetSurf: f64,
    field: &FieldMngt,
    cond: &mut InitCond,
    et0: f64,
    Infl: f64,
    Rain: f64,
    Irr: f64,
    growing_season: bool,
) -> (f64, f64) {
    let Soil_EvapZmin = soil.evap_z_min;
    let Soil_EvapZmax = soil.evap_z_max;
    let Soil_REW = soil.rew;
    let Soil_Kex = soil.kex;
    let Soil_fwcc = soil.fwcc;
    let Soil_fWrelExp = soil.f_wrel_exp;
    let Soil_fevap = soil.f_evap;
    let FieldMngt_Mulches = field.mulches;
    let FieldMngt_fMulch = field.f_mulch;
    let FieldMngt_MulchPct = field.mulch_pct;
    // The NewCond_* arguments, as captured when the Python call is made.
    let NewCond_DAP = cond.dap;
    let mut NewCond_Wsurf = cond.w_surf;
    let mut NewCond_EvapZ = cond.evap_z;
    let mut NewCond_Stage2 = cond.stage2;
    let NewCond_DelayedCDs = cond.delayed_cds;
    let NewCond_GDDcum = cond.gdd_cum;
    let NewCond_DelayedGDDs = cond.delayed_gdds;
    let NewCond_CCxW = cond.ccx_w;
    let NewCond_CCadj = cond.canopy_cover_adj;
    let NewCond_CCxAct = cond.ccx_act;
    let NewCond_CC = cond.canopy_cover;
    let NewCond_PrematSenes = cond.premat_senes;
    let mut NewCond_SurfaceStorage = cond.surface_storage;
    let mut NewCond_Wstage2 = cond.w_stage_2;

    // Prepare stage 2 evaporation on the first simulated day, or the first day
    // of a season when the off-season is not simulated.
    if (ClockStruct_TimeStepCounter == 0) || ((NewCond_DAP == 1.0) && !ClockStruct_SimOffSeason) {
        NewCond_Wsurf = 0.0;
        NewCond_EvapZ = Soil_EvapZmin;
        NewCond_Stage2 = true;
        let (Wevap_Sat, Wevap_Fc, _Wevap_Wp, _Wevap_Dry, Wevap_Act) = evap_layer_water_content(&cond.th, NewCond_EvapZ, prof);
        // numpy floats: numpy's rounding.
        NewCond_Wstage2 = np_round((Wevap_Act - (Wevap_Fc - Soil_REW)) / (Wevap_Sat - (Wevap_Fc - Soil_REW)), 2);
        if NewCond_Wstage2 < 0.0 {
            NewCond_Wstage2 = 0.0;
        }
    }

    // Stage 1: refill the surface layer from today's infiltration.
    if (Rain > 0.0) || ((Irr > 0.0) && (IrrMngt_IrrMethod != 4)) {
        if Infl > 0.0 {
            NewCond_Wsurf = Infl;
            if NewCond_Wsurf > Soil_REW {
                NewCond_Wsurf = Soil_REW;
            }
            NewCond_Wstage2 = 0.0;
            NewCond_EvapZ = Soil_EvapZmin;
            NewCond_Stage2 = false;
        }
    }

    // Potential soil evaporation (mm/day).
    let mut EsPot;
    if growing_season {
        let tAdj = if Crop_CalendarType == 1.0 { NewCond_DAP - NewCond_DelayedCDs } else { NewCond_GDDcum - NewCond_DelayedGDDs };
        let EsPotMax = Soil_Kex * et0 * (1.0 - NewCond_CCxW * (Soil_fwcc / 100.0));
        EsPot = Soil_Kex * (1.0 - NewCond_CCadj) * et0;
        if (tAdj > Crop_Senescence) && (NewCond_CCxAct > 0.0) {
            let mult = if NewCond_CC > (NewCond_CCxAct / 2.0) {
                if NewCond_CC > NewCond_CCxAct {
                    0.0
                } else {
                    (NewCond_CCxAct - NewCond_CC) / (NewCond_CCxAct / 2.0)
                }
            } else {
                1.0
            };
            EsPot *= 1.0 - NewCond_CCxAct * (Soil_fwcc / 100.0) * mult;
            let CCxActAdj = (1.72 * NewCond_CCxAct) - NewCond_CCxAct.powf(2.0) + 0.3 * NewCond_CCxAct.powf(3.0);
            let mut EsPotMin = Soil_Kex * (1.0 - CCxActAdj) * et0;
            if EsPotMin < 0.0 {
                EsPotMin = 0.0;
            }
            if EsPot < EsPotMin {
                EsPot = EsPotMin;
            } else if EsPot > EsPotMax {
                EsPot = EsPotMax;
            }
        }
        if NewCond_PrematSenes && EsPot > EsPotMax {
            EsPot = EsPotMax;
        }
    } else {
        EsPot = Soil_Kex * et0;
    }

    // Mulches and partial wetting.
    let EsPotMul = if NewCond_SurfaceStorage < 0.000001 {
        if !FieldMngt_Mulches {
            EsPot
        } else {
            EsPot * (1.0 - FieldMngt_fMulch * (FieldMngt_MulchPct / 100.0))
        }
    } else {
        EsPot
    };
    let EsPotIrr = if (Irr > 0.0) && (IrrMngt_IrrMethod != 4) {
        if (Rain > 1.0) || (NewCond_SurfaceStorage > 0.0) {
            EsPot
        } else {
            EsPot * (IrrMngt_WetSurf / 100.0)
        }
    } else {
        EsPot
    };
    // Python's min(a, b) keeps a unless b is smaller.
    EsPot = if EsPotMul < EsPotIrr { EsPotMul } else { EsPotIrr };

    // Evaporate surface storage.
    let mut EsAct = 0.0;
    if NewCond_SurfaceStorage > 0.0 {
        if NewCond_SurfaceStorage > EsPot {
            EsAct = EsPot;
            NewCond_SurfaceStorage -= EsAct;
        } else {
            EsAct = NewCond_SurfaceStorage;
            NewCond_SurfaceStorage = 0.0;
            NewCond_Wsurf = Soil_REW;
            NewCond_Wstage2 = 0.0;
            NewCond_EvapZ = Soil_EvapZmin;
            NewCond_Stage2 = false;
        }
    }

    // Stage 1 evaporation.
    let mut ToExtract = EsPot - EsAct;
    let mut ExtractPotStg1 = if NewCond_Wsurf < ToExtract { NewCond_Wsurf } else { ToExtract };
    if ExtractPotStg1 > 0.0 {
        let comp_sto = prof.dzsum.iter().filter(|&&d| d < Soil_EvapZmin).count() as i64 + 1;
        let mut comp: i64 = -1;
        while (ExtractPotStg1 > 0.0) && (comp < comp_sto) {
            comp += 1;
            // Deviation: Python would index past the last compartment (IndexError).
            if comp as usize >= prof.dz.len() {
                break;
            }
            let c = comp as usize;
            let factor = if prof.dzsum[c] > Soil_EvapZmin { 1.0 - ((prof.dzsum[c] - Soil_EvapZmin) / prof.dz[c]) } else { 1.0 };
            let Wdry = 1000.0 * prof.th_dry[c] * prof.dz[c];
            let mut W = 1000.0 * cond.th[c] * prof.dz[c];
            let mut AvW = (W - Wdry) * factor;
            if AvW < 0.0 {
                AvW = 0.0;
            }
            if AvW >= ExtractPotStg1 {
                EsAct += ExtractPotStg1;
                W -= ExtractPotStg1;
                ToExtract -= ExtractPotStg1;
                ExtractPotStg1 = 0.0;
            } else {
                EsAct += AvW;
                ExtractPotStg1 -= AvW;
                ToExtract -= AvW;
                W -= AvW;
            }
            cond.set_th(c, W / (1000.0 * prof.dz[c]));
        }
        NewCond_Wsurf -= EsAct;
        if (NewCond_Wsurf < 0.0) || (ExtractPotStg1 > 0.0001) {
            NewCond_Wsurf = 0.0;
        }
        if NewCond_Wsurf < 0.0001 {
            let (Wevap_Sat, Wevap_Fc, _Wevap_Wp, _Wevap_Dry, Wevap_Act) = evap_layer_water_content(&cond.th, NewCond_EvapZ, prof);
            NewCond_Wstage2 = np_round((Wevap_Act - (Wevap_Fc - Soil_REW)) / (Wevap_Sat - (Wevap_Fc - Soil_REW)), 2);
            if NewCond_Wstage2 < 0.0 {
                NewCond_Wstage2 = 0.0;
            }
        }
    }

    // Stage 2 evaporation, in sub-daily steps.
    if ToExtract > 0.0 {
        NewCond_Stage2 = true;
        let Edt = ToExtract / ClockStruct_EvapTimeSteps as f64;
        for _jj in 0..ClockStruct_EvapTimeSteps {
            let (mut Wevap_Sat, mut Wevap_Fc, _, mut Wevap_Dry, mut Wevap_Act) = evap_layer_water_content(&cond.th, NewCond_EvapZ, prof);
            let mut Wupper = NewCond_Wstage2 * (Wevap_Sat - (Wevap_Fc - Soil_REW)) + (Wevap_Fc - Soil_REW);
            let mut Wlower = Wevap_Dry;
            let mut Wrel = (Wevap_Act - Wlower) / (Wupper - Wlower);
            if Soil_EvapZmax > Soil_EvapZmin {
                let mut Wcheck = Soil_fWrelExp * ((Soil_EvapZmax - NewCond_EvapZ) / (Soil_EvapZmax - Soil_EvapZmin));
                while (Wrel < Wcheck) && (NewCond_EvapZ < Soil_EvapZmax) {
                    NewCond_EvapZ += 0.001;
                    (Wevap_Sat, Wevap_Fc, _, Wevap_Dry, Wevap_Act) = evap_layer_water_content(&cond.th, NewCond_EvapZ, prof);
                    Wupper = NewCond_Wstage2 * (Wevap_Sat - (Wevap_Fc - Soil_REW)) + (Wevap_Fc - Soil_REW);
                    Wlower = Wevap_Dry;
                    Wrel = (Wevap_Act - Wlower) / (Wupper - Wlower);
                    Wcheck = Soil_fWrelExp * ((Soil_EvapZmax - NewCond_EvapZ) / (Soil_EvapZmax - Soil_EvapZmin));
                }
            }
            let mut Kr = ((Soil_fevap * Wrel).exp() - 1.0) / (Soil_fevap.exp() - 1.0);
            if Kr > 1.0 {
                Kr = 1.0;
            }
            let mut ToExtractStg2 = Kr * Edt;
            let comp_sto = prof.dzsum.iter().filter(|&&d| d < NewCond_EvapZ).count() as i64 + 1;
            let mut comp: i64 = -1;
            while (ToExtractStg2 > 0.0) && (comp < comp_sto) {
                comp += 1;
                // Deviation: Python would index past the last compartment (IndexError).
                if comp as usize >= prof.dz.len() {
                    break;
                }
                let c = comp as usize;
                let factor = if prof.dzsum[c] > NewCond_EvapZ { 1.0 - ((prof.dzsum[c] - NewCond_EvapZ) / prof.dz[c]) } else { 1.0 };
                let Wdry = 1000.0 * prof.th_dry[c] * prof.dz[c];
                let mut W = 1000.0 * cond.th[c] * prof.dz[c];
                // No `AvW < 0` guard in stage 2, as in the Python.
                let AvW = (W - Wdry) * factor;
                if AvW >= ToExtractStg2 {
                    EsAct += ToExtractStg2;
                    W -= ToExtractStg2;
                    ToExtract -= ToExtractStg2;
                    ToExtractStg2 = 0.0;
                } else {
                    EsAct += AvW;
                    W -= AvW;
                    ToExtractStg2 -= AvW;
                    ToExtract -= AvW;
                }
                cond.set_th(c, W / (1000.0 * prof.dz[c]));
            }
        }
    }

    cond.e_pot = EsPot;
    cond.stage2 = NewCond_Stage2;
    cond.w_stage_2 = NewCond_Wstage2;
    cond.w_surf = NewCond_Wsurf;
    cond.surface_storage = NewCond_SurfaceStorage;
    cond.evap_z = NewCond_EvapZ;
    (EsAct, EsPot)
}

#[cfg(test)]
mod tests {
    use crate::config::{FieldMngt, FieldMngtSpec};
    use crate::soil::{Soil, SoilSpec};
    use crate::solution::drainage::tests::loam;
    use crate::state::InitCond;

    #[test]
    fn layer_content_at_field_capacity() {
        let p = loam(12);
        let (sat, fc, wp, dry, act) = super::evap_layer_water_content(&p.th_fc, 0.15, &p);
        assert!((fc - 0.31 * 150.0).abs() < 1e-9);
        assert!((act - fc).abs() < 1e-12);
        assert!((sat - 0.46 * 150.0).abs() < 1e-9);
        assert!((wp - 0.15 * 150.0).abs() < 1e-9);
        assert!((dry - 0.075 * 150.0).abs() < 1e-9);
    }

    #[test]
    fn bare_soil_evaporates_water_it_removes() {
        let p = loam(12);
        let soil = Soil::new(&SoilSpec { soil_type: "Loam".into(), ..SoilSpec::default() }).unwrap();
        let field = FieldMngt::from(&FieldMngtSpec::default());
        let mut c = InitCond::new(12);
        c.th = p.th_fc.clone();
        let before: f64 = c.th.iter().map(|t| t * 100.0).sum();
        let (es, espot) = super::soil_evaporation(20, false, 0, &p, &soil, 2.0, 1000.0, 0, 100.0, &field, &mut c, 5.0, 0.0, 0.0, 0.0, false);
        let after: f64 = c.th.iter().map(|t| t * 100.0).sum();
        assert!((espot - 1.1 * 5.0).abs() < 1e-12);
        assert!(es > 0.0 && es <= espot);
        assert!((before - after - es).abs() < 1e-9);
        assert!(c.stage2);
        assert_eq!(c.e_pot, espot);
    }
}
