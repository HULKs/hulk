use std::{f32::consts::FRAC_PI_2, hint::black_box, time::Duration, vec::Vec};

use criterion::{criterion_group, criterion_main, Criterion};
use geometry::{arc::Arc, circle::Circle, direction::Direction, line_segment::LineSegment};
use linear_algebra::{point, Orientation2, Point2};
use nalgebra::DVector;
use step_planning::geometry::{angle::Angle, pose::Pose};
use types::{
    motion_command::OrientationMode,
    parameters::{StepPlanningCostFactors, StepPlanningOptimizationParameters},
    planned_path::{Path, PathSegment},
    step::Step,
    support_foot::Side,
    walk_volume_extents::WalkVolumeExtents,
};

fn plan_steps(path: &Path) -> Vec<Step> {
    const STEP_PLANNING_OPTIMIZATION_PARAMETERS: StepPlanningOptimizationParameters =
        StepPlanningOptimizationParameters {
            optimizer_steps: 20,
            num_steps: 10,
            walk_volume_extents: WalkVolumeExtents {
                forward: 0.045,
                backward: 0.03,
                outward: 0.1,
                inward: 0.01,
                outward_rotation: 0.5,
                inward_rotation: 0.5,
            },
            cost_factors: StepPlanningCostFactors {
                path_progress: 0.5,
                path_distance: 10.0,
                target_orientation: 1.0,
                walk_orientation: 0.1,
            },
            path_progress_smoothness: 0.05,
            alignment_ramp_steepness: 50.0,
        };

    let distance_to_be_aligned = 0.1;

    let (step_plan, _, _) = step_planning_solver::plan_steps(
        path,
        OrientationMode::Unspecified,
        Orientation2::identity(),
        distance_to_be_aligned,
        Pose {
            position: Point2::origin(),
            orientation: Angle(0.0),
        },
        Side::Left,
        DVector::zeros(STEP_PLANNING_OPTIMIZATION_PARAMETERS.num_steps * 3),
        &black_box(STEP_PLANNING_OPTIMIZATION_PARAMETERS),
    )
    .unwrap();

    step_plan
}

fn straight_line(c: &mut Criterion) {
    let path = Path {
        segments: vec![PathSegment::LineSegment(LineSegment(
            Point2::origin(),
            point![3.0, 0.0],
        ))],
    };

    c.bench_function("straight line", |b| b.iter(|| plan_steps(black_box(&path))));
}

fn example_path(c: &mut Criterion) {
    let path = Path {
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
    };

    c.bench_function("example path", |b| b.iter(|| plan_steps(black_box(&path))));
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(20)).sample_size(200);
    targets = straight_line,example_path
}
criterion_main!(benches);
