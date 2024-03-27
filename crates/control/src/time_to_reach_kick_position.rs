use std::time::Duration;

use color_eyre::Result;
use framework::AdditionalOutput;
use serde::{Deserialize, Serialize};
use types::{
    motion_selection::{MotionSelection, MotionVariant},
    parameters::BehaviorParameters,
    planned_path::PathSegment,
};

#[derive(Deserialize, Serialize)]
pub struct TimeToReachKickPosition {}

use context_attribute::context;
#[context]
pub struct CycleContext {
    dribble_path: Input<Option<Vec<PathSegment>>, "dribble_path?">,
    stand_up_back_remaining_duration: Input<Duration, "stand_up_back.remaining_duration">,
    stand_up_front_remaining_duration: Input<Duration, "stand_up_front.remaining_duration">,
    motion_selection: Input<MotionSelection, "motion_selection">,

    time_to_reach_kick_position_output:
        AdditionalOutput<Option<Duration>, "time_to_reach_kick_position_output">,

    time_to_reach_kick_position: CyclerState<Duration, "time_to_reach_kick_position">,

    configuration: Parameter<BehaviorParameters, "behavior">,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct MainOutputs {}

impl TimeToReachKickPosition {
    pub fn new(_: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let walk_time = context
            .dribble_path
            .as_ref()
            .map(|path| {
                path.iter()
                    .map(|segment: &PathSegment| {
                        let length = segment.length();
                        match segment {
                            PathSegment::LineSegment(_) => {
                                length / context.configuration.path_planning.line_walking_speed
                            }
                            PathSegment::Arc(_, _) => {
                                length / context.configuration.path_planning.arc_walking_speed
                            }
                        }
                    })
                    .sum()
            })
            .map(Duration::from_secs_f32);
        let time_to_stand_up = match context.motion_selection.current_motion {
            MotionVariant::StandUpBack => *context.stand_up_back_remaining_duration,
            MotionVariant::StandUpFront => *context.stand_up_front_remaining_duration,
            _ => Duration::ZERO,
        };
        let time_to_reach_kick_position = walk_time.map(|walk_time| walk_time + time_to_stand_up);

        context
            .time_to_reach_kick_position_output
            .fill_if_subscribed(|| time_to_reach_kick_position);
        // 1800 seconds is 30 minutes, which is essentially maximum as it pertains to game time.
        // Prevents Duration::MAX from breaking the behavior simulator.
        *context.time_to_reach_kick_position = time_to_reach_kick_position
            .unwrap_or(Duration::MAX)
            .min(Duration::from_secs(1800));

        Ok(MainOutputs {})
    }
}
