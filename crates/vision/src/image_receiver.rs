use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{
    hardware::{Image, Interface},
    CameraPosition, Rgb, YCbCr444,
};

use crate::CyclerInstance;

pub struct ImageReceiver {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub hardware_interface: HardwareInterface,
    pub instance: CyclerInstance,
}

#[context]
pub struct MainOutputs {
    pub image: MainOutput<Image>,
    pub average_color: MainOutput<Rgb>,
}

impl ImageReceiver {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl Interface>) -> Result<MainOutputs> {
        let image = context
            .hardware_interface
            .read_from_camera(match context.instance {
                CyclerInstance::VisionTop => CameraPosition::Top,
                CyclerInstance::VisionBottom => CameraPosition::Bottom,
            })?;
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
        let amount_of_additions = 2 * image.buffer.len();
        Ok(MainOutputs {
            image: image.into(),
            average_color: Rgb::new(
                (red / amount_of_additions as u32) as u8,
                (green / amount_of_additions as u32) as u8,
                (blue / amount_of_additions as u32) as u8,
            )
            .into(),
        })
    }
}
