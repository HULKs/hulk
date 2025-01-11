use image::GrayImage;
use nalgebra::{DMatrix, DMatrixView};

use crate::{
    gaussian::gaussian_blur_integer_approximation,
    sobel::{sobel_operator, DerivativeDirection},
    zip_three_slices_enumerated,
};

pub fn canny(
    image_transposed: DMatrixView<u8>,
    gaussian_sigma: Option<f32>,
    low_threshold: f32,
    high_threshold: f32,
) -> (DMatrix<EdgeClassification>, usize) {
    let sigma = gaussian_sigma.unwrap_or(1.4);

    let converted = gaussian_blur_integer_approximation::<u8, u8>(image_transposed, sigma);
    let converted_view = converted.as_view();

    let gx = sobel_operator(converted_view, DerivativeDirection::Horizontal);
    let gy = sobel_operator(converted_view, DerivativeDirection::Vertical);

    let peak_gradients =
        non_maximum_suppression(&gx, &gy, low_threshold as u16, high_threshold as u16);
    hysteresis(&peak_gradients)
}

pub fn canny_edges_with_directions(
    image: &GrayImage,
    gaussian_sigma: Option<f32>,
    low_threshold: f32,
    high_threshold: f32,
) -> (Vec<i8>, usize) {
    let sigma = gaussian_sigma.unwrap_or(1.4);

    let input = DMatrixView::from_slice(
        image.as_raw(),
        image.width() as usize,
        image.height() as usize,
    );

    // Transposed shape, as GrayImage is row-major while the matrix is col. major
    assert_eq!(
        (image.height() as usize, image.width() as usize),
        (input.ncols(), input.nrows())
    );
    let converted = gaussian_blur_integer_approximation::<u8, u8>(input.as_view(), sigma);
    let converted_view = converted.as_view();

    let gx = sobel_operator(converted_view, DerivativeDirection::Horizontal);
    let gy = sobel_operator(converted_view, DerivativeDirection::Vertical);

    let peak_gradients =
        non_maximum_suppression(&gx, &gy, low_threshold as u16, high_threshold as u16);
    let (filterd_peak_gradients, count) = hysteresis(&peak_gradients);

    (
        filterd_peak_gradients
            .as_slice()
            .iter()
            .zip(gx.as_slice().iter())
            .zip(gy.as_slice().iter())
            .filter_map(|((&classification, &gx), &gy)| {
                if classification >= EdgeClassification::LowConfidence {
                    let basic_octants = approximate_direction_integer_only(gx, gy);
                    // This math is wrong :P
                    Some((basic_octants as i8) * gy.signum() as i8)
                } else {
                    None
                }
            })
            .collect(),
        count,
    )
}

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u8)]
pub enum EdgeClassification {
    HighConfidence = 2,
    LowConfidence = 1,
    #[default]
    NoConfidence = 0,
}

// TODO investigate a way to improve this, it appears on profiling.
fn gradient_magnitude(gx: i16, gy: i16) -> u32 {
    (gx.unsigned_abs() as u32).pow(2) + (gy.unsigned_abs() as u32).pow(2)
}

