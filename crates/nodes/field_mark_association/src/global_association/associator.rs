use super::{
    FeatureAssociation, GlobalAssociationConfig, GlobalLocalizationDetailedDebug,
    PoseHintAssociationConfig, solver,
};

pub(crate) use solver::{GlobalLocalizationInput, GlobalLocalizationResult};

#[derive(Clone, Debug)]
pub(crate) struct GlobalAssociator {
    config: GlobalAssociationConfig,
}

impl GlobalAssociator {
    /// Creates a global localizer with fixed association and scoring parameters.
    pub fn new(config: GlobalAssociationConfig) -> Self {
        Self { config }
    }

    /// Attempts to localize one frame of field-feature detections against the known field map.
    ///
    /// Returns `None` when no safe stable association set passes the configured gates.
    pub fn localize(&self, input: GlobalLocalizationInput<'_>) -> Option<GlobalLocalizationResult> {
        solver::solve(input, self.config)
    }

    pub(crate) fn localize_detailed(
        &self,
        input: GlobalLocalizationInput<'_>,
    ) -> Option<GlobalLocalizationDetailedDebug> {
        solver::solve_detailed(input, self.config)
    }

    pub(crate) fn associate_with_pose_hint(
        &self,
        input: GlobalLocalizationInput<'_>,
        config: PoseHintAssociationConfig,
    ) -> PoseHintAssociationResult {
        solver::associate_with_pose_hint(input, self.config, config)
    }
}

impl Default for GlobalAssociator {
    fn default() -> Self {
        Self::new(GlobalAssociationConfig::default())
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PoseHintAssociationResult {
    pub associations: Vec<FeatureAssociation>,
    pub reprojection_rmse: Option<f32>,
}

impl PoseHintAssociationResult {
    pub(crate) fn is_healthy(&self, config: PoseHintAssociationConfig) -> bool {
        self.associations.len() >= config.healthy_min_inliers
            && self
                .reprojection_rmse
                .is_some_and(|rmse| rmse <= config.healthy_max_rmse_px)
    }
}
