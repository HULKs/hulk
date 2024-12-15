use num_traits::{AsPrimitive, Bounded, PrimInt, Signed};
use std::{
    fmt::{Debug, Display},
    num::NonZeroU32,
    ops::{Add, AddAssign, DivAssign, Mul, MulAssign},
};

use nalgebra::{ClosedMul, DMatrix, SMatrix, Scalar};

pub fn direct_convolution<const KSIZE: usize, P, KType, S>(
    image: &DMatrix<P>,
    kernel: &SMatrix<KType, KSIZE, KSIZE>,
    scale_value: NonZeroU32,
) -> DMatrix<S>
where
    P: Into<KType> + PrimInt + AsPrimitive<KType> + Scalar + Mul + MulAssign + Add,
    KType: PrimInt + Scalar + AddAssign + AsPrimitive<S> + ClosedMul + Signed + Display,
    S: Into<KType>
        + TryFrom<KType>
        + AsPrimitive<KType>
        + PrimInt
        + Scalar
        + Debug
        + Bounded
        + AddAssign
        + DivAssign,
    u32: AsPrimitive<KType>,
{
    let (image_rows, image_cols) = image.shape();

    let mut result = DMatrix::<S>::zeros(image_rows, image_cols);

    direct_convolution_mut_try_again(image, result.as_mut_slice(), kernel, scale_value);
    result
}

pub fn direct_convolution_mut<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: &DMatrix<InputType>,
    dst: &mut [OutputType],
    kernel: &SMatrix<KType, KSIZE, KSIZE>,
    scale_value: NonZeroU32,
) where
    InputType: Into<KType> + AsPrimitive<KType> + PrimInt + Mul + MulAssign + Scalar,
    KType: PrimInt
        + Scalar
        + AddAssign
        + AsPrimitive<OutputType>
        + Signed
        + Display
        + DivAssign
        + MulAssign,
    OutputType: Into<KType> + AsPrimitive<KType> + PrimInt + Debug + Bounded + DivAssign,
    u32: AsPrimitive<KType>,
{
    assert!(
        dst.len() >= transposed_image.len(),
        "dst matrix ({:?}) must have same or larger size than input: {:?}",
        dst.len(),
        transposed_image.shape(),
    );

    let (image_rows, image_cols) = transposed_image.shape();
    let kernel_half = KSIZE / 2;

    let max_allowed_sum: KType = OutputType::max_value().into();
    let min_allowed_sum: KType = OutputType::min_value().into();

    let transposed_image_slice = transposed_image.data.as_slice();

    // scale_value.checked_next_power_of_two()
    let divisor: KType = scale_value.get().as_();
    let should_divide_or_shift = divisor > KType::one();
    let bit_shift_amount = if should_divide_or_shift {
        scale_value
            .checked_next_power_of_two()
            .unwrap()
            .trailing_zeros() as usize
    } else {
        0
    };

    // let k_arr: [KType; KSIZE] = kernel.data.0;

    // Nalgebra works on column-major order, therefore the loops are transposed.
    for j in kernel_half..image_cols - kernel_half {
        let j_top_left = j - kernel_half;
        for i in kernel_half..image_rows - kernel_half {
            let i_top_left = i - kernel_half;
            let mut sum = KType::zero();

            for kj in 0..KSIZE {
                // let jj = kj + j_top_left;
                let ko = kj * KSIZE;
                let offset = (kj + j_top_left) * image_rows;
                for ki in 0..KSIZE {
                    let ii = ki + i_top_left;
                    sum += transposed_image_slice[ii + offset].as_() * kernel[ki + ko];
                    // sum += unsafe { *image_ptr.add(ii + offset) }.as_() * kernel[ki + ko];
                }
            }

            dst[j * image_rows + i] = (sum >> bit_shift_amount)
                .clamp(min_allowed_sum, max_allowed_sum)
                .as_()
        }
    }
}

pub fn direct_convolution_mut_try_again<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: &DMatrix<InputType>,
    // dst: &mut DMatrix<OutputType>,
    dst_as_slice: &mut [OutputType],
    kernel: &SMatrix<KType, KSIZE, KSIZE>,
    scale_value: NonZeroU32,
) where
    InputType: AsPrimitive<KType> + PrimInt + Mul + MulAssign + Scalar,
    KType: PrimInt + AddAssign + AsPrimitive<OutputType> + Scalar + ClosedMul,
    OutputType: AsPrimitive<KType> + PrimInt + Debug + Bounded + AddAssign + DivAssign,
    u32: AsPrimitive<KType>,
{
    assert!(
        dst_as_slice.len() >= transposed_image.len(),
        "dst matrix ({:?}) must have same or larger size than input: {:?}",
        dst_as_slice.len(),
        transposed_image.len(),
    );

    let nrows = transposed_image.nrows();
    let ncols = transposed_image.ncols();
    let ksize_floor_half = KSIZE / 2;

    let max_allowed_sum: KType = OutputType::max_value().as_();
    let min_allowed_sum: KType = OutputType::min_value().as_();

    let input_mat_copy = transposed_image.map(|v| v.as_());

    // scale_value.checked_next_power_of_two()
    let divisor: KType = scale_value.get().as_();
    let should_divide_or_shift = divisor > KType::one();

    let bit_shift_amount = if should_divide_or_shift {
        scale_value
            .checked_next_power_of_two()
            .unwrap()
            .trailing_zeros() as usize
    } else {
        0
    };

    for col_index in ksize_floor_half..ncols - ksize_floor_half {
        let middle_offset = (col_index) * nrows;
        let left_top = col_index - ksize_floor_half;
        for row_index in ksize_floor_half..nrows - ksize_floor_half {
            let sum = kernel
                .component_mul(
                    &input_mat_copy
                        .fixed_view::<KSIZE, KSIZE>(row_index - ksize_floor_half, left_top),
                )
                .sum()
                .shr(bit_shift_amount);

            dst_as_slice[middle_offset + row_index] = (sum >> bit_shift_amount)
                .clamp(min_allowed_sum, max_allowed_sum)
                .as_();
        }
    }
    // if divide {
    //     dst_as_slice.iter_mut().for_each(|v| *v /= divisor.as_());
    // }
}

