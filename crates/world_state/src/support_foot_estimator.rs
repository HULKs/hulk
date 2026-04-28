use booster::{FallDownState, FallDownStateType, ImuState};
use color_eyre::Result;
use kinematics::robot_kinematics::RobotKinematics;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::Robot;
use filtering::hysteresis::less_than_with_hysteresis;
use framework::{MainOutput, PerceptionInput};
use linear_algebra::{Isometry3, Orientation3};
use types::support_foot::Side;

#[derive(Deserialize, Serialize)]
pub struct SupportFootEstimator {
    last_support_side: Side,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    imu_state: PerceptionInput<ImuState, "Motion", "imu_state">,

    fall_down_state: Input<Option<FallDownState>, "fall_down_state?">,
    robot_kinematics: Input<RobotKinematics, "robot_kinematics">,

    switch_hysteresis: Parameter<f32, "support_foot_provider.switch_hysteresis">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub support_foot: MainOutput<Option<Side>>,
}

impl SupportFootEstimator {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_support_side: Side::Left,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        if !matches!(
            context.fall_down_state,
            Some(FallDownState {
                fall_down_state: FallDownStateType::IsReady,
                ..
            })
        ) {
            return Ok(MainOutputs {
                support_foot: None.into(),
            });
        }

        struct Horizontal;

        let imu_state = Self::latest_imu_state(&context);

        let imu_orientation = Orientation3::from_euler_angles(
            imu_state.roll_pitch_yaw.x(),
            imu_state.roll_pitch_yaw.y(),
            imu_state.roll_pitch_yaw.z(),
        )
        .mirror();
        let horizontal_to_robot = Isometry3::<Horizontal, Robot>::from(imu_orientation);
        let robot_to_horizontal = horizontal_to_robot.inverse();

        let left_sole_in_horizontal = robot_to_horizontal
            * context
                .robot_kinematics
                .left_leg
                .sole_to_robot
                .translation();
        let right_sole_in_horizontal = robot_to_horizontal
            * context
                .robot_kinematics
                .right_leg
                .sole_to_robot
                .translation();
        let height_difference = left_sole_in_horizontal.z() - right_sole_in_horizontal.z();

        let support_foot = Some(Self::select_support_side(
            height_difference,
            self.last_support_side,
            *context.switch_hysteresis,
        ));

        if let Some(side) = support_foot {
            self.last_support_side = side;
        }

        Ok(MainOutputs {
            support_foot: support_foot.into(),
        })
    }

    fn select_support_side(
        height_difference: f32,
        last_support_side: Side,
        switch_hysteresis: f32,
    ) -> Side {
        let left_was_lower_last_time = last_support_side == Side::Left;

        let left_sole_is_lower_than_right_sole = less_than_with_hysteresis(
            left_was_lower_last_time,
            height_difference,
            0.0,
            switch_hysteresis,
        );

        if left_sole_is_lower_than_right_sole {
            Side::Left
        } else {
            Side::Right
        }
    }

    fn latest_imu_state(context: &CycleContext) -> ImuState {
        context
            .imu_state
            .persistent
            .iter()
            .chain(context.imu_state.temporary.iter())
            .flat_map(|(_timestamp, imu_states)| imu_states.iter().copied().copied())
            .next_back()
            .unwrap_or_default()
    }
}
