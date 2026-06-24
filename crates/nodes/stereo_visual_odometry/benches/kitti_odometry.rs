use std::{
    env,
    fs::File,
    hint::black_box,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail},
};
use nalgebra as na;
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use stereo_visual_odometry::{
    parameters::StereoVisualOdometryPoseEstimationParameters, pipeline::VisualOdometryPipeline,
};
use types::{stereo_camera_info::StereoCameraInfo, stereo_image_pair::StereoImagePair};
use zip::ZipArchive;

const MODEL_WIDTH: u32 = 544;
const MODEL_HEIGHT: u32 = 448;
const DEFAULT_SEQUENCES: [&str; 11] = [
    "00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10",
];

fn main() -> Result<()> {
    color_eyre::install()?;

    let config = BenchmarkConfig::from_env()?;
    println!(
        "KITTI dataset: {} | sequences: {} | model input: {}x{}",
        config.dataset_root.display(),
        config.sequences.join(","),
        MODEL_WIDTH,
        MODEL_HEIGHT,
    );
    if let Some(max_frames) = config.max_frames {
        println!("frame limit per sequence: {max_frames}");
    }
    println!(
        "timing note: PNG ZIP read/decode/resize/NV12 conversion is measured separately and excluded from visual_odometry_ms"
    );

    let model_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../etc/neural_networks/xfeat-lighterglue.onnx");

    let pose_estimation_parameters = StereoVisualOdometryPoseEstimationParameters {
        minimum_pnp_correspondences: 8,
        ransac_reprojection_threshold_px: 6.0,
        ransac_max_iterations: 100,
        ransac_confidence: 0.99,
        lm_max_iterations: 20,
        lm_initial_lambda: 0.001,
        lm_min_lambda: 1e-7,
        lm_max_lambda: 1e9,
        lm_step_tolerance: 1e-6,
        lm_cost_tolerance: 1e-6,
        lm_huber_threshold_px: 3.0,
        full_weight_disparity_px: 8.0,
        min_disparity_weight: 0.5,
        max_vertical_disparity_px: 3.0,
    };
    let mut aggregate = SequenceMetrics::new("all");

    for sequence in &config.sequences {
        let report = run_sequence(&config, sequence, &model_path, &pose_estimation_parameters)?;
        report.print();
        aggregate.extend(report);
    }

    aggregate.print();
    Ok(())
}

struct BenchmarkConfig {
    dataset_root: PathBuf,
    sequences: Vec<String>,
    max_frames: Option<usize>,
}

