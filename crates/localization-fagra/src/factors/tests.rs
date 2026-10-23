use fagra::{
    BlockId, EvaluationError, Factor, FactorId, JacobianBlock, LinearizationSink, StateKey,
    StateStore, Variable,
    testing::{
        self, FactorProperty, TestFactor, TestFactorBatch, TestStates, proptest::prelude::*,
    },
};
use linear_algebra::{Framed, Transform};
use nalgebra::{DimName, Isometry2, Isometry3, RealField, SMatrix, UnitQuaternion, Vector3};

use super::*;
use crate::variables::{CameraIntrinsics, FieldAlignment, ImuBias, PoseControl, TrajectoryState};

fn c<R: RealField>(x: f64) -> R {
    R::from_f64(x).unwrap()
}

fn root<R: RealField + Copy, const N: usize>() -> SMatrix<R, N, N> {
    // Correlated residual coordinates catch transposed/incorrect whitening.
    SMatrix::from_fn(|i, j| {
        if i == j {
            c(1.0 + i as f64 * 0.1)
        } else if i > j {
            c(0.07)
        } else {
            R::zero()
        }
    })
}

#[derive(Clone, Debug)]
struct Inputs {
    values: [f64; 9],
    bridge: bool,
    stationary: bool,
}

impl Inputs {
    fn cases() -> impl Strategy<Value = Self> {
        prop_oneof![
            Just(Self {
                values: [0.0; 9],
                bridge: false,
                stationary: true
            }),
            (prop::array::uniform9(-0.3..0.3), any::<bool>()).prop_map(|(values, bridge)| Self {
                values,
                bridge,
                stationary: false
            }),
        ]
    }

    fn control<R: RealField + Copy>(&self, i: usize) -> PoseControl<R> {
        if self.stationary {
            return PoseControl::identity();
        }
        let x = i as f64;
        let v = self.values;
        PoseControl {
            pose: Framed::wrap(Isometry3::from_parts(
                Vector3::new(c(v[3] + x * 0.15), c(v[4] - x * x * 0.03), c(0.6 + v[5])).into(),
                UnitQuaternion::from_euler_angles(
                    c(v[0] + x * 0.04),
                    c(v[1] - x * 0.03),
                    c(v[2] + x * 0.06),
                ),
            )),
        }
    }

    fn controls<R: RealField + Copy, const N: usize>(
        &self,
        states: &mut TestStates<R>,
    ) -> [StateKey<PoseControl<R>>; N] {
        std::array::from_fn(|i| states.insert(self.control(i)))
    }

    fn measured_up<R: RealField + Copy>(
        &self,
    ) -> linear_algebra::Vector3<coordinate_systems::Robot, R> {
        Framed::wrap(
            UnitQuaternion::from_euler_angles(
                c(self.values[6]),
                c(self.values[7]),
                c(self.values[8]),
            )
            .inverse()
                * Vector3::z(),
        )
    }

    fn odometry<R: RealField + Copy>(
        &self,
        previous: f64,
        current: f64,
    ) -> VisualOdometryObservation<R> {
        VisualOdometryObservation {
            previous_tau: c(previous),
            current_tau: c(current),
            current_to_previous: Transform::wrap(Isometry3::from_parts(
                Vector3::new(c(self.values[6]), c(self.values[7]), c(self.values[8])).into(),
                UnitQuaternion::from_euler_angles(
                    c(self.values[7]),
                    c(self.values[8]),
                    c(self.values[6]),
                ),
            )),
        }
    }
}

macro_rules! ordinary_case {
    ($name:ident, $model:ident, |$input:ident, $states:ident| $build:block) => {
        #[derive(Debug)]
        struct $name(Inputs);
        impl TestFactor for $name {
            type Factor<R: RealField + Copy> = $model<R>;
            fn cases() -> impl Strategy<Value = Self> {
                Inputs::cases().prop_map(Self)
            }
            fn build<R: RealField + Copy>(&self, $states: &mut TestStates<R>) -> $model<R> {
                let $input = &self.0;
                $build
            }
        }
    };
}

ordinary_case!(TrajectoryCase, TrajectoryPrior, |input, states| {
    TrajectoryPrior {
        controls: input.controls(states),
        duration: c(0.25),
        tau: c(0.4),
        reference: TrajectoryState {
            pose: input.control::<R>(1).pose,
            velocity: Framed::wrap(Vector3::new(
                c(input.values[6]),
                c(input.values[7]),
                c(input.values[8]),
            )),
        },
        information_root: root(),
    }
});
ordinary_case!(IntrinsicsCase, CameraIntrinsicsPrior, |input, states| {
    let current = CameraIntrinsics {
        focal_lengths: Framed::wrap(nalgebra::Vector2::new(
            c(300.0 + input.values[0]),
            c(310.0 + input.values[1]),
        )),
        optical_center: Framed::wrap(nalgebra::Point2::new(
            c(160.0 + input.values[2]),
            c(120.0 + input.values[3]),
        )),
    };
    CameraIntrinsicsPrior {
        intrinsics: states.insert(current),
        reference: calibration(),
        information_root: root(),
    }
});
ordinary_case!(MotionCase, MotionPrior, |input, states| {
    MotionPrior {
        controls: input.controls(states),
        duration: c(0.25),
        information_root: root(),
        use_start_velocity: !input.bridge,
    }
});
ordinary_case!(YawCase, RelativeYaw, |input, states| {
    RelativeYaw {
        controls: input.controls(states),
        duration: c(0.25),
        end_tau: c(if input.stationary {
            0.0
        } else if input.bridge {
            1.0
        } else {
            0.5 + input.values[0]
        }),
        measured_yaw_change: c(input.values[8]),
        information_root: c(1.7),
    }
});
ordinary_case!(ContainmentCase, FieldContainment, |input, states| {
    FieldContainment {
        controls: input.controls(states),
        duration: c(0.25),
        tau: c(0.71),
        alignment: states.insert(FieldAlignment {
            local_to_field: Transform::wrap(Isometry2::new(
                nalgebra::Vector2::new(
                    c(if input.bridge { 5.0 } else { -5.0 }),
                    c(input.values[6]),
                ),
                c(0.7),
            )),
        }),
        half_extents: Framed::wrap(nalgebra::Vector2::new(c(3.0), c(2.0))),
        sigma: c(0.8),
    }
});

