//! Reproduce timings with:
//! cargo test -p localization-fagra --release --test spline_performance -- --ignored --nocapture --test-threads=1

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
    time::Instant,
};

use linear_algebra::Pose3;
use localization_fagra::{spline::PoseSpline, variables::PoseControl};
use nalgebra::{Isometry3, RealField, UnitQuaternion, Vector3};

struct CountingAllocator;

thread_local! {
    // Count only this test thread, not allocations made by the test harness.
    static ALLOCATIONS: Cell<Option<usize>> = const { Cell::new(None) };
}

fn record_allocation() {
    let _ = ALLOCATIONS.try_with(|count| {
        if let Some(n) = count.get() {
            count.set(Some(n + 1));
        }
    });
}

// SAFETY: forwards every allocation operation unchanged to the system allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn controls<R: RealField + Copy>(angle_scale: f64) -> [PoseControl<R>; 4] {
    let c = |x| R::from_f64(x).unwrap();
    std::array::from_fn(|i| {
        let x = i as f64;
        PoseControl {
            pose: Pose3::wrap(Isometry3::from_parts(
                Vector3::new(c(0.2 * x * x), c(-0.1 * x), c(0.5 + 0.1 * x.sin())).into(),
                UnitQuaternion::from_euler_angles(
                    c(angle_scale * x),
                    c(angle_scale * x * x * 0.5),
                    c(-angle_scale * x),
                ),
            )),
        }
    })
}

fn exercise<R: RealField + Copy>() {
    let c = |x| R::from_f64(x).unwrap();
    for scale in [0.02, 0.6] {
        let controls = controls::<R>(scale);
        for i in 0..32 {
            let spline = PoseSpline::new(black_box(controls.each_ref()), c(0.2)).unwrap();
            let tau = black_box(c(i as f64 / 31.0));
            black_box(spline.pose(tau).unwrap());
            black_box(spline.velocity(tau).unwrap());
            black_box(spline.state(tau).unwrap());
            black_box(spline.kinematics(tau).unwrap());
            let linearized = spline.linearize().unwrap();
            black_box(linearized.pose(tau).unwrap());
            black_box(linearized.velocity(tau).unwrap());
            black_box(linearized.state(tau).unwrap());
            black_box(linearized.kinematics(tau).unwrap());
        }
    }
}

#[test]
fn preparation_and_evaluation_allocate_nothing() {
    ALLOCATIONS.with(|count| count.set(Some(0)));
    exercise::<f32>();
    exercise::<f64>();
    let count = ALLOCATIONS.with(|count| count.replace(None).unwrap());
    assert_eq!(count, 0, "spline preparation/evaluation allocated");
}

fn measure<T>(name: &str, mut operation: impl FnMut(usize) -> T) {
    const ITERATIONS: usize = 20_000;
    for i in 0..1000 {
        black_box(operation(i));
    }
    let mut timings = [0.0; 7];
    for timing in &mut timings {
        let start = Instant::now();
        for i in 0..ITERATIONS {
            black_box(operation(i));
        }
        *timing = start.elapsed().as_nanos() as f64 / ITERATIONS as f64;
    }
    timings.sort_by(f64::total_cmp);
    println!("{name:24} {:8.1} ns (median of 7)", timings[3]);
}

fn timings<R: RealField + Copy>() {
    let c = |x| R::from_f64(x).unwrap();
    for scale in [0.02, 0.6] {
        println!("\n{} rotation scale={scale}", std::any::type_name::<R>());
        let controls = controls::<R>(scale);
        let spline = PoseSpline::new(controls.each_ref(), c(0.2)).unwrap();
        let linearized = spline.linearize().unwrap();
        let taus: [_; 32] = std::array::from_fn(|i| c(i as f64 / 31.0));
        measure("prepare spline", |_| {
            PoseSpline::new(black_box(controls.each_ref()), black_box(c(0.2))).unwrap()
        });
        measure("prepare derivatives", |_| {
            black_box(&spline).linearize().unwrap()
        });
        measure("pose", |i| {
            black_box(&spline).pose(black_box(taus[i % 32])).unwrap()
        });
        measure("velocity", |i| {
            black_box(&spline)
                .velocity(black_box(taus[i % 32]))
                .unwrap()
        });
        measure("state", |i| {
            black_box(&spline).state(black_box(taus[i % 32])).unwrap()
        });
        measure("kinematics", |i| {
            black_box(&spline)
                .kinematics(black_box(taus[i % 32]))
                .unwrap()
        });
        measure("pose + Jacobians", |i| {
            black_box(&linearized)
                .pose(black_box(taus[i % 32]))
                .unwrap()
        });
        measure("velocity + Jacobians", |i| {
            black_box(&linearized)
                .velocity(black_box(taus[i % 32]))
                .unwrap()
        });
        measure("state + Jacobians", |i| {
            black_box(&linearized)
                .state(black_box(taus[i % 32]))
                .unwrap()
        });
        measure("kinematics + Jacobians", |i| {
            black_box(&linearized)
                .kinematics(black_box(taus[i % 32]))
                .unwrap()
        });
    }
}

