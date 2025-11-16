use std::path::Path;

use color_eyre::Result;
use framework::deserialize_not_implemented;
use ndarray::{Array1, Axis};
use ort::{
    inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use serde::{Deserialize, Serialize};
use types::{
    joints::{leg::LegJoints, Joints},
    parameters::RLWalkingParameters,
};

use crate::inputs::WalkingInferenceInputs;

#[derive(Deserialize, Serialize)]
pub struct WalkingInference {
    #[serde(skip, default = "deserialize_not_implemented")]
    session: Session,
}

impl WalkingInference {
    pub fn new(neural_network_folder: impl AsRef<Path>) -> Result<Self> {
        let neural_network_path = neural_network_folder.as_ref().join("T1.onnx");

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(neural_network_path)?;

        Ok(Self { session })
    }

    pub fn do_inference(
        &mut self,
        walking_inference_inputs: WalkingInferenceInputs,
        walking_parameters: &RLWalkingParameters,
    ) -> Result<Joints> {
        let inputs: Array1<f32> = walking_inference_inputs.as_vec().into();

        assert!(inputs.len() == walking_parameters.number_of_observations);
        let inputs_tensor = Tensor::from_array(inputs.insert_axis(Axis(0)))?;

        let outputs = self.session.run(inputs![inputs_tensor])?;
        let predictions = outputs["15"].try_extract_array::<f32>()?.squeeze();

        predictions.clamp(
            -walking_parameters.normalization.clip_actions,
            walking_parameters.normalization.clip_actions,
        );

        assert!(predictions.len() == walking_parameters.number_of_actions);

        Ok(Joints {
            left_leg: LegJoints {
                hip_pitch: predictions[0],
                hip_roll: predictions[1],
                hip_yaw: predictions[2],
                knee: predictions[3],
                ankle_up: predictions[4],
                ankle_down: predictions[5],
            },
            right_leg: LegJoints {
                hip_pitch: predictions[6],
                hip_roll: predictions[7],
                hip_yaw: predictions[8],
                knee: predictions[9],
                ankle_up: predictions[10],
                ankle_down: predictions[11],
            },
            ..Default::default()
        })
    }
}
