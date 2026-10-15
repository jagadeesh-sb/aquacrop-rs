//! Soil: AquaCrop-OSPy's `Soil` class (entities/soil.py) and the compartment
//! table it keeps in a pandas DataFrame (`soil.profile`).

#![allow(non_snake_case)]

use crate::error::Error;
use crate::numeric::{np_cumsum, np_sum, pandas_mean};
use crate::round::{np_round, py_round, round0};
use serde::{Deserialize, Serialize};

pub const NO_VALUE: f64 = -999.0;

/// The compartment table, one entry per compartment. NaN marks a cell pandas
/// would hold as NaN (a compartment no layer has reached yet).
#[derive(Clone, Debug, Default)]
pub struct ProfileTable {
    pub comp: Vec<f64>,
    pub layer: Vec<f64>,
    pub dz: Vec<f64>,
    pub dzsum: Vec<f64>,
    pub zBot: Vec<f64>,
    pub z_top: Vec<f64>,
    pub zMid: Vec<f64>,
    pub th_dry: Vec<f64>,
    pub th_wp: Vec<f64>,
    pub th_fc: Vec<f64>,
    pub th_s: Vec<f64>,
    pub Ksat: Vec<f64>,
    pub penetrability: Vec<f64>,
    pub tau: Vec<f64>,
    pub th_fc_Adj: Vec<f64>,
    pub aCR: Vec<f64>,
    pub bCR: Vec<f64>,
}

impl ProfileTable {
    pub fn len(&self) -> usize {
        self.dz.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dz.is_empty()
    }
}

/// A layer given by its hydraulic properties.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LayerSpec {
    /// Layer thickness (m).
    pub thickness: f64,
    #[serde(flatten)]
    pub properties: LayerProperties,
    /// Root penetrability (%).
    #[serde(default = "hundred")]
    pub penetrability: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum LayerProperties {
    /// Water contents (m3/m3) and saturated conductivity (mm/day).
    Hydraulic { th_wp: f64, th_fc: f64, th_s: f64, ksat: f64 },
    /// Sand and clay (%) and organic matter (%), through Saxton and Rawls (2006).
    Texture { sand: f64, clay: f64, organic_matter: f64 },
}

fn hundred() -> f64 {
    100.0
}

/// Inputs of `Soil(...)`. Every field but `soil_type` has AquaCrop-OSPy's default.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct SoilSpec {
    pub soil_type: String,
    pub dz: Vec<f64>,
    pub adj_rew: f64,
    pub rew: f64,
    pub calc_cn: f64,
    pub cn: f64,
    pub z_res: f64,
    pub evap_z_surf: f64,
    pub evap_z_min: f64,
    pub evap_z_max: f64,
    pub kex: f64,
    pub f_evap: f64,
    pub f_wrel_exp: f64,
    pub fwcc: f64,
    pub z_cn: f64,
    pub z_germ: f64,
    pub adj_cn: f64,
    pub fshape_cr: f64,
    pub z_top: f64,
    pub tile_drainage: bool,
    pub drain_depth: Option<f64>,
    pub drain_target: Option<f64>,
    pub drain_coeff: f64,
    /// Layers added after the soil type's own (`soil.add_layer(...)`); a
    /// "custom" soil is built from these alone.
    pub layers: Vec<LayerSpec>,
}

impl Default for SoilSpec {
    fn default() -> Self {
        SoilSpec {
            soil_type: "custom".into(),
            dz: vec![0.1; 12],
            adj_rew: 1.0,
            rew: 9.0,
            calc_cn: 0.0,
            cn: 61.0,
            z_res: NO_VALUE,
            evap_z_surf: 0.04,
            evap_z_min: 0.15,
            evap_z_max: 0.30,
            kex: 1.1,
            f_evap: 4.0,
            f_wrel_exp: 0.4,
            fwcc: 50.0,
            z_cn: 0.3,
            z_germ: 0.3,
            adj_cn: 1.0,
            fshape_cr: 16.0,
            z_top: 0.1,
            tile_drainage: false,
            drain_depth: None,
            drain_target: None,
            drain_coeff: 1.0,
            layers: vec![],
        }
    }
}

pub const SOIL_TYPES: &[&str] = &[
    "custom", "Clay", "ClayLoam", "Default", "Loam", "LoamySand", "Sand", "SandyClay", "SandyClayLoam", "SandyLoam",
    "Silt", "SiltClayLoam", "SiltLoam", "SiltClay", "Paddy", "ac_TunisLocal",
];

