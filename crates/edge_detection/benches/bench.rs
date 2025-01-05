use std::{env, fs::File};

use divan::{bench, bench_group, black_box, Bencher};
use image::GrayImage;
use imageproc::{edges::canny, filter::gaussian_blur_f32, gradients::sobel_gradients};

use edge_detection::{
    get_edge_source_image_old, get_edge_source_transposed_image,
    transposed_matrix_view_to_gray_image, EdgeSourceType,
};
use nalgebra::DMatrixView;
use pprof::{ProfilerGuard, ProfilerGuardBuilder};
use types::ycbcr422_image::YCbCr422Image;

fn main() {
    let args: Vec<String> = env::args().collect();
    dbg!(args);
    divan::main();
}

const GAUSSIAN_SIGMA: f32 = 1.4;
const EDGE_SOURCE_TYPE: EdgeSourceType = EdgeSourceType::LumaOfYCbCr;

fn get_profiler_guard() -> Option<ProfilerGuard<'static>> {
    if env::var("ENABLE_FLAMEGRAPH").is_ok_and(|v| v == "1") {
        ProfilerGuardBuilder::default()
            .frequency(10000)
            .blocklist(&["pthread", "vdso"])
            .build()
            .ok()
    } else {
        None
    }
}

fn get_flamegraph(file_name: &str, guard: Option<ProfilerGuard<'static>>) {
    if let Some(report) = guard.map(|guard| guard.report().build().ok()).flatten() {
        let file = File::create(format!(
            "{}/test_data/output/{}.svg",
            env!("CARGO_MANIFEST_DIR"),
            file_name
        ))
        .unwrap();
        report.flamegraph(file).unwrap();
    };
}

fn load_test_image() -> YCbCr422Image {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    YCbCr422Image::load_from_rgb_file(format!("{crate_dir}/test_data/center_circle_webots.png"))
        .unwrap()
}

fn get_blurred_source_image(image: &YCbCr422Image) -> GrayImage {
    let transposed_matrix =
        get_edge_source_transposed_image(black_box(image), EDGE_SOURCE_TYPE, None);
    let edges_source = transposed_matrix_view_to_gray_image(transposed_matrix.as_view());
    gaussian_blur_f32(&edges_source, 3.5)
}

#[bench]
fn imageproc_sobel_gradients(bencher: Bencher) {
    let image = load_test_image();
    let blurred = get_blurred_source_image(&image);

    bencher.bench_local(move || sobel_gradients(black_box(&blurred)));
}

#[bench]
fn imageproc_canny(bencher: Bencher) {
    let image =
        get_edge_source_transposed_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE, None);
    let edges_source = transposed_matrix_view_to_gray_image(image.as_view());

    bencher.bench_local(move || canny(black_box(&edges_source), 20.0, 50.0));
}

#[bench(args=[EdgeSourceType::DifferenceOfGrayAndRgbRange ,EdgeSourceType::LumaOfYCbCr])]
fn edge_source_select(bencher: Bencher, source_type: EdgeSourceType) {
    let image = load_test_image();

    bencher.bench_local(move || {
        black_box(get_edge_source_transposed_image(
            black_box(&image),
            black_box(source_type),
            black_box(None),
        ))
    });
}

#[bench(args=[EdgeSourceType::DifferenceOfGrayAndRgbRange ,EdgeSourceType::LumaOfYCbCr])]
fn edge_source_select_old(bencher: Bencher, source_type: EdgeSourceType) {
    let image = load_test_image();

    bencher.bench_local(move || {
        black_box(get_edge_source_image_old(
            black_box(&image),
            black_box(source_type),
            None,
        ))
    });
}

const GAUSSIAN_VALUES: &[f32] = &[0.5, 1.0, 1.4, 2.0, 3.5];
#[bench_group]
mod blurring {
    use divan::{bench, black_box, Bencher};
    use edge_detection::{
        gaussian::{gaussian_blur_box_filter, gaussian_blur_integer_approximation},
        get_edge_source_transposed_image, transposed_matrix_view_to_gray_image,
    };
    use imageproc::filter::gaussian_blur_f32;

