use nalgebra::DMatrix;

// fn canny(image: &DMatrix<u8>, low_threshold: f32, high_threshold: f32) -> GrayImage {
//     let sigma = 1.4;
//     const SOBEL_KERNEL_SIZE: usize = 3;
//     let blurred = gaussian_blur_box_filter_nalgebra(&image, sigma);

//     let gx = sobel_operator_horizontal::<SOBEL_KERNEL_SIZE, i16>(&blurred);
//     let gy = sobel_operator_vertical::<SOBEL_KERNEL_SIZE, i16>(&blurred);

//     let peak_gradients = non_maximum_suppression(&gx, &gy);
//     todo!()
// }

#[inline(always)]
pub(crate) fn get_gradient_magnitude(
    gradients_x: &DMatrix<i16>,
    gradients_y: &DMatrix<i16>,
) -> DMatrix<u16> {
    gradients_x.zip_map(gradients_y, |gx, gy| {
        ((gx as f32).powi(2) + (gy as f32).powi(2)).sqrt() as u16
    })
}

#[inline(always)]
fn approximate_direction(y: i16, x: i16) -> u8 {
    if y == 0 {
        return 0;
    }
    if x == 0 {
        return 90;
    }

    let mut angle = (y as f32).atan2(x as f32).to_degrees();
    if angle < 0.0 {
        angle += 180.0
    }
    // Clamp angle.
    if !(22.5..157.5).contains(&angle) {
        0
    } else if (22.5..67.5).contains(&angle) {
        45
    } else if (67.5..112.5).contains(&angle) {
        90
    } else if (112.5..157.5).contains(&angle) {
        135
    } else {
        unreachable!()
    }
}

#[inline(always)]
fn approximate_direction_integer_only(y: i16, x: i16) -> u8 {
    if y == 0 {
        return 0;
    }
    if x == 0 {
        return 90;
    }

    // This trick is taken from OpenCV's Canny implementation
    // The idea is to perform the tan22.5 and tan67.5 boundary calculations with integers only avoiding float calculations and atan2, etc

    // Select the case based on the following inequality
    // tan(x) > tan(22deg)
    // let t = tan(22deg)
    // -> tan(x) * 2^15 > t * 2^15
    // -> t * 2^15 > y * 2^15 / x
    // -> t * x * 2^15 > y * 2^15

    // round(tan(22.5) * 2**15), tan(67.5) * 2**15 -> 22.5 = 45/2, 67.5 = 45 + 22.5
    const TAN_SHIFTED_22_5: u32 = 13573;
    const TAN_SHIFTED_67_5: u32 = 79109;

    // To grab the perpendicular two pixels to edge direction, only 4 of the 8 possible directions are needed (the opposide ones of each direction yields the same.)
    // Due to this symmetry, calculations can be done in first quadrant (-> abs values) and then check the signs only for the diagonals.
    let abs_y = y.unsigned_abs() as u32;
    let abs_x = x.unsigned_abs() as u32;

    // NOTE: u32 can work as TAN_SHIFTED_67_5 * i16::MAX < u32::MAX
    let tan_22_5_mul_x = TAN_SHIFTED_22_5 * abs_x as u32;
    let tan_67_5_mul_x = TAN_SHIFTED_67_5 * abs_x as u32;

    // y * 2^15
    let y_shifted = (abs_y) << 15;

    // check if the inequalities hold
    if y_shifted < tan_22_5_mul_x {
        0
    } else if y_shifted < tan_67_5_mul_x {
        let only_one_is_negative = y.is_positive() ^ x.is_positive();
        if only_one_is_negative {
            135
        } else {
            45
        }
    } else {
        90
    }
}

#[inline]
pub fn non_maximum_suppression(
    gradients_x: &DMatrix<i16>,
    gradients_y: &DMatrix<i16>,
) -> DMatrix<u16> {
    // let angles = gradients_y.zip_map(gradients_x, approximate_direction_integer_only);
    let gradients_magnitude = get_gradient_magnitude(&gradients_x, &gradients_y);

    let gradients_x_slice = gradients_x.as_slice();
    let gradients_y_slice = gradients_y.as_slice();

    let (xmax, ymax) = (gradients_x.nrows() - 1, gradients_x.ncols() - 1);
    DMatrix::<u16>::from_iterator(
        gradients_x.nrows(),
        gradients_x.ncols(),
        // angles
        (0..gradients_magnitude.len()).map(|index| {
            let (x, y) = gradients_magnitude.vector_to_matrix_index(index);
            if x == 0 || y == 0 || x == xmax || y == ymax {
                return 0;
            }
            select_maximum_pixel(
                x,
                y,
                gradients_x_slice[index],
                gradients_y_slice[index],
                &gradients_magnitude,
            )
        }),
    )
}

fn select_maximum_pixel(
    center_x: usize,
    center_y: usize,
    gx: i16,
    gy: i16,
    gradients_magnitude: &DMatrix<u16>,
) -> u16 {
    // TODO Optimize the element access method -> take columns as slices and do the up/down shifting to avoid the bound checks, etc

    let direction = approximate_direction_integer_only(gx, gy);
    let (cmp1, cmp2) = match direction {
        0 => (
            gradients_magnitude[(center_x - 1, center_y)],
            gradients_magnitude[(center_x + 1, center_y)],
        ),
        45 => (
            gradients_magnitude[(center_x + 1, center_y + 1)],
            gradients_magnitude[(center_x - 1, center_y - 1)],
        ),
        90 => (
            gradients_magnitude[(center_x, center_y - 1)],
            gradients_magnitude[(center_x, center_y + 1)],
        ),
        135 => (
            gradients_magnitude[(center_x - 1, center_y + 1)],
            gradients_magnitude[(center_x + 1, center_y - 1)],
        ),
        _ => unreachable!(),
    };
    let pixel = gradients_magnitude[(center_x, center_y)];
    // Suppress non-maximum pixel

    if pixel < cmp1 || pixel < cmp2 {
        0
    } else {
        pixel
    }
}

fn hysteresis() {
    todo!()
}

#[cfg(test)]
mod tests {

    use std::i16;

    use super::{approximate_direction, approximate_direction_integer_only};

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
                    "Failed! x:{x_component}, y:{y_component}, radius:{radius}, angle:{angle:?}"
                );
            }
        }
    }
}
