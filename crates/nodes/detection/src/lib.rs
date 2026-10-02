use std::{boxed::Box, future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::{Result, eyre::bail};
use ndarray::{ArrayView2, ArrayViewD, Axis, IxDyn};
use ort::{
    ep::{CUDA, TensorRT},
    inputs,
    session::{Session, SessionOutputs, builder::GraphOptimizationLevel},
    value::TensorRef,
};
use ros_z_streams::CreateAnnouncingPublisher;
use ros2::sensor_msgs::image::Image;

use ros_z::{prelude::*, qos::QosHistory};
use tokio::{task::block_in_place, time::Instant};
use types::{
    bounding_box::BoundingBox,
    object_detection::{LabelIndex, NUMBER_OF_VALUES_PER_OBJECT, Object, RobocupObjectLabel},
    parameters::DetectionParameters,
    time_wrapper::TimeWrapper,
};

pub const NUMBER_OF_DETECTIONS: usize = 300;

#[derive(Clone, Copy, Debug)]
enum TaskHead {
    HSLVisionObjectDetection,
    NaoObjectDetection,
}

struct DetectionOutput {
    inference_duration: Duration,
    post_processing_duration: Duration,
    non_maximum_suppression_duration: Duration,
    detected_objects: Vec<Object<RobocupObjectLabel>>,
}

impl TaskHead {
    fn output_name(self) -> &'static str {
        match self {
            TaskHead::HSLVisionObjectDetection => "hslvision_output",
            TaskHead::NaoObjectDetection => "nao_output",
        }
    }

    fn expected_shape(self) -> [usize; 3] {
        match self {
            Self::HSLVisionObjectDetection => {
                [1, NUMBER_OF_DETECTIONS, NUMBER_OF_VALUES_PER_OBJECT]
            }
            Self::NaoObjectDetection => [1, NUMBER_OF_DETECTIONS, NUMBER_OF_VALUES_PER_OBJECT],
        }
    }
}

#[derive(Debug)]
struct ModelOutputs<'a> {
    hslvision_objects: ArrayView2<'a, f32>,
    nao_objects: ArrayView2<'a, f32>,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("detection").build().await?;

    let node_parameters = node.bind_parameter_as::<DetectionParameters>("detection")?;
    let mut parameter_receiver = node_parameters.subscribe();

    let image_sub = node
        .subscriber::<Image>("inputs/left_image")
        .qos(QosProfile {
            history: QosHistory::from_depth(1),
            ..Default::default()
        })
        .build()
        .await?;
    let inference_duration_pub = node
        .publisher::<Duration>("inference_duration")
        .build()
        .await?;
    let post_processing_duration_pub = node
        .publisher::<Duration>("post_processing_duration")
        .build()
        .await?;
    let non_maximum_suppression_duration_pub = node
        .publisher::<Duration>("non_maximum_suppression_duration")
        .build()
        .await?;
    let detected_objects_pub = node
        .announcing_publisher::<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>("detected_objects")
        .await?;

    let initial_parameters_snapshot = node_parameters.snapshot();
    let parameters = initial_parameters_snapshot.typed();
    let model_path = parameters
        .neural_networks_folder
        .join(&parameters.model_name);

    let tensor_rt = TensorRT::default()
        .with_device_id(0)
        .with_fp16(true)
        .with_engine_cache(true)
        .with_engine_cache_path(parameters.neural_networks_folder.display())
        .build();
    let cuda = CUDA::default().build();

    let mut session = block_in_place(|| {
        Session::builder()?
            .with_execution_providers([tensor_rt, cuda])?
            .with_optimization_level(GraphOptimizationLevel::All)?
            .with_intra_threads(2)?
            .commit_from_file(model_path)
    })?;

    loop {
        parameter_receiver
            .wait_for(|parameters| parameters.typed().enable)
            .await?;

        let image = image_sub.recv().await?;

        let parameter_snapshot = node_parameters.snapshot();
        let parameters = parameter_snapshot.typed();
        if !parameters.enable {
            continue;
        }

        let image_time = image.header.stamp.into();
        let detected_objects_pending = detected_objects_pub.announce(image_time).await?;

        check_image(&image)?;

        let output = block_in_place(|| {
            let inference_start = Instant::now();

            let nv12_data = TensorRef::from_array_view((
                [image.height as usize / 2, image.width as usize / 2, 6],
                &image.data[..],
            ))?;
            let outputs: SessionOutputs = session.run(inputs!["raw_bytes_input" => nv12_data])?;

            let inference_duration = inference_start.elapsed();

            let post_processing_start = Instant::now();

            let outputs = extract_outputs(&outputs)?;
            let candidate_detections = extract_candidate_object_detections(
                &outputs,
                parameters
                    .object_detection_parameters
                    .minimum_candidate_confidence,
            )?;
            let post_processing_duration = post_processing_start.elapsed();
            let non_maximum_suppression_start = Instant::now();
            let detected_objects = non_maximum_suppression(
                candidate_detections,
                parameters
                    .object_detection_parameters
                    .maximum_intersection_over_union,
            );
            let non_maximum_suppression_duration = non_maximum_suppression_start.elapsed();

            Ok::<_, color_eyre::eyre::Error>(DetectionOutput {
                inference_duration,
                post_processing_duration,
                non_maximum_suppression_duration,
                detected_objects,
            })
        })?;

        inference_duration_pub
            .publish(&output.inference_duration)
            .await?;
        post_processing_duration_pub
            .publish(&output.post_processing_duration)
            .await?;
        non_maximum_suppression_duration_pub
            .publish(&output.non_maximum_suppression_duration)
            .await?;

        detected_objects_pending
            .publish(&TimeWrapper {
                time: image_time,
                inner: output.detected_objects,
            })
            .await?;
    }
}

