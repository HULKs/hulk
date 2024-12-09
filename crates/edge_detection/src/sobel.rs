use std::ops::{Mul, MulAssign};

use imageproc::gradients::{horizontal_sobel, vertical_sobel, HORIZONTAL_SOBEL, VERTICAL_SOBEL};

use coordinate_systems::Pixel;
use linear_algebra::{point, Point2};

use nalgebra::{DMatrix, Scalar, SimdPartialOrd};
use types::ycbcr422_image::YCbCr422Image;

use crate::{
    canny::non_maximum_suppression,
    conv::{direct_convolution, imgproc_kernel_to_matrix},
    gaussian::gaussian_blur_box_filter,
    get_edge_source_image, grayimage_to_2d_transposed_matrix_view, EdgeSourceType,
};

#[inline]
pub fn sobel_operator_vertical<const K: usize, T>(
    image_view_transposed: &DMatrix<T>,
) -> DMatrix<i16>
where
    T: Clone + Copy + Scalar + Mul + MulAssign + SimdPartialOrd,
    i16: From<T>,
{
    let kernel = imgproc_kernel_to_matrix::<K>(&VERTICAL_SOBEL);

    direct_convolution::<K, T>(&image_view_transposed, &kernel, None)
}

#[inline]
pub fn sobel_operator_horizontal<const K: usize, T>(
    image_view_transposed: &DMatrix<T>,
) -> DMatrix<i16>
where
    T: Clone + Copy + Scalar + Mul + MulAssign + SimdPartialOrd,
    i16: From<T>,
{
    let kernel = imgproc_kernel_to_matrix::<K>(&HORIZONTAL_SOBEL);

    direct_convolution::<K, T>(&image_view_transposed, &kernel, None)
}

pub fn get_edges_sobel(
    gaussian_sigma: f32,
    threshold: u16,
    image: &YCbCr422Image,
    source_channel: EdgeSourceType,
) -> Vec<Point2<Pixel>> {
    let edges_source = get_edge_source_image(image, source_channel);
    let blurred = gaussian_blur_box_filter(&edges_source, gaussian_sigma);

    let gradients_vertical = vertical_sobel(&blurred);
    let gradients_horizontal = horizontal_sobel(&blurred);

    let squared_threshold = (threshold as i32).pow(2);
    gradients_vertical
        .enumerate_pixels()
        .filter_map(|(x, y, gradient_vertical)| {
            let magnitude = (gradients_horizontal[(x, y)][0] as i32).pow(2)
                + (gradient_vertical[0] as i32).pow(2);

            if magnitude > squared_threshold {
                Some(point![x as f32, y as f32])
            } else {
                None
            }
        })
        .collect()
}

pub fn get_edges_sobel_nalgebra(
    gaussian_sigma: f32,
    threshold: u16,
    image: &YCbCr422Image,
    source_channel: EdgeSourceType,
) -> Vec<Point2<Pixel>> {
    let edges_source = get_edge_source_image(image, source_channel);
    // let converted = grayimage_to_2d_transposed_matrix_view(&edges_source);
    // let blurred = gaussian_blur_box_filter_nalgebra(&converted, gaussian_sigma);
    // let gaussed = gaussian_blur_f32(&edges_source, gaussian_sigma);
    // let blurred = grayimage_to_2d_transposed_matrix_view(&gaussed);
    let blurred = gaussian_blur_box_filter(&edges_source, gaussian_sigma);
    let converted = grayimage_to_2d_transposed_matrix_view(&blurred).clone_owned();

    const KERNEL_SIZE: usize = 3;
    let min_x = KERNEL_SIZE / 2;
    let max_x = image.width() as usize - min_x;

    let gradients_y_transposed = sobel_operator_vertical::<KERNEL_SIZE, u8>(&converted);
    let gradients_x_transposed = sobel_operator_horizontal::<KERNEL_SIZE, u8>(&converted);

    let suppressed = non_maximum_suppression(&gradients_x_transposed, &gradients_y_transposed);
    let mut points = Vec::new();

    suppressed.column_iter().enumerate().for_each(|(y, col)| {
        let col_slice = &col.as_slice()[min_x..max_x];
        col_slice.iter().enumerate().for_each(|(x, gradient)| {
            if *gradient > threshold {
                points.push(point![x as f32, y as f32]);
            }
        })
    });
    points
}

