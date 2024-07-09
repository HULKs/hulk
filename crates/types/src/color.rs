use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
#[repr(C)]
pub struct YCbCr422 {
    pub y1: u8,
    pub cb: u8,
    pub y2: u8,
    pub cr: u8,
}

impl YCbCr422 {
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

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct YCbCr444 {
    pub y: u8,
    pub cb: u8,
    pub cr: u8,
}

impl YCbCr444 {
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
            y: (0.299 * (rgb.red as f32) + 0.587 * (rgb.green as f32) + 0.114 * (rgb.blue as f32))
                .clamp(0.0, 255.0) as u8,
            cb: (128.0 - 0.168736 * (rgb.red as f32) - 0.331264 * (rgb.green as f32)
                + 0.5 * (rgb.blue as f32))
                .clamp(0.0, 255.0) as u8,
            cr: (128.0 + 0.5 * (rgb.red as f32)
                - 0.418688 * (rgb.green as f32)
                - 0.081312 * (rgb.blue as f32))
                .clamp(0.0, 255.0) as u8,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Intensity {
    Low,
    High,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
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

    pub fn convert_to_rgchromaticity(&self) -> RgChromaticity {
        let sum = (self.red + self.green + self.blue) as f32;
        let mut chromaticity = RgChromaticity {
            red: 0.0,
            green: 0.0,
        };
        if sum != 0.0 {
            chromaticity.red = (self.red as f32) / sum;
            chromaticity.green = (self.green as f32) / sum;
        }
        chromaticity
    }

    pub fn get_luminance(&self) -> u8 {
        (0.299 * (self.red as f32) + 0.587 * (self.green as f32) + 0.114 * (self.blue as f32)) as u8
    }
}

impl From<YCbCr422> for Rgb {
    fn from(ycbcr422: YCbCr422) -> Self {
        let y = ycbcr422.averaged_y();
        let centered_cb = ycbcr422.cb as f32 - 128.0;
        let centered_cr = ycbcr422.cr as f32 - 128.0;
        Rgb {
            red: ((y as f32 + 1.40200 * centered_cr).round() as u8).clamp(0, 255),
            green: ((y as f32 - 0.34414 * centered_cb - 0.71414 * centered_cr).round() as u8)
                .clamp(0, 255),
            blue: ((y as f32 + 1.77200 * centered_cb).round() as u8).clamp(0, 255),
        }
    }
}

impl From<YCbCr444> for Rgb {
    fn from(ycbcr444: YCbCr444) -> Self {
        let y = ycbcr444.y;
        let centered_cb = ycbcr444.cb as f32 - 128.0;
        let centered_cr = ycbcr444.cr as f32 - 128.0;
        Rgb {
            red: ((y as f32 + 1.40200 * centered_cr).round() as u8).clamp(0, 255),
            green: ((y as f32 - 0.34414 * centered_cb - 0.71414 * centered_cr).round() as u8)
                .clamp(0, 255),
            blue: ((y as f32 + 1.77200 * centered_cb).round() as u8).clamp(0, 255),
        }
    }
}

pub struct RgChromaticity {
    pub red: f32,
    pub green: f32,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub struct Hsv {
    pub h: u16,
    pub s: u8,
    pub v: u8,
}

impl From<Rgb> for Hsv {
    fn from(value: Rgb) -> Self {
        const HUE_DEGREE: i32 = 512;
        let (r, g, b): (i32, i32, i32) = (value.red.into(), value.green.into(), value.blue.into());
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let (h, s) = if delta == 0 {
            (0, 0)
        } else {
            let h = if r == max {
                ((g - b) * 60 * HUE_DEGREE) / delta
            } else if g == max {
                ((b - r) * 60 * HUE_DEGREE) / delta + 120 * HUE_DEGREE
            } else if b == max {
                ((r - g) * 60 * HUE_DEGREE) / delta + 240 * HUE_DEGREE
            } else {
                0
            };
            let h = if h < 0 { h + 360 * HUE_DEGREE } else { h };
            let s = (256 * delta - 8) / max;

            (h / HUE_DEGREE, s)
        };

        Hsv {
            h: h as u16,
            s: s as u8,
            v: max as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_000_rgb() {
        let rgb = Rgb {
            red: 0,
            green: 0,
            blue: 0,
        };
        assert_eq!(rgb.red, 0);
        assert_eq!(rgb.green, 0);
        assert_eq!(rgb.blue, 0);
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
                red: 0,
                green: 99,
                blue: 193,
            }
        );

        let ycbcr = YCbCr422 {
            y1: 0,
            cb: 128,
            y2: 0,
            cr: 128,
        };
        let rgb = Rgb::from(ycbcr);
        assert_eq!(
            rgb,
            Rgb {
                red: 0,
                green: 0,
                blue: 0
            }
        );

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
                red: 255,
                green: 255,
                blue: 255,
            }
        );

        let ycbcr = YCbCr422 {
            y1: 128,
            cb: 0,
            y2: 128,
            cr: 0,
        };
        let rgb = Rgb::from(ycbcr);
        assert_eq!(
            rgb,
            Rgb {
                red: 0,
                green: 255,
                blue: 0
            }
        );
    }

    #[test]
    fn convert_from_rgb_to_ycbcr444() {
        let rgb = Rgb {
            red: 255,
            green: 0,
            blue: 0,
        };
        let ycbcr444 = YCbCr444::from(rgb);
        assert_eq!(
            ycbcr444,
            YCbCr444 {
                y: 76,
                cb: 84,
                cr: 255,
            }
        );

        let rgb = Rgb {
            red: 0,
            green: 255,
            blue: 0,
        };
        let ycbcr444 = YCbCr444::from(rgb);
        assert_eq!(
            ycbcr444,
            YCbCr444 {
                y: 149,
                cb: 43,
                cr: 21,
            }
        );

        let rgb = Rgb {
            red: 0,
            green: 0,
            blue: 255,
        };
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
            red: 255,
            green: 255,
            blue: 0,
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
            red: 146,
            green: 14,
            blue: 43,
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
            red: 30,
            green: 70,
            blue: 200,
        };
        let chromaticity = rgb.convert_to_rgchromaticity().red;
        assert_eq!(chromaticity, 0.1);
    }

    #[test]
    fn rgb_hsv_conversion() {
        for (rgb, hsv) in [
            (
                Rgb {
                    red: 0,
                    green: 0,
                    blue: 0,
                },
                Hsv { h: 0, s: 0, v: 0 },
            ),
            (
                Rgb {
                    red: 255,
                    green: 255,
                    blue: 255,
                },
                Hsv { h: 0, s: 0, v: 255 },
            ),
            (
                Rgb {
                    red: 255,
                    green: 0,
                    blue: 0,
                },
                Hsv {
                    h: 0,
                    s: 255,
                    v: 255,
                },
            ),
            (
                Rgb {
                    red: 0,
                    green: 255,
                    blue: 0,
                },
                Hsv {
                    h: 120,
                    s: 255,
                    v: 255,
                },
            ),
            (
                Rgb {
                    red: 0,
                    green: 0,
                    blue: 255,
                },
                Hsv {
                    h: 240,
                    s: 255,
                    v: 255,
                },
            ),
            (
                Rgb {
                    red: 128,
                    green: 0,
                    blue: 255,
                },
                Hsv {
                    h: 270,
                    s: 255,
                    v: 255,
                },
            ),
        ] {
            let converted: Hsv = rgb.into();
            assert_eq!(converted, hsv);
        }
    }
}
