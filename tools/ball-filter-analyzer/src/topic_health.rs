use crate::records::TopicHealthRow;

#[derive(Clone, Debug, Default)]
pub struct TopicStats {
    message_count: i64,
    first_time_ns: Option<i64>,
    last_time_ns: Option<i64>,
    max_gap_ns: Option<i64>,
}

impl TopicStats {
    pub fn observe(&mut self, time_ns: i64) {
        self.message_count += 1;
        if self.first_time_ns.is_none() {
            self.first_time_ns = Some(time_ns);
        }
        if let Some(last_time_ns) = self.last_time_ns {
            let gap = time_ns.saturating_sub(last_time_ns);
            self.max_gap_ns = Some(self.max_gap_ns.unwrap_or(0).max(gap));
        }
        self.last_time_ns = Some(time_ns);
    }

    pub fn to_row(&self, topic: &str, _required: bool) -> TopicHealthRow {
        let average_rate_hz = match (self.first_time_ns, self.last_time_ns, self.message_count) {
            (Some(first), Some(last), count) if count > 1 && last > first => {
                Some((count - 1) as f64 / ((last - first) as f64 / 1_000_000_000.0))
            }
            _ => None,
        };

        TopicHealthRow {
            topic: topic.to_string(),
            message_count: self.message_count,
            first_time_ns: self.first_time_ns,
            last_time_ns: self.last_time_ns,
            average_rate_hz,
            max_gap_ms: self.max_gap_ns.map(|gap| gap as f64 / 1_000_000.0),
            missing: self.message_count == 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TopicStats;

    #[test]
    fn topic_stats_compute_count_rate_and_max_gap() {
        let mut stats = TopicStats::default();
        stats.observe(1_000_000_000);
        stats.observe(1_500_000_000);
        stats.observe(2_500_000_000);

        let row = stats.to_row("ball_filter/tracks", true);

        assert_eq!(row.topic, "ball_filter/tracks");
        assert_eq!(row.message_count, 3);
        assert_eq!(row.first_time_ns, Some(1_000_000_000));
        assert_eq!(row.last_time_ns, Some(2_500_000_000));
        assert_eq!(row.max_gap_ms, Some(1000.0));
        assert!((row.average_rate_hz.unwrap() - 1.3333334).abs() < 1e-5);
        assert!(!row.missing);
    }

    #[test]
    fn missing_topic_stats_mark_topic_missing() {
        let stats = TopicStats::default();

        let row = stats.to_row("ball_filter/debug_state", false);

        assert_eq!(row.message_count, 0);
        assert!(row.first_time_ns.is_none());
        assert!(row.last_time_ns.is_none());
        assert!(row.average_rate_hz.is_none());
        assert!(row.max_gap_ms.is_none());
        assert!(row.missing);
    }
}