#[cfg(test)]
mod tests {

    use super::*;

    const EDGE_SOURCE_TYPE: EdgeSourceType = EdgeSourceType::LumaOfYCbCr;
    const GAUSSIAN_SIGMA: f32 = 3.5;
    const THRESHOLD: u16 = 0; // allow everything

    fn load_test_image() -> YCbCr422Image {
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        YCbCr422Image::load_from_rgb_file(format!("{crate_dir}/test_data/center_circle_webots.png"))
            .unwrap()
    }

    #[test]
    fn compare_edge_creation_imageproc_sobel_with_direct_sobel() {
        let image = load_test_image();

        let kernel_size = 3;
        let (min_x, min_y) = (kernel_size / 2, kernel_size / 2);
        let (max_x, max_y) = (image.width() - min_x, image.height() - min_y);

        let expected_points: Vec<Point2<Pixel>> =
            get_edges_sobel(GAUSSIAN_SIGMA, THRESHOLD, &image, EDGE_SOURCE_TYPE)
                .into_iter()
                .filter(|p| {
                    p.x() > min_x as f32
                        && p.x() < max_x as f32
                        && p.y() > min_y as f32
                        && p.y() < max_y as f32
                })
                .collect();
        let output_points: Vec<Point2<Pixel>> =
            get_edges_sobel_nalgebra(GAUSSIAN_SIGMA, THRESHOLD, &image, EDGE_SOURCE_TYPE)
                .into_iter()
                .filter(|p| {
                    p.x() > min_x as f32
                        && p.x() < max_x as f32
                        && p.y() > min_y as f32
                        && p.y() < max_y as f32
                })
                .collect();

        assert_eq!(output_points.len(), expected_points.len());
        for (gradient, expected) in output_points.iter().zip(expected_points.iter()) {
            assert_eq!(gradient, expected);
        }
    }

    #[test]
    fn compare_imageproc_sobel_with_direct_convolution() {
        let edges_source = get_edge_source_image(&load_test_image(), EDGE_SOURCE_TYPE);
        let blurred = gaussian_blur_box_filter(&edges_source, GAUSSIAN_SIGMA);

        let kernel = imgproc_kernel_to_matrix(&VERTICAL_SOBEL);
        let image_view_transposed = grayimage_to_2d_transposed_matrix_view(&blurred).clone_owned();

        let sobel_image_transposed =
            direct_convolution::<3, u8>(&image_view_transposed, &kernel, None);
        let imageproc_sobel = vertical_sobel(&blurred);

        let kernel_size = 3;
        let (min_x, min_y) = (kernel_size / 2, kernel_size / 2);
        let (max_x, max_y) = (blurred.width() - min_x, blurred.height() - min_y);

        assert_eq!(
            sobel_image_transposed.shape().0,
            imageproc_sobel.width() as usize,
            "{:?} {:?}",
            sobel_image_transposed.shape(),
            (imageproc_sobel.width(), imageproc_sobel.height())
        );
        assert_eq!(
            sobel_image_transposed.shape().1,
            imageproc_sobel.height() as usize
        );

        imageproc_sobel
            .enumerate_pixels()
            .for_each(|(x, y, pixel)| {
                if x <= min_x || x >= max_x || y <= min_y || y >= max_y {
                    return;
                }
                let sobel_pixel = sobel_image_transposed[(x as usize, y as usize)];
                assert_eq!(pixel[0], sobel_pixel);
            });
    }
}
