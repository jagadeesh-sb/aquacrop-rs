//! solution/pre_irrigation.py

#![allow(non_snake_case)]

use crate::config::IrrMngt;
use crate::crop::Crop;
use crate::round::py_round;
use crate::soil::SoilProfile;
use crate::state::InitCond;

/// Pre-irrigation on the first day of the season in net irrigation mode
/// (method 4): raise the root zone to the net irrigation threshold. Returns
/// the depth applied (mm).
///
/// This writes into `th` through [`InitCond::set_th`]: on the first day of a
/// season `th` is still the `thini` array in AquaCrop-OSPy, so the raised
/// water contents also become the starting point of the following seasons.
pub fn pre_irrigation(prof: &SoilProfile, crop: &Crop, cond: &mut InitCond, growing_season: bool, irr: &IrrMngt) -> f64 {
    let mut PreIrr = 0.0;
    if growing_season && (irr.irrigation_method == 4) && (cond.dap == 1.0) {
        // z_root and Zmin are Python floats here: Python's rounding.
        let rootdepth = py_round(if crop.Zmin > cond.z_root { crop.Zmin } else { cond.z_root }, 2);
        let compRz = prof.dzsum.iter().position(|&d| d >= rootdepth).expect("root zone deeper than the soil profile");
        for ii in 0..compRz {
            let thCrit = prof.th_wp[ii] + ((irr.NetIrrSMT / 100.0) * (prof.th_fc[ii] - prof.th_wp[ii]));
            if cond.th[ii] < thCrit {
                PreIrr += (thCrit - cond.th[ii]) * 1000.0 * prof.dz[ii];
                cond.set_th(ii, thCrit);
            }
        }
    }
    PreIrr
}

#[cfg(test)]
mod tests {
    use crate::config::IrrMngt;
    use crate::crop::Crop;
    use crate::date::MonthDay;
    use crate::solution::drainage::tests::loam;
    use crate::state::InitCond;

    #[test]
    fn raises_root_zone_and_thini_while_aliased() {
        let p = loam(12);
        let crop = Crop::new("Maize", MonthDay::parse("05/01").unwrap(), None, &Default::default()).unwrap();
        let mut irr = IrrMngt::fallow(10);
        irr.irrigation_method = 4;
        let mut c = InitCond::new(12);
        c.th = p.th_wp.clone();
        c.thini = p.th_wp.clone();
        c.th_is_thini = true;
        c.dap = 1.0;
        c.z_root = crop.Zmin;
        let pre = super::pre_irrigation(&p, &crop, &mut c, true, &irr);
        assert!(pre > 0.0);
        assert!(c.th[0] > p.th_wp[0]);
        assert_eq!(c.th, c.thini);
        c.dap = 2.0;
        assert_eq!(super::pre_irrigation(&p, &crop, &mut c, true, &irr), 0.0);
    }
}
