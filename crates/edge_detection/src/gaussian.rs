use core::f32;
use num_traits::{AsPrimitive, PrimInt};
use simba::{scalar::SupersetOf, simd::PrimitiveSimdValue};
use std::{
    num::NonZeroU32,
    ops::{Div, Mul, MulAssign},
};

use image::{GrayImage, ImageBuffer, Luma};
use imageproc::filter::box_filter;
use nalgebra::{DMatrix, SMatrix, Scalar};

use crate::conv::{direct_convolution, direct_convolution_mut};

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

#[inline(always)]
fn gaussian(x: f32, r: f32) -> f32 {
    ((2.0 * f32::consts::PI).sqrt() * r).recip() * (-x.powi(2) / (2.0 * r.powi(2))).exp()
}

#[inline(always)]
fn gaussian_int_divisor_as_power_of_two(width: usize) -> u32 {
    match width {
        2 => 8,
        3 => 10,
        5 => 12,
        7 => 14,
        11 => 16,
        _ => panic!("Unsupported kernel size"),
    }
}
const fn check_kernel_size(width: usize) {
    assert!(width % 2 == 1, "Kernel size must be odd");
}

#[inline(always)]
fn gaussian_2d_integer_kernel_2d<const S: usize>(sigma: f32) -> (SMatrix<i32, S, S>, NonZeroU32) {
    let mut one_size = [0f32; S];
    let k_half = S / 2;
    check_kernel_size(S);

    for i in 0..((S / 2) + 1) {
        let v = gaussian(i as f32, sigma);
        one_size[k_half - i] = v;
        one_size[k_half + i] = v;
    }

    let mut kernel_float = SMatrix::<f32, S, S>::zeros();

    for j in 0..S {
        for i in 0..S {
            kernel_float[(i, j)] = one_size[i] * one_size[j];
        }
    }

    let int_factor_power_of_two = gaussian_int_divisor_as_power_of_two(S);
    let sum = kernel_float.sum();
    kernel_float = kernel_float / sum * 2.pow(int_factor_power_of_two) as f32;

    (
        kernel_float.map(|v| v as i32),
        // int_factor_power_of_two as usize,
        NonZeroU32::new(2.pow(int_factor_power_of_two)).unwrap(),
    )
}

