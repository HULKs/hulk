use std::collections::{HashMap, HashSet};

use crate::{factors::prior::SchurPriorResidual, symbols::State};
use factrs::{
    containers::{FactorBuilder, Key, ValuesOrder},
    core::{GaussNewton, Graph, Values},
    residuals::DynVarPack,
    traits::Optimizer,
    variables::{SE23, VariableSafe},
};
use faer::{Conj, Mat, MatMut, MatRef, Par, prelude::ReborrowMut};
use faer_ext::IntoNalgebra;
use itertools::Itertools;

const RANK_RELATIVE_TOLERANCE_SCALE: f64 = 64.0;

pub fn marginalize(optimizer: &mut GaussNewton, values: &mut Values, cutoff_state: State) {
    let keys_to_marginalize = find_keys_to_marginalize(values, cutoff_state);
    if keys_to_marginalize.is_empty() {
        return;
    }

    let keys_to_marginalize_set: HashSet<_> = keys_to_marginalize.iter().copied().collect();

    // Remove only the factors connected to a marginalized state
    let removed_factors = optimizer.graph_mut().remove_factors(|factor| {
        factor
            .keys()
            .iter()
            .any(|key| keys_to_marginalize_set.contains(key))
    });

    let boundary_keys = find_markov_blanket(&keys_to_marginalize_set, &removed_factors);

    if boundary_keys.is_empty() {
        values.retain(|key| !keys_to_marginalize_set.contains(key));
        return;
    }

    let mut map = HashMap::default();
    let mut offset = 0usize;

    for key in keys_to_marginalize.iter().copied() {
        let dim = values.get_raw(key).expect("missing key").dim();
        map.insert(key, factrs::containers::Idx { idx: offset, dim });
        offset += dim;
    }

    let marginal_dim = offset;
    for key in boundary_keys.iter().copied() {
        let dim = values.get_raw(key).expect("missing key").dim();
        map.insert(key, factrs::containers::Idx { idx: offset, dim });
        offset += dim;
    }

    log::debug!("marginalizing {marginal_dim} DOFs");

    let mut temporary_graph = Graph::default();
    for factor in removed_factors {
        temporary_graph.add_factor(factor);
    }

    let value_order = ValuesOrder::new(map);
    let linearized_graph = temporary_graph.linearize(values);
    let graph_order = linearized_graph.sparsity_pattern(value_order);
    let linearized_graph = linearized_graph.with_order(&graph_order);

    let residual_jacobian = linearized_graph.residual_jacobian();
    let dense_jacobian = residual_jacobian.diff.to_dense();
    let rank_relative_tolerance =
        rank_relative_tolerance(dense_jacobian.nrows().max(dense_jacobian.ncols()));
    let prior = square_root_marginalize(
        dense_jacobian.as_ref(),
        residual_jacobian.value.as_ref(),
        marginal_dim,
        rank_relative_tolerance,
    );

    values.retain(|key| !keys_to_marginalize_set.contains(key));

    if prior.jacobian.nrows() == 0 {
        return;
    }

    let jacobian_matrix = prior.jacobian.as_ref().into_nalgebra().clone_owned();
    let target_error_vector = prior
        .target
        .as_ref()
        .into_nalgebra()
        .column(0)
        .clone_owned();

    let input = DynVarPack::new(boundary_keys.clone()).expect("boundary keys must be unique");
    let factor = FactorBuilder::new_dyn(
        SchurPriorResidual::new(
            boundary_keys.clone(),
            jacobian_matrix,
            target_error_vector,
            make_linearization_point(values, &boundary_keys),
        ),
        input,
    )
    .build();

    optimizer.graph_mut().add_factor(factor);
}

struct SquareRootPrior {
    jacobian: Mat<f64>,
    target: Mat<f64>,
}

/// Marginalizes the first `marginal_dim` columns of `jacobian` from
/// `0.5 * ||jacobian * delta - target||^2` while preserving square-root form.
fn square_root_marginalize(
    jacobian: MatRef<'_, f64>,
    target: MatRef<'_, f64>,
    marginal_dim: usize,
    rank_relative_tolerance: f64,
) -> SquareRootPrior {
    assert_eq!(jacobian.nrows(), target.nrows());
    assert_eq!(target.ncols(), 1);
    assert!(marginal_dim <= jacobian.ncols());

    let keep_dim = jacobian.ncols() - marginal_dim;
    if keep_dim == 0 || jacobian.nrows() == 0 {
        return empty_prior(keep_dim);
    }

    let j_k = jacobian.get(.., marginal_dim..);
    let mut projected = augment_with_target(j_k, target);
    let marginal_rank = if marginal_dim == 0 {
        0
    } else {
        let j_m = jacobian.get(.., ..marginal_dim);
        if j_m.norm_max() == 0.0 {
            0
        } else {
            let qr = j_m.col_piv_qr();
            let rank = numerical_rank_from_r(qr.thin_R(), rank_relative_tolerance);
            apply_q_transpose_in_place(qr.Q_basis(), qr.Q_coeff(), projected.rb_mut());
            rank
        }
    };

    let raw_prior = projected.as_ref().get(marginal_rank.., ..);
    let raw_jacobian = raw_prior.get(.., ..keep_dim);
    let raw_target = raw_prior.get(.., keep_dim..keep_dim + 1);

    compact_prior(raw_jacobian, raw_target, rank_relative_tolerance)
}

