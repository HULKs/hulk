use coordinate_systems::{Field, Ground, Pixel};
use linear_algebra::{Orientation2, Point2};
use localization_fagra::alignment::fit_ground_similarity;
use nalgebra::{Matrix2, Matrix2x3, Vector2, Vector3};
use types::visual_localization::{
    FieldMarkAssociation, GlobalLocalizationDebug, VisualAssociationSource,
};

use super::GlobalAssociationConfig;
use crate::{
    AssociationResult, DetectedVisualFeature, GlobalAssociationInput, VisualFeatureClass,
    features::raw_detections, map::LandmarkMap,
};

#[derive(Clone, Debug)]
pub(crate) struct Detection {
    pub id: usize,
    pub class: VisualFeatureClass,
    pub pixel: Point2<Pixel>,
    pub confidence: f32,
    /// Ground intersection at unit camera height, relative to the camera XY.
    pub xy: Vector2<f32>,
    pixel_covariance: Matrix2<f32>,
    tilt_jacobian: Matrix2<f32>,
}

impl Detection {
    pub(crate) fn covariance(&self, config: GlobalAssociationConfig) -> Matrix2<f32> {
        self.pixel_covariance
            + config.imu_tilt_sigma.powi(2) * self.tilt_jacobian * self.tilt_jacobian.transpose()
    }
}

/// Only orientation is consumed. Neither local translation nor a ground-height guess
/// participates in startup matching; the landmark fit determines camera height.
pub(crate) fn preprocess(
    input: GlobalAssociationInput<'_>,
) -> Option<(LandmarkMap, Vec<Detection>)> {
    let config = *input.parameters;
    config.validate().ok()?;
    if input.visual_features.supported_feature_count() > config.max_input_detections
        || !input.camera_intrinsic.is_valid()
        || !input
            .robot_to_camera
            .inner
            .to_homogeneous()
            .iter()
            .all(|x| x.is_finite())
        || !input
            .robot_to_ground
            .inner
            .coords
            .iter()
            .all(|x| x.is_finite())
    {
        return None;
    }
    let map = LandmarkMap::new(input.field_dimensions, config.symmetry_epsilon);
    if map
        .landmarks
        .iter()
        .any(|p| !p.xy.coords().inner.iter().all(|x| x.is_finite()))
    {
        return None;
    }
    let mut detections = raw_detections(input.visual_features)
        .enumerate()
        .filter(|(_, (_, feature))| {
            (config.confidence_threshold..=1.0).contains(&feature.confidence)
        })
        .filter_map(|(id, (class, feature))| project_detection(input, id, class, feature))
        .collect::<Vec<_>>();
    detections.sort_by(|a, b| {
        (b.confidence * map.rarity_weight(b.class))
            .total_cmp(&(a.confidence * map.rarity_weight(a.class)))
            .then_with(|| a.id.cmp(&b.id))
    });
    let mut retained = Vec::<Detection>::new();
    for detection in detections {
        if retained.iter().any(|other| {
            other.class == detection.class
                && (other.pixel - detection.pixel).inner.norm() <= config.duplicate_pixel_distance
        }) {
            continue;
        }
        if retained.len() == config.max_retained_detections {
            return None;
        }
        retained.push(detection);
    }
    Some((map, retained))
}

