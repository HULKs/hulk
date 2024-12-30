use std::{
    num::NonZeroU32,
    ops::{Mul, MulAssign},
};

use coordinate_systems::Pixel;
use imageproc::gradients::{
    horizontal_sobel as imageproc_horizontal_sobel, vertical_sobel as imageproc_vertical_sobel,
};
use linear_algebra::{point, Point2};

use nalgebra::{DMatrix, DMatrixView, Scalar, SimdPartialOrd};
use num_traits::{AsPrimitive, PrimInt};
use simba::scalar::{SubsetOf, SupersetOf};
use types::ycbcr422_image::YCbCr422Image;

use crate::{
    canny::{non_maximum_suppression, EdgeClassification},
    conv::piecewise_2d_convolution_mut,
    gaussian::gaussian_blur_box_filter,
    get_edge_source_image, grayimage_to_2d_transposed_matrix_view, EdgeSourceType,
};

#[inline]
pub fn sobel_operator_vertical<T>(image_view_transposed: DMatrixView<T>) -> DMatrix<i16>
where
    T: AsPrimitive<i32> + SubsetOf<i32> + PrimInt + Scalar + Mul + MulAssign + SimdPartialOrd,
    i16: From<T>,
    i32: SupersetOf<T>,
{
    let piecewise_kernel_horizontal = [1, 2, 1];
    let piecewise_kernel_vertical = [-1, 0, 1];

    let mut out =
        DMatrix::<i16>::zeros(image_view_transposed.nrows(), image_view_transposed.ncols());
    piecewise_2d_convolution_mut(
        image_view_transposed,
        out.as_mut_slice(),
        &piecewise_kernel_horizontal,
        &piecewise_kernel_vertical,
        NonZeroU32::new(1).unwrap(),
    );
    out
}

#[inline]
pub fn sobel_operator_horizontal<T>(image_view_transposed: DMatrixView<T>) -> DMatrix<i16>
where
    T: AsPrimitive<i32> + SubsetOf<i32> + PrimInt + Scalar + Mul + MulAssign + SimdPartialOrd,
    i16: From<T>,
    i32: SupersetOf<T>,
{
    let piecewise_kernel_horizontal = [-1, 0, 1];
    let piecewise_kernel_vertical = [1, 2, 1];

    let mut out =
        DMatrix::<i16>::zeros(image_view_transposed.nrows(), image_view_transposed.ncols());
    piecewise_2d_convolution_mut(
        image_view_transposed,
        out.as_mut_slice(),
        &piecewise_kernel_horizontal,
        &piecewise_kernel_vertical,
        NonZeroU32::new(1).unwrap(),
    );
    out
}

pub fn get_edges_sobel(
    gaussian_sigma: f32,
    threshold: u16,
    image: &YCbCr422Image,
    source_channel: EdgeSourceType,
) -> Vec<Point2<Pixel>> {
    let edges_source = get_edge_source_image(image, source_channel);
    let blurred = gaussian_blur_box_filter(&edges_source, gaussian_sigma);

    let gradients_vertical = imageproc_vertical_sobel(&blurred);
    let gradients_horizontal = imageproc_horizontal_sobel(&blurred);

    let decisions = non_maximum_suppression(
        &DMatrix::<i16>::from_iterator(
            image.width() as usize,
            image.height() as usize,
            gradients_horizontal.iter().copied(),
        ),
        &DMatrix::<i16>::from_iterator(
            image.width() as usize,
            image.height() as usize,
            gradients_vertical.iter().copied(),
        ),
        threshold,
        threshold,
    );

    decisions
        .iter()
        .enumerate()
        .filter_map(|(index, decision)| {
            if *decision >= EdgeClassification::LowConfidence {
                let (x, y) = decisions.vector_to_matrix_index(index);
                Some(point![x as f32, y as f32])
            } else {
                None
            }
        })
        .collect()
}

