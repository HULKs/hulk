use nalgebra::RealField;
use num_dual::DualNum;

use coordinate_systems::Ground;
use linear_algebra::Orientation2;
use step_plan::StepPlan;
use types::{
    motion_command::OrientationMode, parameters::StepPlanningOptimizationParameters,
    planned_path::Path, support_foot::Side, walk_volume_extents::WalkVolumeExtents,
};

use crate::{
    cost_fields::{
        path_distance::PathDistanceField, path_progress::PathProgressField,
        target_orientation::TargetOrientationField, walk_orientation::WalkOrientationField,
    },
    geometry::{
        angle::Angle,
        pose::{Pose, PoseGradient},
    },
    traits::{Length, PathProgress},
};

pub mod cost_fields;
pub mod geometry;
pub mod step_plan;
pub mod traits;
pub mod utils;

pub const VARIABLES_PER_STEP: usize = 3;

#[derive(Clone, Debug)]
pub struct StepPlanning<'a> {
    pub path: &'a Path,
    pub target_orientation: Orientation2<Ground>,
    pub distance_to_be_aligned: f32,
    pub initial_pose: Pose<f32>,
    pub initial_support_foot: Side,
    pub orientation_mode: OrientationMode,
    pub walk_volume_extents: &'a WalkVolumeExtents,
    pub parameters: &'a StepPlanningOptimizationParameters,
}

impl StepPlanning<'_> {
    pub fn step_end_poses<'a, T: RealField + DualNum<f32>>(
        &self,
        initial_pose: Pose<T>,
        initial_support_side: Side,
        walk_volume_extents: WalkVolumeExtents,
        step_plan: &StepPlan<'a, T>,
    ) -> impl Iterator<Item = Pose<T>> + 'a {
        step_plan.steps().scan(
            (initial_pose, initial_support_side),
            move |(pose, support_side), step| {
                *pose += step.unnormalize(&walk_volume_extents, *support_side);
                *support_side = support_side.opposite();

                Some(pose.clone())
            },
        )
    }

    pub fn cost(&self, pose: Pose<f32>) -> f32 {
        let cost_factors = &self.parameters.cost_factors;

        let progress = self.path.progress(pose.position);
        let forward = self.path.forward(pose.position);
        let path_length = self.path.length();
        let distance_to_target = path_length - progress;
        let target_alignment_importance = self.target_alignment_importance(distance_to_target);
        let walk_alignment_importance = 1.0 - target_alignment_importance;

        let path_progress_cost =
            self.path_progress().cost(progress, path_length) * cost_factors.path_progress;
        let path_distance_cost =
            self.path_distance().cost(pose.position) * cost_factors.path_distance;
        let walk_orientation_cost = self.walk_orientation().cost(pose.clone(), forward)
            * cost_factors.walk_orientation
            * walk_alignment_importance;
        let target_orientation_cost = self.target_orientation().cost(pose)
            * cost_factors.target_orientation
            * target_alignment_importance;

        path_progress_cost + path_distance_cost + walk_orientation_cost + target_orientation_cost
    }

    pub fn grad(&self, pose: Pose<f32>) -> PoseGradient<f32> {
        let cost_factors = &self.parameters.cost_factors;

        let progress = self.path.progress(pose.position);
        let forward = self.path.forward(pose.position);
        let path_length = self.path.length();
        let distance_to_target = path_length - progress;
        let target_alignment_importance = self.target_alignment_importance(distance_to_target);
        let walk_alignment_importance = 1.0 - target_alignment_importance;

        let path_progress_gradient =
            self.path_progress().grad(progress, forward, path_length) * cost_factors.path_progress;
        let path_distance_gradient =
            self.path_distance().grad(pose.position) * cost_factors.path_distance;
        let walk_orientation_gradient = self.walk_orientation().grad(pose.clone(), forward)
            * cost_factors.walk_orientation
            * walk_alignment_importance;
        let target_orientation_gradient = self.target_orientation().grad(pose)
            * cost_factors.target_orientation
            * target_alignment_importance;

        PoseGradient {
            position: path_progress_gradient + path_distance_gradient,
            ..PoseGradient::zeros()
        } + walk_orientation_gradient
            + target_orientation_gradient
    }

    // https://www.desmos.com/calculator/mzuvbmrxym
    fn target_alignment_importance(&self, distance_to_target: f32) -> f32 {
        (1.0 - f32::tanh(
            (distance_to_target - self.distance_to_be_aligned)
                * self.parameters.alignment_ramp_steepness,
        )) / 2.0
    }

    fn path_distance(&self) -> PathDistanceField<'_> {
        PathDistanceField { path: self.path }
    }

    fn path_progress(&self) -> PathProgressField {
        PathProgressField {
            smoothness: self.parameters.path_progress_smoothness,
        }
    }

    fn walk_orientation(&self) -> WalkOrientationField {
        WalkOrientationField {
            orientation_mode: self.orientation_mode,
            path_alignment_tolerance: self.parameters.path_alignment_tolerance,
        }
    }

    fn target_orientation(&self) -> TargetOrientationField {
        TargetOrientationField {
            target_orientation: Angle(self.target_orientation.angle()),
        }
    }
}

