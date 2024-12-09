use divan::{black_box, Bencher};
use image::GrayImage;
use imageproc::{edges::canny, filter::gaussian_blur_f32, gradients::sobel_gradients};

use edge_detection::{
    gaussian::gaussian_blur_box_filter_nalgebra,
    get_edge_source_image, grayimage_to_2d_transposed_matrix_view,
    sobel::{sobel_operator_horizontal, sobel_operator_vertical},
    EdgeSourceType,
};
use types::ycbcr422_image::YCbCr422Image;

fn main() {
    divan::main();
}

const GAUSSIAN_SIGMA: f32 = 1.4;
const EDGE_SOURCE_TYPE: EdgeSourceType = EdgeSourceType::LumaOfYCbCr;

fn load_test_image() -> YCbCr422Image {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    YCbCr422Image::load_from_rgb_file(format!("{crate_dir}/test_data/center_circle_webots.png"))
        .unwrap()
}

fn get_blurred_source_image(image: &YCbCr422Image) -> GrayImage {
    let edges_source = get_edge_source_image(&image, EDGE_SOURCE_TYPE);
    gaussian_blur_f32(&edges_source, 3.5)
}

#[divan::bench]
fn imageproc_sobel_gradients(bencher: Bencher) {
    let image = load_test_image();
    let blurred = get_blurred_source_image(&image);

    bencher.bench_local(move || sobel_gradients(black_box(&blurred)));
}

#[divan::bench]
fn imageproc_canny(bencher: Bencher) {
    let image = load_test_image();
    let mono = get_edge_source_image(&image, EDGE_SOURCE_TYPE);

    bencher.bench_local(move || canny(black_box(&mono), 20.0, 50.0));
}

#[divan::bench]
fn edge_source_select(bencher: Bencher) {
    let image = load_test_image();

    bencher
        .bench_local(move || get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE)));
}

#[divan::bench_group]
mod blurring {
    use divan::{black_box, Bencher};
    use edge_detection::{
        gaussian::{gaussian_blur_box_filter, gaussian_blur_box_filter_nalgebra},
        get_edge_source_image, grayimage_to_2d_transposed_matrix_view,
    };
    use imageproc::filter::gaussian_blur_f32;
    use nalgebra::DMatrix;

    use crate::{load_test_image, EDGE_SOURCE_TYPE, GAUSSIAN_SIGMA};

    #[divan::bench]
    fn gaussian_blur_with_box_filter(bencher: Bencher) {
        let image = get_edge_source_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE);
        bencher.bench_local(move || {
            gaussian_blur_box_filter(black_box(&image), black_box(GAUSSIAN_SIGMA))
        });
    }

    #[divan::bench]
    fn gaussian_blur_with_box_filter_nalgebra(bencher: Bencher) {
        let image = get_edge_source_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE);
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view(&image);
        bencher.bench_local(move || {
            gaussian_blur_box_filter_nalgebra::<u8>(
                black_box(&transposed_matrix_view),
                black_box(GAUSSIAN_SIGMA),
            )
        });
    }

    #[divan::bench]
    fn gaussian_blur_with_box_filter_nalgebra_i16_input(bencher: Bencher) {
        let image = get_edge_source_image(black_box(&load_test_image()), EDGE_SOURCE_TYPE);
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view(&image)
            .clone_owned()
            .cast();
        bencher.bench_local(move || {
            gaussian_blur_box_filter_nalgebra::<i16>(
                black_box(&transposed_matrix_view.as_view()),
                black_box(GAUSSIAN_SIGMA),
            )
        });
    }

    #[divan::bench]
    fn imageproc_blurring(bencher: Bencher) {
        let image = load_test_image();
        let edges_source = get_edge_source_image(&image, EDGE_SOURCE_TYPE);

        bencher.bench_local(move || gaussian_blur_f32(black_box(&edges_source), 3.5));
    }
}

#[divan::bench_group]
mod sobel_operator {
    use divan::{black_box, Bencher};
    use edge_detection::{
        conv::{direct_convolution, imgproc_kernel_to_matrix},
        get_edge_source_image, grayimage_to_2d_transposed_matrix_view,
        sobel::sobel_operator_vertical,
    };
    use imageproc::gradients::{vertical_sobel, HORIZONTAL_SOBEL, VERTICAL_SOBEL};
    use nalgebra::DMatrix;

    use crate::{get_blurred_source_image, load_test_image, EDGE_SOURCE_TYPE};

    #[divan::bench]
    fn direct_convolution_vertical(bencher: Bencher) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view(&gray);
        let kernel_vert = imgproc_kernel_to_matrix::<3>(&VERTICAL_SOBEL);