fn calibration<R: RealField + Copy>() -> CameraIntrinsics<R> {
    CameraIntrinsics {
        focal_lengths: Framed::wrap(nalgebra::Vector2::new(c(300.0), c(310.0))),
        optical_center: Framed::wrap(nalgebra::Point2::new(c(160.0), c(120.0))),
    }
}

ordinary_case!(BiasPriorCase, ImuBiasPrior, |input, states| {
    ImuBiasPrior {
        bias: states.insert(ImuBias::exp(&nalgebra::SVector::<R, 6>::from_fn(|i, _| {
            c(input.values[i])
        }))),
        reference: ImuBias::identity(),
        information_root: root(),
    }
});

ordinary_case!(PreintegratedCase, PreintegratedImu, |input, states| {
    use crate::preintegration::{ImuDelta, PreintegrationInformation};
    let biases = std::array::from_fn(|i| {
        states.insert(ImuBias {
            gyroscope: Framed::wrap(Vector3::new(c(input.values[i]), c(0.01), c(-0.02))),
            accelerometer: Framed::wrap(Vector3::new(c(0.02), c(input.values[i + 2]), c(-0.01))),
        })
    });
    PreintegratedImu {
        controls: input.controls(states),
        biases,
        duration: c(0.25),
        start_tau: c(0.2),
        end_tau: c(0.6),
        delta: ImuDelta {
            duration: c(0.1),
            rotation: Transform::wrap(UnitQuaternion::from_euler_angles(
                c(0.01),
                c(-0.02),
                c(0.03),
            )),
            velocity: Framed::wrap(Vector3::new(c(0.01), c(-0.02), c(0.98))),
            position: Framed::wrap(Vector3::new(c(0.001), c(0.002), c(0.049))),
            reference_biases: [ImuBias::identity(), ImuBias::identity()],
            bias_jacobians: std::array::from_fn(|i| {
                SMatrix::from_fn(|r, k| c((1 + i + r + 2 * k) as f64 * 0.0003))
            }),
        },
        information: if input.bridge {
            PreintegrationInformation::Rotation(root())
        } else {
            PreintegrationInformation::Full(root())
        },
        gravity_compensation: Framed::wrap(Vector3::new(c(0.0), c(0.0), c(9.81))),
        position: Framed::wrap(if input.stationary {
            Vector3::zeros()
        } else {
            Vector3::new(c(0.12), c(-0.03), c(0.08))
        }),
    }
});
fagra::factor_tests!(preintegrated_f64, PreintegratedCase, f64);
fagra::factor_tests!(preintegrated_f32, PreintegratedCase, f32);

#[test]
fn preintegration_at_offset_imu_preserves_fast_rotation_about_robot_origin() {
    use crate::preintegration::{ImuNoise, ImuPreintegrator};
    let mut states = Scene::new();
    let controls = std::array::from_fn(|i| {
        states.graph.add(PoseControl {
            pose: Framed::wrap(Isometry3::from_parts(
                Vector3::zeros().into(),
                UnitQuaternion::from_euler_angles(0.0, 0.0, 4.0 * (i as f64 - 1.0) * 0.2),
            )),
        })
    });
    let biases = std::array::from_fn(|_| states.graph.add(ImuBias::identity()));
    let position = Vector3::new(0.12, -0.03, 0.08);
    let omega = Vector3::new(0.0, 0.0, 4.0);
    let force = Vector3::new(0.0, 0.0, 9.81) + omega.cross(&omega.cross(&position));
    let mut integrated = ImuPreintegrator::new([ImuBias::identity(), ImuBias::identity()], true);
    for i in 0..50 {
        integrated
            .integrate(
                Framed::wrap(omega),
                Some(Framed::wrap(force)),
                0.002,
                (0.05 + i as f64 * 0.002 + 0.001) / 5.0,
                ImuNoise {
                    gyroscope: 2e-5,
                    accelerometer: 0.09,
                    integration: 1e-8,
                },
            )
            .unwrap();
    }
    let mut factor = PreintegratedImu {
        controls,
        biases,
        duration: 0.2,
        start_tau: 0.25,
        end_tau: 0.75,
        information: integrated.information().unwrap(),
        delta: integrated.delta,
        position: Framed::wrap(position),
        gravity_compensation: Framed::wrap(Vector3::new(0.0, 0.0, 9.81)),
    };
    assert!(factor.cost(&states).unwrap() < 1e-3);
    factor.position = Framed::wrap(Vector3::zeros());
    assert!(factor.cost(&states).unwrap() > 1.0);
}
ordinary_case!(BiasWalkCase, ImuBiasWalk, |input, states| {
    ImuBiasWalk {
        biases: [
            states.insert(ImuBias::identity()),
            states.insert(ImuBias::exp(&nalgebra::SVector::<R, 6>::from_fn(|i, _| {
                c(input.values[i])
            }))),
        ],
        information_root: root(),
    }
});
fagra::factor_tests!(bias_prior_f64, BiasPriorCase, f64);
fagra::factor_tests!(bias_prior_f32, BiasPriorCase, f32);
fagra::factor_tests!(bias_walk_f64, BiasWalkCase, f64);
fagra::factor_tests!(bias_walk_f32, BiasWalkCase, f32);