fn compact_prior(
    jacobian: MatRef<'_, f64>,
    target: MatRef<'_, f64>,
    rank_relative_tolerance: f64,
) -> SquareRootPrior {
    let keep_dim = jacobian.ncols();
    if keep_dim == 0 || jacobian.nrows() == 0 {
        return empty_prior(keep_dim);
    }

    if jacobian.norm_max() == 0.0 {
        return empty_prior(keep_dim);
    }

    let qr = jacobian.col_piv_qr();
    let rank = numerical_rank_from_r(qr.thin_R(), rank_relative_tolerance);
    if rank == 0 {
        return empty_prior(keep_dim);
    }

    let mut compact = augment_with_target(jacobian, target);
    apply_q_transpose_in_place(qr.Q_basis(), qr.Q_coeff(), compact.rb_mut());
    split_prior(compact.as_ref().get(..rank, ..), keep_dim)
}

fn apply_q_transpose_in_place(
    q_basis: MatRef<'_, f64>,
    q_coeff: MatRef<'_, f64>,
    rhs: MatMut<'_, f64>,
) {
    let scratch =
        faer::linalg::householder::apply_block_householder_sequence_transpose_on_the_left_in_place_scratch::<f64>(
            q_basis.nrows(),
            q_coeff.nrows(),
            rhs.ncols(),
        );
    let mut buffer = faer::dyn_stack::MemBuffer::new(scratch);
    faer::linalg::householder::apply_block_householder_sequence_transpose_on_the_left_in_place_with_conj(
        q_basis,
        q_coeff,
        Conj::No,
        rhs,
        Par::Seq,
        faer::dyn_stack::MemStack::new(&mut buffer),
    );
}

fn split_prior(prior: MatRef<'_, f64>, keep_dim: usize) -> SquareRootPrior {
    SquareRootPrior {
        jacobian: prior.get(.., ..keep_dim).to_owned(),
        target: prior.get(.., keep_dim..keep_dim + 1).to_owned(),
    }
}

fn augment_with_target(jacobian: MatRef<'_, f64>, target: MatRef<'_, f64>) -> Mat<f64> {
    assert_eq!(jacobian.nrows(), target.nrows());
    assert_eq!(target.ncols(), 1);

    let mut augmented = Mat::zeros(jacobian.nrows(), jacobian.ncols() + 1);
    augmented
        .as_mut()
        .subcols_mut(0, jacobian.ncols())
        .copy_from(jacobian);
    augmented
        .as_mut()
        .col_mut(jacobian.ncols())
        .copy_from(target.col(0));
    augmented
}

fn empty_prior(keep_dim: usize) -> SquareRootPrior {
    SquareRootPrior {
        jacobian: Mat::zeros(0, keep_dim),
        target: Mat::zeros(0, 1),
    }
}

fn numerical_rank_from_r(r: MatRef<'_, f64>, relative_tolerance: f64) -> usize {
    let diagonal_len = r.nrows().min(r.ncols());
    if diagonal_len == 0 {
        return 0;
    }

    let scale = (0..diagonal_len)
        .map(|i| r[(i, i)].abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return 0;
    }

    let threshold = relative_tolerance * scale;
    (0..diagonal_len)
        .take_while(|&i| r[(i, i)].abs() > threshold)
        .count()
}

fn rank_relative_tolerance(max_dimension: usize) -> f64 {
    RANK_RELATIVE_TOLERANCE_SCALE * f64::EPSILON * max_dimension as f64
}

fn find_markov_blanket(
    keys_to_marginalize_set: &HashSet<Key>,
    removed_factors: &[factrs::core::Factor],
) -> Vec<Key> {
    removed_factors
        .iter()
        .flat_map(|factor| {
            factor
                .keys()
                .iter()
                .filter(|key| !keys_to_marginalize_set.contains(key))
        })
        .copied()
        .unique()
        .collect()
}

