use eframe::{egui::Key, epaint::Color32};
use serde::{Deserialize, Serialize};

use crate::widgets::class_selector::EnumIter;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Class {
    Ball,
    Robot,
    GoalPost,
    PenaltySpot,
    XSpot,
    LSpot,
    TSpot,
}

impl EnumIter for Class {
    fn list() -> Vec<Self> {
        use Class::*;
        vec![Ball, Robot, GoalPost, PenaltySpot, XSpot, LSpot, TSpot]
    }
}

impl From<usize> for Class {
    fn from(value: usize) -> Self {
        *Class::list().get(value).unwrap()
    }
}
impl From<&Class> for usize {
    fn from(value: &Class) -> Self {
        Class::list().iter().position(|&r| r == *value).unwrap()
    }
}

impl Class {
    pub fn from_key(key: Key) -> Option<Class> {
        match key {
            Key::Num1 => Some(Class::Ball),
            Key::Num2 => Some(Class::Robot),
            Key::Num3 => Some(Class::GoalPost),
            Key::Num4 => Some(Class::PenaltySpot),
            Key::Num5 => Some(Class::XSpot),
            Key::Num6 => Some(Class::LSpot),
            Key::Num7 => Some(Class::TSpot),
            _ => None,
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Class::Robot => Color32::BLUE,
            Class::Ball => Color32::LIGHT_RED,
            Class::GoalPost => Color32::DARK_RED,
            Class::PenaltySpot => Color32::GOLD,
            Class::XSpot => Color32::LIGHT_BLUE,
            Class::LSpot => Color32::BLACK,
            Class::TSpot => Color32::LIGHT_GREEN,
        }
    }
}
