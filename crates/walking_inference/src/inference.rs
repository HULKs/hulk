use std::path::Path;

use booster::{ImuState, MotorState};
use color_eyre::Result;
use coordinate_systems::Ground;
use framework::deserialize_not_implemented;
use linear_algebra::{vector, Vector2};
use ndarray::{Array1, Axis};
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use serde::{Deserialize, Serialize};
use types::{
    cycle_time::CycleTime,
    joints::{leg::LegJoints, Joints},
    motion_command::MotionCommand,
    parameters::{MotorCommandParameters, RLWalkingParameters},
};

use crate::inputs::WalkingInferenceInputs;

#[derive(Deserialize, Serialize)]
pub struct WalkingInference {
    #[serde(skip, default = "deserialize_not_implemented")]
    session: Session,
    last_linear_velocity_command: Vector2<Ground>,
    last_angular_velocity_command: f32,
    last_gait_progress: f32,
    pub last_target_joint_positions: Joints,
}

impl WalkingInference {
    pub fn new(
        neural_network_folder: impl AsRef<Path>,
        prepare_motor_command_parameters: &MotorCommandParameters,
    ) -> Result<Self> {
        let neural_network_path = neural_network_folder.as_ref().join("T1.onnx");

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_execution_providers([
                TensorRTExecutionProvider::default().build(),
                CUDAExecutionProvider::default().build(),
            ])?
            .commit_from_file(neural_network_path)?;

        Ok(Self {
            session,
            last_linear_velocity_command: vector![0.0, 0.0],
            last_angular_velocity_command: 0.0,
            last_gait_progress: 0.0,
            last_target_joint_positions: prepare_motor_command_parameters.default_positions,
        })
    }

    fn calculate_inputs(
        &mut self,
        cycle_time: CycleTime,
        motion_command: &MotionCommand,
        imu_state: &ImuState,
        current_serial_joints: Joints<MotorState>,
        walking_parameters: &RLWalkingParameters,
        motor_command_parameters: &MotorCommandParameters,
    ) -> Result<WalkingInferenceInputs> {
        let walking_inference_inputs = WalkingInferenceInputs::try_new(
            cycle_time,
            motion_command,
            imu_state.roll_pitch_yaw,
            imu_state.angular_velocity,
            current_serial_joints,
            self.last_linear_velocity_command,
            self.last_angular_velocity_command,
            self.last_gait_progress,
            self.last_target_joint_positions,
            walking_parameters,
            motor_command_parameters,
        )?;

        self.last_linear_velocity_command = walking_inference_inputs.linear_velocity_command;
        self.last_angular_velocity_command = walking_inference_inputs.angular_velocity_command;
        self.last_gait_progress = walking_inference_inputs.gait_progress;

        Ok(walking_inference_inputs)
    }

    pub fn do_inference(
        &mut self,
        cycle_time: CycleTime,
        motion_command: &MotionCommand,
        imu_state: &ImuState,
        current_serial_joints: Joints<MotorState>,
        walking_parameters: &RLWalkingParameters,
        motor_command_parameters: &MotorCommandParameters,
    ) -> Result<(WalkingInferenceInputs, Joints)> {
        let walking_inference_inputs = self.calculate_inputs(
            cycle_time,
            motion_command,
            imu_state,
            current_serial_joints,
            walking_parameters,
            motor_command_parameters,
        )?;

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

        self.last_target_joint_positions = Joints {
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
        };

        Ok((walking_inference_inputs, self.last_target_joint_positions))
    }
}
