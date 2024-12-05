use divan::{black_box, Bencher};
use image::open;
use imageproc::{
    edges::canny,
    filter::gaussian_blur_f32,
    gradients::{horizontal_sobel, vertical_sobel},
};

use edge_detection::{get_edge_source_image, EdgeSourceType};
use types::ycbcr422_image::YCbCr422Image;

fn main() {
    // Run registered benchmarks.
    divan::main();
}

fn load_test_image() -> YCbCr422Image {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    YCbCr422Image::load_from_rgb_file(format!("{crate_dir}/benches/data/center_circle_webots.png"))
        .unwrap()
}

// Register a `fibonacci` function and benchmark it over multiple cases.
#[divan::bench]
fn imageproc_sobel_horizontal(bencher: Bencher) {
    let image = load_test_image();
    let edges_source = get_edge_source_image(&image, EdgeSourceType::LumaOfYCbCr);
    let blurred = gaussian_blur_f32(&edges_source, 3.5);

    bencher.bench_local(move || horizontal_sobel(black_box(&blurred)));
}

#[divan::bench]
fn imageproc_sobel_vertical(bencher: Bencher) {
    let image = load_test_image();
    let edges_source = get_edge_source_image(&image, EdgeSourceType::LumaOfYCbCr);
    let blurred = gaussian_blur_f32(&edges_source, 3.5);

    bencher.bench_local(move || vertical_sobel(black_box(&blurred)));
}

// Register a `fibonacci` function and benchmark it over multiple cases.
#[divan::bench]
fn imageproc_canny(bencher: Bencher) {
    let image = load_test_image();
    let edges_source = get_edge_source_image(&image, EdgeSourceType::LumaOfYCbCr);
    let blurred = gaussian_blur_f32(&edges_source, 3.5);

    bencher.bench_local(move || canny(black_box(&blurred), 20.0, 50.0));
}

#[divan::bench]
fn imageproc_blurring(bencher: Bencher) {
    let image = load_test_image();
    let edges_source = get_edge_source_image(&image, EdgeSourceType::LumaOfYCbCr);

    bencher.bench_local(move || gaussian_blur_f32(black_box(&edges_source), 3.5));
}

#[divan::bench]
fn edge_source_select(bencher: Bencher) {
    let edge_source_types = [
        EdgeSourceType::LumaOfYCbCr,
        EdgeSourceType::DifferenceOfGrayAndRgbRange,
    ];
    let image = load_test_image();

    // edge_source_types.iter().for_each(|&edge_source_type| {
    bencher.bench_local(move || {
        get_edge_source_image(black_box(&image), black_box(EdgeSourceType::LumaOfYCbCr))
    });
    // });
}