pub fn get_edges_sobel_nalgebra(
    gaussian_sigma: f32,
    low_threshold: u16,
    high_threshold: u16,
    image: &YCbCr422Image,
    source_channel: EdgeSourceType,
) -> Vec<Point2<Pixel>> {
    let edges_source = get_edge_source_image(image, source_channel);

    let blurred = gaussian_blur_box_filter(&edges_source, gaussian_sigma);
    let converted = grayimage_to_2d_transposed_matrix_view(&blurred);
    let converted_view = converted.as_view();

    let gradients_y_transposed = sobel_operator_vertical::<u8>(converted_view);
    let gradients_x_transposed = sobel_operator_horizontal::<u8>(converted_view);

    let decisions = non_maximum_suppression(
        &gradients_x_transposed,
        &gradients_y_transposed,
        low_threshold,
        high_threshold,
    );

    decisions
        .iter()
        .enumerate()
        .filter_map(|(index, decision)| {
            if *decision >= EdgeClassification::LowConfidence {
                let (x, y) = decisions.vector_to_matrix_index(index);
                Some(point![x as f32, y as f32])
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {

    use image::{GrayImage, ImageBuffer, Luma};
    use imageproc::{
        definitions::HasWhite,
        filter::gaussian_blur_f32,
        gradients::{
            horizontal_sobel as imageproc_horizontal_sobel,
            vertical_sobel as imageproc_vertical_sobel,
        },
    };
    use nalgebra::{Matrix, ViewStorage};

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

        let output_points: Vec<Point2<Pixel>> = get_edges_sobel_nalgebra(
            GAUSSIAN_SIGMA,
            THRESHOLD,
            THRESHOLD,
            &image,
            EDGE_SOURCE_TYPE,
        )
        .into_iter()
        .filter(|p| {
            p.x() > min_x as f32
                && p.x() < max_x as f32
                && p.y() > min_y as f32
                && p.y() < max_y as f32
        })
        .collect();

        {
            let mut new_image = GrayImage::new(image.width(), image.height());
            output_points.iter().for_each(|point| {
                new_image[(point.x() as u32, point.y() as u32)] = Luma::white();
            });
            new_image
                .save(format!(
                    "{}/test_data/output/sobel_direct_points_nalgebra.png",
                    env!("CARGO_MANIFEST_DIR")
                ))
                .unwrap();
            new_image.fill(0);

            expected_points.iter().for_each(|point| {
                new_image[(point.x() as u32, point.y() as u32)] = Luma::white();
            });
            new_image
                .save(format!(
                    "{}/test_data/output/sobel_direct_points_expected.png",
                    env!("CARGO_MANIFEST_DIR")
                ))
                .unwrap();
        }

        // 0 --> no diff
        // 128 --> Only present in expected
        // 255 --> Only present in output
        let mut diff = DMatrix::<u8>::zeros(image.height() as usize, image.width() as usize);
        expected_points.iter().for_each(|point| {
            diff[(point.y() as usize, point.x() as usize)] = 128;
        });
        output_points.iter().for_each(|point| {
            // If the location is already marked as 1, then it is a match => no diff (0)
            diff[(point.y() as usize, point.x() as usize)] =
                if diff[(point.y() as usize, point.x() as usize)] == 128 {
                    0
                } else {
                    255
                };
        });
        {
            GrayImage::from_raw(image.width(), image.height(), diff.data.as_vec().clone())
                .unwrap()
                .save(format!(
                    "{}/test_data/output/sobel_direct_diff.png",
                    env!("CARGO_MANIFEST_DIR")
                ))
                .unwrap();
        }
        // For unknown reasons, there are very minor differences in the two varients.
        let non_zero_diffs = diff.iter().filter(|&v| *v != 0).count();
        assert!(
            non_zero_diffs <= 32,
            "Too many non-zero diffs: {non_zero_diffs}"
        );
        assert!((output_points.len() as isize - expected_points.len() as isize).abs() <= 10);
    }

    #[test]
    fn compare_imageproc_sobel_with_piecewise() {
        let edges_source = get_edge_source_image(&load_test_image(), EDGE_SOURCE_TYPE);
        let blurred = gaussian_blur_f32(&edges_source, GAUSSIAN_SIGMA);

        // TODO remove once the operators handle the boundaries
        let kernel_size = 3;
        let (min_x, min_y) = (kernel_size / 2, kernel_size / 2);
        let (max_x, max_y) = (blurred.width() - min_x, blurred.height() - min_y);

        let image_view_transposed = grayimage_to_2d_transposed_matrix_view::<i16>(&blurred);

        let operators: &[(
            &str,
            for<'a> fn(&'a GrayImage) -> ImageBuffer<Luma<i16>, Vec<i16>>,
            for<'a> fn(Matrix<_, _, _, ViewStorage<'a, _, _, _, _, _>>) -> DMatrix<i16>,
        )] = &[
            (
                "horizontal",
                imageproc_horizontal_sobel,
                sobel_operator_horizontal::<i16>,
            ),
            (
                "vertical",
                imageproc_vertical_sobel,
                sobel_operator_vertical::<i16>,
            ),
        ];
        for (name, imageproc_operator, our_operator) in operators {
            let sobel_image_ours = our_operator(image_view_transposed.as_view());
            let sobel_image_imageproc = imageproc_operator(&blurred);

            assert_eq!(
                sobel_image_ours.shape().0,
                sobel_image_imageproc.width() as usize,
                "{:?} {:?}",
                sobel_image_ours.shape(),
                (
                    sobel_image_imageproc.width(),
                    sobel_image_imageproc.height()
                )
            );
            assert_eq!(
                sobel_image_ours.shape().1,
                sobel_image_imageproc.height() as usize
            );

            {
                let mut new_image = GrayImage::new(blurred.width(), blurred.height());

                sobel_image_ours
                    .iter()
                    .enumerate()
                    .for_each(|(index, pixel)| {
                        let (x, y) = sobel_image_ours.vector_to_matrix_index(index);
                        new_image[(x as u32, y as u32)][0] = *pixel as u8; //(pixel >> 15).abs() as u8;
                    });
                new_image
                    .save(format!(
                        "{}/test_data/output/sobel_{name}_piecewise_our.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                    .unwrap();
                new_image.fill(0);

                sobel_image_imageproc
                    .enumerate_pixels()
                    .for_each(|(x, y, pixel)| {
                        new_image[(x as u32, y as u32)][0] = pixel[0] as u8; //(pixel[0] >> 15).abs() as u8;
                    });
                new_image
                    .save(format!(
                        "{}/test_data/output/sobel_{name}_improc.png",
                        env!("CARGO_MANIFEST_DIR")
                    ))
                    .unwrap();
            }
            sobel_image_imageproc
                .enumerate_pixels()
                .for_each(|(x, y, pixel)| {
                    if x <= min_x || x >= max_x || y <= min_y || y >= max_y {
                        return;
                    }
                    let sobel_pixel = sobel_image_ours[(x as usize, y as usize)];
                    assert_eq!(pixel[0], sobel_pixel);
                });
        }
    }
}
