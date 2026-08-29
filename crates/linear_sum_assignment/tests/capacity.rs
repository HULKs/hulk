use linear_sum_assignment::AssignmentSolver;

#[test]
fn default_solver_starts_without_capacity() {
    let solver = AssignmentSolver::<f32>::default();

    assert_eq!(solver.capacity(), (0, 0));
}

#[test]
fn reserve_grows_each_dimension_independently_without_shrinking() {
    let mut solver = AssignmentSolver::<f32>::new((2, 5));

    solver.reserve((4, 3));
    assert_eq!(solver.capacity(), (4, 5));

    solver.reserve((1, 2));
    assert_eq!(solver.capacity(), (4, 5));
}
