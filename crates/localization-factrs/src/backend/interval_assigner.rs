use std::{
    cell::OnceCell,
    time::{Duration, SystemTime},
};

#[derive(Debug, Clone)]
pub(super) struct IntervalAssigner {
    first_observed_time: OnceCell<SystemTime>,
    interval_length: Duration,
}

impl IntervalAssigner {
    pub(super) fn new(interval_length: Duration) -> Self {
        Self {
            first_observed_time: OnceCell::new(),
            interval_length,
        }
    }

    pub(super) fn assign_or_initialize_interval(
        &self,
        measurement_time: SystemTime,
    ) -> Option<u32> {
        let start_time = self.first_observed_time.get_or_init(|| measurement_time);
        let time_since_start = measurement_time.duration_since(*start_time).ok()?;
        Some(self.index_of_duration(time_since_start))
    }

    pub(super) fn current_or_initialize_interval_start_time(
        &self,
        measurement_time: SystemTime,
    ) -> Option<SystemTime> {
        let index = self.assign_or_initialize_interval(measurement_time)?;
        self.interval_start_time(index)
    }

    pub(super) fn interval_start_time(&self, index: u32) -> Option<SystemTime> {
        let start_time = *self.first_observed_time.get()?;
        Some(start_time + self.interval_length * index)
    }

    fn index_of_duration(&self, duration: Duration) -> u32 {
        let index = duration.as_nanos() / self.interval_length.as_nanos();
        index.try_into().expect("does not fit interval index")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assigner(interval: Duration) -> IntervalAssigner {
        let assigner = IntervalAssigner::new(interval);
        assigner
            .first_observed_time
            .set(SystemTime::UNIX_EPOCH)
            .expect("could not set start time");
        assigner
    }

    #[test]
    fn test_get_previous_interval_start_time() {
        let interval = Duration::from_millis(200);
        let assigner = assigner(interval);

        // 1. Middle of an interval
        // 265ms since epoch should snap back to 200ms
        let t1 = SystemTime::UNIX_EPOCH + Duration::from_millis(265);
        let expected1 = SystemTime::UNIX_EPOCH + interval;
        assert_eq!(
            assigner.current_or_initialize_interval_start_time(t1),
            Some(expected1)
        );

        // 2. Exact boundary
        // 200ms since epoch should stay at 200ms
        let t2 = SystemTime::UNIX_EPOCH + Duration::from_millis(200);
        let expected2 = SystemTime::UNIX_EPOCH + interval;
        assert_eq!(
            assigner.current_or_initialize_interval_start_time(t2),
            Some(expected2)
        );

        // 3. Just before a boundary
        // 399ms since epoch should snap back to 200ms
        let t3 = SystemTime::UNIX_EPOCH + Duration::from_millis(399);
        let expected3 = SystemTime::UNIX_EPOCH + interval;
        assert_eq!(
            assigner.current_or_initialize_interval_start_time(t3),
            Some(expected3)
        );

        // 4. Very early time
        // 50ms since epoch with 100ms interval should snap to 0 (Unix Epoch)
        let t4 = SystemTime::UNIX_EPOCH + Duration::from_millis(50);
        let expected4 = SystemTime::UNIX_EPOCH;
        assert_eq!(
            assigner.current_or_initialize_interval_start_time(t4),
            Some(expected4)
        );
    }

    #[test]
    fn test_large_intervals() {
        let interval = Duration::from_secs(1);
        let assigner = assigner(interval);

        // 10.9 seconds -> 10.0 seconds
        let t = SystemTime::UNIX_EPOCH + Duration::from_millis(10900);
        let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
        assert_eq!(
            assigner.current_or_initialize_interval_start_time(t),
            Some(expected)
        );
    }
}
