use std::{f32::consts::PI, time::Duration};

use color_eyre::{eyre::Ok, Result};
use framework::MainOutput;
use linear_algebra::Vector2;
use serde::{Deserialize, Serialize};
use types::{
    motion_command::OrientationMode, parameters::BehaviorParameters, planned_path::PathSegment,
    stand_up::RemainingStandUpDuration,
};

#[derive(Deserialize, Serialize)]
pub struct TimeToReachKickPosition {}

use context_attribute::context;
#[context]
pub struct CycleContext {
    dribble_path_plan: Input<Option<(OrientationMode, Vec<PathSegment>)>, "dribble_path_plan?">,

    configuration: Parameter<BehaviorParameters, "behavior">,

    stand_up_back_estimated_remaining_duration:
        CyclerState<RemainingStandUpDuration, "stand_up_back_estimated_remaining_duration">,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct MainOutputs {
    pub time_to_reach_kick_position: MainOutput<Option<Duration>>,
}

impl TimeToReachKickPosition {
    pub fn new(_: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let Some((orientation_mode, dribble_path)) = context.dribble_path_plan else {
            return Ok(MainOutputs {
                time_to_reach_kick_position: None.into(),
            });
        };

        let walk_time = dribble_path
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
            .sum();
        let walk_duration = Duration::from_secs_f32(walk_time);

        let turn_angle = match orientation_mode {
            OrientationMode::Override(orientation) => orientation.angle().abs(),
            _ => match dribble_path.first() {
                Some(PathSegment::LineSegment(line_segment)) => {
                    line_segment.1.coords().angle(&Vector2::x_axis()).abs()
                }
                _ => 0.0,
            },
        };
        let turn_duration = context
            .configuration
            .path_planning
            .half_rotation
            .mul_f32(turn_angle / PI);

        let time_to_reach_kick_position = [
            Some(walk_duration),
            (*context.stand_up_back_estimated_remaining_duration).into(),
            Some(turn_duration),
        ]
        .into_iter()
        .flatten()
        .fold(Duration::ZERO, Duration::saturating_add);

        Ok(MainOutputs {
            time_to_reach_kick_position: Some(time_to_reach_kick_position).into(),
        })
    }
}
