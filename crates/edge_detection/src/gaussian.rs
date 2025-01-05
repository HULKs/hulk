use core::f32;
use num_traits::{AsPrimitive, PrimInt};
use std::{
    iter::Sum,
    num::NonZeroU32,
    ops::{AddAssign, Div, MulAssign},
};

use image::{GrayImage, ImageBuffer, Luma};
use imageproc::filter::box_filter;
use nalgebra::{DMatrix, DMatrixView, Scalar};

use crate::conv::piecewise_2d_convolution_mut;

type IntKernelType = i32;
pub fn gaussian_blur_integer_approximation<InputType, OutputType>(
    image: DMatrixView<InputType>,
    sigma: f32,
) -> DMatrix<OutputType>
where
    InputType: PrimInt + AsPrimitive<IntKernelType> + AsPrimitive<i16> + Scalar,
    IntKernelType: PrimInt + AsPrimitive<OutputType> + Scalar + AddAssign + MulAssign + Sum,
    OutputType: PrimInt + AsPrimitive<IntKernelType> + AsPrimitive<i16> + Scalar + AddAssign,
    f32: AsPrimitive<IntKernelType>,
    i16: AsPrimitive<OutputType>,
{
    // Check if this method is accurate.
    let radius = (2.0 * sigma).floor() as usize;
    let mut dst = DMatrix::<OutputType>::zeros(image.nrows(), image.ncols());

    if sigma <= 1.0 {
        let (kernel, factor) = gaussian_2d_seperable_integer_kernel::<3, IntKernelType>(sigma);
        piecewise_2d_convolution_mut(image, dst.as_mut_slice(), &kernel, &kernel, factor);
        dst
    } else if (1..4).contains(&radius) {
        let (kernel, factor) = gaussian_2d_seperable_integer_kernel::<5, IntKernelType>(sigma);
        piecewise_2d_convolution_mut(image, dst.as_mut_slice(), &kernel, &kernel, factor);
        dst
    } else if (4..6).contains(&radius) {
        let (kernel, factor) = gaussian_2d_seperable_integer_kernel::<7, IntKernelType>(sigma);
        piecewise_2d_convolution_mut(image, dst.as_mut_slice(), &kernel, &kernel, factor);
        dst
    } else {
        let (kernel, factor) = gaussian_2d_seperable_integer_kernel::<11, IntKernelType>(sigma);
        piecewise_2d_convolution_mut(image, dst.as_mut_slice(), &kernel, &kernel, factor);
        dst
    }
}

#[inline(always)]
fn gaussian(x: f32, r: f32) -> f32 {
    ((2.0 * f32::consts::PI).sqrt() * r).recip() * (-x.powi(2) / (2.0 * r.powi(2))).exp()
}

