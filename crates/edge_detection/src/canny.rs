use image::GrayImage;

use nalgebra::DMatrix;

use crate::{
    gaussian::gaussian_blur_box_filter_nalgebra,
    grayimage_to_2d_transposed_matrix_view,
    sobel::{sobel_operator_horizontal, sobel_operator_vertical},
};

pub fn canny(
    image: &GrayImage,
    gaussian_sigma: Option<f32>,
    low_threshold: f32,
    high_threshold: f32,
) -> DMatrix<EdgeClassification> {
    let sigma = gaussian_sigma.unwrap_or(1.4);
    const SOBEL_KERNEL_SIZE: usize = 3;

    let input = grayimage_to_2d_transposed_matrix_view::<u8>(image);
    let converted = gaussian_blur_box_filter_nalgebra(&input, sigma);

    let gx = sobel_operator_horizontal::<SOBEL_KERNEL_SIZE, i16>(&converted);
    let gy = sobel_operator_vertical::<SOBEL_KERNEL_SIZE, i16>(&converted);

    let peak_gradients =
        non_maximum_suppression(&gx, &gy, low_threshold as u16, high_threshold as u16);
    hysteresis(&peak_gradients)
}

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u8)]
pub enum EdgeClassification {
    HighConfidence = 2,
    LowConfidence = 1,
    #[default]
    NoConfidence = 0,
}

fn gradient_magnitude(gx: i16, gy: i16) -> u16 {
    ((gx as f32).powi(2) + (gy as f32).powi(2)).sqrt() as u16
}

#[inline(always)]
pub(crate) fn get_gradient_magnitude(
    gradients_x: &DMatrix<i16>,
    gradients_y: &DMatrix<i16>,
) -> DMatrix<u16> {
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

#[inline(always)]
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
    if y_shifted < tan_22_5_mul_x || y == 0 {
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

    for index in nrows..gradients_x_slice.len() - nrows {
        let previous_column_point = index - nrows;
        let next_column_point = index + nrows;

        let pixel = flat_slice[index];
        let (pixel_is_larger_than_lowest_threshold, pixel_is_larger_than_higher_threshold) =
            (pixel > lower_threshold, pixel > upper_threshold);

        let pixel_is_the_largest = pixel_is_larger_than_lowest_threshold
            && match approximate_direction_integer_only(
                gradients_y_slice[index],
                gradients_x_slice[index],
            ) {
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
    }

    out
}

/// Filter out edges with the thresholds.
/// Non-recursive breadth-first search.
fn hysteresis(input: &DMatrix<EdgeClassification>) -> DMatrix<EdgeClassification> {
    // Init output image as all black.
    let mut out = DMatrix::<EdgeClassification>::repeat(
        input.nrows(),
        input.ncols(),
        EdgeClassification::NoConfidence,
    );
    // Stack. Possible optimization: Use previously allocated memory, i.e. gx.
    let mut edges = Vec::with_capacity(out.len() / 2_usize);

    for y in 1..input.ncols() - 1 {
        for x in 1..input.nrows() - 1 {
            let inp_pix = input[(x, y)];
            let out_pix = out[(x, y)];
            // If the edge strength is higher than high_thresh, mark it as an edge.
            if inp_pix == EdgeClassification::HighConfidence
                && out_pix == EdgeClassification::NoConfidence
            {
                out[(x, y)] = EdgeClassification::HighConfidence;
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
                        let in_neighbor = input[(neighbor_idx.0, neighbor_idx.1)];
                        let out_neighbor = out[(neighbor_idx.0, neighbor_idx.1)];
                        if in_neighbor >= EdgeClassification::LowConfidence
                            && out_neighbor == EdgeClassification::NoConfidence
                        {
                            out[(neighbor_idx.0, neighbor_idx.1)] =
                                EdgeClassification::HighConfidence;
                            edges.push((neighbor_idx.0, neighbor_idx.1));
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {

    use std::i16;

    use super::{approximate_direction_integer_only, OctantWithDegName};

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
