use std::cmp::Ordering;

use ndarray::{Array2, Axis};
use ordered_float::NotNan;
use pathfinding::kuhn_munkres::{kuhn_munkres, Weights};

pub struct AssignmentProblem {
    costs: Array2<NotNan<f32>>,

    number_of_workers: usize,
    number_of_tasks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Assignment {
    pub to: usize,
    pub cost: f32,
}

impl Weights<NotNan<f32>> for AssignmentProblem {
    fn rows(&self) -> usize {
        self.costs.nrows()
    }

    fn columns(&self) -> usize {
        self.costs.ncols()
    }

    fn at(&self, row: usize, col: usize) -> NotNan<f32> {
        self.costs[[row, col]]
    }

    fn neg(&self) -> Self
    where
        Self: Sized,
        NotNan<f32>: pathfinding::num_traits::Signed,
    {
        unimplemented!()
    }
}

impl AssignmentProblem {
    pub fn from_costs(costs: Array2<NotNan<f32>>) -> Self {
        let (number_of_tasks, number_of_workers) = costs.dim();
        let costs = match number_of_tasks.cmp(&number_of_workers) {
            Ordering::Less => {
                let new_tasks =
                    Array2::zeros((number_of_workers - number_of_tasks, number_of_workers));

                ndarray::concatenate![Axis(0), costs, new_tasks]
            }
            Ordering::Greater => {
                let new_costs =
                    Array2::zeros((number_of_tasks, number_of_tasks - number_of_workers));
                ndarray::concatenate![Axis(1), costs, new_costs]
            }
            Ordering::Equal => costs,
        };

        Self {
            costs,
            number_of_workers,
            number_of_tasks,
        }
    }

    pub fn solve(self) -> Vec<Option<Assignment>> {
        let (_, assignment) = kuhn_munkres(&self);

        assignment[..self.number_of_tasks]
            .iter()
            .enumerate()
            .map(|(task_index, &job_assignment)| {
                if job_assignment < self.number_of_workers {
                    let cost = self.costs[(task_index, job_assignment)];
                    Some(Assignment {
                        to: job_assignment,
                        cost: cost.into(),
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use ndarray::array;

    use super::*;
    trait Convert<O> {
        fn convert(self) -> O;
    }

    impl Convert<Array2<NotNan<f32>>> for Array2<f32> {
        fn convert(self) -> Array2<NotNan<f32>> {
            self.mapv(|x| NotNan::new(x).unwrap())
        }
    }

    #[test]
    fn test_assignment_problem() {
        let costs = array![[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]].convert();
        let problem = AssignmentProblem::from_costs(costs);

        let solution = problem.solve();
        assert_eq!(
            solution,
            vec![
                Some(Assignment { to: 0, cost: 1.0 }),
                Some(Assignment { to: 1, cost: 1.0 }),
                Some(Assignment { to: 2, cost: 1.0 })
            ]
        );
    }

    #[test]
    fn test_unbalanced_1() {
        let costs = array![[1., 0.9, 0.], [0.8, 0., 1.]].convert();
        let problem = AssignmentProblem::from_costs(costs);

        let solution = problem.solve();
        assert_eq!(
            solution,
            vec![
                Some(Assignment { to: 0, cost: 1.0 }),
                Some(Assignment { to: 2, cost: 1.0 })
            ]
        );
    }

    #[test]
    fn test_unbalanced_2() {
        let costs = array![[1., 0.], [0., 1.], [0., 2.]].convert();
        let problem = AssignmentProblem::from_costs(costs);

        let solution = problem.solve();
        assert_eq!(
            solution,
            vec![
                Some(Assignment { to: 0, cost: 1.0 }),
                None,
                Some(Assignment { to: 1, cost: 2.0 })
            ]
        );
    }
}
