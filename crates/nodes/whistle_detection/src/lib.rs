use std::{boxed::Box, future::Future, pin::Pin};
use std::{f32::consts::PI, sync::Arc};

use color_eyre::Result;
use filtering::statistics::{mean, standard_deviation};
use log::warn;
use rustfft::{
    Fft, FftPlanner,
    num_complex::{Complex32, ComplexFloat},
    num_traits::Zero,
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

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("whistle_detection").build().await?;

    let parameters = node.bind_parameter_as::<WhistleDetectionParameters>("whistle_detection")?;
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
        let mut audio_spectrums = Vec::new();

        let (is_detected, detection_infos): (Vec<bool>, Vec<DetectionInfo>) =
            tokio::task::block_in_place(|| {
                samples
                    .channels_of_samples
                    .iter()
                    .map(|buffer| {
                        whistle_detection.is_whistle_detected_in_buffer(
                            buffer,
                            parameters,
                            &mut audio_spectrums,
                        )
                    })
                    .unzip()
            });
        detected_whistle_pub
            .publish(&Whistle { is_detected })
            .await?;

        audio_spectrums_pub.publish(&audio_spectrums).await?;
        detection_infos_pub.publish(&detection_infos).await?;
    }
}

pub struct WhistleDetection {
    fft: Arc<dyn Fft<f32>>,
    scratch: Vec<Complex32>,
}

impl WhistleDetection {
    pub fn new(detection_parameters: &WhistleDetectionParameters) -> Result<Self> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(detection_parameters.number_audio_samples);
        let scratch = vec![Complex32::zero(); fft.get_inplace_scratch_len()];
        Ok(Self { fft, scratch })
    }

    fn is_whistle_detected_in_buffer(
        &mut self,
        buffer: &[f32],
        detection_parameters: &WhistleDetectionParameters,
        audio_spectrums: &mut Vec<AudioSpectrum>,
    ) -> (bool, DetectionInfo) {
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
            audio_spectrums.push(Vec::new());
            return (false, DetectionInfo::default());
        }

        let frequency_resolution = detection_parameters.audio_sample_rate as f32
            / detection_parameters.number_audio_samples as f32;
        let mut buffer: Vec<_> = buffer
            .iter()
            .enumerate()
            .map(|(i, &sample)| {
                let hann = (PI * i as f32 / detection_parameters.number_audio_samples as f32)
                    .sin()
                    .powi(2);
                Complex32::new(hann * sample, 0.0)
            })
            .collect();
        self.fft
            .process_with_scratch(&mut buffer, &mut self.scratch);

        let number_frequency_samples = detection_parameters.number_audio_samples / 2;
        let absolute_values: Vec<_> = buffer
            .iter()
            .take(number_frequency_samples)
            .map(|sample| {
                let normalized_sample = sample * 1.0 / (number_frequency_samples as f32).sqrt();
                normalized_sample.abs()
            })
            .collect();

        let spectrum: AudioSpectrum = absolute_values
            .iter()
            .enumerate()
            .map(|(i, &magnitude)| AudioSpectrumBin {
                frequency: i as f32 * frequency_resolution,
                magnitude,
            })
            .collect();

        audio_spectrums.push(spectrum);

        let (detected, detection_info) =
            spectrum_contains_whistle(&absolute_values, detection_parameters, frequency_resolution);

        (detected, detection_info)
    }
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
    let band_size = max_frequency_index - min_frequency_index;
    let band_values: Vec<_> = absolute_values
        .iter()
        .skip(min_frequency_index)
        .take(band_size)
        .cloned()
        .collect();
    let band_mean = mean(&band_values);
    let chunk_size = band_size / detection_parameters.number_of_chunks;
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
    let whistle_band: Vec<_> = absolute_values
        .iter()
        .skip(lower_band_index)
        .take(upper_band_index - lower_band_index)
        .cloned()
        .collect();
    let whistle_mean = mean(&whistle_band);
    detection_info.whistle_mean = Some(whistle_mean);
    (whistle_mean > whistle_threshold, detection_info)
}
