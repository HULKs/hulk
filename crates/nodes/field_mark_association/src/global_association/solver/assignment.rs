use std::ops::Range;

use color_eyre::eyre::WrapErr;
use linear_sum_assignment::{AssignmentSolver, Objective};
use ndarray::Array2;
use ordered_float::NotNan;

pub(super) fn solve_assignment_options<'a, T>(
    row_ranges: &[Range<usize>],
    options: &'a [T],
    column_count: usize,
    default_cost: NotNan<f32>,
    option_cost: impl Fn(&T) -> Option<(usize, NotNan<f32>)>,
) -> Vec<&'a T> {
    let mut costs = Array2::from_elem((row_ranges.len(), column_count), default_cost.into_inner());
    for (row, range) in row_ranges.iter().enumerate() {
        for option in &options[range.clone()] {
            let Some((column, cost)) = option_cost(option) else {
                continue;
            };
            costs[(row, column)] = cost.into_inner();
        }
    }

    let mut solver = AssignmentSolver::new(costs.dim());
    let assignment = match solver
        .solve(costs.view(), Objective::Maximize)
        .wrap_err("failed to solve field mark assignment")
    {
        Ok(assignment) => assignment,
        Err(error) => {
            log::error!("{error}");
            return Vec::new();
        }
    };
    assignment
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(row, column)| {
            let column = column?;
            if costs[(row, column)] <= 0.0 {
                return None;
            }
            options[row_ranges[row].clone()].iter().find(|option| {
                option_cost(option).is_some_and(|(option_column, _)| option_column == column)
            })
        })
        .collect()
}
