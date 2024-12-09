use std::ops::{Div, Mul, MulAssign};

use image::{GrayImage, ImageBuffer, Luma};
use imageproc::filter::box_filter;
use nalgebra::{DMatrix, SMatrix, Scalar, SimdPartialOrd};

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

/// Gaussian smoothing approximation with box filters
/// - https://en.wikipedia.org/wiki/Gaussian_blur
/// - Kovesi, Peter. "Fast almost-gaussian filtering."
///     2010 International Conference on Digital Image Computing: Techniques and Applications. IEEE, 2010.
pub fn gaussian_blur_box_filter_nalgebra<T>(
    transposed_image: &DMatrix<T>,
    sigma: f32,
) -> DMatrix<i16>
where
    T: Copy + Mul + MulAssign + Scalar + SimdPartialOrd,
    i16: From<T>,
{
    // average sigma = sqrt( (w**2 -1) / 12 ): w is box width, n is passes

    const PASSES: usize = 6;
    // box_filter_direct_convolve::<3, T>(transposed_image, PASSES)

    let w_ideal_half = ((12.0 * sigma.div(2.0).powi(2) / (PASSES as f32)) + 1.0)
        .sqrt()
        .div(2.0)
        .round() as u32
        - 1;

    match w_ideal_half {
        0 => box_filter_direct_convolve::<3, T>(transposed_image, PASSES),
        1 => box_filter_direct_convolve::<5, T>(transposed_image, PASSES),
        2 => box_filter_direct_convolve::<7, T>(transposed_image, PASSES),
        _ => unreachable!("Box filter width must be between 3 and 11"),
    }
}

#[inline(always)]
fn box_filter_direct_convolve<const K: usize, T>(
    transposed_image: &DMatrix<T>,
    passes: usize,
) -> DMatrix<i16>
where
    T: Copy + Mul + MulAssign + Scalar + SimdPartialOrd,
    i16: From<T>,
{
    // let mut output = DMatrix::zeros(transposed_image.nrows(), transposed_image.ncols());

    // transposed_image.data.

    // // for i in 0..output.ncols() {
    // // output
    // //     .column_iter_mut()
    // //     .zip(transposed_image.column_iter())
    // //     .for_each(|(mut output_col, input_col)| {
    // //         let col_len = output_col.len() - 2;
    // //         // let output_slice = &output_col.as_mut_slice()[1..col_len - 1];

    // //         let input_slice = &input_col.as_slice()[1..col_len - 1];

    // //         for index in 1..col_len - 1 {
    // //             let sum: i32 = input_slice[index - 1].into()
    // //                 + input_slice[index].into()
    // //                 + input_slice[index + 1].into();
    // //             output_col[index] = (sum / 3) as i16;
    // //         }
    // //     });

    // // output

    let kernel = SMatrix::<i16, K, K>::repeat(1);
    let mut first = direct_convolution::<K, T>(transposed_image, &kernel, Some(0));
    for _ in 1..passes {
        first = direct_convolution::<K, i16>(&first, &kernel, Some(0));
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
        let blurred = gaussian_blur_box_filter_nalgebra::<u8>(&converted, 1.0);

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
    }
}
