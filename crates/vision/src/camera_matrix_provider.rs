use framework::{
    MainOutput, OptionalInput
};

pub struct CameraMatrixProvider {}

#[context]
pub struct NewContext {
}

#[context]
pub struct CycleContext {


    pub camera_matrices: OptionalInput<CameraMatrices, "Control", "camera_matrices">,




}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub camera_matrix: MainOutput<CameraMatrix>,
}

impl CameraMatrixProvider {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