#[inline(always)]
fn gaussian_int_divisor_as_power_of_two(width: usize) -> u32 {
    // These values are choosen by making kernels for these sizes and gettin large enough divisor.
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
fn gaussian_2d_seperable_integer_kernel<const S: usize, KType>(
    sigma: f32,
) -> ([KType; S], NonZeroU32)
where
    f32: AsPrimitive<KType>,
    KType: PrimInt + Copy + Scalar,
{
    let mut one_axis = [0f32; S];
    let k_half = S / 2;
    check_kernel_size(S);

    for i in 0..((S / 2) + 1) {
        let v = gaussian(i as f32, sigma);
        one_axis[k_half - i] = v;
        one_axis[k_half + i] = v;
    }

    let int_factor_power_of_two = if sigma > 1.0 {
        gaussian_int_divisor_as_power_of_two(S)
        // 10
    } else {
        16
    };

    let one_axis_sum: f32 = one_axis.iter().sum();
    (
        one_axis.map(|v| (v / one_axis_sum * 2.pow(int_factor_power_of_two) as f32).as_()),
        NonZeroU32::new(2.pow(int_factor_power_of_two)).unwrap(),
    )
}

/// Box filter
/// Gaussian smoothing approximation with box filters
/// - https://en.wikipedia.org/wiki/Gaussian_blur
/// - Kovesi, Peter. "Fast almost-gaussian filtering."
///     2010 International Conference on Digital Image Computing: Techniques and Applications. IEEE, 2010.
pub fn gaussian_blur_box_filter(image: &GrayImage, sigma: f32) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    // TODO Implement the paper completely

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

// TODO FIX ME
// pub fn gaussian_blur_box_filter_nalgebra<InputType, OutputType, KernelType>(
//     transposed_image: DMatrixView<InputType>,
//     sigma: f32,
// ) -> DMatrix<OutputType>
// where
//     InputType: AsPrimitive<KernelType> + PrimInt + Scalar + Mul + MulAssign,
//     KernelType: PrimInt + AsPrimitive<OutputType> + Scalar + MulAssign + AddAssign + Sum,
//     OutputType: PrimInt + AsPrimitive<KernelType> + Scalar + AddAssign,
// {
//     // TODO Implement the paper completely and match with gaussian_blur_box_filter

//     const PASSES: usize = 2;
//     let w_ideal_half = ((12.0 * sigma.div(2.0).powi(2) / (PASSES as f32)) + 1.0)
//         .sqrt()
//         .div(2.0)
//         .round() as u32
//         - 1;

//     match w_ideal_half {
//         0 => box_filter_direct_convolve::<3, InputType, OutputType, KernelType>(
//             transposed_image,
//             PASSES,
//         ),
//         1 => box_filter_direct_convolve::<5, InputType, OutputType, KernelType>(
//             transposed_image,
//             PASSES,
//         ),
//         2 => box_filter_direct_convolve::<7, InputType, OutputType, KernelType>(
//             transposed_image,
//             PASSES,
//         ),
//         _ => unreachable!("Box filter width must be between 3 and 11"),
//     }
// // }

// #[inline(always)]
// fn box_filter_direct_convolve<const KSIZE: usize, InputType, OutputType, KernelType>(
//     transposed_image: DMatrixView<InputType>,
//     passes: usize,
// ) -> DMatrix<OutputType>
// where
//     InputType: AsPrimitive<KernelType> + PrimInt + Scalar + Mul + MulAssign,
//     KernelType: PrimInt + AsPrimitive<OutputType> + Scalar + AddAssign + MulAssign + Sum,
//     OutputType: PrimInt + AsPrimitive<KernelType> + Scalar + AddAssign,
// {
//     let kernel = SMatrix::<KernelType, KSIZE, KSIZE>::repeat(KernelType::one());
//     let mut first = direct_convolution::<KSIZE, InputType, KernelType, OutputType>(
//         transposed_image,
//         &kernel,
//         NonZeroU32::new(1).unwrap(),
//     );

//     for _ in 1..passes {
//         first = direct_convolution::<KSIZE, OutputType, KernelType, OutputType>(
//             first.as_view(),
//             &kernel, // Some(scale_value as KernelType),
//             NonZeroU32::new(1).unwrap(),
//         );
//     }
//     first
// }

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
        const SIGMA: &[f32] = &[0.5, 1.0, 1.4, 2.0, 3.5, 4.5];
        const TOLERANCE: f64 = 0.01;

        // For input types, will the conv. cause overflow with i32 kernels.
        const MAX_OUTPUT_VALUES: &[usize] =
            &[u8::MAX as usize, i8::MAX as usize, i16::MAX as usize];
        const MAX_KERNEL_TYPE: usize = i32::MAX as usize;

        SIGMA.iter().for_each(|&sigma| {
            let (kernel, factor) = gaussian_2d_seperable_integer_kernel::<3,_>(sigma);
            let scaled_sum = kernel.iter().sum::<i32>() as f64 / factor.get() as f64;

            assert!(
                (1.0 - scaled_sum).abs() < TOLERANCE,
                "Sigma:{} factor:{}, scaled_sum (ideally 1.0): {}  kernel:{:?}",
                sigma,
                factor,
                scaled_sum,
                kernel
            );

            for &max_value in MAX_OUTPUT_VALUES {
                let max_convolved_patch = kernel.map(|v| v as usize * max_value);
                let sum:usize = max_convolved_patch.iter().sum();

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
            let (kernel, factor) = gaussian_2d_seperable_integer_kernel::<5,_>(sigma);
            let scaled_sum =  kernel.iter().sum::<i32>()as f64 / factor.get() as f64;

            assert!(
                (1.0 - scaled_sum).abs() < TOLERANCE,
                "Sigma:{} factor:{}, scaled_sum (ideally 1.0): {}  kernel:{:?}",
                sigma,
                factor,
                scaled_sum,
                kernel
            );


            for &max_value in MAX_OUTPUT_VALUES {
                let max_convolved_patch = kernel.map(|v| v as usize* max_value) ;
                let sum:usize = max_convolved_patch.iter().sum();
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
            let (kernel, factor) = gaussian_2d_seperable_integer_kernel::<7,_>(sigma);
            let scaled_sum = kernel.iter().sum::<i32>() as f64 / factor.get() as f64;

            assert!(
                (1.0 - scaled_sum).abs() < TOLERANCE,
                "Sigma:{} factor:{}, scaled_sum (ideally 1.0): {}  kernel:{:?}",
                sigma,
                factor,
                scaled_sum,
                kernel
            );

            for &max_value in MAX_OUTPUT_VALUES {
                let max_convolved_patch = kernel.map(|v| v as usize* max_value) ;
                let sum:usize = max_convolved_patch.iter().sum();
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
    fn test_gaussian_seperable_kernel() {
        let sigma = 3.5;
        let crate_dir = env!("CARGO_MANIFEST_DIR");
        let image = open(format!("{crate_dir}/test_data/center_circle_webots.png"))
            .expect("The image should be in this path");

        let luma8 = image.to_luma8();
        let converted = grayimage_to_2d_transposed_matrix_view(&luma8);
        let converted_view = converted.as_view();

        // TODO: Fix this test
        // let blurred = gaussian_blur_box_filter_nalgebra::<u8, i16, i32>(converted_view, sigma);
        // GrayImage::from_raw(
        //     image.width(),
        //     image.height(),
        //     blurred.iter().map(|&v: &i16| v as u8).collect::<Vec<u8>>(),
        // )
        // .unwrap()
        // .save(format!(
        //     "{crate_dir}/test_data/output/gaussian_box_filter_nalgebra.png"
        // ))
        // .expect("The image saving should not fail");

        let blurred_int_approximation =
            gaussian_blur_integer_approximation::<u8, u8>(converted_view, sigma);
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
