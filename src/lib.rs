//! # aquacrop
//!
//! The FAO AquaCrop crop-water productivity model, ported line by line from
//! [AquaCrop-OSPy](https://github.com/aquacropos/aquacrop) 3.1.0 (Apache-2.0)
//! to Rust, for native programs and for WebAssembly in the browser.
//!
//! Each daily process module keeps the Python module's name and its variable
//! names, so every formula can be checked against the original. The test suite
//! compares daily outputs with AquaCrop-OSPy's on a set of reference runs.
//!
//! This is not an official implementation of FAO AquaCrop, and neither is
//! AquaCrop-OSPy.
//!
//! ```no_run
//! let input: aquacrop::ModelInput = serde_json::from_str(r#"{ ... }"#).unwrap();
//! let results = aquacrop::run(&input).unwrap();
//! for season in &results.final_stats {
//!     println!("{} {:.2} t/ha", season.harvest_date, season.dry_yield);
//! }
//! ```

// The port keeps the Python's statement shapes (x = x + y, explicit index
// loops, long argument lists, manual clamps) so each line can be checked
// against the original; these lints would ask for rewrites that hide that.
#![allow(
    clippy::assign_op_pattern,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::manual_clamp,
    clippy::needless_late_init,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::excessive_precision,
    clippy::type_complexity,
    // AquaCrop-OSPy writes these the same way (the CO2 branches, -1 * x).
    clippy::if_same_then_else,
    clippy::neg_multiply,
    clippy::manual_memcpy,
    // -0.6366 is a capillary-rise regression coefficient, not 2/pi.
    clippy::approx_constant
)]

pub mod config;
pub mod crop;
pub mod date;
pub mod error;
pub mod init;
pub mod model;
pub mod numeric;
pub mod output;
pub mod round;
pub mod soil;
pub mod solution;
pub mod state;
pub mod timestep;
#[cfg(feature = "wasm")]
pub mod wasm;

pub use config::ModelInput;
pub use error::Error;
pub use model::{run, Model};
pub use output::Results;

/// The AquaCrop-OSPy version this port follows.
pub const UPSTREAM_VERSION: &str = "3.1.0";
