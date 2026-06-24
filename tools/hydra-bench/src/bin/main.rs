use std::{
    hint::black_box,
    path::Path,
    sync::{Arc, Barrier, Mutex},
    thread,
    time::{Duration, Instant},
};

use clap::Parser;
use color_eyre::{Result, eyre::Context, eyre::ensure};
use hydra_bench::{CliArguments, run_inference, sample_image, setup};
use ort::session::Session;
use serde::Serialize;

#[derive(Serialize)]
struct BenchmarkReport {
    warmup: usize,
    iterations: usize,
    results: Vec<BenchmarkResult>,
}

#[derive(Serialize)]
struct BenchmarkResult {
    onnx_path: String,
    count: usize,
    avg_ms: f64,
    p50_ms: f64,
    p90_ms: f64,
    p99_ms: f64,
    min_ms: f64,
    max_ms: f64,
    latencies_ms: Vec<f64>,
}

fn main() -> Result<()> {
    let args = CliArguments::parse();
    color_eyre::install()?;
    std::fs::create_dir_all(&args.cache_path).wrap_err("failed to create cache path")?;

    let warmup = args.warmup;
    let iterations = args.iterations;
    ensure!(iterations > 0, "iterations must be > 0");

    let mut results = Vec::with_capacity(args.onnx_paths.len());
    if args.parallel {
        let barrier = Barrier::new(args.onnx_paths.len());
        let results = Arc::new(Mutex::new(&mut results));
        thread::scope(|scope| {
            for onnx_path in &args.onnx_paths {
                let session = setup(onnx_path, &args.cache_path).unwrap();
                scope.spawn(|| {
                    let result = benchmark_model(&args, onnx_path, &barrier, session).unwrap();
                    results.lock().unwrap().push(result)
                });
            }
        })
    } else {
        let barrier = Barrier::new(1);
        for onnx_path in &args.onnx_paths {
            let session = setup(onnx_path, &args.cache_path)?;
            results.push(benchmark_model(&args, onnx_path, &barrier, session)?);
        }
    }

    if args.json {
        let report = BenchmarkReport {
            warmup,
            iterations,
            results,
        };
        if let Some(output) = args.output {
            if let Some(parent) = output
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                std::fs::create_dir_all(parent).wrap_err("failed to create output directory")?;
            }
            let file = std::fs::File::create(&output)
                .wrap_err_with(|| format!("failed to create output file {}", output.display()))?;
            serde_json::to_writer_pretty(file, &report).wrap_err("failed to write JSON")?;
        } else {
            serde_json::to_writer_pretty(std::io::stdout(), &report)
                .wrap_err("failed to write JSON")?;
            println!();
        }
    } else {
        for result in &results {
            println!(
                "onnx_path={} count={} avg_ms={} p50_ms={} p90_ms={} p99_ms={} min_ms={} max_ms={}",
                result.onnx_path,
                result.count,
                result.avg_ms,
                result.p50_ms,
                result.p90_ms,
                result.p99_ms,
                result.min_ms,
                result.max_ms,
            );
        }
    }

    Ok(())
}

fn benchmark_model(
    args: &CliArguments,
    onnx_path: &Path,
    barrier: &Barrier,
    mut session: Session,
) -> Result<BenchmarkResult> {
    let sample_image = sample_image();

    for _ in 0..args.warmup {
        drop(run_inference(&mut session, black_box(&sample_image))?);
    }

    let iterations = args.iterations;
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        barrier.wait();
        let start = Instant::now();
        drop(run_inference(&mut session, black_box(&sample_image))?);
        let elapsed = start.elapsed();
        latencies.push(elapsed);
    }

    latencies.sort_unstable();
    let avg = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    let result = BenchmarkResult {
        onnx_path: onnx_path.display().to_string(),
        count: latencies.len(),
        avg_ms: ms(avg),
        p50_ms: ms(percentile(&latencies, 50.0)),
        p90_ms: ms(percentile(&latencies, 90.0)),
        p99_ms: ms(percentile(&latencies, 99.0)),
        min_ms: ms(latencies[0]),
        max_ms: ms(latencies[latencies.len() - 1]),
        latencies_ms: latencies.iter().copied().map(ms).collect(),
    };

    Ok(result)
}

fn percentile(sorted: &[Duration], percentile: f64) -> Duration {
    let index = ((percentile / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[index]
}

fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
