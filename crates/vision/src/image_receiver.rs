use std::sync::Arc;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{hardware::Interface, CameraPosition, Rgb, YCbCr444};

pub struct ImageReceiver {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub image: MainOutput<Arc<bool>>,
    pub average_color: MainOutput<Rgb>,
}

impl ImageReceiver {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let image = context
            .hardware_interface
            .read_from_camera(CameraPosition::Top)?;
        let mut red = 0;
        let mut green = 0;
        let mut blue = 0;
        for y in 0..image.height {
            for x in 0..image.width_422 {
                let pixel = image.buffer[(y * image.width_422 + x) as usize];
                let left_color: Rgb = YCbCr444 {
                    y: pixel.y1,
                    cb: pixel.cb,
                    cr: pixel.cr,
                }
                .into();
                red += left_color.r as u32;
                green += left_color.g as u32;
                blue += left_color.b as u32;
                let right_color: Rgb = YCbCr444 {
                    y: pixel.y2,
                    cb: pixel.cb,
                    cr: pixel.cr,
                }
                .into();
                red += right_color.r as u32;
                green += right_color.g as u32;
                blue += right_color.b as u32;
            }
        }
        Ok(MainOutputs {
            image: Default::default(),
            average_color: Rgb::new(
                (red / (2 * image.buffer.len()) as u32) as u8,
                (green / (2 * image.buffer.len()) as u32) as u8,
                (blue / (2 * image.buffer.len()) as u32) as u8,
            )
            .into(),
        })
    }
}
