use std::{sync::Arc, time::Duration};

use ros_z::{
    Message,
    dynamic::{CdrSelectionError, DynamicCdrCodec, SelectionError},
    message::WireEncoder,
    time::Time,
};
use ros_z_debug::query::{ProjectionError, TimeEncoding, TimeProjection};
use ros2::builtin_interfaces::time::Time as RosTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
struct Stamped {
    time: Time,
    value: u32,
}

#[derive(Serialize, Deserialize, Message)]
struct RosStamped {
    prefix: u32,
    time: RosTime,
    trailing: Vec<u8>,
}

fn project<T: Message>(value: &T, encoding: TimeEncoding) -> Result<Time, ProjectionError> {
    let schema = Arc::new(T::schema());
    let bytes = T::Codec::serialize(value).unwrap();
    let projection = TimeProjection::field("", encoding).unwrap();
    projection.validate(&schema)?;
    projection.project_cdr(Time::zero(), &bytes, &schema)
}

#[test]
fn projection_preserves_integer_precision_and_reads_only_required_cdr_fields() {
    let time = Time::from_nanos(1_790_000_000_123_456_789);
    let bytes = <Stamped as Message>::Codec::serialize(&Stamped { time, value: 7 }).unwrap();
    let schema = Arc::new(Stamped::schema());
    let projection = TimeProjection::field(".time", TimeEncoding::RosZTime).unwrap();
    assert_eq!(
        projection
            .project_cdr(Time::zero(), &bytes, &schema)
            .unwrap(),
        time
    );
    // The timestamp prefix is readable without materializing or reading the tail.
    assert_eq!(
        projection
            .project_cdr(Time::zero(), &bytes[..bytes.len() - 4], &schema)
            .unwrap(),
        time
    );
    assert!(DynamicCdrCodec::decode(&bytes[..bytes.len() - 4], &schema).is_err());
}

#[test]
fn ros_time_projection_uses_the_concrete_leaf_and_checks_its_values() {
    let value = RosStamped {
        prefix: 1,
        time: RosTime {
            sec: 3,
            nanosec: 123,
        },
        trailing: vec![0; 1024],
    };
    let schema = Arc::new(RosStamped::schema());
    let bytes = <RosStamped as Message>::Codec::serialize(&value).unwrap();
    let projection = TimeProjection::field(".time", TimeEncoding::RosTime).unwrap();
    projection.validate(&schema).unwrap();
    assert_eq!(
        projection
            .project_cdr(Time::zero(), &bytes[..16], &schema)
            .unwrap(),
        Time::from_nanos(3_000_000_123)
    );
    assert!(matches!(
        project(
            &RosTime {
                sec: -1,
                nanosec: 0
            },
            TimeEncoding::RosTime
        ),
        Err(ProjectionError::Negative { seconds: -1, .. })
    ));
    assert!(matches!(
        project(
            &RosTime {
                sec: 1,
                nanosec: 1_000_000_000
            },
            TimeEncoding::RosTime
        ),
        Err(ProjectionError::InvalidNanoseconds {
            nanoseconds: 1_000_000_000,
            ..
        })
    ));
}

#[test]
fn ros_z_time_projection_checks_range_and_uses_standard_duration_deserialization() {
    let overflow = Time::zero().saturating_add(Duration::from_secs(u64::MAX));
    assert!(matches!(
        project(&overflow, TimeEncoding::RosZTime),
        Err(ProjectionError::Overflow { .. })
    ));
    let schema = Arc::new(Time::schema());
    let mut bytes = <Time as Message>::Codec::serialize(&Time::from_nanos(123)).unwrap();
    bytes[12..16].copy_from_slice(&1_000_000_000u32.to_le_bytes());
    let projection = TimeProjection::field("", TimeEncoding::RosZTime).unwrap();
    let normalized = projection
        .project_cdr(Time::zero(), &bytes, &schema)
        .unwrap();
    assert_eq!(normalized, Time::from_nanos(1_000_000_000));
}

#[test]
fn projection_errors_preserve_selection_and_schema_failures() {
    let schema = Arc::new(Stamped::schema());
    let missing = TimeProjection::field(".missing", TimeEncoding::RosZTime).unwrap();
    assert!(matches!(
        missing.validate(&schema),
        Err(ProjectionError::Selection {
            source: CdrSelectionError::Selection(SelectionError::Field(_)),
            ..
        })
    ));
    let mismatch = TimeProjection::field(".value", TimeEncoding::RosZTime).unwrap();
    assert!(matches!(
        mismatch.validate(&schema),
        Err(ProjectionError::Selection {
            source: CdrSelectionError::TypeMismatch { .. },
            ..
        })
    ));
    assert!(matches!(
        project(&None::<Time>, TimeEncoding::RosZTime),
        Err(ProjectionError::Selection {
            source: CdrSelectionError::Selection(SelectionError::AbsentOptional),
            ..
        })
    ));
    assert!(matches!(
        TimeProjection::field("not_a_path", TimeEncoding::RosZTime),
        Err(ProjectionError::Path(SelectionError::Syntax(_)))
    ));
    assert_eq!(
        TimeProjection::SourceTime
            .project_cdr(Time::from_nanos(7), &[], &schema)
            .unwrap(),
        Time::from_nanos(7)
    );
}
