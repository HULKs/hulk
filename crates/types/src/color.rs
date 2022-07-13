use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct YCbCr422 {
    pub y1: u8,
    pub cb: u8,
    pub y2: u8,
    pub cr: u8,
}

impl YCbCr422 {
    #[allow(dead_code)]
    pub fn new(y1: u8, cb: u8, y2: u8, cr: u8) -> Self {
        Self { y1, cb, y2, cr }
    }

    pub fn averaged_y(&self) -> u8 {
        ((self.y1 as u16 + self.y2 as u16) / 2) as u8
    }
}

impl From<[YCbCr444; 2]> for YCbCr422 {
    fn from(ycbcr444: [YCbCr444; 2]) -> Self {
        let averaged_cb = ((ycbcr444[0].cb as u16 + ycbcr444[1].cb as u16) / 2) as u8;
        let averaged_cr = ((ycbcr444[0].cr as u16 + ycbcr444[1].cr as u16) / 2) as u8;

        Self {
            y1: ycbcr444[0].y,
            cb: averaged_cb,
            y2: ycbcr444[1].y,
            cr: averaged_cr,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct YCbCr444 {
    pub y: u8,
    pub cb: u8,
    pub cr: u8,
}

impl YCbCr444 {
    #[allow(dead_code)]
    pub fn new(y: u8, cb: u8, cr: u8) -> Self {
        Self { y, cb, cr }
    }
}

impl From<YCbCr422> for [YCbCr444; 2] {
    fn from(color: YCbCr422) -> Self {
        [
            YCbCr444 {
                y: color.y1,
                cb: color.cb,
                cr: color.cr,
            },
            YCbCr444 {
                y: color.y2,
                cb: color.cb,
                cr: color.cr,
            },
        ]
    }
}

impl From<YCbCr422> for YCbCr444 {
    fn from(color: YCbCr422) -> Self {
        YCbCr444 {
            y: color.y1,
            cb: color.cb,
            cr: color.cr,
        }
    }
}

impl From<Rgb> for YCbCr444 {
    fn from(rgb: Rgb) -> Self {
        // RGB to YCbCr conversion
        // Conversion factors from https://de.wikipedia.org/wiki/YCbCr-Farbmodell#Umrechnung_zwischen_RGB_und_YCbCr

        Self {
            y: (0.299 * (rgb.r as f32) + 0.587 * (rgb.g as f32) + 0.114 * (rgb.b as f32))
                .clamp(0.0, 255.0) as u8,
            cb: (128.0 - 0.168736 * (rgb.r as f32) - 0.331264 * (rgb.g as f32)
                + 0.5 * (rgb.b as f32))
                .clamp(0.0, 255.0) as u8,
            cr: (128.0 + 0.5 * (rgb.r as f32)
                - 0.418688 * (rgb.g as f32)
                - 0.081312 * (rgb.b as f32))
                .clamp(0.0, 255.0) as u8,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RgbChannel {
    Red,
    Green,
    Blue,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum Intensity {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize, SerializeHierarchy)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const RED: Rgb = Rgb::new(255, 0, 0);
    pub const GREEN: Rgb = Rgb::new(0, 255, 0);
    pub const BLUE: Rgb = Rgb::new(0, 0, 255);
    pub const YELLOW: Rgb = Rgb::new(255, 220, 0);
    pub const PURPLE: Rgb = Rgb::new(255, 0, 255);
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn get_chromaticity(&self, channel: RgbChannel) -> f32 {
        let sum = self.r as f32 + self.g as f32 + self.b as f32;
        if sum == 0.0 {
            return 0.0;
        }
        let value = match channel {
            RgbChannel::Red => self.r,
            RgbChannel::Green => self.g,
            RgbChannel::Blue => self.b,
        } as f32;
        value / sum
    }
}

impl From<YCbCr422> for Rgb {
    fn from(ycbcr422: YCbCr422) -> Self {
        let y = ycbcr422.averaged_y();
        let centered_cb = ycbcr422.cb as f32 - 128.0;
        let centered_cr = ycbcr422.cr as f32 - 128.0;
        Rgb {
            r: ((y as f32 + 1.40200 * centered_cr).round() as u8).clamp(0, 255),
            g: ((y as f32 - 0.34414 * centered_cb - 0.71414 * centered_cr).round() as u8)
                .clamp(0, 255),
            b: ((y as f32 + 1.77200 * centered_cb).round() as u8).clamp(0, 255),
        }
    }
}

impl From<YCbCr444> for Rgb {
    fn from(ycbcr444: YCbCr444) -> Self {
        let y = ycbcr444.y;
        let centered_cb = ycbcr444.cb as f32 - 128.0;
        let centered_cr = ycbcr444.cr as f32 - 128.0;
        Rgb {
            r: ((y as f32 + 1.40200 * centered_cr).round() as u8).clamp(0, 255),
            g: ((y as f32 - 0.34414 * centered_cb - 0.71414 * centered_cr).round() as u8)
                .clamp(0, 255),
            b: ((y as f32 + 1.77200 * centered_cb).round() as u8).clamp(0, 255),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_000_rgb() {
        let rgb = Rgb { r: 0, g: 0, b: 0 };
        assert_eq!(rgb.r, 0);
        assert_eq!(rgb.g, 0);
        assert_eq!(rgb.b, 0);
    }

    #[test]
    fn compute_averaged_y() {
        let ycbcr = YCbCr422 {
            y1: 100,
            cb: 200,
            y2: 200,
            cr: 46,
        };
        let averaged_y = ycbcr.averaged_y();
        assert_eq!(averaged_y, 150);
    }

    #[test]
    fn convert_from_ycbcr_to_rgb() {
        let ycbcr = YCbCr422 {
            y1: 100,
            cb: 200,
            y2: 30,
            cr: 46,
        };
        let rgb = Rgb::from(ycbcr);
        assert_eq!(
            rgb,
            Rgb {
                r: 0,
                g: 99,
                b: 193,
            }
        );

        let ycbcr = YCbCr422 {
            y1: 0,
            cb: 128,
            y2: 0,
            cr: 128,
        };
        let rgb = Rgb::from(ycbcr);
        assert_eq!(rgb, Rgb { r: 0, g: 0, b: 0 });

        let ycbcr = YCbCr422 {
            y1: 255,
            cb: 128,
            y2: 255,
            cr: 128,
        };
        let rgb = Rgb::from(ycbcr);
        assert_eq!(
            rgb,
            Rgb {
                r: 255,
                g: 255,
                b: 255,
            }
        );

        let ycbcr = YCbCr422 {
            y1: 128,
            cb: 0,
            y2: 128,
            cr: 0,
        };
        let rgb = Rgb::from(ycbcr);
        assert_eq!(rgb, Rgb { r: 0, g: 255, b: 0 });
    }

    #[test]
    fn convert_from_rgb_to_ycbcr444() {
        let rgb = Rgb { r: 255, g: 0, b: 0 };
        let ycbcr444 = YCbCr444::from(rgb);
        assert_eq!(
            ycbcr444,
            YCbCr444 {
                y: 76,
                cb: 84,
                cr: 255,
            }
        );

        let rgb = Rgb { r: 0, g: 255, b: 0 };
        let ycbcr444 = YCbCr444::from(rgb);
        assert_eq!(
            ycbcr444,
            YCbCr444 {
                y: 149,
                cb: 43,
                cr: 21,
            }
        );

        let rgb = Rgb { r: 0, g: 0, b: 255 };
        let ycbcr444 = YCbCr444::from(rgb);
        assert_eq!(
            ycbcr444,
            YCbCr444 {
                y: 29,
                cb: 255,
                cr: 107,
            }
        );

        let rgb = Rgb {
            r: 255,
            g: 255,
            b: 0,
        };
        let ycbcr444 = YCbCr444::from(rgb);
        assert_eq!(
            ycbcr444,
            YCbCr444 {
                y: 225,
                cb: 0,
                cr: 148,
            }
        );

        let rgb = Rgb {
            r: 146,
            g: 14,
            b: 43,
        };
        let ycbcr444 = YCbCr444::from(rgb);
        assert_eq!(
            ycbcr444,
            YCbCr444 {
                y: 56,
                cb: 120,
                cr: 191,
            }
        );
    }

    #[test]
    fn convert_from_ycbcr444_to_ycbcr422() {
        let ycbcr444 = [
            YCbCr444 {
                y: 12,
                cb: 137,
                cr: 122,
            },
            YCbCr444 {
                y: 250,
                cb: 137,
                cr: 122,
            },
        ];
        let ycbcr422 = YCbCr422::from(ycbcr444);
        assert_eq!(
            ycbcr422,
            YCbCr422 {
                y1: 12,
                y2: 250,
                cb: 137,
                cr: 122,
            }
        );

        let ycbcr444 = [
            YCbCr444 {
                y: 12,
                cb: 84,
                cr: 235,
            },
            YCbCr444 {
                y: 250,
                cb: 137,
                cr: 122,
            },
        ];
        let ycbcr422 = YCbCr422::from(ycbcr444);
        assert_eq!(
            ycbcr422,
            YCbCr422 {
                y1: 12,
                y2: 250,
                cb: 110,
                cr: 178,
            }
        );
    }

    #[test]
    fn calculate_red_chromaticity() {
        let rgb = Rgb {
            r: 30,
            g: 70,
            b: 200,
        };
        let chromaticity = rgb.get_chromaticity(RgbChannel::Red);
        assert_eq!(chromaticity, 0.1);
    }
}
