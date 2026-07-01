use super::*;

impl Problem {
    pub(super) fn new(
        input: GlobalLocalizationInput<'_>,
        cfg: GlobalAssociationConfig,
    ) -> Option<Self> {
        if !valid_intrinsic(input.camera_intrinsic) {
            return None;
        }
        let field_boundary = field_boundary_limits(input.field_dimensions)?;

        let map = cached_landmark_map(input.field_dimensions, cfg.min_map_baseline);
        let ground_to_camera = input.robot_to_camera * input.ground_to_robot;
        let camera_to_ground = ground_to_camera.inverse();
        let camera_origin = camera_to_ground.inner.translation.vector;
        if !camera_origin.iter().all(|value| value.is_finite())
            || camera_origin.z.abs() <= HORIZON_EPSILON
        {
            return None;
        }

        let detection_set = detection_points(
            input.visual_features,
            &map,
            &camera_to_ground,
            input.camera_intrinsic,
            cfg,
        );
        let detections = detection_set.detections;
        if detections.len() < cfg.min_inliers.max(3) {
            return None;
        }
        let high_confidence_unmatched_penalty = detections
            .iter()
            .filter(|detection| detection.confidence >= UNMATCHED_HIGH_CONFIDENCE)
            .map(|detection| UNMATCHED_EVIDENCE_PENALTY * detection_priority(detection, &map))
            .sum();

        Some(Self {
            cfg,
            k: input.camera_intrinsic,
            ground_to_robot: input.ground_to_robot,
            robot_to_camera: input.robot_to_camera,
            pose_hint: input.pose_hint,
            map,
            detections,
            detections_truncated: detection_set.truncated,
            camera_xy_ground: Vector2::new(camera_origin.x, camera_origin.y),
            field_boundary,
            high_confidence_unmatched_penalty,
        })
    }
}

type LandmarkMapCacheEntry = (LandmarkMapCacheKey, Arc<LandmarkMap>);
type LandmarkMapCache = Mutex<Vec<LandmarkMapCacheEntry>>;

pub(super) fn field_boundary_limits(field_dimensions: &FieldDimensions) -> Option<Vector2<f32>> {
    let x_limit = field_dimensions.length * 0.5 + field_dimensions.border_strip_width;
    let y_limit = field_dimensions.width * 0.5 + field_dimensions.border_strip_width;
    (x_limit.is_finite() && x_limit > 0.0 && y_limit.is_finite() && y_limit > 0.0)
        .then_some(Vector2::new(x_limit, y_limit))
}

pub(super) fn cached_landmark_map(
    field_dimensions: &FieldDimensions,
    min_map_baseline: f32,
) -> Arc<LandmarkMap> {
    static CACHE: OnceLock<LandmarkMapCache> = OnceLock::new();
    let key = landmark_map_cache_key(field_dimensions, min_map_baseline);
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::new()));
    {
        let cache = match cache.lock() {
            Ok(cache) => cache,
            Err(poisoned) => poisoned.into_inner(),
        };
        if let Some((_, map)) = cache.iter().find(|(cached_key, _)| *cached_key == key) {
            return Arc::clone(map);
        }
    }

    let map = Arc::new(LandmarkMap::new(field_dimensions, min_map_baseline));
    let mut cache = match cache.lock() {
        Ok(cache) => cache,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some((_, map)) = cache.iter().find(|(cached_key, _)| *cached_key == key) {
        return Arc::clone(map);
    }
    if cache.len() >= MAX_LANDMARK_MAP_CACHE_ENTRIES {
        cache.remove(0);
    }
    cache.push((key, Arc::clone(&map)));
    map
}

