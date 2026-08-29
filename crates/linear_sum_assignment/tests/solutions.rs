use linear_sum_assignment::{AssignmentError, AssignmentSolver, Objective};
use ndarray::{ArrayView1, ArrayView2, array, s};

fn assert_assignment(actual: ArrayView1<'_, Option<usize>>, expected: &[Option<usize>]) {
    assert_eq!(actual.as_slice(), Some(expected));
}

#[test]
fn matrix_without_rows_has_empty_assignment() {
    let costs = ArrayView2::<f32>::from_shape((0, 3), &[]).unwrap();
    let mut solver = AssignmentSolver::default();

    assert!(solver.solve(costs, Objective::Minimize).unwrap().is_empty());
}

#[test]
fn matrix_without_columns_leaves_every_row_unassigned() {
    let costs = ArrayView2::<f32>::from_shape((3, 0), &[]).unwrap();
    let mut solver = AssignmentSolver::default();

    assert_assignment(
        solver.solve(costs, Objective::Minimize).unwrap(),
        &[None, None, None],
    );
}

#[test]
fn wide_matrix_assigns_every_row() {
    let costs = array![[4.0, 1.0, 3.0], [2.0, 0.0, 5.0]];
    let mut solver = AssignmentSolver::default();

    assert_assignment(
        solver.solve(costs.view(), Objective::Minimize).unwrap(),
        &[Some(1), Some(0)],
    );
}

#[test]
fn tall_matrix_marks_unselected_rows_as_unassigned() {
    let costs = array![[1.0, 100.0], [2.0, 3.0], [100.0, 1.0]];
    let mut solver = AssignmentSolver::default();

    assert_assignment(
        solver.solve(costs.view(), Objective::Minimize).unwrap(),
        &[Some(0), None, Some(1)],
    );
}

#[test]
fn tall_matrix_follows_an_augmenting_path() {
    let costs = array![[4.0, 2.0], [1.0, 0.0], [3.0, 5.0]];
    let mut solver = AssignmentSolver::default();

    assert_assignment(
        solver.solve(costs.view(), Objective::Minimize).unwrap(),
        &[Some(1), Some(0), None],
    );
}

#[test]
fn maximize_selects_largest_total_weight() {
    let weights = array![[1.0, 2.0, 3.0], [2.0, 4.0, 6.0]];
    let mut solver = AssignmentSolver::default();

    assert_assignment(
        solver.solve(weights.view(), Objective::Maximize).unwrap(),
        &[Some(1), Some(2)],
    );
}

#[test]
fn tall_maximize_reconstructs_original_row_order() {
    let weights = array![[1.0, 2.0], [2.0, 4.0], [3.0, 6.0]];
    let mut solver = AssignmentSolver::default();

    assert_assignment(
        solver.solve(weights.view(), Objective::Maximize).unwrap(),
        &[None, Some(0), Some(1)],
    );
}

#[test]
fn constant_square_matrix_has_identity_tie_breaking() {
    let costs = [[1.0; 3]; 3];
    let mut solver = AssignmentSolver::default();

    assert_assignment(
        solver
            .solve(ArrayView2::from(&costs), Objective::Minimize)
            .unwrap(),
        &[Some(0), Some(1), Some(2)],
    );
}

#[test]
fn constant_tall_matrix_uses_identity_prefix_tie_breaking() {
    let costs = [[1.0; 2]; 3];
    let mut solver = AssignmentSolver::default();

    assert_assignment(
        solver
            .solve(ArrayView2::from(&costs), Objective::Minimize)
            .unwrap(),
        &[Some(0), Some(1), None],
    );
}

#[test]
fn non_contiguous_ndarray_view_is_supported() {
    let storage = array![
        [4.0, 9.0, 1.0, 9.0, 3.0, 9.0],
        [2.0, 9.0, 0.0, 9.0, 5.0, 9.0]
    ];
    let costs = storage.slice(s![.., ..;2]);
    let mut solver = AssignmentSolver::default();

    assert_assignment(
        solver.solve(costs, Objective::Minimize).unwrap(),
        &[Some(1), Some(0)],
    );
}

#[test]
fn explicit_dummy_columns_allow_optional_matching() {
    let weights = array![[10.0, 0.0, 0.0], [9.0, 0.0, 0.0]];
    let mut solver = AssignmentSolver::default();
    let assignment = solver.solve(weights.view(), Objective::Maximize).unwrap();

    assert_eq!(
        assignment
            .iter()
            .filter(|column| **column == Some(0))
            .count(),
        1
    );
    assert_eq!(
        assignment
            .iter()
            .filter(|column| column.is_some_and(|column| column > 0))
            .count(),
        1
    );
}

#[test]
fn solve_grows_capacity_to_fit_input() {
    let large = array![[1.0, 3.0, 2.0], [4.0, 0.0, 5.0]];
    let mut solver = AssignmentSolver::new((1, 1));

    let _ = solver.solve(large.view(), Objective::Minimize).unwrap();
    assert_eq!(solver.capacity(), (2, 3));
}

#[test]
fn solver_can_be_reused_for_a_different_shape() {
    let first = array![[1.0, 3.0, 2.0], [4.0, 0.0, 5.0]];
    let second = array![[2.0, 1.0]];
    let mut solver = AssignmentSolver::default();

    let _ = solver.solve(first.view(), Objective::Minimize).unwrap();
    assert_assignment(
        solver.solve(second.view(), Objective::Minimize).unwrap(),
        &[Some(1)],
    );
}

#[test]
fn f64_costs_are_supported() {
    let costs = array![[1.0_f64, 4.0], [3.0, 2.0]];
    let mut solver = AssignmentSolver::<f64>::default();

    assert_assignment(
        solver.solve(costs.view(), Objective::Minimize).unwrap(),
        &[Some(0), Some(1)],
    );
}

#[test]
fn overflowing_f32_intermediate_is_rejected() {
    let costs = array![
        [-f32::MAX, -1e38, 2e38],
        [-f32::MAX, 2e38, 2e38],
        [-f32::MAX, -1e38, f32::MAX]
    ];
    let mut solver = AssignmentSolver::default();

    assert_eq!(
        solver.solve(costs.view(), Objective::Minimize),
        Err(AssignmentError::ArithmeticOverflow)
    );
}