#[derive(Clone, Debug)]
pub struct Soil {
    pub Name: String,
    pub tile_drainage: bool,
    pub drain_depth: Option<f64>,
    pub drain_target: Option<f64>,
    pub drain_coeff: f64,
    pub zSoil: f64,
    pub nComp: usize,
    pub nLayer: usize,
    pub adj_rew: f64,
    pub rew: f64,
    pub calc_cn: f64,
    pub cn: f64,
    pub z_res: f64,
    pub evap_z_surf: f64,
    pub evap_z_min: f64,
    pub evap_z_max: f64,
    pub kex: f64,
    pub f_evap: f64,
    pub f_wrel_exp: f64,
    pub fwcc: f64,
    pub z_cn: f64,
    pub z_germ: f64,
    pub adj_cn: f64,
    pub fshape_cr: f64,
    pub z_top: f64,
    pub profile: ProfileTable,
}

impl Soil {
    pub fn new(spec: &SoilSpec) -> Result<Soil, Error> {
        let dz = &spec.dz;
        if dz.is_empty() || dz.iter().any(|d| !(*d > 0.0)) {
            return Err(Error::Input("soil compartments (dz) must be a non-empty list of positive thicknesses".into()));
        }
        let mut s = Soil {
            Name: spec.soil_type.clone(),
            tile_drainage: spec.tile_drainage,
            drain_depth: spec.drain_depth,
            drain_target: spec.drain_target,
            drain_coeff: spec.drain_coeff,
            // Python's built-in sum, left to right.
            zSoil: dz.iter().fold(0.0, |a, b| a + b),
            nComp: dz.len(),
            nLayer: 0,
            adj_rew: spec.adj_rew,
            rew: spec.rew,
            calc_cn: spec.calc_cn,
            cn: spec.cn,
            z_res: spec.z_res,
            evap_z_surf: spec.evap_z_surf,
            evap_z_min: spec.evap_z_min,
            evap_z_max: spec.evap_z_max,
            kex: spec.kex,
            f_evap: spec.f_evap,
            f_wrel_exp: spec.f_wrel_exp,
            fwcc: spec.fwcc,
            z_cn: spec.z_cn,
            z_germ: spec.z_germ,
            adj_cn: spec.adj_cn,
            fshape_cr: spec.fshape_cr,
            z_top: spec.z_top.max(dz[0]),
            profile: ProfileTable::default(),
        };
        let total: f64 = dz.iter().fold(0.0, |a, b| a + b);
        let preset = |s: &mut Soil, cn: f64, rew: f64| {
            s.cn = cn;
            s.calc_cn = 0.0;
            s.rew = rew;
        };
        match spec.soil_type.as_str() {
            "custom" => s.create_df(dz),
            "Clay" => { preset(&mut s, 77.0, 14.0); s.create_df(dz); s.add_layer(total, 0.39, 0.54, 0.55, 35.0, 100.0); }
            "ClayLoam" => { preset(&mut s, 72.0, 11.0); s.create_df(dz); s.add_layer(total, 0.23, 0.39, 0.5, 125.0, 100.0); }
            "Default" => { preset(&mut s, 61.0, 9.0); s.create_df(dz); s.add_layer(total, 0.1, 0.3, 0.5, 500.0, 100.0); }
            "Loam" => { preset(&mut s, 61.0, 9.0); s.create_df(dz); s.add_layer(total, 0.15, 0.31, 0.46, 500.0, 100.0); }
            "LoamySand" => { preset(&mut s, 46.0, 5.0); s.create_df(dz); s.add_layer(total, 0.08, 0.16, 0.38, 2200.0, 100.0); }
            "Sand" => { preset(&mut s, 46.0, 4.0); s.create_df(dz); s.add_layer(total, 0.06, 0.13, 0.36, 3000.0, 100.0); }
            "SandyClay" => { preset(&mut s, 77.0, 10.0); s.create_df(dz); s.add_layer(total, 0.27, 0.39, 0.5, 35.0, 100.0); }
            "SandyClayLoam" => { preset(&mut s, 72.0, 9.0); s.create_df(dz); s.add_layer(total, 0.20, 0.32, 0.47, 225.0, 100.0); }
            "SandyLoam" => { preset(&mut s, 46.0, 7.0); s.create_df(dz); s.add_layer(total, 0.10, 0.22, 0.41, 1200.0, 100.0); }
            "Silt" => { preset(&mut s, 61.0, 11.0); s.create_df(dz); s.add_layer(total, 0.09, 0.33, 0.43, 500.0, 100.0); }
            "SiltClayLoam" => { preset(&mut s, 72.0, 13.0); s.create_df(dz); s.add_layer(total, 0.23, 0.44, 0.52, 150.0, 100.0); }
            "SiltLoam" => { preset(&mut s, 61.0, 11.0); s.create_df(dz); s.add_layer(total, 0.13, 0.33, 0.46, 575.0, 100.0); }
            "SiltClay" => { preset(&mut s, 72.0, 14.0); s.create_df(dz); s.add_layer(total, 0.32, 0.50, 0.54, 100.0, 100.0); }
            "Paddy" => {
                preset(&mut s, 77.0, 10.0);
                s.create_df(dz);
                s.add_layer(0.5, 0.32, 0.50, 0.54, 15.0, 100.0);
                s.add_layer(1.5, 0.39, 0.54, 0.55, 2.0, 100.0);
            }
            "ac_TunisLocal" => {
                preset(&mut s, 72.0, 11.0);
                let mut tunis = vec![0.1; 6];
                tunis.extend([0.15; 5]);
                tunis.push(0.2);
                s.create_df(&tunis);
                s.add_layer(0.3, 0.24, 0.40, 0.50, 155.0, 100.0);
                s.add_layer(1.7, 0.11, 0.33, 0.46, 500.0, 100.0);
            }
            other => {
                return Err(Error::Input(format!("unknown soil type {other:?}; use one of: {}", SOIL_TYPES.join(", "))))
            }
        }
        for l in &spec.layers {
            match l.properties {
                LayerProperties::Hydraulic { th_wp, th_fc, th_s, ksat } => {
                    s.add_layer(l.thickness, th_wp, th_fc, th_s, ksat, l.penetrability)
                }
                LayerProperties::Texture { sand, clay, organic_matter } => {
                    s.add_layer_from_texture(l.thickness, sand, clay, organic_matter, l.penetrability)
                }
            }
        }
        if s.nLayer == 0 {
            return Err(Error::Input("a custom soil needs at least one layer".into()));
        }
        Ok(s)
    }

