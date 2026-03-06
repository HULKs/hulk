use color_eyre::{Result, eyre::Context};
use hsl_network_messages::PlayerNumber;
use hula_types::hardware::Ids;
use parameters::{
    directory::{Id, Location, Scope, serialize},
    json::nest_value_at_path,
};

use crate::Repository;

impl Repository {
    pub async fn configure_player_number(
        &self,
        robot_id: &str,
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
                id: Id::Robot,
            },
            path,
            parameters_root,
            &Ids {
                robot_id: robot_id.to_string(),
            },
        )
        .wrap_err("failed to serialize parameters directory")?;
        Ok(())
    }
}
