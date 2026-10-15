//! The model driver: AquaCrop-OSPy's `AquaCropModel` (core.py).

use crate::config::ModelInput;
use crate::crop::Crop;
use crate::error::Error;
use crate::init::{initialize, WeatherDay};
use crate::output::{Results, Table, CROP_GROWTH_COLUMNS, WATER_FLUX_COLUMNS};
use crate::state::{Clock, InitCond, Params};
use crate::timestep::{check_model_is_finished, solution_single_time_step, update_time, Outputs};

/// A simulation in progress. Most callers want [`run`]; `Model` lets a caller
/// step day by day and look at the state between days.
pub struct Model {
    pub clock: Clock,
    pub params: Params,
    pub state: InitCond,
    weather: Vec<WeatherDay>,
    crop: Crop,
    outputs: Outputs,
}

impl Model {
    pub fn new(input: &ModelInput) -> Result<Model, Error> {
        let (clock, params, state, weather, crop) = initialize(input)?;
        let outputs = Outputs::new(clock.n_steps, params.Profile.n());
        Ok(Model { clock, params, state, weather, crop, outputs })
    }

    pub fn is_finished(&self) -> bool {
        self.clock.model_is_finished
    }

    /// `_perform_timestep`: simulate one day.
    pub fn step(&mut self) -> Result<(), Error> {
        if self.clock.model_is_finished {
            return Ok(());
        }
        let w = self.weather[self.clock.time_step_counter];
        solution_single_time_step(&mut self.state, &mut self.params, &self.clock, &w, &mut self.outputs);
        self.clock.model_is_finished = check_model_is_finished(&self.clock, self.state.harvest_flag);
        update_time(&mut self.clock, &mut self.state, &mut self.params, &self.weather, &self.crop)
    }

    /// Run to the end of the simulation period.
    pub fn run_to_end(&mut self) -> Result<(), Error> {
        while !self.clock.model_is_finished {
            self.step()?;
        }
        Ok(())
    }

    pub fn results(&self) -> Results {
        let n_comp = self.params.Profile.n();
        let mut ws_cols: Vec<String> = ["time_step_counter", "growing_season", "dap"].iter().map(|s| s.to_string()).collect();
        ws_cols.extend((1..=n_comp).map(|i| format!("th{i}")));
        Results {
            start: self.clock.simulation_start_date,
            water_flux: Table::new(WATER_FLUX_COLUMNS.iter().map(|s| s.to_string()).collect(), &self.outputs.water_flux),
            water_storage: Table::new(ws_cols, &self.outputs.water_storage),
            crop_growth: Table::new(CROP_GROWTH_COLUMNS.iter().map(|s| s.to_string()).collect(), &self.outputs.crop_growth),
            final_stats: self.outputs.final_stats.clone(),
            compartments: self.params.Profile.dz.clone(),
        }
    }
}

/// Run a whole simulation.
pub fn run(input: &ModelInput) -> Result<Results, Error> {
    let mut m = Model::new(input)?;
    m.run_to_end()?;
    Ok(m.results())
}
