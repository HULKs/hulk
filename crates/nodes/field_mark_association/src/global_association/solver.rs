use std::collections::HashSet;

use coordinate_systems::{Field, Pixel};
use linear_algebra::{Orientation2, Point2};
use localization_fagra::alignment::{GravityCamera, GroundPose};
use nalgebra::{Matrix2, Vector2, Vector3};
use ndarray::Array2;
use types::visual_localization::{
    FieldMarkAssociation, GlobalLocalizationDebug, VisualAssociationSource,
};

use super::GlobalAssociationConfig;
use crate::{
    AssociationResult, DetectedVisualFeature, GlobalAssociationInput, VisualFeatureClass,
    assignment::unique_assignment, features::filter_detections, map::LandmarkMap,
};

const HYPOTHESES: usize = 128;
const PROPOSALS: usize = HYPOTHESES * 16;
const CANDIDATES: usize = 8;
const MIN_PAIR_PIXELS: f32 = 20.0;

#[derive(Clone, Debug)]
pub(crate) struct Detection {
    pub id: usize,
    pub class: VisualFeatureClass,
    pub pixel: Point2<Pixel>,
    pub confidence: f32,
    pub xy: Vector2<f32>,
}

/// Exposure-time tilt only: no optimizer translation or assumed standing height.
pub(crate) fn preprocess(
    input: GlobalAssociationInput<'_>,
) -> Option<(LandmarkMap, Vec<Detection>, usize)> {
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
    let retained = filter_detections(input.visual_features, &map, config)?;
    let observation_count = retained.len();
    let detections = retained
        .into_iter()
        .filter_map(|(id, class, feature)| project_detection(input, id, class, feature))
        .collect();
    Some((map, detections, observation_count))
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
    let detection = Detection {
        id,
        class,
        pixel: feature.pixel,
        confidence: feature.confidence,
        xy: -ray.xy() / ray.z,
    };
    detection
        .xy
        .iter()
        .all(|x| x.is_finite())
        .then_some(detection)
}

#[derive(Clone)]
struct Candidate {
    pose: GroundPose,
    pairs: Vec<(usize, usize)>,
    score: f64,
    rms: f64,
}

struct Prediction {
    pixel: Vector2<f64>,
    information: Matrix2<f64>,
}

#[derive(Clone, Copy)]
struct Edge {
    detection: usize,
    landmark: usize,
    benefit: f64,
    squared_error: f64,
}

struct Search<'a> {
    input: GlobalAssociationInput<'a>,
    map: &'a LandmarkMap,
    detections: &'a [Detection],
    observation_count: usize,
    camera: GravityCamera,
    body_offset_z: f64,
    remaining_work: usize,
    predictions: Vec<Option<Prediction>>,
    edges: Vec<Edge>,
    used_detections: Vec<bool>,
    used_landmarks: Vec<bool>,
}

