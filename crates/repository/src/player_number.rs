use color_eyre::{eyre::Context, Result};
use hula_types::hardware::Ids;
use parameters::{
    directory::{serialize, Id, Location, Scope},
    json::nest_value_at_path,
};
use spl_network_messages::PlayerNumber;

use crate::Repository;

impl Repository {
    pub async fn configure_player_number(
        &self,
        head_id: &str,
        player_number: PlayerNumber,
    ) -> Result<()> {
        let parameters_root = self.root.join("etc/parameters/");
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
        .wrap_err("failed to serialize parameters directory")?;
        Ok(())
    }
}
