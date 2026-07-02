use std::time::SystemTime;

use crate::{
    factors::{
        foot_above_ground::FootHeightMeasurement, visual_odometry::VisualOdometryMeasurement,
    },
    measurements::{
        GlobalPoseMeasurement, ImuMeasurement, ResetMeasurement, SensorMeasurement,
        VisualReprojectionMeasurement,
    },
};

pub struct IntervalMeasurements {
    pub resets: Vec<ResetMeasurement>,
    pub imu: Vec<ImuMeasurement>,
    pub global_poses: Vec<GlobalPoseMeasurement>,
    pub visual: Vec<Vec<VisualReprojectionMeasurement>>,
    pub pose_hint_visual: Vec<Vec<VisualReprojectionMeasurement>>,
    pub visual_odometry: Vec<VisualOdometryMeasurement>,
    pub foot_heights: Vec<FootHeightMeasurement>,
}

impl IntervalMeasurements {
    pub fn new() -> Self {
        Self {
            resets: Vec::new(),
            imu: Vec::new(),
            global_poses: Vec::new(),
            visual: Vec::new(),
            pose_hint_visual: Vec::new(),
            visual_odometry: Vec::new(),
            foot_heights: Vec::new(),
        }
    }

    pub fn push_imu(&mut self, imu: ImuMeasurement) {
        insert_sorted(&mut self.imu, imu, |imu| imu.time);
    }

    pub fn push(&mut self, measurement: SensorMeasurement) {
        match measurement {
            SensorMeasurement::Reset(reset) => self.push_reset(reset),
            SensorMeasurement::Imu(imu) => self.push_imu(imu),
            SensorMeasurement::GlobalPose(global_pose) => self.push_global_pose(global_pose),
            SensorMeasurement::Visual(visual) => self.push_visual(visual),
            SensorMeasurement::PoseHintVisual(visual) => self.push_pose_hint_visual(visual),
            SensorMeasurement::VisualOdometry(visual_odometry) => {
                self.push_visual_odometry(visual_odometry)
            }
            SensorMeasurement::FootHeights(foot_heights) => self.push_foot_heights(foot_heights),
        }
    }

    pub fn push_visual(&mut self, visual: Vec<VisualReprojectionMeasurement>) {
        insert_visual_frame(&mut self.visual, visual);
    }

    pub fn push_reset(&mut self, reset: ResetMeasurement) {
        insert_sorted(&mut self.resets, reset, |measurement| measurement.time);
    }

    pub fn push_global_pose(&mut self, global_pose: GlobalPoseMeasurement) {
        insert_sorted(&mut self.global_poses, global_pose, |measurement| {
            measurement.time
        });
    }

    pub fn push_pose_hint_visual(&mut self, visual: Vec<VisualReprojectionMeasurement>) {
        insert_visual_frame(&mut self.pose_hint_visual, visual);
    }

    pub fn push_visual_odometry(&mut self, visual_odometry: VisualOdometryMeasurement) {
        insert_sorted(&mut self.visual_odometry, visual_odometry, |measurement| {
            measurement.current_time
        });
    }

    pub fn push_foot_heights(&mut self, foot_heights: FootHeightMeasurement) {
        insert_sorted(&mut self.foot_heights, foot_heights, |measurement| {
            measurement.time
        });
    }

    pub fn latest_global_pose(&self) -> Option<&GlobalPoseMeasurement> {
        self.global_poses.last()
    }

    pub fn latest_reset(&self) -> Option<&ResetMeasurement> {
        self.resets.last()
    }

    pub fn retain_at_or_after(&mut self, time: SystemTime) {
        self.resets.retain(|measurement| measurement.time >= time);
        self.global_poses
            .retain(|measurement| measurement.time >= time);
        self.imu.retain(|measurement| measurement.time >= time);
        self.visual
            .retain(|frame| visual_frame_time(frame).is_some_and(|frame_time| frame_time >= time));
        self.pose_hint_visual
            .retain(|frame| visual_frame_time(frame).is_some_and(|frame_time| frame_time >= time));
        self.visual_odometry.retain(|measurement| {
            measurement.previous_time >= time && measurement.current_time >= time
        });
        self.foot_heights
            .retain(|measurement| measurement.time >= time);
    }
}

fn insert_visual_frame(
    frames: &mut Vec<Vec<VisualReprojectionMeasurement>>,
    visual: Vec<VisualReprojectionMeasurement>,
) {
    insert_sorted(frames, visual, |visual| {
        visual_frame_time(visual).expect("visual frames must contain at least one measurement")
    });
}

fn visual_frame_time(visual: &[VisualReprojectionMeasurement]) -> Option<SystemTime> {
    Some(visual.first()?.time)
}

fn insert_sorted<T, K: Ord>(vec: &mut Vec<T>, item: T, key: impl Fn(&T) -> K) {
    let index = vec
        .binary_search_by_key(&key(&item), key)
        .unwrap_or_else(|index| index);
    vec.insert(index, item);
}
