use std::{future::Future, future::pending, pin::Pin, sync::Arc};

use booster::LowState;
use color_eyre::Result;
use coordinate_systems::{Ground, Robot};
use kinematics::joints::head::HeadJoints;
use linear_algebra::Isometry3;
use projection::camera_matrix::CameraMatrix;
use ros_z::prelude::*;
use types::{
    filtered_game_controller_state::FilteredGameControllerState, motion_command::HeadMotion,
    robot_command::MotorCommand, time_wrapper::TimeWrapper,
};

use crate::{head::HeadController, parameters::Parameters};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("head_motion").build().await?;

    let _parameters = node.bind_parameter_as::<Parameters>("head_motion")?;

    let _low_state_sub = node
        .subscriber::<LowState>("inputs/low_state")
        .build()
        .await?;
    let _camera_matrix_cache = node
        .subscriber::<TimeWrapper<CameraMatrix>>("camera_matrix")
        .cache(1)
        .with_stamp(|wrapper: &TimeWrapper<CameraMatrix>| wrapper.time)
        .build()
        .await?;
    let _ground_to_robot_cache = node
        .subscriber::<TimeWrapper<Option<Isometry3<Ground, Robot>>>>("ground_to_robot")
        .cache(1)
        .with_stamp(|wrapper: &TimeWrapper<Option<Isometry3<Ground, Robot>>>| wrapper.time)
        .build()
        .await?;
    let _filtered_game_controller_state_cache = node
        .subscriber::<FilteredGameControllerState>("filtered_game_controller_state")
        .cache(1)
        .build()
        .await?;

    let _head_motion_service = node
        .service_server::<HeadMotionService>("services/head_motion")
        .build()
        .await?;

    let _controller = HeadController::default();

    // TODO: Feed LowState observations to the controller and evaluate service requests.
    // Keep the controller's unimplemented entry points disconnected until logic is added.
    pending::<()>().await;

    Ok(())
}

pub struct HeadMotionService;

impl Service for HeadMotionService {
    type Request = HeadMotion;
    type Response = HeadJoints<MotorCommand>;
}

impl ServiceTypeInfo for HeadMotionService {
    fn service_type_info() -> TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "head_motion::node::HeadMotionService",
            HeadMotion::type_name(),
            HeadJoints::<MotorCommand>::type_name(),
        )
        .expect("static head motion service descriptor is valid");
        let hash = ros_z_schema::compute_hash(&descriptor)
            .expect("static head motion service hash is valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}
