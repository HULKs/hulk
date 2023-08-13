use std::{num::NonZeroU32, ops::Range, path::PathBuf};

use color_eyre::Result;
use compiled_nn::CompiledNN;
use context_attribute::context;
use fast_image_resize::{
    DynamicImageView, FilterType, ImageBufferError, ImageView, ResizeAlg, Resizer,
};
use framework::{deserialize_not_implemented, AdditionalOutput, MainOutput};
use hardware::PathsInterface;
use itertools::Itertools;
use nalgebra::{vector, Isometry3, Vector2};
use projection::Projection;
use serde::{Deserialize, Serialize};
use types::{
    camera_matrix::CameraMatrix,
    detected_robots::{BoundingBox, DetectedRobots},
    grayscale_image::GrayscaleImage,
    ycbcr422_image::YCbCr422Image,
};

const NUMBER_OF_SCALINGS: usize = 4;
const PARAMETERS_PER_BOX: usize = 6;
const BOX_SCALINGS: [Vector2<f32>; NUMBER_OF_SCALINGS] = [
    Vector2::new(0.5, 1.0),
    Vector2::new(1.0, 2.0),
    Vector2::new(2.0, 4.0),
    Vector2::new(3.0, 6.0),
];
const OUTPUT_SCALING: f32 = 10.0;

#[derive(Deserialize, Serialize)]
pub struct RobotDetection {
    #[serde(skip, default = "deserialize_not_implemented")]
    neural_network: CompiledNN,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
    neural_network_file: Parameter<PathBuf, "robot_detection.$cycler_instance.neural_network">,
}

#[context]
pub struct CycleContext {
    image: Input<YCbCr422Image, "image">,
    camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    robot_to_ground: RequiredInput<Option<Isometry3<f32>>, "Control", "robot_to_ground?">,
    luminance_image: AdditionalOutput<GrayscaleImage, "robot_detection.luminance_image">,
    object_threshold: Parameter<f32, "robot_detection.$cycler_instance.object_threshold">,
    enable: Parameter<bool, "robot_detection.$cycler_instance.enable">,
    enable_filter_by_size:
        Parameter<bool, "robot_detection.$cycler_instance.enable_filter_by_size">,
    enable_filter_by_pixel_position:
        Parameter<bool, "robot_detection.$cycler_instance.enable_filter_by_pixel_position">,
    lowest_bottom_pixel_position:
        Parameter<f32, "robot_detection.$cycler_instance.lowest_bottom_pixel_position">,
    allowed_projected_robot_height:
        Parameter<Range<f32>, "robot_detection.$cycler_instance.allowed_projected_robot_height">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub detected_robots: MainOutput<DetectedRobots>,
}

impl RobotDetection {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let mut neural_network = CompiledNN::default();
        neural_network.compile(paths.neural_networks.join(context.neural_network_file));
        Ok(Self { neural_network })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if !context.enable {
            return Ok(MainOutputs::default());
        }

        let luminance_image = generate_luminance_image(context.image)?;
        context
            .luminance_image
            .fill_if_subscribed(|| luminance_image.clone());

        let input_layer = self.neural_network.input_mut(0);
        copy_into_tensor(
            &luminance_image,
            luminance_image.height() as usize,
            luminance_image.width() as usize,
            input_layer.data,
        );

        self.neural_network.apply();

        let camera_image_size =
            Vector2::new(context.image.width() as f32, context.image.height() as f32);
        let grid_boxes = create_boxes(
            &mut self.neural_network,
            camera_image_size,
            *context.object_threshold,
        );

        let mut filtered_detections = grid_boxes;

        if *context.enable_filter_by_pixel_position {
            filtered_detections = filter_by_pixel_position(
                filtered_detections,
                *context.lowest_bottom_pixel_position,
            );
        }

        if *context.enable_filter_by_size {
            filtered_detections = filter_by_size(
                filtered_detections,
                context.camera_matrix,
                context.robot_to_ground,
                context.allowed_projected_robot_height,
            );
        }

        let on_ground = filtered_detections
            .iter()
            .filter_map(|bounding_box| {
                let box_bottom = bounding_box.center + vector![0.0, bounding_box.size.y / 2.0];
                context.camera_matrix.pixel_to_ground(box_bottom).ok()
            })
            .collect();

        let detected_robots = DetectedRobots {
            in_image: filtered_detections,
            on_ground,
        };
        Ok(MainOutputs {
            detected_robots: detected_robots.into(),
        })
    }
}

fn filter_by_pixel_position(
    mut grid_boxes: Vec<BoundingBox>,
    lowest_bottom_pixel_position: f32,
) -> Vec<BoundingBox> {
    grid_boxes.retain(|bounding_box| {
        let box_bottom = bounding_box.center.y + bounding_box.size.y / 2.0;
        box_bottom < lowest_bottom_pixel_position
    });
    grid_boxes
}

