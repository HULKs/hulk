use std::{
    boxed::Box,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use color_eyre::Result;

use kinematics::joints::head::HeadJoints;
use ros_z::prelude::*;
use types::{
    field_dimensions::GlobalFieldSide,
    filtered_game_controller_state::FilteredGameControllerState,
    initial_look_around::{
        BallSearchLookAround, InitialLookAround, LookAroundMode, QuickLookAround,
    },
    motion_command::{HeadMotion, MotionCommand},
    parameters::LookAroundParameters,
    support_foot::Side,
    time_wrapper::TimeWrapper,
};

const MAX_INPUT_DRAIN_PER_TICK: usize = 10;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("look_around").build().await?;

    let parameters = node.bind_parameter_as::<LookAroundParameters>("look_around")?;
    let motion_command_sub = node
        .subscriber::<MotionCommand>("motion_command")?
        .build()
        .await?;
    let filtered_game_controller_state_sub = node
        .subscriber::<TimeWrapper<FilteredGameControllerState>>("filtered_game_controller_state")?
        .build()
        .await?;
    let current_mode_pub = node
        .publisher::<LookAroundMode>("look_around_mode")?
        .build()
        .await?;
    let look_around_target_joints_pub = node
        .publisher::<HeadJoints<f32>>("look_around_target_joints")?
        .build()
        .await?;

    let mut state = LookAroundState::new();
    let mut latest_motion_command = MotionCommand::default();
    let mut latest_filtered_game_controller_state = None;
    let mut tick = tokio::time::interval(Duration::from_millis(10));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            motion_command = motion_command_sub.recv() => {
                latest_motion_command = motion_command?;
            }
            filtered_game_controller_state = filtered_game_controller_state_sub.recv() => {
                latest_filtered_game_controller_state = Some(filtered_game_controller_state?.inner);
            }
            _ = tick.tick() => {
                for _ in 0..MAX_INPUT_DRAIN_PER_TICK {
                    if !motion_command_sub.is_ready() {
                        break;
                    }

                    latest_motion_command = motion_command_sub.recv().await?;
                }

                for _ in 0..MAX_INPUT_DRAIN_PER_TICK {
                    if !filtered_game_controller_state_sub.is_ready() {
                        break;
                    }

                    latest_filtered_game_controller_state =
                        Some(filtered_game_controller_state_sub.recv().await?.inner);
                }

                let now = SystemTime::now();
                let parameters_snapshot = parameters.snapshot();
                let parameters = parameters_snapshot.typed();

                state.update_head_motion(
                    &latest_motion_command,
                    latest_filtered_game_controller_state.as_ref(),
                    now,
                );

                match latest_motion_command.head_motion() {
                    Some(HeadMotion::LookAround) => {
                        state.advance_after_timeout(now, parameters.look_around_timeout);
                    }
                    Some(HeadMotion::SearchForLostBall) => {
                        state.advance_after_timeout(now, parameters.quick_search_timeout);
                    }
                    _ => {}
                }

                let current_mode = state.current_mode;
                current_mode_pub
                    .publish_if_subscribed(|| async move { current_mode })
                    .await?;
                let target_joints = target_joints_for_mode(state.current_mode, parameters);
                look_around_target_joints_pub
                    .publish(&target_joints)
                    .await?;
            }
        }
    }
}

#[derive(Debug)]
struct LookAroundState {
    current_mode: LookAroundMode,
    last_mode_switch: SystemTime,
    last_head_motion: Option<HeadMotion>,
}

impl LookAroundState {
    fn new() -> Self {
        Self {
            current_mode: LookAroundMode::Initial(Default::default()),
            last_mode_switch: UNIX_EPOCH,
            last_head_motion: None,
        }
    }

