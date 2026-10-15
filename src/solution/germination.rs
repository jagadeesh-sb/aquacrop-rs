//! solution/germination.py

#![allow(non_snake_case)]

use crate::round::np_round;
use crate::soil::SoilProfile;
use crate::state::InitCond;

/// Germination: the crop emerges once the top soil holds enough water;
/// until then development is delayed day by day.
pub fn germination(
    cond: &mut InitCond,
    Soil_zGerm: f64,
    prof: &SoilProfile,
    Crop_GermThr: f64,
    Crop_PlantMethod: f64,
    gdd: f64,
    growing_season: bool,
) {
    if growing_season {
        if !cond.germination {
            let comp_sto = prof.dzsum.iter().position(|&d| d >= Soil_zGerm).expect("germination depth below the soil profile");
            let mut Wr = 0.0;
            let mut WrFC = 0.0;
            let mut WrWP = 0.0;
            for ii in 0..comp_sto + 1 {
                let factor = if prof.dzsum[ii] > Soil_zGerm { 1.0 - ((prof.dzsum[ii] - Soil_zGerm) / prof.dz[ii]) } else { 1.0 };
                // numpy floats: numpy's rounding.
                Wr += np_round(factor * 1000.0 * cond.th[ii] * prof.dz[ii], 3);
                WrFC += np_round(factor * 1000.0 * prof.th_fc[ii] * prof.dz[ii], 3);
                WrWP += np_round(factor * 1000.0 * prof.th_wp[ii] * prof.dz[ii], 3);
            }
            if Wr < 0.0 {
                Wr = 0.0;
            }
            let WcProp = 1.0 - ((WrFC - Wr) / (WrFC - WrWP));
            if WcProp >= Crop_GermThr {
                cond.germination = true;
                // Python: `Crop_PlantMethod == True`, true for PlantMethod 1.
                cond.protected_seed = Crop_PlantMethod == 1.0;
            } else {
                cond.delayed_cds += 1.0;
                cond.delayed_gdds += gdd;
                cond.protected_seed = false;
            }
        }
    } else {
        cond.germination = false;
        cond.protected_seed = false;
        cond.delayed_cds = 0.0;
        cond.delayed_gdds = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use crate::solution::drainage::tests::loam;
    use crate::state::InitCond;

    #[test]
    fn wet_soil_germinates_dry_soil_delays() {
        let p = loam(12);
        let mut c = InitCond::new(12);
        c.th = p.th_fc.clone();
        super::germination(&mut c, 0.3, &p, 0.2, 1.0, 10.0, true);
        assert!(c.germination && c.protected_seed);

        let mut d = InitCond::new(12);
        d.th = p.th_wp.clone();
        super::germination(&mut d, 0.3, &p, 0.2, 1.0, 10.0, true);
        assert!(!d.germination);
        assert_eq!((d.delayed_cds, d.delayed_gdds), (1.0, 10.0));
    }
}
