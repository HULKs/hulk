//! Dense rectangular linear sum assignment.
//!
//! This crate provides a reusable implementation of the shortest augmenting path algorithm
//! described by D. F. Crouse in "On implementing 2D rectangular assignment algorithms",
//! IEEE Transactions on Aerospace and Electronic Systems 52(4), 2016.
//! Its implementation follows SciPy's
//! [`rectangular_lsap`](https://github.com/scipy/scipy/blob/main/scipy/optimize/rectangular_lsap/rectangular_lsap.cpp)
//! implementation.

use std::{error::Error, fmt::Display};

use ndarray::{ArrayView1, ArrayView2};
use num_traits::Float;

/// Whether [`AssignmentSolver`] minimizes costs or maximizes weights.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Objective {
    /// Find the assignment with the smallest sum.
    Minimize,
    /// Find the assignment with the largest sum.
    Maximize,
}

/// Failure to construct a complete assignment of the smaller matrix dimension.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignmentError {
    /// The matrix contains a value that is invalid for the selected [`Objective`].
    ///
    /// `NaN` is always invalid. Negative infinity is invalid when minimizing, and positive
    /// infinity is invalid when maximizing. The opposite infinity represents a forbidden edge.
    InvalidValue {
        /// Row containing the invalid value.
        row: usize,
        /// Column containing the invalid value.
        column: usize,
    },
    /// Forbidden edges prevent a complete assignment of the smaller matrix dimension.
    Infeasible,
    /// Finite values overflowed during shortest-path or dual-variable arithmetic.
    ArithmeticOverflow,
}

impl Display for AssignmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidValue { row, column } => {
                write!(formatter, "invalid assignment value at ({row}, {column})")
            }
            Self::Infeasible => formatter.write_str("assignment problem is infeasible"),
            Self::ArithmeticOverflow => formatter.write_str("assignment arithmetic overflowed"),
        }
    }
}

impl Error for AssignmentError {}

/// Reusable solver for dense rectangular linear sum assignment problems.
///
/// The solver reserves scratch storage for `initial_capacity` and grows it automatically when a
/// larger matrix is passed to [`solve`](Self::solve). Call [`reserve`](Self::reserve) before a
/// latency-sensitive solve to avoid workspace growth during that solve.
///
/// A solution contains one entry per input row. `Some(column)` identifies the assigned input
/// column. Rows left unmatched because the matrix has more rows than columns contain `None`.
#[derive(Debug)]
pub struct AssignmentSolver<T = f32> {
    capacity: (usize, usize),
    row_potentials: Vec<T>,
    column_potentials: Vec<T>,
    shortest_path_costs: Vec<T>,
    path: Vec<usize>,
    column_for_row: Vec<Option<usize>>,
    row_for_column: Vec<Option<usize>>,
    remaining_columns: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
struct AugmentingPath<T> {
    sink: usize,
    minimum: T,
    visited_start: usize,
}

impl<T> AssignmentSolver<T> {
    /// Creates a solver with scratch capacity for the given `(rows, columns)` shape.
    pub fn new(initial_capacity: (usize, usize)) -> Self {
        let (rows, columns) = initial_capacity;
        let shorter_dimension = rows.min(columns);
        let longer_dimension = rows.max(columns);

        Self {
            capacity: initial_capacity,
            row_potentials: Vec::with_capacity(shorter_dimension),
            column_potentials: Vec::with_capacity(longer_dimension),
            shortest_path_costs: Vec::with_capacity(longer_dimension),
            path: Vec::with_capacity(longer_dimension),
            column_for_row: Vec::with_capacity(shorter_dimension),
            row_for_column: Vec::with_capacity(longer_dimension),
            remaining_columns: Vec::with_capacity(longer_dimension),
        }
    }

    /// Returns the largest `(rows, columns)` shape guaranteed not to require growth.
    pub fn capacity(&self) -> (usize, usize) {
        self.capacity
    }

