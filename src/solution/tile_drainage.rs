//! solution/tile_drainage.py

#![allow(non_snake_case)]

use crate::soil::SoilProfile;

/// Remove water held above `drain_target` (default: field capacity) in the
/// compartments whose midpoint is at or below `drain_depth` (m), at the
/// fractional rate `drain_coeff` per day. Returns the new water contents and
/// the lateral outflow (mm).
pub fn tile_drainage(prof: &SoilProfile, th: &[f64], drain_depth: f64, drain_target: Option<f64>, drain_coeff: f64) -> (Vec<f64>, f64) {
    let mut thnew = th.to_vec();
    let mut q = 0.0;
    for ii in 0..th.len() {
        if prof.zMid[ii] < drain_depth {
            continue;
        }
        let tgt = drain_target.unwrap_or(prof.th_fc[ii]);
        if thnew[ii] > tgt {
            let removed = drain_coeff * (thnew[ii] - tgt);
            thnew[ii] -= removed;
            q += removed * prof.dz[ii] * 1000.0;
        }
    }
    (thnew, q)
}

#[cfg(test)]
mod tests {
    use crate::solution::drainage::tests::loam;

    #[test]
    fn drains_only_below_depth() {
        let p = loam(12);
        let th = vec![0.40; 12];
        let (out, q) = super::tile_drainage(&p, &th, 0.6, None, 1.0);
        assert_eq!(out[0], 0.40);
        assert!((out[11] - 0.31).abs() < 1e-15);
        let n_below = p.zMid.iter().filter(|z| **z >= 0.6).count() as f64;
        assert!((q - n_below * 0.09 * 100.0).abs() < 1e-9);
    }
}
