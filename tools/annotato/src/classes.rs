use eframe::{
    egui::Key,
    epaint::{Color32, Rgba},
};
use serde::{Deserialize, Serialize};

use crate::widgets::class_selector::EnumIter;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Classes {
    Ball,
    Robot,
    GoalPost,
    PenaltySpot,
    X_Spot,
    L_Spot,
    T_Spot,
}

impl EnumIter for Classes {
    fn list() -> Vec<Self> {
        use Classes::*;
        vec![Ball, Robot, GoalPost, PenaltySpot, X_Spot, L_Spot, T_Spot]
    }
}

impl From<usize> for Classes {
    fn from(value: usize) -> Self {
        *Classes::list().get(value).unwrap()
    }
}
impl From<&Classes> for usize {
    fn from(value: &Classes) -> Self {
        Classes::list().iter().position(|&r| r == *value).unwrap()
    }
}

impl Classes {
    pub fn from_key(key: Key) -> Option<Classes> {
        match key {
            Key::Num1 => Some(Classes::Ball),
            Key::Num2 => Some(Classes::Robot),
            Key::Num3 => Some(Classes::GoalPost),
            Key::Num4 => Some(Classes::PenaltySpot),
            Key::Num5 => Some(Classes::X_Spot),
            Key::Num6 => Some(Classes::L_Spot),
            Key::Num7 => Some(Classes::T_Spot),
            _ => None,
        }
    }

    pub fn color(&self) -> Color32 {
        let color = match self {
            Classes::Robot => Color32::BLUE,
            Classes::Ball => Color32::LIGHT_RED,
            Classes::GoalPost => Color32::DARK_RED,
            Classes::PenaltySpot => Color32::GOLD,
            Classes::X_Spot => Color32::LIGHT_BLUE,
            Classes::L_Spot => Color32::BLACK,
            Classes::T_Spot => Color32::LIGHT_GREEN,
        };
        let [r, g, b, _] = color.to_normalized_gamma_f32();
        Rgba::from_rgba_unmultiplied(r, g, b, 0.05).into()
    }
}
