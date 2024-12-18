use itertools::{izip, Itertools};
use num_traits::{AsPrimitive, Bounded, PrimInt, Signed};
use simba::scalar::{SubsetOf, SupersetOf};
use std::{
    fmt::{Debug, Display},
    iter::Sum,
    num::NonZeroU32,
    ops::{Add, AddAssign, DivAssign, Mul, MulAssign},
};

use nalgebra::{ClosedMul, DMatrix, DMatrixView, SMatrix, SVector, Scalar, SimdValue};

pub fn direct_convolution<const KSIZE: usize, P, KType, S>(
    image: DMatrixView<P>,
    kernel: &SMatrix<KType, KSIZE, KSIZE>,
    scale_value: NonZeroU32,
) -> DMatrix<S>
where
    P: PrimInt + AsPrimitive<KType> + Scalar + Mul + MulAssign + Add,
    KType: PrimInt
        + AsPrimitive<S>
        + Scalar
        + AddAssign
        + ClosedMul
        + Signed
        + Sum<KType>
        + SupersetOf<P>,
    S: Into<KType>
        + PrimInt
        + AsPrimitive<KType>
        + Scalar
        + Debug
        + Bounded
        + AddAssign
        + DivAssign,
    u32: AsPrimitive<KType>,
{
    let (image_rows, image_cols) = image.shape();

    let mut result = DMatrix::<S>::zeros(image_rows, image_cols);

    // direct_convolution_mut scales well while direct_convolution_mut_try_again is great for small sized kernels
    if KSIZE > 5 {
        direct_convolution_mut(image, result.as_mut_slice(), kernel.clone(), scale_value);
    } else {
        direct_convolution_mut_try_again(image, result.as_mut_slice(), kernel.clone(), scale_value);
    }
    result
}

// This is great for larger kernels (i.e. 5x5) while direct_convolution_mut_try_again is best for smaller kernels
pub fn direct_convolution_mut<const KSIZE: usize, InputType, MyKtype, OutputType>(
    transposed_image: DMatrixView<InputType>,
    dst: &mut [OutputType],
    kernel: SMatrix<MyKtype, KSIZE, KSIZE>,
    scale_value: NonZeroU32,
) where
    InputType: AsPrimitive<MyKtype> + PrimInt + Scalar,
    MyKtype: PrimInt
        + Scalar
        + AddAssign
        + AsPrimitive<OutputType>
        + MulAssign
        + SupersetOf<InputType>
        + Sum<MyKtype>,

    OutputType: AsPrimitive<MyKtype> + PrimInt + Sized + Debug + Bounded,
    u32: AsPrimitive<MyKtype>,
{
    assert!(
        dst.len() >= transposed_image.len(),
        "dst matrix ({:?}) must have same or larger size than input: {:?}",
        dst.len(),
        transposed_image.shape(),
    );

    let (image_rows, image_cols) = transposed_image.shape();
    let kernel_half = KSIZE / 2;

    // let max_allowed_sum: MyKtype = OutputType::max_value().as_();
    // let min_allowed_sum: MyKtype = OutputType::min_value().as_();

    let input_mat_copy = transposed_image.map(|v| v.as_());
    let transposed_image_slice = input_mat_copy.as_slice();

    // scale_value.checked_next_power_of_two()
    let divisor: MyKtype = scale_value.get().as_();
    let should_divide_or_shift = divisor > MyKtype::one();
    let bit_shift_amount = if should_divide_or_shift {
        scale_value
            .checked_next_power_of_two()
            .unwrap()
            .trailing_zeros() as usize
    } else {
        0
    };

    let kernel_slice = kernel.as_slice();
    for column_index in kernel_half..image_cols - kernel_half {
        let column_top_left = column_index - kernel_half;

        // let neighbour_columns = (column_top_left..column_top_left + KSIZE)
        //     .map(|c| input_mat_copy.column(c))
        //     .collect::<Vec<_>>();

        dst[column_index * image_rows + kernel_half..(column_index + 1) * image_rows - kernel_half]
            .iter_mut()
            .enumerate()
            .for_each(|(i_top_left, dst_value)| {
                // .for_each(|dst_value| {
                // Semi-smart
                // let mut sum = MyKtype::zero();
                // for kj in 0..KSIZE {
                //     let ko = kj * KSIZE;
                //     let column_begin = (kj + column_top_left) * image_rows;
                //     for ki in 0..KSIZE {
                //         sum += transposed_image_slice[ki + i_top_left + column_begin]
                //             * kernel_slice[ki + ko];
                //     }
                // }

                // TODO find a way to flatten this?
                *dst_value = (0..KSIZE)
                    .map(move |kj| {
                        // TODO optimise this
                        let ko = kj * KSIZE;
                        let column_begin_flat = ((kj + column_top_left) * image_rows) + i_top_left;
                        let vs =
                            &transposed_image_slice[column_begin_flat..column_begin_flat + KSIZE];
                        let ks = &kernel_slice[ko..ko + KSIZE];
                        ks.iter().zip(vs).map(|(&k, &v)| k * v).sum::<MyKtype>()
                    })
                    .sum::<MyKtype>()
                    .shr(bit_shift_amount)
                    .as_();
            });
    }
}

