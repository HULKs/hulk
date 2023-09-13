use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use context_attribute::context;
use framework::MainOutput;
use hardware::PathsInterface;
use openvino::{Blob, Core, ExecutableNetwork, Layout, Precision, TensorDesc};
use types::{ycbcr422_image::YCbCr422Image, Rgb};
use ndarray::{ArrayView, Axis};
pub struct SemanticSegmentation {
    scratchpad: Vec<f32>,
    network: ExecutableNetwork,
    output_name: String,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    image: Input<YCbCr422Image, "image">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub segmented_image: MainOutput<Vec<f32>>,
}

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

        dbg!(network.get_input_name(0));
        network
            .set_input_layout("data", Layout::NCHW)
            .wrap_err("failed to set input data format")?;

        let output_name = network.get_output_name(0)?;

        Ok(Self {
            scratchpad: Vec::new(),
            network: core.load_network(&network, "CPU")?,
            output_name,
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

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let image = context.image;
        SemanticSegmentation::downsample_image_into_rgb2::<4>(&mut self.scratchpad, image);

        let mut infer_request = self.network.create_infer_request()?;

        let tensor_description = TensorDesc::new(Layout::NCHW, &[1, 3, 120, 160], Precision::FP32);
        let blob = Blob::new(
            &tensor_description,
            SemanticSegmentation::as_bytes(&self.scratchpad[..]),
        )?;

        infer_request.set_blob("data", &blob)?;
        infer_request.infer()?;

        let mut results = infer_request.get_blob(&self.output_name)?;
        let buffer = unsafe { results.buffer_mut_as_type::<f32>().unwrap().to_vec() };
        
        let result = ArrayView::from_shape((4,120,160), &buffer[..])?;
        let result = result.fold_axis(Axis(0), 0., |acc, e| {
            if e > acc {
                *e
            } else {
                *acc
            }
        });

        let buffer = result.iter().chu.map(|a| ).collect()

        dbg!(result.shape());

        Ok(MainOutputs {
            // TODO: no clone
            segmented_image: self.scratchpad.clone().into(),
        })
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
        scratchpad: &mut Vec<f32>,
        image: &YCbCr422Image,
    ) {
        let width = image.width() as usize;
        let height = image.height() as usize;
        scratchpad.clear();

        let downsampled_width = width / DOWNSAMPLE_RATIO;
        let downsampled_height = height / DOWNSAMPLE_RATIO;


        for row in image.buffer().chunks(width / 2).step_by(DOWNSAMPLE_RATIO) {
            for pixel in row.iter().step_by(DOWNSAMPLE_RATIO / 2) {
                // dbg!((ridx, cidx));
                let y = pixel.averaged_y();
                let centered_cr = pixel.cr as f32 - 128.0;
                let r = (y as f32 + 1.40200 * centered_cr).clamp(0.0, 255.0);
                scratchpad.push(r / 255.0);
            }
        }

        for row in image.buffer().chunks(width / 2).step_by(DOWNSAMPLE_RATIO) {
            for pixel in row.iter().step_by(DOWNSAMPLE_RATIO / 2) {
                let y = pixel.averaged_y();
                let centered_cr = pixel.cr as f32 - 128.0;
                let centered_cb = pixel.cb as f32 - 128.0;
                let g =
                    (y as f32 - 0.34414 * centered_cb - 0.71414 * centered_cr).clamp(0.0, 255.0);
                scratchpad.push(g / 255.0);
            }
        }

        for row in image.buffer().chunks(width / 2).step_by(DOWNSAMPLE_RATIO) {
            for pixel in row.iter().step_by(DOWNSAMPLE_RATIO / 2) {
                let y = pixel.averaged_y();
                let centered_cb = pixel.cb as f32 - 128.0;
                let b = (y as f32 + 1.77200 * centered_cb).clamp(0.0, 255.0);
                scratchpad.push(b / 255.0);
            }
        }
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
