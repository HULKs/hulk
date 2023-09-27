#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::{
    _mm256_add_epi16, _mm256_castsi256_si128, _mm256_insert_epi8, _mm256_loadu_si256,
    _mm256_mullo_epi16, _mm256_permute4x64_epi64, _mm256_setr_epi16, _mm256_setr_epi8,
    _mm256_shuffle_epi8, _mm256_sra_epi16, _mm_setr_epi8, _mm_storeu_si128,
};

use color_eyre::{
    eyre::{eyre, WrapErr},
    Result,
};
use parking_lot::{Condvar, Mutex};
use types::{camera_position::CameraPosition, color::YCbCr422, ycbcr422_image::YCbCr422Image};
use webots::Robot;

use super::hardware_interface::SIMULATION_TIME_STEP;

pub struct Camera {
    camera: webots::Camera,
    buffer: Mutex<Option<Vec<u8>>>,
    buffer_updated: Condvar,
}

impl Camera {
    pub fn new(position: CameraPosition) -> Self {
        let camera = Robot::get_camera(match position {
            CameraPosition::Top => "CameraTop",
            CameraPosition::Bottom => "CameraBottom",
        });
        camera.enable(SIMULATION_TIME_STEP);

        Self {
            camera,
            buffer: Mutex::new(None),
            buffer_updated: Condvar::new(),
        }
    }

    pub fn update_image(&self) -> Result<()> {
        let image_data_input = match self
            .camera
            .get_image()
            .wrap_err("failed to get image from camera")
        {
            Ok(image_data_input) => image_data_input,
            Err(error) => {
                self.unblock_read();
                return Err(error);
            }
        };

        {
            let mut bgra_buffer = self.buffer.lock();
            *bgra_buffer = Some(image_data_input.to_vec());
        }
        self.buffer_updated.notify_all();

        Ok(())
    }

    pub fn unblock_read(&self) {
        self.buffer_updated.notify_all();
    }

    pub fn read(&self) -> Result<YCbCr422Image> {
        let bgra_buffer = {
            let mut bgra_buffer = self.buffer.lock();
            self.buffer_updated.wait(&mut bgra_buffer);
            bgra_buffer
                .take()
                .ok_or_else(|| eyre!("no updated image found"))?
        };
        assert_eq!(bgra_buffer.len(), 4 * 640 * 480);
        let mut ycbcr_buffer = vec![
            YCbCr422 {
                y1: 0,
                cb: 0,
                y2: 0,
                cr: 0
            };
            320 * 480
        ];
        bgra_444_to_ycbcr_422(&bgra_buffer, &mut ycbcr_buffer);
        Ok(YCbCr422Image::from_ycbcr_buffer(320, 480, ycbcr_buffer))
    }
}

