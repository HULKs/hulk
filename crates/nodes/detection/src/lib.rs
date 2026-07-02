use std::{
    boxed::Box,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::{
    Result,
    eyre::{WrapErr as _, bail},
};
use ndarray::{ArrayView2, ArrayView3, Axis};
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    inputs,
    session::{Session, SessionOutputs, builder::GraphOptimizationLevel},
    value::TensorRef,
};
use ros_z_streams::CreateAnnouncingPublisher;
use ros2::sensor_msgs::image::Image;

use ros_z::prelude::*;
use types::{
    bounding_box::BoundingBox,
    object_detection::{NUMBER_OF_VALUES_PER_OBJECT, Object, RobocupObjectLabel, YOLOObjectLabel},
    parameters::DetectionParameters,
    pose_detection::{NUMBER_OF_VALUES_PER_POSE, Pose},
    time_wrapper::TimeWrapper,
};

pub const NUMBER_OF_DETECTIONS: usize = 300;

#[derive(Clone, Copy, Debug)]
enum TaskHead {
    ObjectDetection,
    PoseDetection,
}

impl TaskHead {
    fn output_name(self) -> &'static str {
        match self {
            TaskHead::ObjectDetection => "object_output",
            TaskHead::PoseDetection => "pose_output",
        }
    }

    fn expected_shape(self) -> [usize; 3] {
        match self {
            Self::ObjectDetection => [1, NUMBER_OF_DETECTIONS, NUMBER_OF_VALUES_PER_OBJECT],
            Self::PoseDetection => [1, NUMBER_OF_DETECTIONS, NUMBER_OF_VALUES_PER_POSE],
        }
    }
}

#[derive(Debug)]
struct ModelOutputs<'a> {
    objects: ArrayView2<'a, f32>,
    poses: ArrayView2<'a, f32>,
}

struct DetectionOutput {
    session: Session,
    detected_objects: Vec<Object<RobocupObjectLabel>>,
    detected_poses: Vec<Pose<YOLOObjectLabel>>,
    inference_duration: Duration,
    post_processing_duration: Duration,
    non_maximum_suppression_duration: Duration,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("detection").build().await?;

    let node_parameters = node.bind_parameter_as::<DetectionParameters>("detection")?;

    let image_sub = node
        .subscriber::<TimeWrapper<Image>>("inputs/left_image")
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
        .announcing_publisher::<Vec<Object<RobocupObjectLabel>>>("detected_objects")
        .await?;
    let detected_poses_pub = node
        .announcing_publisher::<Vec<Pose<YOLOObjectLabel>>>("detected_poses")
        .await?;

    let initial_parameters_snapshot = node_parameters.snapshot();
    let initial_parameters = initial_parameters_snapshot.typed().clone();
    let mut session = tokio::task::spawn_blocking(move || create_session(&initial_parameters))
        .await
        .wrap_err("detection session creation task failed")??;

    loop {
        let parameters_snapshot = node_parameters.snapshot();
        let parameters = parameters_snapshot.typed().clone();
        if !parameters.enable {
            continue;
        }

        let timed_image = image_sub.recv().await?;
        let image_time = timed_image.time;

        let detected_objects_pending = detected_objects_pub.announce(image_time).await?;
        let detected_poses_pending = detected_poses_pub.announce(image_time).await?;

        let image = timed_image.inner;
        let detection_output =
            tokio::task::spawn_blocking(move || run_detection(session, image, parameters))
                .await
                .wrap_err("detection inference task failed")??;

        session = detection_output.session;
        let inference_duration = detection_output.inference_duration;
        let post_processing_duration = detection_output.post_processing_duration;
        let non_maximum_suppression_duration = detection_output.non_maximum_suppression_duration;
        let detected_objects = detection_output.detected_objects;
        let detected_poses = detection_output.detected_poses;

        inference_duration_pub.publish(&inference_duration).await?;
        post_processing_duration_pub
            .publish(&post_processing_duration)
            .await?;
        non_maximum_suppression_duration_pub
            .publish(&non_maximum_suppression_duration)
            .await?;

        detected_objects_pending.publish(&detected_objects).await?;
        detected_poses_pending.publish(&detected_poses).await?;
    }
}

fn create_session(parameters: &DetectionParameters) -> Result<Session> {
    let model_path = parameters
        .neural_networks_folder
        .join(&parameters.model_name);

    let tensor_rt = TensorRTExecutionProvider::default()
        .with_device_id(0)
        .with_fp16(true)
        .with_engine_cache(true)
        .with_engine_cache_path(parameters.neural_networks_folder.display())
        .build();
    let cuda = CUDAExecutionProvider::default().build();

    Ok(Session::builder()?
        .with_execution_providers([tensor_rt, cuda])?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(2)?
        .commit_from_file(model_path)?)
}

