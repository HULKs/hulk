use std::ops::{Deref, DerefMut};

use bevy::prelude::Resource;

use types::field_dimensions::FieldDimensions;

#[derive(Resource, Default)]
pub struct SimulatorFieldDimenstions {
    field_dimensions: FieldDimensions,
}

impl Deref for SimulatorFieldDimenstions {
    type Target = FieldDimensions;

    fn deref(&self) -> &Self::Target {
        &self.field_dimensions
    }
}

impl DerefMut for SimulatorFieldDimenstions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.field_dimensions
    }
}

impl From<FieldDimensions> for SimulatorFieldDimenstions {
    fn from(field_dimensions: FieldDimensions) -> Self {
        Self { field_dimensions }
    }
}