    fn create_df(&mut self, dz: &[f64]) {
        let n = dz.len();
        let dzsum: Vec<f64> = np_cumsum(dz).iter().map(|v| np_round(*v, 2)).collect();
        let zBot = dzsum.clone();
        let z_top: Vec<f64> = zBot.iter().zip(dz).map(|(b, d)| b - d).collect();
        let zMid = z_top.iter().zip(&zBot).map(|(t, b)| (t + b) / 2.0).collect();
        let nan = vec![f64::NAN; n];
        self.profile = ProfileTable {
            comp: (0..n).map(|i| i as f64).collect(),
            layer: nan.clone(),
            dz: dz.to_vec(),
            dzsum,
            zBot,
            z_top,
            zMid,
            th_dry: nan.clone(),
            th_wp: nan.clone(),
            th_fc: nan.clone(),
            th_s: nan.clone(),
            Ksat: nan.clone(),
            penetrability: nan.clone(),
            tau: nan.clone(),
            th_fc_Adj: nan.clone(),
            aCR: nan.clone(),
            bCR: nan,
        };
    }

    /// Saxton and Rawls (2006) pedotransfer functions.
    pub fn calculate_soil_hydraulic_properties(Sand: f64, Clay: f64, OrgMat: f64, DF: f64) -> (f64, f64, f64, f64) {
        let Pred_thWP = -(0.024 * Sand) + (0.487 * Clay) + (0.006 * OrgMat) + (0.005 * Sand * OrgMat)
            - (0.013 * Clay * OrgMat)
            + (0.068 * Sand * Clay)
            + 0.031;
        let th_wp = Pred_thWP + (0.14 * Pred_thWP) - 0.02;
        let Pred_thFC = -(0.251 * Sand) + (0.195 * Clay) + (0.011 * OrgMat) + (0.006 * Sand * OrgMat)
            - (0.027 * Clay * OrgMat)
            + (0.452 * Sand * Clay)
            + 0.299;
        let PredAdj_thFC = Pred_thFC + ((1.283 * Pred_thFC.powf(2.0)) - (0.374 * Pred_thFC) - 0.015);
        let Pred_thS33 = (0.278 * Sand) + (0.034 * Clay) + (0.022 * OrgMat)
            - (0.018 * Sand * OrgMat)
            - (0.027 * Clay * OrgMat)
            - (0.584 * Sand * Clay)
            + 0.078;
        let PredAdj_thS33 = Pred_thS33 + ((0.636 * Pred_thS33) - 0.107);
        let Pred_thS = (PredAdj_thFC + PredAdj_thS33) + ((-0.097 * Sand) + 0.043);
        let pN = (1.0 - Pred_thS) * 2.65;
        let pDF = pN * DF;
        let PorosComp = (1.0 - (pDF / 2.65)) - (1.0 - (pN / 2.65));
        let PorosCompOM = 1.0 - (pDF / 2.65);
        let DensAdj_thFC = PredAdj_thFC + (0.2 * PorosComp);
        let DensAdj_thS = PorosCompOM;
        let th_fc = DensAdj_thFC;
        let th_s = DensAdj_thS;
        let lmbda = 1.0 / ((1500f64.ln() - 33f64.ln()) / (th_fc.ln() - th_wp.ln()));
        let Ksat = (1930.0 * (th_s - th_fc).powf(3.0 - lmbda)) * 24.0;
        let th_wp = round0(1000.0 * th_wp) / 1000.0;
        let th_fc = round0(1000.0 * th_fc) / 1000.0;
        let th_s = round0(1000.0 * th_s) / 1000.0;
        let Ksat = round0(10.0 * Ksat) / 10.0;
        (th_wp, th_fc, th_s, Ksat)
    }