#[inline(always)]
pub(crate) fn get_gradient_magnitude(
    gradients_x: &DMatrix<i16>,
    gradients_y: &DMatrix<i16>,
) -> DMatrix<u32> {
    gradients_x.zip_map(gradients_y, gradient_magnitude)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum OctantWithDegName {
    FirstOctant0,
    SecondOctant45,
    ThirdOctant90,
    FourthOctant135,
}

#[inline]
fn approximate_direction_integer_only(y: i16, x: i16) -> OctantWithDegName {
    // This trick is taken from OpenCV's Canny implementation
    // The idea is to perform the tan22.5 and tan67.5 boundary calculations with integers only avoiding float calculations and atan2, etc
    // To grab the perpendicular two pixels to edge direction, only 4 of the 8 possible directions are needed (the opposide ones of each direction yields the same.)
    // Due to this symmetry, calculations can be done in first quadrant (-> abs values) and then check the signs only for the diagonals.

    // Select the case based on the following inequality
    // tan(x) > tan(22deg)
    // let t = tan(22deg)
    // -> tan(x) * 2^15 > t * 2^15
    // -> t * 2^15 > y * 2^15 / x
    // -> t * x * 2^15 > y * 2^15

    // round(tan(22.5) * 2**15), tan(67.5) * 2**15 -> 22.5 = 45/2, 67.5 = 45 + 22.5
    const TAN_SHIFTED_22_5: u32 = 13573;
    const TAN_SHIFTED_67_5: u32 = 79109;

    let abs_y = y.unsigned_abs() as u32;
    let abs_x = x.unsigned_abs() as u32;

    let tan_22_5_mul_x = TAN_SHIFTED_22_5 * abs_x;
    let tan_67_5_mul_x = TAN_SHIFTED_67_5 * abs_x;

    // y * 2^15
    let y_shifted = (abs_y) << 15;

    // check if the inequalities hold
    if y == 0 || y_shifted < tan_22_5_mul_x {
        OctantWithDegName::FirstOctant0
    } else if y_shifted >= tan_67_5_mul_x {
        OctantWithDegName::ThirdOctant90
    } else {
        // let only_one_is_negative = y.is_positive() ^ x.is_positive();
        // if only_one_is_negative {
        if y.signum() == x.signum() {
            OctantWithDegName::SecondOctant45
        } else {
            OctantWithDegName::FourthOctant135
        }
    }
}

/// Non-maximum suppression of edges.
/// One major change is to classify points as high-confidence or low-confidence earlier than the canonical implementation.
/// This will be used in hysteresis thresholding.
#[inline]
pub fn non_maximum_suppression(
    gradients_x: &DMatrix<i16>,
    gradients_y: &DMatrix<i16>,
    lower_threshold: u16,
    upper_threshold: u16,
) -> DMatrix<EdgeClassification> {
    // TODO try doing this in chunks.
    let gradients_magnitude = get_gradient_magnitude(gradients_x, gradients_y);

    let gradients_x_slice = gradients_x.as_slice();
    let gradients_y_slice = gradients_y.as_slice();

    let nrows = gradients_x.nrows();

    let mut out = DMatrix::<EdgeClassification>::repeat(
        gradients_x.nrows(),
        gradients_x.ncols(),
        EdgeClassification::NoConfidence,
    );

    let flat_slice = gradients_magnitude.as_slice();
    let out_slice = out.as_mut_slice();

    let start = nrows;
    let end = gradients_x_slice.len() - nrows;

    let gxs = &gradients_x_slice[start..end];
    let gys = &gradients_y_slice[start..end];
    let fs = &flat_slice[start..end];

    let lower_threshold_squared = (lower_threshold as u32).pow(2);
    let upper_threshold_squared = (upper_threshold as u32).pow(2);

    zip_three_slices_enumerated(fs, gxs, gys).for_each(
        |(previous_column_point, &pixel, gx, gy)| {
            let index = previous_column_point + nrows;

            let next_column_point = index + nrows;

            let (pixel_is_larger_than_lowest_threshold, pixel_is_larger_than_higher_threshold) = (
                pixel > lower_threshold_squared,
                pixel > upper_threshold_squared,
            );

            let pixel_is_the_largest = pixel_is_larger_than_lowest_threshold
                && match approximate_direction_integer_only(*gy, *gx) {
                    OctantWithDegName::FirstOctant0 => {
                        pixel > flat_slice[index - 1] && pixel > flat_slice[index + 1]
                    }
                    OctantWithDegName::SecondOctant45 => {
                        pixel > flat_slice[previous_column_point - 1]
                            && pixel > flat_slice[next_column_point + 1]
                    }
                    OctantWithDegName::ThirdOctant90 => {
                        pixel > flat_slice[previous_column_point]
                            && pixel > flat_slice[next_column_point]
                    }
                    OctantWithDegName::FourthOctant135 => {
                        pixel > flat_slice[previous_column_point + 1]
                            && pixel > flat_slice[next_column_point - 1]
                    }
                };

            // Suppress non-maximum pixel. low threshold is earlier handled
            if pixel_is_the_largest {
                out_slice[index] = if pixel_is_larger_than_higher_threshold {
                    EdgeClassification::HighConfidence
                } else {
                    EdgeClassification::LowConfidence
                };
            }
        },
    );

    out
}

/// Implementation taken from imageproc with some modifications.
/// https://github.com/image-rs/imageproc/blob/master/src/edges.rs
/// Filter out edges with the thresholds.
fn hysteresis(input: &DMatrix<EdgeClassification>) -> (DMatrix<EdgeClassification>, usize) {
    // Init output image as all black.
    let mut out = DMatrix::<EdgeClassification>::repeat(
        input.nrows(),
        input.ncols(),
        EdgeClassification::NoConfidence,
    );
    let in_slice = input.as_slice();
    let out_slice = out.as_mut_slice();
    let in_out_len = in_slice.len();
    // Stack. Possible optimization: Use previously allocated memory, i.e. gx.
    let mut edges = Vec::with_capacity(out_slice.len() / 2_usize);

    let mut counter = 0;
    for y in 1..input.ncols() - 1 {
        for x in 1..input.nrows() - 1 {
            // These need profiling
            let flat_slice_location = y * input.nrows() + x;
            assert!(in_out_len > flat_slice_location);
            // let inp_pix = input[(x, y)];
            // let out_pix = out[(x, y)];
            let inp_pix = in_slice[flat_slice_location];
            let out_pix = &mut out_slice[flat_slice_location];
            // If the edge strength is higher than high_thresh, mark it as an edge.
            if inp_pix == EdgeClassification::HighConfidence
                && *out_pix == EdgeClassification::NoConfidence
            {
                // out[(x, y)] = EdgeClassification::HighConfidence;
                // out_slice[flat_slice_location] = EdgeClassification::HighConfidence;
                *out_pix = EdgeClassification::HighConfidence;
                counter += 1;
                edges.push((x, y));
                // Track neighbors until no neighbor is >= low_thresh.
                while let Some((nx, ny)) = edges.pop() {
                    let neighbor_indices = [
                        (nx + 1, ny),
                        (nx + 1, ny + 1),
                        (nx, ny + 1),
                        (nx - 1, ny - 1),
                        (nx - 1, ny),
                        (nx - 1, ny + 1),
                    ];

                    for neighbor_idx in &neighbor_indices {
                        // These need profiling
                        let neighbour_flat_idx = neighbor_idx.1 * input.nrows() + neighbor_idx.0;
                        assert!(in_out_len > flat_slice_location);
                        // let in_neighbor = input[(neighbor_idx.0, neighbor_idx.1)];
                        // let out_neighbor = out[(neighbor_idx.0, neighbor_idx.1)];
                        let in_neighbor = in_slice[neighbour_flat_idx];
                        let out_neighbor = &mut out_slice[neighbour_flat_idx];
                        if in_neighbor >= EdgeClassification::LowConfidence
                            && *out_neighbor == EdgeClassification::NoConfidence
                        {
                            // out[(neighbor_idx.0, neighbor_idx.1)] =
                            //     EdgeClassification::HighConfidence;
                            // out_slice[neighbour_flat_idx] = EdgeClassification::HighConfidence;
                            *out_neighbor = EdgeClassification::HighConfidence;
                            counter += 1;
                            edges.push((neighbor_idx.0, neighbor_idx.1));
                        }
                    }
                }
            }
        }
    }
    (out, counter)
}

#[cfg(test)]
mod tests {

    use std::i16;

    use image::GrayImage;

    use super::{approximate_direction_integer_only, canny, OctantWithDegName};
    use crate::{
        get_edge_source_transposed_image, get_test_data_location, load_test_image, EdgeSourceType,
    };

    #[test]
    fn test_overall() {
        let cropped_input =
            get_edge_source_transposed_image(&load_test_image(), EdgeSourceType::LumaOfYCbCr, None);

        assert!(cropped_input.nrows() > cropped_input.ncols());
        let (transposed_canny_image_matrix, _point_count) =
            canny(cropped_input.as_view(), Some(1.4), 20.0, 50.0);

        let mut new_image = GrayImage::new(
            transposed_canny_image_matrix.nrows() as u32,
            transposed_canny_image_matrix.ncols() as u32,
        );
        transposed_canny_image_matrix
            .iter()
            .enumerate()
            .for_each(|(index, pixel)| {
                let (x, y) = transposed_canny_image_matrix.vector_to_matrix_index(index);

                assert!(
                    x < new_image.width() as usize && y < new_image.height() as usize,
                    "Starting point x:{x} y:{y}, index: {index}, shape: {:?}",
                    (
                        transposed_canny_image_matrix.nrows(),
                        transposed_canny_image_matrix.ncols()
                    )
                );

                new_image[(x as u32, y as u32)][0] = *pixel as u8;
            });
        new_image
            .save(format!(
                "{}/test_data/output/canny_ours.png",
                get_test_data_location()
            ))
            .unwrap();
    }
    #[test]
    fn non_maximum_suppression_direction_approximation() {
        let bounds = [
            (i16::MIN, i16::MAX),
            (i16::MAX, i16::MIN),
            (i16::MAX, i16::MAX),
            (i16::MIN, i16::MIN),
            (0, 0),
        ];
        for (x, y) in bounds {
            assert_eq!(
                approximate_direction(y, x),
                approximate_direction_integer_only(y, x),
                "Failed! x:{x}, y:{y}"
            );
        }

        // Radius mode
        let angles: Vec<_> = (0..360).map(|deg| (deg as f32).to_radians()).collect();

        for radius in [0, 20, 1000, 5000, i16::MAX] {
            for angle in angles.iter() {
                let x_component = radius as f32 * angle.cos();
                let y_component = radius as f32 * angle.sin();

                let atan_based_clamped_angle =
                    approximate_direction(y_component as i16, x_component as i16);

                let integer_approximation =
                    approximate_direction_integer_only(y_component as i16, x_component as i16);
                assert_eq!(
                    atan_based_clamped_angle, integer_approximation,
                    "Failed! x:{x_component}, y:{y_component}, radius:{radius}, angle(rad):{angle:?}"
                );
            }
        }
    }

    fn approximate_direction(y: i16, x: i16) -> OctantWithDegName {
        if y == 0 {
            return OctantWithDegName::FirstOctant0;
        }
        if x == 0 {
            return OctantWithDegName::ThirdOctant90;
        }

        let mut angle = (y as f32).atan2(x as f32).to_degrees();
        if angle < 0.0 {
            angle += 180.0
        }
        // Clamp angle.
        if !(22.5..157.5).contains(&angle) {
            OctantWithDegName::FirstOctant0
        } else if (22.5..67.5).contains(&angle) {
            OctantWithDegName::SecondOctant45
        } else if (67.5..112.5).contains(&angle) {
            OctantWithDegName::ThirdOctant90
        } else if (112.5..157.5).contains(&angle) {
            OctantWithDegName::FourthOctant135
        } else {
            unreachable!()
        }
    }
}
