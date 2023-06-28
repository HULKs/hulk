use std::{
    fmt,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use serde::{
    de::{value::MapAccessDeserializer, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{cycler::CyclerKind, error::Error};

#[derive(Deserialize, Debug)]
pub struct FrameworkManifest {
    pub cyclers: Vec<CyclerManifest>,
}

impl FrameworkManifest {
    pub fn try_from_toml(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        toml::from_str(&read_to_string(path).map_err(|error| Error::Io {
            source: error,
            path: path.to_path_buf(),
        })?)
        .map_err(|error| Error::ConfigurationParsing {
            source: error,
            path: path.to_path_buf(),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CyclerManifest {
    pub name: String,
    pub kind: CyclerKind,
    pub instances: Option<Vec<String>>,
    pub setup_nodes: Vec<NodeSpecification>,
    pub nodes: Vec<NodeSpecification>,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "TomlNodeSpecification")]
pub struct NodeSpecification {
    pub module: syn::Path,
    pub path: PathBuf,
}

impl TryFrom<TomlNodeSpecification> for NodeSpecification {
    type Error = Error;

    fn try_from(toml: TomlNodeSpecification) -> Result<Self, Self::Error> {
        match toml {
            TomlNodeSpecification::Simple(module) => {
                let path_segments: Vec<_> = module
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect();
                let (crate_name, path_segments) = path_segments
                    .split_first()
                    .ok_or(Error::InvalidModulePath)?;
                let path_to_module = path_segments.join("/");
                let path = format!("{crate_name}/src/{path_to_module}.rs");
                Ok(Self {
                    module,
                    path: path.into(),
                })
            }
            TomlNodeSpecification::Detailed(DetailedNodeSpecification { module, path }) => {
                Ok(Self { module, path })
            }
        }
    }
}

enum TomlNodeSpecification {
    Simple(syn::Path),
    Detailed(DetailedNodeSpecification),
}

impl<'de> Deserialize<'de> for TomlNodeSpecification {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TomlNodeSpecificationVisitor;

        impl<'de> Visitor<'de> for TomlNodeSpecificationVisitor {
            type Value = TomlNodeSpecification;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(
                    "a module path string like \"ball_detection\" or a \
                     detailed node description like { module = \"ball_detection\" }",
                )
            }

            fn visit_str<E>(self, path: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let module = syn::parse_str(path).map_err(E::custom)?;
                Ok(TomlNodeSpecification::Simple(module))
            }

            fn visit_map<V>(self, map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let deserializer = MapAccessDeserializer::new(map);
                DetailedNodeSpecification::deserialize(deserializer)
                    .map(TomlNodeSpecification::Detailed)
            }
        }
        deserializer.deserialize_any(TomlNodeSpecificationVisitor)
    }
}

#[derive(Deserialize)]
struct DetailedNodeSpecification {
    #[serde(deserialize_with = "deserialize_syn_path")]
    module: syn::Path,
    path: PathBuf,
}

fn deserialize_syn_path<'de, D>(deserializer: D) -> Result<syn::Path, D::Error>
where
    D: Deserializer<'de>,
{
    let path = <&str>::deserialize(deserializer)?;
    syn::parse_str(path).map_err(serde::de::Error::custom)
}
