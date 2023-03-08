use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use nalgebra::{Isometry2, Translation2, UnitComplex, Vector2};
use types::{RobotKinematics, Side, SupportFoot};

pub struct Odometry {
    last_orientation: UnitComplex<f32>,
    last_left_sole_to_right_sole: Vector2<f32>,
    last_accumulated_odometry: Isometry2<f32>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub accumulated_odometry: AdditionalOutput<Isometry2<f32>, "accumulated_odometry">,

    pub robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    pub robot_orientation: Input<UnitComplex<f32>, "robot_orientation">,
    pub support_foot: Input<SupportFoot, "support_foot">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub current_odometry_to_last_odometry: MainOutput<Option<Isometry2<f32>>>,
}

impl Odometry {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_left_sole_to_right_sole: Vector2::zeros(),
            last_orientation: UnitComplex::default(),
            last_accumulated_odometry: Isometry2::identity(),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let left_sole_to_right_sole = (context
            .robot_kinematics
            .right_sole_to_robot
            .translation
            .vector
            - context
                .robot_kinematics
                .left_sole_to_robot
                .translation
                .vector)
            .xy();
        let offset_to_last_position = calculate_offset_to_last_position(
            context.support_foot,
            &left_sole_to_right_sole,
            &self.last_left_sole_to_right_sole,
        );
        self.last_left_sole_to_right_sole = left_sole_to_right_sole;

        let orientation_offset = self.last_orientation.rotation_to(context.robot_orientation);
        self.last_orientation = *context.robot_orientation;

        let current_odometry_to_last_odometry = Isometry2::from_parts(
            Translation2::from(offset_to_last_position),
            orientation_offset,
        );
        let accumulated_odometry =
            current_odometry_to_last_odometry * self.last_accumulated_odometry;
        context
            .accumulated_odometry
            .fill_if_subscribed(|| accumulated_odometry);
        self.last_accumulated_odometry = accumulated_odometry;

        Ok(MainOutputs {
            current_odometry_to_last_odometry: Some(current_odometry_to_last_odometry).into(),
        })
    }
}

fn calculate_offset_to_last_position(
    support_foot: &SupportFoot,
    left_sole_to_right_sole: &Vector2<f32>,
    last_left_sole_to_right_sole: &Vector2<f32>,
) -> Vector2<f32> {
    match support_foot.support_side {
        Some(Side::Left) => (left_sole_to_right_sole - last_left_sole_to_right_sole) / 2.0,
        Some(Side::Right) => (-left_sole_to_right_sole + last_left_sole_to_right_sole) / 2.0,
        None => Vector2::zeros(),
    }
}