ordinary_case!(ImuCase, ImuKinematics, |input, states| {
    let tau = if input.stationary {
        0.0
    } else if input.bridge {
        1.0
    } else {
        0.4
    };
    ImuKinematics {
        controls: input.controls(states),
        biases: std::array::from_fn(|i| {
            states.insert(ImuBias {
                gyroscope: Framed::wrap(Vector3::new(c(input.values[i]), c(0.03), c(-0.02))),
                accelerometer: Framed::wrap(Vector3::zeros()),
            })
        }),
        duration: c(0.25),
        tau: c(tau),
        bias_tau: c(0.1 + 0.8 * tau),
        gyroscope_information_root: if input.bridge {
            nalgebra::Matrix3::zeros()
        } else {
            root()
        },
        tilt_information_root: root(),
        measured_up: (!input.stationary).then(|| input.measured_up()),
        angular_velocity: Framed::wrap(Vector3::new(
            c(input.values[6]),
            c(input.values[7]),
            c(input.values[8]),
        )),
    }
});

#[derive(Debug)]
struct FootCase(Inputs);
impl TestFactorBatch for FootCase {
    type Batch<R: RealField + Copy> = FootGround<R>;
    fn tolerance<R: testing::TestScalar>() -> testing::Tolerance {
        // The cost gradient contains sigma^-2 = 100 and cancellation between rows.
        let mut t = testing::Tolerance::for_scalar::<R>();
        t.absolute = t.absolute.max(1024.0 * R::EPSILON);
        t
    }
    fn cases() -> impl Strategy<Value = Self> {
        Inputs::cases().prop_map(Self)
    }
    fn build<R: RealField + Copy>(
        &self,
        states: &mut TestStates<R>,
    ) -> (FootGround<R>, Vec<FootObservation<R>>) {
        (
            FootGround {
                controls: self.0.controls(states),
                duration: c(0.25),
                sigma: c(0.1),
            },
            [0.0, 0.43, 1.0]
                .into_iter()
                .map(|tau| FootObservation {
                    tau: c(tau),
                    left_sole: Framed::wrap(nalgebra::Point3::new(c(0.1), c(0.15), c(-2.0))),
                    right_sole: Framed::wrap(nalgebra::Point3::new(c(-0.1), c(-0.15), c(1.0))),
                })
                .collect(),
        )
    }
}

#[derive(Debug)]
struct ReprojectionCase<const ROBUST: bool>(Inputs);
impl<const ROBUST: bool> TestFactorBatch for ReprojectionCase<ROBUST> {
    type Batch<R: RealField + Copy> = FrameReprojections<R>;
    fn cases() -> impl Strategy<Value = Self> {
        Inputs::cases().prop_map(Self)
    }
    fn build<R: RealField + Copy>(
        &self,
        states: &mut TestStates<R>,
    ) -> (FrameReprojections<R>, Vec<ReprojectionObservation<R>>) {
        let model = FrameReprojections {
            controls: self.0.controls(states),
            alignment: states.insert(FieldAlignment {
                local_to_field: Transform::wrap(Isometry2::new(
                    nalgebra::Vector2::new(c(0.2), c(-0.3)),
                    c(0.4),
                )),
            }),
            intrinsics: states.insert(calibration()),
            duration: c(0.25),
            tau: c(0.41),
            robot_to_camera: Transform::wrap(Isometry3::from_parts(
                Vector3::new(c(0.1), c(-0.2), c(0.3)).into(),
                UnitQuaternion::from_euler_angles(c(0.1), c(-0.2), c(0.15)),
            )),
            angular_information_root: c(3.0),
            huber_threshold: c(if ROBUST { 2.0 } else { 1e6 }),
            min_range: c(0.01),
        };
        let observations = [0.0, 0.5, -0.5]
            .into_iter()
            .map(|x| ReprojectionObservation {
                // Exercise front, side-plane and behind-camera configurations.
                field_point: Framed::wrap(nalgebra::Point3::new(
                    c(1.0 + x),
                    c(self.0.values[6]),
                    c(x * 10.0),
                )),
                detection: Framed::wrap(nalgebra::Point2::new(
                    c(if x < 0.0 { 800.0 } else { 160.0 }),
                    c(120.0),
                )),
            })
            .collect();
        (model, observations)
    }
}

