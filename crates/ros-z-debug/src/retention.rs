use std::{num::NonZeroUsize, time::Duration};

use crate::{Error, Result};

/// Default sample cap used by [`RetentionPolicy::time_window`].
pub const DEFAULT_TIME_WINDOW_MAX_SAMPLES: usize = 4096;

/// Sample retention policy for a debug subscription handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RetentionPolicy {
    /// Keep only the latest sample.
    LatestOnly,
    /// Keep samples inside a source-time window.
    TimeWindow(RetentionWindow),
}

/// Source-time retention window configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RetentionWindow {
    duration: Duration,
    max_samples: Option<NonZeroUsize>,
}

impl RetentionWindow {
    /// Source-time duration retained from the newest sample.
    pub fn duration(self) -> Duration {
        self.duration
    }

    /// Optional cap for the number of retained samples.
    pub fn max_samples(self) -> Option<NonZeroUsize> {
        self.max_samples
    }
}

impl RetentionPolicy {
    /// Retain samples whose source time is inside `duration` from the newest sample.
    ///
    /// This also caps retained samples to [`DEFAULT_TIME_WINDOW_MAX_SAMPLES`] so
    /// stalled or repeated source timestamps cannot grow memory without bound.
    pub fn time_window(duration: Duration) -> Result<Self> {
        Self::time_window_inner(duration, Some(default_time_window_max_samples()))
    }

    /// Retain samples inside `duration`, capped to `max_samples` newest entries.
    pub fn time_window_with_max_samples(
        duration: Duration,
        max_samples: NonZeroUsize,
    ) -> Result<Self> {
        Self::time_window_inner(duration, Some(max_samples))
    }

    fn time_window_inner(duration: Duration, max_samples: Option<NonZeroUsize>) -> Result<Self> {
        if duration.is_zero() {
            return Err(Error::InvalidRetention(
                "time window duration must be greater than zero".to_string(),
            ));
        }

        Ok(Self::TimeWindow(RetentionWindow {
            duration,
            max_samples,
        }))
    }
}

fn default_time_window_max_samples() -> NonZeroUsize {
    NonZeroUsize::new(DEFAULT_TIME_WINDOW_MAX_SAMPLES)
        .expect("default time-window retention cap must be non-zero")
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroUsize, time::Duration};

    use super::{DEFAULT_TIME_WINDOW_MAX_SAMPLES, RetentionPolicy};

    #[test]
    fn time_window_rejects_zero_duration() {
        let error = RetentionPolicy::time_window(Duration::ZERO).unwrap_err();

        assert!(error.to_string().contains("duration"));
    }

    #[test]
    fn time_window_retains_non_zero_duration() {
        let policy = RetentionPolicy::time_window(Duration::from_secs(1)).unwrap();

        let RetentionPolicy::TimeWindow(window) = policy else {
            panic!("expected time window retention");
        };
        assert_eq!(window.duration(), Duration::from_secs(1));
        assert_eq!(
            window.max_samples(),
            NonZeroUsize::new(DEFAULT_TIME_WINDOW_MAX_SAMPLES)
        );
    }

    #[test]
    fn time_window_with_max_samples_uses_non_zero_cap() {
        let policy = RetentionPolicy::time_window_with_max_samples(
            Duration::from_secs(1),
            NonZeroUsize::new(2).unwrap(),
        )
        .unwrap();

        let RetentionPolicy::TimeWindow(window) = policy else {
            panic!("expected time window retention");
        };
        assert_eq!(window.duration(), Duration::from_secs(1));
        assert_eq!(window.max_samples(), NonZeroUsize::new(2));
    }
}