    pub fn add_layer_from_texture(&mut self, thickness: f64, Sand: f64, Clay: f64, OrgMat: f64, penetrability: f64) {
        let (th_wp, th_fc, th_s, Ksat) = Soil::calculate_soil_hydraulic_properties(Sand / 100.0, Clay / 100.0, OrgMat, 1.0);
        self.add_layer(thickness, th_wp, th_fc, th_s, Ksat, penetrability);
    }

    pub fn add_layer(&mut self, thickness: f64, thWP: f64, thFC: f64, thS: f64, Ksat: f64, penetrability: f64) {
        self.nLayer += 1;
        let p = &mut self.profile;
        let n = p.len();
        // len(profile.dropna().Layer.unique()): layers among rows with no NaN.
        let mut seen: Vec<f64> = vec![];
        for i in 0..n {
            let row = [p.layer[i], p.th_dry[i], p.th_wp[i], p.th_fc[i], p.th_s[i], p.Ksat[i], p.penetrability[i], p.tau[i]];
            if row.iter().all(|v| !v.is_nan()) && !seen.contains(&p.layer[i]) {
                seen.push(p.layer[i]);
            }
        }
        let new_layer = (seen.len() + 1) as f64;
        if new_layer == 1.0 {
            let t = py_round(thickness, 2);
            for i in 0..n {
                if t >= np_round(p.dzsum[i], 2) {
                    p.layer[i] = new_layer;
                }
            }
        } else {
            let last = (0..n).rev().find(|&i| p.layer[i] == new_layer - 1.0).map(|i| p.dzsum[i]);
            if let Some(last) = last {
                for i in 0..n {
                    if thickness + last >= p.dzsum[i] && p.layer[i].is_nan() {
                        p.layer[i] = new_layer;
                    }
                }
            }
        }
        let mut tau = py_round(0.0866 * Ksat.powf(0.35), 2);
        if tau > 1.0 {
            tau = 1.0;
        } else if tau < 0.0 {
            tau = 0.0;
        }
        for i in 0..n {
            if p.layer[i] == new_layer {
                p.th_dry[i] = thWP / 2.0;
                p.th_wp[i] = thWP;
                p.th_fc[i] = thFC;
                p.th_s[i] = thS;
                p.Ksat[i] = Ksat;
                p.penetrability[i] = penetrability;
                p.tau[i] = tau;
            }
        }
    }

    /// Soil.fill_nan: forward-fill the rows below the last layer.
    pub fn fill_nan(&mut self) {
        let p = &mut self.profile;
        let n = p.len();
        for col in [
            &mut p.layer, &mut p.th_dry, &mut p.th_wp, &mut p.th_fc, &mut p.th_s, &mut p.Ksat, &mut p.penetrability,
            &mut p.tau, &mut p.dz, &mut p.dzsum, &mut p.zBot, &mut p.z_top, &mut p.zMid,
        ] {
            for i in 1..n {
                if col[i].is_nan() {
                    col[i] = col[i - 1];
                }
            }
        }
        for d in p.dz.iter_mut() {
            *d = np_round(*d, 2);
        }
        p.dzsum = np_cumsum(&p.dz).iter().map(|v| np_round(*v, 2)).collect();
        self.zSoil = np_round(np_sum(&p.dz), 2);
        self.nComp = n;
    }

    /// Distinct layer numbers, ascending (`profile.groupby("Layer")`).
    pub fn layers(&self) -> Vec<f64> {
        let mut l: Vec<f64> = self.profile.layer.iter().copied().filter(|v| !v.is_nan()).collect();
        l.sort_by(|a, b| a.partial_cmp(b).unwrap());
        l.dedup();
        l
    }

