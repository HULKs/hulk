use std::{marker::PhantomData, path::Path};

use color_eyre::{
    Result,
    eyre::{ContextCompat, bail, ensure},
};

use ort::{
    execution_providers::{CUDAExecutionProvider, TensorRTExecutionProvider},
    inputs,
    session::{
        HasSelectedOutputs, RunOptions, Session, SessionOutputs, builder::GraphOptimizationLevel,
        run_options::OutputSelector,
    },
    tensor::PrimitiveTensorElementType,
    value::{TensorRef, ValueType},
};
use ros2::sensor_msgs::image::Image;
use types::stereo_image_pair::StereoImagePair;

pub const KEYPOINTS: usize = 512;
const DESCRIPTOR_DIMENSION: usize = 64;

pub struct FeatureExtractor {
    session: Session,
    run_options: RunOptions<HasSelectedOutputs>,
}

pub struct FeatureOutput<'a> {
    outputs: SessionOutputs<'a>,
}

pub struct PreviousFeatureState {
    keypoints: Vec<f32>,
    descriptors: Vec<f32>,
    valid: Vec<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct PreviousLeft;

#[derive(Clone, Copy, Debug)]
pub struct CurrentLeft;

#[derive(Clone, Copy, Debug)]
pub struct CurrentRight;

#[derive(Clone, Copy, Debug)]
pub struct FrameFeatures<'a, Frame> {
    keypoints: &'a [f32],
    valid: &'a [bool],
    _frame: PhantomData<Frame>,
}

#[derive(Clone, Copy, Debug)]
pub struct FrameKeypoints<'a, Frame> {
    keypoints: &'a [f32],
    _frame: PhantomData<Frame>,
}

#[derive(Clone, Copy, Debug)]
pub struct Matches<'a, From, To> {
    matches: &'a [i32],
    scores: &'a [f32],
    _frames: PhantomData<(From, To)>,
}

impl FeatureExtractor {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let parent = path.as_ref().parent().wrap_err("failed to find parent")?;
        let tensorrt = TensorRTExecutionProvider::default()
            .with_device_id(0)
            .with_fp16(true)
            .with_engine_cache(true)
            .with_engine_cache_path(parent.display())
            .build();
        let cuda = CUDAExecutionProvider::default().build();
        let mut session = Session::builder()?
            .with_execution_providers([tensorrt, cuda])?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(2)?
            .commit_from_file(path)?;

        let run_options = RunOptions::new()?.with_outputs(
            OutputSelector::no_default()
                .with("current_left_keypoints")
                .with("current_left_descriptors")
                .with("current_left_valid")
                .with("current_right_keypoints")
                .with("stereo_matches")
                .with("stereo_scores")
                .with("temporal_matches")
                .with("temporal_scores"),
        );
        warm_up_session(&mut session, &run_options)?;

        Ok(Self {
            session,
            run_options,
        })
    }

    pub fn extract<'a>(
        &'a mut self,
        current: &StereoImagePair,
        previous: &PreviousFeatureState,
    ) -> Result<FeatureOutput<'a>> {
        check_stereo_pair_support(current)?;

        let current_left = image_tensor(&current.left)?;
        let current_right = image_tensor(&current.right)?;
        let previous_left_keypoints = previous.keypoints_tensor()?;
        let previous_left_descriptors = previous.descriptors_tensor()?;
        let previous_left_valid = previous.valid_tensor()?;

        let outputs = self.session.run_with_options(
            inputs![
                "current_left" => current_left,
                "current_right" => current_right,
                "previous_left_keypoints" => previous_left_keypoints,
                "previous_left_descriptors" => previous_left_descriptors,
                "previous_left_valid" => previous_left_valid,
            ],
            &self.run_options,
        )?;

        Ok(FeatureOutput { outputs })
    }
}

fn warm_up_session(
    session: &mut Session,
    run_options: &RunOptions<HasSelectedOutputs>,
) -> Result<()> {
    let current_left_shape = input_shape(session, "current_left")?;
    let current_right_shape = input_shape(session, "current_right")?;
    let current_left_data = vec![0_u8; current_left_shape.iter().product()];
    let current_right_data = vec![0_u8; current_right_shape.iter().product()];
    let previous = PreviousFeatureState::new();

    let current_left =
        TensorRef::from_array_view((current_left_shape, current_left_data.as_slice()))?;
    let current_right =
        TensorRef::from_array_view((current_right_shape, current_right_data.as_slice()))?;
    let previous_left_keypoints = previous.keypoints_tensor()?;
    let previous_left_descriptors = previous.descriptors_tensor()?;
    let previous_left_valid = previous.valid_tensor()?;

    session.run_with_options(
        inputs![
            "current_left" => current_left,
            "current_right" => current_right,
            "previous_left_keypoints" => previous_left_keypoints,
            "previous_left_descriptors" => previous_left_descriptors,
            "previous_left_valid" => previous_left_valid,
        ],
        run_options,
    )?;

    Ok(())
}

