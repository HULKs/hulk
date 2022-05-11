use macros::{module, require_some};

use crate::types::{CameraMatrices, CameraMatrix, CameraPosition};

#[derive(Default)]
pub struct CameraMatrixProvider;

#[module(vision)]
#[input(path = camera_matrices, data_type = CameraMatrices, cycler = control)]
#[main_output(data_type = CameraMatrix)]
impl CameraMatrixProvider {}

impl CameraMatrixProvider {
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
