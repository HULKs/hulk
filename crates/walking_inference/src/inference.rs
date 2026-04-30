use std::{collections::VecDeque, path::Path, time::Duration};

use booster::{ImuState, MotorCommandParameters, MotorState};
use color_eyre::Result;
use coordinate_systems::Ground;
use framework::deserialize_not_implemented;
use kinematics::joints::{Joints, leg::LegJoints};
use linear_algebra::{Vector2, vector};
use ndarray::{Array1, Axis};
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    inputs,
    session::{Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use serde::{Deserialize, Serialize};
use types::parameters::RLWalkingParameters;

use crate::inputs::{WalkCommand, WalkingInferenceInputs};

#[derive(Deserialize, Serialize)]
pub struct WalkingInference {
    #[serde(skip, default = "deserialize_not_implemented")]
    session: Session,
    last_linear_velocity_command: Vector2<Ground>,
    last_angular_velocity_command: f32,
    last_gait_progress: f32,
    last_target_joint_positions: Joints,
    input_history: VecDeque<Option<WalkingInferenceInputs>>,
}

impl WalkingInference {
    pub fn new(neural_network_folder: impl AsRef<Path>, history_length: usize) -> Result<Self> {
        let neural_network_path = neural_network_folder
            .as_ref()
            .join("2026-04-20_23-59-21-9999.onnx");

        let tensor_rt = TensorRTExecutionProvider::default()
            .with_device_id(0)
            .with_fp16(true)
            .with_engine_cache(true)
            .with_engine_cache_path(neural_network_folder.as_ref().to_path_buf().display())
            .build();

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_execution_providers([tensor_rt, CUDAExecutionProvider::default().build()])?
            .with_intra_threads(1)?
            .commit_from_file(neural_network_path)?;

        let mut input_history = VecDeque::with_capacity(history_length);
        for _ in 0..history_length {
            input_history.push_front(Default::default());
        }

        Ok(Self {
            session,
            last_linear_velocity_command: vector![0.0, 0.0],
            last_angular_velocity_command: 0.0,
            last_gait_progress: 0.0,
            last_target_joint_positions: Default::default(),
            input_history,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn do_inference(
        &mut self,
        last_cycle_duration: Duration,
        walk_command: &WalkCommand,
        imu_state: &ImuState,
        current_serial_joints: Joints<MotorState>,
        walking_parameters: &RLWalkingParameters,
        motor_command_parameters: &MotorCommandParameters,
    ) -> Result<Joints> {
        let walking_inference_inputs = WalkingInferenceInputs::try_new(
            last_cycle_duration,
            walk_command,
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

        if self.input_history.iter().any(|elem| elem.is_none()) {
            for _ in 0..self.input_history.len() {
                self.input_history
                    .push_front(Some(walking_inference_inputs.clone()));
            }
        } else {
            self.input_history
                .push_front(Some(walking_inference_inputs));
        }
        self.input_history
            .truncate(walking_parameters.observation_history_length);

        let inputs: Array1<f32> = Vec::from_iter(
            history(
                &self.input_history,
                walking_parameters.observation_history_length,
                |i: &WalkingInferenceInputs| {
                    [
                        i.angular_velocity.x(),
                        i.angular_velocity.y(),
                        i.angular_velocity.z(),
                    ]
                },
            )
            .chain(history(
                &self.input_history,
                walking_parameters.observation_history_length,
                |i: &WalkingInferenceInputs| {
                    [i.gravity.x(), i.gravity.y(), i.gravity.z()].into_iter()
                },
            ))
            .chain(history(
                &self.input_history,
                walking_parameters.observation_history_length,
                |i: &WalkingInferenceInputs| joints_as_array(i.joint_position_differences),
            ))
            .chain(history(
                &self.input_history,
                walking_parameters.observation_history_length,
                |i: &WalkingInferenceInputs| joints_as_array(i.joint_velocities),
            ))
            .chain(history(
                &self.input_history,
                walking_parameters.observation_history_length,
                |i: &WalkingInferenceInputs| joints_as_array(i.last_target_joint_positions),
            ))
            .chain(history(
                &self.input_history,
                walking_parameters.observation_history_length,
                |i: &WalkingInferenceInputs| {
                    [
                        i.linear_velocity_command.x(),
                        i.linear_velocity_command.y(),
                        i.angular_velocity_command,
                    ]
                    .into_iter()
                },
            )),
        )
        .into();

        assert!(
            inputs.len()
                == walking_parameters.number_of_observations
                    * walking_parameters.observation_history_length
        );
        let inputs_tensor = Tensor::from_array(inputs.insert_axis(Axis(0)))?;

        let inference_input = inputs![inputs_tensor];

        let outputs = self.session.run(inference_input)?;

        let predictions = outputs["actions"].try_extract_array::<f32>()?.squeeze();

        assert!(predictions.len() == walking_parameters.number_of_actions);

        // ALeft_Shoulder_Pitch,Left_Shoulder_Roll,Left_Elbow_Pitch,Left_Elbow_Yaw,
        // ARight_Shoulder_Pitch,Right_Shoulder_Roll,Right_Elbow_Pitch,Right_Elbow_Yaw,
        // Left_Hip_Pitch,Left_Hip_Roll,Left_Hip_Yaw,Left_Knee_Pitch,Left_Ankle_Pitch,Left_Ankle_Roll,
        // Right_Hip_Pitch,Right_Hip_Roll,Right_Hip_Yaw,Right_Knee_Pitch,Right_Ankle_Pitch,Right_Ankle_Roll
        self.last_target_joint_positions = Joints {
            // left_arm: ArmJoints {
            //     shoulder_pitch: predictions[0],
            //     shoulder_roll: predictions[1],
            //     elbow: predictions[2],
            //     shoulder_yaw: predictions[3],
            // },
            // right_arm: ArmJoints {
            //     shoulder_pitch: predictions[4],
            //     shoulder_roll: predictions[5],
            //     elbow: predictions[6],
            //     shoulder_yaw: predictions[7],
            // },
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

        Ok(self.last_target_joint_positions)
    }
}

fn history<'a, T, F>(
    input_history: &'a VecDeque<Option<WalkingInferenceInputs>>,
    n: usize,
    f: F,
) -> impl Iterator<Item = f32> + use<'a, T, F>
where
    T: IntoIterator<Item = f32>,
    F: FnMut(&'a WalkingInferenceInputs) -> T,
{
    input_history.iter().flatten().take(n).flat_map(f)
}

fn joints_as_array(joints: Joints) -> [f32; 12] {
    // ALeft_Shoulder_Pitch,Left_Shoulder_Roll,Left_Elbow_Pitch,Left_Elbow_Yaw,
    // ARight_Shoulder_Pitch,Right_Shoulder_Roll,Right_Elbow_Pitch,Right_Elbow_Yaw,
    // Left_Hip_Pitch,Left_Hip_Roll,Left_Hip_Yaw,Left_Knee_Pitch,Left_Ankle_Pitch,Left_Ankle_Roll,
    // Right_Hip_Pitch,Right_Hip_Roll,Right_Hip_Yaw,Right_Knee_Pitch,Right_Ankle_Pitch,Right_Ankle_Roll
    [
        // joints.left_arm.shoulder_pitch,
        // joints.left_arm.shoulder_roll,
        // joints.left_arm.elbow,
        // joints.left_arm.shoulder_yaw,
        // joints.right_arm.shoulder_pitch,
        // joints.right_arm.shoulder_roll,
        // joints.right_arm.elbow,
        // joints.right_arm.shoulder_yaw,
        joints.left_leg.hip_pitch,
        joints.left_leg.hip_roll,
        joints.left_leg.hip_yaw,
        joints.left_leg.knee,
        joints.left_leg.ankle_up,
        joints.left_leg.ankle_down,
        joints.right_leg.hip_pitch,
        joints.right_leg.hip_roll,
        joints.right_leg.hip_yaw,
        joints.right_leg.knee,
        joints.right_leg.ankle_up,
        joints.right_leg.ankle_down,
    ]
}