    use crate::{
        get_flamegraph, get_profiler_guard, load_test_image, EDGE_SOURCE_TYPE, GAUSSIAN_VALUES,
    };

    #[bench(args=GAUSSIAN_VALUES)]
    fn gaussian_blur_with_box_filter(bencher: Bencher, sigma: f32) {
        let image =
            get_edge_source_transposed_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE, None);
        let gray_image = transposed_matrix_view_to_gray_image(image.as_view());

        bencher.bench_local(move || {
            black_box(gaussian_blur_box_filter(
                black_box(&gray_image),
                black_box(sigma),
            ))
        });
    }

    #[bench(args=GAUSSIAN_VALUES, min_time=2)]
    fn gaussian_blur_int_approximation(bencher: Bencher, sigma: f32) {
        let transposed_matrix =
            get_edge_source_transposed_image(&load_test_image(), EDGE_SOURCE_TYPE, None);
        let transposed_matrix_view = transposed_matrix.as_view();

        let guard = if sigma == 1.0 {
            get_profiler_guard()
        } else {
            None
        };
        bencher.bench_local(move || {
            black_box(gaussian_blur_integer_approximation::<u8, u8>(
                black_box(transposed_matrix_view),
                black_box(sigma),
            ))
        });
        get_flamegraph("int_approx", guard);
    }

    #[bench(args=GAUSSIAN_VALUES, min_time=2)]
    fn gaussian_blur_int_approximation_i16_i32_i16(bencher: Bencher, sigma: f32) {
        let transposed_matrix =
            get_edge_source_transposed_image(&load_test_image(), EDGE_SOURCE_TYPE, None).cast();
        let transposed_matrix_view = transposed_matrix.as_view();

        let guard = if sigma == 1.0 {
            get_profiler_guard()
        } else {
            None
        };
        bencher.bench_local(move || {
            black_box(gaussian_blur_integer_approximation::<i16, i16>(
                black_box(transposed_matrix_view),
                black_box(sigma),
            ))
        });
        get_flamegraph("int_approx", guard);
    }

    #[bench(args=GAUSSIAN_VALUES)]
    fn imageproc_blurring(bencher: Bencher, sigma: f32) {
        let image =
            get_edge_source_transposed_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE, None);
        let edges_source = transposed_matrix_view_to_gray_image(image.as_view());

        bencher.bench_local(move || black_box(gaussian_blur_f32(black_box(&edges_source), sigma)));
    }
}

fn _bench_with_kernel_size_piecewise<const K: usize, B, O, S>(
    bencher: Bencher,
    output_prefix: &str,
    transposed_matrix_view: DMatrixView<u8>,
    kernel_slice_1: &[i32; K],
    kernel_slice_2: &[i32; K],
    scaled_facot: S,
    benched: B,
) where
    B: Fn(DMatrixView<u8>, &mut [i16], &[i32; K], &[i32; K], S) -> O,
    S: Copy,
{
    let guard = get_profiler_guard();
    bencher.bench_local(move || {
        let mut out = vec![0i16; transposed_matrix_view.len()];
        black_box(benched(
            black_box(transposed_matrix_view),
            black_box(out.as_mut_slice()),
            black_box(kernel_slice_1),
            black_box(kernel_slice_2),
            black_box(scaled_facot),
        ));
    });
    get_flamegraph(format!("{output_prefix}_{K}").as_str(), guard);
}

#[bench_group]
mod sobel_operator {

    use std::num::NonZeroU32;

    use divan::{bench, black_box, Bencher};
    use edge_detection::{
        filter2d::{
            direct_convolution_mut, direct_convolution_mut_alternative,
            piecewise_2d_convolution_mut, piecewise_horizontal_convolution_mut,
            piecewise_vertical_convolution_mut,
        },
        get_edge_source_transposed_image,
        sobel::sobel_operator_vertical,
    };

