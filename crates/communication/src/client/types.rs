use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::messages::Path;

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
    pub output: Path,
    pub data: Value,
}
