use std::fmt;

use serde::{
    de::{self, value::MapAccessDeserializer, MapAccess, Visitor},
    Deserialize, Deserializer,
};
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

#[derive(Debug, Deserialize)]
#[serde(from = "TomlNodeConfiguration")]
pub struct NodeConfiguration {
    pub module: Path,
    pub is_setup: bool,
}

impl From<TomlNodeConfiguration> for NodeConfiguration {
    fn from(value: TomlNodeConfiguration) -> Self {
        match value {
            TomlNodeConfiguration::Simple(module) => Self {
                module,
                is_setup: false,
            },
            TomlNodeConfiguration::Detailed(DetailedNodeConfiguration { module, is_setup }) => {
                Self { module, is_setup }
            }
        }
    }
}

enum TomlNodeConfiguration {
    Simple(Path),
    Detailed(DetailedNodeConfiguration),
}

impl<'de> Deserialize<'de> for TomlNodeConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TomlNodeConfigurationVisitor;

        impl<'de> Visitor<'de> for TomlNodeConfigurationVisitor {
            type Value = TomlNodeConfiguration;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(
                    "a module path string like \"ball_detection\" or a \
                     detailed node description like { module = \"ball_detection\" }",
                )
            }

            fn visit_str<E>(self, path: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let module = syn::parse_str(path).map_err(de::Error::custom)?;
                Ok(TomlNodeConfiguration::Simple(module))
            }

            fn visit_map<V>(self, map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let deserializer = MapAccessDeserializer::new(map);
                DetailedNodeConfiguration::deserialize(deserializer)
                    .map(TomlNodeConfiguration::Detailed)
            }
        }
        deserializer.deserialize_any(TomlNodeConfigurationVisitor)
    }
}

#[derive(Deserialize)]
struct DetailedNodeConfiguration {
    #[serde(deserialize_with = "module_path")]
    pub module: Path,
    #[serde(default)]
    pub is_setup: bool,
}
