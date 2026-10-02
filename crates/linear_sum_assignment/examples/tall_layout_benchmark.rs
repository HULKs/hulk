//! Compares the implemented logical-transpose path with a reusable contiguous-transpose model.
//!
//! The buffered measurement includes copying into a preallocated ndarray before running the same
//! normalized assignment kernel.

use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use linear_sum_assignment::{AssignmentSolver, Objective};
use ndarray::Array2;

const SAMPLE_COUNT: usize = 7;
const SAMPLE_TARGET: Duration = Duration::from_millis(150);

fn main() {
    println!("shape,strided_us,buffered_us,transpose_us,buffered/strided");

    for shape in [
        (64, 8),
        (128, 16),
        (256, 32),
        (512, 64),
        (1024, 32),
        (2048, 128),
        (4096, 128),
    ] {
        benchmark_shape(shape);
    }
}

fn benchmark_shape((rows, columns): (usize, usize)) {
    assert!(rows > columns);

    let costs = Array2::from_shape_fn((rows, columns), |(row, column)| {
        let mut value = (row as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (column as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value ^= value >> 30;
        value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value ^= value >> 27;
        (value % 1_000_003) as f32 / 1024.0
    });

    let mut strided_solver = AssignmentSolver::new((rows, columns));
    let mut buffered_solver = AssignmentSolver::new((columns, rows));
    let mut normalized = Array2::zeros((columns, rows));

    let strided = measure(|| {
        let assignment = strided_solver
            .solve(costs.view(), Objective::Minimize)
            .unwrap();
        black_box(assignment.as_slice());
    });

    let buffered = measure(|| {
        normalized.assign(&costs.t());
        let assignment = buffered_solver
            .solve(normalized.view(), Objective::Minimize)
            .unwrap();
        black_box(assignment.as_slice());
    });

    let transpose = measure(|| {
        normalized.assign(&costs.t());
        black_box(&normalized);
    });

    println!(
        "{rows}x{columns},{:.3},{:.3},{:.3},{:.3}",
        micros(strided),
        micros(buffered),
        micros(transpose),
        buffered.as_secs_f64() / strided.as_secs_f64(),
    );
}

fn measure(mut operation: impl FnMut()) -> Duration {
    for _ in 0..5 {
        operation();
    }

    let probe_start = Instant::now();
    for _ in 0..3 {
        operation();
    }
    let probe = probe_start.elapsed() / 3;
    let iterations = (SAMPLE_TARGET.as_secs_f64() / probe.as_secs_f64())
        .round()
        .clamp(3.0, 100_000.0) as usize;

    let mut samples = [Duration::ZERO; SAMPLE_COUNT];
    for sample in &mut samples {
        let start = Instant::now();
        for _ in 0..iterations {
            operation();
        }
        *sample = start.elapsed() / iterations as u32;
    }
    samples.sort_unstable();
    samples[SAMPLE_COUNT / 2]
}

fn micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000_000.0
}