pub fn piecewise_horizontal_convolution_mut<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: &DMatrix<InputType>,
    dst: &mut [OutputType],
    piecewise_kernel: &[KType; KSIZE],
) where
    InputType: AsPrimitive<KType> + PrimInt + Mul + MulAssign + Scalar + Display,
    KType: PrimInt + AddAssign + AsPrimitive<OutputType> + Scalar + ClosedMul,
    OutputType: AsPrimitive<KType> + PrimInt + Debug + Bounded + AddAssign,
{
    let kernel_half = KSIZE / 2;
    let max_allowed_sum: KType = OutputType::max_value().as_();
    let min_allowed_sum: KType = OutputType::min_value().as_();

    let ncols = transposed_image.ncols();
    let nrows = transposed_image.nrows();

    // let mut dst_mat =
    // DMatrixViewMut::from_slice(dst, transposed_image.nrows(), transposed_image.ncols());
    for j in 0..ncols {
        let col = &transposed_image.column(j);
        // print!("for col \n{}\n sums: ", col.transpose());
        for i in kernel_half..nrows - kernel_half {
            let mut sum = KType::zero();
            for ki in 0..KSIZE {
                sum += col[i - kernel_half + ki].as_() * piecewise_kernel[ki];
                // sum += transposed_image[(i - kernel_half + ki, j)].as_() * piecewise_kernel[ki];

                debug_assert_eq!(
                    transposed_image.vector_to_matrix_index(j * nrows +i - kernel_half + ki),
                    (i - kernel_half + ki, j),
                    "Source column index calculations for row:{} and col:{} are not matching the produced flat index:{}",
                    j,
                    i,
                    i - kernel_half + ki
                );
            }

            debug_assert_eq!(
                transposed_image.vector_to_matrix_index(j * nrows + i),
                (i, j),
                "Destination index calculations for row:{} and col:{} are not matching the produced flat index:{}",
                j,
                i,
                j * nrows + i
            );
            dst[j * nrows + i] = sum.clamp(min_allowed_sum, max_allowed_sum).as_();
        }
    }
}

#[inline]
pub fn piecewise_vertical_convolution_mut<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: &DMatrix<InputType>,
    dst: &mut [OutputType],
    piecewise_kernel: &[KType; KSIZE],
) where
    InputType: AsPrimitive<KType> + PrimInt + Mul + MulAssign + Scalar + Display,
    KType: PrimInt + AddAssign + AsPrimitive<OutputType> + Scalar + ClosedMul,
    OutputType: AsPrimitive<KType> + PrimInt + Debug + Bounded + AddAssign,
{
    let kernel_half = KSIZE / 2;
    let max_allowed_sum: KType = OutputType::max_value().as_();
    let min_allowed_sum: KType = OutputType::min_value().as_();

    let ncols = transposed_image.ncols();
    let nrows = transposed_image.nrows();

    for i in 0..nrows {
        let row = &transposed_image.row(i);
        for j in kernel_half..ncols - kernel_half {
            let mut sum = KType::zero();
            for ki in 0..KSIZE {
                sum += row[j - kernel_half + ki].as_() * piecewise_kernel[ki];
                // sum += transposed_image[(i, j - kernel_half + ki)].as_() * piecewise_kernel[ki];

                debug_assert_eq!(
                    transposed_image.vector_to_matrix_index((j - kernel_half + ki)* nrows +i),
                    (i, j - kernel_half + ki),
                    "Source column index calculations for row:{} and col:{} are not matching the produced flat index:{}",
                    j,
                    i,
                    j - kernel_half + ki
                );
            }

            debug_assert_eq!(
                transposed_image.vector_to_matrix_index(j * nrows + i),
                (i, j),
                "Destination index calculations for row:{} and col:{} are not matching the produced flat index:{}",
                j,
                i,
                j * nrows + i
            );
            dst[j * nrows + i] = sum.clamp(min_allowed_sum, max_allowed_sum).as_();
        }
    }
}

