use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use booster::LowState;
use color_eyre::{Report, Result, eyre::WrapErr};
use coordinate_systems::{Ground, Robot};
use kinematics::joints::head::HeadJoints;
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::{
    Result as RosResult,
    prelude::*,
    pubsub::Received,
    qos::{QosDurability, QosHistory},
    time::Time,
};
use ros_z_schema::{ServiceDef, compute_hash};
use types::motor_command::MotorCommand;
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    joint_limits::JointLimits, motion_command::HeadMotion, time_wrapper::TimeWrapper,
};

use crate::{
    head::{HeadContext, HeadController},
    joint_control::HeadObservation,
    logging::{FailureKind, NodeLogger},
    look_at::GazeGeometry,
    parameters::Parameters,
};

pub const HEAD_MOTION_SERVICE_TOPIC: &str = "services/head_motion";

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("head_motion").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("head_motion")?;
    parameters.add_validation_hook(Parameters::validate)?;
    let joint_limits_cache = node
        .subscriber::<JointLimits>("joint_limits")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;

    let field_dimensions_cache = node
        .subscriber::<FieldDimensions>("field_dimensions")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;

    let low_state_sub = node
        .subscriber::<LowState>("inputs/low_state")
        .qos(QosProfile {
            history: QosHistory::from_depth(1),
            ..Default::default()
        })
        .build()
        .await?;
    let camera_matrix_cache = node
        .subscriber::<TimeWrapper<CameraMatrix>>("camera_matrix")
        .cache(1)
        .with_stamp(|wrapper: &TimeWrapper<CameraMatrix>| wrapper.time)
        .build()
        .await?;
    let ground_to_robot_cache = node
        .subscriber::<TimeWrapper<Option<Isometry3<Ground, Robot>>>>("ground_to_robot")
        .cache(1)
        .with_stamp(|wrapper: &TimeWrapper<Option<Isometry3<Ground, Robot>>>| wrapper.time)
        .build()
        .await?;
    let filtered_game_controller_state_cache = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .cache(1)
        .build()
        .await?;

    let mut head_motion_service = node
        .service_server::<HeadMotionService>(HEAD_MOTION_SERVICE_TOPIC)
        .qos(QosProfile {
            history: QosHistory::from_depth(1),
            ..Default::default()
        })
        .build()
        .await?;

    let mut controller = HeadController::default();
    let mut logger = NodeLogger::default();

    loop {
        tokio::select! {
            received = low_state_sub.recv_with_metadata() => {
                receive_observation(
                    received, &mut controller, &mut logger,
                    parameters.snapshot().typed().joint_control.warning_interval,
                    node.clock().now(),
                );
            }
            received = head_motion_service.take_request_async() => {
                let snapshot = parameters.snapshot();
                let parameters = snapshot.typed();
                let (request, reply) = match received {
                    Ok(received) => received.into_parts(),
                    Err(error) => {
                        logger.log_error(FailureKind::Request, None, &error.into(),
                            parameters.joint_control.warning_interval, node.clock().now());
                        continue;
                    }
                };
                // This loop is the subscriber's only consumer. A ready sample can be
                // taken immediately, including when both select branches were ready.
                if low_state_sub.is_ready() {
                    receive_observation(
                        low_state_sub.recv_with_metadata().await,
                        &mut controller, &mut logger,
                        parameters.joint_control.warning_interval, node.clock().now(),
                    );
                }
                let camera = camera_matrix_cache.get_latest();
                let ground = ground_to_robot_cache.get_latest();
                let limits = joint_limits_cache.get_latest();
                let field = field_dimensions_cache.get_latest();
                let game = filtered_game_controller_state_cache.get_latest();
                let context = HeadContext {
                    geometry: camera.as_deref().zip(ground.as_deref()).and_then(|(camera, ground)| {
                        ground.inner.map(|ground_to_robot| GazeGeometry {
                            camera_matrix: &camera.inner,
                            ground_to_robot,
                        })
                    }),
                    joint_limits: limits.as_deref(),
                    field_dimensions: field.as_deref(),
                    field_side: game.as_deref().map(|game| game.global_field_side),
                };
                let now = node.clock().now();
                let output = match controller.evaluate(&request, &context, parameters, now) {
                    Ok(output) => output,
                    Err(error) => {
                        logger.log_error(FailureKind::Request, Some(&request), &error,
                            parameters.joint_control.warning_interval, now);
                        // The response contract contains commands only. Dropping the
                        // reply lets central motion's timed call fail without commands.
                        continue;
                    }
                };
                logger.log_output(&request, &output, &parameters.joint_control, now);
                if let Err(error) = reply.reply_async(&output.joint_control.commands).await {
                    logger.log_error(FailureKind::Response, Some(&request), &error.into(),
                        parameters.joint_control.warning_interval, now);
                }
            }
        }
    }
}

fn receive_observation(
    received: RosResult<Received<LowState>>,
    controller: &mut HeadController,
    logger: &mut NodeLogger,
    warning_interval: Duration,
    now: Time,
) {
    let result = received.map_err(Report::new).and_then(|received| {
        observe_low_state(controller, &received.message, received.source_time)
    });
    if let Err(error) = result {
        controller.invalidate_observation();
        logger.log_error(
            FailureKind::Observation,
            None,
            &error,
            warning_interval,
            now,
        );
    }
}

fn observe_low_state(controller: &mut HeadController, state: &LowState, time: Time) -> Result<()> {
    let head = state
        .serial_motor_states()
        .wrap_err("invalid serial motor states in LowState")?
        .head;
    controller.observe(
        HeadObservation {
            positions: HeadJoints {
                yaw: head.yaw.position,
                pitch: head.pitch.position,
            },
            velocities: HeadJoints {
                yaw: head.yaw.velocity,
                pitch: head.pitch.velocity,
            },
        },
        time,
    )
}

