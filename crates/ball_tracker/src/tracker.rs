use std::time::Duration;

use coordinate_systems::Ground;
use linear_algebra::{Isometry2, Point2, Vector2};
use nalgebra::Matrix2;
use types::ball_tracking::{TrackStatus, TrackerDebugState};

use crate::{
    FieldBounds,
    association::{AssociationChoice, AssociationKind, generate_assignments},
    config::TrackerParameters,
    imm::ImmBallState,
    measurement::Measurement,
};

pub type TrackId = u64;

#[derive(Clone, Debug)]
pub struct TrackSnapshot {
    pub id: TrackId,
    pub position: Point2<Ground>,
    pub velocity: Vector2<Ground>,
    pub covariance: Matrix2<f32>,
    pub existence_probability: f32,
    pub status: TrackStatus,
    pub last_seen_age: Duration,
}

#[derive(Clone, Debug)]
pub struct TrackerOutput {
    pub tracks: Vec<TrackSnapshot>,
    pub primary_track: Option<TrackSnapshot>,
    pub debug: TrackerDebugState,
}

#[derive(Clone, Debug)]
struct BernoulliTrack {
    id: TrackId,
    existence_probability: f32,
    state: ImmBallState,
    status: TrackStatus,
    last_seen_age: Duration,
    age: Duration,
    missed_count: u32,
    hit_count: u32,
}

#[derive(Clone, Debug)]
struct GlobalHypothesis {
    weight_log: f32,
    tracks: Vec<BernoulliTrack>,
}

#[derive(Clone, Debug)]
pub struct Tracker {
    global_hypotheses: Vec<GlobalHypothesis>,
    next_track_id: TrackId,
    previous_primary_track_id: Option<TrackId>,
    output_birth_existence_probability: f32,
    last_debug: TrackerDebugState,
}

impl Tracker {
    pub fn new() -> Self {
        Self {
            global_hypotheses: vec![GlobalHypothesis {
                weight_log: 0.0,
                tracks: Vec::new(),
            }],
            next_track_id: 1,
            previous_primary_track_id: None,
            output_birth_existence_probability: TrackerParameters::default()
                .birth_existence_probability,
            last_debug: TrackerDebugState {
                global_hypothesis_count: 1,
                best_hypothesis_weight_log: Some(0.0),
                track_count: 0,
                primary_track_id: None,
                assignment_count: 0,
                pruned_hypothesis_count: 0,
                pruned_track_count: 0,
            },
        }
    }

    fn allocate_track_id(&mut self) -> TrackId {
        let id = self.next_track_id;
        self.next_track_id += 1;
        id
    }

    fn create_birth(
        &mut self,
        measurement: &Measurement,
        parameters: &TrackerParameters,
    ) -> BernoulliTrack {
        BernoulliTrack {
            id: self.allocate_track_id(),
            existence_probability: (parameters.birth_existence_probability
                * measurement.confidence)
                .clamp(0.01, 0.99),
            state: ImmBallState::new_static(measurement.position, parameters),
            status: TrackStatus::Tentative,
            last_seen_age: Duration::ZERO,
            age: Duration::ZERO,
            missed_count: 0,
            hit_count: 1,
        }
    }

    fn update_status(track: &mut BernoulliTrack, parameters: &TrackerParameters) {
        if track.existence_probability >= parameters.confirm_existence_threshold
            && track.hit_count >= parameters.minimum_confirming_hits
        {
            track.status = TrackStatus::Confirmed;
        } else if track.status == TrackStatus::Confirmed && track.missed_count > 0 {
            track.status = TrackStatus::Stale;
        }
    }

