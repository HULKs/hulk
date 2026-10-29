mod player;

use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc, time::Duration};

use color_eyre::{Result, eyre::WrapErr};
use ros_z::{prelude::*, qos::QosHistory};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use tokio_util::sync::CancellationToken;
use tracing::warn;
use types::audio::{SOUND_REQUEST_TOPIC, Sound, SpeakerRequest};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub enabled: bool,
    pub sounds_directory: PathBuf,
    pub hardware_device_name: String,
    pub playback_timeout: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("sound_handler").build().await?;
    let parameters = node.bind_parameter_as::<Parameters>("sound_handler")?;
    let sound_sub = node
        .subscriber::<SpeakerRequest>(SOUND_REQUEST_TOPIC)
        .qos(QosProfile {
            history: QosHistory::from_depth(1),
            ..Default::default()
        })
        .build()
        .await?;

    loop {
        let SpeakerRequest::PlaySound { sound } = sound_sub.recv().await?;
        let snapshot = parameters.snapshot();
        let parameters = snapshot.typed();
        if !parameters.enabled {
            continue;
        }

        if let Err(error) = play_sound(parameters.clone(), sound).await {
            warn!(%sound, "failed to play sound: {error:#}");
        }
    }
}

fn sound_path(parameters: &Parameters, sound: Sound) -> PathBuf {
    parameters.sounds_directory.join(format!("{sound}.wav"))
}

async fn play_sound(parameters: Parameters, sound: Sound) -> Result<()> {
    let path = sound_path(&parameters, sound);
    let stop = CancellationToken::new();
    let _cancel_on_drop = stop.clone().drop_guard();
    spawn_blocking(move || {
        player::play_wav(
            &path,
            &parameters.hardware_device_name,
            parameters.playback_timeout,
            &stop,
        )
    })
    .await
    .wrap_err("WAV playback worker failed")?
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs};

    use super::*;

    fn parameters() -> Parameters {
        json5::from_str(include_str!(
            "../../../../etc/parameters/base/sound_handler.json5"
        ))
        .unwrap()
    }

    #[test]
    fn every_sound_maps_to_a_wav_in_the_configured_directory() {
        let mut parameters = parameters();
        parameters.sounds_directory = PathBuf::from("sounds with spaces");
        for sound in enum_iterator::all::<Sound>() {
            assert_eq!(
                sound_path(&parameters, sound),
                PathBuf::from(format!("sounds with spaces/{sound}.wav"))
            );
        }
    }

    #[test]
    fn sound_enum_matches_deployed_wav_assets() {
        let mut parameters = parameters();
        parameters.sounds_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join(&parameters.sounds_directory);
        let expected: BTreeSet<_> = enum_iterator::all::<Sound>()
            .map(|sound| sound_path(&parameters, sound))
            .collect();
        let actual: BTreeSet<_> = fs::read_dir(&parameters.sounds_directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "wav"))
            .collect();
        assert_eq!(actual, expected, "Sound variants must match the WAV assets");
    }

    #[tokio::test]
    async fn missing_wav_is_reported_without_opening_the_device() {
        let directory = tempfile::tempdir().unwrap();
        let mut parameters = parameters();
        parameters.sounds_directory = directory.path().to_owned();
        parameters.hardware_device_name = "nonexistent-sound-handler-device".to_owned();
        let error = play_sound(parameters, Sound::Ouch).await.unwrap_err();
        assert!(error.to_string().contains("failed to open WAV"));
    }
}
