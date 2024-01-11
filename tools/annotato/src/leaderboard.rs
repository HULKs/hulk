use std::{
    io::Write,
    net::{SocketAddr, TcpStream},
};

use color_eyre::eyre::{ContextCompat, Result};
use serde::{Deserialize, Serialize};

use crate::user_toml::CONFIG;

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    username: String,
}

pub fn send_score_up() -> Result<()> {
    let config = CONFIG.get().wrap_err("could not find config file")?;
    if !config.leaderboard.enable {
        return Ok(());
    }

    let mut connection = TcpStream::connect(SocketAddr::from((
        config.leaderboard.host,
        config.leaderboard.port,
    )))?;

    let message = Message {
        username: config.leaderboard.githubname.clone(),
    };

    connection.write(serde_json::to_string(&message)?.as_bytes())?;

    Ok(())
}