fn landmark_map_cache_key(field: &FieldDimensions, min_map_baseline: f32) -> LandmarkMapCacheKey {
    LandmarkMapCacheKey {
        values: [
            min_map_baseline.to_bits(),
            field.ball_radius.to_bits(),
            field.length.to_bits(),
            field.width.to_bits(),
            field.line_width.to_bits(),
            field.penalty_marker_size.to_bits(),
            field.goal_box_area_length.to_bits(),
            field.goal_box_area_width.to_bits(),
            field.penalty_area_length.to_bits(),
            field.penalty_area_width.to_bits(),
            field.penalty_marker_distance.to_bits(),
            field.center_circle_diameter.to_bits(),
            field.border_strip_width.to_bits(),
            field.goal_inner_width.to_bits(),
            field.goal_post_diameter.to_bits(),
            field.goal_depth.to_bits(),
            field.corner_arc_radius.to_bits(),
        ],
    }
}

pub(super) fn valid_intrinsic(intrinsic: Intrinsic) -> bool {
    intrinsic.focals.x.is_finite()
        && intrinsic.focals.y.is_finite()
        && intrinsic.optical_center.x().is_finite()
        && intrinsic.optical_center.y().is_finite()
        && intrinsic.focals.x.abs() > 1.0e-6
        && intrinsic.focals.y.abs() > 1.0e-6
}

pub(super) fn detection_points(
    features: &DetectedVisualFeatures,
    map: &LandmarkMap,
    camera_to_ground: &Isometry3<Camera, Ground>,
    intrinsic: Intrinsic,
    cfg: GlobalAssociationConfig,
) -> DetectionSet {
    let camera_origin = camera_to_ground.inner.translation.vector;
    let camera_height = camera_origin.z.abs();
    let mut detections = Vec::new();

    for (id, (class, feature)) in raw_detections(features).enumerate() {
        if !map.has_class(class) || !usable_confidence(feature.confidence, cfg.min_confidence) {
            continue;
        }

        let bearing_camera = intrinsic.bearing(feature.pixel).inner;
        let direction_ground = camera_to_ground.inner.rotation * bearing_camera;
        if !direction_ground.iter().all(|value| value.is_finite())
            || direction_ground.z.abs() <= HORIZON_EPSILON
            || camera_origin.z * direction_ground.z >= 0.0
        {
            continue;
        }

        let a = Vector2::new(
            direction_ground.x / direction_ground.z.abs(),
            direction_ground.y / direction_ground.z.abs(),
        );
        if !a.iter().all(|value| value.is_finite()) {
            continue;
        }

        let ground_xy = Vector2::new(camera_origin.x, camera_origin.y) + camera_height * a;
        detections.push(DetectionPoint {
            id,
            class,
            pixel: feature.pixel,
            confidence: feature.confidence,
            a,
            ground: point![<Ground>, ground_xy.x, ground_xy.y],
        });
    }

    let truncated = retain_best_detections(&mut detections, map);
    DetectionSet {
        detections,
        truncated,
    }
}

fn raw_detections(
    features: &DetectedVisualFeatures,
) -> impl Iterator<Item = (VisualFeatureClass, DetectedVisualFeature)> + '_ {
    [
        (VisualFeatureClass::GoalPost, features.goalposts.as_slice()),
        (VisualFeatureClass::LSpot, features.l_spots.as_slice()),
        (VisualFeatureClass::TSpot, features.t_spots.as_slice()),
        (VisualFeatureClass::XSpot, features.x_spots.as_slice()),
        (
            VisualFeatureClass::PenaltySpot,
            features.penalty_spots.as_slice(),
        ),
    ]
    .into_iter()
    .flat_map(|(class, detections)| detections.iter().copied().map(move |pixel| (class, pixel)))
}

fn retain_best_detections(detections: &mut Vec<DetectionPoint>, map: &LandmarkMap) -> bool {
    if detections.len() <= MAX_DETECTIONS {
        return false;
    }

    detections.sort_by(|left, right| {
        detection_priority(right, map)
            .total_cmp(&detection_priority(left, map))
            .then_with(|| left.id.cmp(&right.id))
    });
    detections.truncate(MAX_DETECTIONS);
    true
}

pub(super) fn detection_priority(detection: &DetectionPoint, map: &LandmarkMap) -> f32 {
    detection.confidence * map.rarity_weight(detection.class)
}

fn usable_confidence(confidence: f32, min_confidence: f32) -> bool {
    confidence.is_finite() && confidence >= min_confidence && confidence <= 1.0
}