    use imageproc::gradients::vertical_sobel;
    use nalgebra::SMatrix;
    use num_traits::One;

    use crate::{
        _bench_with_kernel_size_piecewise, get_blurred_source_image, get_flamegraph,
        get_profiler_guard, load_test_image, EDGE_SOURCE_TYPE,
    };

    #[bench(args=[3,5,7])]
    fn direct_convolution_mut_new(bencher: Bencher, kernel_size: usize) {
        let image = load_test_image();
        let transposed_matrix = get_edge_source_transposed_image(&image, EDGE_SOURCE_TYPE, None);
        let transposed_matrix_view = transposed_matrix.as_view();
        let mat_len = transposed_matrix.len();
        let output_prefix = "direct_convolution_mut_try_again_";

        match kernel_size {
            3 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                SMatrix::<i32, 3, 3>::one(),
                NonZeroU32::new(1).unwrap(),
                direct_convolution_mut_alternative,
            ),
            5 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                SMatrix::<i32, 5, 5>::one(),
                NonZeroU32::new(1).unwrap(),
                direct_convolution_mut_alternative,
            ),
            7 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                SMatrix::<i32, 7, 7>::one(),
                NonZeroU32::new(1).unwrap(),
                direct_convolution_mut_alternative,
            ),
            _ => unreachable!("Unsupported kernel size"),
        }
    }

    #[bench(args=[3,5,7,11,21])]
    fn direct_convolution_mut_old_vertical(bencher: Bencher, kernel_size: usize) {
        let image = load_test_image();
        let transposed_matrix = get_edge_source_transposed_image(&image, EDGE_SOURCE_TYPE, None);
        let transposed_matrix_view = transposed_matrix.as_view();
        let mat_len = transposed_matrix.len();
        let output_prefix = "direct_convolution_mut_old_";

        match kernel_size {
            3 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                SMatrix::<i32, 3, 3>::one(),
                NonZeroU32::new(1).unwrap(),
                direct_convolution_mut,
            ),
            5 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                SMatrix::<i32, 5, 5>::one(),
                NonZeroU32::new(1).unwrap(),
                direct_convolution_mut,
            ),
            7 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                SMatrix::<i32, 7, 7>::one(),
                NonZeroU32::new(1).unwrap(),
                direct_convolution_mut,
            ),
            11 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                SMatrix::<i32, 11, 11>::one(),
                NonZeroU32::new(1).unwrap(),
                direct_convolution_mut,
            ),

            21 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                SMatrix::<i32, 21, 21>::one(),
                NonZeroU32::new(1).unwrap(),
                direct_convolution_mut,
            ),
            _ => unreachable!("Unsupported kernel size"),
        }

        // bencher.bench_local(move || {
        //     let mut out = vec![0i16; transposed_matrix_view.len()];
        //     black_box(direct_convolution_mut::<3, u8, i32, i16>(
        //         black_box(&transposed_matrix_view),
        //         black_box(out.as_mut_slice()),
        //         black_box(&kernel_vert),
        //         black_box(scale_factor),
        //     ));
        // });
    }

    fn _bench_with_kernel_size<KT, B, O, F, Mat>(
        bencher: Bencher,
        output_prefix: &str,
        transposed_matrix_view: Mat,
        out_length: usize,
        kernel_slice: KT,
        _scale_factor: F,
        benched: B,
    ) where
        B: Fn(Mat, &mut [i16], KT, F) -> O,
        F: Copy,
        KT: Copy,
        Mat: Copy,
    {
        let guard = get_profiler_guard();
        bencher.bench_local(move || {
            let mut out = vec![0i16; out_length];
            black_box(benched(
                black_box(transposed_matrix_view),
                black_box(out.as_mut_slice()),
                black_box(kernel_slice),
                black_box(_scale_factor),
            ));
        });
        get_flamegraph(output_prefix, guard);
    }

    #[bench(args=[3,5,7,11,21])]
    fn piecewise_2d_mut(bencher: Bencher, kernel_size: usize) {
        let image = load_test_image();
        let transposed_matrix = get_edge_source_transposed_image(&image, EDGE_SOURCE_TYPE, None);
        let transposed_matrix_view = transposed_matrix.as_view();

        let output_prefix = "piecewise_horiz_2d";
        match kernel_size {
            3 => {
                _bench_with_kernel_size_piecewise(
                    bencher,
                    format!("{output_prefix}_{kernel_size}").as_str(),
                    transposed_matrix_view,
                    &[1; 3],
                    &[2; 3],
                    NonZeroU32::new(1).unwrap(),
                    piecewise_2d_convolution_mut::<3, _, _, _>,
                );
            }
            5 => {
                _bench_with_kernel_size_piecewise(
                    bencher,
                    format!("{output_prefix}_{kernel_size}").as_str(),
                    transposed_matrix_view,
                    &[1; 5],
                    &[2; 5],
                    NonZeroU32::new(1).unwrap(),
                    piecewise_2d_convolution_mut::<5, _, _, _>,
                );
            }
            7 => {
                _bench_with_kernel_size_piecewise(
                    bencher,
                    format!("{output_prefix}_{kernel_size}").as_str(),
                    transposed_matrix_view,
                    &[1; 7],
                    &[2; 7],
                    NonZeroU32::new(1).unwrap(),
                    piecewise_2d_convolution_mut::<7, _, _, _>,
                );
            }
            11 => {
                _bench_with_kernel_size_piecewise(
                    bencher,
                    format!("{output_prefix}_{kernel_size}").as_str(),
                    transposed_matrix_view,
                    &[1; 11],
                    &[2; 11],
                    NonZeroU32::new(1).unwrap(),
                    piecewise_2d_convolution_mut::<11, _, _, _>,
                );
            }
            21 => {
                _bench_with_kernel_size_piecewise(
                    bencher,
                    format!("{output_prefix}_{kernel_size}").as_str(),
                    transposed_matrix_view,
                    &[1; 21],
                    &[2; 21],
                    NonZeroU32::new(1).unwrap(),
                    piecewise_2d_convolution_mut::<21, _, _, _>,
                );
            }
            _ => panic!("Unsupported kernel size"),
        }
    }

    #[bench(args=[3,5,7,11,13,21])]
    fn piecewise_vertical_mut_ksizes(bencher: Bencher, kernel_size: usize) {
        let image = load_test_image();
        let transposed_matrix = get_edge_source_transposed_image(&image, EDGE_SOURCE_TYPE, None);
        let mat_len = transposed_matrix.len();

        let output_prefix = "piecewise_vert";
        match kernel_size {
            3 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                &transposed_matrix,
                mat_len,
                &[1; 3],
                NonZeroU32::new(1).unwrap(),
                piecewise_vertical_convolution_mut::<3, _, _, _>,
            ),
            5 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                &transposed_matrix,
                mat_len,
                &[1; 5],
                NonZeroU32::new(1).unwrap(),
                piecewise_vertical_convolution_mut::<5, _, _, _>,
            ),
            7 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                &transposed_matrix,
                mat_len,
                &[1; 7],
                NonZeroU32::new(1).unwrap(),
                piecewise_vertical_convolution_mut::<7, _, _, _>,
            ),
            11 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                &transposed_matrix,
                mat_len,
                &[1; 11],
                NonZeroU32::new(1).unwrap(),
                piecewise_vertical_convolution_mut::<11, _, _, _>,
            ),
            13 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                &transposed_matrix,
                mat_len,
                &[1; 13],
                NonZeroU32::new(1).unwrap(),
                piecewise_vertical_convolution_mut::<13, _, _, _>,
            ),
            21 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                &transposed_matrix,
                mat_len,
                &[1; 21],
                NonZeroU32::new(1).unwrap(),
                piecewise_vertical_convolution_mut::<21, _, _, _>,
            ),
            _ => panic!("Unsupported kernel size"),
        }
    }

    #[bench(args=[3,5,7,11,13,21])]
    fn piecewise_horizontal_mut_ksizes(bencher: Bencher, kernel_size: usize) {
        let image = load_test_image();
        let transposed_matrix = get_edge_source_transposed_image(&image, EDGE_SOURCE_TYPE, None);
        let transposed_matrix_view = transposed_matrix.as_view();
        let mat_len = transposed_matrix.len();
        let output_prefix = "piecewise_horiz";
        match kernel_size {
            3 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                &[1; 3],
                NonZeroU32::new(1).unwrap(),
                piecewise_horizontal_convolution_mut::<3, u8, i32, i16>,
            ),
            5 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                &[1; 5],
                NonZeroU32::new(1).unwrap(),
                piecewise_horizontal_convolution_mut::<5, u8, i32, i16>,
            ),
            7 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                &[1; 7],
                NonZeroU32::new(1).unwrap(),
                piecewise_horizontal_convolution_mut::<7, u8, i32, i16>,
            ),
            11 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                &[1; 11],
                NonZeroU32::new(1).unwrap(),
                piecewise_horizontal_convolution_mut::<11, u8, i32, i16>,
            ),
            13 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                &[1; 13],
                NonZeroU32::new(1).unwrap(),
                piecewise_horizontal_convolution_mut::<13, u8, i32, i16>,
            ),
            21 => _bench_with_kernel_size(
                bencher,
                format!("{output_prefix}_{kernel_size}").as_str(),
                transposed_matrix_view,
                mat_len,
                &[1; 21],
                NonZeroU32::new(1).unwrap(),
                piecewise_horizontal_convolution_mut::<21, u8, i32, i16>,
            ),
            _ => panic!("Unsupported kernel size"),
        }
    }

    #[bench]
    fn direct_convolution_sobel_vertical_wrapper(bencher: Bencher) {
        let image = load_test_image();
        let transposed_matrix = get_edge_source_transposed_image(&image, EDGE_SOURCE_TYPE, None);
        let transposed_matrix_view = transposed_matrix.as_view();
        bencher.bench_local(move || {
            black_box(sobel_operator_vertical::<u8>(black_box(
                transposed_matrix_view,
            )));
        });
    }

    #[bench]
    fn direct_convolution_sobel_vertical_wrapper_i16_input(bencher: Bencher) {
        let image = load_test_image();
        let transposed_matrix =
            get_edge_source_transposed_image(&image, EDGE_SOURCE_TYPE, None).cast();
        let transposed_matrix_view = transposed_matrix.as_view();
        bencher.bench_local(move || {
            black_box(sobel_operator_vertical::<i16>(black_box(
                transposed_matrix_view,
            )));
        });
    }

    #[bench]
    fn imageproc_sobel_vertical(bencher: Bencher) {
        let image = load_test_image();
        let blurred = get_blurred_source_image(&image);
        bencher.bench_local(move || black_box(vertical_sobel(black_box(&blurred))));
    }
}

