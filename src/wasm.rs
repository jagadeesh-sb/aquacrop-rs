//! The WebAssembly interface: JSON in, JSON out.
//!
//! Built with `--features wasm` and wasm-bindgen. Every function takes and
//! returns JSON strings so the JavaScript side needs no generated types.

use crate::config::ModelInput;
use crate::crop::{crop_names as names, crop_preset as preset};
use crate::soil::SOIL_TYPES;
use wasm_bindgen::prelude::*;

/// Run a simulation. `input` is a [`ModelInput`] as JSON; the result is the
/// [`crate::Results`] as JSON. Errors are thrown as JavaScript errors.
#[wasm_bindgen]
pub fn run(input: &str) -> Result<String, JsError> {
    let input: ModelInput = serde_json::from_str(input).map_err(|e| JsError::new(&format!("invalid input: {e}")))?;
    let results = crate::run(&input).map_err(|e| JsError::new(&e.to_string()))?;
    serde_json::to_string(&results).map_err(|e| JsError::new(&e.to_string()))
}

/// The built-in crop names, as a JSON array.
#[wasm_bindgen]
pub fn crop_names() -> String {
    serde_json::to_string(&names()).unwrap_or_default()
}

/// One built-in crop's parameters, as a JSON object (null if unknown).
#[wasm_bindgen]
pub fn crop_preset(name: &str) -> String {
    serde_json::to_string(&preset(name)).unwrap_or_default()
}

/// The built-in soil types, as a JSON array.
#[wasm_bindgen]
pub fn soil_types() -> String {
    serde_json::to_string(SOIL_TYPES).unwrap_or_default()
}

/// This crate's version and the AquaCrop-OSPy version it follows.
#[wasm_bindgen]
pub fn version() -> String {
    format!("aquacrop-rs {} (AquaCrop-OSPy {})", env!("CARGO_PKG_VERSION"), crate::UPSTREAM_VERSION)
}