    /// Ensures capacity for at least the given `(rows, columns)` shape.
    ///
    /// Existing capacity is never reduced.
    pub fn reserve(&mut self, minimum_capacity: (usize, usize)) {
        self.capacity.0 = self.capacity.0.max(minimum_capacity.0);
        self.capacity.1 = self.capacity.1.max(minimum_capacity.1);

        let shorter_dimension = self.capacity.0.min(self.capacity.1);
        let longer_dimension = self.capacity.0.max(self.capacity.1);
        reserve_for_length(&mut self.row_potentials, shorter_dimension);
        reserve_for_length(&mut self.column_potentials, longer_dimension);
        reserve_for_length(&mut self.shortest_path_costs, longer_dimension);
        reserve_for_length(&mut self.path, longer_dimension);
        reserve_for_length(&mut self.column_for_row, shorter_dimension);
        reserve_for_length(&mut self.row_for_column, longer_dimension);
        reserve_for_length(&mut self.remaining_columns, longer_dimension);
    }
}

impl<T> Default for AssignmentSolver<T> {
    fn default() -> Self {
        Self::new((0, 0))
    }
}

impl<T> AssignmentSolver<T>
where
    T: Float,
{
    /// Solves a dense rectangular assignment problem.
    ///
    /// The input may be any two-dimensional ndarray view, including non-contiguous slices and
    /// views over fixed Rust arrays. The returned view remains valid until this solver is mutably
    /// borrowed again.
    ///
    /// For optional matching, append explicit dummy rows or columns containing the desired
    /// unassignment cost. Dummy columns are returned as their ordinary `Some(column)` index and
    /// must be interpreted by the caller. When dummy rows are appended, the caller must ignore
    /// their corresponding output entries.
    ///
    /// Ties are resolved deterministically using Crouse's traversal order: rows are augmented in
    /// ascending order, remaining columns start in reverse order, and an unmatched sink is
    /// preferred when shortest-path costs are equal.
    ///
    /// # Errors
    ///
    /// Returns [`AssignmentError::InvalidValue`] for `NaN` or an infinity whose sign is invalid
    /// for `objective`, [`AssignmentError::Infeasible`] when forbidden edges prevent a complete
    /// assignment, and [`AssignmentError::ArithmeticOverflow`] when finite intermediate
    /// arithmetic produces a non-finite value.
    pub fn solve(
        &mut self,
        costs: ArrayView2<'_, T>,
        objective: Objective,
    ) -> Result<ArrayView1<'_, Option<usize>>, AssignmentError> {
        validate_costs(&costs, objective)?;

        let (original_rows, original_columns) = costs.dim();
        self.reserve(costs.dim());
        let transposed = original_rows > original_columns;
        let costs = if transposed {
            costs.reversed_axes()
        } else {
            costs
        };
        let (rows, columns) = costs.dim();
        self.prepare(rows, columns);

        for current_row in 0..rows {
            let path = self.find_augmenting_path(&costs, objective, current_row)?;
            self.update_duals(current_row, path)?;
            self.augment_matching(current_row, path.sink);
        }

        let assignment = if transposed {
            self.row_for_column.as_slice()
        } else {
            self.column_for_row.as_slice()
        };

        Ok(ArrayView1::from(assignment))
    }

    fn prepare(&mut self, rows: usize, columns: usize) {
        let zero = T::zero();

        self.row_potentials.clear();
        self.row_potentials.resize(rows, zero);
        self.column_potentials.clear();
        self.column_potentials.resize(columns, zero);
        self.shortest_path_costs.resize(columns, T::infinity());
        self.path.resize(columns, 0);
        self.column_for_row.clear();
        self.column_for_row.resize(rows, None);
        self.row_for_column.clear();
        self.row_for_column.resize(columns, None);
    }

