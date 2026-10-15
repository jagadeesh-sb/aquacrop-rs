//! solution/check_groundwater_table.py

#![allow(non_snake_case)]

use crate::soil::SoilProfile;

/// Update the water table depth for the day and the field capacity of the
/// compartments it raises.
///
/// Returns `(th_fc_Adj, wt_in_soil, z_gw)`. Without a water table AquaCrop-OSPy
/// returns `None` for the last two; here they are `false` and NaN. (The Python
/// also takes `NewCond_th`, which it never reads.)
pub fn check_groundwater_table(
    prof: &SoilProfile,
    NewCond_zGW: f64,
    NewCond_th_fc_Adj: &[f64],
    water_table_presence: i32,
    z_gw: f64,
) -> (Vec<f64>, bool, f64) {
    let _ = NewCond_zGW;
    if water_table_presence == 1 {
        let NewCond_zGW = z_gw;
        let zMid = &prof.zMid;
        // Deviation: with a negative depth Python leaves NewCond_WTinSoil unset
        // and fails with UnboundLocalError; here the table is not in the soil.
        let mut NewCond_WTinSoil = false;
        if NewCond_zGW >= 0.0 {
            NewCond_WTinSoil = zMid.iter().any(|&z| z >= NewCond_zGW);
        }
        let n = prof.Comp.len();
        let mut thfcAdj = vec![0.0; n];
        let mut compi = n as i64 - 1;
        while compi >= 0 {
            let c = compi as usize;
            let Xmax = if prof.th_fc[c] <= 0.1 {
                1.0
            } else if prof.th_fc[c] >= 0.3 {
                2.0
            } else {
                let pF = 2.0 + 0.3 * (prof.th_fc[c] - 0.1) / 0.2;
                (pF * 10f64.ln()).exp() / 100.0
            };
            if (NewCond_zGW < 0.0) || ((NewCond_zGW - zMid[c]) >= Xmax) {
                for ii in 0..c + 1 {
                    thfcAdj[ii] = prof.th_fc[ii];
                }
                compi = -1;
            } else {
                if prof.th_fc[c] >= prof.th_s[c] {
                    thfcAdj[c] = prof.th_fc[c];
                } else if zMid[c] >= NewCond_zGW {
                    thfcAdj[c] = prof.th_s[c];
                } else {
                    let dV = prof.th_s[c] - prof.th_fc[c];
                    let dFC = (dV / (Xmax * Xmax)) * (zMid[c] - (NewCond_zGW - Xmax)).powf(2.0);
                    thfcAdj[c] = prof.th_fc[c] + dFC;
                }
                compi -= 1;
            }
        }
        return (thfcAdj, NewCond_WTinSoil, NewCond_zGW);
    }
    (NewCond_th_fc_Adj.to_vec(), false, f64::NAN)
}

#[cfg(test)]
mod tests {
    use crate::solution::drainage::tests::loam;

    #[test]
    fn deep_table_leaves_field_capacity() {
        let p = loam(12);
        let (fc, in_soil, z) = super::check_groundwater_table(&p, 0.0, &p.th_fc_Adj, 1, 10.0);
        assert_eq!(fc, p.th_fc);
        assert!(!in_soil);
        assert_eq!(z, 10.0);
    }

    #[test]
    fn shallow_table_saturates_below_it() {
        let p = loam(12);
        let (fc, in_soil, _) = super::check_groundwater_table(&p, 0.0, &p.th_fc_Adj, 1, 0.8);
        assert!(in_soil);
        assert_eq!(fc[11], p.th_s[11]);
        assert!(fc[5] > p.th_fc[5] && fc[5] < p.th_s[5]);
    }

    #[test]
    fn no_table() {
        let p = loam(4);
        let (fc, in_soil, z) = super::check_groundwater_table(&p, 1.0, &p.th_fc_Adj, 0, 0.0);
        assert_eq!(fc, p.th_fc_Adj);
        assert!(!in_soil && z.is_nan());
    }
}