pub fn direct_convolution_mut_try_again<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: DMatrixView<InputType>,
    dst_as_slice: &mut [OutputType],
    kernel: SMatrix<KType, KSIZE, KSIZE>,
    scale_value: NonZeroU32,
) where
    InputType: AsPrimitive<KType> + PrimInt + Mul + MulAssign + Scalar,
    KType: PrimInt
        + AsPrimitive<OutputType>
        + SupersetOf<InputType>
        + Scalar
        + AddAssign
        + ClosedMul
        + Sum,
    OutputType: AsPrimitive<KType> + PrimInt + Debug + Bounded + AddAssign + DivAssign,
    u32: AsPrimitive<KType>,
{
    assert!(
        dst_as_slice.len() >= transposed_image.len(),
        "dst matrix ({:?}) must have same or larger size than input: {:?}",
        dst_as_slice.len(),
        transposed_image.len(),
    );

    let image_rows = transposed_image.nrows();
    let ncols = transposed_image.ncols();
    let kernel_half = KSIZE / 2;

    let max_allowed_sum: KType = OutputType::max_value().as_();
    let min_allowed_sum: KType = OutputType::min_value().as_();

    let input_mat_copy = transposed_image.map(|v| v.as_());

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

    for column_index in kernel_half..ncols - kernel_half {
        let column_top_left = column_index - kernel_half;

        dst_as_slice[column_index * image_rows + kernel_half
            ..(column_index + 1) * image_rows - kernel_half]
            .iter_mut()
            .enumerate()
            .for_each(|(row_top_left, dst_value)| {
                // TODO find a better way for this, doesn't scale well for large kernels
                let sum = kernel
                    .component_mul(
                        &input_mat_copy.fixed_view::<KSIZE, KSIZE>(row_top_left, column_top_left),
                    )
                    .sum();

                *dst_value = (sum >> bit_shift_amount)
                    .clamp(min_allowed_sum, max_allowed_sum)
                    .as_();
            });
    }
}

#[allow(dead_code)]
#[inline(always)]
fn is_kernel_symmetric<const KSIZE: usize, KType: PrimInt>(kernel: &[KType; KSIZE]) -> bool {
    kernel[..KSIZE / 2] == kernel[KSIZE - (KSIZE / 2)..]
}

