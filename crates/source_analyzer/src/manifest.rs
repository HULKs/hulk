use serde::Deserialize;

use crate::cyclers::CyclerKind;

#[derive(Debug, Default)]
pub struct FrameworkManifest {
    pub cyclers: Vec<CyclerManifest>,
}

#[derive(Debug, Deserialize)]
pub struct CyclerManifest {
    pub name: &'static str,
    pub kind: CyclerKind,
    pub instances: Vec<&'static str>,
    pub setup_nodes: Vec<&'static str>,
    pub nodes: Vec<&'static str>,
}
