use std::ops::Range;

use hungarian_algorithm::AssignmentProblem;
use ndarray::Array2;
use ordered_float::NotNan;

pub(super) fn solve_assignment_options<'a, T>(
    row_ranges: &[Range<usize>],
    options: &'a [T],
    column_count: usize,
    default_cost: NotNan<f32>,
    option_cost: impl Fn(&T) -> Option<(usize, NotNan<f32>)>,
) -> Vec<&'a T> {
    let mut costs = Array2::from_elem((row_ranges.len(), column_count), default_cost);
    for (row, range) in row_ranges.iter().enumerate() {
        for option in &options[range.clone()] {
            let Some((column, cost)) = option_cost(option) else {
                continue;
            };
            costs[(row, column)] = cost;
        }
    }

    AssignmentProblem::from_costs(costs)
        .solve()
        .into_iter()
        .enumerate()
        .filter_map(|(row, assignment)| {
            let assignment = assignment?;
            if assignment.cost <= 0.0 {
                return None;
            }
            options[row_ranges[row].clone()].iter().find(|option| {
                option_cost(option).is_some_and(|(column, _)| column == assignment.to)
            })
        })
        .collect()
}