#[bench_group]
mod edge_points {

    use divan::{bench, black_box, Bencher};

    use edge_detection::{
        canny::non_maximum_suppression,
        gaussian::gaussian_blur_integer_approximation,
        get_edge_source_transposed_image, get_edges_canny, get_edges_canny_imageproc,
        sobel::{get_edges_sobel_and_nms, sobel_operator_horizontal, sobel_operator_vertical},
    };
    use nalgebra::DMatrix;

    use crate::{
        get_flamegraph, get_profiler_guard, load_test_image, EDGE_SOURCE_TYPE, GAUSSIAN_SIGMA,
        GAUSSIAN_VALUES,
    };

    #[bench(args=GAUSSIAN_VALUES, min_time=2)]
    fn our_canny(bencher: Bencher, sigma: f32) {
        let image = load_test_image();

        let guard = get_profiler_guard();
        bencher.bench_local(move || {
            black_box(get_edges_canny(
                black_box(sigma),
                black_box(20.0),
                black_box(50.0),
                black_box(&image),
                EDGE_SOURCE_TYPE,
                None,
            ))
        });
        get_flamegraph("edges_our_canny", guard);
    }

    #[bench]
    fn direct_convolution_sobel_both_axes(bencher: Bencher) {
        let image = load_test_image();

        let guard = get_profiler_guard();
        bencher.bench_local(move || {
            black_box(get_edges_sobel_and_nms(
                black_box(3.5),
                black_box(100),
                black_box(100),
                black_box(&image),
                EDGE_SOURCE_TYPE,
            ))
        });
        get_flamegraph("edges_sobel", guard);
    }

