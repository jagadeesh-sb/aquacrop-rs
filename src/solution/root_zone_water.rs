//! solution/root_zone_water.py

#![allow(non_snake_case)]

use crate::round::{np_round, py_round};
use crate::soil::SoilProfile;
use crate::state::RootZone;

/// Water in the root zone and in the top soil: actual, saturated, field
/// capacity, wilting point, air-dry and aeration-stress storage.
pub fn root_zone_water(prof: &SoilProfile, InitCond_Zroot: f64, InitCond_th: &[f64], Soil_zTop: f64, Crop_Zmin: f64, Crop_Aer: f64) -> RootZone {
    // np.maximum gives a numpy float, so numpy's rounding.
    let rootdepth = np_round(InitCond_Zroot.max(Crop_Zmin), 2);
    let comp_sto = prof.dzsum.iter().position(|&d| d >= rootdepth).expect("root zone deeper than the soil profile");
    let mut WrAct = 0.0;
    let mut WrS = 0.0;
    let mut WrFC = 0.0;
    let mut WrWP = 0.0;
    let mut WrDry = 0.0;
    let mut WrAer = 0.0;
    for ii in 0..comp_sto + 1 {
        let factor = if prof.dzsum[ii] > rootdepth { 1.0 - ((prof.dzsum[ii] - rootdepth) / prof.dz[ii]) } else { 1.0 };
        WrAct += np_round(factor * 1000.0 * InitCond_th[ii] * prof.dz[ii], 2);
        WrS += np_round(factor * 1000.0 * prof.th_s[ii] * prof.dz[ii], 2);
        WrFC += np_round(factor * 1000.0 * prof.th_fc[ii] * prof.dz[ii], 2);
        WrWP += np_round(factor * 1000.0 * prof.th_wp[ii] * prof.dz[ii], 2);
        WrDry += np_round(factor * 1000.0 * prof.th_dry[ii] * prof.dz[ii], 2);
        WrAer += np_round(factor * 1000.0 * (prof.th_s[ii] - (Crop_Aer / 100.0)) * prof.dz[ii], 2);
    }
    if WrAct < 0.0 {
        WrAct = 0.0;
    }
    let TAW_Rz = (WrFC - WrWP).max(0.0);
    let Dr_Rz = (WrFC - WrAct).min(TAW_Rz);
    let thRZ_Act = WrAct / (rootdepth * 1000.0);
    let thRZ_S = WrS / (rootdepth * 1000.0);
    let thRZ_FC = WrFC / (rootdepth * 1000.0);
    let thRZ_WP = WrWP / (rootdepth * 1000.0);
    let thRZ_Dry = WrDry / (rootdepth * 1000.0);
    let thRZ_Aer = WrAer / (rootdepth * 1000.0);
    let (Dr_Zt, TAW_Zt);
    if rootdepth > Soil_zTop {
        // Soil_zTop is a Python float: Python's rounding.
        let ztopdepth = py_round(Soil_zTop, 2);
        let comp_sto = prof.dzsum.iter().filter(|&&d| d <= ztopdepth).count();
        let mut WrAct_Zt = 0.0;
        let mut WrFC_Zt = 0.0;
        let mut WrWP_Zt = 0.0;
        assert!(comp_sto > 0);
        for ii in 0..comp_sto {
            let factor = if prof.dzsum[ii] > ztopdepth { 1.0 - ((prof.dzsum[ii] - ztopdepth) / prof.dz[ii]) } else { 1.0 };
            WrAct_Zt += factor * 1000.0 * InitCond_th[ii] * prof.dz[ii];
            WrFC_Zt += factor * 1000.0 * prof.th_fc[ii] * prof.dz[ii];
            WrWP_Zt += factor * 1000.0 * prof.th_wp[ii] * prof.dz[ii];
        }
        if WrAct_Zt < 0.0 {
            WrAct_Zt = 0.0;
        }
        TAW_Zt = (WrFC_Zt - WrWP_Zt).max(0.0);
        Dr_Zt = (WrFC_Zt - WrAct_Zt).min(TAW_Zt);
    } else {
        Dr_Zt = Dr_Rz;
        TAW_Zt = TAW_Rz;
    }
    RootZone { WrAct, Dr_Zt, Dr_Rz, TAW_Zt, TAW_Rz, thRZ_Act, thRZ_S, thRZ_FC, thRZ_WP, thRZ_Dry, thRZ_Aer }
}