fn find_keys_to_marginalize(values: &Values, cutoff_key: State) -> Vec<Key> {
    let cutoff_key: Key = cutoff_key.into();

    values
        .iter()
        .filter(|(key, value)| value.is::<SE23>() && key.0 < cutoff_key.0)
        .map(|(key, _)| *key)
        .collect()
}

fn make_linearization_point(values: &Values, keys: &[Key]) -> Vec<Box<dyn VariableSafe>> {
    keys.iter()
        .map(|k| {
            values
                .get_raw(*k)
                .unwrap_or_else(|| panic!("missing key {:?}", k))
                .clone_box()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use faer::mat;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-10,
            "actual {actual} != expected {expected}"
        );
    }

    fn assert_matrix_close(actual: MatRef<'_, f64>, expected: MatRef<'_, f64>) {
        assert_eq!(actual.nrows(), expected.nrows());
        assert_eq!(actual.ncols(), expected.ncols());
        for row in 0..actual.nrows() {
            for col in 0..actual.ncols() {
                assert_close(actual[(row, col)], expected[(row, col)]);
            }
        }
    }

    #[test]
    fn householder_projection_matches_explicit_q_projection() {
        let matrix = mat![
            [1.0, 0.0, 2.0],
            [2.0, 1.0, 0.0],
            [0.0, 3.0, 1.0],
            [1.0, -1.0, 4.0],
        ];
        let mut actual = mat![[1.0, 2.0], [3.0, 5.0], [8.0, 13.0], [21.0, 34.0],];
        let qr = matrix.as_ref().col_piv_qr();
        let expected = qr.compute_Q().transpose() * &actual;

        apply_q_transpose_in_place(qr.Q_basis(), qr.Q_coeff(), actual.rb_mut());

        assert_matrix_close(actual.as_ref(), expected.as_ref());
    }

    #[test]
    fn square_root_marginalization_preserves_factrs_rhs_sign() {
        let jacobian = mat![[1.0, 0.0], [0.0, 2.0]];
        let target = mat![[0.0], [3.0]];

        let prior = square_root_marginalize(
            jacobian.as_ref(),
            target.as_ref(),
            1,
            rank_relative_tolerance(2),
        );

        assert_eq!(prior.jacobian.nrows(), 1);
        assert_eq!(prior.jacobian.ncols(), 1);
        assert_close(prior.jacobian[(0, 0)] * prior.jacobian[(0, 0)], 4.0);
        assert_close(prior.jacobian[(0, 0)] * prior.target[(0, 0)], 6.0);
    }

    #[test]
    fn square_root_marginalization_preserves_kept_terms_with_zero_marginal_block() {
        let jacobian = mat![[0.0, 2.0], [0.0, 5.0]];
        let target = mat![[3.0], [7.0]];

        let prior = square_root_marginalize(
            jacobian.as_ref(),
            target.as_ref(),
            1,
            rank_relative_tolerance(2),
        );

        assert_eq!(prior.jacobian.nrows(), 1);
        assert_eq!(prior.jacobian.ncols(), 1);
        assert_close(prior.jacobian[(0, 0)] * prior.jacobian[(0, 0)], 29.0);
        assert_close(prior.jacobian[(0, 0)] * prior.target[(0, 0)], 41.0);
    }

    #[test]
    fn square_root_marginalization_does_not_anchor_relative_only_factor() {
        let jacobian = mat![[1.0, -1.0]];
        let target = mat![[3.0]];

        let prior = square_root_marginalize(
            jacobian.as_ref(),
            target.as_ref(),
            1,
            rank_relative_tolerance(2),
        );

        assert_eq!(prior.jacobian.nrows(), 0);
        assert_eq!(prior.jacobian.ncols(), 1);
        assert_eq!(prior.target.nrows(), 0);
    }

    #[test]
    fn square_root_marginalization_matches_schur_normal_equations() {
        let jacobian = mat![[1.0, 1.0], [1.0, 0.0]];
        let target = mat![[2.0], [0.0]];

        let prior = square_root_marginalize(
            jacobian.as_ref(),
            target.as_ref(),
            1,
            rank_relative_tolerance(2),
        );

        assert_eq!(prior.jacobian.nrows(), 1);
        assert_eq!(prior.jacobian.ncols(), 1);
        assert_close(prior.jacobian[(0, 0)] * prior.jacobian[(0, 0)], 0.5);
        assert_close(prior.jacobian[(0, 0)] * prior.target[(0, 0)], 1.0);
    }
}