impl Search<'_> {
    fn spend(&mut self, work: usize) -> Option<()> {
        self.remaining_work = self.remaining_work.checked_sub(work)?;
        Some(())
    }

    fn valid_pose(&self, pose: GroundPose) -> bool {
        let config = self.input.parameters;
        pose.is_valid()
            && pose.height > self.body_offset_z
            && (f64::from(config.min_camera_height)..=f64::from(config.max_camera_height))
                .contains(&pose.height)
            && self
                .input
                .heading
                .is_none_or(|heading| heading.accepts(Orientation2::<Field, f64>::new(pose.yaw)))
    }

    fn predict(&mut self, pose: GroundPose) -> Option<()> {
        self.spend(self.map.landmarks.len())?;
        let config = self.input.parameters;
        for (slot, landmark) in self.predictions.iter_mut().zip(&self.map.landmarks) {
            *slot = self
                .camera
                .project(pose, landmark.xy.inner.coords.cast())
                .and_then(|projection| {
                    let covariance = Matrix2::identity()
                        * f64::from(config.detection_pixel_sigma).powi(2)
                        + projection.tilt_jacobian
                            * projection.tilt_jacobian.transpose()
                            * f64::from(config.imu_tilt_sigma).powi(2);
                    Some(Prediction {
                        pixel: projection.pixel,
                        information: covariance.try_inverse()?,
                    })
                });
        }
        Some(())
    }

    fn edge(&self, detection: usize, landmark: usize) -> Option<Edge> {
        let observation = &self.detections[detection];
        let prediction = self.predictions[landmark].as_ref()?;
        let config = self.input.parameters;
        let penalty = class_penalty(
            observation.class,
            self.map.landmarks[landmark].class,
            config,
        )?;
        let error = prediction.pixel - observation.pixel.inner.coords.cast::<f64>();
        let distance = error.dot(&(prediction.information * error)) + penalty;
        let gate = f64::from(config.mahalanobis_gate);
        if !distance.is_finite() || !(0.0..gate).contains(&distance) {
            return None;
        }
        Some(Edge {
            detection,
            landmark,
            benefit: (gate - distance) * f64::from(observation.confidence),
            squared_error: error.norm_squared(),
        })
    }

    fn score(&mut self, pose: GroundPose) -> Option<Candidate> {
        self.predict(pose)?;
        // Each assignment row scans the fixed field map; iteration counts are bounded separately.
        self.spend(self.detections.len())?;
        self.edges.clear();
        for detection in 0..self.detections.len() {
            for landmark in 0..self.map.landmarks.len() {
                if let Some(edge) = self.edge(detection, landmark) {
                    self.edges.push(edge);
                }
            }
        }
        self.edges.sort_unstable_by(|a, b| {
            b.benefit
                .total_cmp(&a.benefit)
                .then_with(|| (a.detection, a.landmark).cmp(&(b.detection, b.landmark)))
        });
        self.used_detections.fill(false);
        self.used_landmarks.fill(false);
        let mut pairs = Vec::new();
        // ponytail: greedy assignment screens hypotheses; exact assignment certifies the final pose.
        for edge in &self.edges {
            if self.used_detections[edge.detection] || self.used_landmarks[edge.landmark] {
                continue;
            }
            self.used_detections[edge.detection] = true;
            self.used_landmarks[edge.landmark] = true;
            pairs.push((edge.detection, edge.landmark));
        }
        pairs.sort_unstable();
        self.candidate(pose, pairs)
    }

    fn candidate(&self, pose: GroundPose, pairs: Vec<(usize, usize)>) -> Option<Candidate> {
        let mut score = 0.0;
        let mut squared = 0.0;
        for &(d, m) in &pairs {
            let edge = self.edge(d, m)?;
            score += edge.benefit;
            squared += edge.squared_error;
        }
        let rms = if pairs.is_empty() {
            f64::INFINITY
        } else {
            (squared / pairs.len() as f64).sqrt()
        };
        let mut candidate = Candidate {
            pose,
            pairs,
            score,
            rms,
        };
        self.canonicalize(&mut candidate);
        Some(candidate)
    }

    fn canonicalize(&self, candidate: &mut Candidate) {
        if self.input.heading.is_some() {
            return;
        }
        let flip = candidate
            .pairs
            .iter()
            .filter_map(|&(d, m)| {
                let point = self.map.landmarks[m].xy;
                (point.x() != 0.0 || point.y() != 0.0).then_some((self.detections[d].id, point))
            })
            .min_by_key(|&(id, _)| id)
            .is_some_and(|(_, p)| {
                if p.x() != 0.0 {
                    p.x() > 0.0
                } else {
                    p.y() > 0.0
                }
            });
        if flip {
            candidate.pose.position = -candidate.pose.position;
            candidate.pose.yaw += std::f64::consts::PI;
            for (_, m) in &mut candidate.pairs {
                *m = self.map.symmetric_id(*m);
            }
        }
    }

    fn refine(&mut self, mut candidate: Candidate) -> Option<Candidate> {
        for _ in 0..2 {
            // Reserve the ceiling for eight normal-equation builds and four trial evaluations/step.
            self.spend(48 * candidate.pairs.len())?;
            let observations = candidate
                .pairs
                .iter()
                .map(|&(d, m)| {
                    (
                        self.map.landmarks[m].xy.inner.coords.cast::<f64>(),
                        self.detections[d].pixel.inner.coords.cast::<f64>(),
                    )
                })
                .collect::<Vec<_>>();
            let Some(pose) = self
                .camera
                .refine(candidate.pose, &observations, |pose| self.valid_pose(pose))
            else {
                break;
            };
            let next = self.score(pose)?;
            let unchanged = next.pairs == candidate.pairs;
            candidate = next;
            if unchanged {
                break;
            }
        }
        Some(candidate)
    }

    fn consensus(&self, candidate: &Candidate) -> bool {
        let config = self.input.parameters;
        let minimum = minimum_inliers(self.observation_count, config);
        candidate.pairs.len() >= minimum
            && candidate.rms <= f64::from(config.max_rms_px)
            && spread(
                &candidate.pairs,
                self.detections,
                self.map,
                candidate.pose.height,
                config,
            )
    }

    fn certify(&mut self, winner: Candidate) -> Option<Candidate> {
        self.predict(winner.pose)?;
        let n = self.detections.len();
        let m = self.map.landmarks.len();
        self.spend(n.checked_mul(n)?.checked_mul(n + 1)?)?;
        let mut benefits = Array2::zeros((n, m + n));
        for d in 0..n {
            for l in 0..m {
                if let Some(edge) = self.edge(d, l) {
                    benefits[(d, l)] = edge.benefit as f32;
                }
            }
        }
        let pairs = unique_assignment(&mut benefits, m, self.input.parameters.score_ratio)?;
        let winner = self.candidate(winner.pose, pairs)?;
        self.consensus(&winner).then_some(winner)
    }
}

