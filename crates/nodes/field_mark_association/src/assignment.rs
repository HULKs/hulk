use linear_sum_assignment::{AssignmentSolver, Objective};
use ndarray::Array2;

/// One zero-benefit dummy per detection permits outliers. Test alternatives by
/// excluding each winning edge; never silently drop an ambiguous winning edge.
pub(crate) fn unique_assignment(
    benefits: &mut Array2<f32>,
    landmarks: usize,
    score_ratio: f32,
) -> Option<Vec<(usize, usize)>> {
    let mut assignment = AssignmentSolver::new(benefits.dim());
    let columns = assignment
        .solve(benefits.view(), Objective::Maximize)
        .ok()?;
    let pairs = columns
        .iter()
        .enumerate()
        .filter_map(|(row, column)| {
            let column = (*column)?;
            (column < landmarks && benefits[(row, column)] > 0.0).then_some((row, column))
        })
        .collect::<Vec<_>>();
    let score = pairs.iter().map(|&(r, c)| benefits[(r, c)]).sum::<f32>();
    for &(row, column) in &pairs {
        let saved = benefits[(row, column)];
        benefits[(row, column)] = 0.0;
        let alternative = assignment
            .solve(benefits.view(), Objective::Maximize)
            .ok()?;
        let alternative_score = alternative
            .iter()
            .enumerate()
            .filter_map(|(r, c)| c.map(|c| benefits[(r, c)]))
            .sum::<f32>();
        benefits[(row, column)] = saved;
        if score - alternative_score <= saved * (1.0 - 1.0 / score_ratio) {
            return None;
        }
    }
    Some(pairs)
}
