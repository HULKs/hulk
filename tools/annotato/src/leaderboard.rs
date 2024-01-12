use color_eyre::eyre::{ContextCompat, Result};

use crate::user_toml::CONFIG;

pub fn send_score_up() -> Result<()> {
    let config = CONFIG.get().wrap_err("could not find config file")?;
    if !config.leaderboard.enable {
        return Ok(());
    }

    if let Err(_error) = reqwest::blocking::get(format!(
        "http://{}/score/{}",
        config.leaderboard.host, config.leaderboard.githubname
    )) {
        eprintln!("Failed to send score up");
    }

    Ok(())
}
