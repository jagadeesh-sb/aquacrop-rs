//! Crop parameters: AquaCrop-OSPy's `Crop` class (entities/crop.py).
//!
//! Field names keep the Python spelling so every formula can be checked
//! against the original line by line.

#![allow(non_snake_case)]

use crate::date::MonthDay;
use crate::error::Error;
use serde_json::{Map, Value};
use std::sync::OnceLock;

/// Built-in crop parameter sets, exported verbatim from AquaCrop-OSPy 3.1.0
/// (`aquacrop/entities/crops/crop_params.py`).
const CROP_PARAMS_JSON: &str = include_str!("../data/crop_params.json");

fn crop_params() -> &'static Map<String, Value> {
    static PARAMS: OnceLock<Map<String, Value>> = OnceLock::new();
    PARAMS.get_or_init(|| serde_json::from_str(CROP_PARAMS_JSON).expect("bundled crop_params.json is valid"))
}

/// Names of the built-in crops, in the order AquaCrop-OSPy lists them.
pub fn crop_names() -> Vec<String> {
    crop_params().keys().cloned().collect()
}

/// The parameter set of a built-in crop, as AquaCrop-OSPy stores it.
pub fn crop_preset(name: &str) -> Option<&'static Map<String, Value>> {
    crop_params().get(name).and_then(Value::as_object)
}

#[derive(Clone, Debug)]
pub struct Crop {
    pub Name: String,
    pub planting_date: MonthDay,
    pub harvest_date: Option<MonthDay>,

    pub fshape_b: f64,
    pub PctZmin: f64,
    pub fshape_ex: f64,
    pub ETadj: f64,
    pub ET0dorm: f64,
    pub Aer: f64,
    pub LagAer: f64,
    pub beta: f64,
    pub a_Tr: f64,
    pub GermThr: f64,
    pub CCmin: f64,
    pub MaxFlowPct: f64,
    pub HIini: f64,
    pub bsted: f64,
    pub bface: f64,

    pub CropType: f64,
    pub PlantMethod: f64,
    pub CalendarType: f64,
    pub SwitchGDD: f64,
    /// "mean" or "median".
    pub SwitchGDDType: String,

    pub EmergenceCD: f64,
    pub Canopy10PctCD: f64,
    pub MaxRootingCD: f64,
    pub SenescenceCD: f64,
    pub MaturityCD: f64,
    pub MaxCanopyCD: f64,
    pub CanopyDevEndCD: f64,
    pub HIstartCD: f64,
    pub HIendCD: f64,
    pub YldFormCD: f64,

    pub Emergence: f64,
    pub MaxRooting: f64,
    pub Senescence: f64,
    pub Maturity: f64,
    pub HIstart: f64,
    pub Flowering: f64,
    pub YldForm: f64,
    pub HIend: f64,
    pub MaxCanopy: f64,
    pub CanopyDevEnd: f64,
    pub Canopy10Pct: f64,
    pub YldWC: f64,
    pub GDDmethod: f64,
    pub Tbase: f64,
    pub Tupp: f64,
    pub PolHeatStress: f64,
    pub Tmax_up: f64,
    pub Tmax_lo: f64,
    pub PolColdStress: f64,
    pub Tmin_up: f64,
    pub Tmin_lo: f64,
    pub TrColdStress: f64,
    pub GDD_up: f64,
    pub GDD_lo: f64,
    pub Zmin: f64,
    pub Zmax: f64,
    pub fshape_r: f64,
    pub SxTopQ: f64,
    pub SxBotQ: f64,
    pub SxTop: f64,
    pub SxBot: f64,
    pub SeedSize: f64,
    pub PlantPop: f64,
    pub CCx: f64,
    pub CDC: f64,
    pub CGC: f64,
    pub CDC_CD: f64,
    pub CGC_CD: f64,
    pub Kcb: f64,
    pub fage: f64,
    pub WP: f64,
    pub WPy: f64,
    pub fsink: f64,
    pub HI0: f64,
    pub dHI_pre: f64,
    pub a_HI: f64,
    pub b_HI: f64,
    pub dHI0: f64,
    pub Determinant: f64,
    pub exc: f64,

    pub p_up1: f64,
    pub p_up2: f64,
    pub p_up3: f64,
    pub p_up4: f64,
    pub p_lo1: f64,
    pub p_lo2: f64,
    pub p_lo3: f64,
    pub p_lo4: f64,
    pub fshape_w1: f64,
    pub fshape_w2: f64,
    pub fshape_w3: f64,
    pub fshape_w4: f64,
    pub p_up: [f64; 4],
    pub p_lo: [f64; 4],
    pub fshape_w: [f64; 4],

