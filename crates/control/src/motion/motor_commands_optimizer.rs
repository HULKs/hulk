use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Robot};
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use kinematics::forward;
use linear_algebra::Orientation3;
use serde::{Deserialize, Serialize};
use types::{
    joints::Joints,
    motion_command::{HeadMotion, MotionCommand},
    motor_commands::MotorCommands,
    support_foot::Side,
};
use walking_engine::parameters::FootLevelingParameters;

#[derive(Deserialize, Serialize)]
pub struct MotorCommandsOptimizer {
    pitch_low_pass_filter: LowPassFilter<f32>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    motor_commands: Input<MotorCommands<Joints<f32>>, "motor_commands">,
    only_one_foot_has_ground_contact: Input<bool, "only_one_foot_has_ground_contact">,
    has_ground_contact: Input<bool, "has_ground_contact">,
    motion_command: Input<MotionCommand, "motion_command">,

    last_actuated_joints: CyclerState<Joints<f32>, "last_actuated_motor_commands.positions">,
    robot_orientation: RequiredInput<Option<Orientation3<Field>>, "robot_orientation?">,

    parameters: Parameter<FootLevelingParameters, "walking_engine.foot_leveling">,

    desired_pitch: AdditionalOutput<f32, "motor_commands_optimizer.desired_pitch">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub optimized_motor_commands: MainOutput<MotorCommands<Joints<f32>>>,
}

impl MotorCommandsOptimizer {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            pitch_low_pass_filter: LowPassFilter::with_smoothing_factor(0.0, 0.1),
        })
    }
    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let mut motor_commands = *context.motor_commands;
        motor_commands.stiffnesses.left_arm.hand = 0.0;
        motor_commands.stiffnesses.right_arm.hand = 0.0;

        if (*context.only_one_foot_has_ground_contact || !*context.has_ground_contact)
            && (*context.motion_command
                == MotionCommand::Initial {
                    head: HeadMotion::Center,
                }
                || *context.motion_command == MotionCommand::Penalized)
        {
            motor_commands.stiffnesses = Joints::fill(0.3);
        }

        let parameters = &context.parameters;
        let current_orientation = context.robot_orientation.rotation::<Robot>();

        // manual support side for testing purposes
        let support_side = Side::Left;

        // let current_orientation = context.robot_orientation;
        //
        // let leveling_error = current_orientation.inner;
        // let (roll_angle, pitch_angle, _) = leveling_error.euler_angles();

        // * forward::right_sole_to_robot(&context.last_actuated_joints.right_leg);

        let rotation = match support_side {
            Side::Left => {
                let right_sole_to_field = current_orientation
                    * forward::right_sole_to_robot(&context.last_actuated_joints.right_leg);

                right_sole_to_field.inner.rotation
            }
            Side::Right => {
                let left_sole_to_field = current_orientation
                    * forward::left_sole_to_robot(&context.last_actuated_joints.left_leg);

                left_sole_to_field.inner.rotation
            }
        };

        let ([desired_pitch, desired_roll, _], _) = rotation
            .inverse()
            .to_rotation_matrix()
            .euler_angles_ordered(
                [
                    nalgebra::Vector3::y_axis(),
                    nalgebra::Vector3::x_axis(),
                    nalgebra::Vector3::z_axis(),
                ],
                false,
            );

        // Choose the base pitch factor depending on whether the robot is leaning forward or backward
        // let base_pitch_factor = if pitch_angle > 0.0 {
        //     parameters.leaning_forward_factor
        // } else {
        //     parameters.leaning_backwards_factor
        // };

        // let pitch_scaling = (pitch_angle.abs() / parameters.pitch_scale).min(1.0);
        // let desired_pitch = -pitch_angle; // * base_pitch_factor * pitch_scaling;

        // let base_roll_factor = parameters.roll_factor;
        // let roll_scaling = (roll_angle.abs() / parameters.roll_scale).min(1.0);
        // let desired_roll = -roll_angle; // * base_roll_factor * roll_scaling;

        match support_side {
            Side::Right => {
                self.pitch_low_pass_filter.update(desired_pitch);

                motor_commands.positions.left_leg.ankle_roll = desired_roll;
                motor_commands.positions.left_leg.ankle_pitch = self.pitch_low_pass_filter.state();

                motor_commands.stiffnesses.right_leg.ankle_pitch = 0.0;
                motor_commands.stiffnesses.right_leg.ankle_roll = 0.0;

                motor_commands.stiffnesses.left_leg.knee_pitch = 0.0;
            }

            Side::Left => {
                self.pitch_low_pass_filter.update(desired_pitch);
                motor_commands.positions.right_leg.ankle_roll = desired_roll;
                motor_commands.positions.right_leg.ankle_pitch = self.pitch_low_pass_filter.state();

                motor_commands.stiffnesses.left_leg.ankle_pitch = 0.0;
                motor_commands.stiffnesses.left_leg.ankle_roll = 0.0;

                motor_commands.stiffnesses.right_leg.knee_pitch = 0.0;
            }
        }

        context
            .desired_pitch
            .fill_if_subscribed(|| self.pitch_low_pass_filter.state());

        Ok(MainOutputs {
            optimized_motor_commands: motor_commands.into(),
        })
    }
}
