use std::{future::Future, pin::Pin, sync::Arc};

use booster::MotorState;
use color_eyre::{Report, Result, eyre::eyre};
use coordinate_systems::{Ground, Robot};
use kinematics::joints::{Joints, head::HeadJoints};
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::{
    cache::Cache,
    prelude::*,
    qos::{QosDurability, QosHistory},
    time::Time,
};
use ros_z_schema::{ServiceDef, compute_hash};
use serde::{Deserialize, Serialize};
use types::motor_command::MotorCommand;
use types::{
    field_dimensions::FieldDimensions, filtered_game_controller_state::FilteredGameControllerState,
    joint_limits::JointLimits, motion_command::HeadMotion, time_wrapper::TimeWrapper,
};

use crate::{
    head::{HeadController, HeadInputs},
    logging::{FailureKind, NodeLogger},
    look_at::LookAtGeometry,
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
    let mut inputs = InputCaches::new(&node).await?;

    let joint_limits_sub = node
        .subscriber::<JointLimits>("joint_limits")
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            history: QosHistory::from_depth(1),
            ..Default::default()
        })
        .build()
        .await?;

    let serial_motor_states_sub = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")
        .qos(QosProfile {
            history: QosHistory::from_depth(1),
            ..Default::default()
        })
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
            received = joint_limits_sub.recv() => {
                let result = match received {
                    Ok(joint_limits) => inputs.update_joint_limits(joint_limits),
                    Err(error) => Err(error.into()),
                };
                if let Err(error) = result {
                    logger.log_error(
                        FailureKind::JointLimits, None, &error,
                        parameters.snapshot().typed().joint_control.warning_interval,
                        node.clock().now(),
                    );
                }
            }
            received = serial_motor_states_sub.recv_with_metadata() => {
                let result = match received {
                    Ok(received) => controller.observe(received.message.head.into(), received.source_time),
                    Err(error) => Err(error.into()),
                };
                if let Err(error) = result {
                    logger.log_error(
                        FailureKind::Observation, None, &error,
                        parameters.snapshot().typed().joint_control.warning_interval,
                        node.clock().now(),
                    );
                }
            }
            received = head_motion_service.take_request_async() => {
                let snapshot = parameters.snapshot();
                let warning_interval = snapshot.typed().joint_control.warning_interval;
                let (request, reply) = match received {
                    Ok(received) => received.into_parts(),
                    Err(error) => {
                        controller.clear_command_history();
                        logger.log_error(FailureKind::Request, None, &error.into(),
                            warning_interval, node.clock().now());
                        continue;
                    }
                };
                let now = node.clock().now();
                let result = (|| {
                    request.validate().map_err(Report::msg)?;
                    let inputs = inputs.snapshot(Arc::clone(&snapshot.typed), now)?;
                    controller.evaluate(&request, &inputs, now)
                })();
                let response = match result {
                    Ok(output) => {
                        logger.log_output(&request, &output, warning_interval, now);
                        Ok(output.commands)
                    }
                    Err(error) => {
                        controller.clear_command_history();
                        logger.log_error(FailureKind::Request, Some(&request), &error,
                            warning_interval, now);
                        Err(HeadMotionError { source: Arc::new(error) })
                    }
                };
                if let Err(error) = reply.reply_async(&response).await {
                    controller.clear_command_history();
                    logger.log_error(FailureKind::Response, Some(&request), &error.into(),
                        warning_interval, now);
                }
            }
        }
    }
}

struct InputCaches {
    joint_limits: Option<Arc<JointLimits>>,
    field_dimensions: Cache<FieldDimensions>,
    camera_matrix: Cache<TimeWrapper<CameraMatrix>>,
    ground_to_robot: Cache<TimeWrapper<Option<Isometry3<Ground, Robot>>>>,
    game_controller_state: Cache<FilteredGameControllerState>,
}

impl InputCaches {
    async fn new(node: &Node) -> Result<Self> {
        let field_dimensions_cache = node
            .subscriber::<FieldDimensions>("field_dimensions")
            .qos(QosProfile {
                durability: QosDurability::TransientLocal,
                ..Default::default()
            })
            .cache(1)
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

        Ok(Self {
            joint_limits: None,
            field_dimensions: field_dimensions_cache,
            camera_matrix: camera_matrix_cache,
            ground_to_robot: ground_to_robot_cache,
            game_controller_state: filtered_game_controller_state_cache,
        })
    }

    fn update_joint_limits(&mut self, joint_limits: JointLimits) -> Result<()> {
        self.joint_limits = None;
        joint_limits.validate().map_err(Report::msg)?;
        self.joint_limits = Some(Arc::new(joint_limits));
        Ok(())
    }

    fn snapshot(&self, parameters: Arc<Parameters>, now: Time) -> Result<HeadInputs> {
        let joint_limits = self
            .joint_limits
            .clone()
            .ok_or_else(|| eyre!("head joint limits are unavailable"))?;
        let field_width = self.field_dimensions.get_latest().map(|field| field.width);
        let geometry = self
            .ground_to_robot
            .get_latest()
            .filter(|ground| {
                ground.time <= now
                    && now.duration_since(ground.time) <= parameters.maximum_ground_pose_age
            })
            .and_then(|ground| ground.inner)
            .zip(self.camera_matrix.get_latest())
            .map(|(ground_to_robot, camera)| LookAtGeometry {
                camera_matrix: camera.inner.clone(),
                ground_to_robot,
            });
        let global_field_side = self
            .game_controller_state
            .get_latest()
            .map(|game| game.global_field_side);
        Ok(HeadInputs {
            parameters,
            joint_limits,
            geometry,
            field_width,
            global_field_side,
        })
    }
}

pub struct HeadMotionService;

#[derive(Clone, Debug, Serialize, Deserialize, Message, thiserror::Error)]
#[error("head motion failed: {source:#}")]
pub struct HeadMotionError {
    #[serde(with = "ros_z::message::report")]
    pub source: Arc<Report>,
}

impl Service for HeadMotionService {
    type Request = HeadMotion;
    type Response = Result<HeadJoints<MotorCommand>, HeadMotionError>;
}

impl ServiceTypeInfo for HeadMotionService {
    fn service_type_info() -> TypeInfo {
        let descriptor = ServiceDef::new(
            "head_motion::node::HeadMotionService",
            HeadMotion::type_name(),
            <Self as Service>::Response::type_name(),
        )
        .expect("static head motion service descriptor is valid");
        let hash = compute_hash(&descriptor).expect("static head motion service hash is valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}