#[inline]
pub fn piecewise_2d_convolution_mut<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: &DMatrix<InputType>,
    dst: &mut [OutputType],
    piecewise_kernel_horizontal: &[KType; KSIZE],
    piecewise_kernel_vertical: &[KType; KSIZE],
) where
    InputType: AsPrimitive<KType> + PrimInt + Mul + MulAssign + Scalar + Display,
    KType: PrimInt + AddAssign + AsPrimitive<OutputType> + Scalar + ClosedMul,
    OutputType: AsPrimitive<KType> + PrimInt + Debug + Bounded + AddAssign + MulAssign + Display,
{
    assert!(
        dst.len() >= transposed_image.len(),
        "dst matrix ({:?}) must have same or larger size than input: {:?}",
        dst.len(),
        transposed_image.len(),
    );

    piecewise_horizontal_convolution_mut::<KSIZE, InputType, KType, OutputType>(
        transposed_image,
        dst,
        piecewise_kernel_horizontal,
    );

    piecewise_vertical_convolution_mut::<KSIZE, OutputType, KType, OutputType>(
        &DMatrix::from_column_slice(transposed_image.nrows(), transposed_image.ncols(), dst),
        dst,
        piecewise_kernel_vertical,
    );

    // let result_view = DMatrixView::from_slice(&out, image.nrows(), image.ncols());
}

pub fn imgproc_kernel_to_matrix<const K: usize>(kernel: &[i32]) -> SMatrix<i32, K, K> {
    // na::SMatrix::<i32, K, K>::from_iterator(kernel.iter().map(|&x| x as i32))
    SMatrix::<i32, K, K>::from_iterator(kernel.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use imageproc::gradients::HORIZONTAL_SOBEL;
    use nalgebra::{DMatrix, DMatrixView};

    fn get_image() -> DMatrix<i16> {
        let nrows = 10;
        let ncols = 5;

        let mut image = DMatrix::<i16>::zeros(nrows, ncols);
        image.view_mut((0, 0), (3, 5)).fill(255);
        image.view_mut((6, 0), (4, 5)).fill(255);

        image
    }
    const EXPECTED_SOBEL_OUT: [i16; 50] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -1020, -1020, -1020, -1020, -1020, -1020, -1020, -1020,
        -1020, -1020, 0, 0, 0, 0, 0, 1020, 1020, 1020, 1020, 1020, 1020, 1020, 1020, 1020, 1020, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    #[test]
    fn test_direct_convolution() {
        let image = get_image();
        let nrows = image.nrows();
        let ncols = image.ncols();

        // Since these operations assume the matrix is transposed, the kernel also has to be swapped
        let kernel = imgproc_kernel_to_matrix(&HORIZONTAL_SOBEL);

        let result =
            direct_convolution::<3, i16, i32, i16>(&image, &kernel, NonZeroU32::new(1).unwrap());

        // taken via OpenCV
        let expected_full_result =
            DMatrix::<i16>::from_row_slice(nrows, ncols, &EXPECTED_SOBEL_OUT);

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

        direct_convolution_mut::<3, i16, i32, i16>(
            &image,
            &mut fast_result.as_mut_slice(),
            &kernel,
            NonZeroU32::new(1).unwrap(),
        );
        let fast_result_subview = fast_result
            .view((1, 1), (nrows - 2, ncols - 2))
            .clone_owned();
        assert_eq!(
            fast_result_subview, expected_subview,
            "The faster version should match! {} {}",
            fast_result, expected_full_result
        );
    }

    #[test]
    fn test_piecewise_conv() {
        // Horizontal sobel
        // -1, 0, 1,
        // -2, 0, 2,
        // -1, 0, 1];

        // piecewise -> [1, 2, 1].T * [-1, 0, 1]

        let image = get_image();

        let mut out = vec![0; image.len()];

        let kernel_vertical = [1, 2, 1];
        let kernel_horizontal = [-1, 0, 1];

        piecewise_horizontal_convolution_mut::<3, i16, i32, i16>(
            &image,
            &mut out,
            &kernel_horizontal,
        );

        piecewise_vertical_convolution_mut::<3, i16, i32, i16>(
            &DMatrix::from_column_slice(image.nrows(), image.ncols(), &out),
            &mut out,
            &kernel_vertical,
        );

        let result_view = DMatrixView::from_slice(&out, image.nrows(), image.ncols());
        println!(
            "Input:\n {},\n output:\n{}",
            image,
            DMatrixView::from_slice(&out, image.nrows(), image.ncols())
        );

        let result_subview = result_view
            .view((1, 1), (image.nrows() - 2, image.ncols() - 2))
            .clone_owned();

        let expected_full_result =
            DMatrix::<i16>::from_row_slice(image.nrows(), image.ncols(), &EXPECTED_SOBEL_OUT);
        let expected_subview = expected_full_result
            .view((1, 1), (image.nrows() - 2, image.ncols() - 2))
            .clone_owned();

        assert_eq!(
            result_subview, expected_subview,
            "The sub-views of the results should match! {} {}",
            result_subview, expected_subview
        );
    }
}
