use core::f32;
use num_traits::{AsPrimitive, PrimInt};
use std::ops::{Div, Mul, MulAssign};

use image::{GrayImage, ImageBuffer, Luma};
use imageproc::filter::box_filter;
use nalgebra::{DMatrix, SMatrix, Scalar};

use crate::conv::direct_convolution;

/// Gaussian smoothing approximation with box filters
/// - https://en.wikipedia.org/wiki/Gaussian_blur
/// - Kovesi, Peter. "Fast almost-gaussian filtering."
///     2010 International Conference on Digital Image Computing: Techniques and Applications. IEEE, 2010.
pub fn gaussian_blur_box_filter(image: &GrayImage, sigma: f32) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    // average sigma = sqrt( (w**2 -1) / 12 ): w is box width, n is passes

    const PASSES: usize = 2;
    let w_ideal_half = ((12.0 * sigma.div(2.0).powi(2) / (PASSES as f32)) + 1.0)
        .sqrt()
        .div(2.0)
        .round() as u32
        - 1;

    let mut output = box_filter(image, w_ideal_half, w_ideal_half);

    for _ in 1..PASSES {
        output = box_filter(&output, w_ideal_half, w_ideal_half);
    }

    output
}

// TODO remove after int approximation work is done
#[allow(dead_code)]
#[inline]
fn gaussian(x: f32, r: f32) -> f32 {
    ((2.0 * f32::consts::PI).sqrt() * r).recip() * (-x.powi(2) / (2.0 * r.powi(2))).exp()
}

pub fn gaussian_blur_try_2_nalgebra<InputType>(
    image: &DMatrix<InputType>,
    _sigma: f32,
) -> DMatrix<OutputType>
where
    InputType: Into<KernelType>
        + AsPrimitive<f32>
        + AsPrimitive<KernelType>
        + PrimInt
        + Scalar
        + Mul
        + MulAssign,
    KernelType: PrimInt,
    // OutputType: Into<KernelType>,
{
    // let kernel = imgproc_kernel_to_matrix::<3>(&GAUSSIAN_BLUR_3x3);

    let kernel = SMatrix::<KernelType, 3, 3>::from_row_slice(&[1, 2, 1, 2, 4, 2, 1, 2, 1]);

    let max = 16;
    direct_convolution::<3, InputType, KernelType, OutputType>(image, &kernel) / max
}

type KernelType = i32;
type OutputType = i16;
/// Gaussian smoothing approximation with box filters
/// - https://en.wikipedia.org/wiki/Gaussian_blur
/// - Kovesi, Peter. "Fast almost-gaussian filtering."
///     2010 International Conference on Digital Image Computing: Techniques and Applications. IEEE, 2010.
pub fn gaussian_blur_box_filter_nalgebra<InputType>(
    transposed_image: &DMatrix<InputType>,
    sigma: f32,
) -> DMatrix<OutputType>
where
    InputType: Into<KernelType> + AsPrimitive<KernelType> + PrimInt + Scalar + Mul + MulAssign,
{
    // average sigma = sqrt( (w**2 -1) / 12 ): w is box width, n is passes

    const PASSES: usize = 2;
    // box_filter_direct_convolve::<3, T>(transposed_image, PASSES)

    let w_ideal_half = ((12.0 * sigma.div(2.0).powi(2) / (PASSES as f32)) + 1.0)
        .sqrt()
        .div(2.0)
        .round() as u32
        - 1;

    match w_ideal_half {
        0 => box_filter_direct_convolve::<3, InputType>(transposed_image, PASSES),
        1 => box_filter_direct_convolve::<5, InputType>(transposed_image, PASSES),
        2 => box_filter_direct_convolve::<7, InputType>(transposed_image, PASSES),
        _ => unreachable!("Box filter width must be between 3 and 11"),
    }
}

#[inline(always)]
fn box_filter_direct_convolve<const KSIZE: usize, InputType>(
    transposed_image: &DMatrix<InputType>,
    passes: usize,
) -> DMatrix<OutputType>
where
    InputType: Into<KernelType> + AsPrimitive<KernelType> + PrimInt + Scalar + Mul + MulAssign,
    KernelType: PrimInt,
    OutputType: Into<KernelType>,
{
    let scale_value = (KSIZE as OutputType).pow(2);

    let kernel = SMatrix::<KernelType, KSIZE, KSIZE>::repeat(1);
    let mut first =
        direct_convolution::<KSIZE, InputType, KernelType, OutputType>(transposed_image, &kernel);
    first /= scale_value;
    for _ in 1..passes {
        first = direct_convolution::<KSIZE, OutputType, KernelType, OutputType>(&first, &kernel);
        first /= scale_value;
    }
    first
}

#[cfg(test)]
mod tests {
    use crate::grayimage_to_2d_transposed_matrix_view;

    use super::*;
    use image::open;

    #[test]
    fn test_gaussian_box_filter() {
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        let image = open(format!("{crate_dir}/test_data/center_circle_webots.png"))
            .expect("The image should be in this path");

        let blurred = gaussian_blur_box_filter(&image.to_luma8(), 3.5);

        blurred
            .save(format!(
                "{crate_dir}/test_data/output/gaussian_box_filter.png"
            ))
            .expect("The image saving should not fail");
    }

    #[test]
    fn test_gaussian_box_filter_nalgebra() {
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        let image = open(format!("{crate_dir}/test_data/center_circle_webots.png"))
            .expect("The image should be in this path");

        let luma8 = image.to_luma8();
        let converted = grayimage_to_2d_transposed_matrix_view(&luma8);
        let blurred = gaussian_blur_box_filter_nalgebra::<u8>(&converted, 3.5);

        let blurred_int_approximation = gaussian_blur_try_2_nalgebra(&converted, 3.5);
        GrayImage::from_raw(
            image.width(),
            image.height(),
            blurred.iter().map(|&v| v as u8).collect::<Vec<u8>>(),
        )
        .unwrap()
        .save(format!(
            "{crate_dir}/test_data/output/gaussian_box_filter_nalgebra.png"
        ))
        .expect("The image saving should not fail");

        GrayImage::from_raw(
            image.width(),
            image.height(),
            blurred_int_approximation
                .iter()
                .map(|&v| v as u8)
                .collect::<Vec<u8>>(),
        )
        .unwrap()
        .save(format!(
            "{crate_dir}/test_data/output/gaussian_box_filter_nalgebra_int_approx.png"
        ))
        .expect("The image saving should not fail");
    }
}
