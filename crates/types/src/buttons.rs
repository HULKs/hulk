use std::ops::{Index, IndexMut};

use booster::ButtonEventType;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect, Debug,
)]
pub enum ButtonPressType {
    Short,
    Long,
}

impl ButtonPressType {
    pub fn from_button_event_types(
        last_button_event_type: &Option<ButtonEventType>,
        current_button_event_type: &ButtonEventType,
    ) -> Option<Self> {
        match (last_button_event_type, current_button_event_type) {
            (Some(ButtonEventType::LongPressEnd), ButtonEventType::PressUp) => Some(Self::Long),
            (_, ButtonEventType::PressUp) => Some(Self::Short),
            (_, _) => None,
        }
    }
}

#[derive(
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Debug,
)]
pub struct Buttons<T> {
    pub f1: T,
    pub stand: T,
    pub walking: T,
}

impl<T> Index<i32> for Buttons<T> {
    type Output = T;

    fn index(&self, index: i32) -> &T {
        match index {
            0 => &self.f1,
            1 => &self.stand,
            2 => &self.walking,
            _ => panic!("out of bounds: {index}"),
        }
    }
}

impl<T> IndexMut<i32> for Buttons<T> {
    fn index_mut(&mut self, index: i32) -> &mut Self::Output {
        match index {
            0 => &mut self.f1,
            1 => &mut self.stand,
            2 => &mut self.walking,
            _ => panic!("out of bounds: {index}"),
        }
    }
}
