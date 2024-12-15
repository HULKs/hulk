use std::{env, fs::File};

use divan::{bench, bench_group, black_box, AllocProfiler, Bencher};
use image::GrayImage;
use imageproc::{edges::canny, filter::gaussian_blur_f32, gradients::sobel_gradients};

use edge_detection::{get_edge_source_image, EdgeSourceType};
use pprof::{ProfilerGuard, ProfilerGuardBuilder};
use types::ycbcr422_image::YCbCr422Image;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
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
    let edges_source = get_edge_source_image(&image, EDGE_SOURCE_TYPE);
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
    let image = load_test_image();
    let mono = get_edge_source_image(&image, EDGE_SOURCE_TYPE);

    bencher.bench_local(move || canny(black_box(&mono), 20.0, 50.0));
}

#[bench]
fn edge_source_select(bencher: Bencher) {
    let image = load_test_image();

    bencher
        .bench_local(move || get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE)));
}

#[bench_group]
mod blurring {
    use divan::{bench, black_box, Bencher};
    use edge_detection::{
        gaussian::{
            gaussian_blur_box_filter, gaussian_blur_box_filter_nalgebra,
            gaussian_blur_try_2_nalgebra,
        },
        get_edge_source_image, grayimage_to_2d_transposed_matrix_view,
    };
    use imageproc::filter::gaussian_blur_f32;

    use crate::{get_flamegraph, get_profiler_guard, load_test_image, EDGE_SOURCE_TYPE};

    const GAUSSIAN_VALUES: &[f32] = &[1.0, 1.4, 2.0, 3.5];

    #[bench(args=GAUSSIAN_VALUES)]
    fn gaussian_blur_with_box_filter(bencher: Bencher, sigma: f32) {
        let image = get_edge_source_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE);
        bencher.bench_local(move || {
            black_box(gaussian_blur_box_filter(
                black_box(&image),
                black_box(sigma),
            ))
        });
    }

    #[bench(args=GAUSSIAN_VALUES)]
    fn gaussian_blur_with_box_filter_nalgebra(bencher: Bencher, sigma: f32) {
        let image = get_edge_source_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE);
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view(&image);
        bencher.bench_local(move || {
            black_box(gaussian_blur_box_filter_nalgebra::<u8>(
                black_box(&transposed_matrix_view),
                black_box(sigma),
            ))
        });
    }

    #[bench(args=GAUSSIAN_VALUES)]
    fn gaussian_blur_with_box_filter_nalgebra_i16_input(bencher: Bencher, sigma: f32) {
        let image = get_edge_source_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE);
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view::<i16>(&image);
        bencher.bench_local(move || {
            black_box(gaussian_blur_box_filter_nalgebra::<i16>(
                black_box(&transposed_matrix_view),
                black_box(sigma),
            ))
        });
    }

    #[bench(args=GAUSSIAN_VALUES)]
    fn gaussian_blur_int_approximation(bencher: Bencher, sigma: f32) {
        let image = get_edge_source_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE);
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view::<u8>(&image);

        let guard = if sigma == 1.0 {
            get_profiler_guard()
        } else {
            None
        };
        bencher.bench_local(move || {
            black_box(gaussian_blur_try_2_nalgebra::<u8>(
                black_box(&transposed_matrix_view),
                black_box(sigma),
            ))
        });
        get_flamegraph("int_approx", guard);
    }

    #[bench(args=GAUSSIAN_VALUES)]
    fn imageproc_blurring(bencher: Bencher, sigma: f32) {
        let image = load_test_image();
        let edges_source = get_edge_source_image(&image, EDGE_SOURCE_TYPE);

        bencher.bench_local(move || black_box(gaussian_blur_f32(black_box(&edges_source), sigma)));
    }
}

#[bench_group]
mod sobel_operator {

    use std::num::NonZeroU32;

    use divan::{bench, black_box, Bencher};
    use edge_detection::{
        conv::{
            direct_convolution, direct_convolution_mut, imgproc_kernel_to_matrix,
            piecewise_2d_convolution_mut, piecewise_horizontal_convolution_mut,
            piecewise_vertical_convolution_mut,
        },
        get_edge_source_image, grayimage_to_2d_transposed_matrix_view,
        sobel::sobel_operator_vertical,
    };
    use imageproc::gradients::{vertical_sobel, VERTICAL_SOBEL};
    use nalgebra::DMatrix;

    use crate::{
        get_blurred_source_image, get_flamegraph, get_profiler_guard, load_test_image,
        EDGE_SOURCE_TYPE,
    };

    #[bench(args=[NonZeroU32::new(5).unwrap(),NonZeroU32::new(2).unwrap(),NonZeroU32::new(1).unwrap()])]
    fn direct_convolution_vertical(bencher: Bencher, scale_factor: NonZeroU32) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view::<u8>(&gray);
        let kernel_vert = imgproc_kernel_to_matrix::<3>(&VERTICAL_SOBEL);

