use std::num::TryFromIntError;

use ros_z::{
    dynamic::{CdrSelectionError, Schema, SelectionError, ValuePath},
    time::Time,
};
use ros2::builtin_interfaces::time::Time as RosTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeEncoding {
    RosTime,
    RosZTime,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeProjection {
    #[default]
    SourceTime,
    Field {
        path: ValuePath,
        encoding: TimeEncoding,
    },
}

#[derive(Debug, Error)]
pub enum ProjectionError {
    #[error(transparent)]
    Path(#[from] SelectionError),
    #[error("timestamp projection at {path}: {source}")]
    Selection {
        path: ValuePath,
        #[source]
        source: CdrSelectionError,
    },
    #[error("timestamp at {path} has negative seconds: {seconds}")]
    Negative { path: ValuePath, seconds: i32 },
    #[error("timestamp at {path} has invalid nanoseconds: {nanoseconds}")]
    InvalidNanoseconds { path: ValuePath, nanoseconds: u32 },
    #[error("timestamp at {path} exceeds the supported range: {nanoseconds} ns")]
    Overflow {
        path: ValuePath,
        nanoseconds: u128,
        #[source]
        source: TryFromIntError,
    },
}

impl TimeProjection {
    pub fn field(path: &str, encoding: TimeEncoding) -> Result<Self, ProjectionError> {
        Ok(Self::Field {
            path: path.parse()?,
            encoding,
        })
    }

    pub fn validate(&self, schema: &Schema) -> Result<(), ProjectionError> {
        let Self::Field { path, encoding } = self else {
            return Ok(());
        };
        match encoding {
            TimeEncoding::RosTime => path.validate_cdr_as::<RosTime>(schema),
            TimeEncoding::RosZTime => path.validate_cdr_as::<Time>(schema),
        }
        .map_err(|source| ProjectionError::Selection {
            path: path.clone(),
            source,
        })
    }

    pub fn project_cdr(
        &self,
        source_time: Time,
        bytes: &[u8],
        schema: &Schema,
    ) -> Result<Time, ProjectionError> {
        let Self::Field { path, encoding } = self else {
            return Ok(source_time);
        };
        match encoding {
            TimeEncoding::RosTime => {
                let value = path
                    .select_cdr_as::<RosTime>(bytes, schema)
                    .map_err(|source| ProjectionError::Selection {
                        path: path.clone(),
                        source,
                    })?;
                checked_ros_time(path, value)
            }
            TimeEncoding::RosZTime => {
                let value = path
                    .select_cdr_as::<Time>(bytes, schema)
                    .map_err(|source| ProjectionError::Selection {
                        path: path.clone(),
                        source,
                    })?;
                checked_ros_z_time(path, value)
            }
        }
    }
}

fn checked_ros_time(path: &ValuePath, value: RosTime) -> Result<Time, ProjectionError> {
    if value.sec < 0 {
        return Err(ProjectionError::Negative {
            path: path.clone(),
            seconds: value.sec,
        });
    }
    if value.nanosec >= 1_000_000_000 {
        return Err(ProjectionError::InvalidNanoseconds {
            path: path.clone(),
            nanoseconds: value.nanosec,
        });
    }
    checked_nanoseconds(
        path,
        value.sec as u128 * 1_000_000_000 + u128::from(value.nanosec),
    )
}

fn checked_ros_z_time(path: &ValuePath, value: Time) -> Result<Time, ProjectionError> {
    checked_nanoseconds(path, value.duration_since(Time::zero()).as_nanos())
}

fn checked_nanoseconds(path: &ValuePath, nanoseconds: u128) -> Result<Time, ProjectionError> {
    let nanos = nanoseconds
        .try_into()
        .map_err(|source| ProjectionError::Overflow {
            path: path.clone(),
            nanoseconds,
            source,
        })?;
    Ok(Time::from_nanos(nanos))
}
