use std::collections::VecDeque;
use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::{Result, eyre::Context as _};

use booster::{ImuState, LowState, MotorState};
use kinematics::joints::Joints;
use ros_z::{prelude::*, time::Time};
use ros2::sensor_msgs::joint_state::JointState;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

const SAMPLE_BUFFER_LIMIT: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SampleKey {
    positions: Vec<u32>,
    velocities: Vec<u32>,
    efforts: Vec<u32>,
}

impl SampleKey {
    fn from_low_state(low_state: &LowState) -> Self {
        Self::from_f32_parts(
            low_state
                .motor_state_serial
                .iter()
                .map(|motor_state| motor_state.position),
            low_state
                .motor_state_serial
                .iter()
                .map(|motor_state| motor_state.velocity),
            low_state
                .motor_state_serial
                .iter()
                .map(|motor_state| motor_state.torque),
        )
    }

    fn from_joint_state(joint_state: &JointState) -> Self {
        Self::from_f32_parts(
            joint_state.position.iter().map(|position| *position as f32),
            joint_state.velocity.iter().map(|velocity| *velocity as f32),
            joint_state.effort.iter().map(|effort| *effort as f32),
        )
    }

    fn from_f32_parts(
        positions: impl IntoIterator<Item = f32>,
        velocities: impl IntoIterator<Item = f32>,
        efforts: impl IntoIterator<Item = f32>,
    ) -> Self {
        Self {
            positions: positions.into_iter().map(f32::to_bits).collect(),
            velocities: velocities.into_iter().map(f32::to_bits).collect(),
            efforts: efforts.into_iter().map(f32::to_bits).collect(),
        }
    }
}

#[derive(Debug)]
struct PendingLowState {
    sequence_number: u64,
    key: SampleKey,
    low_state: LowState,
}

#[derive(Debug)]
struct JointStateStamp {
    key: SampleKey,
    source_time: Time,
}

#[derive(Debug)]
struct MatchedLowState {
    low_state: LowState,
    source_time: Time,
}

#[derive(Debug, Default)]
struct DropCounters {
    low_state_buffer_overflows: u64,
    joint_state_buffer_overflows: u64,
    non_monotonic_low_states: u64,
}

#[derive(Debug, Default)]
struct LowStateMatcher {
    pending_low_states: VecDeque<PendingLowState>,
    joint_state_stamps: VecDeque<JointStateStamp>,
    next_sequence_number: u64,
    last_published_source_time: Option<Time>,
    drop_counters: DropCounters,
}

impl LowStateMatcher {
    fn insert_low_state(&mut self, low_state: LowState) -> Option<MatchedLowState> {
        let key = SampleKey::from_low_state(&low_state);
        let sequence_number = self.reserve_sequence_number();

        if let Some(index) = self
            .joint_state_stamps
            .iter()
            .position(|joint_state| joint_state.key == key)
        {
            let joint_state = self
                .joint_state_stamps
                .remove(index)
                .expect("joint state index came from position");
            let low_state = PendingLowState {
                sequence_number,
                key,
                low_state,
            };

            return self.accept_match(low_state, joint_state.source_time);
        }

        self.pending_low_states.push_back(PendingLowState {
            sequence_number,
            key,
            low_state,
        });

        if self.pending_low_states.len() > SAMPLE_BUFFER_LIMIT {
            self.pending_low_states.pop_front();
            self.drop_counters.low_state_buffer_overflows += 1;
            log_drop(
                "low_state_buffer_overflow",
                self.drop_counters.low_state_buffer_overflows,
            );
        }

        None
    }

    fn insert_joint_state(&mut self, joint_state: JointState) -> Option<MatchedLowState> {
        let joint_state = JointStateStamp {
            key: SampleKey::from_joint_state(&joint_state),
            source_time: joint_state.header.stamp.into(),
        };

        if let Some(index) = self
            .pending_low_states
            .iter()
            .position(|low_state| low_state.key == joint_state.key)
        {
            let low_state = self
                .pending_low_states
                .remove(index)
                .expect("low state index came from position");

            return self.accept_match(low_state, joint_state.source_time);
        }

        self.joint_state_stamps.push_back(joint_state);

        if self.joint_state_stamps.len() > SAMPLE_BUFFER_LIMIT {
            self.joint_state_stamps.pop_front();
            self.drop_counters.joint_state_buffer_overflows += 1;
            log_drop(
                "joint_state_buffer_overflow",
                self.drop_counters.joint_state_buffer_overflows,
            );
        }

        None
    }

    fn reserve_sequence_number(&mut self) -> u64 {
        let sequence_number = self.next_sequence_number;
        self.next_sequence_number += 1;
        sequence_number
    }

