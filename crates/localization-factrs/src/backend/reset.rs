use std::time::SystemTime;

use crate::{
    initial_state::InitialState, measurements::GlobalPoseMeasurement, symbols::CameraIntrinsics,
};

use super::{
    VinsBackend,
    graph::{initialize_graph, optimizer_from_graph},
    interval_assigner::IntervalAssigner,
    state::zero_velocity_pose,
};

impl VinsBackend {
    pub(super) fn reset_to_global_pose(&mut self, global_pose: GlobalPoseMeasurement) {
        let camera_intrinsics = self
            .values
            .get(CameraIntrinsics(0))
            .cloned()
            .unwrap_or_else(|| self.initial_state.camera_intrinsics.clone());
        self.initial_state = InitialState::new(
            zero_velocity_pose(&global_pose.robot_to_field),
            camera_intrinsics,
        );

        self.reset_to_initial_state(self.initial_state.clone(), global_pose.time);
    }

    pub(super) fn reset_to_initial_state(&mut self, initial_state: InitialState, time: SystemTime) {
        self.initial_state = initial_state;

        let (graph, values) = initialize_graph(&self.initial_state, &self.config);
        self.optimizer = optimizer_from_graph(&self.config, graph);
        self.values = values;
        self.interval_assigner = IntervalAssigner::new(self.config.knot_spacing);
        let _ = self.interval_assigner.assign_or_initialize_interval(time);
        self.last_knot_time = Some(time);
        self.highest_initialized_interval = None;
        self.last_imu_attitude_measurement = None;
        self.latest_imu_attitude_measurement = None;
        self.next_imu_attitude_knot_index = 0;
        self.last_imu_knot_orientation = None;
        self.last_optimizer_status = None;
        self.last_imu_kinematics_measurement_time = None;
        self.init_intervals_through(0);
    }
}