fn check_image(image: &Image) -> Result<()> {
    if image.encoding != "nv12" {
        bail!("unsupported image encoding: {}", image.encoding);
    }

    if !image.width.is_multiple_of(32) || !image.height.is_multiple_of(32) {
        bail!(
            "image dimensions must be multiples of 32 (got {}x{})",
            image.width,
            image.height
        );
    }

    Ok(())
}

fn extract_outputs<'a>(outputs: &'a SessionOutputs<'_>) -> Result<ModelOutputs<'a>> {
    let hslvision_objects_output = extract_output(outputs, TaskHead::HSLVisionObjectDetection)?;
    if hslvision_objects_output.shape() != TaskHead::HSLVisionObjectDetection.expected_shape() {
        bail!(
            "hslvision object detection output not of expected shape. Expected: {:?}, got: {:?}",
            TaskHead::HSLVisionObjectDetection.expected_shape(),
            hslvision_objects_output.shape()
        )
    }
    let reshaped_hslvision_objects_output =
        hslvision_objects_output.squeeze().into_dimensionality()?;

    let nao_objects_output = extract_output(outputs, TaskHead::NaoObjectDetection)?;
    if nao_objects_output.shape() != TaskHead::NaoObjectDetection.expected_shape() {
        bail!(
            "nao object detection output not of expected shape. Expected: {:?}, got: {:?}",
            TaskHead::NaoObjectDetection.expected_shape(),
            nao_objects_output.shape()
        )
    }
    let reshaped_nao_objects_output = nao_objects_output.squeeze().into_dimensionality()?;

    Ok(ModelOutputs {
        hslvision_objects: reshaped_hslvision_objects_output,
        nao_objects: reshaped_nao_objects_output,
    })
}

fn extract_output<'a>(
    outputs: &'a SessionOutputs<'_>,
    task_head: TaskHead,
) -> Result<ArrayViewD<'a, f32>> {
    let (shape, data) = outputs[task_head.output_name()].try_extract_tensor::<f32>()?;
    let dimensions = shape
        .iter()
        .map(|&dimension| usize::try_from(dimension))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ArrayViewD::from_shape(IxDyn(&dimensions), data)?)
}

fn extract_candidate_object_detections(
    outputs: &ModelOutputs,
    confidence_threshold: f32,
) -> Result<Vec<Object<RobocupObjectLabel>>> {
    let mut object_detections: Vec<Object<RobocupObjectLabel>> = outputs
        .hslvision_objects
        .axis_iter(Axis(0))
        .filter_map(|row| {
            let label = RobocupObjectLabel::from_index(row[5] as usize);
            if matches!(label, RobocupObjectLabel::GoalPost) {
                return None;
            }

            let confidence = row[4usize];
            if confidence < confidence_threshold {
                return None;
            }

            let object_values: [f32; NUMBER_OF_VALUES_PER_OBJECT] = row
                .as_slice()
                .expect("slice is not contiguous")
                .try_into()
                .unwrap_or_else(|_| {
                    panic!("slice is not of length {}", NUMBER_OF_VALUES_PER_OBJECT)
                });

            Some(Object::from(object_values))
        })
        .collect();

    let nao_object_detections: Vec<Object<RobocupObjectLabel>> = outputs
        .nao_objects
        .axis_iter(Axis(0))
        .filter_map(|row| {
            let label = RobocupObjectLabel::from_index(row[5] as usize);
            if !matches!(
                label,
                RobocupObjectLabel::GoalPost | RobocupObjectLabel::Ball
            ) {
                return None;
            }

            let confidence = row[4usize];
            if confidence < confidence_threshold {
                return None;
            }

            let object_values: [f32; NUMBER_OF_VALUES_PER_OBJECT] = row
                .as_slice()
                .expect("slice is not contiguous")
                .try_into()
                .unwrap_or_else(|_| {
                    panic!("slice is not of length {}", NUMBER_OF_VALUES_PER_OBJECT)
                });

            Some(Object::from(object_values))
        })
        .collect();

    object_detections.extend(nao_object_detections);
    Ok(object_detections)
}

trait HasBoundingBox {
    fn bounding_box(&self) -> &BoundingBox;
}

impl<T> HasBoundingBox for Object<T> {
    fn bounding_box(&self) -> &BoundingBox {
        &self.bounding_box
    }
}

fn non_maximum_suppression<T: HasBoundingBox>(
    mut sorted_candidate_detections: Vec<T>,
    maximum_intersection_over_union: f32,
) -> Vec<T> {
    sorted_candidate_detections.sort_by(|detection1, detection2| {
        detection1
            .bounding_box()
            .confidence
            .total_cmp(&detection2.bounding_box().confidence)
    });

    let mut remaining_detections = Vec::new();

    while let Some(detection) = sorted_candidate_detections.pop() {
        sorted_candidate_detections.retain(|detection_candidate| {
            detection
                .bounding_box()
                .intersection_over_union(detection_candidate.bounding_box())
                < maximum_intersection_over_union
        });

        remaining_detections.push(detection)
    }

    remaining_detections
}
