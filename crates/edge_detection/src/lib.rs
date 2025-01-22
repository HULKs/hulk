use std::iter::from_fn;

use image::{GrayImage, Luma, RgbImage};
use imageproc::{edges::canny as imageproc_canny, map::map_colors};

use coordinate_systems::Pixel;
use linear_algebra::{point, Point2};
use nalgebra::{self as na, DMatrix, DMatrixView, Scalar};

use types::ycbcr422_image::YCbCr422Image;

use crate::canny::{canny, EdgeClassification};
pub mod canny;
pub mod filter2d;
pub mod gaussian;
pub mod sobel;

#[derive(Debug, Copy, Clone)]
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
    // exclude points above horizon_y (smaller than)
    horizon_y: Option<u32>,
) -> Vec<Point2<Pixel>> {
    let min_y = horizon_y.unwrap_or(0) as u32;
    let edges_source = get_edge_source_image_old(image, source_channel, horizon_y);

    imageproc_canny(&edges_source, canny_low_threshold, canny_high_threshold)
        .enumerate_pixels()
        .filter_map(|(x, y, color)| {
            if color[0] > 127 {
                Some(point![x as f32, (y + min_y) as f32])
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
    // exclude points above horizon_y (smaller than)
    horizon_y: Option<u32>,
) -> Vec<Point2<Pixel>> {
    let min_y = horizon_y.unwrap_or(0) as usize;

    let transposed_image = get_edge_source_transposed_image(image, source_channel, horizon_y);

    let (canny_image_matrix, point_count) = canny(
        transposed_image.as_view(),
        Some(gaussian_sigma),
        canny_low_threshold,
        canny_high_threshold,
    );

    let mut points = Vec::with_capacity(point_count);
    // Column major access AND transposed
    let canny_slice = canny_image_matrix.as_slice();
    (0..canny_image_matrix.ncols()).for_each(|y| {
        let col_offset = y * canny_image_matrix.nrows();
        (0..canny_image_matrix.nrows()).for_each(|x| {
            if canny_slice[col_offset + x] == EdgeClassification::HighConfidence {
                points.push(point![x as f32, (y + min_y) as f32]);
            }
        });
    });
    // canny_image_matrix
    //     .into_iter()
    //     .enumerate()
    //     .for_each(|(index, &value)| {
    //         if value == EdgeClassification::HighConfidence {
    //             let (x, y) = canny_image_matrix.vector_to_matrix_index(index);
    //             points.push(point![x as f32, (y + min_y) as f32]);
    //         }
    //     });
    points
}

/// Column major access for Nalgebra, hence the function name "transposed" (image width = nrows, image height = ncols)
pub fn get_edge_source_transposed_image(
    image: &YCbCr422Image,
    source_type: EdgeSourceType,
    horizon_y: Option<u32>,
) -> DMatrix<u8> {
    let min_y = horizon_y.unwrap_or(0).clamp(0, image.height() - 1) as usize;
    // iter_pixels() runs as row-major, so width * y gives the correct offset.
    let flat_slice_offset = image.width() as usize * min_y..;
    let new_height = image.height() as usize - min_y;

    match source_type {
        EdgeSourceType::DifferenceOfGrayAndRgbRange => {
            let rgb = RgbImage::from(image);
            let difference = rgb_image_to_difference(&rgb);
            DMatrix::<u8>::from_column_slice(
                image.width() as usize,
                new_height,
                &difference.as_raw()[flat_slice_offset],
            )
        }
        EdgeSourceType::LumaOfYCbCr => {
            // iterator skip has some serious overheads.
            let grayscale_buffer: Vec<_> = image.iter_pixels().map(|pixel| pixel.y).collect();
            DMatrix::<u8>::from_column_slice(
                image.width() as usize,
                new_height,
                &grayscale_buffer[flat_slice_offset],
            )
        }
    }
}

pub fn get_edge_source_image_old(
    image: &YCbCr422Image,
    source_type: EdgeSourceType,
    horizon_y: Option<u32>,
) -> GrayImage {
    let min_y = horizon_y.unwrap_or(0).clamp(0, image.height() - 1) as usize;
    // iter_pixels() runs as row-major, so width * y gives the correct offset.
    let flat_slice_offset = image.width() as usize * min_y..;
    let new_height = image.height() - min_y as u32;
    let new_vec = match source_type {
        EdgeSourceType::DifferenceOfGrayAndRgbRange => {
            let rgb = RgbImage::from(image);
            let difference = rgb_image_to_difference(&rgb);
            difference.as_raw()[flat_slice_offset].to_vec()
        }
        EdgeSourceType::LumaOfYCbCr => {
            let grayscale_buffer: Vec<_> = image.iter_pixels().map(|pixel| pixel.y).collect();

            grayscale_buffer[flat_slice_offset].to_vec()
        }
    };
    GrayImage::from_vec(image.width(), new_height, new_vec)
        .expect("GrayImage construction after resize failed")
}

fn rgb_image_to_difference(rgb: &RgbImage) -> GrayImage {
    map_colors(rgb, |color| Luma([rgb_pixel_to_difference(&color)]))
}

#[inline(always)]
fn rgb_pixel_to_difference(rgb: &image::Rgb<u8>) -> u8 {
    let raw = &rgb.0;
    let minimum = raw[0].min(raw[1]).min(raw[2]) as i16;
    let maximum = raw[0].max(raw[1]).max(raw[2]) as i16;

    let gray = (raw[0] as i16 + raw[1] as i16 + raw[2] as i16) / 3;
    let diff = maximum - minimum;
    (gray - diff).clamp(0, 255) as u8
}

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

pub fn transposed_matrix_view_to_gray_image<T>(transposed_image: DMatrixView<T>) -> GrayImage
where
    T: Into<u8> + Copy,
{
    let (width, height) = transposed_image.shape(); // rows, columns -> width, height
    let mut out: GrayImage = GrayImage::new(width as u32, height as u32);
    assert!(
        out.len() >= transposed_image.len(),
        "The output image is too small"
    );

    out.as_mut()
        .iter_mut()
        .zip(transposed_image.iter())
        .for_each(|(out, &value)| {
            *out = value.into();
        });

    out
}

// Profiling says it is faster than enumerate + zip chain
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

pub(crate) fn get_test_data_location() -> String {
    option_env!("TEST_DATA_ROOT")
        .unwrap_or(env!("CARGO_MANIFEST_DIR"))
        .to_string()
}
// TODO find a way to inject this to the bencher without pub
pub(crate) fn load_test_image() -> YCbCr422Image {
    let test_data_root = get_test_data_location();
    YCbCr422Image::load_from_rgb_file(format!(
        "{test_data_root}/test_data/center_circle_webots.png"
    ))
    .unwrap()
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
