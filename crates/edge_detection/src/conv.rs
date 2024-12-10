use num_traits::{AsPrimitive, Bounded, PrimInt, Signed};
use std::{
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Mul, MulAssign},
};

use nalgebra::{ClosedMul, DMatrix, SMatrix, Scalar};

pub fn direct_convolution<const KSIZE: usize, P, KType, S>(
    image: &DMatrix<P>,
    kernel: &SMatrix<KType, KSIZE, KSIZE>,
) -> DMatrix<S>
where
    P: Into<KType> + PrimInt + Scalar + Mul + MulAssign + Add,
    KType: PrimInt + Scalar + AddAssign + AsPrimitive<S> + ClosedMul + Signed + Display,
    S: Into<KType>
        + TryFrom<KType>
        + AsPrimitive<KType>
        + PrimInt
        + Scalar
        + Debug
        + Bounded
        + AddAssign,
{
    let (image_rows, image_cols) = image.shape();
    let mut result = DMatrix::<S>::zeros(image_rows, image_cols);
    direct_convolution_mut_try_again(image, &mut result, kernel);
    result
}

pub fn direct_convolution_mut<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: &DMatrix<InputType>,
    dst: &mut DMatrix<OutputType>,
    kernel: &SMatrix<KType, KSIZE, KSIZE>,
    // scale_value: Option<i16>,
) where
    InputType: Into<KType> + PrimInt + Mul + MulAssign + Scalar,
    KType: PrimInt + Scalar + AddAssign + AsPrimitive<OutputType> + Signed + Display,
    OutputType: Into<KType> + AsPrimitive<KType> + PrimInt + Debug + Bounded,
{
    assert!(
        dst.shape().0 >= transposed_image.shape().0 && dst.shape().1 >= transposed_image.shape().1,
        "dst matrix ({:?}) must have same or larger size than input: {:?}",
        dst.shape(),
        transposed_image.shape(),
    );

    let (image_rows, image_cols) = transposed_image.shape();
    let kernel_half = KSIZE / 2;

    let max_allowed_sum: KType = OutputType::max_value().into();
    let min_allowed_sum: KType = OutputType::min_value().into();

    // if the highest possible output by multiplying the kernel with the highest possible input is lower than the max value of the output type, we can skip clamping
    // TODO check if we can miss by one
    // let max_sum = kernel.iter().fold(KType::zero(), |accum, cv| {
    //     accum + cv.abs() * InputType::max_value().into()
    // });
    // let skip_clamping = max_sum < OutputType::max_value().into();
    // println!(
    //     "skip clamping: {skip_clamping}, {:?}, {:?} \n{}",
    //     max_sum,
    //     OutputType::max_value(),
    //     kernel
    // );

    // let input_mat_copy = transposed_image.map(|v| v.into());

    // Nalgebra works on column-major order, therefore the loops are transposed.
    for j in kernel_half..image_cols - kernel_half {
        for i in kernel_half..image_rows - kernel_half {
            let j_top_left = j - kernel_half;
            let i_top_left = i - kernel_half;
            let mut sum = KType::zero();
            // For the kernel, seems the order didn't really matter (based on benchmarking)
            for kj in 0..KSIZE {
                for ki in 0..KSIZE {
                    let ii = ki + i_top_left;
                    let jj = kj + j_top_left;
                    sum += transposed_image[(ii, jj)].into() * kernel[(ki, kj)];
                }
            }

            // dst[(i, j)] = if skip_clamping {
            //     sum.as_()
            // } else {
            //     sum.clamp(min_allowed_sum, max_allowed_sum).as_()
            // };

            dst[(i, j)] = sum.clamp(min_allowed_sum, max_allowed_sum).as_()
        }
    }
}