#[derive(Debug)]
struct OdometryCase<const ROBUST: bool>(Inputs);
impl<const ROBUST: bool> TestFactorBatch for OdometryCase<ROBUST> {
    type Batch<R: RealField + Copy> = VisualOdometry<R>;
    fn cases() -> impl Strategy<Value = Self> {
        Inputs::cases().prop_map(Self)
    }
    fn build<R: RealField + Copy>(
        &self,
        states: &mut TestStates<R>,
    ) -> (VisualOdometry<R>, Vec<VisualOdometryObservation<R>>) {
        (
            VisualOdometry {
                controls: self.0.controls(states),
                duration: c(0.25),
                information_root: root(),
                huber_threshold: c(if ROBUST { 0.15 } else { 1e6 }),
            },
            vec![
                self.0.odometry(0.0, 0.4),
                self.0.odometry(0.3, 0.9),
                self.0.odometry(0.0, 1.0),
            ],
        )
    }
}

#[derive(Debug)]
struct AdjacentCase<const ROBUST: bool>(Inputs);
impl<const ROBUST: bool> TestFactor for AdjacentCase<ROBUST> {
    type Factor<R: RealField + Copy> = AdjacentVisualOdometry<R>;
    fn cases() -> impl Strategy<Value = Self> {
        Inputs::cases().prop_map(Self)
    }
    fn build<R: RealField + Copy>(&self, states: &mut TestStates<R>) -> AdjacentVisualOdometry<R> {
        AdjacentVisualOdometry {
            controls: self.0.controls(states),
            duration: c(0.25),
            observation: self.0.odometry(0.8, 0.2),
            information_root: root(),
            huber_threshold: c(if ROBUST { 0.15 } else { 1e6 }),
        }
    }
}

#[derive(Debug)]
struct KinematicCase<const N: usize, const ROBUST: bool>(Inputs);
impl<const N: usize, const ROBUST: bool> TestFactor for KinematicCase<N, ROBUST> {
    type Factor<R: RealField + Copy> = KinematicOdometry<R, N>;
    fn cases() -> impl Strategy<Value = Self> {
        Inputs::cases().prop_map(Self)
    }
    fn build<R: RealField + Copy>(&self, states: &mut TestStates<R>) -> Self::Factor<R> {
        KinematicOdometry {
            controls: self.0.controls(states),
            duration: c(0.2),
            previous_tau: c(0.4),
            current_tau: c(0.7),
            translation: linear_algebra::Vector2::wrap(nalgebra::Vector2::new(
                c(self.0.values[6]),
                c(self.0.values[7]),
            )),
            information_root: root(),
            huber_threshold: c(if ROBUST { 0.15 } else { 1e6 }),
        }
    }
}

fagra::factor_tests!(kinematic_f64, KinematicCase<4, false>, f64);
fagra::factor_tests!(kinematic_f32, KinematicCase<4, false>, f32);
fagra::factor_tests!(adjacent_kinematic_f64, KinematicCase<5, false>, f64);
fagra::factor_tests!(adjacent_kinematic_f32, KinematicCase<5, false>, f32);
fagra::factor_tests!(trajectory_f64, TrajectoryCase, f64);
fagra::factor_tests!(trajectory_f32, TrajectoryCase, f32);
fagra::factor_tests!(intrinsics_f64, IntrinsicsCase, f64);
fagra::factor_tests!(intrinsics_f32, IntrinsicsCase, f32);
fagra::factor_tests!(motion_f64, MotionCase, f64);
fagra::factor_tests!(motion_f32, MotionCase, f32);
fagra::factor_tests!(yaw_f64, YawCase, f64);
fagra::factor_tests!(yaw_f32, YawCase, f32);
fagra::factor_tests!(containment_f64, ContainmentCase, f64);
fagra::factor_tests!(containment_f32, ContainmentCase, f32);
fagra::factor_tests!(adjacent_raw_f64, AdjacentCase<false>, f64);
fagra::factor_tests!(adjacent_raw_f32, AdjacentCase<false>, f32);
fagra::factor_tests!(imu_f64, ImuCase, f64);
fagra::factor_tests!(imu_f32, ImuCase, f32);
fagra::factor_batch_tests!(foot_f64, FootCase, f64);
fagra::factor_batch_tests!(foot_f32, FootCase, f32);
fagra::factor_batch_tests!(reprojection_raw_f64, ReprojectionCase<false>, f64);
fagra::factor_batch_tests!(reprojection_raw_f32, ReprojectionCase<false>, f32);
fagra::factor_batch_tests!(odometry_raw_f64, OdometryCase<false>, f64);
fagra::factor_batch_tests!(odometry_raw_f32, OdometryCase<false>, f32);

#[test]
fn robust_local_models() {
    testing::check_factor::<KinematicCase<4, true>, f64>(FactorProperty::LocalModel);
    testing::check_factor::<KinematicCase<5, true>, f64>(FactorProperty::LocalModel);
    testing::check_factor_batch::<ReprojectionCase<true>, f64>(FactorProperty::LocalModel);
    testing::check_factor_batch::<ReprojectionCase<true>, f32>(FactorProperty::LocalModel);
    testing::check_factor_batch::<OdometryCase<true>, f64>(FactorProperty::LocalModel);
    testing::check_factor_batch::<OdometryCase<true>, f32>(FactorProperty::LocalModel);
    testing::check_factor::<AdjacentCase<true>, f64>(FactorProperty::LocalModel);
    testing::check_factor::<AdjacentCase<true>, f32>(FactorProperty::LocalModel);
}

