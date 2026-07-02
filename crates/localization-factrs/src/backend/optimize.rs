use factrs::{
    optimizers::{OptError, OptStatus},
    residuals::ErasedResidual,
    traits::Optimizer,
};

use crate::{
    factors::{
        gaussian_process_prior::GaussianProcessPriorFactor,
        visual_odometry::{AdjacentVisualOdometryFactor, VisualOdometryFactor},
        visual_reprojection::VisualReprojectionFactor,
    },
    schur_marginalization::marginalize,
    splines::SE23Spline,
    symbols::{CameraIntrinsics, State},
    tau,
};

use super::{
    BackendOptimizerStatus, BackendSolveDiagnostics, OptimizationResult,
    ResidualDiagnosticsAccumulator, VinsBackend, VinsBackendError,
};

impl VinsBackend {
    pub(super) fn optimize_and_publish(
        &mut self,
    ) -> Result<Option<OptimizationResult>, VinsBackendError> {
        let result = self.optimize();
        if self.result_sender.send(result.clone()).is_err() {
            return Err(VinsBackendError::FrontendDisconnected);
        }

        Ok(result)
    }

    pub(super) fn optimize(&mut self) -> Option<OptimizationResult> {
        self.last_optimizer_status = None;
        let time = self.last_knot_time?;
        log::info!("solving graph with {} values", self.values.len());

        self.remove_current_spline_orientation_factor();
        self.add_current_spline_orientation_factor();

        let optimizer_status = match self.optimizer.optimize(&mut self.values) {
            Ok(OptStatus::Converged) => BackendOptimizerStatus::Converged,
            Ok(OptStatus::MaxIterations) => {
                log::warn!("optimizer failed to converge: max iterations reached");
                BackendOptimizerStatus::MaxIterations
            }
            Err(OptError::FailedToStep) => {
                log::warn!("optimizer failed: failed to step");
                BackendOptimizerStatus::FailedToStep
            }
            Err(OptError::InvalidSystem) => {
                log::warn!("optimizer failed: invalid system");
                BackendOptimizerStatus::InvalidSystem
            }
        };
        self.remove_current_spline_orientation_factor();

        if matches!(
            optimizer_status,
            BackendOptimizerStatus::Converged | BackendOptimizerStatus::MaxIterations
        ) {
            let cutoff_time = time - self.config.max_optimization_window;
            if let Some(smallest_interval_index_in_window) = self
                .interval_assigner
                .assign_or_initialize_interval(cutoff_time)
            {
                marginalize(
                    &mut self.optimizer,
                    &mut self.values,
                    State(smallest_interval_index_in_window),
                );
            }
        }

        self.last_optimizer_status = Some(optimizer_status);

        let interval_start_time = self
            .interval_assigner
            .current_or_initialize_interval_start_time(time)?;
        let interval_start_index = self
            .interval_assigner
            .assign_or_initialize_interval(interval_start_time)?;
        let start = self.values.get(State(interval_start_index))?.clone();
        let end = self.values.get(State(interval_start_index + 1))?.clone();
        let interval_end_time = interval_start_time + self.config.knot_spacing;
        let latest_pose = SE23Spline::new(start, end, self.config.knot_spacing.as_secs_f64())
            .evaluate(tau(interval_start_time, interval_end_time, time));
        let camera_intrinsics = self.values.get(CameraIntrinsics(0))?.clone();

        Some(OptimizationResult {
            time,
            latest_pose,
            camera_intrinsics,
        })
    }

    pub(super) fn solve_diagnostics(
        &self,
        optimizer_status: BackendOptimizerStatus,
    ) -> BackendSolveDiagnostics {
        let graph = self.optimizer.graph();
        let mut visual_odometry = self.residual_diagnostics::<VisualOdometryFactor>();
        visual_odometry.extend(self.residual_diagnostics::<AdjacentVisualOdometryFactor>());
        let visual_reprojection = self.residual_diagnostics::<VisualReprojectionFactor>();

        BackendSolveDiagnostics {
            optimizer_status,
            value_count: self.values.len(),
            factor_count: graph.len(),
            total_error: graph.error(&self.values),
            visual_odometry: visual_odometry.finish(),
            visual_reprojection: visual_reprojection.finish(),
            gaussian_process_prior: self
                .residual_diagnostics::<GaussianProcessPriorFactor>()
                .finish(),
        }
    }

    fn residual_diagnostics<R>(&self) -> ResidualDiagnosticsAccumulator
    where
        R: ErasedResidual + 'static,
    {
        let graph = self.optimizer.graph();
        let mut diagnostics = ResidualDiagnosticsAccumulator::default();
        for index in 0..graph.len() {
            let factor = graph.at(index);
            if !factor.is_residual::<R>() {
                continue;
            }
            let Ok(error) = factor.try_error(&self.values) else {
                continue;
            };
            let Ok(dim) = factor.try_dim_out(&self.values) else {
                continue;
            };
            diagnostics.add(error, dim);
        }
        diagnostics
    }
}
