use std::time::Duration;

use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RetentionPolicy {
    LatestOnly,
    TimeWindow {
        duration: Duration,
        max_samples: Option<usize>,
    },
}

impl RetentionPolicy {
    pub fn validate(self) -> Result<Self> {
        if let Self::TimeWindow {
            duration,
            max_samples,
        } = self
        {
            if duration.is_zero() {
                return Err(Error::InvalidRetention(
                    "time window duration must be greater than zero".to_string(),
                ));
            }

            if max_samples == Some(0) {
                return Err(Error::InvalidRetention(
                    "time window max_samples must be greater than zero when set".to_string(),
                ));
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RetentionPolicy;

    #[test]
    fn validation_rejects_zero_duration() {
        let error = RetentionPolicy::TimeWindow {
            duration: Duration::ZERO,
            max_samples: None,
        }
        .validate()
        .unwrap_err();

        assert!(error.to_string().contains("duration"));
    }

    #[test]
    fn validation_rejects_zero_max_samples() {
        let error = RetentionPolicy::TimeWindow {
            duration: Duration::from_secs(1),
            max_samples: Some(0),
        }
        .validate()
        .unwrap_err();

        assert!(error.to_string().contains("max_samples"));
    }
}
