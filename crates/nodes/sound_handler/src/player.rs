use std::{
    io::{ErrorKind, Read},
    path::Path,
    time::{Duration, Instant},
};

use alsa::{
    Direction, PCM, ValueOr,
    pcm::{Access, Format, HwParams},
};
use color_eyre::{Result, eyre::WrapErr, eyre::bail, eyre::ensure};
use hound::{SampleFormat, WavReader, WavSpec};
use tokio_util::sync::CancellationToken;

const FRAMES_PER_CHUNK: usize = 1024;
const DEVICE_WAIT_MILLISECONDS: u32 = 10;

pub(super) fn play_wav(
    path: &Path,
    hardware_device_name: &str,
    timeout: Duration,
    stop: &CancellationToken,
) -> Result<()> {
    let control = PlaybackControl {
        started_at: Instant::now(),
        timeout,
        stop,
    };
    control.check()?;
    let reader =
        WavReader::open(path).wrap_err_with(|| format!("failed to open WAV {}", path.display()))?;
    let spec = reader.spec();
    validate_spec(spec)?;
    control.check()?;

    // Nonblocking mode bounds waits so cancellation and timeouts remain
    // observable even if the output device stops consuming samples.
    let device = create_device(hardware_device_name, spec)?;
    let result = stream_wav(reader, &device, &control);
    if result.is_err() {
        // Best-effort stop: preserve the original error if dropping queued
        // frames also fails. Closing the PCM below releases the device.
        let _ = device.drop();
    }
    result
}

fn validate_spec(spec: WavSpec) -> Result<()> {
    ensure!(
        spec.sample_format == SampleFormat::Int && spec.bits_per_sample == 16,
        "sounds must be 16-bit PCM WAV files"
    );
    ensure!(spec.channels > 0, "WAV must have at least one channel");
    ensure!(spec.sample_rate > 0, "WAV sample rate must be nonzero");
    Ok(())
}

fn create_device(name: &str, spec: WavSpec) -> Result<PCM> {
    let device = PCM::new(name, Direction::Playback, true)
        .wrap_err_with(|| format!("failed to open ALSA playback device {name}"))?;
    {
        let parameters = HwParams::any(&device).wrap_err("failed to create ALSA parameters")?;
        parameters
            .set_access(Access::RWInterleaved)
            .wrap_err("failed to set interleaved playback")?;
        parameters
            .set_format(Format::s16())
            .wrap_err("failed to set 16-bit PCM playback")?;
        parameters
            .set_channels(u32::from(spec.channels))
            .wrap_err("failed to set playback channels")?;
        // Require the file's rate, rather than silently playing at a different
        // speed. ALSA's default/plughw devices can perform rate conversion.
        parameters
            .set_rate(spec.sample_rate, ValueOr::Nearest)
            .wrap_err("failed to set playback sample rate")?;
        parameters
            .set_buffer_time_near(100_000, ValueOr::Nearest)
            .wrap_err("failed to set playback buffer duration")?;
        device
            .hw_params(&parameters)
            .wrap_err("failed to apply ALSA playback parameters")?;
    }
    device.prepare().wrap_err("failed to prepare playback")?;
    Ok(device)
}

fn stream_wav<R: Read>(
    mut reader: WavReader<R>,
    device: &PCM,
    control: &PlaybackControl<'_>,
) -> Result<()> {
    let channels = usize::from(reader.spec().channels);
    let io = device
        .io_i16()
        .wrap_err("failed to create ALSA sample I/O")?;
    let mut samples = reader.samples::<i16>();
    loop {
        control.check()?;
        // Bound memory usage independently of file size.
        let buffer = samples
            .by_ref()
            .take(FRAMES_PER_CHUNK * channels)
            .collect::<std::result::Result<Vec<_>, _>>()
            .wrap_err("failed to read WAV samples")?;
        if buffer.is_empty() {
            break;
        }
        ensure!(
            buffer.len().is_multiple_of(channels),
            "WAV ends with an incomplete audio frame"
        );
        let mut offset = 0;
        while offset < buffer.len() {
            control.check()?;
            match io.writei(&buffer[offset..]) {
                Ok(0) => bail!("ALSA playback made no progress"),
                Ok(frames) => offset += frames * channels,
                Err(error) => recover_or_wait(device, error)?,
            }
        }
    }

    loop {
        control.check()?;
        match device.drain() {
            Ok(()) => return Ok(()),
            Err(error) => recover_or_wait(device, error)?,
        }
    }
}

