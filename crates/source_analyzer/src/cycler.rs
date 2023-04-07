use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    fs::read_to_string,
    path::Path,
};

use itertools::Itertools;
use serde::Deserialize;
use topological_sort::TopologicalSort;

use crate::{
    configuration::{CyclerConfiguration, FrameworkConfiguration},
    contexts::Field,
    error::Error,
    node::Node,
};

pub type CyclerName = String;
pub type InstanceName = String;
pub type ModulePath = String;

#[derive(Debug)]
pub struct Instance {
    pub name: InstanceName,
}

impl Display for Instance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum CyclerKind {
    Perception,
    RealTime,
}

#[derive(Debug)]
pub struct Cycler {
    pub name: CyclerName,
    pub kind: CyclerKind,
    pub instances: Vec<Instance>,
    pub module: ModulePath,
    pub nodes: Vec<Node>,
}

impl Display for Cycler {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let instances = self.instances.iter().map(ToString::to_string).join(", ");
        writeln!(f, "{} ({:?}) [{instances}]", self.name, self.kind)?;
        for node in &self.nodes {
            writeln!(f, "  {node}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Cyclers {
    pub cyclers: Vec<Cycler>,
}

impl Display for Cyclers {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for cycler in &self.cyclers {
            writeln!(f, "{cycler}")?;
        }
        Ok(())
    }
}

impl Cyclers {
    pub fn try_from_toml(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let configuration: FrameworkConfiguration =
            toml::from_str(&read_to_string(path).map_err(|error| Error::Io {
                source: error,
                path: path.to_path_buf(),
            })?)
            .map_err(|error| Error::ConfigurationParsing {
                source: error,
                path: path.to_path_buf(),
            })?;
        let root = path.parent().unwrap();
        Cyclers::try_from_configurations(&configuration.cyclers, root)
    }

    pub fn try_from_configurations<'config>(
        values: impl IntoIterator<Item = &'config CyclerConfiguration>,
        root: &Path,
    ) -> Result<Self, Error> {
        let cyclers = values
            .into_iter()
            .map(|configuration| Self::try_from_configuration(configuration, root))
            .collect::<Result<_, _>>()?;
        Ok(Self { cyclers })
    }

    fn try_from_configuration(
        cycler_config: &CyclerConfiguration,
        root: &Path,
    ) -> Result<Cycler, Error> {
        let instance_names = cycler_config
            .instances
            .clone()
            .unwrap_or_else(|| vec![String::new()]);
        let instances = instance_names
            .iter()
            .map(|instance_name| Instance {
                name: format!("{}{}", cycler_config.name, instance_name),
            })
            .collect();
        let nodes = cycler_config
            .nodes
            .iter()
            .map(|node_config| {
                Node::try_from_configuration(&cycler_config.module, node_config, root)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let output_to_node: HashMap<_, _> = nodes
            .iter()
            .flat_map(|node| {
                node.contexts
                    .main_outputs
                    .iter()
                    .filter_map(move |field| match field {
                        Field::MainOutput { name, .. } => Some((name.to_string(), node)),
                        _ => None,
                    })
            })
            .collect();
        let mut topological_sort = TopologicalSort::<&Node>::new();
        for node in nodes.iter() {
            topological_sort.insert(node);
            for dependency in node
                .contexts
                .cycle_context
                .iter()
                .filter_map(|field| match field {
                    Field::HistoricInput { path, .. }
                    | Field::Input {
                        path,
                        cycler_instance: None,
                        ..
                    }
                    | Field::RequiredInput {
                        path,
                        cycler_instance: None,
                        ..
                    } => {
                        let first_segment = path.segments.first()?;
                        Some(first_segment.name.as_str())
                    }
                    _ => None,
                })
            {
                let producing_node = match output_to_node.get(dependency) {
                    Some(node) => node,
                    None => {
                        return Err(Error::MissingOutput {
                            node: node.name.clone(),
                            output: dependency.to_string(),
                        })
                    }
                };
                if node.is_setup && !producing_node.is_setup {
                    return Err(Error::SetupNodeDependency {
                        depending_node: node.name.clone(),
                        output: dependency.to_string(),
                        producing_node: producing_node.name.clone(),
                    });
                }
                topological_sort.add_dependency(*producing_node, node);
            }
        }

        let mut sorted_nodes = Vec::new();
        while let Some(node) = topological_sort.pop() {
            sorted_nodes.push(node.clone());
        }
        if !topological_sort.is_empty() {
            return Err(Error::CircularDependency);
        }

        let cycler = Cycler {
            name: cycler_config.name.clone(),
            kind: cycler_config.kind,
            instances,
            module: cycler_config.module.clone(),
            nodes: sorted_nodes,
        };
        Ok(cycler)
    }

    pub fn number_of_instances(&self) -> usize {
        self.cyclers
            .iter()
            .map(|cycler| cycler.instances.len())
            .sum()
    }

    pub fn instances(&self) -> impl Iterator<Item = (&Cycler, &Instance)> {
        self.cyclers.iter().flat_map(move |cycler| {
            cycler
                .instances
                .iter()
                .map(move |instance| (cycler, instance))
        })
    }

    pub fn instances_with(&self, kind: CyclerKind) -> impl Iterator<Item = (&Cycler, &Instance)> {
        self.cyclers
            .iter()
            .filter(move |cycler| cycler.kind == kind)
            .flat_map(move |cycler| {
                cycler
                    .instances
                    .iter()
                    .map(move |instance| (cycler, instance))
            })
    }
}