    pub CC0: f64,
    pub HIGC: f64,
    pub tLinSwitch: f64,
    pub dHILinear: f64,
    pub fCO2: f64,
    pub FloweringCD: f64,
    pub FloweringEnd: f64,
    pub FloweringEndCD: f64,

    /// Adaptive planting (timestep/adaptive_planting.py); off unless `PlantTrigger` is set.
    pub PlantTrigger: bool,
    pub PlantTempThr: Option<f64>,
    pub PlantTWindow: f64,
    pub PlantRainThr: Option<f64>,
    pub PlantRainWindow: Option<f64>,
    pub PlantSMThr: Option<f64>,
    pub PlantMaxDelay: f64,
}

/// Keys a caller may override (Crop.__init__'s `allowed_keys`).
const ALLOWED_KEYS: &[&str] = &[
    "fshape_b", "PctZmin", "fshape_ex", "ETadj", "ET0dorm", "Aer", "LagAer", "beta", "a_Tr", "GermThr", "CCmin",
    "MaxFlowPct", "HIini", "bsted", "bface", "Name", "CropType", "PlantMethod", "CalendarType", "SwitchGDD",
    "SwitchGDDType", "planting_date", "harvest_date", "Emergence", "MaxRooting", "Senescence", "Maturity", "HIstart",
    "Flowering", "YldForm", "YldWC", "GDDmethod", "Tbase", "Tupp", "PolHeatStress", "Tmax_up", "Tmax_lo",
    "PolColdStress", "Tmin_up", "Tmin_lo", "TrColdStress", "GDD_up", "GDD_lo", "Zmin", "Zmax", "fshape_r", "SxTopQ",
    "SxBotQ", "SeedSize", "PlantPop", "CCx", "CDC", "CGC", "Kcb", "fage", "WP", "WPy", "fsink", "HI0", "dHI_pre",
    "a_HI", "b_HI", "dHI0", "Determinant", "exc", "p_up1", "p_up2", "p_up3", "p_up4", "p_lo1", "p_lo2", "p_lo3",
    "p_lo4", "fshape_w1", "fshape_w2", "fshape_w3", "fshape_w4", "CGC_CD", "CDC_CD", "EmergenceCD", "MaxRootingCD",
    "SenescenceCD", "MaturityCD", "HIstartCD", "FloweringCD", "YldFormCD", "PlantTrigger", "PlantTempThr",
    "PlantTWindow", "PlantRainThr", "PlantRainWindow", "PlantSMThr", "PlantMaxDelay",
];

impl Crop {
    /// `Crop(c_name, planting_date, harvest_date, **overrides)`.
    pub fn new(
        name: &str,
        planting_date: MonthDay,
        harvest_date: Option<MonthDay>,
        overrides: &Map<String, Value>,
    ) -> Result<Crop, Error> {
        let mut c = Crop::defaults(name, planting_date, harvest_date);
        if name != "custom" {
            let preset = crop_preset(name).ok_or_else(|| {
                Error::Input(format!(
                    "unknown crop {name:?}; use \"custom\" or one of: {}",
                    crop_names().join(", ")
                ))
            })?;
            for (k, v) in preset {
                c.set(k, v)?;
            }
            c.planting_date = planting_date;
            c.harvest_date = harvest_date;
        }
        for (k, v) in overrides {
            if !ALLOWED_KEYS.contains(&k.as_str()) {
                return Err(Error::Input(format!("crop parameter {k:?} is not one AquaCrop accepts")));
            }
            c.set(k, v)?;
        }
        c.calculate_additional_params();
        Ok(c)
    }

