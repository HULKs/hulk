use std::{boxed::Box, future::Future, pin::Pin};
use std::{f32::consts::PI, sync::Arc};

use color_eyre::{Result, eyre::ensure};
use filtering::statistics::{mean, standard_deviation};
use log::warn;
use realfft::{
    RealFftPlanner, RealToComplex,
    num_complex::{Complex32, ComplexFloat},
};

use ros_z::prelude::*;

use types::{
    parameters::WhistleDetectionParameters,
    samples::Samples,
    whistle::{AudioSpectrum, AudioSpectrumBin, DetectionInfo, Whistle},
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

fn validate_whistle_detection_parameters(parameters: &WhistleDetectionParameters) -> Result<()> {
    ensure!(
        parameters.number_of_chunks > 0,
        "number_of_chunks must be greater than zero"
    );
    ensure!(
        parameters.number_audio_samples >= 2,
        "number_audio_samples must be at least 2"
    );

    ensure!(
        parameters.audio_sample_rate > 0,
        "audio_sample_rate must be greater than zero"
    );
    ensure!(
        parameters.detection_band.start.is_finite()
            && parameters.detection_band.end.is_finite()
            && parameters.detection_band.start >= 0.0
            && parameters.detection_band.start < parameters.detection_band.end,
        "detection_band must have finite bounds with 0 <= start < end"
    );

    let frequency_resolution =
        parameters.audio_sample_rate as f32 / parameters.number_audio_samples as f32;
    let min_frequency_index =
        (parameters.detection_band.start / frequency_resolution).ceil() as usize;
    let max_frequency_index =
        (parameters.detection_band.end / frequency_resolution).ceil() as usize;
    let band_size = max_frequency_index
        .min(parameters.number_audio_samples / 2)
        .saturating_sub(min_frequency_index);

    ensure!(
        band_size >= parameters.number_of_chunks,
        "detection_band contains {band_size} frequency bins, but number_of_chunks is {}",
        parameters.number_of_chunks,
    );

    Ok(())
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("whistle_detection").build().await?;

    let parameters = node.bind_parameter_as::<WhistleDetectionParameters>("whistle_detection")?;
    parameters.add_validation_hook(|parameters| {
        validate_whistle_detection_parameters(parameters).map_err(|error| error.to_string())
    })?;
    let samples_sub = node
        .subscriber::<Samples>("inputs/microphones_samples")
        .build()
        .await?;
    let audio_spectrums_pub = node
        .publisher::<Vec<AudioSpectrum>>("audio_spectrums")
        .build()
        .await?;
    let detection_infos_pub = node
        .publisher::<Vec<DetectionInfo>>("detection_infos")
        .build()
        .await?;
    let detected_whistle_pub = node
        .publisher::<Whistle>("detected_whistle")
        .build()
        .await?;

    let mut whistle_detection = WhistleDetection::new(parameters.snapshot().typed())?;

    loop {
        let samples = samples_sub.recv().await?;
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();
        let collect_spectrums = audio_spectrums_pub.has_subscribers();
        let collect_detection_infos = detection_infos_pub.has_subscribers();

        let (is_detected, audio_spectrums, detection_infos) = tokio::task::block_in_place(|| {
            let mut is_detected = Vec::new();
            let mut audio_spectrums = Vec::new();
            let mut detection_infos = Vec::new();
            for buffer in samples.channels_of_samples.iter() {
                let (detected, info, magnitudes) =
                    whistle_detection.is_whistle_detected_in_buffer(buffer, parameters);

                if collect_spectrums {
                    let frequency_resolution = parameters.audio_sample_rate as f32
                        / parameters.number_audio_samples as f32;
                    audio_spectrums.push(to_audio_spectrum(magnitudes, frequency_resolution));
                }

                if collect_detection_infos {
                    detection_infos.push(info);
                }

                is_detected.push(detected);
            }

            (is_detected, audio_spectrums, detection_infos)
        });

        detected_whistle_pub
            .publish(&Whistle { is_detected })
            .await?;

        if collect_spectrums {
            audio_spectrums_pub.publish(&audio_spectrums).await?;
        }

        if collect_detection_infos {
            detection_infos_pub.publish(&detection_infos).await?;
        }
    }
}

pub struct WhistleDetection {
    fft: Arc<dyn RealToComplex<f32>>,
    input: Vec<f32>,
    output: Vec<Complex32>,
    scratch: Vec<Complex32>,
    hann_window: Vec<f32>,
    magnitudes: Vec<f32>,
}

impl WhistleDetection {
    pub fn new(detection_parameters: &WhistleDetectionParameters) -> Result<Self> {
        let mut planner = RealFftPlanner::new();
        let fft = planner.plan_fft_forward(detection_parameters.number_audio_samples);
        let input = fft.make_input_vec();
        let output = fft.make_output_vec();
        let scratch = fft.make_scratch_vec();
        let hann_window = (0..detection_parameters.number_audio_samples)
            .map(|i| {
                (PI * i as f32 / detection_parameters.number_audio_samples as f32)
                    .sin()
                    .powi(2)
            })
            .collect();

        Ok(Self {
            fft,
            input,
            output,
            scratch,
            hann_window,
            magnitudes: vec![0.0; detection_parameters.number_audio_samples / 2],
        })
    }

    fn is_whistle_detected_in_buffer(
        &mut self,
        buffer: &[f32],
        detection_parameters: &WhistleDetectionParameters,
    ) -> (bool, DetectionInfo, &[f32]) {
        let expected_samples = self.fft.len();
        if buffer.len() != expected_samples
            || detection_parameters.number_audio_samples != expected_samples
        {
            warn!(
                "ignoring audio buffer with {} samples: FFT expects {} and number_audio_samples is {}",
                buffer.len(),
                expected_samples,
                detection_parameters.number_audio_samples,
            );
            return (false, DetectionInfo::default(), &[]);
        }

        let frequency_resolution = detection_parameters.audio_sample_rate as f32
            / detection_parameters.number_audio_samples as f32;
        for ((input, &sample), &hann) in self.input.iter_mut().zip(buffer).zip(&self.hann_window) {
            *input = hann * sample;
        }
        if let Err(error) =
            self.fft
                .process_with_scratch(&mut self.input, &mut self.output, &mut self.scratch)
        {
            warn!("ignoring audio buffer: real FFT failed: {error}");
            return (false, DetectionInfo::default(), &[]);
        }

        // Keep the original N/2 bins and normalization, excluding realfft's final bin even for odd N.
        let number_frequency_samples = detection_parameters.number_audio_samples / 2;
        for (magnitude, sample) in self.magnitudes.iter_mut().zip(&self.output) {
            let normalized_sample = sample * 1.0 / (number_frequency_samples as f32).sqrt();
            *magnitude = normalized_sample.abs();
        }

        let (detected, info) =
            spectrum_contains_whistle(&self.magnitudes, detection_parameters, frequency_resolution);
        (detected, info, &self.magnitudes)
    }
}

fn to_audio_spectrum(magnitudes: &[f32], frequency_resolution: f32) -> AudioSpectrum {
    magnitudes
        .iter()
        .enumerate()
        .map(|(i, &magnitude)| AudioSpectrumBin {
            frequency: i as f32 * frequency_resolution,
            magnitude,
        })
        .collect()
}

fn spectrum_contains_whistle(
    absolute_values: &[f32],
    detection_parameters: &WhistleDetectionParameters,
    frequency_resolution: f32,
) -> (bool, DetectionInfo) {
    let overall_mean = mean(absolute_values);
    let overall_standard_deviation = standard_deviation(absolute_values, overall_mean);
    let background_noise_threshold =
        overall_mean + detection_parameters.background_noise_scaling * overall_standard_deviation;
    let whistle_threshold =
        overall_mean + detection_parameters.whistle_scaling * overall_standard_deviation;
    let min_frequency_index =
        (detection_parameters.detection_band.start / frequency_resolution).ceil() as usize;
    let max_frequency_index =
        (detection_parameters.detection_band.end / frequency_resolution).ceil() as usize;
    let band_size = max_frequency_index
        .min(absolute_values.len())
        .saturating_sub(min_frequency_index);
    let chunk_size = band_size / detection_parameters.number_of_chunks;
    let band_size = chunk_size * detection_parameters.number_of_chunks;
    let max_frequency_index = min_frequency_index + band_size;
    let band_start = min_frequency_index.min(absolute_values.len());
    let band_values = &absolute_values[band_start..band_start + band_size];
    let band_mean = mean(band_values);

    let mut detection_info = DetectionInfo {
        overall_mean,
        std_deviation: overall_standard_deviation,
        background_noise_threshold,
        whistle_threshold,
        min_frequency_index,
        max_frequency_index,
        band_size,
        chunk_size,
        whistle_mean: None,
        band_mean,
        lower_whistle_chunk: None,
        upper_whistle_chunk: None,
        lower_band_index: None,
        upper_band_index: None,
    };

    let lower_whistle_chunk =
        band_values
            .chunks_exact(chunk_size)
            .enumerate()
            .find_map(|(chunk_index, chunk)| {
                if mean(chunk) > background_noise_threshold {
                    Some(chunk_index)
                } else {
                    None
                }
            });

    detection_info.lower_whistle_chunk = lower_whistle_chunk;
    let lower_whistle_chunk = match lower_whistle_chunk {
        Some(index) => index,
        None => return (false, detection_info),
    };

    let upper_whistle_chunk = band_values
        .chunks_exact(chunk_size)
        .rev()
        .enumerate()
        .find_map(|(chunk_index, chunk)| {
            if mean(chunk) > background_noise_threshold {
                Some(chunk_index)
            } else {
                None
            }
        });

    detection_info.upper_whistle_chunk = upper_whistle_chunk;
    let upper_whistle_chunk = match upper_whistle_chunk {
        Some(index) => index,
        None => return (false, detection_info),
    };

    let lower_band_index = min_frequency_index + lower_whistle_chunk * chunk_size;
    let upper_band_index = max_frequency_index - upper_whistle_chunk * chunk_size;
    assert!(upper_band_index >= lower_band_index);
    detection_info.lower_band_index = Some(lower_band_index);
    detection_info.upper_band_index = Some(upper_band_index);

    let whistle_band = &absolute_values[lower_band_index..upper_band_index];
    let whistle_mean = mean(whistle_band);
    detection_info.whistle_mean = Some(whistle_mean);

    (whistle_mean > whistle_threshold, detection_info)
}
