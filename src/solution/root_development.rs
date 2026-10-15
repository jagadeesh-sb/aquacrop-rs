//! solution/root_development.py

#![allow(non_snake_case)]

use crate::crop::Crop;
use crate::round::{np_round, round0};
use crate::soil::SoilProfile;

/// Effective rooting depth (m) and the root-extraction correction factor.
pub fn root_development(
    Crop: &Crop,
    prof: &SoilProfile,
    NewCond_DAP: f64,
    NewCond_Zroot: f64,
    NewCond_DelayedCDs: f64,
    NewCond_GDDcum: f64,
    NewCond_DelayedGDDs: f64,
    NewCond_TrRatio: f64,
    NewCond_th: &[f64],
    NewCond_CC: f64,
    NewCond_CC_NS: f64,
    NewCond_Germination: bool,
    NewCond_rCor: f64,
    NewCond_Tpot: f64,
    NewCond_zGW: f64,
    gdd: f64,
    growing_season: bool,
    water_table_presence: i32,
) -> (f64, f64) {
    let mut NewCond_Zroot = NewCond_Zroot;
    let mut NewCond_rCor = NewCond_rCor;
    let mut Zroot_init = NewCond_Zroot * 1.0;
    let Soil_nLayer = prof.n_layers() as i64;
    if growing_season {
        if NewCond_DAP == 1.0 {
            // Python also sets NewCond_Zroot = Zmin here; it is overwritten below before any read.
            Zroot_init = Crop.Zmin * 1.0;
        }
        // Deviation: CalendarType other than 1 or 2 leaves tAdj unbound in Python; read as 2.
        let tAdj = if Crop.CalendarType == 1.0 {
            NewCond_DAP - NewCond_DelayedCDs
        } else {
            NewCond_GDDcum - NewCond_DelayedGDDs
        };
        let Zini = Crop.Zmin * (Crop.PctZmin / 100.0);
        let t0 = round0(Crop.Emergence / 2.0);
        let tmax = Crop.MaxRooting;
        let tOld = if Crop.CalendarType == 1.0 { tAdj - 1.0 } else { tAdj - gdd };
        let mut ZrOld;
        if tOld >= tmax {
            ZrOld = Crop.Zmax;
        } else if tOld <= t0 {
            ZrOld = Zini;
        } else {
            let X = (tOld - t0) / (tmax - t0);
            ZrOld = Zini + (Crop.Zmax - Zini) * X.powf(1.0 / Crop.fshape_r);
        }
        if ZrOld < Crop.Zmin {
            ZrOld = Crop.Zmin;
        }
        let mut Zr;
        if tAdj >= tmax {
            Zr = Crop.Zmax;
        } else if tAdj <= t0 {
            Zr = Zini;
        } else {
            let X = (tAdj - t0) / (tmax - t0);
            Zr = Zini + (Crop.Zmax - Zini) * X.powf(1.0 / Crop.fshape_r);
        }
        if Zr < Crop.Zmin {
            Zr = Crop.Zmin;
        }
        let ZrPot = Zr;
        let mut dZr = Zr - ZrOld;
        if Zr > Crop.Zmin {
            let mut layeri: i64 = 1;
            let mut l_idx = prof.layer_idx(layeri);
            // prof.dz[l_idx].sum(): a numpy sum, so a numpy float and numpy's rounding.
            let mut Zsoil = prof.dz_sum(&l_idx);
            while (np_round(Zsoil, 2) <= Crop.Zmin) && (layeri < Soil_nLayer) {
                layeri = layeri + 1;
                l_idx = prof.layer_idx(layeri);
                Zsoil = Zsoil + prof.dz_sum(&l_idx);
            }
            let mut soil_layer_dz = prof.dz_sum(&l_idx);
            let mut layer_comp = l_idx[0];
            let mut ZrAdj = Crop.Zmin;
            let mut ZrRemain = Zr - Crop.Zmin;
            let mut deltaZ = Zsoil - Crop.Zmin;
            let mut EndProf = false;
            let mut ZrOUT = 0.0;
            while !EndProf {
                let ZrTest = ZrAdj + (ZrRemain * (prof.Penetrability[layer_comp] / 100.0));
                if (layeri == Soil_nLayer) || (prof.Penetrability[layer_comp] == 0.0) || (ZrTest <= Zsoil) {
                    ZrOUT = ZrTest;
                    EndProf = true;
                } else {
                    ZrAdj = Zsoil;
                    ZrRemain = ZrRemain - (deltaZ / (prof.Penetrability[layer_comp] / 100.0));
                    layeri = layeri + 1;
                    l_idx = prof.layer_idx(layeri);
                    layer_comp = l_idx[0];
                    soil_layer_dz = prof.dz_sum(&l_idx);
                    Zsoil = Zsoil + soil_layer_dz;
                    deltaZ = soil_layer_dz;
                }
            }
            let _ = soil_layer_dz;
            Zr = ZrOUT.max(ZrOld);
            dZr = Zr - ZrOld;
        }
        if NewCond_TrRatio < 0.9999 {
            if Crop.fshape_ex >= 0.0 {
                dZr = dZr * NewCond_TrRatio;
            } else {
                let fAdj = ((NewCond_TrRatio * Crop.fshape_ex).exp() - 1.0) / (Crop.fshape_ex.exp() - 1.0);
                dZr = dZr * fAdj;
            }
        }
        if dZr > 0.001 {
            let pZexp = Crop.p_up[1] + ((1.0 - Crop.p_up[1]) / 2.0);
            let ZiTmp = Zroot_init + dZr;
            let idx = prof.dzsum.iter().position(|&d| d >= ZiTmp).expect("roots deeper than the soil profile");
            let TAWprof = prof.th_fc[idx] - prof.th_wp[idx];
            let thThr = prof.th_fc[idx] - (pZexp * TAWprof);
            if NewCond_th[idx] < thThr {
                if NewCond_th[idx] <= prof.th_wp[idx] {
                    dZr = 0.0;
                } else {
                    let Wrel = (prof.th_fc[idx] - NewCond_th[idx]) / TAWprof;
                    let Drel = 1.0 - ((1.0 - Wrel) / (1.0 - pZexp));
                    let Ks = 1.0 - (((Drel * Crop.fshape_w[1]).exp() - 1.0) / (Crop.fshape_w[1].exp() - 1.0));
                    dZr = dZr * Ks;
                }
            }
        }
        if (NewCond_CC <= 0.0) && (NewCond_CC_NS > 0.5) {
            dZr = 0.0;
        }
        if !NewCond_Germination {
            dZr = 0.0;
        }
        NewCond_Zroot = Zroot_init + dZr;
        if NewCond_Zroot < ZrPot {
            NewCond_rCor =
                (2.0 * (ZrPot / NewCond_Zroot) * ((Crop.SxTop + Crop.SxBot) / 2.0) - Crop.SxTop) / Crop.SxBot;
            if NewCond_Tpot > 0.0 {
                NewCond_rCor = NewCond_rCor * NewCond_TrRatio;
                if NewCond_rCor < 1.0 {
                    NewCond_rCor = 1.0;
                }
            }
        } else {
            NewCond_rCor = 1.0;
        }
        if (water_table_presence == 1) && (NewCond_zGW > 0.0) && NewCond_Zroot > NewCond_zGW {
            NewCond_Zroot = NewCond_zGW;
            if NewCond_Zroot < Crop.Zmin {
                NewCond_Zroot = Crop.Zmin;
            }
        }
    } else {
        NewCond_Zroot = 0.0;
    }
    (NewCond_Zroot, NewCond_rCor)
}
