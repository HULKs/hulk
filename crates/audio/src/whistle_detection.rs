use std::{f32::consts::PI, sync::Arc};

use color_eyre::Result;
use context_attribute::context;
use filtering::statistics::{mean, standard_deviation};
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use rustfft::{
    num_complex::{Complex32, ComplexFloat},
    num_traits::Zero,
    Fft, FftPlanner,
};
use serde::{Deserialize, Serialize};
use types::{
    parameters::WhistleDetectionParameters,
    samples::Samples,
    whistle::{DetectionInfo, Whistle},
};

pub const AUDIO_SAMPLE_RATE: u32 = 44100;
pub const NUMBER_OF_AUDIO_CHANNELS: usize = 4;
pub const NUMBER_OF_AUDIO_SAMPLES: usize = 2048;
const NUMBER_OF_FREQUENCY_SAMPLES: usize = NUMBER_OF_AUDIO_SAMPLES / 2;

#[derive(Deserialize, Serialize)]
pub struct WhistleDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    fft: Arc<dyn Fft<f32>>,
    #[serde(skip)]
    scratch: Vec<Complex32>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    parameters: Parameter<WhistleDetectionParameters, "whistle_detection">,

    samples: Input<Samples, "samples">,
    audio_spectrums: AdditionalOutput<Vec<Vec<(f32, f32)>>, "audio_spectrums">,
    detection_infos: AdditionalOutput<Vec<DetectionInfo>, "detection_infos">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_whistle: MainOutput<Whistle>,
}

impl WhistleDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(NUMBER_OF_AUDIO_SAMPLES);
        let scratch = vec![Complex32::zero(); fft.get_inplace_scratch_len()];
        Ok(Self { fft, scratch })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        context.audio_spectrums.fill_if_subscribed(Vec::new);
        context.detection_infos.fill_if_subscribed(Vec::new);
        let is_detected = context
            .samples
            .channels_of_samples
            .iter()
            .map(|buffer| {
                self.is_whistle_detected_in_buffer(
                    buffer,
                    context.parameters,
                    &mut context.audio_spectrums,
                    &mut context.detection_infos,
                )
            })
            .collect();
        Ok(MainOutputs {
            detected_whistle: Whistle { is_detected }.into(),
        })
    }

    fn is_whistle_detected_in_buffer(
        &mut self,
        buffer: &[f32],
        detection_parameters: &WhistleDetectionParameters,
        audio_spectrums: &mut AdditionalOutput<Vec<Vec<(f32, f32)>>>,
        detection_infos: &mut AdditionalOutput<Vec<DetectionInfo>>,
    ) -> bool {
        let frequency_resolution = AUDIO_SAMPLE_RATE as f32 / NUMBER_OF_AUDIO_SAMPLES as f32;
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
        self.fft
            .process_with_scratch(&mut buffer, &mut self.scratch);
        let absolute_values: Vec<_> = buffer
            .iter()
            .take(NUMBER_OF_FREQUENCY_SAMPLES)
            .map(|sample| {
                let normalized_sample = sample * 1.0 / (NUMBER_OF_FREQUENCY_SAMPLES as f32).sqrt();
                normalized_sample.abs()
            })
            .collect();
        audio_spectrums.mutate_if_subscribed(|spectrums| {
            let spectrum = absolute_values
                .iter()
                .enumerate()
                .map(|(i, &value)| (i as f32 * frequency_resolution, value))
                .collect();
            if let Some(spectrums) = spectrums {
                spectrums.push(spectrum);
            }
        });
        let (detected, detection_info) =
            spectrum_contains_whistle(&absolute_values, detection_parameters, frequency_resolution);
        detection_infos.mutate_if_subscribed(|infos| {
            if let Some(infos) = infos {
                infos.push(detection_info);
            }
        });
        detected
    }
}

fn spectrum_contains_whistle(
    absolute_values: &[f32],
    detection_parameters: &WhistleDetectionParameters,
    frequency_resolution: f32,
) -> (bool, DetectionInfo) {
    let WhistleDetectionParameters {
        detection_band,
        background_noise_scaling,
        whistle_scaling,
        number_of_chunks,
    } = detection_parameters;
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
