use std::collections::BTreeSet;

use crate::tracker::TrackId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AssociationKind {
    Assigned { measurement_index: usize },
    Missed,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AssociationChoice {
    pub track_id: TrackId,
    pub kind: AssociationKind,
    pub weight_log: f32,
}

impl AssociationChoice {
    pub(crate) fn assign(track_id: TrackId, measurement_index: usize, weight_log: f32) -> Self {
        Self {
            track_id,
            kind: AssociationKind::Assigned { measurement_index },
            weight_log,
        }
    }

    pub(crate) fn miss(track_id: TrackId, weight_log: f32) -> Self {
        Self {
            track_id,
            kind: AssociationKind::Missed,
            weight_log,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Assignment {
    pub choices: Vec<AssociationChoice>,
    pub weight_log: f32,
}

impl Assignment {
    #[cfg(test)]
    pub(crate) fn measurement_is_used_once(&self) -> bool {
        let mut used = BTreeSet::new();
        for choice in &self.choices {
            if let AssociationKind::Assigned { measurement_index } = choice.kind
                && !used.insert(measurement_index)
            {
                return false;
            }
        }
        true
    }

    pub(crate) fn used_measurements(&self) -> BTreeSet<usize> {
        self.choices
            .iter()
            .filter_map(|choice| match choice.kind {
                AssociationKind::Assigned { measurement_index } => Some(measurement_index),
                AssociationKind::Missed => None,
            })
            .collect()
    }
}

pub(crate) fn generate_assignments(
    choices_by_track: &[Vec<AssociationChoice>],
    maximum_assignments: usize,
) -> Vec<Assignment> {
    let mut assignments = Vec::new();
    enumerate_assignments(
        choices_by_track,
        0,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        0.0,
        &mut assignments,
    );
    assignments.sort_by(|left, right| right.weight_log.total_cmp(&left.weight_log));
    assignments.truncate(maximum_assignments.max(1));
    assignments
}

fn enumerate_assignments(
    choices_by_track: &[Vec<AssociationChoice>],
    track_index: usize,
    used_measurements: &mut BTreeSet<usize>,
    current_choices: &mut Vec<AssociationChoice>,
    current_weight_log: f32,
    assignments: &mut Vec<Assignment>,
) {
    if track_index == choices_by_track.len() {
        assignments.push(Assignment {
            choices: current_choices.clone(),
            weight_log: current_weight_log,
        });
        return;
    }

    for choice in &choices_by_track[track_index] {
        let inserted_measurement = match choice.kind {
            AssociationKind::Assigned { measurement_index } => {
                if !used_measurements.insert(measurement_index) {
                    continue;
                }
                Some(measurement_index)
            }
            AssociationKind::Missed => None,
        };

        current_choices.push(*choice);
        enumerate_assignments(
            choices_by_track,
            track_index + 1,
            used_measurements,
            current_choices,
            current_weight_log + choice.weight_log,
            assignments,
        );
        current_choices.pop();

        if let Some(measurement_index) = inserted_measurement {
            used_measurements.remove(&measurement_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignments_do_not_reuse_measurements() {
        let choices = vec![
            vec![
                AssociationChoice::assign(1, 0, 4.0),
                AssociationChoice::miss(1, -2.0),
            ],
            vec![
                AssociationChoice::assign(2, 0, 3.0),
                AssociationChoice::miss(2, -1.0),
            ],
        ];

        let assignments = generate_assignments(&choices, 8);

        assert!(
            assignments
                .iter()
                .all(|assignment| assignment.measurement_is_used_once())
        );
    }

    #[test]
    fn assignments_are_sorted_by_weight() {
        let choices = vec![
            vec![
                AssociationChoice::assign(1, 0, 1.0),
                AssociationChoice::miss(1, -2.0),
            ],
            vec![
                AssociationChoice::assign(2, 1, 3.0),
                AssociationChoice::miss(2, -1.0),
            ],
        ];

        let assignments = generate_assignments(&choices, 4);

        assert!(assignments[0].weight_log >= assignments[1].weight_log);
    }
}
