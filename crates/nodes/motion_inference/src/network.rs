use crate::config::{Parameters, Policy};
use color_eyre::eyre::{Result, WrapErr, ensure, eyre};
use ort::{
    session::Session,
    value::{Tensor, TensorElementType, ValueType},
};
use std::path::Path;

pub struct Network {
    session: Session,
    policy: Policy,
}

impl Network {
    pub fn load(root: &Path, policy: Policy, parameters: &Parameters) -> Result<Self> {
        let configured = parameters
            .policies
            .get(&policy)
            .ok_or_else(|| eyre!("missing parameters for {policy:?}"))?;
        ensure!(
            parameters.inference_threads > 0,
            "inference_threads must be positive"
        );
        let path = root.join(&configured.model_file);
        let session = Session::builder()?
            .with_intra_threads(parameters.inference_threads)
            .map_err(ort::Error::<()>::from)?
            .with_inter_threads(parameters.inference_threads)
            .map_err(ort::Error::<()>::from)?
            .commit_from_file(&path)
            .wrap_err_with(|| format!("loading {}", path.display()))?;
        ensure!(
            session.inputs().len() == 1 && session.outputs().len() == 1,
            "{policy:?}: expected one input and output"
        );
        let (input, output) = policy.dimensions();
        for (name, dtype, width) in [
            (
                session.inputs()[0].name(),
                session.inputs()[0].dtype(),
                input,
            ),
            (
                session.outputs()[0].name(),
                session.outputs()[0].dtype(),
                output,
            ),
        ] {
            let valid = matches!(dtype, ValueType::Tensor { ty: TensorElementType::Float32, shape, .. }
                if shape.len() == 2 && (shape[0] == -1 || shape[0] == 1) && shape[1] == width as i64);
            ensure!(valid, "{policy:?}: incompatible tensor {name}: {dtype:?}");
        }
        Ok(Self { session, policy })
    }

    pub fn run(&mut self, observation: &[f32]) -> Result<Vec<f32>> {
        let (input_size, output_size) = self.policy.dimensions();
        ensure!(
            observation.len() == input_size,
            "{:?}: expected {input_size} inputs, got {}",
            self.policy,
            observation.len()
        );
        ensure!(
            observation.iter().all(|x| x.is_finite()),
            "non-finite observation"
        );
        let input = Tensor::from_array(([1usize, input_size], observation.to_vec()))?;
        let outputs = self.session.run(ort::inputs![input])?;
        let (shape, values) = outputs[0].try_extract_tensor::<f32>()?;
        ensure!(
            shape.as_ref() == [1, output_size as i64] && values.len() == output_size,
            "unexpected output shape"
        );
        ensure!(
            values.iter().all(|v| v.is_finite()),
            "non-finite network output"
        );
        Ok(values.to_vec())
    }
}
