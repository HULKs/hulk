use std::str::FromStr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CyclerOutput {
    pub cycler: Cycler,
    pub output: Output,
}

impl FromStr for CyclerOutput {
    type Err = anyhow::Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let (cycler_str, output_str) = string.split_once('.').ok_or_else(|| {
            anyhow!("Expected '.' in subscription path (e.g. 'control.main.foo_bar')")
        })?;
        let cycler = match cycler_str {
            "control" => Cycler::Control,
            "vision_top" => Cycler::VisionTop,
            "vision_bottom" => Cycler::VisionBottom,
            _ => anyhow::bail!("Unknown cycler '{cycler_str}'"),
        };
        let (output_str, path) = output_str.split_once('.').ok_or_else(|| {
            anyhow!("Expected '.' after output source (e.g. 'control.main.foo_bar')")
        })?;
        let output = match output_str {
            "main" | "main_outputs" => Output::Main {
                path: path.to_string(),
            },
            "additional" | "additional_outputs" => Output::Additional {
                path: path.to_string(),
            },
            "image" => anyhow::bail!("Image type not supported"),
            _ => anyhow::bail!("Unknown output '{output_str}'"),
        };
        Ok(CyclerOutput { cycler, output })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Cycler {
    Control,
    VisionTop,
    VisionBottom,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Output {
    Main { path: String },
    Additional { path: String },
    Image,
}