    fn update_head_motion(
        &mut self,
        motion_command: &MotionCommand,
        filtered_game_controller_state: Option<&FilteredGameControllerState>,
        now: SystemTime,
    ) {
        let head_motion = motion_command.head_motion();

        if self.last_head_motion != head_motion {
            self.last_mode_switch = now;
            self.current_mode = match head_motion {
                Some(HeadMotion::LookAround) => filtered_game_controller_state.map_or(
                    LookAroundMode::Initial(Default::default()),
                    |filtered_game_controller_state| {
                        if filtered_game_controller_state.global_field_side == GlobalFieldSide::Home
                        {
                            LookAroundMode::Initial(InitialLookAround::Left)
                        } else {
                            LookAroundMode::Initial(InitialLookAround::Right)
                        }
                    },
                ),
                Some(HeadMotion::SearchForLostBall) => {
                    LookAroundMode::QuickSearch(Default::default())
                }
                _ => LookAroundMode::Center,
            };
        }

        if !matches!(
            head_motion,
            Some(HeadMotion::LookAround | HeadMotion::SearchForLostBall)
        ) {
            self.current_mode = LookAroundMode::Center;
        }

        self.last_head_motion = head_motion;
    }

    fn advance_after_timeout(&mut self, now: SystemTime, timeout: Duration) {
        let elapsed = match now.duration_since(self.last_mode_switch) {
            Ok(elapsed) => elapsed,
            Err(_) => {
                self.last_mode_switch = now;
                return;
            }
        };

        if elapsed < timeout {
            return;
        }

        self.last_mode_switch = now;
        self.current_mode = match self.current_mode {
            LookAroundMode::Center => LookAroundMode::Center,
            LookAroundMode::BallSearch(state) => LookAroundMode::BallSearch(state.next()),
            LookAroundMode::QuickSearch(state) => LookAroundMode::QuickSearch(state.next()),
            LookAroundMode::Initial(state) => LookAroundMode::Initial(state.next()),
        };
    }
}

trait NextMode {
    fn next(&self) -> Self;
}

impl NextMode for BallSearchLookAround {
    fn next(&self) -> Self {
        match self {
            BallSearchLookAround::Center {
                moving_towards: Side::Left,
            } => BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::Center {
                moving_towards: Side::Right,
            } => BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::Left => BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::Right => BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Left,
            } => BallSearchLookAround::Left,
            BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Right,
            } => BallSearchLookAround::Center {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Left,
            } => BallSearchLookAround::Center {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Right,
            } => BallSearchLookAround::Right,
        }
    }
}