fn input_shape(session: &Session, name: &str) -> Result<[usize; 3]> {
    let input = session
        .inputs
        .iter()
        .find(|input| input.name == name)
        .wrap_err_with(|| format!("model input '{name}' is missing"))?;
    let ValueType::Tensor { shape, .. } = &input.input_type else {
        bail!(
            "model input '{name}' is not a tensor: {:?}",
            input.input_type
        );
    };
    ensure!(
        shape.len() == 3,
        "model input '{name}' has rank {}, expected 3",
        shape.len()
    );
    let mut resolved = [0_usize; 3];
    for (index, dimension) in shape.iter().enumerate() {
        ensure!(
            *dimension > 0,
            "model input '{name}' dimension {index} is dynamic or invalid: {dimension}"
        );
        resolved[index] = *dimension as usize;
    }

    Ok(resolved)
}

impl PreviousFeatureState {
    pub fn new() -> Self {
        Self {
            keypoints: vec![0.0; KEYPOINTS * 2],
            descriptors: vec![0.0; KEYPOINTS * DESCRIPTOR_DIMENSION],
            valid: vec![false; KEYPOINTS],
        }
    }

    fn replace(&mut self, keypoints: &[f32], descriptors: &[f32], valid: &[bool]) {
        self.keypoints.copy_from_slice(keypoints);
        self.descriptors.copy_from_slice(descriptors);
        self.valid.copy_from_slice(valid);
    }

    fn keypoints_tensor(&self) -> Result<TensorRef<'_, f32>> {
        TensorRef::from_array_view(([KEYPOINTS, 2], self.keypoints.as_slice())).map_err(Into::into)
    }

    fn descriptors_tensor(&self) -> Result<TensorRef<'_, f32>> {
        TensorRef::from_array_view((
            [KEYPOINTS, DESCRIPTOR_DIMENSION],
            self.descriptors.as_slice(),
        ))
        .map_err(Into::into)
    }

    fn valid_tensor(&self) -> Result<TensorRef<'_, bool>> {
        TensorRef::from_array_view(([KEYPOINTS], self.valid.as_slice())).map_err(Into::into)
    }
}

impl<'a> FeatureOutput<'a> {
    pub fn current_left(&self) -> Result<FrameFeatures<'_, CurrentLeft>> {
        self.frame("current_left_keypoints", "current_left_valid")
    }

    pub fn current_right(&self) -> Result<FrameKeypoints<'_, CurrentRight>> {
        self.keypoints("current_right_keypoints")
    }

    pub fn stereo_matches(&self) -> Result<Matches<'_, CurrentLeft, CurrentRight>> {
        self.matches("stereo_matches", "stereo_scores")
    }

    pub fn temporal_matches(&self) -> Result<Matches<'_, PreviousLeft, CurrentLeft>> {
        self.matches("temporal_matches", "temporal_scores")
    }

    pub fn copy_current_left_to(&self, state: &mut PreviousFeatureState) -> Result<()> {
        let keypoints = self.tensor::<f32>("current_left_keypoints")?;
        let descriptors = self.tensor::<f32>("current_left_descriptors")?;
        let valid = self.tensor::<bool>("current_left_valid")?;

        ensure!(
            keypoints.len() == KEYPOINTS * 2,
            "unexpected current_left_keypoints length: {}",
            keypoints.len()
        );
        ensure!(
            descriptors.len() == KEYPOINTS * DESCRIPTOR_DIMENSION,
            "unexpected current_left_descriptors length: {}",
            descriptors.len()
        );
        ensure!(
            valid.len() == KEYPOINTS,
            "unexpected current_left_valid length: {}",
            valid.len()
        );

        state.replace(keypoints, descriptors, valid);
        Ok(())
    }

    fn keypoints<Frame>(&self, keypoints_name: &str) -> Result<FrameKeypoints<'_, Frame>> {
        let keypoints = self.tensor::<f32>(keypoints_name)?;

        ensure!(
            keypoints.len() == KEYPOINTS * 2,
            "unexpected {keypoints_name} length: {}",
            keypoints.len()
        );

        Ok(FrameKeypoints {
            keypoints,
            _frame: PhantomData,
        })
    }

    fn frame<Frame>(
        &self,
        keypoints_name: &str,
        valid_name: &str,
    ) -> Result<FrameFeatures<'_, Frame>> {
        let keypoints = self.tensor::<f32>(keypoints_name)?;
        let valid = self.tensor::<bool>(valid_name)?;

        ensure!(
            keypoints.len() == KEYPOINTS * 2,
            "unexpected {keypoints_name} length: {}",
            keypoints.len()
        );
        ensure!(
            valid.len() == KEYPOINTS,
            "unexpected {valid_name} length: {}",
            valid.len()
        );

        Ok(FrameFeatures {
            keypoints,
            valid,
            _frame: PhantomData,
        })
    }

    fn matches<From, To>(
        &self,
        matches_name: &str,
        scores_name: &str,
    ) -> Result<Matches<'_, From, To>> {
        let matches = self.tensor::<i32>(matches_name)?;
        let scores = self.tensor::<f32>(scores_name)?;

        ensure!(
            matches.len() == KEYPOINTS,
            "unexpected {matches_name} length: {}",
            matches.len()
        );
        ensure!(
            scores.len() == KEYPOINTS,
            "unexpected {scores_name} length: {}",
            scores.len()
        );

        Ok(Matches {
            matches,
            scores,
            _frames: PhantomData,
        })
    }

    fn tensor<T: PrimitiveTensorElementType>(&self, name: &str) -> Result<&[T]> {
        let output = self
            .outputs
            .get(name)
            .wrap_err_with(|| format!("missing model output '{name}'"))?;
        let (_, data) = output.try_extract_tensor::<T>()?;
        Ok(data)
    }
}

