use std::{
    fmt,
    fs::read_to_string,
    path::{Path, PathBuf},
};

use serde::{
    de::{value::MapAccessDeserializer, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::{cyclers::CyclerKind, error::Error};

#[derive(Deserialize, Debug, Default)]
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

    pub fn cycler(mut self, cycler: CyclerManifest) -> Self {
        self.cyclers.push(cycler);

        self
    }
}

#[derive(Debug, Deserialize)]
pub struct CyclerManifest {
    pub name: String,
    pub kind: CyclerKind,
    pub instances: Vec<String>,
    pub setup_nodes: Vec<NodeSpecification>,
    pub nodes: Vec<NodeSpecification>,
}

impl CyclerManifest {
    pub fn new(name: &str, kind: CyclerKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
            instances: Vec::new(),
            setup_nodes: Vec::new(),
            nodes: Vec::new(),
        }
    }

    pub fn instance(mut self, name: String) -> Self {
        self.instances.push(name);
        self
    }

    pub fn setup_node(
        mut self,
        module: impl TryInto<NodeSpecification, Error = Error>,
    ) -> Result<Self, Error> {
        self.setup_nodes.push(module.try_into()?);
        Ok(self)
    }

    pub fn node(
        mut self,
        module: impl TryInto<NodeSpecification, Error = Error>,
    ) -> Result<Self, Error> {
        self.nodes.push(module.try_into()?);
        Ok(self)
    }
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "TomlNodeSpecification")]
pub struct NodeSpecification {
    pub module: syn::Path,
    pub path: PathBuf,
}

impl TryFrom<&str> for NodeSpecification {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let module: syn::Path = syn::parse_str(value).map_err(|_| Error::InvalidModulePath)?;
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
