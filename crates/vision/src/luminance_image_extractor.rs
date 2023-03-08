use std::{num::NonZeroU32, time::Instant};

use color_eyre::Result;
use context_attribute::context;
use fast_image_resize::{DynamicImageView, FilterType, ImageView, ResizeAlg, Resizer};
use framework::{AdditionalOutput, MainOutput};
use types::image::{Image, YImage};

use crate::CyclerInstance;

pub struct LuminanceImageExtractor {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub instance: CyclerInstance,
    pub image: Input<Image, "image">,
    pub duration: AdditionalOutput<f32, "luminance_image_extractor_timing">,
}

#[context]
pub struct MainOutputs {
    pub luminance_image: MainOutput<YImage>,
}

impl LuminanceImageExtractor {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let start_time = Instant::now();
        let grayscale_buffer: Vec<_> = context
            .image
            .buffer
            .iter()
            .flat_map(|pixel| [pixel.y1, pixel.y2])
            .collect();
        let y_image = ImageView::from_buffer(
            NonZeroU32::new(context.image.width()).unwrap(),
            NonZeroU32::new(context.image.height()).unwrap(),
            &grayscale_buffer,
        )?;

        let dst_width = NonZeroU32::new(80).unwrap();
        let dst_height = NonZeroU32::new(60).unwrap();

        let mut dst_image =
            fast_image_resize::Image::new(dst_width, dst_height, y_image.pixel_type());

        let mut resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Hamming));
        resizer
            .resize(&DynamicImageView::U8(y_image), &mut dst_image.view_mut())
            .unwrap();
        let luminance_image =
            YImage::from_vec(dst_width.get(), dst_height.get(), dst_image.into_vec());
        let took = Instant::now() - start_time;
        context.duration.fill_if_subscribed(|| took.as_secs_f32());
        Ok(MainOutputs {
            luminance_image: luminance_image.into(),
        })
    }
}
