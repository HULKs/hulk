use std::{
    collections::HashMap,
    sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError},
    thread::{spawn, JoinHandle},
    time::Duration,
};

use alsa::{
    pcm::{Access, Format, HwParams},
    Direction, ValueOr, PCM,
};
use color_eyre::{eyre::WrapErr, Result};
use log::{error, warn};
use opusfile_ng::OggOpusFile;
use serde::Deserialize;
use types::{
    audio::{Sound, SpeakerRequest},
    hardware::Paths,
};

use crate::audio_parameter_deserializers::{deserialize_access, deserialize_format};

pub struct Speakers {
    worker_sender: Option<SyncSender<SpeakerRequest>>,
    worker: Option<JoinHandle<()>>,
}

impl Speakers {
    pub fn new(parameters: Parameters, paths: &Paths) -> Result<Self> {
        let device = Self::initialize_playback_device(&parameters)
            .wrap_err("failed to initialize playback device")?;
        let sounds =
            Self::load_sounds(paths, parameters.volume).wrap_err("failed to loads sounds")?;
        let (sender, receiver) = sync_channel(5);
        let worker = Some(spawn(move || worker(device, sounds, receiver)));
        Ok(Self {
            worker_sender: Some(sender),
            worker,
        })
    }

    fn initialize_playback_device(parameters: &Parameters) -> Result<PCM> {
        let device = PCM::new("default", Direction::Playback, false)
            .wrap_err("failed to open audio device")?;
        {
            let hardware_parameters =
                HwParams::any(&device).wrap_err("failed to create hardware parameters")?;
            hardware_parameters
                .set_access(parameters.access)
                .wrap_err("failed to set access")?;
            hardware_parameters
                .set_format(parameters.format)
                .wrap_err("failed to set format")?;
            hardware_parameters
                .set_rate_near(parameters.sample_rate, ValueOr::Nearest)
                .wrap_err("failed to set sample rate")?;
            hardware_parameters
                .set_channels(parameters.number_of_channels as u32)
                .wrap_err("failed to set channel")?;
            hardware_parameters
                .set_buffer_time_near(
                    parameters.buffer_time.as_micros().try_into().unwrap(),
                    ValueOr::Nearest,
                )
                .wrap_err("failed to set buffer time")?;
            device
                .hw_params(&hardware_parameters)
                .wrap_err("failed to set hardware parameters")?;
        }
        Ok(device)
    }