        bencher.bench_local(move || {
            direct_convolution(black_box(&transposed_matrix_view), black_box(&kernel_vert));
        });
    }

    #[divan::bench]
    fn direct_convolution_horizontal(bencher: Bencher) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view(&gray);
        let kernel_vert = imgproc_kernel_to_matrix::<3>(&HORIZONTAL_SOBEL);

        bencher.bench_local(move || {
            direct_convolution(black_box(&transposed_matrix_view), black_box(&kernel_vert));
        });
    }

    #[divan::bench]
    fn direct_convolution_vertical_wrapper(bencher: Bencher) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view = grayimage_to_2d_transposed_matrix_view(&gray);

        bencher.bench_local(move || {
            sobel_operator_vertical::<3, u8>(black_box(&transposed_matrix_view));
        });
    }

    #[divan::bench]
    fn direct_convolution_vertical_wrapper_i16_input(bencher: Bencher) {
        let image = load_test_image();
        let gray = get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE));
        let transposed_matrix_view: DMatrix<i16> = grayimage_to_2d_transposed_matrix_view(&gray)
            .clone_owned()
            .cast();

        bencher.bench_local(move || {
            sobel_operator_vertical::<3, i16>(black_box(&transposed_matrix_view.as_view()));
        });
    }

    #[divan::bench]
    fn imageproc_sobel_vertical(bencher: Bencher) {
        let image = load_test_image();
        let blurred = get_blurred_source_image(&image);

        bencher.bench_local(move || vertical_sobel(black_box(&blurred)));
    }
}

#[divan::bench_group]
mod edge_points {

    use std::{fs::File, ops::Div};

    use divan::{black_box, Bencher};

    use edge_detection::{
        canny::non_maximum_suppression,
        gaussian::gaussian_blur_box_filter_nalgebra,
        get_edge_source_image, get_edges_canny, grayimage_to_2d_transposed_matrix_view,
        sobel::{
            get_edges_sobel, get_edges_sobel_nalgebra, sobel_operator_horizontal,
            sobel_operator_vertical,
        },
    };

    use crate::{load_test_image, EDGE_SOURCE_TYPE, GAUSSIAN_SIGMA};

    #[divan::bench]
    fn imageproc_sobel_vertical(bencher: Bencher) {
        let image = load_test_image();

        bencher.bench_local(move || {
            get_edges_sobel(
                black_box(3.5),
                black_box(100),
                black_box(&image),
                EDGE_SOURCE_TYPE,
            )
        });
    }

    #[divan::bench]
    fn direct_convolution_sobel_both_axes(bencher: Bencher) {
        let image = load_test_image();

        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(1000)
            .blocklist(&["libc", "libgcc", "pthread", "vdso", "divan"])
            .build()
            .unwrap();

        bencher.bench_local(move || {
            get_edges_sobel_nalgebra(
                black_box(3.5),
                black_box(100),
                black_box(&image),
                EDGE_SOURCE_TYPE,
            )
        });
        if let Ok(report) = guard.report().build() {
            let file = File::create(format!(
                "{}/test_data/output/edges_sobel.svg",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap();
            report.flamegraph(file).unwrap();
        };
    }

    #[divan::bench]
    fn non_maximum_suppression_our_impl(bencher: Bencher) {
        let image = load_test_image();

        let edges_source = get_edge_source_image(&image, EDGE_SOURCE_TYPE);
        let converted = grayimage_to_2d_transposed_matrix_view(&edges_source);
        let blurred = gaussian_blur_box_filter_nalgebra(&converted, GAUSSIAN_SIGMA);

        let gradients_y_transposed = sobel_operator_vertical::<3, i16>(&blurred.as_view());
        let gradients_x_transposed = sobel_operator_horizontal::<3, i16>(&blurred.as_view());

        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(1000)
            .blocklist(&["libc", "libgcc", "pthread", "vdso", "divan"])
            .build()
            .unwrap();

        let magnitudes = gradients_x_transposed.zip_map(&gradients_y_transposed, |x, y| {
            (x * x) as i32 + (y * y) as i32
        });
        let threshold = (magnitudes.sum() as f32 / magnitudes.len() as f32).sqrt() as u16;

        bencher.bench_local(move || {
            let suppressed =
                non_maximum_suppression(&gradients_x_transposed, &gradients_y_transposed);
        });
        if let Ok(report) = guard.report().build() {
            let file = File::create(format!(
                "{}/test_data/output/non_maximum_suppression_our_impl.svg",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap();
            report.flamegraph(file).unwrap();
        };
    }

    #[divan::bench]
    fn imageproc_canny(bencher: Bencher) {
        let image = load_test_image();

        bencher.bench_local(move || {
            get_edges_canny(
                black_box(3.5),
                black_box(20.0),
                black_box(50.0),
                black_box(&image),
                EDGE_SOURCE_TYPE,
            )
        });
    }
}