    fn missed_existence_probability(
        track: &BernoulliTrack,
        detection_probability: f32,
        parameters: &TrackerParameters,
    ) -> f32 {
        let survival = parameters.survival_probability;
        let predicted_exists = track.existence_probability * survival;
        let missed_exists = predicted_exists * (1.0 - detection_probability);
        let missed_total = 1.0 - predicted_exists + missed_exists;
        if missed_total <= f32::MIN_POSITIVE {
            0.0
        } else {
            (missed_exists / missed_total).clamp(0.0, 0.99)
        }
    }

    fn track_snapshot(track: &BernoulliTrack) -> TrackSnapshot {
        TrackSnapshot {
            id: track.id,
            position: track.state.position(),
            velocity: track.state.velocity(),
            covariance: track.state.position_covariance(),
            existence_probability: track.existence_probability,
            status: track.status,
            last_seen_age: track.last_seen_age,
        }
    }

    pub fn update(
        &mut self,
        dt: Duration,
        odometry_delta: Isometry2<Ground, Ground>,
        measurements: &[Measurement],
        parameters: &TrackerParameters,
        field_bounds: FieldBounds,
        is_visible: impl Fn(Point2<Ground>) -> bool,
    ) -> TrackerOutput {
        self.output_birth_existence_probability = parameters.birth_existence_probability;
        let previous_hypothesis_count = self.global_hypotheses.len();
        let hypotheses = self.global_hypotheses.clone();
        let mut next_hypotheses = Vec::new();
        let mut assignment_count = 0;

        for mut hypothesis in hypotheses {
            for track in &mut hypothesis.tracks {
                track.state.predict(dt, odometry_delta, parameters);
                track.age += dt;
                track.last_seen_age += dt;
            }

            let choices_by_track: Vec<_> = hypothesis
                .tracks
                .iter()
                .map(|track| {
                    let detection_probability = if is_visible(track.state.position()) {
                        parameters.probability_of_detection_visible
                    } else {
                        parameters.probability_of_detection_hidden
                    };
                    let mut choices = Vec::new();
                    for (measurement_index, measurement) in measurements.iter().enumerate() {
                        let residual =
                            measurement.position.inner.coords - track.state.position().inner.coords;
                        let covariance = track.state.position_covariance() + measurement.covariance;
                        let Some(cholesky) = covariance.cholesky() else {
                            continue;
                        };
                        let mahalanobis = residual.dot(&cholesky.solve(&residual));
                        if mahalanobis <= parameters.gating_chi_square {
                            let assigned_weight = detection_probability.ln()
                                + track.existence_probability.clamp(0.01, 0.99).ln()
                                + track.state.measurement_log_likelihood(measurement)
                                + measurement.confidence_likelihood();
                            choices.push(AssociationChoice::assign(
                                track.id,
                                measurement_index,
                                assigned_weight,
                            ));
                        }
                    }
                    let missed_weight = (1.0 - detection_probability * track.existence_probability)
                        .clamp(0.01, 1.0)
                        .ln();
                    choices.push(AssociationChoice::miss(track.id, missed_weight));
                    choices
                })
                .collect();

            let assignments =
                generate_assignments(&choices_by_track, parameters.max_assignments_per_hypothesis);
            assignment_count += assignments.len();

            for assignment in assignments {
                let used_measurements = assignment.used_measurements();
                let mut tracks = hypothesis.tracks.clone();

                for choice in &assignment.choices {
                    let Some(track) = tracks.iter_mut().find(|track| track.id == choice.track_id)
                    else {
                        continue;
                    };
                    match choice.kind {
                        AssociationKind::Assigned { measurement_index } => {
                            let measurement = &measurements[measurement_index];
                            track.state.update(measurement);
                            track.existence_probability = (track.existence_probability
                                + measurement.confidence * (1.0 - track.existence_probability))
                                .clamp(0.0, 0.99);
                            track.last_seen_age = Duration::ZERO;
                            track.missed_count = 0;
                            track.hit_count += 1;
                            Self::update_status(track, parameters);
                        }
                        AssociationKind::Missed => {
                            let detection_probability = if is_visible(track.state.position()) {
                                parameters.probability_of_detection_visible
                            } else {
                                parameters.probability_of_detection_hidden
                            };
                            track.existence_probability = Self::missed_existence_probability(
                                track,
                                detection_probability,
                                parameters,
                            );
                            track.missed_count += 1;
                            Self::update_status(track, parameters);
                        }
                    }
                }

                let mut weight_log = hypothesis.weight_log + assignment.weight_log;
                for (measurement_index, measurement) in measurements.iter().enumerate() {
                    if !used_measurements.contains(&measurement_index) {
                        if field_bounds.contains(measurement.position) {
                            tracks.push(self.create_birth(measurement, parameters));
                            // Include the birth prior so nearby measurements prefer continuing an
                            // existing tentative track over spawning a replacement birth.
                            weight_log += parameters.birth_log_likelihood
                                + parameters
                                    .birth_existence_probability
                                    .clamp(0.01, 0.99)
                                    .ln()
                                + measurement.confidence_likelihood();
                        } else {
                            weight_log += parameters.clutter_log_likelihood;
                        }
                    }
                }

                next_hypotheses.push(GlobalHypothesis { weight_log, tracks });
            }
        }

        if next_hypotheses.is_empty() {
            next_hypotheses.push(GlobalHypothesis {
                weight_log: 0.0,
                tracks: Vec::new(),
            });
        }

        normalize_hypothesis_weights(&mut next_hypotheses);
        let pruned_track_count = prune_tracks(&mut next_hypotheses, parameters, field_bounds);
        let pruned_hypothesis_count =
            prune_hypotheses(&mut next_hypotheses, parameters, previous_hypothesis_count);
        self.global_hypotheses = next_hypotheses;

        let mut output = self.output();
        self.previous_primary_track_id = output.primary_track.as_ref().map(|track| track.id);
        output.debug.primary_track_id = self.previous_primary_track_id;
        self.last_debug = TrackerDebugState {
            global_hypothesis_count: self.global_hypotheses.len(),
            best_hypothesis_weight_log: self
                .best_hypothesis()
                .map(|hypothesis| hypothesis.weight_log),
            track_count: output.tracks.len(),
            primary_track_id: self.previous_primary_track_id,
            assignment_count,
            pruned_hypothesis_count,
            pruned_track_count,
        };
        output.debug = self.last_debug;
        output
    }

