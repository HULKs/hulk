use fast_image_resize::DynamicImageView;
use std::num::NonZeroU32;

use fast_image_resize::{FilterType, ImageView, ResizeAlg, Resizer};
use image::GrayImage;
use types::{grayscale_image::GrayscaleImage, ycbcr422_image::YCbCr422Image};

pub(crate) fn gray_image_to_hulks_grayscale_image(
    image: &GrayImage,
    new_size: Option<(u32, u32)>,
    filter: Option<FilterType>,
) -> GrayscaleImage {
    if let Some(new_size) = new_size {
        let resized = gray_image_resize(image, new_size, filter);
        GrayscaleImage::from_vec(
            resized.width().get(),
            resized.height().get(),
            resized.into_vec(),
        )
    } else {
        GrayscaleImage::from_vec(image.width(), image.height(), image.as_raw().clone())
    }
}

pub(crate) fn generate_luminance_image(
    image: &YCbCr422Image,
    new_size: Option<(u32, u32)>,
) -> Option<GrayImage> {
    let grayscale_buffer: Vec<_> = image
        .buffer()
        .iter()
        .flat_map(|pixel| [pixel.y1, pixel.y2])
        .collect();

    if let Some(new_size) = new_size {
        let y_image = ImageView::from_buffer(
            NonZeroU32::new(image.width()).unwrap(),
            NonZeroU32::new(image.height()).unwrap(),
            &grayscale_buffer,
        );
        if let Ok(y_image) = y_image {
            let new_width = NonZeroU32::new(new_size.0).unwrap();
            let new_height = NonZeroU32::new(new_size.1).unwrap();
            let mut new_image =
                fast_image_resize::Image::new(new_width, new_height, y_image.pixel_type());
            let mut resizer = Resizer::new(ResizeAlg::Convolution(FilterType::Hamming));
            resizer
                .resize(&DynamicImageView::U8(y_image), &mut new_image.view_mut())
                .unwrap();
            GrayImage::from_vec(new_width.get(), new_height.get(), new_image.into_vec())
        } else {
            None
        }
    } else {
        GrayImage::from_vec(image.width(), image.height(), grayscale_buffer.to_vec())
    }
}

#[inline]
pub(crate) fn gray_image_resize(
    image: &GrayImage,
    // new_image_view: &mut DynamicImageViewMut,
    new_size: (u32, u32),
    filter: Option<FilterType>,
) -> fast_image_resize::Image<'_> {
    let image_view = ImageView::from_buffer(
        NonZeroU32::new(image.width()).unwrap(),
        NonZeroU32::new(image.height()).unwrap(),
        &image.as_raw(),
    )
    .expect("ImageView creation failed!");
    let new_width = NonZeroU32::new(new_size.0).unwrap();
    let new_height = NonZeroU32::new(new_size.1).unwrap();
    let mut new_image =
        fast_image_resize::Image::new(new_width, new_height, image_view.pixel_type());
    let mut resizer = Resizer::new(ResizeAlg::Convolution(
        filter.unwrap_or_else(|| FilterType::Hamming),
    ));
    let mut new_image_view = new_image.view_mut();

    resizer
        .resize(&DynamicImageView::U8(image_view), &mut new_image_view)
        .unwrap();
    new_image
}

// pub(crate) fn approximated_gaussian_blur(image: GrayImage, sigma: f32, box_filter_repeats: usize) {
//     let sigma_squared = sigma * sigma;
//     let width_ideal = (((12.0 * sigma_squared) + 1.0) / box_filter_repeats as f32).sqrt();
//     let box_filter_repeats_f32 = box_filter_repeats as f32;

//     // Get the nearest odd number that is lower than width_ideal
//     let width_lower = {
//         let width_floor = width_ideal.floor() as u32;
//         if width_floor % 2 != 0 {
//             width_floor
//         } else {
//             width_floor - 1
//         }
//     };
//     let width_lower_squared = (width_lower * width_lower) as f32;

//     let m_iteration_count_numerator = (12.0 * sigma_squared)
//         - (box_filter_repeats_f32 * width_lower_squared)
//         - (4.0 * box_filter_repeats_f32 * width_lower as f32)
//         - (3.0 * box_filter_repeats_f32);
//     let m_iteration_count =
//         (m_iteration_count_numerator / (-4.0 * width_lower as f32 - 4.0)) as usize;
// }