#[test]
#[ignore = "release microbenchmark; use --release --ignored --nocapture --test-threads=1"]
fn release_timings() {
    timings::<f32>();
    timings::<f64>();
}

mod factor_workload {
    use super::*;
    use fagra::Variable;
    use linear_algebra::{Framed, Transform};
    use localization_fagra::{
        factors::*,
        preintegration::{ImuDelta, PreintegrationInformation},
        variables::{CameraIntrinsics, FieldAlignment, ImuBias, TrajectoryState},
    };
    use nalgebra::{Matrix3, SMatrix};

    fagra::states! { States<R> { controls: PoseControl<R>, alignment: FieldAlignment<R>, intrinsics: CameraIntrinsics<R>, biases: ImuBias<R> } }
    fagra::factors! { Factors<R> {
        trajectory: TrajectoryPrior<R>, calibration: CameraIntrinsicsPrior<R>, motion: MotionPrior<R>,
        bias_prior: ImuBiasPrior<R>, bias_walk: ImuBiasWalk<R>,
        yaw: RelativeYaw<R>, containment: FieldContainment<R>,
        imu: ImuKinematics<R>,
        preintegrated_imu: PreintegratedImu<R>,
        feet: Batch<FootGround<R>, FootObservation<R>>,
        pixels: Batch<FrameReprojections<R>, ReprojectionObservation<R>>,
        odometry: Batch<VisualOdometry<R>, VisualOdometryObservation<R>>, adjacent: AdjacentVisualOdometry<R>,
        kinematic: KinematicOdometry<R>, adjacent_kinematic: AdjacentKinematicOdometry<R>,
    } }