    pub fn output(&self) -> TrackerOutput {
        let tracks = self
            .best_hypothesis()
            .map(|hypothesis| {
                hypothesis
                    .tracks
                    .iter()
                    .filter(|track| {
                        track.status != TrackStatus::Tentative
                            || track.existence_probability
                                >= self.output_birth_existence_probability
                    })
                    .map(Self::track_snapshot)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let primary_track = tracks.iter().cloned().max_by(|left, right| {
            primary_score(left, self.previous_primary_track_id)
                .total_cmp(&primary_score(right, self.previous_primary_track_id))
        });

        TrackerOutput {
            tracks,
            primary_track,
            debug: self.last_debug,
        }
    }

    fn best_hypothesis(&self) -> Option<&GlobalHypothesis> {
        self.global_hypotheses
            .iter()
            .max_by(|left, right| left.weight_log.total_cmp(&right.weight_log))
    }
}

fn normalize_hypothesis_weights(hypotheses: &mut [GlobalHypothesis]) {
    let Some(max_weight_log) = hypotheses
        .iter()
        .map(|hypothesis| hypothesis.weight_log)
        .max_by(f32::total_cmp)
    else {
        return;
    };
    for hypothesis in hypotheses {
        hypothesis.weight_log -= max_weight_log;
    }
}

fn prune_tracks(
    hypotheses: &mut [GlobalHypothesis],
    parameters: &TrackerParameters,
    field_bounds: FieldBounds,
) -> usize {
    let mut pruned_track_count = 0;
    for hypothesis in hypotheses {
        let before_count = hypothesis.tracks.len();
        hypothesis.tracks.retain(|track| {
            (track.existence_probability >= parameters.delete_existence_threshold
                || track.missed_count <= 1)
                && field_bounds.contains(track.state.position())
                && track.last_seen_age <= track_timeout(track, parameters)
                && track.state.velocity().norm() <= parameters.max_ball_speed
        });
        hypothesis.tracks.sort_by(|left, right| {
            right
                .existence_probability
                .total_cmp(&left.existence_probability)
                .then_with(|| left.id.cmp(&right.id))
        });
        if hypothesis.tracks.len() > parameters.max_tracks_per_hypothesis {
            hypothesis
                .tracks
                .truncate(parameters.max_tracks_per_hypothesis);
        }
        pruned_track_count += before_count - hypothesis.tracks.len();
    }
    pruned_track_count
}

fn track_timeout(track: &BernoulliTrack, parameters: &TrackerParameters) -> Duration {
    match track.status {
        TrackStatus::Tentative => parameters.tentative_timeout,
        TrackStatus::Confirmed => parameters.confirmed_timeout,
        TrackStatus::Stale => parameters.stale_timeout,
    }
}

fn prune_hypotheses(
    hypotheses: &mut Vec<GlobalHypothesis>,
    parameters: &TrackerParameters,
    previous_hypothesis_count: usize,
) -> usize {
    let before_count = hypotheses.len();
    hypotheses.retain(|hypothesis| hypothesis.weight_log >= parameters.hypothesis_prune_log_weight);
    hypotheses.sort_by(|left, right| right.weight_log.total_cmp(&left.weight_log));
    hypotheses.truncate(parameters.max_global_hypotheses.max(1));
    if hypotheses.is_empty() {
        hypotheses.push(GlobalHypothesis {
            weight_log: 0.0,
            tracks: Vec::new(),
        });
    }
    before_count.saturating_sub(hypotheses.len().min(previous_hypothesis_count.max(1)))
}

fn primary_score(track: &TrackSnapshot, previous_primary_track_id: Option<TrackId>) -> f32 {
    let status_bonus = match track.status {
        TrackStatus::Confirmed => 2.0,
        TrackStatus::Stale => 0.5,
        TrackStatus::Tentative => -2.0,
    };
    let continuity_bonus = if Some(track.id) == previous_primary_track_id {
        0.5
    } else {
        0.0
    };
    let uncertainty_penalty = track.covariance.trace().sqrt();
    status_bonus + continuity_bonus + track.existence_probability - uncertainty_penalty
}

impl Default for Tracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use linear_algebra::Isometry2;

    use super::*;

    fn test_field() -> FieldBounds {
        FieldBounds {
            x_limit: 4.5,
            y_limit: 3.0,
        }
    }

    fn test_measurement(x: f32, y: f32) -> Measurement {
        Measurement {
            position: linear_algebra::point![x, y],
            covariance: nalgebra::Matrix2::identity() * 0.01,
            confidence: 1.0,
            image_location: None,
        }
    }

    fn update_visible(tracker: &mut Tracker, measurements: &[Measurement]) -> TrackerOutput {
        tracker.update(
            Duration::from_millis(100),
            Isometry2::identity(),
            measurements,
            &TrackerParameters::default(),
            test_field(),
            |_| true,
        )
    }

    #[test]
    fn new_tracker_has_empty_output() {
        let tracker = Tracker::new();
        let output = tracker.output();

        assert!(output.tracks.is_empty());
        assert!(output.primary_track.is_none());
        assert_eq!(output.debug.global_hypothesis_count, 1);
        assert_eq!(output.debug.track_count, 0);
    }

    #[test]
    fn repeated_stationary_measurements_confirm_a_track() {
        let mut tracker = Tracker::new();

        update_visible(&mut tracker, &[test_measurement(1.0, 0.0)]);
        let output = update_visible(&mut tracker, &[test_measurement(1.02, 0.0)]);

        assert!(
            output
                .tracks
                .iter()
                .any(|track| track.status == TrackStatus::Confirmed)
        );
    }

    #[test]
    fn isolated_false_positive_is_deleted_after_visible_misses() {
        let mut tracker = Tracker::new();
        let parameters = TrackerParameters {
            minimum_confirming_hits: 3,
            delete_existence_threshold: 0.2,
            ..Default::default()
        };

        tracker.update(
            Duration::from_millis(100),
            Isometry2::identity(),
            &[test_measurement(1.0, 0.0)],
            &parameters,
            test_field(),
            |_| true,
        );
        for _ in 0..8 {
            tracker.update(
                Duration::from_millis(100),
                Isometry2::identity(),
                &[],
                &parameters,
                test_field(),
                |_| true,
            );
        }
        let output = tracker.output();

        assert!(output.tracks.is_empty());
    }

    #[test]
    fn hidden_miss_preserves_track_better_than_visible_miss() {
        let mut visible_tracker = Tracker::new();
        let mut hidden_tracker = Tracker::new();
        let parameters = TrackerParameters {
            minimum_confirming_hits: 4,
            confirm_existence_threshold: 1.0,
            ..Default::default()
        };

        for tracker in [&mut visible_tracker, &mut hidden_tracker] {
            for _ in 0..3 {
                tracker.update(
                    Duration::from_millis(100),
                    Isometry2::identity(),
                    &[test_measurement(1.0, 0.0)],
                    &parameters,
                    test_field(),
                    |_| true,
                );
            }
        }

        let visible = visible_tracker.update(
            Duration::from_millis(100),
            Isometry2::identity(),
            &[],
            &parameters,
            test_field(),
            |_| true,
        );
        let hidden = hidden_tracker.update(
            Duration::from_millis(100),
            Isometry2::identity(),
            &[],
            &parameters,
            test_field(),
            |_| false,
        );

        assert!(hidden.tracks[0].existence_probability > visible.tracks[0].existence_probability);
    }

    #[test]
    fn output_excludes_tentative_tracks_below_birth_threshold() {
        let mut tracker = Tracker::new();

        update_visible(&mut tracker, &[test_measurement(1.0, 0.0)]);
        let output = update_visible(&mut tracker, &[]);

        assert!(output.tracks.is_empty());
        assert!(output.primary_track.is_none());
    }

    #[test]
    fn two_visible_balls_create_two_tracks() {
        let mut tracker = Tracker::new();

        update_visible(
            &mut tracker,
            &[test_measurement(1.0, 0.0), test_measurement(-1.0, 0.0)],
        );
        let output = update_visible(
            &mut tracker,
            &[test_measurement(1.02, 0.0), test_measurement(-1.02, 0.0)],
        );

        assert_eq!(output.tracks.len(), 2);
    }

    #[test]
    fn primary_ball_survives_far_false_positive() {
        let mut tracker = Tracker::new();

        update_visible(&mut tracker, &[test_measurement(1.0, 0.0)]);
        let first_primary = update_visible(&mut tracker, &[test_measurement(1.01, 0.0)])
            .primary_track
            .unwrap()
            .id;
        let output = update_visible(
            &mut tracker,
            &[test_measurement(1.02, 0.0), test_measurement(-2.0, 1.0)],
        );

        assert_eq!(output.primary_track.unwrap().id, first_primary);
    }

    #[test]
    fn tracks_outside_field_are_pruned() {
        let mut tracker = Tracker::new();

        tracker.update(
            Duration::from_millis(100),
            Isometry2::identity(),
            &[test_measurement(10.0, 0.0)],
            &TrackerParameters::default(),
            test_field(),
            |_| true,
        );
        let output = tracker.output();

        assert!(output.tracks.is_empty());
    }
}