#[inline]
pub fn piecewise_horizontal_convolution_mut<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: DMatrixView<InputType>,
    dst: &mut [OutputType],
    piecewise_kernel: &[KType; KSIZE],
    _scale_value: usize,
) where
    InputType: AsPrimitive<KType> + PrimInt + Mul + MulAssign + Scalar + SubsetOf<KType> + Sized,
    KType: PrimInt
        + AddAssign
        + AsPrimitive<OutputType>
        + Scalar
        + ClosedMul
        + Display
        + SimdValue<Element = KType, SimdBool = bool>
        + Sum,
    OutputType:
        AsPrimitive<KType> + PrimInt + Debug + Bounded + AddAssign + Display + SubsetOf<KType>,
{
    let kernel_half = KSIZE / 2;
    // let kernel_half_ceil = KSIZE.div_ceil(2);
    // let is_symmetric = is_kernel_symmetric(piecewise_kernel);

    let max_allowed_sum: KType = OutputType::max_value().as_();
    let min_allowed_sum: KType = OutputType::min_value().as_();

    let nrows = transposed_image.nrows();
    let col_size_without_kernel_size = nrows - (kernel_half * 2);

    // Use this to cast the input data temporarily
    let mut temp_col = vec![KType::zero(); nrows];

    transposed_image
        .column_iter()
        .enumerate()
        .for_each(|(j, col)| {
            let out_non_chunked_begin = (j) * nrows + kernel_half;
            let out_non_chunked_end = out_non_chunked_begin + col_size_without_kernel_size;

            // Find a better way to do this!
            temp_col
                .iter_mut()
                .zip(col.as_slice())
                .for_each(|(dst, src)| *dst = src.as_());

            dst[out_non_chunked_begin..out_non_chunked_end]
                .iter_mut()
                .zip(temp_col.windows(KSIZE))
                .for_each(|(dst, src_col_piece)| {
                    // Non chunked basic version with windowing:
                    assert!(
                        src_col_piece.len() == piecewise_kernel.len(),
                        "src_col_piece.len() == KSIZE"
                    );

                    *dst = piecewise_kernel
                        .iter()
                        .zip(src_col_piece)
                        .map(|(k_cell, src_cell)| *src_cell * *k_cell)
                        .sum::<KType>()
                        .clamp(min_allowed_sum, max_allowed_sum)
                        .as_();
                });

            // let mut out_chunks_remainder = dst[out_non_chunked_end..].iter_mut();
        });
}

