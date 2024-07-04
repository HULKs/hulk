use std::sync::Arc;

use color_eyre::Result;

use types::players::Players;

use crate::{nao::Nao, value_buffer::BufferHandle};

pub struct PlayersBufferHandle<T>(pub Players<BufferHandle<T>>);

impl<T> PlayersBufferHandle<T>
where
    for<'de> T: serde::Deserialize<'de> + Send + Sync + 'static,
{
    pub fn try_new(nao: Arc<Nao>, prefix: &str, path: &str) -> Result<Self> {
        let buffers = Players {
            one: nao.subscribe_value(format!("{prefix}.one.{path}")),
            two: nao.subscribe_value(format!("{prefix}.two.{path}")),
            three: nao.subscribe_value(format!("{prefix}.three.{path}")),
            four: nao.subscribe_value(format!("{prefix}.four.{path}")),
            five: nao.subscribe_value(format!("{prefix}.five.{path}")),
            six: nao.subscribe_value(format!("{prefix}.six.{path}")),
            seven: nao.subscribe_value(format!("{prefix}.seven.{path}")),
        };

        Ok(Self(buffers))
    }
}