        bencher.bench_local(move || {
            black_box(direct_convolution::<3, u8, i32, i16>(
                black_box(&transposed_matrix_view),
                black_box(&kernel_vert),
                black_box(scale_factor),
            ));
        });
    }

    #[bench(args=[NonZeroU32::new(5).unwrap(),NonZeroU32::new(2).unwrap(),NonZeroU32::new(1).unwrap()])]
    fn direct_convolution_mut_old_vertical(bencher: Bencher, scale_factor: NonZeroU32) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view::<u8>(&gray);
        let kernel_vert = imgproc_kernel_to_matrix::<3>(&VERTICAL_SOBEL);

        bencher.bench_local(move || {
            let mut out = vec![0i16; transposed_matrix_view.len()];
            black_box(direct_convolution_mut::<3, u8, i32, i16>(
                black_box(&transposed_matrix_view),
                black_box(out.as_mut_slice()),
                black_box(&kernel_vert),
                black_box(scale_factor),
            ));
        });
    }

    #[bench]
    fn piecewise_2d_mut_sobel(bencher: Bencher) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view::<u8>(&gray);

        let kernel_vertical = [1, 2, 1];
        let kernel_horizontal = [-1, 0, 1];

        let guard = get_profiler_guard();
        bencher.bench_local(move || {
            let mut out = vec![0i16; transposed_matrix_view.len()];
            black_box(piecewise_2d_convolution_mut::<3, u8, i32, i16>(
                black_box(&transposed_matrix_view),
                black_box(out.as_mut_slice()),
                black_box(&kernel_horizontal),
                black_box(&kernel_vertical),
            ));
        });

        get_flamegraph("piecewise_horiz", guard);
    }

    #[bench]
    fn piecewise_vertical_mut_sobel(bencher: Bencher) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view::<u8>(&gray);

        let kernel_vertical = [1, 2, 1];

        bencher.bench_local(move || {
            let mut out = vec![0i16; transposed_matrix_view.len()];
            black_box(piecewise_vertical_convolution_mut::<3, u8, i32, i16>(
                black_box(&transposed_matrix_view),
                black_box(out.as_mut_slice()),
                black_box(&kernel_vertical),
            ));
        });
    }

    #[bench]
    fn piecewise_horizontal_mut_sobel(bencher: Bencher) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view::<u8>(&gray);

        let kernel_horizontal = [-1, 0, 1];

        bencher.bench_local(move || {
            let mut out = vec![0i16; transposed_matrix_view.len()];
            black_box(piecewise_horizontal_convolution_mut::<3, u8, i32, i16>(
                black_box(&transposed_matrix_view),
                black_box(out.as_mut_slice()),
                black_box(&kernel_horizontal),
            ));
        });
    }

    #[bench]
    fn direct_convolution_sobel_vertical_wrapper(bencher: Bencher) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view(&gray);

        bencher.bench_local(move || {
            black_box(sobel_operator_vertical::<3, u8>(black_box(
                &transposed_matrix_view,
            )));
        });
    }

    #[bench]
    fn direct_convolution_sobel_vertical_wrapper_i16_input(bencher: Bencher) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view: DMatrix<i16> =
            grayimage_to_2d_transposed_matrix_view::<i16>(&gray);

        bencher.bench_local(move || {
            black_box(sobel_operator_vertical::<3, i16>(black_box(
                &transposed_matrix_view,
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
        gaussian::gaussian_blur_box_filter_nalgebra,
        get_edge_source_image, get_edges_canny, get_edges_canny_imageproc,
        grayimage_to_2d_transposed_matrix_view,
        sobel::{
            get_edges_sobel, get_edges_sobel_nalgebra, sobel_operator_horizontal,
            sobel_operator_vertical,
        },
    };
    use nalgebra::DMatrix;

    use crate::{
        get_flamegraph, get_profiler_guard, load_test_image, EDGE_SOURCE_TYPE, GAUSSIAN_SIGMA,
    };

    #[bench]
    fn our_canny(bencher: Bencher) {
        let image = load_test_image();

        let guard = get_profiler_guard();
        bencher.bench_local(move || {
            black_box(get_edges_canny(
                black_box(3.5),
                black_box(20.0),
                black_box(50.0),
                black_box(&image),
                EDGE_SOURCE_TYPE,
            ))
        });
        get_flamegraph("edges_our_canny", guard);
    }

    #[bench]
    fn imageproc_sobel_vertical(bencher: Bencher) {
        let image = load_test_image();

        bencher.bench_local(move || {
            black_box(get_edges_sobel(
                black_box(3.5),
                black_box(100),
                black_box(&image),
                EDGE_SOURCE_TYPE,
            ))
        });
    }

    #[bench]
    fn direct_convolution_sobel_both_axes(bencher: Bencher) {
        let image = load_test_image();

        let guard = get_profiler_guard();
        bencher.bench_local(move || {
            black_box(get_edges_sobel_nalgebra(
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

        let edges_source = get_edge_source_image(&image, EDGE_SOURCE_TYPE);
        let converted = grayimage_to_2d_transposed_matrix_view::<i16>(&edges_source);
        let blurred = gaussian_blur_box_filter_nalgebra(&converted, GAUSSIAN_SIGMA);

        let gradients_y_transposed = sobel_operator_vertical::<3, i16>(&blurred);
        let gradients_x_transposed = sobel_operator_horizontal::<3, i16>(&blurred);

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
            ))
        });
        get_flamegraph("edges_canny", guard);
    }
}