pub fn gaussian_blur_try_2_nalgebra<InputType>(
    image: &DMatrix<InputType>,
    sigma: f32,
) -> DMatrix<OutputType>
where
    InputType: Into<KernelType>
        + AsPrimitive<f32>
        + AsPrimitive<KernelType>
        + PrimInt
        + Scalar
        + Mul
        + MulAssign
        + PrimitiveSimdValue,
    KernelType: PrimInt + SupersetOf<InputType>,
    // OutputType: Into<KernelType>,
{
    let radius = (2.0 * sigma).floor() as usize;

    match radius {
        1 => {
            let (kernel, factor) = gaussian_2d_integer_kernel_2d::<5>(sigma);
            let mut dst = DMatrix::<OutputType>::zeros(image.nrows(), image.ncols());
            direct_convolution_mut::<5, InputType, KernelType, i16>(
                image,
                dst.as_mut_slice(),
                &kernel,
                factor,
            );
            dst
        }
        2 => {
            let (kernel, factor) = gaussian_2d_integer_kernel_2d::<7>(sigma);
            // direct_convolution::<5, InputType, KernelType, OutputType>(image, &kernel, factor)
            let mut dst = DMatrix::<OutputType>::zeros(image.nrows(), image.ncols());
            direct_convolution_mut::<7, InputType, KernelType, i16>(
                image,
                dst.as_mut_slice(),
                &kernel,
                factor,
            );
            dst
        }
        _ => {
            let (kernel, factor) = gaussian_2d_integer_kernel_2d::<11>(sigma);
            // direct_convolution::<7, InputType, KernelType, OutputType>(image, &kernel, factor)
            let mut dst = DMatrix::<OutputType>::zeros(image.nrows(), image.ncols());
            direct_convolution_mut::<11, InputType, KernelType, i16>(
                image,
                dst.as_mut_slice(),
                &kernel,
                factor,
            );
            dst
        } // _ => {
          //     let (kernel, factor) = gaussian_2d_integer_kernel::<3>(sigma);
          //     // direct_convolution::<7, InputType, KernelType, OutputType>(image, &kernel, factor)
          //     let mut dst = DMatrix::<OutputType>::zeros(image.nrows(), image.ncols());
          //     direct_convolution_mut(image, dst.as_mut_slice(), &kernel, factor);
          //     dst
          // }
    }
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
    KernelType: SupersetOf<InputType>,
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
    KernelType: PrimInt + SupersetOf<InputType>,
    OutputType: Into<KernelType>,
{
    // let scale_value = (KSIZE as OutputType).pow(2);

    let kernel = SMatrix::<KernelType, KSIZE, KSIZE>::repeat(1);
    let mut first = direct_convolution::<KSIZE, InputType, KernelType, OutputType>(
        transposed_image,
        &kernel,
        // Some(scale_value as KernelType),
        NonZeroU32::new(1).unwrap(),
    );

    for _ in 1..passes {
        first = direct_convolution::<KSIZE, OutputType, KernelType, OutputType>(
            &first,
            &kernel, // Some(scale_value as KernelType),
            NonZeroU32::new(1).unwrap(),
        );
    }
    first
}

#[cfg(test)]
mod tests {
    use crate::grayimage_to_2d_transposed_matrix_view;

    use super::*;
    use image::open;
    use imageproc::filter::gaussian_blur_f32;

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
    fn test_gaussian_kernel_gen() {
        const SIGMA: &[f32] = &[1.0, 1.4, 2.0, 3.5, 4.5];
        const TOLERANCE: f64 = 0.02;

        // For input types, will the conv. cause overflow with i32 kernels.
        const MAX_SUMMED_VALUES: &[usize] =
            &[u8::MAX as usize, i8::MAX as usize, i16::MAX as usize];
        const MAX_KERNEL_TYPE: usize = i32::MAX as usize;

        SIGMA.iter().for_each(|&sigma| {
            let (kernel, factor) = gaussian_2d_integer_kernel_2d::<3>(sigma);
            let scaled_sum = kernel.sum() as f64 / factor.get() as f64;

            assert!(
                (1.0 - scaled_sum).abs() < TOLERANCE,
                "Sigma:{} factor:{}, scaled_sum (ideally 1.0): {}  kernel:{}",
                sigma,
                factor,
                scaled_sum,
                kernel
            );

            for &max_value in MAX_SUMMED_VALUES {
                let max_convolved_patch = kernel.map(|v| v as usize) * max_value;
                let sum = max_convolved_patch.sum();

                assert!(
                    sum <= MAX_KERNEL_TYPE,
                    "Convolution in kernel type causes overflow (before division): {sum} should be < {MAX_KERNEL_TYPE}"
                );

                let max_scaled_sum=sum/factor.get() as usize;
                assert!(
                    max_scaled_sum<=max_value,
                    "Causes overflow for output type (2^{}): max_scaled_sum:{} max_value:{}",
                    max_value.trailing_zeros(),
                    max_scaled_sum,
                    max_value
                )
            }
        });

        SIGMA.iter().for_each(|&sigma| {
            let (kernel, factor) = gaussian_2d_integer_kernel_2d::<5>(sigma);
            let scaled_sum = kernel.sum() as f64 / factor.get() as f64;

            assert!(
                (1.0 - scaled_sum).abs() < TOLERANCE,
                "Sigma:{} factor:{}, scaled_sum (ideally 1.0): {}  kernel:{}",
                sigma,
                factor,
                scaled_sum,
                kernel
            );


            for &max_value in MAX_SUMMED_VALUES {
                let max_convolved_patch = kernel.map(|v| v as usize) * max_value;
                let sum = max_convolved_patch.sum();

                assert!(
                    sum <= MAX_KERNEL_TYPE,
                    "Convolution in kernel type causes overflow (before division): {sum} should be < {MAX_KERNEL_TYPE}"
                );

                let max_scaled_sum=sum/factor.get() as usize;
                assert!(
                    max_scaled_sum<=max_value,
                    "Causes overflow for output type (2^{}): max_scaled_sum:{} max_value:{}",
                    max_value.trailing_zeros(),
                    max_scaled_sum,
                    max_value
                )
            }
        });

        SIGMA.iter().for_each(|&sigma| {
            let (kernel, factor) = gaussian_2d_integer_kernel_2d::<7>(sigma);
            let scaled_sum = kernel.sum() as f64 / factor.get() as f64;

            assert!(
                (1.0 - scaled_sum).abs() < TOLERANCE,
                "Sigma:{} factor:{}, scaled_sum (ideally 1.0): {}  kernel:{}",
                sigma,
                factor,
                scaled_sum,
                kernel
            );

            for &max_value in MAX_SUMMED_VALUES {
                let max_convolved_patch = kernel.map(|v| v as usize) * max_value;
                let sum = max_convolved_patch.sum();

                assert!(
                    sum <= MAX_KERNEL_TYPE,
                    "Convolution in kernel type causes overflow (before division): {sum} should be < {MAX_KERNEL_TYPE}"
                );

                let max_scaled_sum=sum/factor.get() as usize;
                assert!(
                    max_scaled_sum<=max_value,
                    "Causes overflow for output type (2^{}): max_scaled_sum:{} max_value:{}",
                    max_value.trailing_zeros(),
                    max_scaled_sum,
                    max_value
                )
            }
        });
    }

    #[test]
    fn test_gaussian_box_filter_nalgebra() {
        let sigma = 3.5;
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        let image = open(format!("{crate_dir}/test_data/center_circle_webots.png"))
            .expect("The image should be in this path");

        let luma8 = image.to_luma8();
        let converted = grayimage_to_2d_transposed_matrix_view(&luma8);
        let blurred = gaussian_blur_box_filter_nalgebra::<u8>(&converted, sigma);

        let blurred_int_approximation = gaussian_blur_try_2_nalgebra(&converted, sigma);
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

        gaussian_blur_f32(&luma8, sigma)
            .save(format!(
                "{crate_dir}/test_data/output/gaussian_imgproc_expected.png"
            ))
            .expect("The image saving should not fail");
    }
}