fn run_detection(
    mut session: Session,
    image: Image,
    parameters: DetectionParameters,
) -> Result<DetectionOutput> {
    check_image(&image)?;

    let inference_start = Instant::now();
    let nv12_data = ArrayView3::from_shape(
        [image.height as usize / 2, image.width as usize / 2, 6],
        &image.data,
    )?;
    let session_outputs: SessionOutputs =
        session.run(inputs!["raw_bytes_input" => TensorRef::from_array_view(nv12_data)?])?;
    let inference_duration = inference_start.elapsed();

    let post_processing_start = Instant::now();
    let model_outputs = extract_outputs(&session_outputs)?;
    let candidate_detections = extract_candidate_object_detections(
        &model_outputs,
        parameters.object_detection_parameters.confidence_threshold,
    )?;
    let candidate_human_poses = extract_candidate_pose_detections(
        &model_outputs,
        parameters.pose_detection_parameters.confidence_threshold,
    )?;
    let post_processing_duration = post_processing_start.elapsed();
    drop(model_outputs);
    drop(session_outputs);

    let non_maximum_suppression_start = Instant::now();
    let detected_objects = non_maximum_suppression(
        candidate_detections,
        parameters
            .object_detection_parameters
            .maximum_intersection_over_union,
    );
    let detected_poses = non_maximum_suppression(
        candidate_human_poses,
        parameters
            .pose_detection_parameters
            .maximum_intersection_over_union,
    );
    let non_maximum_suppression_duration = non_maximum_suppression_start.elapsed();

    Ok(DetectionOutput {
        session,
        detected_objects,
        detected_poses,
        inference_duration,
        post_processing_duration,
        non_maximum_suppression_duration,
    })
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

fn extract_outputs<'a>(outputs: &'a SessionOutputs<'a>) -> Result<ModelOutputs<'a>> {
    let objects_output =
        outputs[TaskHead::ObjectDetection.output_name()].try_extract_array::<f32>()?;
    if objects_output.shape() != TaskHead::ObjectDetection.expected_shape() {
        bail!(
            "object detection output not of expected shape. Expected: {:?}, got: {:?}",
            TaskHead::ObjectDetection.expected_shape(),
            objects_output.shape()
        )
    }
    let reshaped_objects_output = objects_output.squeeze().into_dimensionality()?;

    let poses_output = outputs[TaskHead::PoseDetection.output_name()].try_extract_array::<f32>()?;
    if poses_output.shape() != TaskHead::PoseDetection.expected_shape() {
        bail!(
            "pose detection output not of expected shape. Expected: {:?}, got: {:?}",
            TaskHead::PoseDetection.expected_shape(),
            poses_output.shape()
        )
    }
    let reshaped_pose_output = poses_output.squeeze().into_dimensionality()?;

    Ok(ModelOutputs {
        objects: reshaped_objects_output,
        poses: reshaped_pose_output,
    })
}

fn extract_candidate_object_detections(
    outputs: &ModelOutputs,
    confidence_threshold: f32,
) -> Result<Vec<Object<RobocupObjectLabel>>> {
    Ok(outputs
        .objects
        .axis_iter(Axis(0))
        .filter_map(|row| {
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
        .collect())
}

fn extract_candidate_pose_detections(
    outputs: &ModelOutputs,
    confidence_threshold: f32,
) -> Result<Vec<Pose<YOLOObjectLabel>>> {
    Ok(outputs
        .poses
        .axis_iter(Axis(0))
        .filter_map(|row| {
            let confidence = row[4usize];
            if confidence < confidence_threshold {
                return None;
            }

            let pose_values: [f32; NUMBER_OF_VALUES_PER_POSE] = row
                .as_slice()
                .expect("slice is not contiguous")
                .try_into()
                .unwrap_or_else(|_| panic!("slice is not of length {}", NUMBER_OF_VALUES_PER_POSE));

            Some(Pose::from(&pose_values))
        })
        .collect())
}

trait HasBoundingBox {
    fn bounding_box(&self) -> &BoundingBox;
}

impl<T> HasBoundingBox for Object<T> {
    fn bounding_box(&self) -> &BoundingBox {
        &self.bounding_box
    }
}

impl<T> HasBoundingBox for Pose<T> {
    fn bounding_box(&self) -> &BoundingBox {
        &self.object.bounding_box
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

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u32, height: u32, encoding: &str) -> Image {
        Image {
            height,
            width,
            encoding: encoding.to_string(),
            data: vec![0; (width * height * 3 / 2) as usize].into(),
            ..Default::default()
        }
    }

    #[test]
    fn check_image_accepts_nv12_multiple_of_32() {
        check_image(&image(640, 480, "nv12")).expect("valid image should pass");
    }

    #[test]
    fn check_image_rejects_non_nv12_encoding() {
        let error =
            check_image(&image(640, 480, "rgb8")).expect_err("invalid encoding should fail");
        assert!(error.to_string().contains("unsupported image encoding"));
    }

    #[test]
    fn check_image_rejects_dimensions_not_multiple_of_32() {
        let error = check_image(&image(641, 480, "nv12")).expect_err("invalid width should fail");
        assert!(
            error
                .to_string()
                .contains("image dimensions must be multiples of 32")
        );
    }
}
