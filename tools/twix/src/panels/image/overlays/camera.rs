use linear_algebra::Isometry3;
use projection::{camera_matrix::CameraMatrix, horizon::Horizon};
use ros_z::time::Time;
use ros_z_debug::{SampleRecord, query::TemporalMatch};
use std::sync::Arc;
use types::time_wrapper::TimeWrapper;

type CameraRecord = Arc<SampleRecord<TimeWrapper<CameraMatrix>>>;

pub(super) struct CameraSample {
    pub horizon: Option<Horizon>,
}

pub(super) fn interpolation_endpoints<T>(
    samples: &TemporalMatch<Arc<SampleRecord<TimeWrapper<T>>>>,
    time: Time,
) -> (&TimeWrapper<T>, &TimeWrapper<T>, f32) {
    match samples {
        TemporalMatch::Exact(record) | TemporalMatch::Before(record) => {
            (&record.value, &record.value, 0.0)
        }
        TemporalMatch::Bracket { before, after } => {
            let fraction = time
                .duration_since(before.effective_time)
                .div_duration_f32(after.effective_time.duration_since(before.effective_time));
            (&before.value, &after.value, fraction)
        }
    }
}

pub(super) fn interpolate_transform<From, To>(
    a: Isometry3<From, To>,
    b: Isometry3<From, To>,
    t: f32,
) -> Isometry3<From, To> {
    Isometry3::wrap(a.inner.lerp_slerp(&b.inner, t))
}

pub(super) fn projected_camera_at(
    samples: &TemporalMatch<CameraRecord>,
    time: Time,
) -> Option<CameraSample> {
    let (a, b, fraction) = interpolation_endpoints(samples, time);
    let (a, b) = (&a.inner, &b.inner);
    if a.intrinsics != b.intrinsics || a.image_size != b.image_size {
        return None;
    }
    let ground_to_robot = interpolate_transform(a.ground_to_robot, b.ground_to_robot, fraction);
    let robot_to_head = interpolate_transform(a.robot_to_head, b.robot_to_head, fraction);
    let head_to_camera = interpolate_transform(a.head_to_camera, b.head_to_camera, fraction);
    let robot_to_camera = head_to_camera * robot_to_head;
    Some(CameraSample {
        horizon: Horizon::from_parameters(robot_to_camera * ground_to_robot, &a.intrinsics),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::image::tests::raw_sample;
    use ros_z_debug::query::decode_record;

    fn record(value: CameraMatrix, nanos: i64) -> CameraRecord {
        let value = TimeWrapper {
            time: Time::from_nanos(100),
            inner: value,
        };
        let sample = raw_sample(
            "/camera",
            nanos as u64,
            nanos,
            value.time.as_nanos(),
            &value,
        );
        decode_record(&sample.record, &sample.topic).unwrap()
    }

    #[test]
    fn camera_interpolation_preserves_endpoints_and_requires_matching_calibration() {
        let camera = |x| CameraMatrix {
            robot_to_head: Isometry3::from_translation(x, 0.0, 0.0),
            ..Default::default()
        };
        let before = record(camera(2.0), 10);
        let after = record(camera(6.0), 30);
        assert!(
            projected_camera_at(&TemporalMatch::Exact(before.clone()), Time::from_nanos(10))
                .is_some()
        );
        let bracket = TemporalMatch::Bracket { before, after };
        for (nanos, expected) in [(10, 2.0), (20, 4.0), (30, 6.0)] {
            let (a, b, fraction) = interpolation_endpoints(&bracket, Time::from_nanos(nanos));
            let transform =
                interpolate_transform(a.inner.robot_to_head, b.inner.robot_to_head, fraction);
            assert_eq!(transform.inner.translation.x, expected);
            assert!(projected_camera_at(&bracket, Time::from_nanos(nanos)).is_some());
        }
        let mut mismatched = camera(6.0);
        mismatched.intrinsics.focals.x = 2.0;
        let bracket = TemporalMatch::Bracket {
            before: record(camera(2.0), 10),
            after: record(mismatched, 30),
        };
        assert!(projected_camera_at(&bracket, Time::from_nanos(20)).is_none());
    }
}
