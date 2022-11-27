use serde::Deserialize;

#[cfg(feature = "nao")]
use crate::nao::Parameters as NaoParameters;
#[cfg(feature = "webots")]
use crate::webots::Parameters as WebotsParameters;

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    #[cfg(feature = "nao")]
    pub nao: NaoParameters,
    #[cfg(feature = "webots")]
    pub webots: WebotsParameters,
}
