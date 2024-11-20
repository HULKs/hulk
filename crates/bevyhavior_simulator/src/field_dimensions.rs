use std::ops::{Deref, DerefMut};

use bevy::prelude::Resource;

use types::field_dimensions::FieldDimensions;

/// Field dimensions only for simulation logic!
/// Each robot has its own parameters and field dimensions
/// which are not aufomatically synchronized.
#[derive(Resource, Default)]
pub struct SimulatorFieldDimensions {
    field_dimensions: FieldDimensions,
}

impl Deref for SimulatorFieldDimensions {
    type Target = FieldDimensions;

    fn deref(&self) -> &Self::Target {
        &self.field_dimensions
    }
}

impl DerefMut for SimulatorFieldDimensions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.field_dimensions
    }
}

impl From<FieldDimensions> for SimulatorFieldDimensions {
    fn from(field_dimensions: FieldDimensions) -> Self {
        Self { field_dimensions }
    }
}