#[test]
fn huber_cost_and_frozen_weight_reference() {
    for (r, expected_cost, expected_scale) in [
        (nalgebra::Vector2::<f64>::zeros(), 0.0, 1.0),
        (nalgebra::Vector2::new(1.0, 0.0), 0.5, 1.0),
        (nalgebra::Vector2::new(2.0, 0.0), 2.0, 1.0),
        (
            nalgebra::Vector2::new(3.0, 4.0),
            8.0,
            (2.0_f64 / 5.0).sqrt(),
        ),
    ] {
        let (cost, scale) = common::huber(&r, 2.0).unwrap();
        assert!((cost - expected_cost).abs() < 1e-12);
        assert!((scale - expected_scale).abs() < 1e-12);
    }
}

fagra::states! { States { poses: PoseControl, alignments: FieldAlignment, intrinsics: CameraIntrinsics, biases: ImuBias } }
fagra::factors! { Factors {
    trajectory: TrajectoryPrior, calibration: CameraIntrinsicsPrior, motion: MotionPrior,
    yaw: RelativeYaw, containment: FieldContainment,
    imu: ImuKinematics, feet: Batch<FootGround, FootObservation>,
    pixels: Batch<FrameReprojections, ReprojectionObservation>,
    odometry: Batch<VisualOdometry, VisualOdometryObservation>, adjacent: AdjacentVisualOdometry,
} }

struct Scene {
    graph: fagra::Solver<States, Factors>,
    controls: [StateKey<PoseControl>; 5],
    alignment: StateKey<FieldAlignment>,
    intrinsics: StateKey<CameraIntrinsics>,
}

impl Scene {
    fn new() -> Self {
        let mut graph = fagra::Solver::new();
        let controls = std::array::from_fn(|_| graph.add(PoseControl::identity()));
        let alignment = graph.add(FieldAlignment::identity());
        let intrinsics = graph.add(calibration());
        Self {
            graph,
            controls,
            alignment,
            intrinsics,
        }
    }
    fn segment(&self) -> [StateKey<PoseControl>; 4] {
        [
            self.controls[0],
            self.controls[1],
            self.controls[2],
            self.controls[3],
        ]
    }
    fn tilt(&mut self) -> ImuKinematics {
        ImuKinematics {
            controls: self.segment(),
            biases: std::array::from_fn(|_| self.graph.add(ImuBias::identity())),
            duration: 0.25,
            tau: 0.5,
            bias_tau: 0.5,
            angular_velocity: Framed::wrap(Vector3::zeros()),
            measured_up: Some(Framed::wrap(Vector3::z())),
            gyroscope_information_root: nalgebra::Matrix3::zeros(),
            tilt_information_root: nalgebra::Matrix3::identity(),
        }
    }
    fn reprojections(&self) -> FrameReprojections {
        FrameReprojections {
            controls: self.segment(),
            duration: 0.25,
            tau: 0.5,
            alignment: self.alignment,
            intrinsics: self.intrinsics,
            robot_to_camera: Transform::wrap(Isometry3::identity()),
            angular_information_root: 300.0,
            huber_threshold: 2.0,
            min_range: 0.01,
        }
    }
}

macro_rules! store {
    ($ty:ty) => {
        impl StateStore<$ty> for Scene {
            fn get(&self, key: StateKey<$ty>) -> Result<&$ty, fagra::KeyError> {
                self.graph.get(key)
            }
        }
    };
}
store!(PoseControl);
store!(FieldAlignment);
store!(CameraIntrinsics);
store!(ImuBias);