impl<Frame> FrameFeatures<'_, Frame> {
    pub fn keypoint(&self, index: usize) -> Option<[f32; 2]> {
        keypoint(self.keypoints, index)
    }

    pub fn is_valid(&self, index: usize) -> bool {
        self.valid.get(index).copied().unwrap_or(false)
    }
}

impl<Frame> FrameKeypoints<'_, Frame> {
    pub fn keypoint(&self, index: usize) -> Option<[f32; 2]> {
        keypoint(self.keypoints, index)
    }
}

fn keypoint(keypoints: &[f32], index: usize) -> Option<[f32; 2]> {
    let offset = index.checked_mul(2)?;
    Some([*keypoints.get(offset)?, *keypoints.get(offset + 1)?])
}

impl<From, To> Matches<'_, From, To> {
    pub fn left_to_right(&self) -> impl Iterator<Item = (usize, usize, f32)> + '_ {
        self.matches
            .iter()
            .zip(self.scores.iter())
            .enumerate()
            .filter_map(|(left_index, (&right_index, &score))| {
                let right_index = usize::try_from(right_index).ok()?;
                (score > 0.0 && right_index < KEYPOINTS).then_some((left_index, right_index, score))
            })
    }
}

fn image_tensor(image: &Image) -> Result<TensorRef<'_, u8>> {
    TensorRef::from_array_view((
        [image.height as usize / 2, image.width as usize / 2, 6],
        image.data.as_ref(),
    ))
    .map_err(Into::into)
}

fn check_stereo_pair_support(stereo: &StereoImagePair) -> Result<()> {
    check_image_support(&stereo.left)?;
    check_image_support(&stereo.right)?;
    ensure_same_shape(&stereo.left, &stereo.right, "left", "right")
}

fn check_image_support(image: &Image) -> Result<()> {
    if image.encoding != "nv12" {
        bail!("unsupported encoding: {}", image.encoding);
    }

    let height = image.height as usize;
    let width = image.width as usize;

    if !(width.is_multiple_of(8) && height.is_multiple_of(8)) {
        bail!(
            "image dimensions must be multiples of 8: {}x{}",
            width,
            height
        );
    }

    if image.data.len() != height * width * 3 / 2 {
        bail!("image data length does not match dimensions");
    }

    Ok(())
}

fn ensure_same_shape(left: &Image, right: &Image, left_name: &str, right_name: &str) -> Result<()> {
    if left.height != right.height
        || left.width != right.width
        || left.data.len() != right.data.len()
    {
        bail!(
            "{left_name} and {right_name} images must have the same shape: {}x{} ({} bytes) != {}x{} ({} bytes)",
            left.width,
            left.height,
            left.data.len(),
            right.width,
            right.height,
            right.data.len()
        );
    }

    Ok(())
}