    fn graph<R: fagra::Real>() -> fagra::Solver<States<R>, Factors<R>> {
        let c = |x| R::from_f64(x).unwrap();
        let mut graph = fagra::Solver::new();
        let controls = controls::<R>(0.02).map(|control| graph.add(control));
        let fifth = graph.add(PoseControl::identity());
        let alignment = graph.add(FieldAlignment::identity());
        let calibration = CameraIntrinsics {
            focal_lengths: Framed::wrap(nalgebra::Vector2::new(c(300.0), c(310.0))),
            optical_center: Framed::wrap(nalgebra::Point2::new(c(160.0), c(120.0))),
        };
        let intrinsics = graph.add(calibration.clone());
        graph
            .add_factor(CameraIntrinsicsPrior {
                intrinsics,
                reference: calibration,
                information_root: SMatrix::identity(),
            })
            .unwrap();
        graph
            .add_factor(TrajectoryPrior {
                controls,
                duration: c(0.2),
                tau: c(0.0),
                reference: TrajectoryState::identity(),
                information_root: SMatrix::identity(),
            })
            .unwrap();
        graph
            .add_factor(MotionPrior {
                controls,
                duration: c(0.2),
                information_root: SMatrix::identity(),
                use_start_velocity: true,
            })
            .unwrap();
        graph
            .add_factor(RelativeYaw {
                controls,
                duration: c(0.2),
                end_tau: c(1.0),
                measured_yaw_change: c(0.1),
                information_root: c(1.0),
            })
            .unwrap();
        graph
            .add_factor(RelativeYaw {
                controls,
                duration: c(0.2),
                end_tau: c(0.6),
                measured_yaw_change: c(0.1),
                information_root: c(1.0),
            })
            .unwrap();
        graph
            .add_factor(FieldContainment {
                controls,
                duration: c(0.2),
                tau: c(0.5),
                alignment,
                half_extents: Framed::wrap(nalgebra::Vector2::new(c(5.0), c(3.0))),
                sigma: c(0.5),
            })
            .unwrap();
        graph
            .add_factor(AdjacentVisualOdometry {
                controls: [controls[0], controls[1], controls[2], controls[3], fifth],
                duration: c(0.2),
                observation: VisualOdometryObservation {
                    previous_tau: c(0.8),
                    current_tau: c(0.2),
                    current_to_previous: Transform::wrap(Isometry3::identity()),
                },
                information_root: SMatrix::identity(),
                huber_threshold: c(2.0),
            })
            .unwrap();
        let biases = std::array::from_fn(|_| graph.add(ImuBias::identity()));
        graph
            .add_factor(KinematicOdometry {
                controls,
                duration: c(0.2),
                previous_tau: c(0.2),
                current_tau: c(0.8),
                translation: Framed::wrap(nalgebra::Vector2::zeros()),
                information_root: nalgebra::Matrix2::identity(),
                huber_threshold: c(2.0),
            })
            .unwrap();
        graph
            .add_factor(AdjacentKinematicOdometry {
                controls: [controls[0], controls[1], controls[2], controls[3], fifth],
                duration: c(0.2),
                previous_tau: c(0.8),
                current_tau: c(0.2),
                translation: Framed::wrap(nalgebra::Vector2::zeros()),
                information_root: nalgebra::Matrix2::identity(),
                huber_threshold: c(2.0),
            })
            .unwrap();
        for information in [
            PreintegrationInformation::Rotation(Matrix3::identity()),
            PreintegrationInformation::Full(SMatrix::identity()),
        ] {
            graph
                .add_factor(PreintegratedImu {
                    controls,
                    biases,
                    duration: c(0.2),
                    start_tau: c(0.0),
                    end_tau: c(0.5),
                    delta: ImuDelta {
                        duration: c(0.1),
                        rotation: Transform::wrap(UnitQuaternion::identity()),
                        velocity: Framed::wrap(Vector3::new(c(0.0), c(0.0), c(0.981))),
                        position: Framed::wrap(Vector3::new(c(0.0), c(0.0), c(0.04905))),
                        reference_biases: std::array::from_fn(|_| ImuBias::identity()),
                        bias_jacobians: [SMatrix::repeat(c(0.01)); 2],
                    },
                    information,
                    gravity_compensation: Framed::wrap(Vector3::new(c(0.0), c(0.0), c(9.81))),
                    position: Framed::wrap(Vector3::new(c(0.1), c(0.0), c(0.05))),
                })
                .unwrap();
        }
        graph
            .add_factor(ImuBiasPrior {
                bias: biases[0],
                reference: ImuBias::identity(),
                information_root: SMatrix::identity(),
            })
            .unwrap();
        graph
            .add_factor(ImuBiasWalk {
                biases,
                information_root: SMatrix::identity(),
            })
            .unwrap();
        let imu = ImuKinematics {
            controls,
            biases,
            duration: c(0.2),
            gyroscope_information_root: Matrix3::identity(),
            tilt_information_root: Matrix3::identity(),
            tau: c(0.0),
            bias_tau: c(0.0),
            angular_velocity: Framed::wrap(Vector3::zeros()),
            measured_up: Some(Framed::wrap(Vector3::z())),
        };
        let feet = graph.add_batch(FootGround {
            controls,
            duration: c(0.2),
            sigma: c(0.1),
        });
        let odometry = graph.add_batch(VisualOdometry {
            controls,
            duration: c(0.2),
            information_root: SMatrix::identity(),
            huber_threshold: c(2.0),
        });
        for i in 0..10 {
            let tau = c(i as f64 / 10.0);
            graph
                .add_factor(ImuKinematics {
                    tau,
                    bias_tau: tau,
                    ..imu.clone()
                })
                .unwrap();
            graph
                .add_factor_to(
                    feet,
                    FootObservation {
                        tau,
                        left_sole: Framed::wrap(nalgebra::Point3::new(c(0.0), c(0.1), c(-0.6))),
                        right_sole: Framed::wrap(nalgebra::Point3::new(c(0.0), c(-0.1), c(-0.6))),
                    },
                )
                .unwrap();
            graph
                .add_factor_to(
                    odometry,
                    VisualOdometryObservation {
                        previous_tau: tau,
                        current_tau: tau + c(0.05),
                        current_to_previous: Transform::wrap(Isometry3::identity()),
                    },
                )
                .unwrap();
        }
        let pixels = graph.add_batch(FrameReprojections {
            controls,
            alignment,
            intrinsics,
            duration: c(0.2),
            tau: c(0.5),
            robot_to_camera: Transform::wrap(Isometry3::identity()),
            angular_information_root: c(300.0),
            huber_threshold: c(2.0),
            min_range: c(0.01),
        });
        for i in 0..100 {
            graph
                .add_factor_to(
                    pixels,
                    ReprojectionObservation {
                        field_point: Framed::wrap(nalgebra::Point3::new(
                            c(i as f64 * 0.01),
                            c(0.0),
                            c(5.0),
                        )),
                        detection: Framed::wrap(nalgebra::Point2::new(c(160.0), c(120.0))),
                    },
                )
                .unwrap();
        }
        graph
    }

    fn check_allocations<R: fagra::Real>() {
        let mut graph = graph::<R>();
        let mut method = fagra::GaussNewton::default();
        // Deliberate evaluation-only pass: evaluate cost and assemble all factor
        // Jacobians, then stop at the gradient check without changing any state.
        let options = fagra::OptimizeOptions {
            gradient_tolerance: R::from_f64(1e30).unwrap(),
            ..Default::default()
        };
        graph.optimize_with(&mut method, &options).unwrap();
        ALLOCATIONS.with(|count| count.set(Some(0)));
        for _ in 0..10 {
            black_box(graph.optimize_with(&mut method, &options).unwrap());
        }
        let count = ALLOCATIONS.with(|count| count.replace(None).unwrap());
        assert_eq!(count, 0, "warmed factor evaluation allocated");
    }

    #[test]
    fn all_factors_allocate_nothing_after_workspace_warmup() {
        check_allocations::<f32>();
        check_allocations::<f64>();
    }

    #[test]
    #[ignore = "release microbenchmark; cost + Jacobians + dense assembly, no solve"]
    fn all_factor_timings() {
        let mut graph = graph::<f64>();
        let mut method = fagra::GaussNewton::default();
        let options = fagra::OptimizeOptions {
            gradient_tolerance: 1e30,
            ..Default::default()
        };
        measure("factor workload (f64)", |_| {
            graph.optimize_with(&mut method, &options).unwrap()
        });
    }
}