fn project_detection(
    input: GlobalAssociationInput<'_>,
    id: usize,
    class: VisualFeatureClass,
    feature: DetectedVisualFeature,
) -> Option<Detection> {
    let config = *input.parameters;
    let rotation = (input.robot_to_ground * input.robot_to_camera.rotation().inverse()).inner;
    let ray = rotation * input.camera_intrinsic.bearing(feature.pixel).inner;
    let pixel_rays = [
        (rotation * Vector3::x()) / input.camera_intrinsic.focals.x,
        (rotation * Vector3::y()) / input.camera_intrinsic.focals.y,
    ];
    let ray_z_sigma = config.detection_pixel_sigma * pixel_rays[0].z.hypot(pixel_rays[1].z)
        + config.imu_tilt_sigma * ray.xy().norm();
    if !ray.iter().all(|x| x.is_finite())
        || -ray.z
            <= (config.mahalanobis_gate.sqrt() * ray_z_sigma)
                .max(config.min_downward_ray_fraction * ray.norm())
    {
        return None;
    }
    let ray_jacobian = Matrix2x3::new(
        -1.0 / ray.z,
        0.0,
        ray.x / ray.z.powi(2),
        0.0,
        -1.0 / ray.z,
        ray.y / ray.z.powi(2),
    );
    let pixel_jacobian = Matrix2::from_columns(&pixel_rays.map(|r| ray_jacobian * r));
    let detection = Detection {
        id,
        class,
        pixel: feature.pixel,
        confidence: feature.confidence,
        xy: -ray.xy() / ray.z,
        pixel_covariance: config.detection_pixel_sigma.powi(2)
            * pixel_jacobian
            * pixel_jacobian.transpose()
            + Matrix2::identity() * config.projected_covariance_floor,
        tilt_jacobian: Matrix2::from_columns(
            &[Vector3::x(), Vector3::y()].map(|axis| ray_jacobian * axis.cross(&ray)),
        ),
    };
    detection
        .xy
        .iter()
        .chain(detection.covariance(config).iter())
        .all(|x| x.is_finite())
        .then_some(detection)
}

fn invariant_sigma(terms: &[(&Detection, Vector2<f32>)], config: GlobalAssociationConfig) -> f32 {
    let mut pixel_variance = 0.0;
    let mut tilt = Vector2::zeros();
    for (detection, gradient) in terms {
        pixel_variance += gradient.dot(&(detection.pixel_covariance * gradient));
        tilt += detection.tilt_jacobian.transpose() * gradient;
    }
    pixel_variance.max(0.0).sqrt() + config.imu_tilt_sigma * tilt.norm()
}

fn cross(a: Vector2<f32>, b: Vector2<f32>) -> f32 {
    a.x * b.y - a.y * b.x
}

fn select_seed(detections: &[Detection], config: GlobalAssociationConfig) -> Option<[usize; 3]> {
    let mut seed = None;
    let mut best: f32 = 0.0;
    // A bounded seed pool changes only search order. Every retained detection must match.
    for a in 0..detections.len().min(config.seed_pool_size) {
        for b in a + 1..detections.len().min(config.seed_pool_size) {
            for c in b + 1..detections.len().min(config.seed_pool_size) {
                let u = detections[b].xy - detections[a].xy;
                let v = detections[c].xy - detections[a].xy;
                let quality = cross(u, v).abs()
                    / (u.norm_squared() + v.norm_squared()).max(config.min_triangle_denominator);
                if quality > best.max(config.min_seed_quality) {
                    best = quality;
                    seed = Some([a, b, c]);
                }
            }
        }
    }
    seed
}

struct Search<'a> {
    input: GlobalAssociationInput<'a>,
    map: &'a LandmarkMap,
    detections: &'a [Detection],
    order: Vec<usize>,
    remaining_work: usize,
    solution: Option<(Vec<(usize, usize)>, f32)>,
}