    fn defaults(name: &str, planting_date: MonthDay, harvest_date: Option<MonthDay>) -> Crop {
        Crop {
            Name: name.to_string(),
            planting_date,
            harvest_date,
            fshape_b: 13.8135,
            PctZmin: 70.0,
            fshape_ex: -6.0,
            ETadj: 1.0,
            ET0dorm: 0.0,
            Aer: 5.0,
            LagAer: 3.0,
            beta: 12.0,
            a_Tr: 1.0,
            GermThr: 0.2,
            CCmin: 0.05,
            MaxFlowPct: 100.0 / 3.0,
            HIini: 0.01,
            bsted: 0.000138,
            bface: 0.001165,
            CropType: 3.0,
            PlantMethod: 1.0,
            CalendarType: 2.0,
            SwitchGDD: 0.0,
            SwitchGDDType: "mean".to_string(),
            EmergenceCD: 0.0,
            Canopy10PctCD: 0.0,
            MaxRootingCD: 0.0,
            SenescenceCD: 0.0,
            MaturityCD: 0.0,
            MaxCanopyCD: 0.0,
            CanopyDevEndCD: 0.0,
            HIstartCD: 0.0,
            HIendCD: 0.0,
            YldFormCD: 0.0,
            Emergence: 80.0,
            MaxRooting: 1420.0,
            Senescence: 1420.0,
            Maturity: 1670.0,
            HIstart: 850.0,
            Flowering: 190.0,
            YldForm: 775.0,
            HIend: 0.0,
            MaxCanopy: 0.0,
            CanopyDevEnd: 0.0,
            Canopy10Pct: 0.0,
            YldWC: 0.0,
            GDDmethod: 2.0,
            Tbase: 8.0,
            Tupp: 30.0,
            PolHeatStress: 1.0,
            Tmax_up: 40.0,
            Tmax_lo: 45.0,
            PolColdStress: 1.0,
            Tmin_up: 10.0,
            Tmin_lo: 5.0,
            TrColdStress: 1.0,
            GDD_up: 12.0,
            GDD_lo: 0.0,
            Zmin: 0.3,
            Zmax: 1.7,
            fshape_r: 1.3,
            SxTopQ: 0.0480,
            SxBotQ: 0.0117,
            SxTop: 0.0,
            SxBot: 0.0,
            SeedSize: 6.5,
            PlantPop: 75_000.0,
            CCx: 0.96,
            CDC: 0.01,
            CGC: 0.0125,
            CDC_CD: 0.01,
            CGC_CD: 0.0125,
            Kcb: 1.05,
            fage: 0.3,
            WP: 33.7,
            WPy: 100.0,
            fsink: 0.5,
            HI0: 0.48,
            dHI_pre: 0.0,
            a_HI: 7.0,
            b_HI: 3.0,
            dHI0: 15.0,
            Determinant: 1.0,
            exc: 50.0,
            // A custom crop must give these; AquaCrop-OSPy fails without them.
            p_up1: f64::NAN,
            p_up2: f64::NAN,
            p_up3: f64::NAN,
            p_up4: f64::NAN,
            p_lo1: f64::NAN,
            p_lo2: f64::NAN,
            p_lo3: f64::NAN,
            p_lo4: f64::NAN,
            fshape_w1: f64::NAN,
            fshape_w2: f64::NAN,
            fshape_w3: f64::NAN,
            fshape_w4: f64::NAN,
            p_up: [0.0; 4],
            p_lo: [0.0; 4],
            fshape_w: [1.0; 4],
            CC0: 0.0,
            HIGC: 0.0,
            tLinSwitch: 0.0,
            dHILinear: 0.0,
            fCO2: 0.0,
            FloweringCD: 0.0,
            FloweringEnd: 0.0,
            FloweringEndCD: 0.0,
            PlantTrigger: false,
            PlantTempThr: None,
            PlantTWindow: 5.0,
            PlantRainThr: None,
            PlantRainWindow: None,
            PlantSMThr: None,
            PlantMaxDelay: 60.0,
        }
    }

