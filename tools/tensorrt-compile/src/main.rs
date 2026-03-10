use std::path::PathBuf;

use clap::Parser;
use color_eyre::{Result, eyre::Context};
use ndarray::Array3;
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    inputs,
    session::{Session, SessionOutputs, builder::GraphOptimizationLevel},
    value::TensorRef,
};

#[derive(Debug, Parser)]
struct CliArguments {
    /// Path to onnx model
    onnx_path: PathBuf,

    /// Path to cache folder
    #[arg(long, default_value = "/home/booster/.cache/hulk/tensor-rt")]
    cache_path: PathBuf,
}

fn main() -> Result<()> {
    const IMAGE_WIDTH: usize = 554;
    const IMAGE_HEIGHT: usize = 448;

    let args = CliArguments::parse();
    color_eyre::install()?;
    std::fs::create_dir_all(&args.cache_path).wrap_err("failed to create cache path")?;

    let tensor_rt = TensorRTExecutionProvider::default()
        .with_device_id(0)
        .with_fp16(true)
        .with_engine_cache(true)
        .with_engine_cache_path(args.cache_path.display())
        .build()
        .error_on_failure();
    let cuda = CUDAExecutionProvider::default().build();

    let mut session = Session::builder()?
        .with_execution_providers([tensor_rt, cuda])?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(2)?
        .commit_from_file(args.onnx_path)?;

    let sample_image = Array3::<u8>::default([IMAGE_HEIGHT / 2, IMAGE_WIDTH / 2, 6]);
    let outputs: SessionOutputs = session
        .run(inputs!["raw_bytes_input" => TensorRef::from_array_view(sample_image.view())?])?;
    let _ = outputs["network_detections"]
        .try_extract_array::<f32>()?
        .t()
        .into_owned();
    eprintln!("object detection setup complete");

    Ok(())
}
