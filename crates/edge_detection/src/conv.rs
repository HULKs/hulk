use std::ops::{Mul, MulAssign};

use nalgebra::{self as na, Scalar, SimdPartialOrd};

pub fn direct_convolution<const K: usize, T>(
    image: &na::DMatrix<T>,
    kernel: &na::SMatrix<i16, K, K>,
    _scale_value: Option<i16>,
) -> na::DMatrix<i16>
where
    T: Clone + Copy + Scalar + Mul + MulAssign + SimdPartialOrd,
    i16: From<T>,
{
    let (image_rows, image_cols) = image.shape();
    let mut result: nalgebra::Matrix<
        i16,
        nalgebra::Dyn,
        nalgebra::Dyn,
        nalgebra::VecStorage<i16, nalgebra::Dyn, nalgebra::Dyn>,
    > = na::DMatrix::zeros(image_rows, image_cols);
    direct_convolution_mut(image, &mut result, kernel);
    result
}

pub fn direct_convolution_mut<const K: usize, T>(
    transposed_image: &na::DMatrix<T>,
    dst: &mut na::DMatrix<i16>,
    kernel: &na::SMatrix<i16, K, K>,
    // scale_value: Option<i16>,
) where
    T: Clone + Copy + Scalar + Mul + MulAssign + SimdPartialOrd,
    i16: From<T>,
{
    if dst.shape().0 < transposed_image.shape().0 || dst.shape().1 < transposed_image.shape().1 {
        panic!(
            "dst matrix must have the same or larger as input image{:?} {:?}",
            transposed_image.shape(),
            dst.shape()
        );
    }

    let (image_rows, image_cols) = transposed_image.shape();

    let kernel_half = K / 2;

    // let calculated_scale_value = scale_value.unwrap_or_default() as i32;

    const MAX_ALLOWED_SUM: i32 = i16::MAX as i32;

    // Nalgebra works on column-major order, therefore the loops are transposed.
    for j in kernel_half..image_cols - kernel_half {
        let j_top_left = j - kernel_half;
        for i in kernel_half..image_rows - kernel_half {
            let i_top_left = i - kernel_half;
            let mut sum: i32 = 0;
            // For the kernel, seems the order didn't really matter (based on benchmarking)
            for ki in 0..kernel_half {
                for kj in 0..kernel_half {
                    let ii = ki + i_top_left;
                    let jj = kj + j_top_left;
                    // TODO find a better way to do this!!!
                    let image_px: i16 = transposed_image[(ii, jj)].into();
                    sum += image_px as i32 * kernel[(ki, kj)] as i32;
                }
            }
            dst[(i, j)] = sum.min(MAX_ALLOWED_SUM) as i16;
        }
    }
}

pub fn imgproc_kernel_to_matrix<const K: usize>(kernel: &[i32]) -> na::SMatrix<i16, K, K> {
    na::SMatrix::<i16, K, K>::from_iterator(kernel.iter().map(|&x| x as i16))
}
