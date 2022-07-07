use std::{fs::File, path::Path, time::Duration};

use anyhow::Context;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::from_reader;
use types::Joints;

use crate::control::linear_interpolator::LinearInterpolator;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MotionFile {
    initial_positions: Joints,
    frames: Vec<MotionFileFrame>,
}

impl MotionFile {
    pub fn from_path<P>(motion_file_path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let file = File::open(&motion_file_path).with_context(|| {
            format!(
                "Failed to open motion file {}",
                motion_file_path.as_ref().display()
            )
        })?;
        from_reader(file).with_context(|| {
            format!(
                "Failed to parse motion file {}",
                motion_file_path.as_ref().display()
            )
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
struct MotionFileFrame {
    #[serde(
        serialize_with = "serialize_float_seconds",
        deserialize_with = "deserialize_float_seconds"
    )]
    duration: Duration,
    positions: Joints,
}

fn serialize_float_seconds<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f32(duration.as_secs_f32())
}

fn deserialize_float_seconds<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Duration::from_secs_f32(f32::deserialize(deserializer)?))
}

pub struct MotionFileInterpolator {
    interpolators: Vec<LinearInterpolator<Joints>>,
    interpolator_index: usize,
}

impl From<MotionFile> for MotionFileInterpolator {
    fn from(motion_file: MotionFile) -> Self {
        assert!(!motion_file.frames.is_empty());
        let mut interpolators = vec![LinearInterpolator::new(
            motion_file.initial_positions,
            motion_file.frames[0].positions,
            motion_file.frames[0].duration,
        )];
        interpolators.extend(
            motion_file
                .frames
                .iter()
                .zip(motion_file.frames.iter().skip(1))
                .map(|(start_frame, end_frame)| {
                    LinearInterpolator::new(
                        start_frame.positions,
                        end_frame.positions,
                        end_frame.duration,
                    )
                }),
        );
        Self {
            interpolators,
            interpolator_index: 0,
        }
    }
}

impl MotionFileInterpolator {
    pub fn reset(&mut self) {
        self.interpolators
            .iter_mut()
            .for_each(|interpolator| interpolator.reset());
        self.interpolator_index = 0;
    }

    pub fn step(&mut self, time_step: Duration) -> Joints {
        let mut remaining_time_step = time_step;
        loop {
            let current_interpolator = &self.interpolators[self.interpolator_index];
            let remaining_duration = current_interpolator.remaining_duration();
            if remaining_time_step < remaining_duration
                || self.interpolator_index >= self.interpolators.len() - 1
            {
                break;
            }
            remaining_time_step -= remaining_duration;
            self.interpolator_index += 1;
        }
        self.interpolators[self.interpolator_index].step(remaining_time_step)
    }

    pub fn value(&self) -> Joints {
        self.interpolators[self.interpolator_index].value()
    }

    pub fn is_finished(&self) -> bool {
        self.interpolators.last().unwrap().is_finished()
    }
}
