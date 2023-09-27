use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use context_attribute::context;
use framework::AdditionalOutput;
use hardware::PathsInterface;
use itertools::Itertools;
use ndarray::{Array2, Array3, ArrayView};
use openvino::{Blob, Core, ExecutableNetwork, Layout, Precision, TensorDesc};
use types::{ycbcr422_image::YCbCr422Image, Rgb, YCbCr422, YCbCr444};

const SEMANTIC_SEGMENTATION_IMAGE_WIDTH: usize = 160;
const SEMANTIC_SEGMENTATION_IMAGE_HEIGHT: usize = 120;
const SEMANTIC_SEGMENTATION_NUMBER_CHANNELS: usize = 3;

const SEMANTIC_SEGMENTATION_SCRATCHPAD_SIZE: usize = SEMANTIC_SEGMENTATION_IMAGE_HEIGHT
    * SEMANTIC_SEGMENTATION_IMAGE_WIDTH
    * SEMANTIC_SEGMENTATION_NUMBER_CHANNELS;
type Scratchpad = [f32; SEMANTIC_SEGMENTATION_SCRATCHPAD_SIZE];

pub struct SemanticSegmentation {
    scratchpad: Scratchpad,
    network: ExecutableNetwork,

    input_name: String,
    output_name: String,

    class_frequency: Vec<f32>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    image: Input<YCbCr422Image, "image">,

    segmented_image: AdditionalOutput<YCbCr422Image, "segmented_output">,
    class_frequency: AdditionalOutput<Vec<f32>, "class_frequency">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl SemanticSegmentation {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let neural_network_folder = paths.neural_networks;

        let model_path = neural_network_folder.join("segmentation_down4_rgb_nchw-ov.xml");
        let weights_path = neural_network_folder.join("segmentation_down4_rgb_nchw-ov.bin");

        let mut core = Core::new(None)?;
        let mut network = core
            .read_network_from_file(
                model_path
                    .to_str()
                    .wrap_err("failed to get semantic segmentation model path")?,
                weights_path
                    .to_str()
                    .wrap_err("failed to get semantic segmentation weights path")?,
            )
            .wrap_err("failed to create semantic segmentation network")?;

        network
            .set_input_layout("data", Layout::NCHW)
            .wrap_err("failed to set input data format")?;

        let input_name = network.get_input_name(0)?;
        let output_name = network.get_output_name(0)?;

        Ok(Self {
            scratchpad: [0.; SEMANTIC_SEGMENTATION_SCRATCHPAD_SIZE],
            network: core.load_network(&network, "CPU")?,
            input_name,
            output_name,
            class_frequency: vec![0.; 4],
        })
    }

    fn as_bytes(v: &[f32]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<f32>(),
            )
        }
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let image = context.image;
        SemanticSegmentation::downsample_image_into_rgb2::<4>(&mut self.scratchpad, image);

        let mut infer_request = self.network.create_infer_request()?;

        let tensor_description = TensorDesc::new(Layout::NCHW, &[1, 3, 120, 160], Precision::FP32);
        let blob = Blob::new(
            &tensor_description,
            SemanticSegmentation::as_bytes(&self.scratchpad[..]),
        )?;

        infer_request.set_blob(&self.input_name, &blob)?;
        infer_request.infer()?;

        let mut results = infer_request.get_blob(&self.output_name)?;
        let buffer = unsafe { results.buffer_mut_as_type::<f32>().unwrap().to_vec() };

        // let result = Array3::<f32>::from_shape_vec((3, 120, 160), self.scratchpad.to_vec())?;
        let result = ArrayView::from_shape((4, 120, 160), &buffer[..])?;
        self.class_frequency = vec![0.; 4];
        let class_map = Array2::<u8>::from_shape_vec(
            (120, 160),
            result
                .columns()
                .into_iter()
                .map(|lane| {
                    lane.into_iter()
                        .enumerate()
                        .max_by(|(_, &value0), (_, &value1)| value0.total_cmp(&value1))
                        .map(|(idx, _)| {
                            self.class_frequency[idx] += 1.
                                / (SEMANTIC_SEGMENTATION_IMAGE_HEIGHT
                                    * SEMANTIC_SEGMENTATION_IMAGE_WIDTH)
                                    as f32;
                            idx as u8
                        })
                        .unwrap()
                })
                .collect(),
        )?;

        context.segmented_image.fill_if_subscribed(|| {
            SemanticSegmentation::map_array_to_image(class_map, |class_index| {
                match class_index {
                    0 => Rgb::GREEN,
                    1 => Rgb::BLACK,
                    2 => Rgb::WHITE,
                    3 => Rgb::BLUE,
                    other => panic!("{other} is not a valid class"),
                }
                .into()
            })
        });
        context
            .class_frequency
            .fill_if_subscribed(|| self.class_frequency.clone());
        Ok(MainOutputs {})
    }

    // pub fn downsample_image_into_rgb<const DOWNSAMPLE_RATIO: usize>(
    //     scratchpad: &mut Vec<Rgb>,
    //     image: &YCbCr422Image,
    // ) {
    //     let height = image.height() as usize;
    //     let width = image.width() as usize;

    //     assert!(
    //         DOWNSAMPLE_RATIO % 2 == 0,
    //         "the down sampling factor has to be even"
    //     );

    //     assert!(
    //         height % DOWNSAMPLE_RATIO == 0,
    //         "the image height {} is not divisible by the downsample ratio {}",
    //         height,
    //         DOWNSAMPLE_RATIO
    //     );
    //     assert!(
    //         width % DOWNSAMPLE_RATIO == 0,
    //         "the image width {} is not divisible by the downsample ratio {}",
    //         width,
    //         DOWNSAMPLE_RATIO
    //     );

    //     scratchpad.clear();
    //     scratchpad.extend(
    //         image
    //             .buffer()
    //             .chunks(width / 2)
    //             .step_by(DOWNSAMPLE_RATIO)
    //             // divide by 2 because of 422
    //             .flat_map(|row| row.into_iter().step_by(DOWNSAMPLE_RATIO / 2))
    //             .map(|&pixel| Rgb::from(pixel)),
    //     );
    // }

    pub fn downsample_image_into_rgb2<const DOWNSAMPLE_RATIO: usize>(
        scratchpad: &mut Scratchpad,
        image: &YCbCr422Image,
    ) {
        let width = image.width() as usize;
        let height = image.height() as usize;

        let downsampled_width = width / DOWNSAMPLE_RATIO;
        let downsampled_height = height / DOWNSAMPLE_RATIO;

        assert_eq!(downsampled_height, SEMANTIC_SEGMENTATION_IMAGE_HEIGHT);
        assert_eq!(downsampled_width, SEMANTIC_SEGMENTATION_IMAGE_WIDTH);

        let mut scratchpad_index = 0;
        const STRIDE: usize =
            SEMANTIC_SEGMENTATION_IMAGE_HEIGHT * SEMANTIC_SEGMENTATION_IMAGE_WIDTH;

        for row in image.buffer().chunks(width / 2).step_by(DOWNSAMPLE_RATIO) {
            for pixel in row.iter().step_by(DOWNSAMPLE_RATIO / 2) {
                let rgb = Rgb::from(*pixel);
                scratchpad[scratchpad_index + 0] = rgb.b as f32 / 255.0;
                scratchpad[scratchpad_index + STRIDE] = rgb.g as f32 / 255.0;
                scratchpad[scratchpad_index + 2 * STRIDE] = rgb.r as f32 / 255.0;
                scratchpad_index += 1;
            }
        }
        assert_eq!(scratchpad_index, STRIDE);
    }

    pub fn map_array_to_image<Type: Copy, Mapper: Fn(Type) -> YCbCr444>(
        array: Array2<Type>,
        mapper: Mapper,
    ) -> YCbCr422Image {
        let [height, width]: [usize; 2] = array
            .shape()
            .try_into()
            .expect("the array shape is incorrect, expected two dimensions");

        let mut buffer = vec![YCbCr422::default(); width as usize / 2 * height as usize];

        for (index, mut chunk) in array.into_iter().chunks(2).into_iter().enumerate() {
            let color1 = mapper(chunk.next().unwrap());
            let color2 = mapper(chunk.next().unwrap());

            let color = YCbCr422::from([color1, color2]);
            buffer[index] = color;
        }

        YCbCr422Image::from_ycbcr_buffer(width as u32 / 2, height as u32, buffer)
    }
}

