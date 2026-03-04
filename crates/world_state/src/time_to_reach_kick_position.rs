use std::time::Duration;

use color_eyre::{eyre::Ok, Result};
use serde::{Deserialize, Serialize};

use framework::MainOutput;
use types::world_state::BallState;

#[derive(Deserialize, Serialize)]
pub struct TimeToReachKickPosition {}

use context_attribute::context;
#[context]
pub struct CycleContext {
    //dribble_path_plan: Input<Option<DribblePathPlan>, "dribble_path_plan?">,
    //fall_state: Input<FallState, "fall_state">,
    // configuration: Parameter<BehaviorParameters, "behavior">,
    ball_state: Input<Option<BallState>, "ball_state?">,
    // stand_up_back_estimated_remaining_duration:
    //     CyclerState<RemainingStandUpDuration, "stand_up_back_estimated_remaining_duration">,
    // stand_up_front_estimated_remaining_duration:
    //     CyclerState<RemainingStandUpDuration, "stand_up_front_estimated_remaining_duration">,
    // stand_up_sitting_estimated_remaining_duration:
    //     CyclerState<RemainingStandUpDuration, "stand_up_sitting_estimated_remaining_duration">,
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
        // let Some(DribblePathPlan {
        //     orientation_mode,
        //     path: dribble_path,
        //     ..
        // }) = context.dribble_path_plan
        // else {
        //     return Ok(MainOutputs {
        //         time_to_reach_kick_position: None.into(),
        //     });
        // };

        // let walk_time = dribble_path
        //     .segments
        //     .iter()
        //     .map(|segment: &PathSegment| {
        //         let length = segment.length();
        //         match segment {
        //             PathSegment::LineSegment(_) => {
        //                 length / context.configuration.path_planning.line_walking_speed
        //             }
        //             PathSegment::Arc(_) => {
        //                 length / context.configuration.path_planning.arc_walking_speed
        //             }
        //         }
        //     })
        //     .sum();
        // let walk_duration = Duration::from_secs_f32(walk_time);

        // let turn_angle = match orientation_mode {
        //     OrientationMode::LookTowards { direction, .. } => direction.angle().abs(),
        //     OrientationMode::LookAt { target, .. } => {
        //         target.coords().angle(&Vector2::x_axis()).abs()
        //     }
        //     OrientationMode::Unspecified | OrientationMode::AlignWithPath => {
        //         match dribble_path.segments.first() {
        //             Some(PathSegment::LineSegment(line_segment)) => {
        //                 line_segment.1.coords().angle(&Vector2::x_axis()).abs()
        //             }
        //             _ => 0.0,
        //         }
        //     }
        // };
        // let turn_duration = context
        //     .configuration
        //     .path_planning
        //     .half_rotation
        //     .mul_f32(turn_angle / PI);

        // let stand_up_penalty = if *context.fall_state != FallState::Upright {
        //     context.configuration.time_to_reach_delay_when_fallen
        // } else {
        //     Duration::ZERO
        // };

        // let time_to_reach_kick_position = [
        //     Some(walk_duration),
        //     (*context.stand_up_back_estimated_remaining_duration).into(),
        //     (*context.stand_up_front_estimated_remaining_duration).into(),
        //     (*context.stand_up_sitting_estimated_remaining_duration).into(),
        //     Some(stand_up_penalty),
        //     Some(turn_duration),
        // ]
        // .into_iter()
        // .flatten()
        // .fold(Duration::ZERO, Duration::saturating_add);
        let distance_to_ball = context
            .ball_state
            .as_ref()
            .map(|ball_state| ball_state.ball_in_field.coords().norm())
            .unwrap_or(0.0); // temporaray TODO: change back to real data

        Ok(MainOutputs {
            time_to_reach_kick_position: Some(Duration::from_secs_f32(distance_to_ball)).into(),
        })
    }
}
