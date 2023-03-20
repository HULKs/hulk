use std::{str::FromStr, sync::Arc};

use color_eyre::Result;

use communication::client::CyclerOutput;
use types::Players;

use crate::{nao::Nao, value_buffer::ValueBuffer};

pub struct PlayersValueBuffer(pub Players<ValueBuffer>);

impl PlayersValueBuffer {
    pub fn try_new(nao: &Arc<Nao>, prefix: &str, output: &str) -> Result<Self> {
        let buffers = Players {
            one: nao.subscribe_output(CyclerOutput::from_str(&format!("{prefix}.one.{output}"))?),
            two: nao.subscribe_output(CyclerOutput::from_str(&format!("{prefix}.two.{output}"))?),
            three: nao
                .subscribe_output(CyclerOutput::from_str(&format!("{prefix}.three.{output}"))?),
            four: nao.subscribe_output(CyclerOutput::from_str(&format!("{prefix}.four.{output}"))?),
            five: nao.subscribe_output(CyclerOutput::from_str(&format!("{prefix}.five.{output}"))?),
        };

        Ok(Self(buffers))
    }
}
