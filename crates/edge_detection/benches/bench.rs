use divan::{black_box, Bencher};
use image::{open, GrayImage};
use imageproc::{
    edges::canny,
    filter::gaussian_blur_f32,
    gradients::{horizontal_sobel, sobel_gradients, vertical_sobel},
};

use edge_detection::{
    get_edge_image_canny, get_edge_source_image, get_edges_sobel, EdgeSourceType,
};
use types::ycbcr422_image::YCbCr422Image;

fn main() {
    divan::main();
}

const EDGE_SOURCE_TYPE: EdgeSourceType = EdgeSourceType::LumaOfYCbCr;

fn load_test_image() -> YCbCr422Image {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    YCbCr422Image::load_from_rgb_file(format!("{crate_dir}/benches/data/center_circle_webots.png"))
        .unwrap()
}

fn get_blurred_source_image(image: &YCbCr422Image) -> GrayImage {
    let edges_source = get_edge_source_image(&image, EDGE_SOURCE_TYPE);
    gaussian_blur_f32(&edges_source, 3.5)
}

#[divan::bench]
fn imageproc_sobel_horizontal(bencher: Bencher) {
    let image = load_test_image();
    let blurred = get_blurred_source_image(&image);

    bencher.bench_local(move || horizontal_sobel(black_box(&blurred)));
}

#[divan::bench]
fn imageproc_sobel_vertical(bencher: Bencher) {
    let image = load_test_image();
    let blurred = get_blurred_source_image(&image);
    bencher.bench_local(move || vertical_sobel(black_box(&blurred)));
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
    let blurred = get_blurred_source_image(&image);

    bencher.bench_local(move || canny(black_box(&blurred), 20.0, 50.0));
}

#[divan::bench]
fn imageproc_blurring(bencher: Bencher) {
    let image = load_test_image();
    let edges_source = get_edge_source_image(&image, EDGE_SOURCE_TYPE);

    bencher.bench_local(move || gaussian_blur_f32(black_box(&edges_source), 3.5));
}

#[divan::bench]
fn edge_source_select(bencher: Bencher) {
    let edge_source_types = [
        EdgeSourceType::LumaOfYCbCr,
        EdgeSourceType::DifferenceOfGrayAndRgbRange,
    ];
    let image = load_test_image();

    bencher
        .bench_local(move || get_edge_source_image(black_box(&image), black_box(EDGE_SOURCE_TYPE)));
}

#[divan::bench]
fn edge_image_canny(bencher: Bencher) {
    let image = load_test_image();

    bencher.bench_local(move || {
        get_edge_image_canny(3.5, 20.0, 50.0, black_box(&image), EDGE_SOURCE_TYPE)
    });
}

#[divan::bench]
fn edge_image_sobel(bencher: Bencher) {
    let image = load_test_image();

    bencher.bench_local(move || get_edges_sobel(3.5, 100, black_box(&image), EDGE_SOURCE_TYPE));
}