    fn accept_match(
        &mut self,
        low_state: PendingLowState,
        source_time: Time,
    ) -> Option<MatchedLowState> {
        if let Some(last_published_source_time) = self.last_published_source_time
            && source_time < last_published_source_time
        {
            self.drop_counters.non_monotonic_low_states += 1;
            tracing::debug!(
                sequence_number = low_state.sequence_number,
                matched_source_time = ?source_time,
                ?last_published_source_time,
                "dropping matched low_state because its source time would move backward"
            );
            log_drop(
                "non_monotonic_low_state",
                self.drop_counters.non_monotonic_low_states,
            );
            return None;
        }

        self.last_published_source_time = Some(source_time);
        Some(MatchedLowState {
            low_state: low_state.low_state,
            source_time,
        })
    }
}

fn log_drop(kind: &'static str, count: u64) {
    if count == 1 {
        tracing::warn!(kind, count, "low_state_bridge dropped a buffered sample");
    } else {
        tracing::debug!(kind, count, "low_state_bridge dropped a buffered sample");
    }
}

async fn publish_matched_low_state(
    low_state_pub: &Publisher<LowState>,
    imu_state_pub: &Publisher<ImuState>,
    serial_motor_states_pub: &Publisher<Joints<MotorState>>,
    parallel_motor_states_pub: &Publisher<Option<Joints<MotorState>>>,
    matched_low_state: MatchedLowState,
) -> Result<()> {
    let source_time = matched_low_state.source_time;
    let low_state = matched_low_state.low_state;

    let imu_state = low_state.imu_state;
    let serial_motor_states = low_state.serial_motor_states()?;
    let parallel_motor_states = low_state.parallel_motor_states().ok();

    low_state_pub
        .publish_with_source_time(&low_state, source_time)
        .await?;
    imu_state_pub
        .publish_with_source_time(&imu_state, source_time)
        .await?;
    serial_motor_states_pub
        .publish_with_source_time(&serial_motor_states, source_time)
        .await?;
    parallel_motor_states_pub
        .publish_with_source_time(&parallel_motor_states, source_time)
        .await?;

    Ok(())
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("low_state_bridge").build().await?;

    let zenoh_session = ctx.session();

    let low_state_sub = zenoh_session
        .declare_subscriber("rt/low_state")
        .await
        .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;
    let joint_state_sub = zenoh_session
        .declare_subscriber("rt/joint_states")
        .await
        .map_err(|error| {
            color_eyre::eyre::eyre!("failed to declare subscriber for rt/joint_states: {error}")
        })?;

    let low_state_pub = node
        .publisher::<LowState>("inputs/low_state")
        .build()
        .await?;
    let imu_state_pub = node
        .publisher::<ImuState>("inputs/imu_state")
        .build()
        .await?;
    let serial_motor_states_pub = node
        .publisher::<Joints<MotorState>>("inputs/serial_motor_states")
        .build()
        .await?;
    let parallel_motor_states_pub = node
        .publisher::<Option<Joints<MotorState>>>("inputs/parallel_motor_states")
        .build()
        .await?;

    let mut matcher = LowStateMatcher::default();

    loop {
        tokio::select! {
            low_state = low_state_sub.recv_async() => {
                let low_state = low_state.map_err(|error| color_eyre::eyre::eyre!("{error}"))?;

                let low_state: LowState = cdr::deserialize(&low_state.payload().to_bytes())
                    .wrap_err("low state deserialization failed")?;

                if let Some(matched_low_state) = matcher.insert_low_state(low_state) {
                    publish_matched_low_state(
                        &low_state_pub,
                        &imu_state_pub,
                        &serial_motor_states_pub,
                        &parallel_motor_states_pub,
                        matched_low_state,
                    )
                    .await?;
                }
            }
            joint_state = joint_state_sub.recv_async() => {
                let joint_state = joint_state.map_err(|error| {
                    color_eyre::eyre::eyre!("failed to receive sample from rt/joint_states: {error}")
                })?;

                let joint_state: JointState = cdr::deserialize(&joint_state.payload().to_bytes())
                    .wrap_err("joint state deserialization failed")?;

                if let Some(matched_low_state) = matcher.insert_joint_state(joint_state) {
                    publish_matched_low_state(
                        &low_state_pub,
                        &imu_state_pub,
                        &serial_motor_states_pub,
                        &parallel_motor_states_pub,
                        matched_low_state,
                    )
                    .await?;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use booster::MotorState;
    use ros2::{builtin_interfaces::time::Time as RosTime, std_msgs::header::Header};

    fn low_state(values: &[(f32, f32, f32)]) -> LowState {
        LowState {
            imu_state: Default::default(),
            motor_state_parallel: Vec::new(),
            motor_state_serial: values
                .iter()
                .map(|(position, velocity, torque)| MotorState {
                    position: *position,
                    velocity: *velocity,
                    torque: *torque,
                    ..Default::default()
                })
                .collect(),
        }
    }

    fn joint_state(nanos: i64, values: &[(f64, f64, f64)]) -> JointState {
        let duration = std::time::Duration::from_nanos(nanos as u64);
        JointState {
            header: Header {
                stamp: RosTime {
                    sec: duration.as_secs() as i32,
                    nanosec: duration.subsec_nanos(),
                },
                frame_id: String::new(),
            },
            name: Vec::new(),
            position: values.iter().map(|(position, _, _)| *position).collect(),
            velocity: values.iter().map(|(_, velocity, _)| *velocity).collect(),
            effort: values.iter().map(|(_, _, effort)| *effort).collect(),
        }
    }

    #[test]
    fn low_state_waits_until_matching_joint_state_arrives() {
        let mut matcher = LowStateMatcher::default();
        let low_state = low_state(&[(1.0, 2.0, 3.0)]);

        assert!(matcher.insert_low_state(low_state).is_none());

        let matched = matcher.insert_joint_state(joint_state(10, &[(1.0, 2.0, 3.0)]));

        assert_eq!(matched.unwrap().source_time, Time::from_nanos(10));
        assert_eq!(matcher.pending_low_states.len(), 0);
        assert_eq!(matcher.joint_state_stamps.len(), 0);
    }

    #[test]
    fn joint_state_waits_until_matching_low_state_arrives() {
        let mut matcher = LowStateMatcher::default();

        assert!(
            matcher
                .insert_joint_state(joint_state(20, &[(1.0, 2.0, 3.0)]))
                .is_none()
        );

        let matched = matcher.insert_low_state(low_state(&[(1.0, 2.0, 3.0)]));

        assert_eq!(matched.unwrap().source_time, Time::from_nanos(20));
        assert_eq!(matcher.pending_low_states.len(), 0);
        assert_eq!(matcher.joint_state_stamps.len(), 0);
    }

    #[test]
    fn joint_state_f64_values_match_low_state_f32_values() {
        let mut matcher = LowStateMatcher::default();
        let position = 0.1_f32;
        let velocity = -0.2_f32;
        let torque = 0.3_f32;

        assert!(
            matcher
                .insert_joint_state(joint_state(
                    30,
                    &[(position as f64, velocity as f64, torque as f64)]
                ))
                .is_none()
        );

        let matched = matcher.insert_low_state(low_state(&[(position, velocity, torque)]));

        assert_eq!(matched.unwrap().source_time, Time::from_nanos(30));
    }

    #[test]
    fn duplicate_keys_consume_oldest_joint_state_first() {
        let mut matcher = LowStateMatcher::default();

        assert!(
            matcher
                .insert_joint_state(joint_state(40, &[(1.0, 2.0, 3.0)]))
                .is_none()
        );
        assert!(
            matcher
                .insert_joint_state(joint_state(50, &[(1.0, 2.0, 3.0)]))
                .is_none()
        );

        let first = matcher.insert_low_state(low_state(&[(1.0, 2.0, 3.0)]));
        let second = matcher.insert_low_state(low_state(&[(1.0, 2.0, 3.0)]));

        assert_eq!(first.unwrap().source_time, Time::from_nanos(40));
        assert_eq!(second.unwrap().source_time, Time::from_nanos(50));
    }

    #[test]
    fn low_state_buffer_overflow_drops_oldest_low_state() {
        let mut matcher = LowStateMatcher::default();

        for index in 0..=SAMPLE_BUFFER_LIMIT {
            assert!(
                matcher
                    .insert_low_state(low_state(&[(index as f32, 0.0, 0.0)]))
                    .is_none()
            );
        }

        assert_eq!(matcher.pending_low_states.len(), SAMPLE_BUFFER_LIMIT);
        assert_eq!(matcher.drop_counters.low_state_buffer_overflows, 1);
        assert_eq!(
            matcher.pending_low_states.front().unwrap().key,
            SampleKey::from_f32_parts([1.0], [0.0], [0.0])
        );
    }

    #[test]
    fn joint_state_buffer_overflow_drops_oldest_joint_state() {
        let mut matcher = LowStateMatcher::default();

        for index in 0..=SAMPLE_BUFFER_LIMIT {
            assert!(
                matcher
                    .insert_joint_state(joint_state(index as i64, &[(index as f64, 0.0, 0.0)]))
                    .is_none()
            );
        }

        assert_eq!(matcher.joint_state_stamps.len(), SAMPLE_BUFFER_LIMIT);
        assert_eq!(matcher.drop_counters.joint_state_buffer_overflows, 1);
        assert_eq!(
            matcher.joint_state_stamps.front().unwrap().source_time,
            Time::from_nanos(1)
        );
    }

    #[test]
    fn older_matched_timestamp_is_dropped_after_newer_publication() {
        let mut matcher = LowStateMatcher::default();

        let first = matcher.insert_joint_state(joint_state(100, &[(1.0, 0.0, 0.0)]));
        assert!(first.is_none());
        let first = matcher.insert_low_state(low_state(&[(1.0, 0.0, 0.0)]));
        assert!(first.is_some());

        let second = matcher.insert_joint_state(joint_state(90, &[(2.0, 0.0, 0.0)]));
        assert!(second.is_none());
        let second = matcher.insert_low_state(low_state(&[(2.0, 0.0, 0.0)]));

        assert!(second.is_none());
        assert_eq!(matcher.drop_counters.non_monotonic_low_states, 1);
    }
}