pub fn direct_convolution_mut_try_again<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: &DMatrix<InputType>,
    dst: &mut DMatrix<OutputType>,
    kernel: &SMatrix<KType, KSIZE, KSIZE>,
    // scale_value: Option<i16>,
) where
    InputType: Into<KType> + PrimInt + Mul + MulAssign + Scalar,
    KType: PrimInt + AddAssign + AsPrimitive<OutputType> + Scalar + ClosedMul,
    OutputType: Into<KType> + AsPrimitive<KType> + PrimInt + Debug + Bounded + AddAssign,
{
    assert!(
        dst.shape().0 >= transposed_image.shape().0 && dst.shape().1 >= transposed_image.shape().1,
        "dst matrix ({:?}) must have same or larger size than input: {:?}",
        dst.shape(),
        transposed_image.shape(),
    );

    let nrows = transposed_image.nrows();
    let ncols = transposed_image.ncols();
    let ksize_floor_half = KSIZE / 2;

    let max_allowed_sum: KType = OutputType::max_value().into();
    let min_allowed_sum: KType = OutputType::min_value().into();

    let input_mat_copy = transposed_image.map(|v| v.into());
    let dst_as_slice = dst.as_mut_slice();

    for col_index in ksize_floor_half..ncols - ksize_floor_half {
        let middle_offset = (col_index) * nrows;

        for row_index in ksize_floor_half..nrows - ksize_floor_half {
            let input_patch = input_mat_copy.fixed_view::<KSIZE, KSIZE>(
                row_index - ksize_floor_half,
                col_index - ksize_floor_half,
            );
            let sum = input_patch.component_mul(kernel).sum();
            dst_as_slice[middle_offset + row_index] =
                sum.clamp(min_allowed_sum, max_allowed_sum).as_();
        }

        // let input_patch = input_mat_copy.view((0, col_index - ksize_floor_half), (nrows, KSIZE));
        // let mut col = dst.column_mut(col_index);
        // for i in 0..KSIZE {
        //     col.add_assign(
        //         input_patch
        //             .mul(&kernel.column(i))
        //             .column_sum()
        //             .map(|v| v.clamp(min_allowed_sum, max_allowed_sum).as_()),
        //     );
        // }
        // dst_as_slice[middle_offset] = sum.clamp(min_allowed_sum, max_allowed_sum).as_();
    }
}

pub fn imgproc_kernel_to_matrix<const K: usize>(kernel: &[i32]) -> SMatrix<i32, K, K> {
    // na::SMatrix::<i32, K, K>::from_iterator(kernel.iter().map(|&x| x as i32))
    SMatrix::<i32, K, K>::from_iterator(kernel.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use imageproc::gradients::HORIZONTAL_SOBEL;
    use nalgebra::DMatrix;

    #[test]
    fn test_direct_convolution() {
        let nrows = 10;
        let ncols = 5;

        let mut image = DMatrix::<i16>::zeros(nrows, ncols);
        image.view_mut((0, 0), (3, 5)).fill(255);
        image.view_mut((6, 0), (4, 5)).fill(255);

        // Since these operations assume the matrix is transposed, the kernel also has to be swapped
        let kernel = imgproc_kernel_to_matrix(&HORIZONTAL_SOBEL);

        let result = direct_convolution::<3, i16, i32, i16>(&image, &kernel);

        // taken via OpenCV
        let expected_full_result = DMatrix::<i16>::from_row_slice(
            nrows,
            ncols,
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1020, -1020, -1020, -1020, -1020, -1020, -1020,
                -1020, -1020, -1020, 0, 0, 0, 0, 0, 1020, 1020, 1020, 1020, 1020, 1020, 1020, 1020,
                1020, 1020, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
        );

        let result_subview = result.view((1, 1), (nrows - 2, ncols - 2)).clone_owned();
        let expected_subview = expected_full_result
            .view((1, 1), (nrows - 2, ncols - 2))
            .clone_owned();
        // assert!(false, "{:?}\n{:?}", image, result);
        assert_eq!(
            result_subview, expected_subview,
            "The sub-views of the results should match! {} {}",
            result_subview, expected_subview
        );

        let mut fast_result = DMatrix::<i16>::zeros(nrows, ncols);

        direct_convolution_mut::<3, i16, i32, i16>(&image, &mut fast_result, &kernel);
        let fast_result_subview = fast_result
            .view((1, 1), (nrows - 2, ncols - 2))
            .clone_owned();
        assert_eq!(
            fast_result_subview, expected_subview,
            "The faster version should match! {} {}",
            fast_result, expected_full_result
        );
    }
}