fn class_penalty(
    observed: VisualFeatureClass,
    landmark: VisualFeatureClass,
    config: &GlobalAssociationConfig,
) -> Option<f64> {
    use VisualFeatureClass::{LSpot, TSpot, XSpot};
    if observed == landmark {
        Some(0.0)
    } else if matches!(observed, LSpot | TSpot | XSpot) && matches!(landmark, LSpot | TSpot | XSpot)
    {
        Some(f64::from(config.class_mismatch_penalty))
    } else {
        None
    }
}

fn minimum_inliers(observations: usize, config: &GlobalAssociationConfig) -> usize {
    if observations == 3 {
        config.min_inliers
    } else {
        config
            .min_inliers
            .max(4)
            .max((observations as f32 * config.min_inlier_fraction).ceil() as usize)
    }
}

fn spread(
    pairs: &[(usize, usize)],
    detections: &[Detection],
    map: &LandmarkMap,
    height: f64,
    config: &GlobalAssociationConfig,
) -> bool {
    let n = pairs.len().min(config.seed_pool_size);
    for a in 0..n {
        for b in a + 1..n {
            for c in b + 1..n {
                let [(da, ma), (db, mb), (dc, mc)] = [pairs[a], pairs[b], pairs[c]];
                let q = [detections[da].xy, detections[db].xy, detections[dc].xy];
                let p = [
                    map.landmarks[ma].xy.inner.coords,
                    map.landmarks[mb].xy.inner.coords,
                    map.landmarks[mc].xy.inner.coords,
                ];
                let quality = |points: [Vector2<f32>; 3]| {
                    let u = points[1] - points[0];
                    let v = points[2] - points[0];
                    (u.x * v.y - u.y * v.x).abs()
                        / (u.norm_squared() + v.norm_squared()).max(config.min_triangle_denominator)
                };
                if quality(q) > config.min_seed_quality
                    && quality(p) > config.min_seed_quality
                    && (0..3).all(|i| {
                        (0..i).all(|j| {
                            f64::from((q[i] - q[j]).norm()) * height
                                >= f64::from(config.min_detection_baseline)
                        })
                    })
                {
                    return true;
                }
            }
        }
    }
    false
}

fn keep(candidates: &mut Vec<Candidate>, candidate: Candidate) {
    if candidate.pairs.len() < 3 {
        return;
    }
    if let Some(index) = candidates.iter().position(|other| {
        let compatible = candidate
            .pairs
            .iter()
            .all(|&(d, m)| other.pairs.iter().all(|&(e, n)| d != e || m == n));
        let yaw = candidate.pose.yaw - other.pose.yaw;
        candidate.pairs == other.pairs
            || (compatible
                && (candidate.pose.position - other.pose.position).norm() <= 0.4
                && yaw.sin().atan2(yaw.cos()).abs() <= 0.12
                && (candidate.pose.height - other.pose.height).abs() <= 0.12)
    }) {
        if candidates[index].score >= candidate.score {
            return;
        }
        candidates.remove(index);
    }
    candidates.push(candidate);
    candidates.sort_unstable_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.pairs.cmp(&b.pairs))
    });
    candidates.truncate(CANDIDATES);
}

/// Per-frame reproducible sampling; no global RNG state, workers or parallel loops.
fn draw(state: &mut u64, limit: usize) -> usize {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state as usize % limit
}

