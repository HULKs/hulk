use module_derive::{module, require_some};

use types::{CameraMatrices, CameraMatrix, CameraPosition};

pub struct CameraMatrixProvider;

#[module(vision)]
#[input(path = camera_matrices, data_type = CameraMatrices, cycler = control)]
#[main_output(data_type = CameraMatrix)]
impl CameraMatrixProvider {}

impl CameraMatrixProvider {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self)
    }

    fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        let camera_matrices = require_some!(context.camera_matrices);
        let camera_matrix = match context.camera_position {
            CameraPosition::Top => &camera_matrices.top,
            CameraPosition::Bottom => &camera_matrices.bottom,
        };

        Ok(MainOutputs {
            camera_matrix: Some(camera_matrix.to_owned()),
        })
    }
}
