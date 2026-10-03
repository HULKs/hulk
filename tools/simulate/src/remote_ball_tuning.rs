//! Read-only snapshots from the SSH bridge. Local ROS-Z heartbeats must never
//! make a disconnected remote search appear current or apply stale candidates.
use serde::Deserialize;
use types::ball_filter_tuning::{RemoteProgress, SearchProgress};

const MAX_SNAPSHOT_AGE_SECONDS: f64 = 15.0;

#[derive(Deserialize)]
pub struct RemoteSnapshot {
    pub updated_unix_seconds: f64,
    pub error: Option<String>,
    pub remote: RemoteProgress,
    pub search: Option<SearchProgress>,
}

impl RemoteSnapshot {
    pub fn problem(&self, now: f64) -> Option<String> {
        if let Some(error) = &self.error {
            return Some(error.clone());
        }
        if !self.updated_unix_seconds.is_finite()
            || self.updated_unix_seconds <= 0.0
            || self.updated_unix_seconds > now + 5.0
        {
            return Some("Remote bridge has no valid observation timestamp".into());
        }
        let age = now - self.updated_unix_seconds;
        if age > MAX_SNAPSHOT_AGE_SECONDS {
            return Some(format!(
                "Remote optimizer has not been reached for {age:.0}s; retaining the last preview parameters"
            ));
        }
        match (&self.search, self.remote.best_candidate.is_empty()) {
            (Some(_), true) | (None, false) => {
                Some("Remote bridge candidate identity and search result disagree".into())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> RemoteSnapshot {
        RemoteSnapshot {
            updated_unix_seconds: 100.0,
            error: None,
            remote: RemoteProgress {
                best_candidate: "remote/run/worker-00/round-000001".into(),
                ..Default::default()
            },
            search: Some(SearchProgress::default()),
        }
    }

    #[test]
    fn disconnected_bridge_cannot_apply_old_parameters_despite_local_heartbeat() {
        let mut state = snapshot();
        assert!(state.problem(102.0).is_none());
        assert!(state.problem(116.0).unwrap().contains("retaining"));
        state.error = Some("SSH authentication failed".into());
        assert_eq!(state.problem(102.0).unwrap(), "SSH authentication failed");
    }

    #[test]
    fn pending_search_is_valid_but_inconsistent_candidate_is_not() {
        let mut state = snapshot();
        state.search = None;
        assert!(state.problem(102.0).is_some());
        state.remote.best_candidate.clear();
        assert!(state.problem(102.0).is_none());
        state.search = Some(SearchProgress::default());
        assert!(state.problem(102.0).is_some());
    }

    #[test]
    fn invalid_observation_times_never_look_fresh() {
        let mut state = snapshot();
        for timestamp in [0.0, f64::NAN, f64::INFINITY, 200.0] {
            state.updated_unix_seconds = timestamp;
            assert!(state.problem(102.0).is_some());
        }
    }
}
