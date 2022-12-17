use std::ops::Range;

use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{hardware::Samples, Whistle};

pub struct WhistleDetection {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    // Parameter statt WhistleDetectionConfiguration
    pub background_noise_scaling: Parameter<f32, "whistle_detection.background_noise_scaling">,
    pub detection_band: Parameter<Range<f32>, "whistle_detection.detection_band">,
    pub number_of_chunks: Parameter<usize, "whistle_detection.number_of_chunks">,
    pub whistle_scaling: Parameter<f32, "whistle_detection.whistle_scaling">,

    pub samples: Input<Samples, "samples">,
    pub audio_spectrums: AdditionalOutput<Vec<Vec<(f32, f32)>>, "audio_spectrums">,
    // pub detection_infos: AdditionalOutput<Vec<DetectionInfo>, "detection_infos">, // DetectionInfos bisher in src/audio/database.rs
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_whistle: MainOutput<Whistle>,
}

impl WhistleDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }
    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}

/*
use std::{f32::consts::PI, ops::Range, sync::Arc};

use Result;
use macros::{module, require_some};
use nalgebra::ComplexField;
use rustfft::{num_complex::Complex32, Fft, FftPlanner};

use crate::{
    hardware::{AUDIO_SAMPLE_RATE, NUMBER_OF_AUDIO_CHANNELS, NUMBER_OF_AUDIO_SAMPLES},
    statistics::{mean, standard_deviation},
    types::Whistle,
};

use types::WhistleDetection as WhistleDetectionConfiguration;

use crate::audio::database::{AudioSamples, DetectionInfo};

const NUMBER_OF_FREQUENCY_SAMPLES: usize = NUMBER_OF_AUDIO_SAMPLES / 2;

pub struct WhistleDetection {
    fft: Arc<dyn Fft<f32>>,
}

#[module(audio)]
#[input(path = samples, data_type = AudioSamples)]
#[parameter(path = audio.whistle_detection, data_type = WhistleDetectionConfiguration)]
#[additional_output(path = audio_spectrums, data_type = Vec<Vec<(f32, f32)>>)]
#[additional_output(path = detection_infos, data_type = Vec<DetectionInfo>)]
#[main_output(name = detected_whistle, data_type = Whistle)]
impl WhistleDetection {}

impl WhistleDetection {
    fn new(_context: CreationContext) -> Result<Self> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(NUMBER_OF_AUDIO_SAMPLES);
        Ok(Self { fft })
    }

    fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs {
            detected_whistle: Some(Whistle {
                is_detected: is_whistle_detected_in_buffer(
                    self.fft.clone(),
                    require_some!(context.samples),
                    &context.whistle_detection.detection_band,
                    context.whistle_detection.background_noise_scaling,
                    context.whistle_detection.whistle_scaling,
                    context.whistle_detection.number_of_chunks,
                    &mut context,
                )?,
            }),
        })
    }
}

fn is_whistle_detected_in_buffer(
    fft: Arc<dyn Fft<f32>>,
    buffers: &[[f32; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS],
    detection_band: &Range<f32>,
    background_noise_scaling: f32,
    whistle_scaling: f32,
    number_of_chunks: usize,
    context: &mut CycleContext,
) -> Result<[bool; NUMBER_OF_AUDIO_CHANNELS]> {
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
        if context.audio_spectrums.is_subscribed() {
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
    context
        .audio_spectrums
        .on_subscription(move || audio_spectrums);
    context
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
 */
