use localization_factrs::backend::{
    BackendOptimizerStatus, BackendSolveDiagnostics,
    ResidualDiagnostics as BackendResidualDiagnostics,
};
use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct SolveDiagnostics {
    pub optimizer_status: SolveOptimizerStatus,
    pub value_count: usize,
    pub factor_count: usize,
    pub total_error: f64,
    pub visual_odometry: SolveResidualDiagnostics,
    pub visual_reprojection: SolveResidualDiagnostics,
    pub gaussian_process_prior: SolveResidualDiagnostics,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Message)]
pub enum SolveOptimizerStatus {
    Converged,
    MaxIterations,
    FailedToStep,
    InvalidSystem,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Message)]
pub struct SolveResidualDiagnostics {
    pub factor_count: usize,
    pub residual_dim: usize,
    pub mean_rms: f64,
    pub max_rms: f64,
}

impl From<BackendSolveDiagnostics> for SolveDiagnostics {
    fn from(diagnostics: BackendSolveDiagnostics) -> Self {
        Self {
            optimizer_status: diagnostics.optimizer_status.into(),
            value_count: diagnostics.value_count,
            factor_count: diagnostics.factor_count,
            total_error: diagnostics.total_error,
            visual_odometry: diagnostics.visual_odometry.into(),
            visual_reprojection: diagnostics.visual_reprojection.into(),
            gaussian_process_prior: diagnostics.gaussian_process_prior.into(),
        }
    }
}

impl From<BackendOptimizerStatus> for SolveOptimizerStatus {
    fn from(status: BackendOptimizerStatus) -> Self {
        match status {
            BackendOptimizerStatus::Converged => Self::Converged,
            BackendOptimizerStatus::MaxIterations => Self::MaxIterations,
            BackendOptimizerStatus::FailedToStep => Self::FailedToStep,
            BackendOptimizerStatus::InvalidSystem => Self::InvalidSystem,
        }
    }
}

impl From<BackendResidualDiagnostics> for SolveResidualDiagnostics {
    fn from(diagnostics: BackendResidualDiagnostics) -> Self {
        Self {
            factor_count: diagnostics.factor_count,
            residual_dim: diagnostics.residual_dim,
            mean_rms: diagnostics.mean_rms,
            max_rms: diagnostics.max_rms,
        }
    }
}
