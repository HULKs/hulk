use serde::Deserialize;
use types::hardware::Paths;

use crate::network::Parameters as NetworkParameters;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub network: NetworkParameters,
    pub paths: Paths,
}