#[inline]
pub fn piecewise_vertical_convolution_mut<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: &DMatrix<InputType>,
    dst: &mut [OutputType],
    piecewise_kernel: &[KType; KSIZE],
    _scale_value: usize,
) where
    InputType: AsPrimitive<KType> + PrimInt + Mul + MulAssign + Scalar + Display + SubsetOf<KType>,
    KType: PrimInt + AddAssign + AsPrimitive<OutputType> + Scalar + ClosedMul + SimdValue + Sum,
    OutputType: AsPrimitive<KType> + SubsetOf<KType> + PrimInt + Debug + Bounded + AddAssign,
{
    let kernel_half = KSIZE / 2;
    let max_allowed_sum: KType = OutputType::max_value().as_();
    let min_allowed_sum: KType = OutputType::min_value().as_();

    let ncols = transposed_image.ncols();
    let nrows = transposed_image.nrows();

    const COLUMN_CHUNK_SIZE: usize = 8;
    // Handle remainder
    let chunking_remainder = nrows % COLUMN_CHUNK_SIZE;
    let image_slice = transposed_image.as_slice();

    for j in kernel_half..ncols - kernel_half {
        let flat_slice_column_start_position = j * nrows;
        let flat_slice_column_end_position = flat_slice_column_start_position + nrows;
        let j_top_left = j - kernel_half;
        // TODO try this!
        // let cols = transposed_image.fixed_columns::<KSIZE>(j - kernel_half);
        let column_pack_slices = (j_top_left..j_top_left + KSIZE)
            .map(|kernel_aligned_column_index| {
                &image_slice
                    [kernel_aligned_column_index * nrows..(kernel_aligned_column_index + 1) * nrows]
            })
            .collect_vec();

        dst[flat_slice_column_start_position..flat_slice_column_end_position]
            .chunks_exact_mut(COLUMN_CHUNK_SIZE)
            .enumerate()
            .for_each(|(ci, dst_chunk)| {
                let mut acccum = SVector::<KType, COLUMN_CHUNK_SIZE>::zeros();
                piecewise_kernel
                    .iter()
                    .zip(column_pack_slices.iter())
                    .for_each(|(piece, input_column)| {
                        // assert_eq!(input_column.len(), dst_chunk.len());
                        acccum += SVector::<KType, COLUMN_CHUNK_SIZE>::from_iterator(
                            input_column[ci * COLUMN_CHUNK_SIZE..(ci + 1) * COLUMN_CHUNK_SIZE]
                                .iter()
                                .map(|v| v.as_()),
                        ) * *piece
                    });

                dst_chunk
                    .iter_mut()
                    .zip(acccum.iter())
                    .for_each(|(dst, acc)| {
                        // TODO bit shifting for scaling
                        *dst = acc.clamp(&min_allowed_sum, &max_allowed_sum).as_()
                    });
            });
        // Handle remainder from chunking
        if chunking_remainder != 0 {
            //
            let mut accum = vec![KType::zero(); chunking_remainder]; // Vec::<KType>::with_capacity(COLUMN_CHUNK_SIZE); //[KType::zero(); COLUMN_CHUNK_SIZE];
            let flat_remainder_range =
                flat_slice_column_end_position - chunking_remainder..flat_slice_column_end_position;

            assert!(
                chunking_remainder < COLUMN_CHUNK_SIZE,
                "Remainder is larger than chunk size"
            );
            izip!(
                piecewise_kernel,
                column_pack_slices
                    .iter()
                    .map(|c| { &c[chunking_remainder..] }),
            )
            .for_each(|(piece, src): (&KType, &[InputType])| {
                accum
                    .iter_mut()
                    .zip(src.iter())
                    .for_each(|(acc_dst, src)| *acc_dst += *piece * src.as_());
            });

            accum
                .iter()
                .zip(dst[flat_remainder_range].iter_mut())
                .for_each(|(acc_dst, dst)| {
                    // TODO bit shifting for scaling
                    *dst = acc_dst.as_()
                });

            // take? or some other way?
            // &dst[remainder_range]
            //     .iter_mut()
            //     .for_each(|dst_value| {
            //         dst_chunk
            //             .iter_mut()
            //             .zip(acccum.iter())
            //             .for_each(|(dst, acc)| {
            //                 // TODO bit shifting for scaling
            //                 *dst = acc.clamp(&min_allowed_sum, &max_allowed_sum).as_()
            //             });
            //     });
        }
    }
}

#[inline]
pub fn piecewise_2d_convolution_mut<const KSIZE: usize, InputType, KType, OutputType>(
    transposed_image: DMatrixView<InputType>,
    dst: &mut [OutputType],
    piecewise_kernel_horizontal: &[KType; KSIZE],
    piecewise_kernel_vertical: &[KType; KSIZE],
    // TODO implement
    _scale_value: usize,
) where
    InputType: AsPrimitive<KType> + PrimInt + Mul + MulAssign + Scalar + SubsetOf<KType>,
    KType: PrimInt
        + AddAssign
        + AsPrimitive<OutputType>
        + Scalar
        + ClosedMul
        + Display
        + SimdValue<Element = KType, SimdBool = bool>
        + Sum,
    OutputType: AsPrimitive<KType>
        + PrimInt
        + Debug
        + Bounded
        + AddAssign
        + MulAssign
        + Display
        + SubsetOf<KType>,
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
        1,
    );

    // TODO see if we can avoid this allocation
    piecewise_vertical_convolution_mut::<KSIZE, OutputType, KType, OutputType>(
        &DMatrix::from_column_slice(transposed_image.nrows(), transposed_image.ncols(), dst),
        dst,
        piecewise_kernel_vertical,
        1,
    );
}

