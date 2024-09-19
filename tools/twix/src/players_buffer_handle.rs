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
        let mut buffers = Players::new();
        buffers
            .inner
            .insert(0, nao.subscribe_value(format!("{prefix}.one.{path}")));

        Ok(Self(buffers))
    }
}
