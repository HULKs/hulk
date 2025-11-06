use std::{
    ops::Range,
    time::{Duration, SystemTime},
};

use booster::{
    ButtonEventMsg, FallDownState, LowCommand, LowState, RemoteControllerState, TransformMessage,
};
use pyo3::pyclass;
use serde::{Deserialize, Serialize};
use zed::RGBDSensors;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationMessage<T> {
    pub time: SystemTime,
    pub payload: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessageKind {
    LowState(LowState),
    FallDownState(FallDownState),
    ButtonEventMsg(ButtonEventMsg),
    RemoteControllerState(RemoteControllerState),
    TransformMessage(TransformMessage),
    RGBDSensors(Box<RGBDSensors>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessageKind {
    LowCommand(LowCommand),
}

#[pyclass(frozen, eq)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskName {
    ApplyLowCommand,
    RequestLowState,
    RequestRGBDSensors,
    StepSimulation,
    Reset,
    Invalid,
    RequestSceneState,
    RequestSceneDescription,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub schedule: Vec<TaskSchedule>,
}

/// This functions returns `true` if for any non-negative `k`, `offset + k * interval` is in the half-open interval `[range.start, range.end)`.
fn is_due(interval: Duration, offset: SystemTime, range: Range<SystemTime>) -> bool {
    let (lower, upper) = (range.start, range.end);
    if offset > upper {
        return false;
    }
    if offset >= lower {
        return true;
    }
    if interval.is_zero() {
        return false;
    }
    // offset < lower and k is positive
    let duration_to_start = lower.duration_since(offset).expect("time ran backwards");
    let factor = duration_to_start.div_duration_f64(interval);

    let time_of_event = offset + interval.mul_f64(factor.ceil());
    time_of_event < upper
}

impl ConnectionInfo {
    pub fn control_only() -> Self {
        Self {
            schedule: vec![
                TaskSchedule::Once(OnceTask::Reset),
                TaskSchedule::Periodical {
                    interval: Duration::from_millis(10),
                    offset: SystemTime::UNIX_EPOCH,
                    task: PeriodicalTask::RequestLowState,
                },
                TaskSchedule::Periodical {
                    interval: Duration::from_millis(10),
                    offset: SystemTime::UNIX_EPOCH,
                    task: PeriodicalTask::ApplyLowCommand,
                },
            ],
        }
    }

    pub fn control_and_vision() -> Self {
        Self {
            schedule: vec![
                TaskSchedule::Once(OnceTask::Reset),
                TaskSchedule::Periodical {
                    interval: Duration::from_millis(10),
                    offset: SystemTime::UNIX_EPOCH,
                    task: PeriodicalTask::RequestLowState,
                },
                TaskSchedule::Periodical {
                    interval: Duration::from_millis(33),
                    offset: SystemTime::UNIX_EPOCH,
                    task: PeriodicalTask::RequestRGBDSensors,
                },
                TaskSchedule::Periodical {
                    interval: Duration::from_millis(10),
                    offset: SystemTime::UNIX_EPOCH,
                    task: PeriodicalTask::ApplyLowCommand,
                },
            ],
        }
    }

    pub fn viewer() -> Self {
        Self {
            schedule: vec![
                TaskSchedule::Once(OnceTask::RequestSceneDescription),
                TaskSchedule::OnStep(PeriodicalTask::RequestSceneState),
            ],
        }
    }

    pub fn initial_tasks(&self) -> Vec<OnceTask> {
        self.schedule
            .iter()
            .filter_map(|task| match task {
                TaskSchedule::Once(task_name) => Some(task_name),
                _ => None,
            })
            .copied()
            .collect()
    }

    pub fn due_tasks(&self, range: Range<SystemTime>) -> Vec<PeriodicalTask> {
        self.schedule
            .iter()
            .filter_map(|task| match task {
                TaskSchedule::Periodical {
                    interval,
                    offset,
                    task,
                } if is_due(*interval, *offset, range.clone()) => Some(task),
                TaskSchedule::OnStep(task_name) => Some(task_name),
                _ => None,
            })
            .copied()
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum OnceTask {
    RequestSceneDescription,
    Reset,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PeriodicalTask {
    ApplyLowCommand,
    RequestLowState,
    RequestRGBDSensors,
    RequestSceneState,
    RequestSceneDescription,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TaskSchedule {
    Once(OnceTask),
    Periodical {
        interval: Duration,
        offset: SystemTime,
        task: PeriodicalTask,
    },
    OnStep(PeriodicalTask),
}

#[cfg(test)]
mod tests {
    use std::{
        ops::Range,
        time::{Duration, SystemTime},
    };

    use crate::is_due;

    fn millis_duration(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn millis_systemtime(millis: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + millis_duration(millis)
    }

    fn millis_range(a: u64, b: u64) -> Range<SystemTime> {
        millis_systemtime(a)..millis_systemtime(b)
    }

    #[test]
    pub fn test_is_due_in_range() {
        assert!(is_due(
            millis_duration(2),
            millis_systemtime(0),
            millis_range(0, 1)
        ));

        assert!(is_due(
            millis_duration(2000),
            millis_systemtime(0),
            millis_range(0, 1)
        ));

        assert!(is_due(
            millis_duration(1),
            millis_systemtime(1000),
            millis_range(1000, 2000)
        ));
    }

    #[test]
    pub fn test_is_not_due_at_end() {
        assert!(!is_due(
            millis_duration(2),
            millis_systemtime(0),
            millis_range(1, 2)
        ));

        assert!(!is_due(
            millis_duration(2000),
            millis_systemtime(0),
            millis_range(1999, 2000)
        ));

        assert!(!is_due(
            millis_duration(2),
            millis_systemtime(2),
            millis_range(3, 4)
        ));
    }

    #[test]
    pub fn test_is_due_with_step() {
        assert!(is_due(
            millis_duration(2),
            millis_systemtime(0),
            millis_range(2, 3)
        ));

        assert!(is_due(
            millis_duration(2),
            millis_systemtime(0),
            millis_range(4, 1000)
        ));

        assert!(is_due(
            millis_duration(2),
            millis_systemtime(1),
            millis_range(5, 6)
        ));
        assert!(is_due(
            millis_duration(8),
            millis_systemtime(10),
            millis_range(26, 27)
        ))
    }

    #[test]
    pub fn test_is_due_skip_with_step() {
        assert!(!is_due(
            millis_duration(3),
            millis_systemtime(0),
            millis_range(2, 3)
        ));

        assert!(!is_due(
            millis_duration(10),
            millis_systemtime(5),
            millis_range(6, 8)
        ));

        assert!(!is_due(
            millis_duration(2000),
            millis_systemtime(1),
            millis_range(500, 1500)
        ));
        assert!(!is_due(
            millis_duration(8),
            millis_systemtime(10),
            millis_range(11, 18)
        ))
    }
}