    /// Set one parameter by its AquaCrop-OSPy name.
    pub fn set(&mut self, key: &str, v: &Value) -> Result<(), Error> {
        let num = || -> Result<f64, Error> {
            match v {
                Value::Number(n) => n.as_f64().ok_or_else(|| Error::Input(format!("{key}: not a number"))),
                Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
                _ => Err(Error::Input(format!("crop parameter {key} must be a number, got {v}"))),
            }
        };
        let opt = || -> Result<Option<f64>, Error> { if v.is_null() { Ok(None) } else { num().map(Some) } };
        macro_rules! fields {
            ($($f:ident),* $(,)?) => {
                match key {
                    $(stringify!($f) => self.$f = num()?,)*
                    "Name" => self.Name = v.as_str().unwrap_or_default().to_string(),
                    "SwitchGDDType" => self.SwitchGDDType = v.as_str().unwrap_or("mean").to_string(),
                    "planting_date" => {
                        self.planting_date = v.as_str().and_then(MonthDay::parse)
                            .ok_or_else(|| Error::Input(format!("planting_date must be MM/DD, got {v}")))?
                    }
                    "harvest_date" => {
                        self.harvest_date = if v.is_null() { None } else {
                            Some(v.as_str().and_then(MonthDay::parse)
                                .ok_or_else(|| Error::Input(format!("harvest_date must be MM/DD, got {v}")))?)
                        }
                    }
                    "PlantTrigger" => self.PlantTrigger = match v {
                        Value::Bool(b) => *b,
                        Value::Null => false,
                        Value::String(s) => !s.is_empty(),
                        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
                        _ => true,
                    },
                    "PlantTempThr" => self.PlantTempThr = opt()?,
                    "PlantRainThr" => self.PlantRainThr = opt()?,
                    "PlantRainWindow" => self.PlantRainWindow = opt()?,
                    "PlantSMThr" => self.PlantSMThr = opt()?,
                    _ => return Err(Error::Input(format!("unknown crop parameter {key:?}"))),
                }
            };
        }
        fields!(
            fshape_b, PctZmin, fshape_ex, ETadj, ET0dorm, Aer, LagAer, beta, a_Tr, GermThr, CCmin, MaxFlowPct, HIini,
            bsted, bface, CropType, PlantMethod, CalendarType, SwitchGDD, EmergenceCD, Canopy10PctCD, MaxRootingCD,
            SenescenceCD, MaturityCD, MaxCanopyCD, CanopyDevEndCD, HIstartCD, HIendCD, YldFormCD, Emergence,
            MaxRooting, Senescence, Maturity, HIstart, Flowering, YldForm, HIend, MaxCanopy, CanopyDevEnd,
            Canopy10Pct, YldWC, GDDmethod, Tbase, Tupp, PolHeatStress, Tmax_up, Tmax_lo, PolColdStress, Tmin_up,
            Tmin_lo, TrColdStress, GDD_up, GDD_lo, Zmin, Zmax, fshape_r, SxTopQ, SxBotQ, SeedSize, PlantPop, CCx,
            CDC, CGC, CDC_CD, CGC_CD, Kcb, fage, WP, WPy, fsink, HI0, dHI_pre, a_HI, b_HI, dHI0, Determinant, exc,
            p_up1, p_up2, p_up3, p_up4, p_lo1, p_lo2, p_lo3, p_lo4, fshape_w1, fshape_w2, fshape_w3, fshape_w4,
            FloweringCD, FloweringEnd, FloweringEndCD, PlantTWindow, PlantMaxDelay,
        );
        Ok(())
    }

    /// Crop.calculate_additional_params.
    fn calculate_additional_params(&mut self) {
        self.CC0 = self.PlantPop * self.SeedSize * 1e-8;
        let SxTopQ = self.SxTopQ;
        let SxBotQ = self.SxBotQ;
        let mut S1 = self.SxTopQ;
        let mut S2 = self.SxBotQ;
        let (SxTop, SxBot);
        if S1 == S2 {
            SxTop = S1;
            SxBot = S2;
        } else {
            if SxTopQ < SxBotQ {
                S1 = SxBotQ;
                S2 = SxTopQ;
            }
            let xx = 3.0 * (S2 / (S1 - S2));
            let (SS1, SS2);
            if xx < 0.5 {
                SS1 = (4.0 / 3.5) * S1;
                SS2 = 0.0;
            } else {
                SS1 = (xx + 3.5) * (S1 / (xx + 3.0));
                SS2 = (xx - 0.5) * (S2 / xx);
            }
            if SxTopQ > SxBotQ {
                SxTop = SS1;
                SxBot = SS2;
            } else {
                SxTop = SS2;
                SxBot = SS1;
            }
        }
        self.SxTop = SxTop;
        self.SxBot = SxBot;
        self.p_up = [self.p_up1, self.p_up2, self.p_up3, self.p_up4];
        self.p_lo = [self.p_lo1, self.p_lo2, self.p_lo3, self.p_lo4];
        self.fshape_w = [self.fshape_w1, self.fshape_w2, self.fshape_w3, self.fshape_w4];
    }

    /// The parameters a custom crop must provide before it can run.
    pub fn missing_parameters(&self) -> Vec<&'static str> {
        let named = [
            ("p_up1", self.p_up1), ("p_up2", self.p_up2), ("p_up3", self.p_up3), ("p_up4", self.p_up4),
            ("p_lo1", self.p_lo1), ("p_lo2", self.p_lo2), ("p_lo3", self.p_lo3), ("p_lo4", self.p_lo4),
            ("fshape_w1", self.fshape_w1), ("fshape_w2", self.fshape_w2), ("fshape_w3", self.fshape_w3),
            ("fshape_w4", self.fshape_w4),
        ];
        named.iter().filter(|(_, v)| v.is_nan()).map(|(k, _)| *k).collect()
    }
}
