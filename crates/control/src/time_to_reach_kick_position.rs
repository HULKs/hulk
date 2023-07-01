use color_eyre::Result;
use types::PathSegment;

use std::time::Duration;

use context_attribute::context;
#[context]
pub struct CycleContext {
    pub dribble_path: Input<Option<Vec<PathSegment>>, "dribble_path?">,
    pub time_to_reach_kick_position: PersistentState<Duration, "time_to_reach_kick_position">,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct MainOutputs {}

pub struct TimeToReachKickPosition {}

impl TimeToReachKickPosition {
    pub fn new(_: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let time_to_reach_kick_position = context
            .dribble_path
            .as_ref()
            .map(|path| {
                path.iter()
                    .map(|segment: &PathSegment| segment.length())
                    .sum()
            })
            .map(Duration::from_secs_f32);
        *context.time_to_reach_kick_position =
            time_to_reach_kick_position.unwrap_or(Duration::from_secs(1800));
        /*1800 seconds is 30 minutes, which is essentially max as it pertains to game time and prevents Duration::MAX from breaking the behavior sim
         */
        Ok(MainOutputs {})
    }
}
