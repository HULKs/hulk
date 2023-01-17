use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    str::FromStr,
};

use color_eyre::{
    eyre::{bail, eyre},
    Report, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CyclerOutput {
    pub cycler: Cycler,
    pub output: Output,
}

impl FromStr for CyclerOutput {
    type Err = Report;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let (cycler_str, output_str) = string.split_once('.').ok_or_else(|| {
            eyre!("expected '.' in subscription path (e.g. 'control.main.foo_bar')")
        })?;
        let cycler = Cycler::from_str(cycler_str)?;
        let (output_str, path) = output_str.split_once('.').ok_or_else(|| {
            eyre!("expected '.' after output source (e.g. 'control.main.foo_bar')")
        })?;
        let output = match output_str {
            "main" | "main_outputs" => Output::Main {
                path: path.to_string(),
            },
            "additional" | "additional_outputs" => Output::Additional {
                path: path.to_string(),
            },
            _ => bail!("unknown output '{output_str}'"),
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

impl Display for Cycler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cycler::Control => f.write_str("Control"),
            Cycler::VisionTop => f.write_str("VisionTop"),
            Cycler::VisionBottom => f.write_str("VisionBottom"),
        }
    }
}

impl FromStr for Cycler {
    type Err = Report;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(match string {
            "Control" => Cycler::Control,
            "VisionTop" => Cycler::VisionTop,
            "VisionBottom" => Cycler::VisionBottom,
            _ => bail!("unknown cycler '{string}'"),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Output {
    Main { path: String },
    Additional { path: String },
}

#[derive(Debug, Clone)]
pub enum SubscriberMessage {
    UpdateBinary { data: Vec<u8> },
    Update { value: Value },
    SubscriptionSuccess,
    SubscriptionFailure { info: String },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum HierarchyType {
    Primary {
        name: String,
    },
    Struct {
        fields: BTreeMap<String, HierarchyType>,
    },
    GenericStruct,
    GenericEnum,
    Option {
        nested: Box<HierarchyType>,
    },
    Vec {
        nested: Box<HierarchyType>,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct CyclerOutputsHierarchy {
    pub main: HierarchyType,
    pub additional: HierarchyType,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OutputHierarchy {
    pub control: CyclerOutputsHierarchy,
    pub vision_top: CyclerOutputsHierarchy,
    pub vision_bottom: CyclerOutputsHierarchy,
}

#[derive(Debug, Deserialize)]
pub struct SubscribedOutput {
    pub output: Output,
    pub data: Value,
}
