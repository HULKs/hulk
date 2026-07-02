use std::time::{Duration, Instant, SystemTime};

use booster::ImuState;
use factrs::{
    core::SO3,
    traits::Variable,
    variables::{MatrixLieGroup, SE23},
};
use linear_algebra::IntoFramed;
use localization_factrs::{
    BackendConfiguration, FieldContainmentConfiguration, InitialState, SE23Spline, initialize, tau,
};
use log::LevelFilter;
use nalgebra::{Matrix2, Matrix3, SMatrix, Vector3, vector};

#[test]
fn imu_on_spline() {
    env_logger::builder().filter_level(LevelFilter::Off).init();
    log::info!("Running test");

    let (mut frontend, mut backend) = initialize(
        BackendConfiguration {
            knot_spacing: Duration::from_millis(200),
            max_optimization_window: Duration::from_secs(2),
            optimizer_max_iterations: 1,
            gyroscope_noise: Matrix3::identity() * 0.1_f64.powi(2),
            accelerometer_noise: Matrix3::identity() * 2.0_f64.powi(2),
            use_accelerometer_measurements: false,
            gyroscope_process_noise: Matrix3::identity() * 0.01,
            accelerometer_process_noise: Matrix3::identity() * 0.1,
            roll_pitch_yaw_noise: Matrix3::identity() * 0.01,
            visual_feature_noise: Matrix2::identity() * 5.0,
            pose_hint_visual_feature_noise: Matrix2::identity() * 100.0,
            pose_hint_visual_huber_threshold: 2.0,
            visual_odometry_noise: SMatrix::<f64, 6, 6>::identity() * 0.05,
            foot_ground_sigma: 0.01,
            field_containment: FieldContainmentConfiguration::default(),
            gravity: Vector3::new(0.0, 0.0, 9.81),
        },
        InitialState::default(),
    );

    let start_pose = SE23::from_rot_vel_trans(SO3::identity(), Vector3::zeros(), Vector3::zeros());
    let end_pose = SE23::from_rot_vel_trans(
        SO3::identity(),
        vector![1.0, 0.0, 0.0],
        vector![2.0, 0.0, 0.0],
    );

    let start = SystemTime::UNIX_EPOCH;
    let duration = Duration::from_secs(5);
    let ground_truth_spline = SE23Spline::new(start_pose, end_pose, duration.as_secs_f64());

    let imu_dt = Duration::from_millis(2);
    let n_samples = duration.div_duration_f32(imu_dt).floor() as u32;

    for i in 1..n_samples {
        let time = start + imu_dt * i;
        let tau = tau(start, start + duration, time);
        let pose = ground_truth_spline.evaluate(tau);
        let kinematics = ground_truth_spline.evaluate_derivative(tau);

        let linear_acceleration_local = pose.rot().inverse().apply(
            (kinematics.linear_acceleration_global - nalgebra::vector![0.0, 0.0, -9.81]).as_view(),
        );

        let imu = ImuState {
            angular_velocity: kinematics.angular_velocity_local.cast::<f32>().framed(),
            linear_acceleration: linear_acceleration_local.cast::<f32>().framed(),
            roll_pitch_yaw: Vector3::zeros().framed(),
        };

        frontend
            .ingest_imu(time, imu)
            .expect("failed to ingest imu");

        if i.is_multiple_of(n_samples / 50) {
            let _ = backend.solve_once().expect("failed to solve");
        }
    }
    let now = Instant::now();
    let _ = backend.solve_once().expect("failed to solve");
    let solve_duration = now.elapsed();
    println!("Optimization took {}ms", solve_duration.as_millis());
    let result = frontend.last_optimization_result().expect("no result");
    dbg!(result);
}