impl NextMode for InitialLookAround {
    fn next(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

impl NextMode for QuickLookAround {
    fn next(&self) -> Self {
        let mode = match self.mode {
            BallSearchLookAround::Center {
                moving_towards: Side::Left,
            } => BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::Center {
                moving_towards: Side::Right,
            } => BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::Left => BallSearchLookAround::HalfwayLeft {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::Right => BallSearchLookAround::HalfwayRight {
                moving_towards: Side::Left,
            },
            BallSearchLookAround::HalfwayLeft { .. } => BallSearchLookAround::Center {
                moving_towards: Side::Right,
            },
            BallSearchLookAround::HalfwayRight { .. } => BallSearchLookAround::Center {
                moving_towards: Side::Left,
            },
        };
        Self { mode }
    }
}

fn target_joints_for_mode(
    mode: LookAroundMode,
    parameters: &LookAroundParameters,
) -> HeadJoints<f32> {
    match mode {
        LookAroundMode::Center => parameters.middle_positions,
        LookAroundMode::QuickSearch(QuickLookAround { mode: state })
        | LookAroundMode::BallSearch(state) => match state {
            BallSearchLookAround::Center { .. } => parameters.middle_positions,
            BallSearchLookAround::Left => parameters.left_positions,
            BallSearchLookAround::Right => parameters.right_positions,
            BallSearchLookAround::HalfwayLeft { .. } => parameters.halfway_left_positions,
            BallSearchLookAround::HalfwayRight { .. } => parameters.halfway_right_positions,
        },
        LookAroundMode::Initial(state) => match state {
            InitialLookAround::Left => parameters.initial_left_positions,
            InitialLookAround::Right => parameters.initial_right_positions,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use types::{
        field_dimensions::GlobalFieldSide,
        initial_look_around::{BallSearchLookAround, InitialLookAround, QuickLookAround},
        motion_command::{HeadMotion, ImageRegion},
        support_foot::Side,
    };

    use super::*;

    #[test]
    fn entering_look_around_selects_initial_mode() {
        let now = UNIX_EPOCH + Duration::from_secs(1);
        let mut game_controller_state = FilteredGameControllerState::default();
        game_controller_state.global_field_side = GlobalFieldSide::Home;
        let mut state = LookAroundState::new();

        state.update_head_motion(
            &MotionCommand::Stand {
                head: HeadMotion::LookAround,
            },
            Some(&game_controller_state),
            now,
        );

        assert_eq!(
            state.current_mode,
            LookAroundMode::Initial(InitialLookAround::Left)
        );
        assert_eq!(state.last_mode_switch, now);
        assert_eq!(state.last_head_motion, Some(HeadMotion::LookAround));
    }

    #[test]
    fn non_search_head_motion_centers() {
        let now = UNIX_EPOCH + Duration::from_secs(1);
        let mut state = LookAroundState::new();

        state.update_head_motion(
            &MotionCommand::Stand {
                head: HeadMotion::Center {
                    image_region_target: ImageRegion::Center,
                },
            },
            None,
            now,
        );

        assert_eq!(state.current_mode, LookAroundMode::Center);
        assert_eq!(
            state.last_head_motion,
            Some(HeadMotion::Center {
                image_region_target: ImageRegion::Center,
            })
        );
    }

    #[test]
    fn quick_search_advances_after_timeout() {
        let timeout = Duration::from_millis(10);
        let mut state = LookAroundState {
            current_mode: LookAroundMode::QuickSearch(QuickLookAround {
                mode: BallSearchLookAround::Center {
                    moving_towards: Side::Left,
                },
            }),
            last_mode_switch: UNIX_EPOCH,
            last_head_motion: Some(HeadMotion::SearchForLostBall),
        };

        state.advance_after_timeout(UNIX_EPOCH + timeout - Duration::from_millis(1), timeout);
        assert_eq!(
            state.current_mode,
            LookAroundMode::QuickSearch(QuickLookAround {
                mode: BallSearchLookAround::Center {
                    moving_towards: Side::Left,
                },
            })
        );

        state.advance_after_timeout(UNIX_EPOCH + timeout, timeout);
        assert_eq!(
            state.current_mode,
            LookAroundMode::QuickSearch(QuickLookAround {
                mode: BallSearchLookAround::HalfwayLeft {
                    moving_towards: Side::Right,
                },
            })
        );
    }

    #[test]
    fn backwards_time_resets_mode_switch_without_advancing() {
        let now = UNIX_EPOCH + Duration::from_secs(1);
        let mut state = LookAroundState {
            current_mode: LookAroundMode::QuickSearch(QuickLookAround {
                mode: BallSearchLookAround::Center {
                    moving_towards: Side::Left,
                },
            }),
            last_mode_switch: UNIX_EPOCH + Duration::from_secs(2),
            last_head_motion: Some(HeadMotion::SearchForLostBall),
        };

        state.advance_after_timeout(now, Duration::ZERO);

        assert_eq!(
            state.current_mode,
            LookAroundMode::QuickSearch(QuickLookAround {
                mode: BallSearchLookAround::Center {
                    moving_towards: Side::Left,
                },
            })
        );
        assert_eq!(state.last_mode_switch, now);
    }

    #[test]
    fn target_joints_follow_current_mode() {
        let parameters = LookAroundParameters {
            look_around_timeout: Duration::ZERO,
            quick_search_timeout: Duration::ZERO,
            middle_positions: head_joints(0.0),
            left_positions: head_joints(1.0),
            right_positions: head_joints(2.0),
            halfway_left_positions: head_joints(3.0),
            halfway_right_positions: head_joints(4.0),
            initial_left_positions: head_joints(5.0),
            initial_right_positions: head_joints(6.0),
        };

        assert_eq!(
            target_joints_for_mode(LookAroundMode::Center, &parameters),
            parameters.middle_positions
        );
        assert_eq!(
            target_joints_for_mode(
                LookAroundMode::QuickSearch(QuickLookAround {
                    mode: BallSearchLookAround::HalfwayRight {
                        moving_towards: Side::Left,
                    },
                }),
                &parameters,
            ),
            parameters.halfway_right_positions
        );
        assert_eq!(
            target_joints_for_mode(
                LookAroundMode::Initial(InitialLookAround::Right),
                &parameters,
            ),
            parameters.initial_right_positions
        );
    }

    fn head_joints(value: f32) -> HeadJoints<f32> {
        HeadJoints {
            yaw: value,
            pitch: value + 0.5,
        }
    }
}
