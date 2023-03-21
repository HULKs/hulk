use std::num::NonZeroU32;

use color_eyre::Result;
use context_attribute::context;
use fast_image_resize::{
    DynamicImageView, FilterType, ImageBufferError, ImageView, ResizeAlg, Resizer,
};
use framework::{AdditionalOutput, MainOutput};
use types::{grayscale_image::GrayscaleImage, ycbcr422_image::YCbCr422Image, DetectedRobots};

use crate::CyclerInstance;

pub struct LuminanceImageExtractor {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub instance: CyclerInstance,
    pub image: Input<YCbCr422Image, "image">,
    pub luminance_image: AdditionalOutput<GrayscaleImage, "robot_detection.luminance_image">,
}

#[context]
pub struct MainOutputs {
    pub detected_robots: MainOutput<DetectedRobots>,
}

impl LuminanceImageExtractor {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let luminance_image = generate_luminance_image(context.image)?;
        context
            .luminance_image
            .fill_if_subscribed(|| luminance_image.clone());
        let detected_robots = DetectedRobots {};
        Ok(MainOutputs {
            detected_robots: detected_robots.into(),
        })
    }
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
