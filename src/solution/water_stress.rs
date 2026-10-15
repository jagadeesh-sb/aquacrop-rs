//! solution/water_stress.py

#![allow(non_snake_case)]

use crate::state::Ksw;

/// np.maximum: propagates NaN, unlike f64::max.
fn np_maximum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a >= b {
        a
    } else {
        b
    }
}

/// np.minimum: propagates NaN, unlike f64::min.
fn np_minimum(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a <= b {
        a
    } else {
        b
    }
}

/// Water stress coefficients for canopy expansion, stomatal closure,
/// senescence and pollination.
pub fn water_stress(
    Crop_p_up: &[f64; 4],
    Crop_p_lo: &[f64; 4],
    Crop_ETadj: f64,
    Crop_beta: f64,
    Crop_fshape_w: &[f64; 4],
    InitCond_tEarlySen: f64,
    Dr: f64,
    taw: f64,
    et0: f64,
    beta: bool,
) -> Ksw {
    let nstress = 4;
    // np.ones(nstress) * Crop_p_up
    let mut p_up = [0.0f64; 4];
    let mut p_lo = [0.0f64; 4];
    for ii in 0..nstress {
        p_up[ii] = 1.0 * Crop_p_up[ii];
        p_lo[ii] = 1.0 * Crop_p_lo[ii];
    }
    if Crop_ETadj == 1.0 {
        for ii in 0..3 {
            p_up[ii] = p_up[ii] + (0.04 * (5.0 - et0)) * ((10.0 - 9.0 * p_up[ii]).log10());
            p_lo[ii] = p_lo[ii] + (0.04 * (5.0 - et0)) * ((10.0 - 9.0 * p_lo[ii]).log10());
        }
    }
    if beta && (InitCond_tEarlySen > 0.0) {
        p_up[2] = p_up[2] * (1.0 - Crop_beta / 100.0);
    }
    for ii in 0..4 {
        p_up[ii] = np_maximum(p_up[ii], 0.0);
    }
    for ii in 0..4 {
        p_lo[ii] = np_maximum(p_lo[ii], 0.0);
    }
    for ii in 0..4 {
        p_up[ii] = np_minimum(p_up[ii], 1.0);
    }
    for ii in 0..4 {
        p_lo[ii] = np_minimum(p_lo[ii], 1.0);
    }
    let mut Drel = [0.0f64; 4];
    for ii in 0..nstress {
        if Dr <= (p_up[ii] * taw) {
            Drel[ii] = 0.0;
        } else if (Dr > (p_up[ii] * taw)) && (Dr < (p_lo[ii] * taw)) {
            Drel[ii] = 1.0 - ((p_lo[ii] - (Dr / taw)) / (p_lo[ii] - p_up[ii]));
        } else if Dr >= (p_lo[ii] * taw) {
            Drel[ii] = 1.0;
        }
    }
    let mut Ks = [1.0f64; 3];
    for ii in 0..3 {
        Ks[ii] = 1.0 - (((Drel[ii] * Crop_fshape_w[ii]).exp() - 1.0) / (Crop_fshape_w[ii].exp() - 1.0));
    }
    Ksw { exp: Ks[0], sto: Ks[1], sen: Ks[2], pol: 1.0 - Drel[3], sto_lin: 1.0 - Drel[1] }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_depletion_no_stress() {
        let p_up = [0.2, 0.65, 0.7, 0.85];
        let p_lo = [0.6, 1.0, 1.0, 1.0];
        let fw = [3.0, 3.0, 3.0, 3.0];
        let k = water_stress(&p_up, &p_lo, 1.0, 12.0, &fw, 0.0, 0.0, 100.0, 5.0, true);
        assert_eq!((k.exp, k.sto, k.sen, k.pol, k.sto_lin), (1.0, 1.0, 1.0, 1.0, 1.0));
        // Fully depleted: every coefficient drops to zero.
        let k = water_stress(&p_up, &p_lo, 1.0, 12.0, &fw, 0.0, 100.0, 100.0, 5.0, true);
        assert!(k.exp.abs() < 1e-15 && k.sto.abs() < 1e-15 && k.pol == 0.0 && k.sto_lin == 0.0);
    }
}
