use super::*;

pub(super) const HORIZON_EPSILON: f32 = 1.0e-4;
pub(super) const MAX_DETECTIONS: usize = GLOBAL_LOCALIZER_MAX_DETECTIONS;
pub(super) const MAX_TRIPLET_BIN_PROBES: usize = 64_000;
pub(super) const MAX_TRIPLET_LOOKUP_HITS: usize = 2500;
pub(super) const MAX_CHEAP_SEEDS: usize = 512;
pub(super) const TRUST_REGION_TRANSLATION: f32 = 1.5;
pub(super) const TRUST_REGION_YAW: f32 = 0.45;
pub(super) const TRUST_REGION_HEIGHT: f32 = 0.3;
pub(super) const TRANSFORM_CELL_TRANSLATION: f32 = 0.05;
pub(super) const TRANSFORM_CELL_YAW: f32 = 0.02;
pub(super) const TRANSFORM_CELL_LOG_HEIGHT: f32 = 0.02;
pub(super) const TRIPLET_BIN_WINDOW_WIDTH: usize = TRIPLET_BIN_RADIUS as usize * 2 + 1;
pub(super) const TRIPLET_BIN_PROBES_PER_DETECTION_TRIPLET: usize =
    TRIPLET_BIN_WINDOW_WIDTH * TRIPLET_BIN_WINDOW_WIDTH;
pub(super) const MAX_SCANNED_DETECTION_TRIPLETS: usize =
    MAX_TRIPLET_BIN_PROBES.div_ceil(TRIPLET_BIN_PROBES_PER_DETECTION_TRIPLET);
pub(super) const TRIPLET_BIN_RADIUS: i16 = 2;
pub(super) const MIN_DETECTION_TRIPLET_ABS_BETA: f32 = 1.0e-4;
pub(super) const TRIPLET_DESCRIPTOR_TOLERANCE: f32 = 0.36;
pub(super) const CHEAP_ASSOCIATION_GATE_FACTOR: f32 = 1.25;
pub(super) const INITIAL_ASSIGNMENT_GATE_FACTOR: f32 = 1.5;
pub(super) const UNMATCHED_HIGH_CONFIDENCE: f32 = 0.5;
pub(super) const UNMATCHED_EVIDENCE_PENALTY: f32 = 0.5;
pub(super) const MAX_LANDMARK_MAP_CACHE_ENTRIES: usize = 4;
pub(super) const MAX_REFINED_CANDIDATES: usize = 64;

#[derive(Clone, Debug)]
/// Inputs needed to solve one global field-feature association frame.
pub(crate) struct GlobalLocalizationInput<'a> {
    /// Field-feature detections extracted from the current object-detection frame.
    pub visual_features: &'a DetectedVisualFeatures,
    /// Field geometry used to generate the fixed landmark map.
    pub field_dimensions: &'a FieldDimensions,
    /// Current ground-to-robot transform from the camera matrix.
    pub ground_to_robot: Isometry3<Ground, Robot>,
    /// Current robot-to-camera transform from the camera matrix.
    pub robot_to_camera: Isometry3<Robot, Camera>,
    /// Camera intrinsics used for projection and back-projection.
    pub camera_intrinsic: Intrinsic,
    /// Optional backend pose used only to choose the final 180-degree symmetry branch.
    pub pose_hint: Option<Isometry3<Robot, Field>>,
}

#[derive(Clone, Debug)]
/// Association result for one global localization frame.
pub(crate) enum GlobalLocalizationResult {
    /// Reserved for plausible associations whose uniqueness was not certified.
    #[allow(dead_code)]
    Ambiguous(FeatureAssociations),
    /// The returned stable association set is unique after quotienting the unavoidable 180 degree
    /// field symmetry.
    UniqueModuloSymmetry(FeatureAssociations),
}

