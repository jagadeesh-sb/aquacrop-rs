//! solution/infiltration.py

#![allow(non_snake_case)]

use crate::soil::SoilProfile;

/// Infiltrate rain and irrigation into the profile, storing any excess at
/// the surface (bunds) or losing it as runoff.
///
/// Returns `(th, SurfaceStorage, DeepPerc, RunoffTot, Infl, FluxOut)`.
#[allow(clippy::too_many_arguments)]
pub fn infiltration(
    prof: &SoilProfile,
    NewCond_SurfaceStorage: f64,
    NewCond_th_fc_Adj: &[f64],
    NewCond_th: &[f64],
    Infl: f64,
    Irr: f64,
    IrrMngt_AppEff: f64,
    FieldMngt_Bunds: bool,
    FieldMngt_zBund: f64,
    FluxOut: Vec<f64>,
    DeepPerc0: f64,
    Runoff0: f64,
    growing_season: bool,
) -> (Vec<f64>, f64, f64, f64, f64, Vec<f64>) {
    let mut FluxOut = FluxOut;
    let mut NewCond_SurfaceStorage = NewCond_SurfaceStorage;
    let InitCond_SurfaceStorage = NewCond_SurfaceStorage;
    let InitCond_th_fc_Adj = NewCond_th_fc_Adj;
    let InitCond_th = NewCond_th;
    let mut thnew = NewCond_th.to_vec();
    let Soil_nComp = thnew.len();
    // Python's max(Infl, 0.) keeps Infl unless 0 is larger.
    let mut Infl = if 0.0 > Infl { 0.0 } else { Infl };
    if growing_season {
        Infl += Irr * (IrrMngt_AppEff / 100.0);
    }
    assert!(Infl >= 0.0, "infiltration must not be negative");

    let mut ToStore;
    let mut RunoffIni;
    if FieldMngt_Bunds && FieldMngt_zBund > 0.001 {
        let InflTot = Infl + NewCond_SurfaceStorage;
        if InflTot > 0.0 {
            if InflTot > prof.Ksat[0] {
                ToStore = prof.Ksat[0];
                NewCond_SurfaceStorage = InflTot - prof.Ksat[0];
            } else {
                ToStore = InflTot;
                NewCond_SurfaceStorage = 0.0;
            }
            if NewCond_SurfaceStorage > FieldMngt_zBund {
                RunoffIni = NewCond_SurfaceStorage - FieldMngt_zBund;
                NewCond_SurfaceStorage = FieldMngt_zBund;
            } else {
                RunoffIni = 0.0;
            }
        } else {
            ToStore = 0.0;
            RunoffIni = 0.0;
        }
    } else {
        // Deviation: with bunds of zero height (z_bund <= 0.001) Python leaves
        // ToStore and RunoffIni unset and fails; treat the field as unbunded,
        // as rainfall_partition already does.
        if Infl > prof.Ksat[0] {
            ToStore = prof.Ksat[0];
            RunoffIni = Infl - prof.Ksat[0];
        } else {
            ToStore = Infl;
            RunoffIni = 0.0;
        }
        NewCond_SurfaceStorage = 0.0;
        RunoffIni += InitCond_SurfaceStorage;
    }

    let mut ii: i64 = -1;
    let mut Runoff = 0.0;
    let DeepPerc;
    if ToStore > 0.0 {
        while (ToStore > 0.0) && (ii < Soil_nComp as i64 - 1) {
            ii += 1;
            let i = ii as usize;
            let dthdtS = prof.tau[i] * (prof.th_s[i] - prof.th_fc[i]);
            let factor = prof.Ksat[i] / (dthdtS * 1000.0 * prof.dz[i]);
            let mut dthdt0 = ToStore / (1000.0 * prof.dz[i]);
            let mut theta0;
            if dthdt0 < dthdtS {
                if dthdt0 <= 0.0 {
                    theta0 = InitCond_th_fc_Adj[i];
                } else {
                    let A = 1.0
                        + ((dthdt0 * ((prof.th_s[i] - prof.th_fc[i]).exp() - 1.0))
                            / (prof.tau[i] * (prof.th_s[i] - prof.th_fc[i])));
                    theta0 = prof.th_fc[i] + A.ln();
                }
                if theta0 > prof.th_s[i] {
                    theta0 = prof.th_s[i];
                } else if theta0 <= InitCond_th_fc_Adj[i] {
                    theta0 = InitCond_th_fc_Adj[i];
                    dthdt0 = 0.0;
                }
            } else {
                theta0 = prof.th_s[i];
                dthdt0 = dthdtS;
            }
            let mut drainmax = factor * dthdt0 * 1000.0 * prof.dz[i];
            let drainage = drainmax + FluxOut[i];
            if drainage > prof.Ksat[i] {
                drainmax = prof.Ksat[i] - FluxOut[i];
            }
            let diff = theta0 - InitCond_th[i];
            if diff > 0.0 {
                thnew[i] += ToStore / (1000.0 * prof.dz[i]);
                if thnew[i] > theta0 {
                    ToStore = (thnew[i] - theta0) * 1000.0 * prof.dz[i];
                    thnew[i] = theta0;
                } else {
                    ToStore = 0.0;
                }
            }
            FluxOut[i] += ToStore;
            let mut excess = ToStore - drainmax;
            if excess < 0.0 {
                excess = 0.0;
            }
            ToStore -= excess;
            if excess > 0.0 {
                let mut precomp = i + 1;
                while (excess > 0.0) && (precomp != 0) {
                    precomp -= 1;
                    FluxOut[precomp] -= excess;
                    thnew[precomp] += excess / (prof.dz[precomp] * 1000.0);
                    if thnew[precomp] > prof.th_s[precomp] {
                        excess = (thnew[precomp] - prof.th_s[precomp]) * 1000.0 * prof.dz[precomp];
                        thnew[precomp] = prof.th_s[precomp];
                    } else {
                        excess = 0.0;
                    }
                }
                if excess > 0.0 {
                    Runoff += excess;
                }
            }
        }
        DeepPerc = ToStore;
    } else {
        DeepPerc = 0.0;
        Runoff = 0.0;
    }
    Runoff += RunoffIni;
    if Runoff > RunoffIni && FieldMngt_Bunds && FieldMngt_zBund > 0.001 {
        NewCond_SurfaceStorage += Runoff - RunoffIni;
        if NewCond_SurfaceStorage > FieldMngt_zBund {
            Runoff = RunoffIni + (NewCond_SurfaceStorage - FieldMngt_zBund);
            NewCond_SurfaceStorage = FieldMngt_zBund;
        } else {
            Runoff = RunoffIni;
        }
    }
    let NewCond_th = thnew;
    let DeepPerc = DeepPerc + DeepPerc0;
    let Infl = Infl - Runoff;
    let RunoffTot = Runoff + Runoff0;
    (NewCond_th, NewCond_SurfaceStorage, DeepPerc, RunoffTot, Infl, FluxOut)
}

