use std::path::{Path, PathBuf};

use clap::Parser;
use color_eyre::Result;
use ndarray::Array4;
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    inputs,
    session::{Session, SessionOutputs, builder::GraphOptimizationLevel},
    value::TensorRef,
};

#[derive(Debug, Parser)]
pub struct CliArguments {
    /// Paths to onnx models
    #[arg(required = true, num_args = 1..)]
    pub onnx_paths: Vec<PathBuf>,

    /// Path to cache folder
    #[arg(long, default_value = "/home/booster/hulk/etc/neural_networks/")]
    pub cache_path: PathBuf,

    /// Warmup inferences before measuring
    #[arg(long, default_value_t = 10)]
    pub warmup: usize,

    /// Measured inferences
    #[arg(short, long, default_value_t = 100)]
    pub iterations: usize,

    /// Print benchmark results as JSON
    #[arg(long)]
    pub json: bool,

    /// Write benchmark results to this file instead of stdout
    #[arg(long)]
    pub output: Option<PathBuf>,

    /// Whether to benchmark all networks in parallel
    #[arg(long)]
    pub parallel: bool,
}

pub fn run_inference<'a>(
    session: &'a mut Session,
    sample_image: &Array4<f32>,
) -> Result<SessionOutputs<'a>> {
    Ok(session.run(inputs!["images" => TensorRef::from_array_view(sample_image.view())?])?)
}

pub fn setup(
    onnx_path: impl AsRef<Path>,
    cache_path: impl AsRef<Path>,
) -> Result<Session, color_eyre::eyre::Error> {
    let tensor_rt = TensorRTExecutionProvider::default()
        .with_device_id(0)
        .with_fp16(true)
        .with_engine_cache(true)
        .with_engine_cache_path(cache_path.as_ref().display())
        .build()
        .error_on_failure();
    let cuda = CUDAExecutionProvider::default().build();
    let session = Session::builder()?
        .with_execution_providers([tensor_rt, cuda])?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(2)?
        .commit_from_file(onnx_path.as_ref())?;
    Ok(session)
}

pub fn sample_image() -> Array4<f32> {
    const IMAGE_WIDTH: usize = 544;
    const IMAGE_HEIGHT: usize = 448;
    Array4::<f32>::default([1, 3, IMAGE_HEIGHT, IMAGE_WIDTH])
}