pub(super) struct Problem {
    pub(super) cfg: GlobalAssociationConfig,
    pub(super) k: Intrinsic,
    pub(super) ground_to_robot: Isometry3<Ground, Robot>,
    pub(super) robot_to_camera: Isometry3<Robot, Camera>,
    pub(super) pose_hint: Option<Isometry3<Robot, Field>>,
    pub(super) map: Arc<LandmarkMap>,
    pub(super) detections: Vec<DetectionPoint>,
    pub(super) detections_truncated: bool,
    pub(super) camera_xy_ground: Vector2<f32>,
    pub(super) field_boundary: Vector2<f32>,
    pub(super) high_confidence_unmatched_penalty: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct LandmarkMapCacheKey {
    pub(super) values: [u32; 17],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DetectionPoint {
    pub(super) id: usize,
    pub(super) class: VisualFeatureClass,
    pub(super) pixel: Point2<Pixel>,
    pub(super) confidence: f32,
    pub(super) a: Vector2<f32>,
    pub(super) ground: Point2<Ground>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct DetectionTriplet {
    pub(super) a: usize,
    pub(super) b: usize,
    pub(super) c: usize,
    pub(super) distance: f32,
    pub(super) angle: f32,
    pub(super) alpha: f32,
    pub(super) beta: f32,
    pub(super) priority: f32,
}

pub(super) struct DetectionTriplets {
    pub(super) triplets: Vec<DetectionTriplet>,
    pub(super) truncated: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Candidate {
    pub(super) score: f32,
    pub(super) matches: Vec<Match>,
    pub(super) metric_rms_residual: f32,
    pub(super) transform: Similarity2<f32>,
}

impl Candidate {
    pub(super) fn metrics(&self) -> CandidateMetrics {
        CandidateMetrics {
            score: self.score,
            inlier_count: self.matches.len(),
            metric_rms_residual: self.metric_rms_residual,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CandidateMetrics {
    pub(super) score: f32,
    pub(super) inlier_count: usize,
    pub(super) metric_rms_residual: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Match {
    pub(super) detection_index: usize,
    pub(super) landmark_id: usize,
    pub(super) class: VisualFeatureClass,
    pub(super) confidence: f32,
    pub(super) residual: f32,
}

impl Match {
    pub(super) fn new(
        problem: &Problem,
        detection_index: usize,
        landmark_id: usize,
        residual: f32,
    ) -> Option<Self> {
        if !residual.is_finite() {
            return None;
        }
        let detection = problem.detections.get(detection_index)?;
        let landmark = problem.map.landmarks.get(landmark_id)?;
        if detection.class != landmark.class {
            return None;
        }
        Some(Self {
            detection_index,
            landmark_id,
            class: detection.class,
            confidence: detection.confidence,
            residual,
        })
    }
}

pub(super) struct DetectionSet {
    pub(super) detections: Vec<DetectionPoint>,
    pub(super) truncated: bool,
}

pub(super) struct CandidateSearch {
    pub(super) candidates: Vec<Candidate>,
    pub(super) truncated: bool,
}

#[derive(Clone, Debug)]
pub(super) struct CheapCandidate {
    pub(super) score: f32,
    pub(super) transform: Similarity2<f32>,
    pub(super) inlier_count: usize,
    pub(super) metric_rms_residual: f32,
    pub(super) key: AssociationKey,
}

impl CheapCandidate {
    pub(super) fn metrics(&self) -> CandidateMetrics {
        CandidateMetrics {
            score: self.score,
            inlier_count: self.inlier_count,
            metric_rms_residual: self.metric_rms_residual,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct AssociationKey(Vec<(usize, usize)>);

impl AssociationKey {
    pub(super) fn empty() -> Self {
        Self(Vec::new())
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn push(&mut self, detection_index: usize, landmark_id: usize) {
        self.0.push((detection_index, landmark_id));
    }

    pub(super) fn sort(&mut self) {
        self.0.sort_unstable();
    }

    pub(super) fn symmetric(mut self, map: &LandmarkMap) -> Self {
        for (_, landmark_id) in &mut self.0 {
            *landmark_id = map.symmetric_id(*landmark_id);
        }
        self.sort();
        self
    }
}

pub(super) struct CheapSeedSearch {
    pub(super) candidates: Vec<CheapCandidate>,
    pub(super) truncated: bool,
}

#[derive(Clone, Debug)]
pub(super) struct SeedState {
    pub(super) seed: CheapCandidate,
    pub(super) upper_bound: f32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct TransformCellKey {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) yaw: i32,
    pub(super) log_height: i32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TransformCell {
    pub(super) transform: Similarity2<f32>,
    pub(super) priority: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct AssignmentOption {
    pub(super) accepted: Match,
    pub(super) column: usize,
    pub(super) value: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SymmetryRelation {
    Exact,
    Symmetric,
    Other,
    Missing,
}
