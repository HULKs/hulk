use nalgebra::Vector3;
use splines::Interpolate;
use types::{
    joints::{body::BodyJoints, leg::LegJoints},
    support_foot::Side,
};

use super::{feet::parabolic_return, step_state::StepState, CycleContext};

pub trait GyroBalancing {
    fn balance_using_gyro(
        self,
        step: &StepState,
        gyro: Vector3<f32>,
        gyro_balance_factors: &LegJoints<f32>,
    ) -> BodyJoints<f32>;
}

impl GyroBalancing for BodyJoints<f32> {
    fn balance_using_gyro(
        self,
        step: &StepState,
        gyro: Vector3<f32>,
        gyro_balance_factors: &LegJoints<f32>,
    ) -> BodyJoints<f32> {
        let (support_leg, swing_leg) = match step.support_side {
            Side::Left => (self.left_leg, self.right_leg),
            Side::Right => (self.right_leg, self.left_leg),
        };
        let balancing = LegJoints {
            ankle_pitch: gyro_balance_factors.ankle_pitch * gyro.y,
            ankle_roll: gyro_balance_factors.ankle_roll * gyro.x,
            hip_pitch: gyro_balance_factors.hip_pitch * gyro.y,
            hip_roll: gyro_balance_factors.hip_roll * gyro.x,
            hip_yaw_pitch: 0.0,
            knee_pitch: gyro_balance_factors.knee_pitch * gyro.y,
        };
        let support_leg = support_leg + balancing;
        let (left_leg, right_leg) = match step.support_side {
            Side::Left => (support_leg, swing_leg),
            Side::Right => (swing_leg, support_leg),
        };
        BodyJoints {
            left_leg,
            right_leg,
            ..self
        }
    }
}

pub trait LevelFeet {
    fn level_feet(self, context: &CycleContext, step: &StepState) -> BodyJoints<f32>;
}

impl LevelFeet for BodyJoints<f32> {
    fn level_feet(self, context: &CycleContext, step: &StepState) -> BodyJoints<f32> {
        let (support_leg, swing_leg) = match step.support_side {
            Side::Left => (self.left_leg, self.right_leg),
            Side::Right => (self.right_leg, self.left_leg),
        };

        let swing_leg = level_swing_foot(context, step, swing_leg);

        let (left_leg, right_leg) = match step.support_side {
            Side::Left => (support_leg, swing_leg),
            Side::Right => (swing_leg, support_leg),
        };
        BodyJoints {
            left_leg,
            right_leg,
            ..self
        }
    }
}

fn level_swing_foot(
    context: &CycleContext,
    step: &StepState,
    swing_leg: LegJoints<f32>,
) -> LegJoints<f32> {
    let torso_pitch = context.sensor_data.inertial_measurement_unit.roll_pitch.y;
    let now = context.cycle_time.start_time;
    let normalized_time = step.normalized_time_since_start(now).clamp(0.0, 1.0);
    let midpoint = 0.2;
    let parabolic_time = parabolic_return(normalized_time, midpoint);

    let ankle_pitch = f32::lerp(
        parabolic_time,
        swing_leg.ankle_pitch,
        swing_leg.ankle_pitch - torso_pitch,
    );
    LegJoints {
        ankle_pitch,
        ..swing_leg
    }
}
