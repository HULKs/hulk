use std::fmt::Debug;
use std::time::Duration;

use crate::{
    condition::{ContinuousConditionType, DiscreteConditionType, Response, TimeOut},
    timed_spline::{InterpolatorError, TimedSpline},
    Condition, MotionFile,
};
use color_eyre::{Report, Result};
use itertools::Itertools;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};
use splines::Interpolate;
use types::condition_input::ConditionInput;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConditionedSpline<T> {
    pub entry_condition: Option<DiscreteConditionType>,
    pub interrupt_conditions: Vec<ContinuousConditionType>,
    pub spline: TimedSpline<T>,
    pub exit_condition: Option<DiscreteConditionType>,
}

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct MotionInterpolator<T> {
    frames: Vec<ConditionedSpline<T>>,
}

#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, PathIntrospect, PathSerialize, PathDeserialize,
)]
pub enum InterpolatorState<T> {
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
    Aborted {
        at_position: T,
    },
}

impl<T> InterpolatorState<T> {
    pub const INITIAL: Self = InterpolatorState::CheckEntry {
        current_frame_index: 0,
        time_since_start: Duration::ZERO,
    };

    pub fn current_frame_index(&self) -> Option<usize> {
        match self {
            InterpolatorState::CheckEntry {
                current_frame_index,
                ..
            }
            | InterpolatorState::InterpolateSpline {
                current_frame_index,
                ..
            }
            | InterpolatorState::CheckExit {
                current_frame_index,
                ..
            } => Some(*current_frame_index),
            _ => None,
        }
    }

    pub fn is_aborted(&self) -> bool {
        matches!(self, Self::Aborted { .. })
    }

    pub fn is_running(&self) -> bool {
        match self {
            InterpolatorState::CheckEntry {
                current_frame_index,
                time_since_start,
            } => *current_frame_index >= 1 || *time_since_start > Duration::ZERO,
            InterpolatorState::InterpolateSpline { .. } | InterpolatorState::CheckExit { .. } => {
                true
            }
            InterpolatorState::Finished | InterpolatorState::Aborted { .. } => false,
        }
    }

    pub fn reset(&mut self) {
        *self = InterpolatorState::CheckEntry {
            current_frame_index: 0,
            time_since_start: Duration::ZERO,
        };
    }
}

impl<T> Default for InterpolatorState<T> {
    fn default() -> Self {
        InterpolatorState::CheckEntry {
            current_frame_index: 0,
            time_since_start: Duration::ZERO,
        }
    }
}

enum ReturnState {
    Return,
    Continue,
}

impl<T: Debug + Interpolate<f32>> MotionInterpolator<T> {
    fn check_continuous_conditions(
        &self,
        state: &mut InterpolatorState<T>,
        condition_input: &ConditionInput,
    ) -> ReturnState {
        if let Some(continuous_conditions) = state
            .current_frame_index()
            .map(|frame_index| &self.frames[frame_index].interrupt_conditions)
        {
            return match continuous_conditions
                .iter()
                .map(|condition| condition.evaluate(condition_input))
                .reduce(|accumulated, current| match (&accumulated, &current) {
                    (Response::Abort, _) => Response::Abort,
                    (_, Response::Abort) => Response::Abort,
                    (Response::Wait, _) => Response::Wait,
                    (_, Response::Wait) => Response::Wait,
                    _ => accumulated,
                }) {
                Some(Response::Abort) => {
                    *state = InterpolatorState::Aborted {
                        at_position: self.value(*state),
                    };
                    ReturnState::Return
                }
                Some(Response::Wait) => ReturnState::Return,
                _ => ReturnState::Continue,
            };
        }

        ReturnState::Continue
    }

