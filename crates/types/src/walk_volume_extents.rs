use std::ops::Add;

use serde::{Deserialize, Serialize};

use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct WalkVolumeExtents {
    pub forward: f32,
    pub backward: f32,
    pub outward: f32,
    pub inward: f32,
    pub outward_rotation: f32,
    pub inward_rotation: f32,
}

impl Add for &WalkVolumeExtents {
    type Output = WalkVolumeExtents;

    fn add(self, rhs: Self) -> Self::Output {
        WalkVolumeExtents {
            forward: self.forward + rhs.forward,
            backward: self.backward + rhs.backward,
            outward: self.outward + rhs.outward,
            inward: self.inward + rhs.inward,
            outward_rotation: self.outward_rotation + rhs.outward_rotation,
            inward_rotation: self.inward_rotation + rhs.inward_rotation,
        }
    }
}