    /// One column averaged per layer, as `profile.groupby("Layer").mean()` does.
    pub fn layer_mean(&self, layer: f64, column: &[f64]) -> f64 {
        let vals: Vec<f64> =
            (0..self.profile.len()).filter(|&i| self.profile.layer[i] == layer).map(|i| column[i]).collect();
        pandas_mean(&vals)
    }

    /// Soil.add_capillary_rise_params.
    pub fn add_capillary_rise_params(&mut self) {
        for layer in self.layers() {
            let thwp = self.layer_mean(layer, &self.profile.th_wp);
            let thfc = self.layer_mean(layer, &self.profile.th_fc);
            let ths = self.layer_mean(layer, &self.profile.th_s);
            let Ksat = self.layer_mean(layer, &self.profile.Ksat);
            let aCR_sandy = -0.3112 - Ksat / 100000.0;
            let bCR_sandy = -1.4936 + 0.2416 * Ksat.ln();
            let aCR_loamy = -0.4986 + 9.0 * Ksat / 100000.0;
            let bCR_loamy = -2.1320 + 0.4778 * Ksat.ln();
            let aCR_sandy_clayey = -0.5677 - 4.0 * Ksat / 100000.0;
            let bCR_sandy_clayey = -3.7189 + 0.5922 * Ksat.ln();
            let aCR_silty_clayey = -0.6366 + 8.0 * Ksat / 10000.0;
            let bCR_silty_clayey = -1.9165 + 0.7063 * Ksat.ln();
            let (aCR, bCR);
            if ths <= 0.55 {
                if thwp >= 0.20 {
                    if (ths >= 0.49) && (thfc >= 0.40) {
                        aCR = aCR_silty_clayey;
                        bCR = bCR_silty_clayey;
                    } else {
                        aCR = aCR_sandy_clayey;
                        bCR = bCR_sandy_clayey;
                    }
                } else if thfc < 0.23 {
                    aCR = aCR_sandy;
                    bCR = bCR_sandy;
                } else if (thwp > 0.16) && (Ksat < 100.0) {
                    aCR = aCR_sandy_clayey;
                    bCR = bCR_sandy_clayey;
                } else if (thwp < 0.06) && (thfc < 0.28) && (Ksat > 750.0) {
                    aCR = aCR_sandy;
                    bCR = bCR_sandy;
                } else {
                    aCR = aCR_loamy;
                    bCR = bCR_loamy;
                }
            } else {
                aCR = aCR_silty_clayey;
                bCR = bCR_silty_clayey;
            }
            for i in 0..self.profile.len() {
                if self.profile.layer[i] == layer {
                    self.profile.aCR[i] = aCR;
                    self.profile.bCR[i] = bCR;
                }
            }
        }
    }
}

/// The compartment arrays the daily solution reads (`SoilProfile`).
#[derive(Clone, Debug, Default)]
pub struct SoilProfile {
    pub Comp: Vec<usize>,
    pub dz: Vec<f64>,
    pub Layer: Vec<i64>,
    pub dzsum: Vec<f64>,
    pub th_fc: Vec<f64>,
    pub th_s: Vec<f64>,
    pub th_wp: Vec<f64>,
    pub Ksat: Vec<f64>,
    pub Penetrability: Vec<f64>,
    pub th_dry: Vec<f64>,
    pub tau: Vec<f64>,
    pub zBot: Vec<f64>,
    pub z_top: Vec<f64>,
    pub zMid: Vec<f64>,
    pub th_fc_Adj: Vec<f64>,
    pub aCR: Vec<f64>,
    pub bCR: Vec<f64>,
}

impl SoilProfile {
    pub fn n(&self) -> usize {
        self.dz.len()
    }

    /// `np.unique(prof.Layer).shape[0]`.
    pub fn n_layers(&self) -> usize {
        let mut l = self.Layer.clone();
        l.sort_unstable();
        l.dedup();
        l.len()
    }

    /// Indices of the compartments in a layer (`np.argwhere(prof.Layer == layer)`).
    pub fn layer_idx(&self, layer: i64) -> Vec<usize> {
        (0..self.n()).filter(|&i| self.Layer[i] == layer).collect()
    }

    /// `prof.dz[idx].sum()`.
    pub fn dz_sum(&self, idx: &[usize]) -> f64 {
        let v: Vec<f64> = idx.iter().map(|&i| self.dz[i]).collect();
        np_sum(&v)
    }
}
