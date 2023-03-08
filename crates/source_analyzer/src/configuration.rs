use serde::{de, Deserialize, Deserializer};
use syn::Path;

use crate::cycler::CyclerKind;

#[derive(Deserialize, Debug)]
pub struct FrameworkConfiguration {
    pub cyclers: Vec<CyclerConfiguration>,
}

#[derive(Debug, Deserialize)]
pub struct CyclerConfiguration {
    pub name: String,
    pub kind: CyclerKind,
    pub instances: Option<Vec<String>>,
    pub module: String,
    pub nodes: Vec<NodeConfiguration>,
}

fn module_path<'de, D>(deserializer: D) -> Result<Path, D::Error>
where
    D: Deserializer<'de>,
{
    let path = <&str>::deserialize(deserializer)?;
    syn::parse_str(path).map_err(de::Error::custom)
}

#[derive(Deserialize, Debug)]
pub struct NodeConfiguration {
    #[serde(deserialize_with = "module_path")]
    pub module: Path,
    #[serde(default)]
    pub is_setup: bool,
}
