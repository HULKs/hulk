use std::{f32::consts::FRAC_1_PI, time::Duration};

use color_eyre::{eyre::Ok, Result};
use framework::MainOutput;
use linear_algebra::Vector2;
use serde::{Deserialize, Serialize};
use types::{
    motion_command::OrientationMode, parameters::BehaviorParameters, planned_path::PathSegment,
};

#[derive(Deserialize, Serialize)]
pub struct TimeToReachKickPosition {}

use context_attribute::context;
#[context]
pub struct CycleContext {
    dribble_path_plan: Input<Option<(OrientationMode, Vec<PathSegment>)>, "dribble_path_plan?">,

    configuration: Parameter<BehaviorParameters, "behavior">,

    stand_up_back_estimated_remaining_duration:
        CyclerState<Duration, "stand_up_back_estimated_remaining_duration">,
    stand_up_front_estimated_remaining_duration:
        CyclerState<Duration, "stand_up_front_estimated_remaining_duration">,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct MainOutputs {
    pub time_to_reach_kick_position: MainOutput<Duration>,
}

impl TimeToReachKickPosition {
    pub fn new(_: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let Some((orientation_mode, dribble_path)) = context.dribble_path_plan else {
            return Ok(MainOutputs {
                time_to_reach_kick_position: Duration::MAX.into(),
            });
        };
        let walk_time = Duration::from_secs_f32(
            dribble_path
                .iter()
                .map(|segment: &PathSegment| {
                    let length = segment.length();
                    match segment {
                        PathSegment::LineSegment(_) => {
                            length / context.configuration.path_planning.line_walking_speed
                        }
                        PathSegment::Arc(_) => {
                            length / context.configuration.path_planning.arc_walking_speed
                        }
                    }
                })
                .sum(),
        );
        let turning_angle = match orientation_mode {
            OrientationMode::Override(orientation) => Some(orientation.angle().abs()),
            _ => {
                let turning_angle_towards_path = match dribble_path.first() {
                    Some(PathSegment::LineSegment(line_segment)) => {
                        Some(line_segment.1.coords().angle(&Vector2::x_axis()).abs())
                    }
                    _ => None,
                };
                turning_angle_towards_path
            }
        };
        let time_to_turn = turning_angle.map_or(Duration::ZERO, |angle| {
            context
                .configuration
                .path_planning
                .half_rotation
                .mul_f32(angle * FRAC_1_PI)
        });

        let time_to_reach_kick_position = [
            walk_time,
            *context.stand_up_back_estimated_remaining_duration,
            *context.stand_up_front_estimated_remaining_duration,
            time_to_turn,
        ]
        .into_iter()
        .fold(Duration::ZERO, Duration::saturating_add);

        Ok(MainOutputs {
            time_to_reach_kick_position: time_to_reach_kick_position.into(),
        })
    }
}
