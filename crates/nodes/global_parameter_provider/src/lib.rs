use std::sync::Arc;

use color_eyre::Result;
use hsl_network_messages::PlayerNumber;
use serde::{Deserialize, Serialize};

use ros_z::{prelude::*, qos::QosDurability};
use types::field_dimensions::FieldDimensions;

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub player_number: PlayerNumber,
    pub field_dimensions: FieldDimensions,
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("global_parameter_provider").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("global")?;

    let player_number_pub = node
        .publisher::<PlayerNumber>("player_number")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let field_dimensions_pub = node
        .publisher::<FieldDimensions>("field_dimensions")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let mut parameters_receiver = parameters.subscribe();
    loop {
        let _ = parameters_receiver.changed().await;
        let parameters = parameters_receiver.borrow_and_update().clone();
        let parameters = parameters.typed();

        player_number_pub.publish(&parameters.player_number).await?;
        field_dimensions_pub
            .publish(&parameters.field_dimensions.clone())
            .await?;
    }
}
