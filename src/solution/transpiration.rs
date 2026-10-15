//! solution/transpiration.py

#![allow(non_snake_case)]

use crate::crop::Crop;
use crate::round::py_round;
use crate::soil::SoilProfile;
use crate::solution::aeration_stress::aeration_stress;
use crate::solution::root_zone_water::root_zone_water;
use crate::solution::water_stress::water_stress;
use crate::state::{Co2State, InitCond};

/// Crop transpiration for one day, and the net irrigation needed by method 4.
///
/// Returns (TrAct, TrPot_NS, TrPot0, IrrNet).
pub fn transpiration(
    Soil_Profile: &SoilProfile,
    Soil_nComp: usize,
    Soil_zTop: f64,
    Crop: &Crop,
    IrrMngt_IrrMethod: i32,
    IrrMngt_NetIrrSMT: f64,
    cond: &mut InitCond,
    et0: f64,
    CO2: &Co2State,
    growing_season: bool,
    gdd: f64,
) -> (f64, f64, f64, f64) {
    let NewCond = cond;
    // `InitCond_th = InitCond.th` is the same numpy array as NewCond.th in
    // Python, so InitCond_th[comp] below always reads NewCond.th[comp].
    let prof = Soil_Profile;
    let TrAct;
    let TrPot0;
    let TrPot_NS;
    let IrrNet;
    if growing_season {
        // Potential transpiration without water stress.
        let mut DAPadj = NewCond.dap - NewCond.delayed_cds;
        if DAPadj > Crop.MaxCanopyCD {
            NewCond.age_days_ns = DAPadj - Crop.MaxCanopyCD;
        }
        let mut Kcb_NS = if NewCond.age_days_ns > 5.0 {
            Crop.Kcb - ((NewCond.age_days_ns - 5.0) * (Crop.fage / 100.0)) * NewCond.ccx_w_ns
        } else {
            Crop.Kcb
        };
        let CO2CurrentConc = CO2.current_concentration;
        let CO2RefConc = CO2.ref_concentration;
        if CO2CurrentConc > CO2RefConc {
            Kcb_NS = Kcb_NS * (1.0 - 0.05 * ((CO2CurrentConc - CO2RefConc) / (550.0 - CO2RefConc)));
        }
        let mut TrPot_NS_ = Kcb_NS * (NewCond.canopy_cover_adj_ns) * et0;
        if NewCond.canopy_cover_ns < NewCond.ccx_w_ns && (NewCond.ccx_w_ns > 0.001) && (NewCond.canopy_cover_ns > 0.001) {
            TrPot_NS_ = TrPot_NS_ * (NewCond.canopy_cover_ns / NewCond.ccx_w_ns).powf(Crop.a_Tr);
        }

        // Potential transpiration with water stress effects on the canopy.
        DAPadj = NewCond.dap - NewCond.delayed_cds;
        if DAPadj > Crop.MaxCanopyCD {
            NewCond.age_days = DAPadj - Crop.MaxCanopyCD;
        }
        let mut Kcb = if NewCond.age_days > 5.0 {
            Crop.Kcb - ((NewCond.age_days - 5.0) * (Crop.fage / 100.0)) * NewCond.ccx_w
        } else {
            Crop.Kcb
        };
        if CO2CurrentConc > CO2RefConc {
            Kcb = Kcb * (1.0 - 0.05 * ((CO2CurrentConc - CO2RefConc) / (550.0 - CO2RefConc)));
        }
        let mut TrPot0_ = Kcb * (NewCond.canopy_cover_adj) * et0;
        if NewCond.canopy_cover < NewCond.ccx_w && (NewCond.ccx_w > 0.001) && (NewCond.canopy_cover > 0.001) {
            TrPot0_ = TrPot0_ * (NewCond.canopy_cover / NewCond.ccx_w).powf(Crop.a_Tr);
        }

        // Cold stress.
        // Deviation: a TrColdStress other than 0 or 1 leaves KsCold unbound in Python; read as no stress.
        let mut KsCold = 1.0;
        if Crop.TrColdStress == 0.0 {
            KsCold = 1.0;
        } else if Crop.TrColdStress == 1.0 {
            if gdd >= Crop.GDD_up {
                KsCold = 1.0;
            } else if gdd <= Crop.GDD_lo {
                KsCold = 0.0;
            } else {
                let KsTr_up: f64 = 1.0;
                let KsTr_lo: f64 = 0.02;
                let fshapeb = (-1.0) * ((((KsTr_lo * KsTr_up) - 0.98 * KsTr_lo) / (0.98 * (KsTr_up - KsTr_lo))).ln());
                let GDDrel = (gdd - Crop.GDD_lo) / (Crop.GDD_up - Crop.GDD_lo);
                KsCold = (KsTr_up * KsTr_lo) / (KsTr_lo + (KsTr_up - KsTr_lo) * (-fshapeb * GDDrel).exp());
                KsCold = KsCold - KsTr_lo * (1.0 - GDDrel);
            }
        }
        TrPot0_ = TrPot0_ * KsCold;
        TrPot_NS_ = TrPot_NS_ * KsCold;

        // Surface ponding.
        let mut TrPot;
        let TrAct0;
        if (NewCond.surface_storage > 0.0) && (NewCond.day_submerged < Crop.LagAer) {
            NewCond.day_submerged = NewCond.day_submerged + 1.0;
            for ii in 0..Soil_nComp {
                NewCond.aer_days_comp[ii] = NewCond.aer_days_comp[ii] + 1.0;
                if NewCond.aer_days_comp[ii] > Crop.LagAer {
                    NewCond.aer_days_comp[ii] = Crop.LagAer;
                }
            }
            let fSub = 1.0 - (NewCond.day_submerged / Crop.LagAer);
            if NewCond.surface_storage > (fSub * TrPot0_) {
                NewCond.surface_storage = NewCond.surface_storage - (fSub * TrPot0_);
                TrAct0 = fSub * TrPot0_;
            } else {
                TrAct0 = 0.0;
            }
            if TrAct0 < (fSub * TrPot0_) {
                TrPot = (fSub * TrPot0_) - TrAct0;
            } else {
                TrPot = 0.0;
            }
        } else {
            TrPot = TrPot0_;
            TrAct0 = 0.0;
        }

        // Water and aeration stress on root uptake.
        let thRZ = root_zone_water(prof, NewCond.z_root, &NewCond.th, Soil_zTop, Crop.Zmin, Crop.Aer);
        let (water_root_depletion, taw);
        if (thRZ.Dr_Rz / thRZ.TAW_Rz) <= (thRZ.Dr_Zt / thRZ.TAW_Zt) {
            water_root_depletion = thRZ.Dr_Rz;
            taw = thRZ.TAW_Rz;
        } else {
            water_root_depletion = thRZ.Dr_Zt;
            taw = thRZ.TAW_Zt;
        }
        let beta = true;
        let water_stress_coef = water_stress(
            &Crop.p_up,
            &Crop.p_lo,
            Crop.ETadj,
            Crop.beta,
            &Crop.fshape_w,
            NewCond.t_early_sen,
            water_root_depletion,
            taw,
            et0,
            beta,
        );
        let (Ksa_Aer, aer_days) = aeration_stress(NewCond.aer_days, Crop.LagAer, &thRZ);
        NewCond.aer_days = aer_days;
        let Ks = water_stress_coef.sto_lin.min(Ksa_Aer);
        if IrrMngt_IrrMethod != 4 {
            TrPot = TrPot * Ks;
        }

        // Maximum extraction per compartment. Python floats here: Python's rounding.
        let rootdepth = py_round(NewCond.z_root.max(Crop.Zmin), 2);
        let comp_sto = (Soil_Profile.dzsum.iter().filter(|&&d| d < rootdepth).count() + 1).min(Soil_nComp);
        let mut RootFact = vec![0.0f64; Soil_nComp];
        for ii in 0..comp_sto {
            if Soil_Profile.dzsum[ii] > rootdepth {
                RootFact[ii] = 1.0 - ((Soil_Profile.dzsum[ii] - rootdepth) / Soil_Profile.dz[ii]);
            } else {
                RootFact[ii] = 1.0;
            }
        }
        let mut SxComp = vec![0.0f64; Soil_nComp];
        if IrrMngt_IrrMethod == 4 {
            for ii in 0..comp_sto {
                SxComp[ii] = (Crop.SxTop + Crop.SxBot) / 2.0;
            }
        } else {
            let mut SxCompBot = Crop.SxTop;
            for ii in 0..comp_sto {
                let SxCompTop = SxCompBot;
                if Soil_Profile.dzsum[ii] <= rootdepth {
                    SxCompBot = Crop.SxBot * NewCond.r_cor
                        + ((Crop.SxTop - Crop.SxBot * NewCond.r_cor) * ((rootdepth - Soil_Profile.dzsum[ii]) / rootdepth));
                } else {
                    SxCompBot = Crop.SxBot * NewCond.r_cor;
                }
                SxComp[ii] = (SxCompTop + SxCompBot) / 2.0;
            }
        }

        // Extract water, compartment by compartment.
        let mut ToExtract = TrPot;
        let mut comp: i64 = -1;
        let mut TrAct_ = 0.0;
        while (ToExtract > 0.0) && (comp < comp_sto as i64 - 1) {
            comp = comp + 1;
            let c = comp as usize;
            let thTAW = prof.th_fc[c] - prof.th_wp[c];
            let p_up_sto = if Crop.ETadj == 1.0 {
                Crop.p_up[1] + (0.04 * (5.0 - et0)) * ((10.0 - 9.0 * Crop.p_up[1]).log10())
            } else {
                // Deviation: Python leaves p_up_sto undefined here (NameError); the
                // unadjusted threshold is used instead.
                Crop.p_up[1]
            };
            let thCrit = prof.th_fc[c] - (thTAW * p_up_sto);
            let mut KsComp;
            if NewCond.th[c] >= thCrit {
                KsComp = 1.0;
            } else if NewCond.th[c] > prof.th_wp[c] {
                let Wrel = (prof.th_fc[c] - NewCond.th[c]) / (prof.th_fc[c] - prof.th_wp[c]);
                let pRel = (Wrel - Crop.p_up[1]) / (Crop.p_lo[1] - Crop.p_up[1]);
                if pRel <= 0.0 {
                    KsComp = 1.0;
                } else if pRel >= 1.0 {
                    KsComp = 0.0;
                } else {
                    KsComp = 1.0 - (((pRel * Crop.fshape_w[1]).exp() - 1.0) / (Crop.fshape_w[1].exp() - 1.0));
                }
                if KsComp > 1.0 {
                    KsComp = 1.0;
                } else if KsComp < 0.0 {
                    KsComp = 0.0;
                }
            } else {
                KsComp = 0.0;
            }
            let mut AerComp;
            if NewCond.day_submerged >= Crop.LagAer {
                AerComp = 0.0;
            } else if NewCond.th[c] > (prof.th_s[c] - (Crop.Aer / 100.0)) {
                NewCond.aer_days_comp[c] = NewCond.aer_days_comp[c] + 1.0;
                let fAer;
                if NewCond.aer_days_comp[c] >= Crop.LagAer {
                    NewCond.aer_days_comp[c] = Crop.LagAer;
                    fAer = 0.0;
                } else {
                    fAer = 1.0;
                }
                AerComp = (prof.th_s[c] - NewCond.th[c]) / (prof.th_s[c] - (prof.th_s[c] - (Crop.Aer / 100.0)));
                if AerComp < 0.0 {
                    AerComp = 0.0;
                }
                AerComp = (fAer + (NewCond.aer_days_comp[c] - 1.0) * AerComp) / (fAer + NewCond.aer_days_comp[c] - 1.0);
            } else {
                AerComp = 1.0;
                NewCond.aer_days_comp[c] = 0.0;
            }
            let ThToExtract = (ToExtract / 1000.0) / Soil_Profile.dz[c];
            let mut Sink;
            if IrrMngt_IrrMethod == 4 {
                Sink = AerComp * SxComp[c] * RootFact[c];
            } else if KsComp == AerComp {
                Sink = KsComp * SxComp[c] * RootFact[c];
            } else {
                Sink = KsComp.min(AerComp) * SxComp[c] * RootFact[c];
            }
            if ThToExtract < Sink {
                Sink = ThToExtract;
            }
            let InitCond_th_c = NewCond.th[c];
            if (InitCond_th_c - Sink) < prof.th_dry[c] {
                Sink = InitCond_th_c - prof.th_dry[c];
                if Sink < 0.0 {
                    Sink = 0.0;
                }
            }
            NewCond.set_th(c, InitCond_th_c - Sink);
            ToExtract = ToExtract - (Sink * 1000.0 * prof.dz[c]);
            TrAct_ = TrAct_ + (Sink * 1000.0 * prof.dz[c]);
        }

        // Net irrigation (method 4).
        if (IrrMngt_IrrMethod == 4) && (TrPot > 0.0) {
            let mut IrrNet_ = 0.0;
            let thRZ = root_zone_water(prof, NewCond.z_root, &NewCond.th, Soil_zTop, Crop.Zmin, Crop.Aer);
            NewCond.depletion = thRZ.Dr_Rz;
            NewCond.taw = thRZ.TAW_Rz;
            let mut thCrit = thRZ.thRZ_WP + ((IrrMngt_NetIrrSMT / 100.0) * (thRZ.thRZ_FC - thRZ.thRZ_WP));
            if thRZ.thRZ_Act < thCrit {
                let mut prelayer: i64 = 0;
                for ii in 0..comp_sto {
                    let layeri = Soil_Profile.Layer[ii];
                    if layeri > prelayer {
                        thCrit = prof.th_wp[ii] + ((IrrMngt_NetIrrSMT / 100.0) * (prof.th_fc[ii] - prof.th_wp[ii]));
                        prelayer = layeri;
                    }
                    let dWC = RootFact[ii] * (thCrit - NewCond.th[ii]) * 1000.0 * prof.dz[ii];
                    let v = NewCond.th[ii] + (dWC / (1000.0 * prof.dz[ii]));
                    NewCond.set_th(ii, v);
                    IrrNet_ = IrrNet_ + dWC;
                }
            }
            NewCond.irr_net_cum = NewCond.irr_net_cum + IrrNet_;
            IrrNet = IrrNet_;
        } else if (IrrMngt_IrrMethod == 4) && (TrPot <= 0.0) {
            IrrNet = 0.0;
        } else {
            IrrNet = 0.0;
            NewCond.irr_net_cum = 0.0;
        }

        // Totals and the transpiration ratio.
        TrAct_ = TrAct_ + TrAct0;
        if ((NewCond.canopy_cover - NewCond.cc_prev) > 0.005) && (TrAct_ == 0.0) {
            NewCond.canopy_cover = NewCond.cc_prev;
        }
        if TrPot0_ > 0.0 {
            if TrAct_ < TrPot0_ {
                NewCond.tr_ratio = TrAct_ / TrPot0_;
            } else {
                NewCond.tr_ratio = 1.0;
            }
        } else {
            NewCond.tr_ratio = 1.0;
        }
        if NewCond.tr_ratio < 0.0 {
            NewCond.tr_ratio = 0.0;
        } else if NewCond.tr_ratio > 1.0 {
            NewCond.tr_ratio = 1.0;
        }
        TrAct = TrAct_;
        TrPot0 = TrPot0_;
        TrPot_NS = TrPot_NS_;
    } else {
        TrAct = 0.0;
        TrPot0 = 0.0;
        TrPot_NS = 0.0;
        IrrNet = 0.0;
        NewCond.irr_net_cum = 0.0;
    }
    NewCond.t_pot = TrPot0;
    (TrAct, TrPot_NS, TrPot0, IrrNet)
}