    fn find_augmenting_path(
        &mut self,
        costs: &ArrayView2<'_, T>,
        objective: Objective,
        current_row: usize,
    ) -> Result<AugmentingPath<T>, AssignmentError> {
        let columns = costs.ncols();
        let infinity = T::infinity();
        self.shortest_path_costs.fill(infinity);
        self.remaining_columns.clear();
        self.remaining_columns.extend((0..columns).rev());

        let mut row = current_row;
        let mut minimum = T::zero();
        let mut remaining = columns;
        let sink = loop {
            let mut lowest = infinity;
            let mut lowest_index = None;

            for index in 0..remaining {
                let column = self.remaining_columns[index];
                let mut cost = costs[(row, column)];
                if cost.is_finite() {
                    if objective == Objective::Maximize {
                        cost = -cost;
                    }

                    let reduced_cost = checked_add(minimum, cost)?;
                    let reduced_cost = checked_sub(reduced_cost, self.row_potentials[row])?;
                    let reduced_cost = checked_sub(reduced_cost, self.column_potentials[column])?;
                    if reduced_cost < self.shortest_path_costs[column] {
                        self.path[column] = row;
                        self.shortest_path_costs[column] = reduced_cost;
                    }
                }

                let path_cost = self.shortest_path_costs[column];
                if path_cost < lowest
                    || (path_cost == lowest && self.row_for_column[column].is_none())
                {
                    lowest = path_cost;
                    lowest_index = Some(index);
                }
            }

            minimum = lowest;
            if minimum == infinity {
                return Err(AssignmentError::Infeasible);
            }

            let index = lowest_index.expect("a finite path cost has a candidate column");
            remaining -= 1;
            self.remaining_columns.swap(index, remaining);
            let column = self.remaining_columns[remaining];

            if let Some(next_row) = self.row_for_column[column] {
                row = next_row;
            } else {
                break column;
            }
        };

        Ok(AugmentingPath {
            sink,
            minimum,
            visited_start: remaining,
        })
    }

    fn update_duals(
        &mut self,
        current_row: usize,
        path: AugmentingPath<T>,
    ) -> Result<(), AssignmentError> {
        self.row_potentials[current_row] =
            checked_add(self.row_potentials[current_row], path.minimum)?;

        for index in path.visited_start..self.remaining_columns.len() {
            let column = self.remaining_columns[index];
            if let Some(row) = self.row_for_column[column] {
                let change = checked_sub(path.minimum, self.shortest_path_costs[column])?;
                self.row_potentials[row] = checked_add(self.row_potentials[row], change)?;
            }
        }
        for index in path.visited_start..self.remaining_columns.len() {
            let column = self.remaining_columns[index];
            let change = checked_sub(path.minimum, self.shortest_path_costs[column])?;
            self.column_potentials[column] = checked_sub(self.column_potentials[column], change)?;
        }

        Ok(())
    }

    fn augment_matching(&mut self, current_row: usize, sink: usize) {
        let mut column = sink;
        loop {
            // A predecessor is current whenever the shortest-path cost is finite.
            let row = self.path[column];
            self.row_for_column[column] = Some(row);
            let previous_column = self.column_for_row[row].replace(column);
            if row == current_row {
                break;
            }
            column = previous_column.expect("a non-root path row was already assigned");
        }
    }
}

fn validate_costs<T: Float>(
    costs: &ArrayView2<'_, T>,
    objective: Objective,
) -> Result<(), AssignmentError> {
    for ((row, column), &value) in costs.indexed_iter() {
        let invalid_infinity = match objective {
            Objective::Minimize => value == T::neg_infinity(),
            Objective::Maximize => value == T::infinity(),
        };
        if value.is_nan() || invalid_infinity {
            return Err(AssignmentError::InvalidValue { row, column });
        }
    }

    Ok(())
}

fn reserve_for_length<T>(values: &mut Vec<T>, length: usize) {
    if values.capacity() < length {
        values.reserve(length - values.len());
    }
}

fn checked_add<T: Float>(left: T, right: T) -> Result<T, AssignmentError> {
    let result = left + right;
    result
        .is_finite()
        .then_some(result)
        .ok_or(AssignmentError::ArithmeticOverflow)
}

fn checked_sub<T: Float>(left: T, right: T) -> Result<T, AssignmentError> {
    let result = left - right;
    result
        .is_finite()
        .then_some(result)
        .ok_or(AssignmentError::ArithmeticOverflow)
}
