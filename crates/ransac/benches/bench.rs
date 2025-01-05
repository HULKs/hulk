use std::{env, fs::File};

use divan::{bench, black_box, AllocProfiler, Bencher};
use linear_algebra::{point, Point2};
use pprof::{ProfilerGuard, ProfilerGuardBuilder};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use ransac::circles::{circle::RansacCircleWithTransformation, test_utilities::generate_circle};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

fn get_profiler_guard() -> Option<ProfilerGuard<'static>> {
    if env::var("ENABLE_FLAMEGRAPH").is_ok_and(|v| v == "1") {
        ProfilerGuardBuilder::default()
            .frequency(1000)
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
            "{}/benches/output/{}.svg",
            env!("CARGO_MANIFEST_DIR"),
            file_name
        ))
        .unwrap();
        report.flamegraph(file).unwrap();
    };
}

const TYPICAL_RADIUS: f32 = 0.75;
const GENERATION_VARIATION: f32 = 0.08;
const ACCEPTED_RADIUS_VARIANCE: f32 = 0.1;
const HIGH_POINT_COUNT: usize = 2000;
const HIGH_ITERATIONS: usize = 1500;
const RANDOM_SEED: u64 = 666;

struct SomeFrame {}
struct OtherFrame {}

fn _some_to_other(v: &Point2<SomeFrame>) -> Option<Point2<OtherFrame>> {
    Some(point![v.x() + 1.0, v.y() + 1.0])
}

#[bench(min_time = 30)]
fn noisy_circle(bencher: Bencher) {
    let center = point![2.0, 1.5];
    let radius = TYPICAL_RADIUS;
    let points: Vec<Point2<SomeFrame>> = generate_circle(
        &center,
        HIGH_POINT_COUNT,
        radius,
        GENERATION_VARIATION,
        RANDOM_SEED,
    );
    let mut rng = ChaChaRng::seed_from_u64(RANDOM_SEED);

    let gen = bencher.with_inputs(|| {
        let ransac = RansacCircleWithTransformation::<SomeFrame, OtherFrame>::new(
            TYPICAL_RADIUS,
            ACCEPTED_RADIUS_VARIANCE,
            black_box(points.clone()),
            _some_to_other,
            None,
            Some(0.35),
        );
        ransac
    });
    let guard = get_profiler_guard();
    gen.bench_local_values(move |mut ransac| {
        black_box(
            ransac
                .next_candidate(black_box(&mut rng), black_box(HIGH_ITERATIONS))
                .expect("No circle was found"),
        );
    });
    get_flamegraph("ransac_noisy_circle", guard);
}
