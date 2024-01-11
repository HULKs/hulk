use std::{net::Ipv4Addr, path::PathBuf};

use eframe::egui::{Key, Modifiers, InputState};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

pub static CONFIG: OnceCell<ConfigFile> = OnceCell::new();

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub keybindings: KeyBindings,
    pub remote: Remote,
    pub leaderboard: LeaderBoard,
}

#[derive(Debug, Deserialize)]

pub struct KeyBindings {
    pub next: KeyBind,
    pub previous: KeyBind,
    pub zoom: KeyBind,
    pub edit: KeyBind,
    pub draw: KeyBind,
    pub abort: KeyBind,

    pub select_ball: Key,
    pub select_robot: Key,
    pub select_goalpost: Key,
    pub select_penaltyspot: Key,
    pub select_xspot: Key,
    pub select_lspot: Key,
    pub select_tspot: Key,
}

#[derive(Debug, Deserialize)]
pub struct KeyBind {
    pub primary: Key,
    #[serde(default)]
    pub extra: Vec<Key>,
    #[serde(deserialize_with = "deserialize_modifiers", default)]
    pub modifiers: Modifiers,
}

impl KeyBind {
    pub fn is_pressed(&self, input: &InputState) -> bool {
        let modifiers_pressed = input.modifiers == self.modifiers;
        let keys_pressed = [self.primary].iter().chain(self.extra.iter()).all(|&key| input.key_pressed(key));

        modifiers_pressed && keys_pressed
    }
}

fn deserialize_modifiers<'de, D>(deserializer: D) -> Result<Modifiers, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let string_modifiers = Vec::<String>::deserialize(deserializer)?;
    let mut modifiers = Modifiers::default();
    for modifier in string_modifiers {
        match modifier.as_str() {
            "alt" => modifiers = modifiers | Modifiers::ALT,
            "ctrl" => modifiers = modifiers | Modifiers::CTRL,
            "shift" => modifiers = modifiers | Modifiers::SHIFT,
            invalid => {
                return Err(serde::de::Error::custom(format!(
                    "invalid modifier: {invalid}"
                )))
            }
        }
    }
    Ok(modifiers)
}

#[derive(Debug, Deserialize)]
pub struct Remote {
    pub user: String,
    pub host: Ipv4Addr,
    pub folder: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LeaderBoard {
    pub enable: bool,
    pub githubname: String,
    pub host: Ipv4Addr,
    pub port: u16,
}
