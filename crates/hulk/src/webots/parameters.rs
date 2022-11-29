use serde::Deserialize;

use crate::network::Parameters as NetworkParameters;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    pub network: NetworkParameters,
}