fn recover_or_wait(device: &PCM, error: alsa::Error) -> Result<()> {
    match std::io::Error::from_raw_os_error(error.errno()).kind() {
        ErrorKind::BrokenPipe => device
            .prepare()
            .wrap_err("failed to recover playback underrun"),
        ErrorKind::WouldBlock => match device.wait(Some(DEVICE_WAIT_MILLISECONDS)) {
            Ok(_) => Ok(()),
            Err(error)
                if std::io::Error::from_raw_os_error(error.errno()).kind()
                    == ErrorKind::BrokenPipe =>
            {
                device
                    .prepare()
                    .wrap_err("failed to recover playback underrun")
            }
            Err(error) => Err(error).wrap_err("failed to wait for ALSA playback"),
        },
        _ => Err(error).wrap_err("ALSA playback failed"),
    }
}

struct PlaybackControl<'a> {
    started_at: Instant,
    timeout: Duration,
    stop: &'a CancellationToken,
}

impl PlaybackControl<'_> {
    fn check(&self) -> Result<()> {
        ensure!(!self.stop.is_cancelled(), "sound playback was cancelled");
        ensure!(
            self.started_at.elapsed() < self.timeout,
            "sound playback exceeded {:?}",
            self.timeout
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write};

    use hound::WavWriter;
    use tempfile::NamedTempFile;

    use super::*;

    fn spec() -> WavSpec {
        WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        }
    }

    fn wav(spec: WavSpec) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        let mut writer = WavWriter::new(file.as_file_mut(), spec).unwrap();
        for sample in [0, i16::MAX, i16::MIN, 0].repeat(1024 * usize::from(spec.channels)) {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        file
    }

    #[test]
    fn plays_mono_and_stereo_wav_through_alsa_without_a_speaker() {
        for channels in [1, 2] {
            let file = wav(WavSpec { channels, ..spec() });
            play_wav(
                file.path(),
                "null",
                Duration::from_secs(5),
                &CancellationToken::new(),
            )
            .unwrap();
        }
    }

    #[test]
    fn rejects_unsupported_sample_formats() {
        for spec in [
            WavSpec {
                bits_per_sample: 8,
                ..spec()
            },
            WavSpec {
                bits_per_sample: 24,
                ..spec()
            },
            WavSpec {
                bits_per_sample: 32,
                sample_format: SampleFormat::Float,
                ..spec()
            },
            WavSpec {
                channels: 0,
                ..spec()
            },
            WavSpec {
                sample_rate: 0,
                ..spec()
            },
        ] {
            assert!(
                validate_spec(spec).is_err(),
                "unexpectedly accepted {spec:?}"
            );
        }
    }

    #[test]
    fn malformed_wav_is_reported() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"not a WAV file").unwrap();
        assert!(
            play_wav(
                file.path(),
                "null",
                Duration::from_secs(5),
                &CancellationToken::new()
            )
            .is_err()
        );
    }

    #[test]
    fn truncated_samples_are_reported() {
        let file = wav(spec());
        let mut bytes = fs::read(file.path()).unwrap();
        bytes.truncate(bytes.len() - 4);
        fs::write(file.path(), bytes).unwrap();
        let error = play_wav(
            file.path(),
            "null",
            Duration::from_secs(5),
            &CancellationToken::new(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("failed to read WAV samples"));
    }

    #[test]
    fn cancellation_prevents_playback() {
        let file = wav(spec());
        let stop = CancellationToken::new();
        stop.cancel();
        let error = play_wav(file.path(), "null", Duration::from_secs(5), &stop).unwrap_err();
        assert!(error.to_string().contains("cancelled"));
    }

    #[test]
    fn timeout_prevents_playback() {
        let file = wav(spec());
        let error = play_wav(
            file.path(),
            "null",
            Duration::ZERO,
            &CancellationToken::new(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("exceeded"));
    }
}
