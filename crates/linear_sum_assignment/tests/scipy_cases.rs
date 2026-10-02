use linear_sum_assignment::{AssignmentSolver, Objective};
use ndarray::{ArrayView1, ArrayView2};

struct Case {
    shape: (usize, usize),
    costs: &'static [f64],
    selected_costs: &'static [f64],
}

const CASES: &[Case] = &[
    Case {
        shape: (3, 3),
        costs: &[
            400.0, 150.0, 400.0, 400.0, 450.0, 600.0, 300.0, 225.0, 300.0,
        ],
        selected_costs: &[150.0, 400.0, 300.0],
    },
    Case {
        shape: (3, 4),
        costs: &[
            400.0, 150.0, 400.0, 1.0, 400.0, 450.0, 600.0, 2.0, 300.0, 225.0, 300.0, 3.0,
        ],
        selected_costs: &[150.0, 2.0, 300.0],
    },
    Case {
        shape: (3, 3),
        costs: &[10.0, 10.0, 8.0, 9.0, 8.0, 1.0, 9.0, 7.0, 4.0],
        selected_costs: &[10.0, 1.0, 7.0],
    },
    Case {
        shape: (3, 4),
        costs: &[
            10.0, 10.0, 8.0, 11.0, 9.0, 8.0, 1.0, 1.0, 9.0, 7.0, 4.0, 10.0,
        ],
        selected_costs: &[10.0, 1.0, 4.0],
    },
    Case {
        shape: (3, 3),
        costs: &[
            10.0,
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            f64::INFINITY,
            1.0,
            f64::INFINITY,
            7.0,
            f64::INFINITY,
        ],
        selected_costs: &[10.0, 1.0, 7.0],
    },
];

#[test]
fn scipy_reference_cases_match_in_both_orientations_and_objectives() {
    let mut solver = AssignmentSolver::default();

    for case in CASES {
        let base = ArrayView2::from_shape(case.shape, case.costs).unwrap();
        for (objective, sign) in [(Objective::Minimize, 1.0), (Objective::Maximize, -1.0)] {
            let costs = base.mapv(|cost| sign * cost);
            let expected = case
                .selected_costs
                .iter()
                .map(|cost| sign * cost)
                .collect::<Vec<_>>();

            assert_selected_costs(
                costs.view(),
                solver.solve(costs.view(), objective).unwrap(),
                &expected,
            );
            assert_selected_costs(
                costs.t(),
                solver.solve(costs.t(), objective).unwrap(),
                &expected,
            );
        }
    }
}

fn assert_selected_costs(
    costs: ArrayView2<'_, f64>,
    assignment: ArrayView1<'_, Option<usize>>,
    expected: &[f64],
) {
    let mut selected_columns = assignment.iter().flatten().copied().collect::<Vec<_>>();
    assert_eq!(selected_columns.len(), costs.nrows().min(costs.ncols()));
    selected_columns.sort_unstable();
    selected_columns.dedup();
    assert_eq!(selected_columns.len(), costs.nrows().min(costs.ncols()));

    let mut selected_costs = assignment
        .iter()
        .enumerate()
        .filter_map(|(row, column)| column.map(|column| costs[(row, column)]))
        .collect::<Vec<_>>();
    assert!(selected_costs.iter().all(|cost| cost.is_finite()));
    selected_costs.sort_by(f64::total_cmp);

    let mut expected = expected.to_vec();
    expected.sort_by(f64::total_cmp);
    assert_eq!(selected_costs, expected);
}