pub(crate) fn associate(input: GlobalAssociationInput<'_>) -> Option<AssociationResult> {
    if input.heading.is_some_and(|heading| !heading.is_valid()) {
        return None;
    }
    let (map, detections, observation_count) = preprocess(input)?;
    let minimum = minimum_inliers(observation_count, input.parameters);
    if detections.len() < minimum || map.landmarks.len() < minimum {
        return None;
    }
    let camera_to_level = (input.robot_to_ground.inner
        * input.robot_to_camera.inner.rotation.inverse())
    .cast::<f64>();
    let body_offset_z = (input.robot_to_ground.inner.cast::<f64>()
        * input
            .robot_to_camera
            .inner
            .inverse()
            .translation
            .vector
            .cast::<f64>())
    .z;
    let mut search = Search {
        input,
        map: &map,
        detections: &detections,
        observation_count,
        camera: GravityCamera::new(
            camera_to_level.to_rotation_matrix().into_inner(),
            input.camera_intrinsic.focals.cast(),
            input.camera_intrinsic.optical_center.inner.coords.cast(),
            f64::from(input.parameters.min_reprojection_depth),
        )?,
        body_offset_z,
        remaining_work: input.parameters.max_work,
        predictions: (0..map.landmarks.len()).map(|_| None).collect(),
        edges: Vec::with_capacity(detections.len() * map.landmarks.len()),
        used_detections: vec![false; detections.len()],
        used_landmarks: vec![false; map.landmarks.len()],
    };
    let mut pool = Vec::new();
    let mut weight = 0.0;
    for a in 0..detections.len() {
        for b in a + 1..detections.len() {
            search.spend(1)?;
            let distance = (detections[a].pixel - detections[b].pixel).inner.norm();
            if distance < MIN_PAIR_PIXELS {
                continue;
            }
            weight += f64::from(
                distance.min(200.0)
                    * detections[a].confidence
                    * detections[b].confidence
                    * map.rarity_weight(detections[a].class)
                    * map.rarity_weight(detections[b].class),
            );
            pool.push((weight, a, b));
        }
    }
    if pool.is_empty() || !weight.is_finite() || weight <= 0.0 {
        return None;
    }
    let mut random = detections.iter().fold(0x9e3779b97f4a7c15_u64, |state, d| {
        state.rotate_left(7)
            ^ u64::from(d.pixel.x().to_bits())
            ^ (u64::from(d.pixel.y().to_bits()) << 32)
    }) | 1;
    let mut visited = HashSet::with_capacity(PROPOSALS);
    let mut candidates = Vec::with_capacity(CANDIDATES + 1);
    let mut hypotheses = 0;
    for _ in 0..PROPOSALS {
        if hypotheses == HYPOTHESES {
            break;
        }
        search.spend(1)?;
        let target = draw(&mut random, 1_000_000) as f64 / 1_000_000.0 * weight;
        let &(_, a, b) = &pool[pool
            .partition_point(|&(w, _, _)| w < target)
            .min(pool.len() - 1)];
        let left = map.landmarks_for_class(detections[a].class);
        let right = map.landmarks_for_class(detections[b].class);
        if left.is_empty() || right.is_empty() {
            continue;
        }
        let m = left[draw(&mut random, left.len())];
        let n = right[draw(&mut random, right.len())];
        if m == n || !visited.insert((a, b, m, n)) {
            continue;
        }
        let field = [
            map.landmarks[m].xy.inner.coords.cast::<f64>(),
            map.landmarks[n].xy.inner.coords.cast::<f64>(),
        ];
        if (field[0] - field[1]).norm() < f64::from(input.parameters.min_detection_baseline) {
            continue;
        }
        let Some(pose) = GroundPose::from_pair(
            [detections[a].xy.cast(), detections[b].xy.cast()],
            field,
            f64::from(input.parameters.min_pair_distance),
        ) else {
            continue;
        };
        if !search.valid_pose(pose) {
            continue;
        }
        hypotheses += 1;
        keep(&mut candidates, search.score(pose)?);
    }
    let mut refined = Vec::with_capacity(CANDIDATES + 1);
    for candidate in candidates {
        let candidate = search.refine(candidate)?;
        if search.consensus(&candidate) {
            keep(&mut refined, candidate);
        }
    }
    let winner = refined.first()?;
    if refined
        .get(1)
        .is_some_and(|rival| winner.score <= rival.score * f64::from(input.parameters.score_ratio))
    {
        return None;
    }
    // Exact assignment checks the winning pose only. RANSAC does not prove absence of unsampled poses.
    let mut winner = search.certify(winner.clone())?;
    winner.pairs.sort_by_key(|&(d, _)| detections[d].id);
    let mut squared = 0.0;
    let mut count = 0;
    for (i, &(d, m)) in winner.pairs.iter().enumerate() {
        for &(e, n) in &winner.pairs[..i] {
            search.spend(1)?;
            let distance =
                f64::from((detections[d].xy - detections[e].xy).norm()) * winner.pose.height;
            let metric = f64::from((map.landmarks[m].xy - map.landmarks[n].xy).inner.norm());
            squared += (distance - metric).powi(2);
            count += 1;
        }
    }
    Some(AssociationResult {
        associations: winner
            .pairs
            .iter()
            .map(|&(d, m)| FieldMarkAssociation {
                detection: detections[d].pixel,
                field_point: map.landmarks[m].xy.extend(0.0),
            })
            .collect(),
        source: VisualAssociationSource::Global,
        debug: Some(GlobalLocalizationDebug {
            association_count: winner.pairs.len(),
            pairwise_distance_rms: (squared / count as f64).sqrt() as f32,
        }),
    })
}