fn bgra_444_to_ycbcr_422(bgra_444: &[u8], ycbcr_422: &mut [YCbCr422]) {
    assert_eq!(bgra_444.len() % 32, 0);
    assert_eq!(8 * ycbcr_422.len(), bgra_444.len());

    #[cfg(target_arch = "x86_64")]
    if is_x86_feature_detected!("avx2") {
        // Conversion factors from https://de.wikipedia.org/wiki/YCbCr-Farbmodell#Umrechnung_zwischen_RGB_und_YCbCr
        // Consider two 444 BGRA pixels: [ B0 G0 R0 A0 ] [ B1 G1 R1 A1 ]
        // Then the single 422 YCbCr pixel [ Y0 Cb Y1 Cr ] is calculated via:
        //
        // Y0 =         0.299    * R0 + 0.587    * G0 + 0.114    * B0
        // Cb = 128.0 - 0.168736 * R0 - 0.331264 * G0 + 0.5      * B0
        // Y1 =         0.299    * R1 + 0.587    * G1 + 0.114    * B1
        // Cr = 128.0 + 0.5      * R0 - 0.418688 * G0 - 0.081312 * B0
        //
        // The vectorized implementation uses two simplifications:
        // 1. Integer calculus for performance (all values are multiplied by 128)
        // 2. Cb and Cr are calculated from the first BGRA pixel instead of both
        //
        // This leads to the modified formulas:
        //
        // Y0 =         38 * R0 + 75 * G0 + 15 * B0
        // Cb = 16384 - 22 * R0 - 42 * G0 + 64 * B0
        // Y1 =         38 * R1 + 75 * G1 + 15 * B1
        // Cr = 16384 + 64 * R0 - 54 * G0 - 10 * B0
        //      ^       ^    ^    ^    ^    ^    ^
        // offset       |    |    |    |    |    blue_values
        //    red_factors    |    |    |    blue_factors
        //          red_values    |    green_valus
        //                  green_factors
        //
        // The multiplication by 128 transforms the color range from 0 - 255 to 0 - 32640.
        // The implementation therefore uses i16 which range from -32768 - 32767.
        //
        // 256 bit AVX2 vector registers allow to process
        //   32 BGRA color components = 8 BGRA 444 pixels = 4 resulting YCbCr 422 pixels
        // The vector is therefore split into 4 identical quarters that calculate the same formulas on different input slices (SIMD).
        // This means many vectors in the calculation repeat themselves 4 times.
        //
        // The output will be a 128 bit AVX vector with packed YCbCr 422 color components:
        //
        //  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15
        // ---------------------------------------------------------------
        //  Y0 Cb0 Y1  Cr0 Y2  Cb2 Y3  Cr2 Y4  Cb4 Y5  Cr4 Y6  Cb6 Y7  Cr6
        //
        // Starting from the beginning, for calculating a whole 256 bit vector we need 4 identical quarters to form the factor vectors:
        //
        //                0   1   2   3   4   5   6   7   ...
        //                -----------------------------------
        // offset:        0       16384   0       16384   ...
        // red_factors:   38     -22      38      64      ...
        // green_factors: 75     -42      75     -54      ...
        // blue_factors:  15      64      15     -10      ...
        //
        // These are already of type i16. We also need the color components correctly aligned to match the formulas above:
        //
        //                0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  ...
        //               -------------------------------------------------------------------
        // red_values:   R0   0  R0   0  R1   0  R0   0  R2   0  R2   0  R3   0  R2   0  ...
        // green_values: G0   0  G0   0  G1   0  G0   0  G2   0  G2   0  G3   0  G2   0  ...
        // blue_values:  B0   0  B0   0  B1   0  B0   0  B2   0  B2   0  B3   0  B2   0  ...
        //
        // To achieve this we're using some tricks with shuffling. _mm256_shuffle_epi8 allows to permute the i8 items of a 256 bit vector.
        // It takes as input the original vector and an index vector specifying which original items to pick for the resulting vector.
        //
        // Example: Consider an input vector of [7 6 5 4 3 2 1 0] and an index vector of [0 2 4 6 0 2 4 6],
        //          then the resulting vector will be [7 5 3 1 7 5 3 1].
        //
        // The input vector will be the original BGRA 444 pixel data. We are only interested in BGR and not the alpha channel. The alpha
        // channel is therefore used as zero provider by overwriting the fourth (index = 3) byte of the input vector to zero.
        // The input vector is:
        //
        //  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  ...
        // -------------------------------------------------------------------
        // B0  G0  R0  0   B1  G1  R1  A1  B2  G2  R2  A2  B3  G3  R3  A3  ...
        //
        // We can use the shuffle operation to construct the vectors red_values, green_values, and blue_values by rearranging the items.
        // At the zero places the input index 3 is used to create a zero. The resulting vector can be interpreted as i16 vector.
        //
        // The next instructions are straight-forward multiplications and additions to achieve the resulting sum of weighted color channels.
        // Since the calculations used the whole range of i16 because of the multiplication by 128, the last step is to divide by 128 again.
        // This is done by shifting the resulting vector 7 bits to the right.
        //
        // The shuffle operation can only shuffle within both 128 bit halfs and cannot shuffle across halfs. To convert the sparse i16 to
        // packed u8 again, we pack all items within the two halfs (with shuffle) and then permute the 64 bit of packed data into 128 bit.
        // The 256 bit containing packed 128 bit of data is then truncated and written to the output data slice.

        unsafe {
            let offset = _mm256_setr_epi16(
                0, 16384, 0, 16384, 0, 16384, 0, 16384, 0, 16384, 0, 16384, 0, 16384, 0, 16384,
            );
            let red_factors = _mm256_setr_epi16(
                38, -22, 38, 64, 38, -22, 38, 64, 38, -22, 38, 64, 38, -22, 38, 64,
            );
            let green_factors = _mm256_setr_epi16(
                75, -42, 75, -54, 75, -42, 75, -54, 75, -42, 75, -54, 75, -42, 75, -54,
            );
            let blue_factors = _mm256_setr_epi16(
                15, 64, 15, -10, 15, 64, 15, -10, 15, 64, 15, -10, 15, 64, 15, -10,
            );
            let red_value_indices = _mm256_setr_epi8(
                2, 3, 2, 3, 6, 3, 2, 3, 10, 3, 10, 3, 14, 3, 10, 3, 18, 3, 18, 3, 22, 3, 18, 3, 26,
                3, 26, 3, 30, 3, 26, 3,
            );
            let green_value_indices = _mm256_setr_epi8(
                1, 3, 1, 3, 5, 3, 1, 3, 9, 3, 9, 3, 13, 3, 9, 3, 17, 3, 17, 3, 21, 3, 17, 3, 25, 3,
                25, 3, 29, 3, 25, 3,
            );
            let blue_value_indices = _mm256_setr_epi8(
                0, 3, 0, 3, 4, 3, 0, 3, 8, 3, 8, 3, 12, 3, 8, 3, 16, 3, 16, 3, 20, 3, 16, 3, 24, 3,
                24, 3, 28, 3, 24, 3,
            );
            let shift_counts = _mm_setr_epi8(7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
            let result_indices = _mm256_setr_epi8(
                0, 2, 4, 6, 8, 10, 12, 14, 0, 0, 0, 0, 0, 0, 0, 0, 16, 18, 20, 22, 24, 26, 28, 30,
                0, 0, 0, 0, 0, 0, 0, 0,
            );
            const RESULT_PERMUTATION: i32 = 0b00_00_10_00;

            for (bgra_444, ycbcr_422) in
                bgra_444.chunks_exact(32).zip(ycbcr_422.chunks_exact_mut(4))
            {
                let bgra_444 = _mm256_loadu_si256(bgra_444.as_ptr() as *const _);
                let bgra_444 = _mm256_insert_epi8(bgra_444, 0, 3);

                let red_values = _mm256_shuffle_epi8(bgra_444, red_value_indices);
                let green_values = _mm256_shuffle_epi8(bgra_444, green_value_indices);
                let blue_values = _mm256_shuffle_epi8(bgra_444, blue_value_indices);

                let red_summand = _mm256_mullo_epi16(red_factors, red_values);
                let green_summand = _mm256_mullo_epi16(green_factors, green_values);
                let blue_summand = _mm256_mullo_epi16(blue_factors, blue_values);

                let sum = _mm256_add_epi16(offset, red_summand);
                let sum = _mm256_add_epi16(sum, green_summand);
                let sum = _mm256_add_epi16(sum, blue_summand);

                let result = _mm256_sra_epi16(sum, shift_counts);

                let result = _mm256_shuffle_epi8(result, result_indices);
                let result = _mm256_permute4x64_epi64(result, RESULT_PERMUTATION);
                let result = _mm256_castsi256_si128(result);

                _mm_storeu_si128(ycbcr_422.as_ptr() as *mut _, result);
            }
        }
        return;
    }

    bgra_444_to_ycbcr_422_fallback(bgra_444, ycbcr_422);
}

fn bgra_444_to_ycbcr_422_fallback(bgra_444: &[u8], ycbcr_422: &mut [YCbCr422]) {
    assert_eq!(bgra_444.len() % 32, 0);
    assert_eq!(8 * ycbcr_422.len(), bgra_444.len());

    for (bgra_444, ycbcr_422) in bgra_444.chunks_exact(8).zip(ycbcr_422.iter_mut()) {
        let first_blue = bgra_444[0] as i16;
        let first_green = bgra_444[1] as i16;
        let first_red = bgra_444[2] as i16;
        let second_blue = bgra_444[4] as i16;
        let second_green = bgra_444[5] as i16;
        let second_red = bgra_444[6] as i16;

        let first_luminance = 38 * first_red + 75 * first_green + 15 * first_blue;
        let chromaticity_blue = 16384 - 22 * first_red - 42 * first_green + 64 * first_blue;
        let second_luminance = 38 * second_red + 75 * second_green + 15 * second_blue;
        let chromaticity_red = 16384 + 64 * first_red - 54 * first_green - 10 * first_blue;

        ycbcr_422.y1 = (first_luminance / 128) as u8;
        ycbcr_422.cb = (chromaticity_blue / 128) as u8;
        ycbcr_422.y2 = (second_luminance / 128) as u8;
        ycbcr_422.cr = (chromaticity_red / 128) as u8;
    }
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use rand::{prelude::StdRng, Rng, SeedableRng};

    use super::*;

    const SEED: u64 = 42;

    fn assert_slice_eq<T>(left: &[T], right: &[T])
    where
        T: fmt::Debug + PartialEq,
    {
        assert_eq!(left.len(), right.len());
        for index in 0..left.len() {
            assert_eq!(left[index], right[index], "left[{index}] != right[{index}]",);
        }
    }

    #[test]
    fn zero_image() {
        let bgra_444 = [0; 32];
        let mut ycbcr_422 = [Default::default(); 4];
        bgra_444_to_ycbcr_422(&bgra_444, &mut ycbcr_422);
        let mut manual_ycbcr_422 = [Default::default(); 4];
        bgra_444_to_ycbcr_422_fallback(&bgra_444, &mut manual_ycbcr_422);
        assert_slice_eq(&ycbcr_422, &manual_ycbcr_422);
    }

    #[test]
    fn same_value_images() {
        for value in 0..=255 {
            let bgra_444 = [value; 32];
            let mut ycbcr_422 = [Default::default(); 4];
            bgra_444_to_ycbcr_422(&bgra_444, &mut ycbcr_422);
            let mut manual_ycbcr_422 = [Default::default(); 4];
            bgra_444_to_ycbcr_422_fallback(&bgra_444, &mut manual_ycbcr_422);
            assert_slice_eq(&ycbcr_422, &manual_ycbcr_422);
        }
    }

    #[test]
    fn random_value_images() {
        let mut random_number_generator = StdRng::seed_from_u64(SEED);
        for _ in 0..1000 {
            let bgra_444 = (0..32)
                .map(|_| random_number_generator.gen_range(0..=255))
                .collect::<Vec<_>>();
            let mut ycbcr_422 = [Default::default(); 4];
            bgra_444_to_ycbcr_422(&bgra_444, &mut ycbcr_422);
            let mut manual_ycbcr_422 = [Default::default(); 4];
            bgra_444_to_ycbcr_422_fallback(&bgra_444, &mut manual_ycbcr_422);
            assert_slice_eq(&ycbcr_422, &manual_ycbcr_422);
        }
    }
}