#[cfg(test)]
pub mod test_utils {
    pub mod decompose;
    pub mod gradient_type;
    pub mod verify_gradient;

    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    use approx::AbsDiffEq;
    use coordinate_systems::Ground;
    use geometry::{arc::Arc, circle::Circle, direction::Direction, line_segment::LineSegment};
    use linear_algebra::{point, vector, Orientation2, Point2};
    use types::planned_path::{Path, PathSegment};

    pub fn test_path() -> Path {
        Path {
            segments: vec![
                PathSegment::LineSegment(LineSegment(point![0.0, 0.0], point![3.0, 0.0])),
                PathSegment::Arc(Arc {
                    circle: Circle {
                        center: point![3.0, 1.0],
                        radius: 1.0,
                    },
                    start: Orientation2::new(3.0 * FRAC_PI_2),
                    end: Orientation2::new(0.0),
                    direction: Direction::Counterclockwise,
                }),
                PathSegment::LineSegment(LineSegment(point![4.0, 1.0], point![4.0, 4.0])),
            ],
        }
    }

    pub fn is_near_test_path_segment_joins(query_point: Point2<Ground>) -> bool {
        is_near_ray(
            query_point,
            point![3.0, 1.0],
            Orientation2::from_vector(vector![1.0, 0.0]),
        ) || is_near_ray(
            query_point,
            point![3.0, 1.0],
            Orientation2::from_vector(vector![0.0, -1.0]),
        ) || is_near_test_path_progress_discontinuity(query_point)
    }

    pub fn is_near_test_path_progress_discontinuity(query_point: Point2<Ground>) -> bool {
        is_near_ray(
            query_point,
            point![3.0, 1.0],
            Orientation2::from_vector(vector![-1.0, 1.0]),
        )
    }

    fn is_near_ray(
        query_point: Point2<Ground>,
        start: Point2<Ground>,
        direction: Orientation2<Ground>,
    ) -> bool {
        let direction_vector = direction.as_unit_vector();
        let start_to_query = query_point - start;

        let t = start_to_query.dot(&direction_vector);
        let query_projected_onto_line = start + direction_vector * t.max(0.0);

        let squared_distance_to_ray = (query_point - query_projected_onto_line).norm_squared();

        squared_distance_to_ray < 1e-2
    }

    pub fn is_roughly_opposite(a: f32, b: f32) -> bool {
        (a - b).rem_euclid(TAU).abs_diff_eq(&PI, 1e-2)
    }

    #[cfg(test)]
    mod tests {
        use linear_algebra::point;

        use crate::test_utils::is_near_test_path_progress_discontinuity;

        #[test]
        fn test_is_near_test_path_discontinuity() {
            assert!(is_near_test_path_progress_discontinuity(point![3.0, 1.0]));
            assert!(is_near_test_path_progress_discontinuity(point![2.0, 2.0]));
            assert!(is_near_test_path_progress_discontinuity(point![1.0, 3.0]));

            assert!(!is_near_test_path_progress_discontinuity(point![4.0, 0.0]));
            assert!(!is_near_test_path_progress_discontinuity(point![2.5, 2.5]));
        }
    }
}
