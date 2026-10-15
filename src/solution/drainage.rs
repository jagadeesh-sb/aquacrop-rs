//! solution/drainage.py

#![allow(non_snake_case)]

use crate::soil::SoilProfile;

/// Redistribution of stored soil water: free drainage down the profile.
///
/// Returns the new water contents, deep percolation (mm) and the flux out of
/// the bottom of each compartment (mm).
pub fn drainage(prof: &SoilProfile, th_init: &[f64], th_fc_Adj_init: &[f64]) -> (Vec<f64>, f64, Vec<f64>) {
    let n = th_init.len();
    let mut thnew = vec![0.0; n];
    let mut FluxOut = vec![0.0; n];
    let mut drainsum = 0.0;
    for ii in 0..n {
        let cth_fc = prof.th_fc[ii];
        let cth_s = prof.th_s[ii];
        let ctau = prof.tau[ii];
        let cdz = prof.dz[ii];
        let cdzsum = prof.dzsum[ii];
        let cKsat = prof.Ksat[ii];
        let mut dthdt;
        if th_init[ii] <= th_fc_Adj_init[ii] {
            dthdt = 0.0;
        } else if th_init[ii] >= cth_s {
            dthdt = ctau * (cth_s - cth_fc);
            if (th_init[ii] - dthdt) < th_fc_Adj_init[ii] {
                dthdt = th_init[ii] - th_fc_Adj_init[ii];
            }
        } else {
            dthdt = ctau * (cth_s - cth_fc) * (((th_init[ii] - cth_fc).exp() - 1.0) / ((cth_s - cth_fc).exp() - 1.0));
            if (th_init[ii] - dthdt) < th_fc_Adj_init[ii] {
                dthdt = th_init[ii] - th_fc_Adj_init[ii];
            }
        }
        let mut draincomp = dthdt * cdz * 1000.0;
        let mut excess = 0.0;
        let prethick = cdzsum - cdz;
        let mut drainmax = dthdt * 1000.0 * prethick;
        let drainability = drainsum <= drainmax;
        if drainability {
            thnew[ii] = th_init[ii] - dthdt;
            drainsum += draincomp;
            if drainsum > cKsat {
                excess = excess + drainsum - cKsat;
                drainsum = cKsat;
            }
        } else {
            dthdt = drainsum / (1000.0 * prethick);
            let mut thX;
            if dthdt <= 0.0 {
                thX = th_fc_Adj_init[ii];
            } else if ctau > 0.0 {
                let A = 1.0 + ((dthdt * ((cth_s - cth_fc).exp() - 1.0)) / (ctau * (cth_s - cth_fc)));
                thX = cth_fc + A.ln();
                if thX < th_fc_Adj_init[ii] {
                    thX = th_fc_Adj_init[ii];
                }
            } else {
                thX = cth_s + 0.01;
            }
            if thX <= cth_s {
                thnew[ii] = th_init[ii] + (drainsum / (1000.0 * cdz));
                if thnew[ii] > thX {
                    drainsum = (thnew[ii] - thX) * 1000.0 * cdz;
                    if thX <= th_fc_Adj_init[ii] {
                        dthdt = 0.0;
                    } else if thX >= cth_s {
                        dthdt = ctau * (cth_s - cth_fc);
                        if (thX - dthdt) < th_fc_Adj_init[ii] {
                            dthdt = thX - th_fc_Adj_init[ii];
                        }
                    } else {
                        dthdt = ctau * (cth_s - cth_fc) * (((thX - cth_fc).exp() - 1.0) / ((cth_s - cth_fc).exp() - 1.0));
                        if (thX - dthdt) < th_fc_Adj_init[ii] {
                            dthdt = thX - th_fc_Adj_init[ii];
                        }
                    }
                    drainsum += dthdt * 1000.0 * cdz;
                    if drainsum > cKsat {
                        excess = excess + drainsum - cKsat;
                        drainsum = cKsat;
                    }
                    thnew[ii] = thX - dthdt;
                } else if thnew[ii] > th_fc_Adj_init[ii] {
                    // The first test can never hold here; kept as in the Python.
                    if thnew[ii] <= th_fc_Adj_init[ii] {
                        dthdt = 0.0;
                    } else if thnew[ii] >= cth_s {
                        dthdt = ctau * (cth_s - cth_fc);
                        if (thnew[ii] - dthdt) < th_fc_Adj_init[ii] {
                            dthdt = thnew[ii] - th_fc_Adj_init[ii];
                        }
                    } else {
                        dthdt = ctau
                            * (cth_s - cth_fc)
                            * (((thnew[ii] - cth_fc).exp() - 1.0) / ((cth_s - cth_fc).exp() - 1.0));
                        if (thnew[ii] - dthdt) < th_fc_Adj_init[ii] {
                            dthdt = thnew[ii] - th_fc_Adj_init[ii];
                        }
                    }
                    thnew[ii] -= dthdt;
                    drainsum = dthdt * 1000.0 * cdz;
                    if drainsum > cKsat {
                        excess = excess + drainsum - cKsat;
                        drainsum = cKsat;
                    }
                } else {
                    drainsum = 0.0;
                }
            } else if thX > cth_s {
                thnew[ii] = th_init[ii] + (drainsum / (1000.0 * cdz));
                if thnew[ii] <= cth_s {
                    if thnew[ii] > th_fc_Adj_init[ii] {
                        if thnew[ii] <= th_fc_Adj_init[ii] {
                            dthdt = 0.0;
                        } else if thnew[ii] >= cth_s {
                            dthdt = ctau * (cth_s - cth_fc);
                            if (thnew[ii] - dthdt) < th_fc_Adj_init[ii] {
                                dthdt = thnew[ii] - th_fc_Adj_init[ii];
                            }
                        } else {
                            dthdt = ctau
                                * (cth_s - cth_fc)
                                * (((thnew[ii] - cth_fc).exp() - 1.0) / ((cth_s - cth_fc).exp() - 1.0));
                            if (thnew[ii] - dthdt) < th_fc_Adj_init[ii] {
                                dthdt = thnew[ii] - th_fc_Adj_init[ii];
                            }
                        }
                        thnew[ii] -= dthdt;
                        drainsum = dthdt * 1000.0 * cdz;
                        if drainsum > cKsat {
                            excess = excess + drainsum - cKsat;
                            drainsum = cKsat;
                        }
                    } else {
                        drainsum = 0.0;
                    }
                } else if thnew[ii] > cth_s {
                    excess = (thnew[ii] - cth_s) * 1000.0 * cdz;
                    if thnew[ii] <= th_fc_Adj_init[ii] {
                        dthdt = 0.0;
                    } else if thnew[ii] >= cth_s {
                        dthdt = ctau * (cth_s - cth_fc);
                        if (thnew[ii] - dthdt) < th_fc_Adj_init[ii] {
                            dthdt = thnew[ii] - th_fc_Adj_init[ii];
                        }
                    } else {
                        dthdt = ctau
                            * (cth_s - cth_fc)
                            * (((thnew[ii] - cth_fc).exp() - 1.0) / ((cth_s - cth_fc).exp() - 1.0));
                        if (thnew[ii] - dthdt) < th_fc_Adj_init[ii] {
                            dthdt = thnew[ii] - th_fc_Adj_init[ii];
                        }
                    }
                    thnew[ii] = cth_s - dthdt;
                    draincomp = dthdt * 1000.0 * cdz;
                    drainmax = dthdt * 1000.0 * prethick;
                    if drainmax > excess {
                        drainmax = excess;
                    }
                    excess -= drainmax;
                    drainsum = draincomp + drainmax;
                    if drainsum > cKsat {
                        excess = excess + drainsum - cKsat;
                        drainsum = cKsat;
                    }
                }
            }
        }
        FluxOut[ii] = drainsum;
        if excess > 0.0 {
            let mut precomp = ii + 1;
            while (excess > 0.0) && (precomp != 0) {
                precomp -= 1;
                if precomp < ii {
                    FluxOut[precomp] -= excess;
                }
                thnew[precomp] += excess / (1000.0 * prof.dz[precomp]);
                if thnew[precomp] > prof.th_s[precomp] {
                    excess = (thnew[precomp] - prof.th_s[precomp]) * 1000.0 * prof.dz[precomp];
                    thnew[precomp] = prof.th_s[precomp];
                } else {
                    excess = 0.0;
                }
            }
        }
    }
    let DeepPerc = drainsum;
    (thnew, DeepPerc, FluxOut)
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::soil::SoilProfile;

    /// A uniform Loam profile of `n` compartments of 0.1 m, for unit tests.
    #[allow(non_snake_case)]
    pub(crate) fn loam(n: usize) -> SoilProfile {
        let dz = vec![0.1; n];
        let dzsum: Vec<f64> = (1..=n).map(|i| crate::round::np_round(i as f64 * 0.1, 2)).collect();
        let z_top: Vec<f64> = dzsum.iter().map(|b| b - 0.1).collect();
        let zMid = z_top.iter().zip(&dzsum).map(|(t, b)| (t + b) / 2.0).collect();
        SoilProfile {
            Comp: (0..n).collect(),
            dz,
            Layer: vec![1; n],
            dzsum: dzsum.clone(),
            th_fc: vec![0.31; n],
            th_s: vec![0.46; n],
            th_wp: vec![0.15; n],
            Ksat: vec![500.0; n],
            Penetrability: vec![100.0; n],
            th_dry: vec![0.075; n],
            tau: vec![0.76; n],
            zBot: dzsum,
            z_top,
            zMid,
            th_fc_Adj: vec![0.31; n],
            aCR: vec![0.0; n],
            bCR: vec![0.0; n],
        }
    }

    #[test]
    fn field_capacity_does_not_drain() {
        let p = loam(12);
        let (th, deep, flux) = super::drainage(&p, &p.th_fc, &p.th_fc_Adj);
        assert_eq!(th, p.th_fc);
        assert_eq!(deep, 0.0);
        assert!(flux.iter().all(|f| *f == 0.0));
    }

    #[test]
    fn saturated_top_drains_and_conserves_water() {
        let p = loam(12);
        let mut th0 = p.th_fc.clone();
        th0[0] = 0.46;
        let (th, deep, _) = super::drainage(&p, &th0, &p.th_fc_Adj);
        let before: f64 = th0.iter().map(|t| t * 100.0).sum();
        let after: f64 = th.iter().map(|t| t * 100.0).sum();
        assert!(th[0] < 0.46);
        assert!((before - after - deep).abs() < 1e-9);
    }
}
