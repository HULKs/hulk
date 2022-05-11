mod cycler;
mod database;
mod image_receiver;
mod modules;

pub use cycler::Vision;
pub use database::{AdditionalOutputs, Database, MainOutputs};

pub type Configuration = crate::framework::configuration::Vision;