// #[cfg(test)]
// mod tests {
//     extern crate test;

//     use types::ycbcr422_image::YCbCr422Image;

//     use super::SemanticSegmentation;
//     use std::hint::black_box;
//     use test::Bencher;

//     #[bench]
//     fn sample_image(b: &mut Bencher) {
//         let image = YCbCr422Image::load_from_444_png(
//             "../../tests/data/white_wall_with_a_little_desk_in_front.png",
//         )
//         .unwrap();
//         let mut scratchpad = Vec::new();
//         b.iter(|| {
//             black_box(SemanticSegmentation::downsample_image_into_rgb::<4>(
//                 &mut scratchpad,
//                 &image,
//             ))
//         })
//     }
//     #[bench]
//     fn sample_image2(b: &mut Bencher) {
//         let image = YCbCr422Image::load_from_444_png(
//             "../../tests/data/white_wall_with_a_little_desk_in_front.png",
//         )
//         .unwrap();
//         let mut scratchpad = Vec::new();
//         b.iter(|| {
//             black_box(SemanticSegmentation::downsample_image_into_rgb2::<4>(
//                 &mut scratchpad,
//                 &image,
//             ))
//         })
//     }

//     #[test]
//     fn downsampling_number_of_pixels() {
//         let image = YCbCr422Image::load_from_444_png(
//             "../../tests/data/white_wall_with_a_little_desk_in_front.png",
//         )
//         .unwrap();
//         let mut scratchpad = Vec::new();

//         SemanticSegmentation::downsample_image_into_rgb::<2>(&mut scratchpad, &image);
//         assert_eq!(
//             scratchpad.len(),
//             (image.width() as usize / 2) * (image.height() as usize / 2)
//         );

//         SemanticSegmentation::downsample_image_into_rgb::<4>(&mut scratchpad, &image);
//         assert_eq!(
//             scratchpad.len(),
//             (image.width() as usize / 4) * (image.height() as usize / 4)
//         );

//         SemanticSegmentation::downsample_image_into_rgb2::<2>(&mut scratchpad, &image);
//         assert_eq!(
//             scratchpad.len(),
//             (image.width() as usize / 2) * (image.height() as usize / 2)
//         );

//         SemanticSegmentation::downsample_image_into_rgb2::<4>(&mut scratchpad, &image);
//         assert_eq!(
//             scratchpad.len(),
//             (image.width() as usize / 4) * (image.height() as usize / 4)
//         );
//     }

//     #[test]
//     fn same_result() {
//         let image = YCbCr422Image::load_from_444_png(
//             "../../tests/data/white_wall_with_a_little_desk_in_front.png",
//         )
//         .unwrap();
//         let mut scratchpad1 = Vec::new();
//         let mut scratchpad2 = Vec::new();

//         SemanticSegmentation::downsample_image_into_rgb::<4>(&mut scratchpad1, &image);
//         SemanticSegmentation::downsample_image_into_rgb2::<4>(&mut scratchpad2, &image);

//         assert_eq!(scratchpad1, scratchpad2);
//     }
// }
