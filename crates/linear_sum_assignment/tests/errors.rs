use linear_sum_assignment::{AssignmentError, AssignmentSolver, Objective};
use ndarray::array;

#[test]
fn nan_is_rejected_with_its_location() {
    let costs = array![[0.0, f32::NAN], [1.0, 2.0]];
    let mut solver = AssignmentSolver::default();

    assert_eq!(
        solver.solve(costs.view(), Objective::Minimize),
        Err(AssignmentError::InvalidValue { row: 0, column: 1 })
    );
}

#[test]
fn infinity_with_wrong_sign_is_rejected_for_objective() {
    let costs = array![[0.0, f32::NEG_INFINITY]];
    let weights = array![[0.0, f32::INFINITY]];
    let mut solver = AssignmentSolver::default();

    assert_eq!(
        solver.solve(costs.view(), Objective::Minimize),
        Err(AssignmentError::InvalidValue { row: 0, column: 1 })
    );
    assert_eq!(
        solver.solve(weights.view(), Objective::Maximize),
        Err(AssignmentError::InvalidValue { row: 0, column: 1 })
    );
}

#[test]
fn infinity_with_allowed_sign_forbids_an_edge() {
    let costs = array![
        [0.0, f32::INFINITY, f32::INFINITY],
        [0.0, 1.0, f32::INFINITY]
    ];
    let weights = costs.mapv(|cost| -cost);
    let mut solver = AssignmentSolver::default();

    let assignment = solver.solve(costs.view(), Objective::Minimize).unwrap();
    assert_eq!(assignment.as_slice(), Some([Some(0), Some(1)].as_slice()));
    let assignment = solver.solve(costs.t(), Objective::Minimize).unwrap();
    assert_eq!(
        assignment.as_slice(),
        Some([Some(0), Some(1), None].as_slice())
    );

    let assignment = solver.solve(weights.view(), Objective::Maximize).unwrap();
    assert_eq!(assignment.as_slice(), Some([Some(0), Some(1)].as_slice()));
    let assignment = solver.solve(weights.t(), Objective::Maximize).unwrap();
    assert_eq!(
        assignment.as_slice(),
        Some([Some(0), Some(1), None].as_slice())
    );
}

#[test]
fn wide_matrix_without_distinct_columns_for_each_row_is_infeasible() {
    let costs = array![
        [1.0, f32::INFINITY, f32::INFINITY],
        [2.0, f32::INFINITY, f32::INFINITY]
    ];
    let mut solver = AssignmentSolver::default();

    assert_eq!(
        solver.solve(costs.view(), Objective::Minimize),
        Err(AssignmentError::Infeasible)
    );
}

#[test]
fn tall_matrix_without_a_candidate_for_each_column_is_infeasible() {
    let costs = array![
        [1.0, f32::INFINITY],
        [2.0, f32::INFINITY],
        [3.0, f32::INFINITY]
    ];
    let mut solver = AssignmentSolver::default();

    assert_eq!(
        solver.solve(costs.view(), Objective::Minimize),
        Err(AssignmentError::Infeasible)
    );
}
