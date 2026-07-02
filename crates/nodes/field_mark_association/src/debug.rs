use crate::global_association::GlobalLocalizationResult;
use types::visual_localization::{GlobalLocalizationDebug, GlobalLocalizationDebugStatus};

pub(crate) fn global_localization_debug_from_result(
    result: &GlobalLocalizationResult,
) -> GlobalLocalizationDebug {
    let associations = result.associations();
    GlobalLocalizationDebug {
        robot_to_field: associations.robot_to_field,
        status: GlobalLocalizationDebugStatus::UniqueModuloSymmetry,
        inliers: associations.score.inliers,
        candidate_score: associations.score.candidate_score,
        metric_rms_residual: associations.score.metric_rms_residual,
        reprojection_rmse: associations.score.reprojection_rmse,
        total_cost: associations.score.total_cost,
    }
}
