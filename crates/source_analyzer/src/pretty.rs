use std::fmt::{self, Write};

use crate::{
    contexts::{Contexts, Field},
    cyclers::{Cycler, Cyclers, Instance},
    node::Node,
};

pub fn to_string_pretty(value: &impl ToWriterPretty) -> Result<String, fmt::Error> {
    let mut string = String::new();
    value.to_writer_pretty(&mut string)?;
    Ok(string)
}

pub trait ToWriterPretty {
    fn to_writer_pretty(&self, f: &mut impl Write) -> fmt::Result;
}

impl ToWriterPretty for Instance {
    fn to_writer_pretty(&self, f: &mut impl Write) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl ToWriterPretty for Cycler {
    fn to_writer_pretty(&self, f: &mut impl Write) -> fmt::Result {
        let name = &self.name;
        let kind = &self.kind;
        write!(f, "{name} ({kind:?}) [ ")?;
        for instance in &self.instances {
            instance.to_writer_pretty(f)?;
            write!(f, " ")?;
        }
        writeln!(f, "]")?;
        for setup_node in &self.setup_nodes {
            write!(f, "  ")?;
            setup_node.to_writer_pretty(f)?;
            writeln!(f, " (setup)")?;
        }
        for node in &self.cycle_nodes {
            write!(f, "  ")?;
            node.to_writer_pretty(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

impl ToWriterPretty for Cyclers {
    fn to_writer_pretty(&self, f: &mut impl Write) -> fmt::Result {
        for cycler in &self.cyclers {
            cycler.to_writer_pretty(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

impl ToWriterPretty for Node {
    fn to_writer_pretty(&self, f: &mut impl Write) -> fmt::Result {
        let name = &self.name;
        write!(f, "{name}")
    }
}

impl ToWriterPretty for Contexts {
    fn to_writer_pretty(&self, f: &mut impl Write) -> fmt::Result {
        writeln!(f, "CreationContext")?;
        for field in &self.creation_context {
            write!(f, "  ")?;
            field.to_writer_pretty(f)?;
            writeln!(f)?;
        }
        writeln!(f, "CycleContext")?;
        for field in &self.cycle_context {
            write!(f, "  ")?;
            field.to_writer_pretty(f)?;
            writeln!(f)?;
        }
        writeln!(f, "MainOutputs")?;
        for field in &self.main_outputs {
            write!(f, "  ")?;
            field.to_writer_pretty(f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

impl ToWriterPretty for Field {
    fn to_writer_pretty(&self, f: &mut impl Write) -> fmt::Result {
        match self {
            Field::AdditionalOutput { name, .. } => write!(f, "{name}: AdditfmtnalOutput"),
            Field::HardwareInterface { name, .. } => write!(f, "{name}: HardwareInterface"),
            Field::HistoricInput { name, .. } => write!(f, "{name}: HistoricInput"),
            Field::Input { name, .. } => write!(f, "{name}: Input"),
            Field::MainOutput { name, .. } => write!(f, "{name}: MainOutput"),
            Field::Parameter { name, .. } => write!(f, "{name}: Parameter"),
            Field::PerceptionInput { name, .. } => write!(f, "{name}: PerceptfmtnInput"),
            Field::PersistentState { name, .. } => write!(f, "{name}: PersistentState"),
            Field::RequiredInput { name, .. } => write!(f, "{name}: RequiredInput"),
        }
    }
}
