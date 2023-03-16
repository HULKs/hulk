use std::num::NonZeroU32;

use color_eyre::Result;
use context_attribute::context;
use fast_image_resize::{DynamicImageView, FilterType, ImageView, ResizeAlg, Resizer};
use framework::MainOutput;
use types::{grayscale_image::GrayscaleImage, nao_image::NaoImage};

use crate::CyclerInstance;

pub struct LuminanceImageExtractor {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub instance: CyclerInstance,
    pub image: Input<NaoImage, "image">,
}

#[context]
pub struct MainOutputs {
    pub luminance_image: MainOutput<GrayscaleImage>,
}

impl LuminanceImageExtractor {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let grayscale_buffer: Vec<_> = context
            .image
            .buffer()
            .iter()
            .flat_map(|pixel| [pixel.y1, pixel.y2])
            .collect();
        let y_image = ImageView::from_buffer(
            NonZeroU32::new(context.image.width()).unwrap(),
            NonZeroU32::new(context.image.height()).unwrap(),
            &grayscale_buffer,
        )?;
        let new_width = NonZeroU32::new(80).unwrap();
        let new_height = NonZeroU32::new(60).unwrap();
        let mut new_image =
            fast_image_resize::Image::new(new_width, new_height, y_image.pixel_type());
        let mut resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Hamming));
        resizer
            .resize(&DynamicImageView::U8(y_image), &mut new_image.view_mut())
            .unwrap();
        let luminance_image =
            GrayscaleImage::from_vec(new_width.get(), new_height.get(), new_image.into_vec());
        Ok(MainOutputs {
            luminance_image: luminance_image.into(),
        })
    }
}
