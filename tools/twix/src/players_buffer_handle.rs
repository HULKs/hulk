use std::sync::Arc;

use color_eyre::Result;

use types::players::Players;

use crate::{robot::Robot, value_buffer::BufferHandle};

pub struct PlayersBufferHandle<T>(pub Players<BufferHandle<T>>);

impl<T> PlayersBufferHandle<T>
where
    for<'de> T: serde::Deserialize<'de> + Send + Sync + 'static,
{
    pub fn try_new(robot: Arc<Robot>, prefix: &str, path: &str) -> Result<Self> {
        let buffers = Players {
            one: robot.subscribe_value(format!("{prefix}.one.{path}")),
            two: robot.subscribe_value(format!("{prefix}.two.{path}")),
            three: robot.subscribe_value(format!("{prefix}.three.{path}")),
            four: robot.subscribe_value(format!("{prefix}.four.{path}")),
            five: robot.subscribe_value(format!("{prefix}.five.{path}")),
            six: robot.subscribe_value(format!("{prefix}.six.{path}")),
            seven: robot.subscribe_value(format!("{prefix}.seven.{path}")),
        };

        Ok(Self(buffers))
    }
}