fn filter_by_size(
    mut grid_boxes: Vec<BoundingBox>,
    camera_matrix: &CameraMatrix,
    robot_to_ground: &Isometry3<f32>,
    allowed_projected_robot_height: &Range<f32>,
) -> Vec<BoundingBox> {
    grid_boxes.retain(|bounding_box| {
        let box_bottom = bounding_box.center + vector![0.0, bounding_box.size.y / 2.0];
        let feet_position = match camera_matrix.pixel_to_ground(box_bottom) {
            Ok(ok) => ok,
            Err(_) => return false,
        };
        let box_top = bounding_box.center - vector![0.0, bounding_box.size.y / 2.0];
        let head_position = match camera_matrix.pixel_to_robot_with_x(box_top, feet_position.x) {
            Ok(ok) => ok,
            Err(_) => return false,
        };
        let head_position_over_ground = robot_to_ground * head_position;
        allowed_projected_robot_height.contains(&head_position_over_ground.z)
    });
    grid_boxes
}

fn generate_luminance_image(image: &YCbCr422Image) -> Result<GrayscaleImage, ImageBufferError> {
    let grayscale_buffer: Vec<_> = image
        .buffer()
        .iter()
        .flat_map(|pixel| [pixel.y1, pixel.y2])
        .collect();
    let y_image = ImageView::from_buffer(
        NonZeroU32::new(image.width()).unwrap(),
        NonZeroU32::new(image.height()).unwrap(),
        &grayscale_buffer,
    )?;
    let new_width = NonZeroU32::new(80).unwrap();
    let new_height = NonZeroU32::new(60).unwrap();
    let mut new_image = fast_image_resize::Image::new(new_width, new_height, y_image.pixel_type());
    let mut resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Hamming));
    resizer
        .resize(&DynamicImageView::U8(y_image), &mut new_image.view_mut())
        .unwrap();
    Ok(GrayscaleImage::from_vec(
        new_width.get(),
        new_height.get(),
        new_image.into_vec(),
    ))
}

fn copy_into_tensor(
    image: &GrayscaleImage,
    image_height: usize,
    image_width: usize,
    input_layer: &mut [f32],
) {
    for y in 0..image_height {
        for x in 0..image_width {
            input_layer[x + y * image_width] = image.buffer()[x + y * image_width] as f32;
        }
    }
}

fn create_boxes(
    neural_network: &mut CompiledNN,
    camera_image_size: Vector2<f32>,
    object_threshold: f32,
) -> Vec<BoundingBox> {
    let output_layer = neural_network.output(0);

    let grid_height = output_layer.dimensions[0] as usize;
    let grid_width = output_layer.dimensions[1] as usize;
    let grid_size = Vector2::new(grid_width as f32, grid_height as f32);

    (0..grid_height)
        .cartesian_product(0..grid_width)
        .flat_map(|(y, x)| {
            let grid_position = Vector2::new(x as f32, y as f32);
            let data_offset = (y * grid_width + x) * NUMBER_OF_SCALINGS * PARAMETERS_PER_BOX;
            let data_slice = &output_layer.data
                [data_offset..data_offset + NUMBER_OF_SCALINGS * PARAMETERS_PER_BOX];
            let scaled_boxes = boxes_from_output(
                data_slice.try_into().unwrap(),
                grid_position,
                grid_size,
                camera_image_size,
                &BOX_SCALINGS,
            );
            scaled_boxes
                .into_iter()
                .filter(|item| item.probability > object_threshold)
        })
        .collect()
}

fn standard_logistic(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

fn boxes_from_output(
    values: &[f32; NUMBER_OF_SCALINGS * PARAMETERS_PER_BOX],
    grid_position: Vector2<f32>,
    grid_size: Vector2<f32>,
    camera_image_size: Vector2<f32>,
    box_scalings: &[Vector2<f32>; 4],
) -> [BoundingBox; NUMBER_OF_SCALINGS] {
    let values = values.map(standard_logistic);
    [
        box_from_network_data(
            Vector2::new(values[0], values[1]),
            Vector2::new(values[2], values[3]),
            values[4],
            values[5],
            grid_position,
            grid_size,
            camera_image_size,
            box_scalings[0],
        ),
        box_from_network_data(
            Vector2::new(values[6], values[7]),
            Vector2::new(values[8], values[9]),
            values[10],
            values[11],
            grid_position,
            grid_size,
            camera_image_size,
            box_scalings[1],
        ),
        box_from_network_data(
            Vector2::new(values[12], values[13]),
            Vector2::new(values[14], values[15]),
            values[16],
            values[17],
            grid_position,
            grid_size,
            camera_image_size,
            box_scalings[2],
        ),
        box_from_network_data(
            Vector2::new(values[18], values[19]),
            Vector2::new(values[20], values[21]),
            values[22],
            values[23],
            grid_position,
            grid_size,
            camera_image_size,
            box_scalings[3],
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn box_from_network_data(
    center: Vector2<f32>,
    size: Vector2<f32>,
    probability: f32,
    distance: f32,
    grid_position: Vector2<f32>,
    grid_size: Vector2<f32>,
    camera_image_size: Vector2<f32>,
    scaling: Vector2<f32>,
) -> BoundingBox {
    BoundingBox {
        center: (center + grid_position)
            .component_div(&grid_size)
            .component_mul(&camera_image_size)
            .into(),
        size: (size * OUTPUT_SCALING)
            .component_mul(&scaling)
            .component_mul(&camera_image_size)
            .component_div(&grid_size),
        probability,
        distance: distance * OUTPUT_SCALING,
    }
}
