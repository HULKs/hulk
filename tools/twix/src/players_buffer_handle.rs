use std::sync::Arc;

use color_eyre::Result;

use spl_network_messages::bindings::MAX_NUM_PLAYERS;
use types::players::Players;

use crate::{nao::Nao, value_buffer::BufferHandle};

pub struct PlayersBufferHandle<T>(pub Players<BufferHandle<T>>);

impl<T> PlayersBufferHandle<T>
where
    for<'de> T: serde::Deserialize<'de> + Send + Sync + 'static,
{
    pub fn try_new(nao: Arc<Nao>, prefix: &str, path: &str) -> Result<Self> {
        let mut buffers = Players::new();
        for player in 1..=MAX_NUM_PLAYERS {
            buffers.inner.insert(
                player.into(),
                nao.subscribe_value(format!("{prefix}.{player}.{path}")),
            );
        }
        Ok(Self(buffers))
    }
}
