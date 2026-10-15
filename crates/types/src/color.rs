use ros_z::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Message)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const RED: Rgb = Rgb::new(255, 0, 0);
    pub const GREEN: Rgb = Rgb::new(0, 255, 0);
    pub const BLUE: Rgb = Rgb::new(0, 0, 255);
    pub const YELLOW: Rgb = Rgb::new(255, 220, 0);
    pub const PURPLE: Rgb = Rgb::new(255, 0, 255);
    pub const TURQUOISE: Rgb = Rgb::new(0, 255, 255);
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);
    pub const PINK: Rgb = Rgb::new(250, 45, 208);
    pub const ORANGE: Rgb = Rgb::new(255, 69, 0);

    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    pub fn luminance(&self) -> u8 {
        (0.299 * (self.red as f32) + 0.587 * (self.green as f32) + 0.114 * (self.blue as f32)) as u8
    }
}
