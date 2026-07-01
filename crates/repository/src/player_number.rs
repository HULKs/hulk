use std::{
    fmt::{self, Display, Formatter},
    io::ErrorKind,
};

use color_eyre::{Result, eyre::Context};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json, to_string_pretty};
use tokio::fs::{create_dir_all, read_to_string, write};

use crate::Repository;

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum PlayerNumber {
    One,
    Two,
    #[default]
    Three,
    Four,
    Five,
}

impl Display for PlayerNumber {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let number = match self {
            PlayerNumber::One => "1",
            PlayerNumber::Two => "2",
            PlayerNumber::Three => "3",
            PlayerNumber::Four => "4",
            PlayerNumber::Five => "5",
        };

        write!(formatter, "{number}")
    }
}

impl Repository {
    pub async fn configure_player_number(
        &self,
        robot_id: &str,
        player_number: PlayerNumber,
    ) -> Result<()> {
        let robot_parameter_directory = self.root.join("etc/parameters/ros_z/robot").join(robot_id);
        create_dir_all(&robot_parameter_directory)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to create parameter directory {}",
                    robot_parameter_directory.display()
                )
            })?;

        let global_parameters_path = robot_parameter_directory.join("global.json5");
        let mut parameters = match read_to_string(&global_parameters_path).await {
            Ok(contents) => json5::from_str::<Value>(&contents).wrap_err_with(|| {
                format!(
                    "failed to parse existing global parameters in {}",
                    global_parameters_path.display()
                )
            })?,
            Err(error) if error.kind() == ErrorKind::NotFound => json!({}),
            Err(error) => {
                return Err(error).wrap_err_with(|| {
                    format!(
                        "failed to read global parameters from {}",
                        global_parameters_path.display()
                    )
                });
            }
        };

        parameters["player_number"] =
            serde_json::to_value(player_number).wrap_err("failed to serialize player number")?;

        let contents =
            to_string_pretty(&parameters).wrap_err("failed to serialize global parameters")? + "\n";
        write(&global_parameters_path, contents)
            .await
            .wrap_err_with(|| {
                format!(
                    "failed to write global parameters to {}",
                    global_parameters_path.display()
                )
            })?;

        Ok(())
    }
}
