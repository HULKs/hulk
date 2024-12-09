use image::GrayImage;
use nalgebra::{coordinates::X, DMatrix, Matrix3, MatrixView3, SMatrixView};

use crate::{
    gaussian::gaussian_blur_box_filter_nalgebra,
    sobel::{sobel_operator_horizontal, sobel_operator_vertical},
};

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
    gradients_x.zip_map(gradients_y, |gx, gy| (gx * gx + gy * gy) as u16)
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
    // This trick is taken from OpenCV's Canny implementation
    // The idea is to perform the tan22.5 and tan67.5 boundary calculations with integers only

    // Select the case based on the following inequality
    // check if following inequality is true
    // tan(x) > tan(22deg)
    // let t = tan(22deg)
    // -> tan(x) * 2^15 > t * 2^15
    // -> t * 2^15 > y * 2^15 / x
    // -> t * x * 2^15 > y * 2^15

    // To grab the perpendicular two pixels to edge direction, only 4 of the 8 possible directions are needed (the opposide ones of each direction yields the same.)
    // Due to this symmetry, calculations can be done in first quadrant (-> abs values) and then check the signs only for the diagonals.
    let abs_y = y.abs() as i32;
    let abs_x = x.abs() as i32;

    // round(tan(22.5) * 2**15), tan(67.5) * 2**15 -> 22.5 = 45/2, 67.5 = 45 + 22.5
    const TAN_SHIFTED_22_5: i32 = 13573;
    const TAN_SHIFTED_67_5: i32 = 79109;

    // x * tan22.5 * 2^15
    let tan_22_5_mul_x = TAN_SHIFTED_22_5 * abs_x;
    let tan_67_5_mul_x = TAN_SHIFTED_67_5 * abs_x;

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
    gradients_magnitude_region: &DMatrix<u16>,
) -> u16 {
    // TODO Optimize the element access method -> take columns as slices and do the up/down shifting to avoid the bound checks, etc

    let direction = approximate_direction_integer_only(gx, gy);
    let (cmp1, cmp2) = match direction {
        0 => (
            gradients_magnitude_region[(center_x - 1, center_y)],
            gradients_magnitude_region[(center_x + 1, center_y)],
        ),
        45 => (
            gradients_magnitude_region[(center_x + 1, center_y + 1)],
            gradients_magnitude_region[(center_x - 1, center_y - 1)],
        ),
        90 => (
            gradients_magnitude_region[(center_x, center_y - 1)],
            gradients_magnitude_region[(center_x, center_y + 1)],
        ),
        135 => (
            gradients_magnitude_region[(center_x - 1, center_y + 1)],
            gradients_magnitude_region[(center_x + 1, center_y - 1)],
        ),
        _ => unreachable!(),
    };
    let pixel = gradients_magnitude_region[(center_x, center_y)];
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