#[derive(Default)]
struct Capture {
    residuals: Vec<nalgebra::DVector<f64>>,
    blocks: Vec<Vec<(BlockId, nalgebra::DMatrix<f64>)>>,
}
impl LinearizationSink for Capture {
    type Scalar = f64;
    fn factor(
        &mut self,
        _: FactorId,
        emit: impl FnOnce(&mut Self) -> Result<(), EvaluationError>,
    ) -> Result<(), EvaluationError> {
        emit(self)
    }
    fn residual<Rows: DimName, S>(
        &mut self,
        r: &nalgebra::Matrix<f64, Rows, nalgebra::U1, S>,
        blocks: &[JacobianBlock<'_, f64>],
    ) -> Result<(), EvaluationError>
    where
        S: nalgebra::Storage<f64, Rows, nalgebra::U1> + nalgebra::storage::IsContiguous,
    {
        assert!(r.iter().all(|x| x.is_finite()));
        self.residuals
            .push(nalgebra::DVector::from_column_slice(r.as_slice()));
        let mut captured = Vec::new();
        for block in blocks {
            assert!(block.jacobian().iter().all(|x| x.is_finite()));
            assert!(!captured.iter().any(|(key, _)| *key == block.variable()));
            captured.push((block.variable(), block.jacobian().clone_owned()));
        }
        self.blocks.push(captured);
        Ok(())
    }
}

#[test]
fn shared_imu_tilt_has_the_same_objective_as_separate_factors() {
    let mut scene = Scene::new();
    let input = Inputs {
        values: [0.1; 9],
        bridge: false,
        stationary: false,
    };
    for (i, key) in scene.controls.iter().enumerate() {
        scene.graph.set(*key, input.control(i)).unwrap();
    }
    let controls = scene.segment();
    let biases = std::array::from_fn(|_| scene.graph.add(ImuBias::identity()));
    let root = root::<f64, 3>();
    let up = input.measured_up();
    let mut combined = ImuKinematics {
        controls,
        biases,
        duration: 0.25,
        gyroscope_information_root: root,
        tilt_information_root: root,
        tau: 0.4,
        bias_tau: 0.3,
        angular_velocity: Framed::wrap(Vector3::new(0.1, -0.2, 0.3)),
        measured_up: Some(up),
    };
    let with_tilt = combined.cost(&scene).unwrap();
    combined.measured_up = None;
    let tilt = ImuKinematics {
        gyroscope_information_root: nalgebra::Matrix3::zeros(),
        measured_up: Some(up),
        ..combined.clone()
    };
    let separate = combined.cost(&scene).unwrap() + tilt.cost(&scene).unwrap();
    assert!((with_tilt - separate).abs() < 1e-10);
    combined.gyroscope_information_root.fill(0.0);
    combined.measured_up = Some(up);
    assert!((combined.cost(&scene).unwrap() - tilt.cost(&scene).unwrap()).abs() < 1e-10);
}

#[test]
fn exact_static_measurements_and_one_sided_constraints() {
    let mut scene = Scene::new();
    let controls = scene.segment();
    let trajectory = TrajectoryPrior {
        controls,
        duration: 0.25,
        tau: 0.5,
        reference: TrajectoryState::identity(),
        information_root: SMatrix::identity(),
    };
    assert_eq!(trajectory.cost(&scene).unwrap(), 0.0);
    trajectory
        .linearize(&scene, &mut Capture::default())
        .unwrap();
    let tilt = scene.tilt();
    assert_eq!(tilt.cost(&scene).unwrap(), 0.0);
    tilt.linearize(&scene, &mut Capture::default()).unwrap();
    let current = RelativeYaw {
        controls,
        duration: 0.25,
        end_tau: 0.5,
        measured_yaw_change: 0.0,
        information_root: 1.0,
    };
    assert_eq!(current.cost(&scene).unwrap(), 0.0);
    current.linearize(&scene, &mut Capture::default()).unwrap();
    let motion = MotionPrior {
        controls,
        duration: 0.25,
        information_root: SMatrix::identity(),
        use_start_velocity: true,
    };
    assert_eq!(motion.cost(&scene).unwrap(), 0.0);
    motion.linearize(&scene, &mut Capture::default()).unwrap();

    let biases = std::array::from_fn(|_| scene.graph.add(ImuBias::identity()));
    let imu = ImuKinematics {
        controls,
        biases,
        duration: 0.25,
        gyroscope_information_root: nalgebra::Matrix3::identity(),
        tilt_information_root: nalgebra::Matrix3::identity(),
        tau: 0.5,
        bias_tau: 0.4,
        measured_up: None,
        angular_velocity: Framed::wrap(Vector3::zeros()),
    };
    assert_eq!(imu.cost(&scene).unwrap(), 0.0);
    imu.linearize(&scene, &mut Capture::default()).unwrap();

    let feet = scene.graph.add_batch(FootGround {
        controls,
        duration: 0.25,
        sigma: 0.1,
    });
    let foot = scene
        .graph
        .add_factor_to(
            feet,
            FootObservation {
                tau: 0.5,
                left_sole: Framed::wrap(nalgebra::Point3::new(0.0, 0.0, -0.2)),
                right_sole: Framed::wrap(nalgebra::Point3::new(0.0, 0.0, 0.2)),
            },
        )
        .unwrap();
    assert!((scene.graph.factor_cost(foot).unwrap() - 2.0).abs() < 1e-12);
    // Contact and the inactive branch both give zero, without constraining a lifted foot.
    let contact = scene
        .graph
        .add_factor_to(
            feet,
            FootObservation {
                tau: 0.5,
                left_sole: Framed::wrap(nalgebra::Point3::origin()),
                right_sole: Framed::wrap(nalgebra::Point3::new(0.0, 0.0, 1.0)),
            },
        )
        .unwrap();
    assert_eq!(scene.graph.factor_cost(contact).unwrap(), 0.0);

    let containment = FieldContainment {
        controls,
        duration: 0.25,
        tau: 0.5,
        alignment: scene.alignment,
        half_extents: Framed::wrap(nalgebra::Vector2::new(3.0, 2.0)),
        sigma: 1.0,
    };
    for (x, expected) in [(0.0, 0.0), (3.0, 0.0), (-3.0, 0.0), (5.0, 2.0), (-5.0, 2.0)] {
        scene
            .graph
            .set(
                scene.alignment,
                FieldAlignment {
                    local_to_field: Transform::wrap(Isometry2::new(
                        nalgebra::Vector2::new(x, 0.0),
                        0.7,
                    )),
                },
            )
            .unwrap();
        assert_eq!(containment.cost(&scene).unwrap(), expected);
        let mut capture = Capture::default();
        containment.linearize(&scene, &mut capture).unwrap();
        if expected == 0.0 {
            assert!(
                capture
                    .blocks
                    .iter()
                    .flatten()
                    .all(|(_, j)| j.norm() == 0.0)
            );
        }
    }
}

#[test]
fn tilt_distinguishes_inversion_without_observing_yaw() {
    let mut scene = Scene::new();
    let measured = UnitQuaternion::from_euler_angles(0.0, 0.03, 0.0);
    let factor = ImuKinematics {
        measured_up: Some(Framed::wrap(measured.inverse() * Vector3::z())),
        ..scene.tilt()
    };
    for yaw in [0.0, 1.7] {
        for key in scene.controls {
            scene
                .graph
                .set(
                    key,
                    PoseControl {
                        pose: Framed::wrap(Isometry3::from_parts(
                            nalgebra::Translation3::identity(),
                            UnitQuaternion::from_euler_angles(0.0, 0.03, yaw),
                        )),
                    },
                )
                .unwrap();
        }
        assert!(factor.cost(&scene).unwrap() < 1e-20);
    }
    for key in scene.controls {
        scene
            .graph
            .set(
                key,
                PoseControl {
                    pose: Framed::wrap(Isometry3::from_parts(
                        nalgebra::Translation3::identity(),
                        UnitQuaternion::from_euler_angles(std::f64::consts::PI, 0.03, 0.0),
                    )),
                },
            )
            .unwrap();
    }
    assert!(factor.cost(&scene).unwrap() > 1.9);
}

#[test]
fn relative_yaw_uses_the_observation_endpoint() {
    let mut scene = Scene::new();
    for (i, key) in scene.controls.iter().take(4).enumerate() {
        scene
            .graph
            .set(
                *key,
                PoseControl {
                    pose: Framed::wrap(Isometry3::new(
                        Vector3::zeros(),
                        Vector3::new(0.0, 0.0, 0.4 + i as f64 * 0.2),
                    )),
                },
            )
            .unwrap();
    }
    let mut factor = RelativeYaw {
        controls: scene.segment(),
        duration: 0.25,
        end_tau: 0.25,
        measured_yaw_change: 0.05,
        information_root: 1.0,
    };
    for tau in [0.0, 0.25, 1.0] {
        factor.end_tau = tau;
        factor.measured_yaw_change = tau * 0.2;
        assert!(factor.cost(&scene).unwrap() < 1.0e-25);
        factor.linearize(&scene, &mut Capture::default()).unwrap();
    }
    for tau in [-0.01, 1.01, f64::NAN] {
        factor.end_tau = tau;
        assert!(factor.cost(&scene).is_err());
        assert!(factor.linearize(&scene, &mut Capture::default()).is_err());
    }
}

#[test]
fn bearing_geometry_range_and_robust_cost() {
    let mut scene = Scene::new();
    let reprojections = scene.reprojections();
    let batch = scene.graph.add_batch(reprojections);
    let cosine = 1.0 / (1.0_f64 + (3.0_f64 / 300.0).powi(2)).sqrt();
    let raw_cost = 3.0 * 300.0_f64.powi(2) * (1.0 - cosine) / (2.0 + cosine);
    let robust_cost = 2.0 * ((2.0 * raw_cost).sqrt() - 1.0);
    for (x, detection_x, expected) in [
        (0.0, 160.0, 0.0),
        (1.0, 220.0, 0.0),
        (0.0, 157.0, robust_cost),
    ] {
        let key = scene
            .graph
            .add_factor_to(
                batch,
                ReprojectionObservation {
                    field_point: Framed::wrap(nalgebra::Point3::new(x, 0.0, 5.0)),
                    detection: Framed::wrap(nalgebra::Point2::new(detection_x, 120.0)),
                },
            )
            .unwrap();
        assert!((scene.graph.factor_cost(key).unwrap() - expected).abs() < 1e-9);
    }
    for z in [0.0, 0.01, -0.01, f64::NAN, f64::INFINITY] {
        let key = scene
            .graph
            .add_factor_to(
                batch,
                ReprojectionObservation {
                    field_point: Framed::wrap(nalgebra::Point3::new(0.0, 0.0, z)),
                    detection: Framed::wrap(nalgebra::Point2::new(160.0, 120.0)),
                },
            )
            .unwrap();
        assert!(scene.graph.factor_cost(key).is_err());
        scene.graph.remove_factor(key).unwrap();
    }
    let key = scene
        .graph
        .add_factor_to(
            batch,
            ReprojectionObservation {
                field_point: Framed::wrap(nalgebra::Point3::new(0.0, 0.0, 5.0)),
                detection: Framed::wrap(nalgebra::Point2::new(160.0, 120.0)),
            },
        )
        .unwrap();
    let mut invalid = calibration();
    invalid.focal_lengths.inner.x = 0.0;
    scene.graph.set(scene.intrinsics, invalid).unwrap();
    assert!(scene.graph.factor_cost(key).is_err());
}

#[test]
fn bearing_cost_is_monotone_through_side_plane_and_behind_camera() {
    for huber_threshold in [2.0, 1e6] {
        let mut scene = Scene::new();
        let mut factor = scene.reprojections();
        factor.huber_threshold = huber_threshold;
        let batch = scene.graph.add_batch(factor);
        let mut previous = -1.0;
        for index in 0..=360 {
            let theta = index as f64 * std::f64::consts::PI / 360.0;
            let key = scene
                .graph
                .add_factor_to(
                    batch,
                    ReprojectionObservation {
                        field_point: Framed::wrap(nalgebra::Point3::new(
                            5.0 * theta.sin(),
                            0.0,
                            5.0 * theta.cos(),
                        )),
                        detection: Framed::wrap(nalgebra::Point2::new(160.0, 120.0)),
                    },
                )
                .unwrap();
            let cost = scene.graph.factor_cost(key).unwrap();
            let raw = 3.0 * 300.0_f64.powi(2) * (1.0 - theta.cos()) / (2.0 + theta.cos());
            let norm = (2.0 * raw).sqrt();
            let expected = if norm <= huber_threshold {
                raw
            } else {
                huber_threshold * (norm - 0.5 * huber_threshold)
            };
            assert!(
                (cost - expected).abs() < 1e-8,
                "angle={theta}, cost={cost}, expected={expected}"
            );
            assert!(
                cost > previous,
                "angle={theta}, previous={previous}, cost={cost}"
            );
            previous = cost;
            scene.graph.remove_factor(key).unwrap();
        }
    }
}

#[test]
fn odometry_irls_curvature_and_shared_control_blocks() {
    let scene = Scene::new();
    let mut model = AdjacentVisualOdometry {
        controls: scene.controls,
        duration: 0.25,
        observation: VisualOdometryObservation {
            previous_tau: 0.8,
            current_tau: 0.2,
            current_to_previous: Transform::wrap(Isometry3::translation(3.0, 4.0, 0.0)),
        },
        information_root: SMatrix::identity(),
        huber_threshold: 1e6,
    };
    let mut raw = Capture::default();
    model.linearize(&scene, &mut raw).unwrap();
    assert_eq!(
        raw.residuals[0].as_slice(),
        &[0.0, 0.0, 0.0, -3.0, -4.0, 0.0]
    );
    assert_eq!(model.cost(&scene).unwrap(), 12.5);
    model.huber_threshold = 2.0;
    let mut robust = Capture::default();
    model.linearize(&scene, &mut robust).unwrap();
    assert_eq!(model.cost(&scene).unwrap(), 8.0);
    assert_eq!(robust.blocks[0].len(), 5);
    let scale = (2.0_f64 / 5.0).sqrt();
    assert!((&robust.residuals[0] - &raw.residuals[0] * scale).norm() < 1e-12);
    for ((key, j), (raw_key, raw_j)) in robust.blocks[0].iter().zip(&raw.blocks[0]) {
        assert_eq!(key, raw_key);
        assert!((j - raw_j * scale).norm() < 1e-12);
    }
}

#[test]
fn kinematic_odometry_rejects_unsupported_control_counts() {
    fn check<const N: usize>() {
        let mut scene = Scene::new();
        let factor = KinematicOdometry {
            controls: std::array::from_fn::<_, N, _>(|_| scene.graph.add(PoseControl::identity())),
            duration: 0.2,
            previous_tau: 0.4,
            current_tau: 0.7,
            translation: linear_algebra::Vector2::zeros(),
            information_root: nalgebra::Matrix2::identity(),
            huber_threshold: 1.0,
        };
        assert!(matches!(
            factor.cost(&scene),
            Err(fagra::EvaluationError::InvalidEvaluation)
        ));
        assert!(matches!(
            factor.linearize(&scene, &mut Capture::default()),
            Err(fagra::EvaluationError::InvalidEvaluation)
        ));
    }
    check::<0>();
    check::<3>();
    check::<6>();
}

#[test]
fn invalid_inputs_and_yaw_wrapping() {
    let mut scene = Scene::new();
    let mut prior = scene.tilt();
    prior.controls[1] = prior.controls[0];
    assert!(prior.cost(&scene).is_err());
    assert!(prior.linearize(&scene, &mut Capture::default()).is_err());
    prior.controls = scene.segment();
    for z in [2.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        prior.measured_up = Some(Framed::wrap(Vector3::new(0.0, 0.0, z)));
        assert!(prior.cost(&scene).is_err());
        assert!(prior.linearize(&scene, &mut Capture::default()).is_err());
    }
    let start = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), std::f64::consts::PI - 0.1);
    let end = UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -std::f64::consts::PI + 0.1);
    assert!(common::yaw_error(&start, &end, 0.2).unwrap().abs() < 1e-12);
    let vertical = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), std::f64::consts::FRAC_PI_2);
    assert!(common::heading(&vertical).is_err());
    assert!(common::heading_jacobian(&vertical).is_err());
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(common::huber(&nalgebra::Vector2::new(1.0, 0.0), bad).is_err());
    }
    prior.measured_up = Some(Framed::wrap(Vector3::z()));
    let mut seam = PoseControl::identity();
    seam.pose.inner.rotation =
        UnitQuaternion::from_axis_angle(&Vector3::z_axis(), std::f64::consts::PI - 1e-7);
    scene.graph.set(prior.controls[3], seam).unwrap();
    assert!(prior.cost(&scene).is_err());
    assert!(prior.linearize(&scene, &mut Capture::default()).is_err());
}