    fn handle_state_transitions(
        &self,
        state: &mut InterpolatorState<T>,
        time_step: Duration,
        condition_input: &ConditionInput,
    ) {
        *state = match *state {
            InterpolatorState::CheckEntry {
                current_frame_index,
                time_since_start,
            } => {
                let current_frame = &self.frames[current_frame_index];
                match current_frame.entry_condition.as_ref().map(|condition| {
                    condition
                        .evaluate(condition_input)
                        .with_timeout(condition.timeout(time_since_start))
                }) {
                    Some(Response::Abort) => InterpolatorState::Aborted {
                        at_position: self.value(*state),
                    },
                    Some(Response::Wait) => InterpolatorState::CheckEntry {
                        current_frame_index,
                        time_since_start: time_since_start + time_step,
                    },
                    _ => InterpolatorState::InterpolateSpline {
                        current_frame_index,
                        time_since_start: Duration::ZERO,
                    },
                }
            }
            InterpolatorState::InterpolateSpline {
                current_frame_index,
                time_since_start,
            } => {
                let current_frame = &self.frames[current_frame_index];
                if time_since_start >= current_frame.spline.total_duration() {
                    InterpolatorState::CheckExit {
                        current_frame_index,
                        time_since_start: Duration::ZERO,
                    }
                } else {
                    InterpolatorState::InterpolateSpline {
                        current_frame_index,
                        time_since_start: time_since_start + time_step,
                    }
                }
            }
            InterpolatorState::CheckExit {
                current_frame_index,
                time_since_start,
            } => {
                let current_frame = &self.frames[current_frame_index];
                match current_frame.exit_condition.as_ref().map(|condition| {
                    condition
                        .evaluate(condition_input)
                        .with_timeout(condition.timeout(time_since_start))
                }) {
                    Some(Response::Abort) => InterpolatorState::Aborted {
                        at_position: self.value(*state),
                    },
                    Some(Response::Wait) => InterpolatorState::CheckExit {
                        current_frame_index,
                        time_since_start: time_since_start + time_step,
                    },
                    _ if current_frame_index < self.frames.len() - 1 => {
                        InterpolatorState::CheckEntry {
                            current_frame_index: current_frame_index + 1,
                            time_since_start: Duration::ZERO,
                        }
                    }
                    _ => InterpolatorState::Finished,
                }
            }
            other_state => other_state,
        };
    }

    pub fn advance_state(
        &self,
        state: &mut InterpolatorState<T>,
        time_step: Duration,
        condition_input: &ConditionInput,
    ) {
        if let ReturnState::Return = self.check_continuous_conditions(state, condition_input) {
            return;
        }

        self.handle_state_transitions(state, time_step, condition_input);
    }

    pub fn value(&self, state: InterpolatorState<T>) -> T {
        match state {
            InterpolatorState::CheckEntry {
                current_frame_index,
                ..
            } => self.frames[current_frame_index].spline.start_position(),
            InterpolatorState::InterpolateSpline {
                current_frame_index,
                time_since_start,
            } => self.frames[current_frame_index]
                .spline
                .value_at(time_since_start),
            InterpolatorState::CheckExit {
                current_frame_index,
                ..
            } => self.frames[current_frame_index].spline.end_position(),
            InterpolatorState::Finished => self.frames.last().unwrap().spline.end_position(),
            InterpolatorState::Aborted { at_position } => at_position,
        }
    }

    pub fn set_initial_positions(&mut self, position: T) {
        if let Some(keyframe) = self.frames.first_mut() {
            keyframe.spline.set_initial_positions(position);
        }
    }

    pub fn estimated_remaining_duration(&self, state: InterpolatorState<T>) -> Option<Duration> {
        match state.current_frame_index() {
            Some(index) => {
                let mut remaining = self
                    .frames
                    .iter()
                    .skip(index + 1)
                    .map(|frame| frame.spline.total_duration())
                    .sum::<Duration>();
                remaining += match state {
                    InterpolatorState::CheckEntry { .. } => {
                        self.frames[index].spline.total_duration()
                    }
                    InterpolatorState::InterpolateSpline {
                        time_since_start, ..
                    } => Duration::saturating_sub(
                        self.frames[index].spline.total_duration(),
                        time_since_start,
                    ),
                    InterpolatorState::CheckExit { .. } => Duration::ZERO,
                    InterpolatorState::Finished => Duration::ZERO,
                    InterpolatorState::Aborted { .. } => return None,
                };
                Some(remaining)
            }
            None => {
                if state.is_aborted() {
                    None
                } else {
                    Some(Duration::ZERO)
                }
            }
        }
    }
}

impl<T: Debug + Interpolate<f32>> TryFrom<MotionFile<T>> for MotionInterpolator<T> {
    type Error = Report;

    fn try_from(motion_file: MotionFile<T>) -> Result<Self> {
        let interpolation_mode = motion_file.interpolation_mode;

        let first_frame = motion_file.motion.first().unwrap();

        let mut motion_frames = vec![ConditionedSpline {
            entry_condition: first_frame.entry_condition.clone(),
            interrupt_conditions: first_frame.interrupt_conditions.clone(),
            spline: TimedSpline::try_new_with_start(
                motion_file.initial_positions,
                first_frame.keyframes.clone(),
                interpolation_mode,
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
                        interrupt_conditions: second_frame.interrupt_conditions,
                        spline: TimedSpline::try_new_with_start(
                            first_frame.keyframes.last().unwrap().positions,
                            second_frame.keyframes,
                            interpolation_mode,
                        )?,
                        exit_condition: second_frame.exit_condition,
                    })
                })
                .collect::<Result<Vec<_>, InterpolatorError>>()?,
        );

        Ok(Self {
            frames: motion_frames,
        })
    }
}
