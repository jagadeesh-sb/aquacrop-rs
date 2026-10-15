//! solution/groundwater_inflow.py

#![allow(non_snake_case)]

use crate::soil::SoilProfile;
use crate::state::InitCond;

/// Fill the compartments below a water table that lies inside the profile to
/// saturation; returns that horizontal inflow (mm).
pub fn groundwater_inflow(prof: &SoilProfile, cond: &mut InitCond) -> f64 {
    let mut GwIn = 0.0;
    if cond.wt_in_soil {
        let z_gw = cond.z_gw;
        // wt_in_soil guarantees a compartment midpoint at or below the table.
        let Some(idx) = prof.zMid.iter().position(|&z| z >= z_gw) else {
            return GwIn;
        };
        for ii in idx..prof.Comp.len() {
            if cond.th[ii] < prof.th_s[ii] {
                let dth = prof.th_s[ii] - cond.th[ii];
                cond.set_th(ii, prof.th_s[ii]);
                GwIn += dth * 1000.0 * prof.dz[ii];
            }
        }
    }
    GwIn
}

#[cfg(test)]
mod tests {
    use crate::solution::drainage::tests::loam;
    use crate::state::InitCond;

    #[test]
    fn saturates_below_table() {
        let p = loam(12);
        let mut c = InitCond::new(12);
        c.th = p.th_fc.clone();
        c.wt_in_soil = true;
        c.z_gw = 0.9;
        let gw = super::groundwater_inflow(&p, &mut c);
        assert_eq!(c.th[11], p.th_s[11]);
        assert_eq!(c.th[0], p.th_fc[0]);
        let n = p.zMid.iter().filter(|z| **z >= 0.9).count() as f64;
        assert!((gw - n * 0.15 * 100.0).abs() < 1e-9);
    }
}