impl Search<'_> {
    fn spend(&mut self) -> Option<()> {
        self.remaining_work = self.remaining_work.checked_sub(1)?;
        Some(())
    }

    /// Propagate a common positive scale interval through every pair. This is a
    /// scale-independent necessary condition, not a certificate; full fits use pixels.
    fn compatible(
        &mut self,
        pairs: &[(usize, usize)],
        mut scale: (f32, f32),
    ) -> Option<Option<(f32, f32)>> {
        let (&(d, m), previous) = pairs.split_last()?;
        let config = *self.input.parameters;
        for &(e, n) in previous {
            self.spend()?;
            if m == n {
                return Some(None);
            }
            let a = &self.detections[d];
            let b = &self.detections[e];
            let edge = a.xy - b.xy;
            let distance = edge.norm();
            if distance < config.min_pair_distance {
                return Some(None);
            }
            let direction = edge / distance;
            let noise = config.mahalanobis_gate.sqrt()
                * invariant_sigma(&[(a, direction), (b, -direction)], config);
            let metric = (self.map.landmarks[m].xy - self.map.landmarks[n].xy)
                .inner
                .norm();
            scale.0 = scale
                .0
                .max((metric - config.geometric_tolerance) / (distance + noise));
            if distance > noise {
                scale.1 = scale
                    .1
                    .min((metric + config.geometric_tolerance) / (distance - noise));
            }
            if scale.0 > scale.1 || scale.1 <= 0.0 {
                return Some(None);
            }
        }
        Some(Some(scale))
    }

    fn visit(&mut self, pairs: &mut Vec<(usize, usize)>, scale: (f32, f32)) -> Option<()> {
        self.spend()?;
        if pairs.len() == self.order.len() {
            if let Some(rms) = self.validate_fit(pairs) {
                let key = if self.input.heading.is_some() {
                    let mut key = pairs.clone();
                    key.sort_by_key(|&(d, _)| self.detections[d].id);
                    key
                } else {
                    canonical_pairs(pairs, self.detections, self.map)
                };
                if self
                    .solution
                    .as_ref()
                    .is_some_and(|(prior, _)| *prior != key)
                {
                    return None;
                }
                self.solution = Some((key, rms));
            }
            return Some(());
        }
        let d = self.order[pairs.len()];
        for &m in self.map.landmarks_for_class(self.detections[d].class) {
            self.spend()?;
            pairs.push((d, m));
            if let Some(next) = self.compatible(pairs, scale)? {
                self.visit(pairs, next)?;
            }
            pairs.pop();
        }
        Some(())
    }

    fn validate_fit(&mut self, pairs: &[(usize, usize)]) -> Option<f32> {
        // Account for fitting and verification as well as search branching.
        for _ in pairs {
            self.spend()?;
        }
        let fit = fit_ground_similarity(pairs.iter().map(|&(d, m)| {
            (
                linear_algebra::Vector2::<Ground, f64>::wrap(self.detections[d].xy.cast::<f64>()),
                Point2::wrap(self.map.landmarks[m].xy.inner.cast::<f64>()),
            )
        }))
        .ok()?;
        let height = fit.camera_height;
        if let Some(heading) = self.input.heading {
            let yaw = fit.rotation.angle();
            if !heading.accepts(Orientation2::<Field, f64>::new(yaw)) {
                return None;
            }
        }
        let input = self.input;
        let config = *input.parameters;
        let camera_to_level = (input.robot_to_ground.inner
            * input.robot_to_camera.inner.rotation.inverse())
        .cast::<f64>();
        // Height of the body origin, not an assumption about foot contact.
        let lever = input.robot_to_ground.inner.cast::<f64>()
            * input
                .robot_to_camera
                .inner
                .inverse()
                .translation
                .vector
                .cast::<f64>();
        if height - lever.z <= 0.0 {
            return None;
        }
        for i in 0..3 {
            for j in 0..i {
                if (self.detections[pairs[i].0].xy - self.detections[pairs[j].0].xy).norm() as f64
                    * height
                    < config.min_detection_baseline as f64
                {
                    return None;
                }
            }
        }
        let mut squared = 0.0;
        for &(d, m) in pairs {
            let field = self.map.landmarks[m].xy.inner.coords.cast::<f64>();
            let xy = fit.rotation.inner.inverse() * (field - fit.camera_position.inner.coords);
            let leveled = Vector3::new(xy.x, xy.y, -height);
            let camera = camera_to_level.inverse() * leveled;
            if camera.z <= f64::from(config.min_reprojection_depth) {
                return None;
            }
            let pixel = Vector2::new(
                input.camera_intrinsic.focals.x as f64 * camera.x / camera.z
                    + input.camera_intrinsic.optical_center.x() as f64,
                input.camera_intrinsic.focals.y as f64 * camera.y / camera.z
                    + input.camera_intrinsic.optical_center.y() as f64,
            );
            let residual = pixel - self.detections[d].pixel.inner.coords.cast::<f64>();
            // Pixel gate includes attitude uncertainty propagated at this bearing.
            let projection = Matrix2x3::new(
                1.0 / camera.z,
                0.0,
                -camera.x / camera.z.powi(2),
                0.0,
                1.0 / camera.z,
                -camera.y / camera.z.powi(2),
            );
            let projection =
                nalgebra::Matrix2::from_diagonal(&input.camera_intrinsic.focals.cast::<f64>())
                    * projection;
            let tilt = nalgebra::Matrix2::from_columns(
                &[Vector3::x(), Vector3::y()]
                    .map(|axis| projection * (camera_to_level.inverse() * axis.cross(&leveled))),
            );
            let covariance = nalgebra::Matrix2::identity()
                * (config.detection_pixel_sigma as f64).powi(2)
                + tilt * tilt.transpose() * (config.imu_tilt_sigma as f64).powi(2);
            if residual.dot(&(covariance.try_inverse()? * residual))
                > config.mahalanobis_gate as f64
            {
                return None;
            }
            squared += residual.norm_squared();
        }
        // Preserve the metric debug field: distance RMS after fitting the scale.
        let mut metric_squared = 0.0;
        let mut count = 0;
        for (i, &(d, m)) in pairs.iter().enumerate() {
            for &(e, n) in &pairs[..i] {
                self.spend()?;
                let distance =
                    (self.detections[d].xy - self.detections[e].xy).norm() as f64 * height;
                let map_distance = (self.map.landmarks[m].xy - self.map.landmarks[n].xy)
                    .inner
                    .norm() as f64;
                metric_squared += (distance - map_distance).powi(2);
                count += 1;
            }
        }
        squared
            .is_finite()
            .then_some((metric_squared / count as f64).sqrt() as f32)
    }
}

