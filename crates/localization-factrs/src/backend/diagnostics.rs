#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendOptimizerStatus {
    Converged,
    MaxIterations,
    FailedToStep,
    InvalidSystem,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ResidualDiagnostics {
    pub factor_count: usize,
    pub residual_dim: usize,
    pub mean_rms: f64,
    pub max_rms: f64,
}

#[derive(Debug, Clone)]
pub struct BackendSolveDiagnostics {
    pub optimizer_status: BackendOptimizerStatus,
    pub value_count: usize,
    pub factor_count: usize,
    pub total_error: f64,
    pub visual_odometry: ResidualDiagnostics,
    pub visual_reprojection: ResidualDiagnostics,
    pub gaussian_process_prior: ResidualDiagnostics,
}

#[derive(Debug, Default)]
pub(super) struct ResidualDiagnosticsAccumulator {
    factor_count: usize,
    residual_dim: usize,
    sum_squared_norm: f64,
    max_rms: f64,
}

impl ResidualDiagnosticsAccumulator {
    pub(super) fn add(&mut self, factor_error: f64, residual_dim: usize) {
        if residual_dim == 0 {
            return;
        }
        let squared_norm = 2.0 * factor_error;
        let rms = (squared_norm / residual_dim as f64).sqrt();
        self.factor_count += 1;
        self.residual_dim += residual_dim;
        self.sum_squared_norm += squared_norm;
        self.max_rms = self.max_rms.max(rms);
    }

    pub(super) fn extend(&mut self, other: Self) {
        self.factor_count += other.factor_count;
        self.residual_dim += other.residual_dim;
        self.sum_squared_norm += other.sum_squared_norm;
        self.max_rms = self.max_rms.max(other.max_rms);
    }

    pub(super) fn finish(self) -> ResidualDiagnostics {
        ResidualDiagnostics {
            factor_count: self.factor_count,
            residual_dim: self.residual_dim,
            mean_rms: if self.residual_dim == 0 {
                0.0
            } else {
                (self.sum_squared_norm / self.residual_dim as f64).sqrt()
            },
            max_rms: self.max_rms,
        }
    }
}
