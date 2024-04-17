use types::{
    joints::body::{BodyJoints, LowerBodyJoints, UpperBodyJoints},
    motor_commands::MotorCommands,
};

pub trait Stiffness {
    fn apply_stiffness(self, legs: f32, arms: f32) -> MotorCommands<BodyJoints<f32>>;
}

impl Stiffness for BodyJoints<f32> {
    fn apply_stiffness(self, legs: f32, arms: f32) -> MotorCommands<BodyJoints<f32>> {
        let stiffnesses = BodyJoints::from_lower_and_upper(
            LowerBodyJoints::fill(legs),
            UpperBodyJoints::fill(arms),
        );
        MotorCommands {
            positions: self,
            stiffnesses,
        }
    }
}
