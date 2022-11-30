use std::{fs::File, path::Path};

use color_eyre::{
    eyre::{bail, eyre, WrapErr},
    Result,
};
use serde_json::{from_reader, Value};

pub fn to_replay_frame(replay_file_path: impl AsRef<Path>, image_prefix: &str) -> Result<Value> {
    let mut replay_file = File::open(replay_file_path).context("File::open(replay_file_path)")?;
    let replay: Value =
        from_reader(&mut replay_file).context("serde_json::from_reader(&mut replay_file)")?;
    let replay_frames = match replay
        .get("frames")
        .ok_or_else(|| eyre!("replay.get(\"frames\")"))?
    {
        Value::Array(replay_frames) => replay_frames,
        _ => bail!("not Value::Array"),
    };
    let replay_frame = match replay_frames.iter().find(|replay_frame| {
        if let Some(Value::String(image)) = replay_frame
            .get("topImage")
            .or_else(|| replay_frame.get("bottomImage"))
        {
            image.starts_with(image_prefix)
        } else {
            false
        }
    }) {
        Some(replay_frame) => replay_frame,
        None => bail!("missing frame with matching \"topImage\" or \"bottomImage\""),
    };
    Ok(replay_frame.clone())
}