#[cfg(test)]
mod tests {
    use crate::solution::drainage::tests::loam;

    #[test]
    fn water_balance_closes() {
        let p = loam(12);
        let th0 = p.th_fc.clone();
        let flux = vec![0.0; 12];
        let (th, ss, dp, ro, infl, _) =
            super::infiltration(&p, 0.0, &p.th_fc_Adj, &th0, 30.0, 0.0, 100.0, false, 0.0, flux, 0.0, 0.0, true);
        let stored: f64 = th.iter().zip(&th0).map(|(a, b)| (a - b) * 100.0).sum();
        assert_eq!(ss, 0.0);
        assert!((stored + dp - infl).abs() < 1e-9);
        assert!((infl + ro - 30.0).abs() < 1e-9);
    }

    #[test]
    fn nothing_to_infiltrate() {
        let p = loam(4);
        let th0 = p.th_fc.clone();
        let (th, ss, dp, ro, infl, _) =
            super::infiltration(&p, 0.0, &p.th_fc_Adj, &th0, 0.0, 0.0, 100.0, false, 0.0, vec![0.0; 4], 1.5, 2.5, true);
        assert_eq!(th, th0);
        assert_eq!((ss, dp, ro, infl), (0.0, 1.5, 2.5, 0.0));
    }
}
