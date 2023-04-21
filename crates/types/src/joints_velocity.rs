use serde::{Serialize, Deserialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::Joints;
use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};
#[derive(SerializeHierarchy, Serialize, Deserialize, Debug, Copy, Clone, Default)]
pub struct JointsVelocity {
    #[serde(flatten)]
    pub inner: Joints<f32>,
}
impl Deref for JointsVelocity {
    type Target = Joints<f32>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for JointsVelocity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}


#[derive(SerializeHierarchy, Serialize, Deserialize, Debug, Copy, Clone, Default)]
pub struct JointsTime {
    pub inner: Joints<Duration>,
}
impl Deref for JointsTime {
    type Target = Joints<Duration>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for JointsTime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl JointsTime {
    pub fn max(&self) -> Duration {
        self.inner
            .as_vec()
            .into_iter()
            .flatten()
            .reduce(|acc, e| Duration::max(e, acc))
            .unwrap()
    }
}
