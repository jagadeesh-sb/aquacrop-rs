//! solution/rainfall_partition.py

#![allow(non_snake_case)]

use crate::config::FieldMngt;
use crate::round::round0;
use crate::soil::SoilProfile;

/// Partition rainfall into surface runoff and infiltration with the curve
/// number method. Returns `(Runoff, Infl, DaySubmerged)`.
///
/// Like the Python, this writes into `prof.dzsum`: a compartment whose bottom
/// lies below `Soil_zCN` has its `dzsum` cut to `Soil_zCN`, for the rest of
/// the simulation. With the usual compartment layout no `dzsum` crosses
/// `Soil_zCN`, so nothing changes.
#[allow(clippy::too_many_arguments)]
pub fn rainfall_partition(
    precipitation: f64,
    InitCond_th: &[f64],
    NewCond_DaySubmerged: f64,
    field: &FieldMngt,
    Soil_CN: f64,
    Soil_AdjCN: f64,
    Soil_zCN: f64,
    Soil_nComp: usize,
    prof: &mut SoilProfile,
) -> (f64, f64, f64) {
    let FieldMngt_SRinhb = field.sr_inhb;
    let FieldMngt_Bunds = field.bunds;
    let FieldMngt_zBund = field.z_bund;
    let FieldMngt_CNadjPct = field.curve_number_adj_pct;
    let mut NewCond_DaySubmerged = NewCond_DaySubmerged;
    let (Runoff, Infl);
    if !FieldMngt_SRinhb && (!FieldMngt_Bunds || (FieldMngt_zBund < 0.001)) {
        NewCond_DaySubmerged = 0.0;
        let mut cn = Soil_CN * (1.0 + (FieldMngt_CNadjPct / 100.0));
        if Soil_AdjCN == 1.0 {
            let CNbot = round0(
                1.4 * ((-14.0 * 10f64.ln()).exp()) + (0.507 * cn) - (0.00374 * cn.powf(2.0)) + (0.0000867 * cn.powf(3.0)),
            );
            let CNtop = round0(
                5.6 * ((-14.0 * 10f64.ln()).exp()) + (2.33 * cn) - (0.0209 * cn.powf(2.0)) + (0.000076 * cn.powf(3.0)),
            );
            let n_below = prof.dzsum.iter().filter(|&&d| d >= Soil_zCN).count();
            let mut comp_sto = if n_below == 0 { Soil_nComp } else { Soil_nComp - n_below };
            comp_sto += 1;
            // Deviation: when every compartment ends above Soil_zCN, Python asks for
            // compartment nComp and fails with IndexError; stop at the last one.
            let comp_sto = comp_sto.min(prof.dzsum.len());
            let mut xx = 0.0;
            let mut wrel = vec![0.0; comp_sto];
            for ii in 0..comp_sto {
                if prof.dzsum[ii] > Soil_zCN {
                    prof.dzsum[ii] = Soil_zCN;
                }
                let wx = 1.016 * (1.0 - (-4.16 * (prof.dzsum[ii] / Soil_zCN)).exp());
                wrel[ii] = wx - xx;
                if wrel[ii] < 0.0 {
                    wrel[ii] = 0.0;
                } else if wrel[ii] > 1.0 {
                    wrel[ii] = 1.0;
                }
                xx = wx;
            }
            let mut wet_top = 0.0;
            for ii in 0..comp_sto {
                // Python's max(a, b) keeps a unless b is larger.
                let th = if InitCond_th[ii] > prof.th_wp[ii] { InitCond_th[ii] } else { prof.th_wp[ii] };
                wet_top += wrel[ii] * ((th - prof.th_wp[ii]) / (prof.th_fc[ii] - prof.th_wp[ii]));
            }
            if wet_top > 1.0 {
                wet_top = 1.0;
            } else if wet_top < 0.0 {
                wet_top = 0.0;
            }
            cn = round0(CNbot + (CNtop - CNbot) * wet_top);
        }
        let S = (25400.0 / cn) - 254.0;
        let term = precipitation - ((5.0 / 100.0) * S);
        if term <= 0.0 {
            Runoff = 0.0;
            Infl = precipitation;
        } else {
            Runoff = term.powf(2.0) / (precipitation + (1.0 - (5.0 / 100.0)) * S);
            Infl = precipitation - Runoff;
        }
    } else {
        Runoff = 0.0;
        Infl = precipitation;
    }
    (Runoff, Infl, NewCond_DaySubmerged)
}

#[cfg(test)]
mod tests {
    use crate::config::{FieldMngt, FieldMngtSpec};
    use crate::solution::drainage::tests::loam;

    #[test]
    fn no_rain_no_runoff() {
        let mut p = loam(12);
        let th = p.th_fc.clone();
        let f = FieldMngt::from(&FieldMngtSpec::default());
        let (ro, infl, ds) = super::rainfall_partition(0.0, &th, 3.0, &f, 61.0, 1.0, 0.3, 12, &mut p);
        assert_eq!((ro, infl, ds), (0.0, 0.0, 0.0));
    }

    #[test]
    fn heavy_rain_splits_and_conserves() {
        let mut p = loam(12);
        let th = p.th_fc.clone();
        let f = FieldMngt::from(&FieldMngtSpec::default());
        let (ro, infl, _) = super::rainfall_partition(80.0, &th, 0.0, &f, 61.0, 1.0, 0.3, 12, &mut p);
        assert!(ro > 0.0 && infl > 0.0);
        assert!((ro + infl - 80.0).abs() < 1e-12);
    }

    #[test]
    fn bunds_hold_everything() {
        let mut p = loam(12);
        let th = p.th_fc.clone();
        let f = FieldMngt::from(&FieldMngtSpec { bunds: true, z_bund: 0.2, ..FieldMngtSpec::default() });
        let (ro, infl, ds) = super::rainfall_partition(80.0, &th, 2.0, &f, 61.0, 1.0, 0.3, 12, &mut p);
        assert_eq!((ro, infl, ds), (0.0, 80.0, 2.0));
    }
}
