use std::time::{Duration, SystemTime};

pub struct PeriodicalTask<T> {
    interval: Duration,
    last_executed: SystemTime,
    task: T,
}

impl<T> PeriodicalTask<T> {
    pub fn new(interval: Duration, task: T) -> Self {
        Self {
            interval,
            last_executed: SystemTime::UNIX_EPOCH,
            task,
        }
    }

    pub fn task(&mut self, now: SystemTime) -> Option<&T> {
        if now >= self.last_executed + self.interval {
            self.last_executed = now;
            Some(&self.task)
        } else {
            None
        }
    }
}
