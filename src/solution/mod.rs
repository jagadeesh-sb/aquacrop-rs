//! The daily process modules (AquaCrop-OSPy `aquacrop/solution/`), one file
//! per Python module, in the order `solution_single_time_step` calls them.

pub mod aeration_stress;
pub mod biomass_accumulation;
pub mod canopy_cover;
pub mod capillary_rise;
pub mod check_groundwater_table;
pub mod drainage;
pub mod germination;
pub mod groundwater_inflow;
pub mod growing_degree_day;
pub mod growth_stage;
pub mod harvest_index;
pub mod hi_ref_current_day;
pub mod infiltration;
pub mod irrigation;
pub mod pre_irrigation;
pub mod rainfall_partition;
pub mod root_development;
pub mod root_zone_water;
pub mod soil_evaporation;
pub mod temperature_stress;
pub mod tile_drainage;
pub mod transpiration;
pub mod water_stress;