    fn load_sounds(paths: &Paths, volume: f32) -> Result<HashMap<Sound, Vec<f32>>> {
        let mut sounds = HashMap::new();
        for sound in Sound::all() {
            let file_name = match sound {
                Sound::Ball => "ball.ogg",
                Sound::Bishop => "bishop.ogg",
                Sound::CameraReset => "cameraReset.ogg",
                Sound::CenterCircle => "centerCircle.ogg",
                Sound::Corner => "corner.ogg",
                Sound::DefenderLeft => "defenderLeft.ogg",
                Sound::Defender => "defender.ogg",
                Sound::DefenderRight => "defenderRight.ogg",
                Sound::Donk => "donk.ogg",
                Sound::Drift => "drift.ogg",
                Sound::FalsePositiveDetected => "falsePositiveDetected.ogg",
                Sound::FalsePositive => "falsePositive.ogg",
                Sound::FrontLeft => "frontLeft.ogg",
                Sound::Front => "front.ogg",
                Sound::FrontRight => "frontRight.ogg",
                Sound::InvalidImage => "invalidImage.ogg",
                Sound::Keeper => "keeper.ogg",
                Sound::Left => "left.ogg",
                Sound::LolaDesync => "lolaDesync.ogg",
                Sound::Ouch => "ouch.ogg",
                Sound::PenaltyArea => "penaltyArea.ogg",
                Sound::PenaltySpot => "penaltySpot.ogg",
                Sound::RearLeft => "rearLeft.ogg",
                Sound::Rear => "rear.ogg",
                Sound::RearRight => "rearRight.ogg",
                Sound::ReplacementKeeper => "replacementKeeper.ogg",
                Sound::Right => "right.ogg",
                Sound::SameNumberTuhhNao21 => "sameNumbertuhhNao21.ogg",
                Sound::SameNumberTuhhNao22 => "sameNumbertuhhNao22.ogg",
                Sound::SameNumberTuhhNao23 => "sameNumbertuhhNao23.ogg",
                Sound::SameNumberTuhhNao24 => "sameNumbertuhhNao24.ogg",
                Sound::SameNumberTuhhNao25 => "sameNumbertuhhNao25.ogg",
                Sound::SameNumberTuhhNao26 => "sameNumbertuhhNao26.ogg",
                Sound::SameNumberTuhhNao27 => "sameNumbertuhhNao27.ogg",
                Sound::SameNumberTuhhNao28 => "sameNumbertuhhNao28.ogg",
                Sound::SameNumberTuhhNao29 => "sameNumbertuhhNao29.ogg",
                Sound::SameNumberTuhhNao30 => "sameNumbertuhhNao30.ogg",
                Sound::SameNumberTuhhNao31 => "sameNumbertuhhNao31.ogg",
                Sound::SameNumberTuhhNao32 => "sameNumbertuhhNao32.ogg",
                Sound::SameNumberTuhhNao33 => "sameNumbertuhhNao33.ogg",
                Sound::SameNumberTuhhNao34 => "sameNumbertuhhNao34.ogg",
                Sound::SameNumberTuhhNao35 => "sameNumbertuhhNao35.ogg",
                Sound::SameNumberTuhhNao36 => "sameNumbertuhhNao36.ogg",
                Sound::SameNumberUnknownHULKDeviceEth => "sameNumberUnknownHULKDeviceETH.ogg",
                Sound::SameNumberUnknownHULKDeviceWifi => "sameNumberUnknownHULKDeviceWIFI.ogg",
                Sound::Sigh => "sigh.ogg",
                Sound::Squat => "squat.ogg",
                Sound::Striker => "striker.ogg",
                Sound::Supporter => "supporter.ogg",
                Sound::TJunction => "tJunction.ogg",
                Sound::UsbStickMissing => "usbStickMissing.ogg",
                Sound::Weeeee => "weeeee.ogg",
            };
            let path = paths.sounds.join(file_name);
            let file = OggOpusFile::open_file(&path)
                .wrap_err_with(|| format!("failed to open sound file {path:?}"))?;
            let number_of_samples = file.pcm_total(-1).wrap_err_with(|| {
                format!("failed to get number of samples of sound file {path:?}")
            })?;
            let mut samples = Vec::with_capacity(number_of_samples);
            let mut buffer = [0.0; 2048];
            loop {
                let read_bytes = file
                    .read_float(&mut buffer, None)
                    .wrap_err_with(|| format!("failed to read sample of sound file {path:?}"))?;
                if read_bytes == 0 {
                    break;
                }
                for sample in &mut buffer[..read_bytes] {
                    *sample *= volume;
                }
                samples.extend(&buffer[..read_bytes]);
            }
            sounds.insert(*sound, samples);
        }
        Ok(sounds)
    }

    pub fn write_to_speakers(&self, request: SpeakerRequest) {
        match self.worker_sender.as_ref().unwrap().try_send(request) {
            Ok(_) => {}
            Err(TrySendError::Full(request)) => {
                warn!("speaker queue is full, dropping {request:?}");
            }
            Err(TrySendError::Disconnected(_)) => {
                panic!("receiver should always wait for all senders");
            }
        }
    }
}

impl Drop for Speakers {
    fn drop(&mut self) {
        drop(self.worker_sender.take());
        if let Some(worker) = self.worker.take() {
            worker.join().expect("failed to join worker");
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Parameters {
    sample_rate: u32,
    number_of_channels: usize,
    buffer_time: Duration,
    volume: f32,

    #[serde(deserialize_with = "deserialize_access")]
    access: Access,
    #[serde(deserialize_with = "deserialize_format")]
    format: Format,
}

fn worker(device: PCM, sounds: HashMap<Sound, Vec<f32>>, receiver: Receiver<SpeakerRequest>) {
    while let Ok(SpeakerRequest::PlaySound { sound }) = receiver.recv() {
        let samples = sounds
            .get(&sound)
            .expect("missing sound, recheck Sound::all()");
        let io = device
            .io_f32()
            .expect("f32 device should always be available");
        if let Err(error) = device.prepare() {
            error!("device.prepare(): {error:?}");
        }
        if let Err(error) = io.writei(samples) {
            error!("device.writei(): {error:?}");
        }
        if let Err(error) = device.drain() {
            error!("device.drain(): {error:?}");
        }
    }
}
