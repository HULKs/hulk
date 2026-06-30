use std::{
    collections::{BTreeMap, VecDeque},
    num::NonZeroUsize,
    time::{Duration, Instant},
};

use ros_z::{EndpointGlobalId, time::Time};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HzReport {
    pub topic: String,
    pub receive: HzStats,
    pub sources: Vec<SourceHzStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HzStats {
    pub rate_hz: Option<f64>,
    pub min_seconds: Option<f64>,
    pub max_seconds: Option<f64>,
    pub stddev_seconds: Option<f64>,
    pub intervals: usize,
    pub window_limit: usize,
    pub samples: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceHzStats {
    pub source: String,
    #[serde(flatten)]
    pub stats: HzStats,
}

pub struct HzEstimator {
    topic: String,
    receive: IntervalWindow,
    last_receive: Option<Instant>,
    sources: BTreeMap<EndpointGlobalId, SourceWindow>,
    window_limit: NonZeroUsize,
}

impl HzEstimator {
    pub fn new(topic: String, window_limit: NonZeroUsize) -> Self {
        Self {
            topic,
            receive: IntervalWindow::new(window_limit),
            last_receive: None,
            sources: BTreeMap::new(),
            window_limit,
        }
    }

    pub fn observe_receive(&mut self, received_at: Instant) {
        self.receive.observe_sample();
        if let Some(previous) = self.last_receive.replace(received_at) {
            self.receive
                .observe_interval(received_at.saturating_duration_since(previous));
        }
    }

    pub fn observe_source(&mut self, source: EndpointGlobalId, source_time: Time) {
        self.sources
            .entry(source)
            .or_insert_with(|| SourceWindow::new(self.window_limit))
            .observe(source_time);
    }

    pub fn report(&self) -> HzReport {
        HzReport {
            topic: self.topic.clone(),
            receive: self.receive.stats(),
            sources: self
                .sources
                .iter()
                .map(|(source, window)| SourceHzStats {
                    source: source.to_string(),
                    stats: window.stats(),
                })
                .collect(),
        }
    }
}

struct SourceWindow {
    intervals: IntervalWindow,
    last_source_time: Option<Time>,
}

impl SourceWindow {
    fn new(window_limit: NonZeroUsize) -> Self {
        Self {
            intervals: IntervalWindow::new(window_limit),
            last_source_time: None,
        }
    }

    fn observe(&mut self, source_time: Time) {
        self.intervals.observe_sample();
        if let Some(previous) = self.last_source_time.replace(source_time) {
            self.intervals
                .observe_interval(source_time.duration_since(previous));
        }
    }

    fn stats(&self) -> HzStats {
        self.intervals.stats()
    }
}

struct IntervalWindow {
    max_len: NonZeroUsize,
    intervals: VecDeque<Duration>,
    samples: usize,
}

impl IntervalWindow {
    fn new(max_len: NonZeroUsize) -> Self {
        Self {
            max_len,
            intervals: VecDeque::new(),
            samples: 0,
        }
    }

    fn observe_sample(&mut self) {
        self.samples += 1;
    }

    fn observe_interval(&mut self, interval: Duration) {
        self.intervals.push_back(interval);
        while self.intervals.len() > self.max_len.get() {
            self.intervals.pop_front();
        }
    }

    fn stats(&self) -> HzStats {
        let intervals = self.intervals.len();
        let base = HzStats {
            rate_hz: None,
            min_seconds: None,
            max_seconds: None,
            stddev_seconds: None,
            intervals,
            window_limit: self.max_len.get(),
            samples: self.samples,
        };

        if self.intervals.is_empty() {
            return base;
        }

        let mut sum = 0.0;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for seconds in self.intervals.iter().map(Duration::as_secs_f64) {
            sum += seconds;
            min = min.min(seconds);
            max = max.max(seconds);
        }
        if sum <= f64::EPSILON {
            return base;
        }

        let mean = sum / intervals as f64;
        let variance = self
            .intervals
            .iter()
            .map(Duration::as_secs_f64)
            .map(|seconds| {
                let delta = seconds - mean;
                delta * delta
            })
            .sum::<f64>()
            / intervals.saturating_sub(1).max(1) as f64;

        HzStats {
            rate_hz: Some(intervals as f64 / sum),
            min_seconds: Some(min),
            max_seconds: Some(max),
            stddev_seconds: Some(variance.sqrt()),
            intervals,
            window_limit: self.max_len.get(),
            samples: self.samples,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ros_z::ENDPOINT_GLOBAL_ID_SIZE;
    use std::num::NonZeroUsize;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    fn source(byte: u8) -> EndpointGlobalId {
        EndpointGlobalId::from([byte; ENDPOINT_GLOBAL_ID_SIZE])
    }

    fn window(limit: usize) -> NonZeroUsize {
        NonZeroUsize::new(limit).expect("test window limit must be non-zero")
    }

    #[test]
    fn receive_report_uses_interval_statistics() {
        let start = Instant::now();
        let mut estimator = HzEstimator::new("/chatter".to_string(), window(10));

        estimator.observe_receive(start);
        estimator.observe_receive(start + Duration::from_millis(100));
        estimator.observe_receive(start + Duration::from_millis(300));

        let report = estimator.report();
        let receive = report.receive;
        assert_close(receive.rate_hz.expect("rate"), 2.0 / 0.3);
        assert_close(receive.min_seconds.expect("min"), 0.1);
        assert_close(receive.max_seconds.expect("max"), 0.2);
        assert_close(receive.stddev_seconds.expect("stddev"), 0.005f64.sqrt());
        assert_eq!(receive.intervals, 2);
        assert_eq!(receive.window_limit, 10);
        assert_eq!(receive.samples, 3);
    }

    #[test]
    fn receive_window_truncates_old_intervals() {
        let start = Instant::now();
        let mut estimator = HzEstimator::new("/chatter".to_string(), window(2));

        estimator.observe_receive(start);
        estimator.observe_receive(start + Duration::from_millis(100));
        estimator.observe_receive(start + Duration::from_millis(300));
        estimator.observe_receive(start + Duration::from_millis(600));

        let receive = estimator.report().receive;
        assert_close(receive.rate_hz.expect("rate"), 2.0 / 0.5);
        assert_close(receive.min_seconds.expect("min"), 0.2);
        assert_close(receive.max_seconds.expect("max"), 0.3);
        assert_eq!(receive.intervals, 2);
        assert_eq!(receive.window_limit, 2);
        assert_eq!(receive.samples, 4);
    }

    #[test]
    fn receive_report_keeps_sample_count_before_first_interval() {
        let mut estimator = HzEstimator::new("/chatter".to_string(), window(10));
        estimator.observe_receive(Instant::now());

        let report = estimator.report();

        assert_eq!(report.receive.samples, 1);
        assert_eq!(report.receive.intervals, 0);
        assert_eq!(report.receive.window_limit, 10);
        assert_eq!(report.receive.rate_hz, None);
        assert_eq!(report.receive.min_seconds, None);
        assert_eq!(report.receive.max_seconds, None);
        assert_eq!(report.receive.stddev_seconds, None);
        assert!(report.sources.is_empty());
    }

    #[test]
    fn source_report_keeps_sample_count_before_first_interval() {
        let mut estimator = HzEstimator::new("/chatter".to_string(), window(10));
        estimator.observe_source(source(0x01), Time::from_nanos(0));

        let report = estimator.report();

        assert_eq!(report.sources.len(), 1);
        assert_eq!(report.sources[0].source, "01010101010101010101010101010101");
        assert_eq!(report.sources[0].stats.samples, 1);
        assert_eq!(report.sources[0].stats.intervals, 0);
        assert_eq!(report.sources[0].stats.window_limit, 10);
        assert_eq!(report.sources[0].stats.rate_hz, None);
    }

    #[test]
    fn report_serializes_no_rate_statistics_as_nulls() {
        let mut estimator = HzEstimator::new("/chatter".to_string(), window(10));
        estimator.observe_receive(Instant::now());
        estimator.observe_source(source(0x01), Time::from_nanos(0));

        let json = serde_json::to_value(estimator.report()).expect("serialize hz report");
        let receive = &json["receive"];
        assert!(receive["rate_hz"].is_null());
        assert!(receive["min_seconds"].is_null());
        assert!(receive["max_seconds"].is_null());
        assert!(receive["stddev_seconds"].is_null());
        assert_eq!(receive["samples"].as_u64(), Some(1));
        assert_eq!(receive["intervals"].as_u64(), Some(0));
        assert_eq!(receive["window_limit"].as_u64(), Some(10));

        let source = &json["sources"][0];
        assert!(source["rate_hz"].is_null());
        assert!(source["min_seconds"].is_null());
        assert!(source["max_seconds"].is_null());
        assert!(source["stddev_seconds"].is_null());
        assert_eq!(source["samples"].as_u64(), Some(1));
        assert_eq!(source["intervals"].as_u64(), Some(0));
        assert_eq!(source["window_limit"].as_u64(), Some(10));
    }

    #[test]
    fn source_windows_are_independent() {
        let mut estimator = HzEstimator::new("/chatter".to_string(), window(10));

        estimator.observe_source(source(0x01), Time::from_nanos(0));
        estimator.observe_source(source(0x02), Time::from_nanos(0));
        estimator.observe_source(source(0x01), Time::from_nanos(100_000_000));
        estimator.observe_source(source(0x02), Time::from_nanos(200_000_000));

        let report = estimator.report();
        assert_eq!(report.sources.len(), 2);
        assert_close(
            report.sources[0].stats.rate_hz.expect("first source rate"),
            10.0,
        );
        assert_close(
            report.sources[1].stats.rate_hz.expect("second source rate"),
            5.0,
        );
        assert_eq!(report.sources[0].stats.intervals, 1);
        assert_eq!(report.sources[0].stats.window_limit, 10);
        assert_eq!(report.sources[0].stats.samples, 2);
        assert_eq!(report.sources[1].stats.samples, 2);
    }
}
