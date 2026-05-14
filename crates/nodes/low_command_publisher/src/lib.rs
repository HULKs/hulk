use std::sync::Arc;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::{CommandType, LowCommand, MotorCommandParameters};
use kinematics::joints::Joints;
use ros_z::{message::WireEncoder, prelude::*};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub prepare_motor_command_parameters: MotorCommandParameters,
    pub walk_motor_command_parameters: MotorCommandParameters,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("command_sender").build().await?;

    let zenoh_session = ctx.session();

    let parameters = node.bind_parameter_as::<Parameters>("command_sender")?;
    let collected_target_joint_positions_sub = node
        .subscriber::<Joints<f32>>("collected_target_joint_positions")?
        .build()
        .await?;
    let low_command_pub = node
        .publisher::<LowCommand>("actions/low_command")?
        .build()
        .await?;

    let zenoh_publisher = zenoh_session
        .declare_publisher("rt/joint_ctrl")
        .await
        .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;

    loop {
        let parameters = parameters.snapshot().typed().clone();

        tokio::select! {
            collected_target_joint_positions = collected_target_joint_positions_sub.recv() => {
                let collected_target_joint_positions = collected_target_joint_positions?;

                let low_command = LowCommand::new(
                    &collected_target_joint_positions,
                    &parameters.walk_motor_command_parameters,
                    CommandType::Serial,
                );

                low_command_pub.publish(&low_command).await?;

                let low_command_bytes = <LowCommand as Message>::Codec::serialize(&low_command)?;

                zenoh_publisher
                    .put(&low_command_bytes)
                    .await
                    .map_err(|error| color_eyre::eyre::eyre!("{error}"))?;
            }
        }
    }
}