pub fn imgproc_kernel_to_matrix<const K: usize>(kernel: &[i32]) -> SMatrix<i32, K, K> {
    SMatrix::<i32, K, K>::from_iterator(kernel.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    use imageproc::gradients::HORIZONTAL_SOBEL;
    use nalgebra::{DMatrix, DMatrixView};

    const NROWS: usize = 10;
    const NCOLS: usize = 5;

    fn get_image() -> DMatrix<i16> {
        let mut image = DMatrix::<i16>::zeros(NROWS, NCOLS);

        // Draws this (as displayed when printed)
        // ┌                     ┐
        // │ 255 255   0 255 255 │
        // │ 255 255   0 255 255 │
        // │ 255 255   0 255 255 │
        // │   0   0   0   0   0 │
        // │   0   0   0   0   0 │
        // │   0   0   0   0   0 │
        // │ 255 255   0 255 255 │
        // │ 255 255   0 255 255 │
        // │ 255 255   0 255 255 │
        // │ 255 255   0 255 255 │
        // └                     ┘
        image.view_mut((0, 0), (3, 5)).fill(255);
        image.view_mut((6, 0), (4, 5)).fill(255);
        image.view_mut((0, 2), (NROWS, 1)).fill(0);

        image
    }
    const EXPECTED_SOBEL_HORIZONTAL_OUT: [[i16; NROWS]; NCOLS] = [
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        [
            -1020, -765, -510, -765, -1020, -1020, -765, -510, -765, -1020,
        ],
        [0, 0, 0, 0, 0, 1020, 765, 510, 765, 1020],
        [1020, 765, 510, 765, 1020, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ];

    const EXPECTED_SOBEL_VERTICAL_OUT: [[i16; NROWS]; NCOLS] = [
        [0, -1020, 0, 1020, 0, 0, -1020, 0, 1020, 0],
        [0, -765, 0, 765, 0, 0, -255, 0, 255, 0],
        [0, 0, 0, 0, 0, 0, -255, 0, 255, 0],
        [0, -765, 0, 765, 0, 0, -1020, 0, 1020, 0],
        [0, -1020, 0, 1020, 0, 0, -1020, 0, 1020, 0],
    ];

    #[test]
    fn test_kernel_symmetry() {
        let symmetric = &[[1, 2, 1], [2, 4, 2], [1, 2, 1]];
        let asymmetric = &[[1, 2, 5], [2, 4, 3], [0, 2, 1]];

        assert!(symmetric.iter().map(is_kernel_symmetric).all(|v| v));
        assert!(asymmetric.iter().map(is_kernel_symmetric).all(|v| !v));
    }

    #[test]
    fn test_direct_convolution() {
        let image = get_image();
        let nrows = image.nrows();
        let ncols = image.ncols();

        // Since these operations assume the matrix is transposed, the kernel also has to be swapped
        let kernel = imgproc_kernel_to_matrix(&HORIZONTAL_SOBEL);

        let result = direct_convolution::<3, i16, i32, i16>(
            image.as_view(),
            &kernel,
            NonZeroU32::new(1).unwrap(),
        );

        // taken via OpenCV
        let expected_horizontal_full_result = DMatrix::<i16>::from_row_slice(
            NROWS,
            NCOLS,
            EXPECTED_SOBEL_HORIZONTAL_OUT.as_flattened(),
        );

        let result_subview = result.view((1, 1), (nrows - 2, ncols - 2)).clone_owned();
        let expected_subview = expected_horizontal_full_result
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
            image.as_view(),
            &mut fast_result.as_mut_slice(),
            kernel,
            NonZeroU32::new(1).unwrap(),
        );
        let fast_result_subview = fast_result
            .view((1, 1), (nrows - 2, ncols - 2))
            .clone_owned();
        assert_eq!(
            fast_result_subview, expected_subview,
            "The faster version should match! {} {}",
            fast_result, expected_horizontal_full_result
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
            image.as_view(),
            &mut out,
            &kernel_horizontal,
            1,
        );

        piecewise_vertical_convolution_mut::<3, i16, i32, i16>(
            &DMatrix::from_column_slice(image.nrows(), image.ncols(), &out),
            &mut out,
            &kernel_vertical,
            1,
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

        let expected_full_result = DMatrix::<i16>::from_row_slice(
            image.nrows(),
            image.ncols(),
            &EXPECTED_SOBEL_HORIZONTAL_OUT.as_flattened(),
        );
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
