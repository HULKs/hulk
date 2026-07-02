use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat, bail},
};
use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    session::{Input, Session, builder::GraphOptimizationLevel},
    value::{DynTensor, ValueType},
};

const DEFAULT_CACHE_PATH: &str = "/home/booster/hulk/etc/neural_networks/";

#[derive(Debug, Parser)]
struct Arguments {
    /// Path to onnx model
    onnx_path: PathBuf,

    /// Path to cache folder
    #[arg(long, default_value = DEFAULT_CACHE_PATH)]
    cache_path: PathBuf,

    /// Input shapes, for example: --raw_bytes_input 224,272,6
    #[arg(value_name = "input shapes", trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..)]
    input_shapes: Vec<String>,
}

#[derive(Debug)]
struct InputShape {
    name: String,
    shape: Vec<i64>,
    dynamic: bool,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Arguments::parse();
    let shape_overrides = parse_input_shapes(args.input_shapes)?;

    std::fs::create_dir_all(&args.cache_path).wrap_err("failed to create cache path")?;

    let metadata_session = Session::builder()?.commit_from_file(&args.onnx_path)?;
    let input_shapes = resolve_input_shapes(&metadata_session.inputs, &shape_overrides)?;

    let mut tensor_rt = TensorRTExecutionProvider::default()
        .with_device_id(0)
        .with_fp16(true)
        .with_engine_cache(true)
        .with_engine_cache_path(args.cache_path.display());

    if let Some(profile_shapes) = profile_shapes(&input_shapes) {
        tensor_rt = tensor_rt
            .with_profile_min_shapes(profile_shapes.clone())
            .with_profile_opt_shapes(profile_shapes.clone())
            .with_profile_max_shapes(profile_shapes);
    }

    let cuda = CUDAExecutionProvider::default().build();
    let tensor_rt = tensor_rt.build().error_on_failure();
    let mut session = Session::builder()?
        .with_execution_providers([tensor_rt, cuda])?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(2)?
        .commit_from_file(args.onnx_path)?;

    let input_shapes = input_shapes
        .into_iter()
        .map(|input| (input.name, input.shape))
        .collect::<HashMap<_, _>>();
    let inputs = session
        .inputs
        .iter()
        .map(|input| create_dummy_input(&session, input, &input_shapes))
        .collect::<Result<Vec<_>>>()?;

    session.run(inputs)?;
    eprintln!("object detection setup complete");
    Ok(())
}

fn parse_input_shapes(tokens: Vec<String>) -> Result<HashMap<String, Vec<i64>>> {
    let mut shapes = HashMap::new();
    let mut tokens = tokens.into_iter();

    while let Some(token) = tokens.next() {
        if !token.starts_with("--") {
            bail!("expected input shape option, got '{token}'");
        }

        let (name, shape) = if let Some((name, shape)) = token[2..].split_once('=') {
            (name.to_string(), shape.to_string())
        } else {
            let name = token[2..].to_string();
            let shape = tokens
                .next()
                .wrap_err_with(|| format!("missing shape for input '{name}'"))?;
            (name, shape)
        };

        if name.is_empty() {
            bail!("expected input name after '--'");
        }
        if shapes.insert(name.clone(), parse_shape(&shape)?).is_some() {
            bail!("shape for input '{name}' was specified more than once");
        }
    }

    Ok(shapes)
}

fn parse_shape(shape: &str) -> Result<Vec<i64>> {
    shape
        .split(',')
        .map(|dimension| {
            let dimension = dimension
                .parse::<i64>()
                .wrap_err_with(|| format!("invalid shape dimension '{dimension}'"))?;
            if dimension <= 0 {
                bail!("shape dimensions must be positive, got {dimension}");
            }
            Ok(dimension)
        })
        .collect()
}

fn resolve_input_shapes(
    inputs: &[Input],
    overrides: &HashMap<String, Vec<i64>>,
) -> Result<Vec<InputShape>> {
    let mut resolved = Vec::new();
    let mut unused_overrides = overrides.keys().cloned().collect::<Vec<_>>();

    for input in inputs {
        unused_overrides.retain(|name| name != &input.name);
        let model_shape = tensor_shape(input)?;
        let dynamic = model_shape.iter().any(|dimension| *dimension < 0);
        let shape = match overrides.get(&input.name) {
            Some(shape) => validate_shape(&input.name, model_shape, shape)?,
            None if dynamic => bail!(
                "input '{}' has dynamic shape {}; pass --{} dim1,dim2,...",
                input.name,
                format_shape(model_shape),
                input.name
            ),
            None => model_shape.to_vec(),
        };
        resolved.push(InputShape {
            name: input.name.clone(),
            shape,
            dynamic,
        });
    }

    if !unused_overrides.is_empty() {
        unused_overrides.sort();
        bail!(
            "shape specified for unknown input(s): {}",
            unused_overrides.join(", ")
        );
    }
    Ok(resolved)
}

fn tensor_shape(input: &Input) -> Result<&[i64]> {
    let ValueType::Tensor { shape, .. } = &input.input_type else {
        bail!(
            "input '{}' is not a tensor: {:?}",
            input.name,
            input.input_type
        );
    };
    Ok(shape)
}

fn validate_shape(name: &str, model_shape: &[i64], shape: &[i64]) -> Result<Vec<i64>> {
    if shape.len() != model_shape.len() {
        bail!(
            "shape for input '{}' has rank {}, expected {} from model shape {}",
            name,
            shape.len(),
            model_shape.len(),
            format_shape(model_shape)
        );
    }

    for (index, (&model_dimension, &dimension)) in model_shape.iter().zip(shape).enumerate() {
        if model_dimension >= 0 && model_dimension != dimension {
            bail!(
                "shape for input '{}' has dimension {} = {}, expected {} from model shape {}",
                name,
                index,
                dimension,
                model_dimension,
                format_shape(model_shape)
            );
        }
    }
    Ok(shape.to_vec())
}

fn create_dummy_input(
    session: &Session,
    input: &Input,
    shapes: &HashMap<String, Vec<i64>>,
) -> Result<(String, DynTensor)> {
    let ValueType::Tensor { ty, .. } = &input.input_type else {
        bail!(
            "input '{}' is not a tensor: {:?}",
            input.name,
            input.input_type
        );
    };
    Ok((
        input.name.clone(),
        DynTensor::new(
            session.allocator(),
            *ty,
            shapes
                .get(&input.name)
                .wrap_err_with(|| format!("missing resolved shape for '{}'", input.name))?
                .clone(),
        )?,
    ))
}

fn profile_shapes(inputs: &[InputShape]) -> Option<String> {
    let profiles = inputs
        .iter()
        .filter(|input| input.dynamic)
        .map(|input| format!("{}:{}", input.name, profile_shape(&input.shape)))
        .collect::<Vec<_>>();
    (!profiles.is_empty()).then(|| profiles.join(","))
}

fn profile_shape(shape: &[i64]) -> String {
    shape
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join("x")
}

fn format_shape(shape: &[i64]) -> String {
    format!(
        "[{}]",
        shape
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )
}