fn canonical_pairs(
    pairs: &[(usize, usize)],
    detections: &[Detection],
    map: &LandmarkMap,
) -> Vec<(usize, usize)> {
    let mut key = pairs.to_vec();
    key.sort_by_key(|&(d, _)| detections[d].id);
    let flip = key
        .iter()
        .find_map(|&(_, m)| {
            let p = map.landmarks[m].xy;
            if p.x() != 0.0 {
                Some(p.x() > 0.0)
            } else if p.y() != 0.0 {
                Some(p.y() > 0.0)
            } else {
                None
            }
        })
        .unwrap_or(false);
    if flip {
        for (_, m) in &mut key {
            *m = map.symmetric_id(*m);
        }
    }
    key
}

pub(crate) fn associate(input: GlobalAssociationInput<'_>) -> Option<AssociationResult> {
    if input.heading.is_some_and(|heading| !heading.is_valid()) {
        return None;
    }
    let (map, detections) = preprocess(input)?;
    if detections.len() < input.parameters.min_inliers || detections.len() > map.landmarks.len() {
        return None;
    }
    let seed = select_seed(&detections, *input.parameters)?;
    let order = seed
        .into_iter()
        .chain((0..detections.len()).filter(|i| !seed.contains(i)))
        .collect();
    let mut search = Search {
        input,
        map: &map,
        detections: &detections,
        order,
        remaining_work: input.parameters.max_work,
        solution: None,
    };
    search.visit(&mut Vec::new(), (0.0, f32::INFINITY))?;
    // A validation exhausting the budget must not leave an earlier candidate certified.
    if search.remaining_work == 0 {
        return None;
    }
    let (pairs, rms) = search.solution?;
    Some(AssociationResult {
        associations: pairs
            .iter()
            .map(|&(d, m)| FieldMarkAssociation {
                detection: detections[d].pixel,
                field_point: map.landmarks[m].xy.extend(0.0),
            })
            .collect(),
        source: VisualAssociationSource::Global,
        debug: Some(GlobalLocalizationDebug {
            association_count: pairs.len(),
            pairwise_distance_rms: rms,
        }),
    })
}
