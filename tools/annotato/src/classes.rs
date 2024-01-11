use eframe::{egui::Key, epaint::Color32};
use serde::{Deserialize, Serialize};

use crate::{user_toml::CONFIG, widgets::class_selector::EnumIter};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Class {
    Ball,
    Robot,
    GoalPost,
    PenaltySpot,
    LSpot,
    TSpot,
    XSpot,
}

impl EnumIter for Class {
    fn list() -> Vec<Self> {
        use Class::*;
        vec![Ball, Robot, GoalPost, PenaltySpot, LSpot, TSpot, XSpot]
    }
}

impl Class {
    pub fn from_key(key: Key) -> Option<Class> {
        let keybindings = &CONFIG.get().unwrap().keybindings;
        if key == keybindings.select_ball {
            return Some(Class::Ball);
        }
        match key {
            x if x == keybindings.select_ball => Some(Class::Ball),
            x if x == keybindings.select_robot => Some(Class::Robot),
            x if x == keybindings.select_goalpost => Some(Class::GoalPost),
            x if x == keybindings.select_penaltyspot => Some(Class::PenaltySpot),
            x if x == keybindings.select_lspot => Some(Class::LSpot),
            x if x == keybindings.select_tspot => Some(Class::TSpot),
            x if x == keybindings.select_xspot => Some(Class::XSpot),
            _ => None,
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Class::Robot => Color32::BLUE,
            Class::Ball => Color32::LIGHT_RED,
            Class::GoalPost => Color32::DARK_RED,
            Class::PenaltySpot => Color32::GOLD,
            Class::LSpot => Color32::BLACK,
            Class::TSpot => Color32::LIGHT_GREEN,
            Class::XSpot => Color32::LIGHT_BLUE,
        }
    }
}