    // #[bench]
    #[bench]
    fn non_maximum_suppression_our_impl(bencher: Bencher) {
        let image = load_test_image();
        let converted = get_edge_source_transposed_image(&image, EDGE_SOURCE_TYPE, None);
        let gaussian_blur_box_filter_nalgebra =
            gaussian_blur_integer_approximation(converted.as_view(), GAUSSIAN_SIGMA);
        let blurred = gaussian_blur_box_filter_nalgebra;
        let blurred_view = blurred.as_view();
        let gradients_y_transposed = sobel_operator_vertical::<u8>(blurred_view);
        let gradients_x_transposed = sobel_operator_horizontal::<u8>(blurred_view);

        let guard = get_profiler_guard();

        // let magnitudes = gradients_x_transposed.zip_map(&gradients_y_transposed, |x, y| {
        //     (x * x) as i32 + (y * y) as i32
        // });
        // let threshold = (magnitudes.sum() as f32 / magnitudes.len() as f32).sqrt() as u16;

        bencher.bench_local(move || {
            black_box(non_maximum_suppression(
                &gradients_x_transposed,
                &gradients_y_transposed,
                10,
                20,
            ));
        });
        get_flamegraph("non_maximum_suppression_our_impl", guard);
    }

    #[bench]
    fn nms_synthetic(bencher: Bencher) {
        let angles = (0..360).map(|deg| (deg as f32).to_radians());
        let (width, height) = (200, 100);

        let circle_center = (150.0, 50.0);
        let radius = 20.0;

        let gradients_x = {
            let mut mat = DMatrix::<i16>::zeros(height, width);

            for i in 20..height - 20 {
                mat[(i, 20)] = 2000;
                mat[(i, 80)] = -2000;
                for j in 40..60 {
                    mat[(i, j)] = 1000;
                }
            }

            for angle in angles.clone() {
                let x_component = radius * angle.cos();
                let x = (circle_center.0 + x_component) as usize;
                let y = (circle_center.1 + (radius * angle.sin())) as usize;
                mat[(y, x)] = (x_component * 100.0) as i16;
            }
            mat
        };
        let gradients_y = {
            let mut mat = DMatrix::<i16>::zeros(height, width);

            for i in 20..height - 20 {
                for j in 40..60 {
                    mat[(i, j)] = 1000;
                    mat[(20, j)] = 2000;
                    mat[(80, j)] = -2000;
                }
            }

            for angle in angles {
                let y_component = radius * angle.sin();
                let x = (circle_center.0 + radius * angle.cos()) as usize;
                let y = (circle_center.1 + y_component) as usize;
                mat[(y, x)] = (y_component * 100.0) as i16;
            }
            mat
        };

        bencher.bench_local(move || {
            black_box(non_maximum_suppression(&gradients_x, &gradients_y, 10, 20));
        });
    }

    #[bench]
    fn imageproc_canny(bencher: Bencher) {
        let image = load_test_image();

        let guard = get_profiler_guard();
        bencher.bench_local(move || {
            black_box(get_edges_canny_imageproc(
                black_box(3.5),
                black_box(20.0),
                black_box(50.0),
                black_box(&image),
                EDGE_SOURCE_TYPE,
                None,
            ))
        });
        get_flamegraph("edges_canny", guard);
    }
}
