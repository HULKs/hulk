use std::iter::from_fn;

use canny::{canny, EdgeClassification};
use coordinate_systems::Pixel;
use image::{GrayImage, Luma, RgbImage};
use imageproc::{edges::canny as imageproc_canny, map::map_colors};
use linear_algebra::{point, Point2};
use nalgebra::{self as na, Scalar};

use types::ycbcr422_image::YCbCr422Image;

pub mod canny;
pub mod conv;
pub mod gaussian;
pub mod sobel;

pub enum EdgeSourceType {
    DifferenceOfGrayAndRgbRange,
    LumaOfYCbCr,
    // TODO Add HSV based approaches - https://github.com/HULKs/hulk/pull/1078, https://github.com/HULKs/hulk/pull/1081
}

pub fn get_edges_canny_imageproc(
    _gaussian_sigma: f32,
    canny_low_threshold: f32,
    canny_high_threshold: f32,
    image: &YCbCr422Image,
    source_channel: EdgeSourceType,
) -> Vec<Point2<Pixel>> {
    let edges_source = get_edge_source_image(image, source_channel);

    imageproc_canny(&edges_source, canny_low_threshold, canny_high_threshold)
        .enumerate_pixels()
        .filter_map(|(x, y, color)| {
            if color[0] > 127 {
                Some(point![x as f32, y as f32])
            } else {
                None
            }
        })
        .collect()
}

pub fn get_edges_canny(
    gaussian_sigma: f32,
    canny_low_threshold: f32,
    canny_high_threshold: f32,
    image: &YCbCr422Image,
    source_channel: EdgeSourceType,
) -> Vec<Point2<Pixel>> {
    let edges_source = get_edge_source_image(image, source_channel);

    let (canny_image_matrix, point_count) = canny(
        &edges_source,
        Some(gaussian_sigma),
        canny_low_threshold,
        canny_high_threshold,
    );

    let mut points = Vec::with_capacity(point_count);
    canny_image_matrix
        .into_iter()
        .enumerate()
        .for_each(|(index, &value)| {
            if value >= EdgeClassification::LowConfidence {
                let (x, y) = canny_image_matrix.vector_to_matrix_index(index);
                points.push(point![x as f32, y as f32]);
            }
        });
    points
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

// #[inline]
// fn grayimage_to_2d_matrix(image: &GrayImage) -> na::DMatrix<u8> {
//     let data = image.as_raw();

//     na::DMatrix::from_iterator(
//         image.height() as usize,
//         image.width() as usize,
//         data.iter().copied(),
//     )
// }

#[inline]
pub fn grayimage_to_2d_transposed_matrix_view<T>(image: &GrayImage) -> na::DMatrix<T>
where
    T: Scalar + Copy + From<u8>,
    u8: Into<T>,
{
    let data = image.as_raw();

    na::DMatrix::<T>::from_iterator(
        image.width() as usize,
        image.height() as usize,
        data.iter().map(|&v| v.into()),
    )
}

// Just to see if this is faster than chaining zips and enumerate
// Update: Profiling says it is faster!
#[inline]
fn zip_three_slices_enumerated<'a, T, U, V>(
    mut slice1: &'a [T],
    mut slice2: &'a [U],
    mut slice3: &'a [V],
) -> impl Iterator<Item = (usize, &'a T, &'a U, &'a V)> + 'a {
    // unsafe {
    assert!(slice1.len() == slice2.len() && slice2.len() == slice3.len());
    let len = slice1.len();

    let mut counter = 0;

    from_fn(move || {
        counter += 1;
        if len == 0 {
            None
        } else if let ([a, _slice1 @ ..], [b, _slice2 @ ..], [c, _slice3 @ ..]) =
            (slice1, slice2, slice3)
        {
            slice1 = _slice1;
            slice2 = _slice2;
            slice3 = _slice3;
            Some((counter, a, b, c))
        } else {
            None
        }
    })
    // }
}

/// Why? Profiling showed that the compiler optimizes this better than directly using if KSIZE % 2 == 1 { ... }
#[inline(always)]
pub(crate) const fn is_ksize_odd(ksize: usize) -> bool {
    return ksize % 2 == 1;
}

#[cfg(test)]
mod tests {
    use image::ImageBuffer;

    use super::*;

    #[test]
    fn verify_matrix_view() {
        let width = 5;
        let height = 3;
        let buf: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
        let image = ImageBuffer::from_vec(width, height, buf).unwrap();
        let nalgebra_matrix_view_transposed = grayimage_to_2d_transposed_matrix_view::<u8>(&image);

        image.enumerate_pixels().for_each(|(x, y, pixel)| {
            let nalgebra_pixel = nalgebra_matrix_view_transposed[(x as usize, y as usize)];
            assert_eq!(pixel[0], nalgebra_pixel, "x: {}, y: {}", x, y);
        });
    }
}
