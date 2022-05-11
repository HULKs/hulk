use std::{collections::HashSet, f32::consts::PI, ops::Range, sync::Arc};

use nalgebra::ComplexField;
use rustfft::{num_complex::Complex32, Fft};

use crate::{
    framework::AdditionalOutput,
    hardware::{AUDIO_SAMPLE_RATE, NUMBER_OF_AUDIO_CHANNELS, NUMBER_OF_AUDIO_SAMPLES},
    statistics::{mean, standard_deviation},
};

use super::database::{self, DetectionInfo};

const NUMBER_OF_FREQUENCY_SAMPLES: usize = NUMBER_OF_AUDIO_SAMPLES / 2;

pub struct AdditionalOutputs<'a> {
    pub audio_spectrums: AdditionalOutput<'a, Vec<Vec<(f32, f32)>>>,
    pub detection_infos: AdditionalOutput<'a, Vec<DetectionInfo>>,
}

impl<'a> AdditionalOutputs<'a> {
    pub fn new(
        additional_outputs: &'a mut database::AdditionalOutputs,
        subscribed_additional_outputs: &HashSet<String>,
    ) -> Self {
        Self {
            audio_spectrums: AdditionalOutput::new(
                subscribed_additional_outputs.contains("audio_spectrums"),
                &mut additional_outputs.audio_spectrums,
            ),
            detection_infos: AdditionalOutput::new(
                subscribed_additional_outputs.contains("detection_infos"),
                &mut additional_outputs.detection_infos,
            ),
        }
    }
}

pub fn is_whistle_detected_in_buffer(
    fft: Arc<dyn Fft<f32>>,
    buffers: &[[f32; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS],
    detection_band: &Range<f32>,
    background_noise_scaling: f32,
    whistle_scaling: f32,
    number_of_chunks: usize,
    mut additional_outputs: AdditionalOutputs,
) -> anyhow::Result<[bool; NUMBER_OF_AUDIO_CHANNELS]> {
    let mut audio_spectrums = Vec::new();
    let mut detection_infos = Vec::new();
    let frequency_resolution = AUDIO_SAMPLE_RATE as f32 / NUMBER_OF_AUDIO_SAMPLES as f32;
    let mut is_detected = [false; NUMBER_OF_AUDIO_CHANNELS];
    for (channel, buffer) in buffers.iter().enumerate() {
        let mut buffer: Vec<_> = buffer
            .iter()
            .enumerate()
            .map(|(i, &sample)| {
                let hann = (PI * i as f32 / NUMBER_OF_AUDIO_SAMPLES as f32)
                    .sin()
                    .powi(2);
                Complex32::new(hann * sample, 0.0)
            })
            .collect();
        fft.process(&mut buffer);
        let absolute_values: Vec<_> = buffer
            .iter()
            .take(NUMBER_OF_FREQUENCY_SAMPLES)
            .map(|sample| {
                let normalized_sample = sample * 1.0 / (NUMBER_OF_FREQUENCY_SAMPLES as f32).sqrt();
                normalized_sample.abs()
            })
            .collect();
        if additional_outputs.audio_spectrums.is_subscribed() {
            let spectrum = absolute_values
                .iter()
                .enumerate()
                .map(|(i, &value)| (i as f32 * frequency_resolution, value))
                .collect();
            audio_spectrums.push(spectrum);
        }
        let (detected, detection_info) = spectrum_contains_whistle(
            &absolute_values,
            detection_band,
            number_of_chunks,
            background_noise_scaling,
            whistle_scaling,
            frequency_resolution,
        );
        is_detected[channel] = detected;
        detection_infos.push(detection_info);
    }
    additional_outputs
        .audio_spectrums
        .on_subscription(move || audio_spectrums);
    additional_outputs
        .detection_infos
        .on_subscription(move || detection_infos);

    Ok(is_detected)
}

fn spectrum_contains_whistle(
    absolute_values: &[f32],
    detection_band: &Range<f32>,
    number_of_chunks: usize,
    background_noise_scaling: f32,
    whistle_scaling: f32,
    frequency_resolution: f32,
) -> (bool, DetectionInfo) {
    let overall_mean = mean(absolute_values);
    let overall_standard_deviation = standard_deviation(absolute_values, overall_mean);
    let background_noise_threshold =
        overall_mean + background_noise_scaling * overall_standard_deviation;
    let whistle_threshold = overall_mean + whistle_scaling * overall_standard_deviation;
    let min_frequency_index = (detection_band.start / frequency_resolution).ceil() as usize;
    let max_frequency_index = (detection_band.end / frequency_resolution).ceil() as usize;
    let band_size = max_frequency_index - min_frequency_index;
    let band_values: Vec<_> = absolute_values
        .iter()
        .skip(min_frequency_index)
        .take(band_size)
        .cloned()
        .collect();
    let band_mean = mean(&band_values);
    let chunk_size = band_size / number_of_chunks;
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
