use std::path::Path;

use color_eyre::{eyre::Context, Result};
use parameters::{
    directory::{serialize, Id, Location, Scope},
    json::nest_value_at_path,
};
use spl_network_messages::PlayerNumber;
use types::hardware::Ids;

/// Sets the player number in the parameters directory.
///
/// This function takes a head ID of a robot and a player number, and writes it to the parameters.
pub async fn set_player_number(
    head_id: &str,
    player_number: PlayerNumber,
    repository_root: impl AsRef<Path>,
) -> Result<()> {
    let parameters_root = repository_root.as_ref().join("etc/parameters/");
    let path = "player_number";
    let parameters = nest_value_at_path(
        path,
        serde_json::to_value(player_number).wrap_err("failed to serialize player number")?,
    );
    serialize(
        &parameters,
        Scope {
            location: Location::All,
            id: Id::Head,
        },
        path,
        parameters_root,
        &Ids {
            body_id: "unknown_body_id".to_string(),
            head_id: head_id.to_string(),
        },
    )
    .wrap_err("failed to serialize parameters directory")
}
