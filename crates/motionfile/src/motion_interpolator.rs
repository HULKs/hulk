use std::fmt::Debug;
use std::time::Duration;

use crate::timed_spline::{InterpolatorError, TimedSpline};
use crate::Condition;
use crate::{condition::ConditionType, MotionFile};
use color_eyre::{Report, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use splines::Interpolate;
use types::ConditionInput;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConditionedSpline<T> {
    pub entry_condition: Option<ConditionType>,
    pub spline: TimedSpline<T>,
    pub exit_condition: Option<ConditionType>,
}

#[derive(Default, Debug)]
pub struct MotionInterpolator<T> {
    frames: Vec<ConditionedSpline<T>>,
    current_state: State,
}

#[derive(Debug)]
enum State {
    CheckEntry {
        current_frame_index: usize,
        time_since_start: Duration,
    },
    InterpolateSpline {
        current_frame_index: usize,
        time_since_start: Duration,
    },
    CheckExit {
        current_frame_index: usize,
        time_since_start: Duration,
    },
    Finished,
}

impl Default for State {
    fn default() -> Self {
        State::CheckEntry {
            current_frame_index: 0,
            time_since_start: Duration::ZERO,
        }
    }
}

impl<T: Debug + Interpolate<f32>> MotionInterpolator<T> {
    pub fn advance_by(&mut self, time_step: Duration, condition_input: &ConditionInput) {
        self.current_state = match self.current_state {
            State::CheckEntry {
                current_frame_index,
                time_since_start,
            } => {
                let current_frame = &self.frames[current_frame_index];
                if current_frame
                    .entry_condition
                    .as_ref()
                    .map(|condition| condition.is_fulfilled(condition_input, time_since_start))
                    .unwrap_or(true)
                {
                    State::InterpolateSpline {
                        current_frame_index,
                        time_since_start: Duration::ZERO,
                    }
                } else {
                    State::CheckEntry {
                        current_frame_index,
                        time_since_start: time_since_start + time_step,
                    }
                }
            }
            State::InterpolateSpline {
                current_frame_index,
                time_since_start,
            } => {
                let current_frame = &self.frames[current_frame_index];
                if time_since_start >= current_frame.spline.total_duration() {
                    State::CheckExit {
                        current_frame_index,
                        time_since_start: Duration::ZERO,
                    }
                } else {
                    State::InterpolateSpline {
                        current_frame_index,
                        time_since_start: time_since_start + time_step,
                    }
                }
            }
            State::CheckExit {
                current_frame_index,
                time_since_start,
            } => {
                let current_frame = &self.frames[current_frame_index];
                if current_frame
                    .exit_condition
                    .as_ref()
                    .map(|condition| condition.is_fulfilled(condition_input, time_since_start))
                    .unwrap_or(true)
                {
                    if current_frame_index < self.frames.len() - 1 {
                        State::CheckEntry {
                            current_frame_index: current_frame_index + 1,
                            time_since_start: Duration::ZERO,
                        }
                    } else {
                        State::Finished
                    }
                } else {
                    State::CheckExit {
                        current_frame_index,
                        time_since_start: time_since_start + time_step,
                    }
                }
            }
            State::Finished => State::Finished,
        };
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.current_state, State::Finished)
    }

    pub fn value(&self) -> T {
        match self.current_state {
            State::CheckEntry {
                current_frame_index,
                ..
            } => self.frames[current_frame_index].spline.start_position(),
            State::InterpolateSpline {
                current_frame_index,
                time_since_start,
            } => self.frames[current_frame_index]
                .spline
                .value_at(time_since_start),
            State::CheckExit {
                current_frame_index,
                ..
            } => self.frames[current_frame_index].spline.end_position(),
            State::Finished => self.frames.last().unwrap().spline.end_position(),
        }
    }

    pub fn reset(&mut self) {
        self.current_state = State::CheckEntry {
            current_frame_index: 0,
            time_since_start: Duration::ZERO,
        }
    }

    pub fn set_initial_positions(&mut self, position: T) {
        if let Some(keyframe) = self.frames.first_mut() {
            keyframe.spline.set_initial_positions(position);
        }
    }
}

impl<T: Debug + Interpolate<f32>> TryFrom<MotionFile<T>> for MotionInterpolator<T> {
    type Error = Report;

    fn try_from(motion_file: MotionFile<T>) -> Result<Self> {
        let first_frame = motion_file.motion.first().unwrap();

        let mut motion_frames = vec![ConditionedSpline {
            entry_condition: first_frame.entry_condition.clone(),
            spline: TimedSpline::try_new_with_start(
                motion_file.initial_positions,
                first_frame.keyframes.clone(),
            )?,
            exit_condition: first_frame.exit_condition.clone(),
        }];

        motion_frames.extend(
            motion_file
                .motion
                .into_iter()
                .tuple_windows()
                .map(|(first_frame, second_frame)| {
                    Ok(ConditionedSpline {
                        entry_condition: second_frame.entry_condition,
                        spline: TimedSpline::try_new_with_start(
                            first_frame.keyframes.last().unwrap().positions,
                            second_frame.keyframes,
                        )?,
                        exit_condition: second_frame.exit_condition,
                    })
                })
                .collect::<Result<Vec<_>, InterpolatorError>>()?,
        );

        Ok(Self {
            current_state: State::CheckEntry {
                current_frame_index: 0,
                time_since_start: Duration::ZERO,
            },
            frames: motion_frames,
        })
    }
}
