use coordinate_systems::Pixel;
use image::{GrayImage, Luma, RgbImage};
use imageproc::{
    edges::canny, filter::gaussian_blur_f32, gradients::vertical_sobel, map::map_colors,
};

use linear_algebra::{point, Point2};
use types::ycbcr422_image::YCbCr422Image;

pub enum EdgeSourceType {
    DifferenceOfGrayAndRgbRange,
    LumaOfYCbCr,
    // TODO Add HSV based approaches - https://github.com/HULKs/hulk/pull/1078, https://github.com/HULKs/hulk/pull/1081
}

pub fn get_edge_image_canny(
    gaussian_sigma: f32,
    canny_low_threshold: f32,
    canny_high_threshold: f32,
    image: &YCbCr422Image,
    source_channel: EdgeSourceType,
) -> GrayImage {
    let edges_source = get_edge_source_image(image, source_channel);
    let blurred = gaussian_blur_f32(&edges_source, gaussian_sigma);

    canny(&blurred, canny_low_threshold, canny_high_threshold)
}

pub fn get_edges_sobel(
    gaussian_sigma: f32,
    threshold: u16,
    image: &YCbCr422Image,
    source_channel: EdgeSourceType,
) -> Vec<Point2<Pixel>> {
    let edges_source = get_edge_source_image(image, source_channel);
    let blurred = gaussian_blur_f32(&edges_source, gaussian_sigma);

    let gradients = vertical_sobel(&blurred);

    gradients
        .enumerate_pixels()
        .filter_map(|(x, y, color)| {
            if color[0].unsigned_abs() < threshold {
                Some(point![x as f32, y as f32])
            } else {
                None
            }
        })
        .collect()
}

pub fn get_edge_source_image(image: &YCbCr422Image, source_type: EdgeSourceType) -> GrayImage {
    match source_type {
        EdgeSourceType::DifferenceOfGrayAndRgbRange => {
            let rgb = RgbImage::from(image);

            let difference = rgb_image_to_difference(&rgb);

            GrayImage::from_vec(
                difference.width(),
                difference.height(),
                difference.into_vec(),
            )
            .expect("GrayImage construction after resize failed")
        }
        EdgeSourceType::LumaOfYCbCr => {
            generate_luminance_image(image).expect("Generating luma image failed")
        }
    }
}

fn generate_luminance_image(image: &YCbCr422Image) -> Option<GrayImage> {
    let grayscale_buffer: Vec<_> = image.iter_pixels().map(|pixel| pixel.y).collect();
    GrayImage::from_vec(image.width(), image.height(), grayscale_buffer)
}

fn rgb_image_to_difference(rgb: &RgbImage) -> GrayImage {
    map_colors(rgb, |color| {
        Luma([
            (rgb_pixel_to_gray(&color) - rgb_pixel_to_difference(&color) as i16).clamp(0, 255)
                as u8,
        ])
    })
}

#[inline]
fn rgb_pixel_to_gray(rgb: &image::Rgb<u8>) -> i16 {
    (rgb[0] as i16 + rgb[1] as i16 + rgb[2] as i16) / 3
}

#[inline]
fn rgb_pixel_to_difference(rgb: &image::Rgb<u8>) -> u8 {
    let minimum = rgb.0.iter().min().unwrap();
    let maximum = rgb.0.iter().max().unwrap();
    maximum - minimum
}
