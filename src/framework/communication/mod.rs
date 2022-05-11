mod acceptor;
pub mod configuration_directory;
mod connection;
mod database_subscription_manager;
mod parameter_modificator;
mod receiver;
mod runtime;
mod sender;

pub use runtime::{
    ChannelsForDatabases, ChannelsForDatabasesWithImage, ChannelsForParameters, Communication,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CyclerOutput {
    pub cycler: Cycler,
    pub output: Output,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Cycler {
    Audio,
    Control,
    SplNetwork,
    VisionTop,
    VisionBottom,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Output {
    Main { path: String },
    Additional { path: String },
    Image,
}