impl BenchmarkConfig {
    fn from_env() -> Result<Self> {
        let dataset_root = env::var_os("KITTI_DATASET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../datasets"));
        let sequences = env::var("KITTI_SEQUENCES")
            .ok()
            .map(|sequences| {
                sequences
                    .split(',')
                    .map(str::trim)
                    .filter(|sequence| !sequence.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .filter(|sequences| !sequences.is_empty())
            .unwrap_or_else(|| {
                DEFAULT_SEQUENCES
                    .iter()
                    .map(|sequence| (*sequence).to_owned())
                    .collect()
            });
        let max_frames = env::var("KITTI_MAX_FRAMES")
            .ok()
            .map(|value| value.parse().wrap_err("failed to parse KITTI_MAX_FRAMES"))
            .transpose()?;

        Ok(Self {
            dataset_root,
            sequences,
            max_frames,
        })
    }

    fn gray_zip_path(&self) -> PathBuf {
        self.dataset_root.join("data_odometry_gray.zip")
    }

    fn calib_zip_path(&self) -> PathBuf {
        self.dataset_root.join("data_odometry_calib.zip")
    }

    fn poses_zip_path(&self) -> PathBuf {
        self.dataset_root.join("data_odometry_poses.zip")
    }
}

fn run_sequence(
    config: &BenchmarkConfig,
    sequence: &str,
    model_path: &Path,
    pose_estimation_parameters: &StereoVisualOdometryPoseEstimationParameters,
) -> Result<SequenceMetrics> {
    let mut gray_zip = open_zip(&config.gray_zip_path())?;
    let mut calib_zip = open_zip(&config.calib_zip_path())?;
    let mut poses_zip = open_zip(&config.poses_zip_path())?;
    let poses = read_poses(&mut poses_zip, sequence)?;
    let frame_count = config
        .max_frames
        .map(|max_frames| max_frames.min(poses.len()))
        .unwrap_or(poses.len());
    if frame_count == 0 {
        bail!("sequence {sequence} has no frames");
    }

    let calibration = read_calibration(&mut calib_zip, sequence)?;
    let mut first_left = Some(decode_gray_png(&mut gray_zip, sequence, "image_0", 0)?);
    let first_left_ref = first_left.as_ref().expect("first frame was just decoded");
    let stereo_camera_info =
        stereo_camera_info(&calibration, first_left_ref.width, first_left_ref.height);
    let mut pipeline = VisualOdometryPipeline::new(model_path, stereo_camera_info)?;
    let mut metrics = SequenceMetrics::new(sequence);

    for frame_index in 0..frame_count {
        let prepare_start = Instant::now();
        let left = if frame_index == 0 {
            first_left.take().expect("first frame is available")
        } else {
            decode_gray_png(&mut gray_zip, sequence, "image_0", frame_index)?
        };
        let right = decode_gray_png(&mut gray_zip, sequence, "image_1", frame_index)?;
        let stereo_image_pair = StereoImagePair {
            frame_identifier: frame_index as u32,
            left: kitti_image_to_nv12(left),
            right: kitti_image_to_nv12(right),
        };
        metrics.prepare_durations.push(prepare_start.elapsed());

        let process_start = Instant::now();
        let estimated_previous_to_current =
            pipeline.process(&stereo_image_pair, pose_estimation_parameters)?;
        let process_duration = process_start.elapsed();
        metrics.process_durations.push(process_duration);
        black_box(&estimated_previous_to_current);

        if frame_index > 0 {
            metrics.transitions += 1;
            match estimated_previous_to_current {
                Some(estimated) => metrics.add_accuracy(
                    estimated,
                    ground_truth_previous_to_current(&poses[frame_index - 1], &poses[frame_index]),
                ),
                None => metrics.failed_odometry += 1,
            }
        }
    }

    Ok(metrics)
}

fn open_zip(path: &Path) -> Result<ZipArchive<File>> {
    let file = File::open(path).wrap_err_with(|| format!("failed to open {}", path.display()))?;
    ZipArchive::new(file).wrap_err_with(|| format!("failed to read {}", path.display()))
}

#[derive(Clone)]
struct GrayImage {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

fn decode_gray_png(
    archive: &mut ZipArchive<File>,
    sequence: &str,
    camera: &str,
    frame_index: usize,
) -> Result<GrayImage> {
    let path = format!("dataset/sequences/{sequence}/{camera}/{frame_index:06}.png");
    let mut entry = archive
        .by_name(&path)
        .wrap_err_with(|| format!("failed to open {path}"))?;
    let mut encoded = Vec::with_capacity(entry.size() as usize);
    entry
        .read_to_end(&mut encoded)
        .wrap_err_with(|| format!("failed to read {path}"))?;

    let decoder = png::Decoder::new(Cursor::new(encoded));
    let mut reader = decoder
        .read_info()
        .wrap_err_with(|| format!("failed to read PNG metadata for {path}"))?;
    let mut data = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let output = reader
        .next_frame(&mut data)
        .wrap_err_with(|| format!("failed to decode {path}"))?;
    let gray = match (output.color_type, output.bit_depth) {
        (png::ColorType::Grayscale, png::BitDepth::Eight) => {
            data.truncate(output.buffer_size());
            data
        }
        (png::ColorType::Rgb, png::BitDepth::Eight) => data[..output.buffer_size()]
            .chunks_exact(3)
            .map(|pixel| rgb_to_luma(pixel[0], pixel[1], pixel[2]))
            .collect(),
        (png::ColorType::Rgba, png::BitDepth::Eight) => data[..output.buffer_size()]
            .chunks_exact(4)
            .map(|pixel| rgb_to_luma(pixel[0], pixel[1], pixel[2]))
            .collect(),
        unsupported => bail!("unsupported PNG format for {path}: {unsupported:?}"),
    };

    Ok(GrayImage {
        width: output.width,
        height: output.height,
        data: gray,
    })
}

fn rgb_to_luma(red: u8, green: u8, blue: u8) -> u8 {
    ((0.299 * red as f32) + (0.587 * green as f32) + (0.114 * blue as f32)).round() as u8
}

fn kitti_image_to_nv12(image: GrayImage) -> Image {
    let mut nv12 = resize_bilinear(
        &image.data,
        image.width,
        image.height,
        MODEL_WIDTH,
        MODEL_HEIGHT,
    );
    let luma_len = (MODEL_WIDTH * MODEL_HEIGHT) as usize;
    let chroma_len = luma_len / 2;
    nv12.resize(luma_len + chroma_len, 128);

    Image {
        height: MODEL_HEIGHT,
        width: MODEL_WIDTH,
        encoding: "nv12".to_string(),
        is_bigendian: 0,
        step: MODEL_WIDTH,
        data: Arc::from(nv12.into_boxed_slice()),
        ..Default::default()
    }
}

fn resize_bilinear(
    input: &[u8],
    input_width: u32,
    input_height: u32,
    output_width: u32,
    output_height: u32,
) -> Vec<u8> {
    let mut output = vec![0; (output_width * output_height) as usize];
    let scale_x = input_width as f32 / output_width as f32;
    let scale_y = input_height as f32 / output_height as f32;

    for output_y in 0..output_height {
        let source_y = (output_y as f32 + 0.5) * scale_y - 0.5;
        let y0 = source_y.floor().max(0.0) as u32;
        let y1 = (y0 + 1).min(input_height - 1);
        let y_weight = source_y - y0 as f32;

        for output_x in 0..output_width {
            let source_x = (output_x as f32 + 0.5) * scale_x - 0.5;
            let x0 = source_x.floor().max(0.0) as u32;
            let x1 = (x0 + 1).min(input_width - 1);
            let x_weight = source_x - x0 as f32;

            let top_left = input[(y0 * input_width + x0) as usize] as f32;
            let top_right = input[(y0 * input_width + x1) as usize] as f32;
            let bottom_left = input[(y1 * input_width + x0) as usize] as f32;
            let bottom_right = input[(y1 * input_width + x1) as usize] as f32;
            let top = top_left + (top_right - top_left) * x_weight;
            let bottom = bottom_left + (bottom_right - bottom_left) * x_weight;

            output[(output_y * output_width + output_x) as usize] =
                (top + (bottom - top) * y_weight).round().clamp(0.0, 255.0) as u8;
        }
    }

    output
}

struct Calibration {
    left_projection: [f64; 12],
    right_projection: [f64; 12],
}

fn read_calibration(archive: &mut ZipArchive<File>, sequence: &str) -> Result<Calibration> {
    let path = format!("dataset/sequences/{sequence}/calib.txt");
    let mut entry = archive
        .by_name(&path)
        .wrap_err_with(|| format!("failed to open {path}"))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .wrap_err_with(|| format!("failed to read {path}"))?;

    let mut left_projection = None;
    let mut right_projection = None;
    for line in text.lines() {
        if let Some(values) = line.strip_prefix("P0:") {
            left_projection = Some(parse_projection(values)?);
        } else if let Some(values) = line.strip_prefix("P1:") {
            right_projection = Some(parse_projection(values)?);
        }
    }

    Ok(Calibration {
        left_projection: left_projection.wrap_err("missing P0 calibration")?,
        right_projection: right_projection.wrap_err("missing P1 calibration")?,
    })
}

fn parse_projection(values: &str) -> Result<[f64; 12]> {
    let values = values
        .split_whitespace()
        .map(|value| {
            value
                .parse::<f64>()
                .wrap_err("failed to parse projection value")
        })
        .collect::<Result<Vec<_>>>()?;
    if values.len() != 12 {
        bail!("expected 12 projection values, got {}", values.len());
    }

    let mut projection = [0.0; 12];
    projection.copy_from_slice(&values);
    Ok(projection)
}

fn stereo_camera_info(
    calibration: &Calibration,
    source_width: u32,
    source_height: u32,
) -> StereoCameraInfo {
    StereoCameraInfo {
        left: camera_info(scale_projection(
            calibration.left_projection,
            source_width,
            source_height,
        )),
        right: camera_info(scale_projection(
            calibration.right_projection,
            source_width,
            source_height,
        )),
    }
}

fn scale_projection(mut projection: [f64; 12], source_width: u32, source_height: u32) -> [f64; 12] {
    let scale_x = MODEL_WIDTH as f64 / source_width as f64;
    let scale_y = MODEL_HEIGHT as f64 / source_height as f64;

    for index in 0..4 {
        projection[index] *= scale_x;
        projection[4 + index] *= scale_y;
    }

    projection
}

fn camera_info(projection: [f64; 12]) -> CameraInfo {
    CameraInfo {
        height: MODEL_HEIGHT,
        width: MODEL_WIDTH,
        p: projection,
        ..Default::default()
    }
}

fn read_poses(archive: &mut ZipArchive<File>, sequence: &str) -> Result<Vec<na::Isometry3<f32>>> {
    let path = format!("dataset/poses/{sequence}.txt");
    let mut entry = archive
        .by_name(&path)
        .wrap_err_with(|| format!("failed to open {path}"))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .wrap_err_with(|| format!("failed to read {path}"))?;

    text.lines().map(parse_pose).collect()
}

fn parse_pose(line: &str) -> Result<na::Isometry3<f32>> {
    let values = line
        .split_whitespace()
        .map(|value| value.parse::<f32>().wrap_err("failed to parse pose value"))
        .collect::<Result<Vec<_>>>()?;
    if values.len() != 12 {
        bail!("expected 12 pose values, got {}", values.len());
    }

    let rotation = na::Matrix3::new(
        values[0], values[1], values[2], values[4], values[5], values[6], values[8], values[9],
        values[10],
    );
    Ok(na::Isometry3::from_parts(
        na::Translation3::new(values[3], values[7], values[11]),
        na::UnitQuaternion::from_rotation_matrix(&na::Rotation3::from_matrix_unchecked(rotation)),
    ))
}

fn ground_truth_previous_to_current(
    previous_pose: &na::Isometry3<f32>,
    current_pose: &na::Isometry3<f32>,
) -> na::Isometry3<f32> {
    current_pose.inverse() * previous_pose
}

struct SequenceMetrics {
    name: String,
    transitions: usize,
    failed_odometry: usize,
    process_durations: Vec<Duration>,
    prepare_durations: Vec<Duration>,
    rotation_errors_degrees: Vec<f32>,
    translation_errors_meters: Vec<f32>,
    translation_relative_errors: Vec<f32>,
    translation_scale_ratios: Vec<f32>,
}

impl SequenceMetrics {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transitions: 0,
            failed_odometry: 0,
            process_durations: Vec::new(),
            prepare_durations: Vec::new(),
            rotation_errors_degrees: Vec::new(),
            translation_errors_meters: Vec::new(),
            translation_relative_errors: Vec::new(),
            translation_scale_ratios: Vec::new(),
        }
    }

    fn add_accuracy(&mut self, estimated: na::Isometry3<f32>, ground_truth: na::Isometry3<f32>) {
        let rotation_error = (ground_truth.rotation.inverse() * estimated.rotation)
            .angle()
            .to_degrees();
        let translation_error =
            (estimated.translation.vector - ground_truth.translation.vector).norm();
        let ground_truth_translation = ground_truth.translation.vector.norm();
        let estimated_translation = estimated.translation.vector.norm();

        self.rotation_errors_degrees.push(rotation_error);
        self.translation_errors_meters.push(translation_error);
        if ground_truth_translation > 1e-3 {
            self.translation_relative_errors
                .push(translation_error / ground_truth_translation);
            self.translation_scale_ratios
                .push(estimated_translation / ground_truth_translation);
        }
    }

    fn extend(&mut self, mut sequence: SequenceMetrics) {
        self.transitions += sequence.transitions;
        self.failed_odometry += sequence.failed_odometry;
        self.process_durations
            .append(&mut sequence.process_durations);
        self.prepare_durations
            .append(&mut sequence.prepare_durations);
        self.rotation_errors_degrees
            .append(&mut sequence.rotation_errors_degrees);
        self.translation_errors_meters
            .append(&mut sequence.translation_errors_meters);
        self.translation_relative_errors
            .append(&mut sequence.translation_relative_errors);
        self.translation_scale_ratios
            .append(&mut sequence.translation_scale_ratios);
    }

    fn print(&self) {
        let visual_odometry = DurationSummary::from(self.process_durations.as_slice());
        let preparation = DurationSummary::from(self.prepare_durations.as_slice());
        let rotation = FloatSummary::from(self.rotation_errors_degrees.as_slice());
        let translation = FloatSummary::from(self.translation_errors_meters.as_slice());
        let translation_relative = FloatSummary::from(self.translation_relative_errors.as_slice());
        let scale_ratio = FloatSummary::from(self.translation_scale_ratios.as_slice());
        let successes = self.rotation_errors_degrees.len();
        let success_rate = if self.transitions > 0 {
            successes as f32 / self.transitions as f32 * 100.0
        } else {
            0.0
        };

        println!(
            "sequence {}: frames={} transitions={} success={} failed={} success_rate={:.1}%",
            self.name,
            self.process_durations.len(),
            self.transitions,
            successes,
            self.failed_odometry,
            success_rate,
        );
        println!(
            "  visual_odometry_ms avg={:.3} median={:.3} p95={:.3} p99={:.3} min={:.3} max={:.3} fps(avg)={:.2}",
            visual_odometry.average_ms,
            visual_odometry.median_ms,
            visual_odometry.p95_ms,
            visual_odometry.p99_ms,
            visual_odometry.min_ms,
            visual_odometry.max_ms,
            1000.0 / visual_odometry.average_ms,
        );
        println!(
            "  excluded_prepare_ms avg={:.3} median={:.3} p95={:.3} p99={:.3}",
            preparation.average_ms, preparation.median_ms, preparation.p95_ms, preparation.p99_ms,
        );
        println!(
            "  accuracy rotation_deg avg={:.3} median={:.3} p95={:.3} p99={:.3}; translation_m avg={:.3} median={:.3} p95={:.3} p99={:.3}",
            rotation.average,
            rotation.median,
            rotation.p95,
            rotation.p99,
            translation.average,
            translation.median,
            translation.p95,
            translation.p99,
        );
        println!(
            "  translation_relative avg={:.3}% median={:.3}% p95={:.3}% p99={:.3}%; translation_scale_ratio avg={:.3} median={:.3} p95={:.3} p99={:.3}",
            translation_relative.average * 100.0,
            translation_relative.median * 100.0,
            translation_relative.p95 * 100.0,
            translation_relative.p99 * 100.0,
            scale_ratio.average,
            scale_ratio.median,
            scale_ratio.p95,
            scale_ratio.p99,
        );
    }
}

struct DurationSummary {
    average_ms: f64,
    median_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    min_ms: f64,
    max_ms: f64,
}

impl From<&[Duration]> for DurationSummary {
    fn from(durations: &[Duration]) -> Self {
        if durations.is_empty() {
            return Self {
                average_ms: 0.0,
                median_ms: 0.0,
                p95_ms: 0.0,
                p99_ms: 0.0,
                min_ms: 0.0,
                max_ms: 0.0,
            };
        }

        let mut milliseconds = durations
            .iter()
            .map(|duration| duration.as_secs_f64() * 1000.0)
            .collect::<Vec<_>>();
        milliseconds.sort_by(f64::total_cmp);
        let average_ms = milliseconds.iter().sum::<f64>() / milliseconds.len() as f64;

        Self {
            average_ms,
            median_ms: percentile(&milliseconds, 0.50),
            p95_ms: percentile(&milliseconds, 0.95),
            p99_ms: percentile(&milliseconds, 0.99),
            min_ms: milliseconds[0],
            max_ms: milliseconds[milliseconds.len() - 1],
        }
    }
}

struct FloatSummary {
    average: f32,
    median: f32,
    p95: f32,
    p99: f32,
}

impl From<&[f32]> for FloatSummary {
    fn from(values: &[f32]) -> Self {
        if values.is_empty() {
            return Self {
                average: 0.0,
                median: 0.0,
                p95: 0.0,
                p99: 0.0,
            };
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(f32::total_cmp);
        Self {
            average: sorted.iter().sum::<f32>() / sorted.len() as f32,
            median: percentile_f32(&sorted, 0.50),
            p95: percentile_f32(&sorted, 0.95),
            p99: percentile_f32(&sorted, 0.99),
        }
    }
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
    let index = ((values.len() - 1) as f64 * percentile).round() as usize;
    values[index]
}

fn percentile_f32(values: &[f32], percentile: f32) -> f32 {
    let index = ((values.len() - 1) as f32 * percentile).round() as usize;
    values[index]
}