pub struct HeadMotionService;

impl Service for HeadMotionService {
    type Request = HeadMotion;
    type Response = HeadJoints<MotorCommand>;
}

impl ServiceTypeInfo for HeadMotionService {
    fn service_type_info() -> TypeInfo {
        let descriptor = ServiceDef::new(
            "head_motion::node::HeadMotionService",
            HeadMotion::type_name(),
            HeadJoints::<MotorCommand>::type_name(),
        )
        .expect("static head motion service descriptor is valid");
        let hash = compute_hash(&descriptor).expect("static head motion service hash is valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use booster::MotorState;
    use json5::from_str;
    use kinematics::joints::Joints;
    use ros_z::time::Clock;
    use serde::Deserialize;
    use tokio::time::{sleep, timeout};

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn service_rejects_bad_observations_and_recovers_using_source_time() {
        #[derive(Deserialize)]
        struct Global {
            joint_limits: JointLimits,
        }
        let global: Global =
            from_str(include_str!("../../../../etc/parameters/base/global.json5")).unwrap();
        let parameters: Parameters = from_str(include_str!(
            "../../../../etc/parameters/base/head_motion.json5"
        ))
        .unwrap();
        let clock = Clock::logical(Time::zero() + Duration::from_secs(1));
        let context = ContextBuilder::default()
            .with_mode("peer")
            .disable_multicast_scouting()
            .with_clock(clock.clone())
            .with_parameter_layer(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../etc/parameters/base"),
            )
            .build()
            .await
            .unwrap();
        let peer = context
            .create_node("head_motion_test")
            .build()
            .await
            .unwrap();
        let limits = peer
            .publisher::<JointLimits>("joint_limits")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                ..Default::default()
            })
            .build()
            .await
            .unwrap();
        // Verify that a node starting after the global provider receives its limits.
        limits.publish(&global.joint_limits).await.unwrap();
        let sensors = peer
            .publisher::<LowState>("inputs/low_state")
            .build()
            .await
            .unwrap();
        let client = peer
            .service_client::<HeadMotionService>(HEAD_MOTION_SERVICE_TOPIC)
            .build()
            .await
            .unwrap();
        let mut motors = Joints::fill(MotorState::default());
        motors.head.yaw.position = 0.2;
        motors.head.pitch.position = 0.3;
        motors.head.yaw.velocity = 0.4;
        motors.head.pitch.velocity = -0.2;
        let sample = LowState {
            motor_state_serial: motors.into_iter().collect(),
            ..Default::default()
        };

        let exercise = async {
            assert!(
                sensors
                    .wait_for_subscribers(1, Duration::from_secs(2))
                    .await
            );
            // Unavailable observations cannot produce even a Damping reply.
            wait_for_reply_state(&client, &HeadMotion::Damping, false).await;
            sensors.publish(&sample).await.unwrap();
            let active = wait_for_reply_state(&client, &HeadMotion::ZeroAngles, true)
                .await
                .unwrap();
            assert_eq!(active.yaw.position, 0.2);
            assert_eq!(active.pitch.position, 0.3);
            assert_eq!(active.yaw.velocity, 0.4);
            assert_eq!(active.pitch.velocity, -0.2);
            assert_eq!(active.yaw.kp, parameters.joint_control.kp.yaw);
            let damping = wait_for_reply_state(&client, &HeadMotion::Damping, true)
                .await
                .unwrap();
            assert_eq!(damping.yaw.kp, 0.0);
            assert_eq!(damping.pitch.kd, parameters.joint_control.damping_kd.pitch);

            let old_time = clock.now();
            clock
                .advance(parameters.maximum_observation_age + Duration::from_nanos(1))
                .unwrap();
            // Republishing an old source timestamp must not freshen the measurement.
            sensors
                .publish_with_source_time(&sample, old_time)
                .await
                .unwrap();
            wait_for_reply_state(&client, &HeadMotion::ZeroAngles, false).await;
            sensors.publish(&sample).await.unwrap();
            wait_for_reply_state(&client, &HeadMotion::ZeroAngles, true).await;

            // A malformed sample must invalidate a previously usable observation.
            sensors.publish(&LowState::default()).await.unwrap();
            wait_for_reply_state(&client, &HeadMotion::ZeroAngles, false).await;
            sensors.publish(&sample).await.unwrap();
            wait_for_reply_state(&client, &HeadMotion::ZeroAngles, true).await;
            let mut invalid = sample.clone();
            invalid.motor_state_serial[0].position = f32::NAN;
            sensors.publish(&invalid).await.unwrap();
            wait_for_reply_state(&client, &HeadMotion::Damping, false).await;
            sensors.publish(&sample).await.unwrap();
            wait_for_reply_state(&client, &HeadMotion::Damping, true).await;
        };
        // Cancellation drops the node and its cache tasks even if an assertion fails.
        tokio::select! {
            result = run(Arc::new(context)) => panic!("head node stopped: {result:?}"),
            () = exercise => {}
        }
    }

    async fn wait_for_reply_state(
        client: &ServiceClient<HeadMotionService>,
        request: &HeadMotion,
        succeeds: bool,
    ) -> Option<HeadJoints<MotorCommand>> {
        timeout(Duration::from_secs(2), async {
            loop {
                let result = client
                    .call_with_timeout_async(request, Duration::from_millis(100))
                    .await;
                if result.is_ok() == succeeds {
                    return result.ok();
                }
                sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("service did not reach the expected reply state")
    }
}
