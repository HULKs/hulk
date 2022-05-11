mod cycler;
mod database;
mod filtering;
mod linear_interpolator;
mod modules;
mod sensor_data_receiver;

pub use cycler::Control;
pub use database::{AdditionalOutputs, Database, MainOutputs, PersistentState};

pub type Configuration = crate::framework::configuration::Control;
