//! solution/capillary_rise.py

#![allow(non_snake_case)]

use crate::numeric::np_sum;
use crate::round::{np_round, round0};
use crate::soil::SoilProfile;
use crate::state::InitCond;

/// Capillary rise from a shallow water table into the profile (mm).
pub fn capillary_rise(
    prof: &SoilProfile,
    Soil_nLayer: usize,
    Soil_fshape_cr: f64,
    cond: &mut InitCond,
    FluxOut: &[f64],
    water_table_presence: i32,
) -> f64 {
    let z_gw = cond.z_gw;
    if water_table_presence != 1 {
        // water_table_presence == 0: no water table.
        return 0.0;
    }
    let n = prof.Comp.len();
    let mut zBot = prof.dzsum[n - 1];
    let mut zBotMid = prof.zMid[n - 1];
    let mut MaxCR;
    if (prof.Ksat[n - 1] > 0.0) && (z_gw > 0.0) && ((z_gw - zBotMid) < 4.0) {
        if zBotMid >= z_gw {
            MaxCR = 99.0;
        } else {
            MaxCR = (((z_gw - zBotMid).ln() - prof.bCR[n - 1]) / prof.aCR[n - 1]).exp();
            if MaxCR > 99.0 {
                MaxCR = 99.0;
            }
        }
    } else {
        MaxCR = 0.0;
    }

    // The Python totals the layer thicknesses (zTopLayer) and then runs
    //     while (zTopLayer < z_gw) and (layeri < Soil_nLayer): ...
    // with layeri = prof.Layer[-1], which it asserts equals Soil_nLayer, so the
    // loop never runs and MaxCR is not limited by the layers above. It is left
    // out here; the sum is kept for fidelity of the (unused) value.
    let mut layers: Vec<i64> = prof.Layer.clone();
    layers.sort_unstable();
    layers.dedup();
    let mut _zTopLayer = 0.0;
    for layeri in layers {
        let l_idx = prof.layer_idx(layeri);
        let LayThk = np_sum(&l_idx.iter().map(|&i| prof.dz[i]).collect::<Vec<f64>>());
        _zTopLayer += LayThk;
    }
    let _ = Soil_nLayer;

    let mut compi = n as i64 - 1;
    let mut WCr = 0.0;
    while (round0(MaxCR * 1000.0) > 0.0) && (compi > -1) && (round0(FluxOut[compi as usize] * 1000.0) == 0.0) {
        let c = compi as usize;
        let mut Df;
        if (cond.th[c] >= prof.th_wp[c]) && (Soil_fshape_cr > 0.0) {
            Df = 1.0 - ((cond.th[c] - prof.th_wp[c]) / (cond.th_fc_Adj[c] - prof.th_wp[c])).powf(Soil_fshape_cr);
            if Df > 1.0 {
                Df = 1.0;
            } else if Df < 0.0 {
                Df = 0.0;
            }
        } else {
            Df = 1.0;
        }
        let thThr = (prof.th_wp[c] + prof.th_fc[c]) / 2.0;
        let Krel = if cond.th[c] < thThr {
            if (cond.th[c] <= prof.th_wp[c]) || (thThr <= prof.th_wp[c]) {
                0.0
            } else {
                (cond.th[c] - prof.th_wp[c]) / (thThr - prof.th_wp[c])
            }
        } else {
            1.0
        };
        let dth = np_round(cond.th_fc_Adj[c] - cond.th[c], 4);
        if (dth > 0.0) && ((zBot - prof.dz[c] / 2.0) < z_gw) {
            let dthMax = Krel * Df * MaxCR / (1000.0 * prof.dz[c]);
            let CRcomp;
            if dth >= dthMax {
                cond.set_th(c, cond.th[c] + dthMax);
                CRcomp = dthMax * 1000.0 * prof.dz[c];
                MaxCR = 0.0;
            } else {
                cond.set_th(c, cond.th_fc_Adj[c]);
                CRcomp = dth * 1000.0 * prof.dz[c];
                MaxCR = (Krel * MaxCR) - CRcomp;
            }
            WCr += CRcomp;
        }
        zBot -= prof.dz[c];
        compi -= 1;
        if compi > -1 {
            let c = compi as usize;
            zBotMid = zBot - (prof.dz[c] / 2.0);
            let LimCR;
            if (prof.Ksat[c] > 0.0) && (z_gw > 0.0) && ((z_gw - zBotMid) < 4.0) {
                if zBotMid >= z_gw {
                    LimCR = 99.0;
                } else {
                    let l = (((z_gw - zBotMid).ln() - prof.bCR[c]) / prof.aCR[c]).exp();
                    LimCR = if l > 99.0 { 99.0 } else { l };
                }
            } else {
                LimCR = 0.0;
            }
            if MaxCR > LimCR {
                MaxCR = LimCR;
            }
        }
    }
    WCr
}

#[cfg(test)]
mod tests {
    use crate::solution::drainage::tests::loam;
    use crate::state::InitCond;

    #[test]
    fn no_table_no_rise() {
        let p = loam(12);
        let mut c = InitCond::new(12);
        c.th = p.th_wp.clone();
        assert_eq!(super::capillary_rise(&p, 1, 16.0, &mut c, &[0.0; 12], 0), 0.0);
        assert_eq!(c.th, p.th_wp);
    }

    #[test]
    fn shallow_table_wets_dry_bottom() {
        let mut p = loam(12);
        p.aCR = vec![-0.4986 + 9.0 * 500.0 / 100000.0; 12];
        p.bCR = vec![-2.1320 + 0.4778 * 500f64.ln(); 12];
        let mut c = InitCond::new(12);
        c.th = vec![0.2; 12];
        c.th_fc_Adj = p.th_fc.clone();
        c.z_gw = 1.5;
        let cr = super::capillary_rise(&p, 1, 16.0, &mut c, &[0.0; 12], 1);
        assert!(cr > 0.0);
        let gained: f64 = c.th.iter().map(|t| (t - 0.2) * 100.0).sum();
        assert!((gained - cr).abs() < 1e-9);
    }
}
