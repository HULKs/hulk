use std::fmt::{self, Write};

use crate::{
    contexts::{Contexts, Field},
    cyclers::{Cycler, Cyclers},
    node::Node,
};

pub fn to_string_pretty(value: &impl ToWriterPretty) -> Result<String, fmt::Error> {
    let mut string = String::new();
    value.to_writer_pretty(&mut string)?;
    Ok(string)
}

pub trait ToWriterPretty {
    fn to_writer_pretty(&self, writer: &mut impl Write) -> fmt::Result;
}

impl ToWriterPretty for String {
    fn to_writer_pretty(&self, writer: &mut impl Write) -> fmt::Result {
        write!(writer, "{}", self)
    }
}

impl ToWriterPretty for Cycler {
    fn to_writer_pretty(&self, writer: &mut impl Write) -> fmt::Result {
        let name = &self.name;
        let kind = &self.kind;
        write!(writer, "{name} ({kind:?}) [ ")?;
        for instance in &self.instances {
            instance.to_writer_pretty(writer)?;
            write!(writer, " ")?;
        }
        writeln!(writer, "]")?;
        for setup_node in &self.setup_nodes {
            write!(writer, "  ")?;
            setup_node.to_writer_pretty(writer)?;
            writeln!(writer, " (setup)")?;
        }
        for node in &self.cycle_nodes {
            write!(writer, "  ")?;
            node.to_writer_pretty(writer)?;
            writeln!(writer)?;
        }
        Ok(())
    }
}

impl ToWriterPretty for Cyclers {
    fn to_writer_pretty(&self, writer: &mut impl Write) -> fmt::Result {
        for cycler in &self.cyclers {
            cycler.to_writer_pretty(writer)?;
            writeln!(writer)?;
        }
        Ok(())
    }
}

impl ToWriterPretty for Node {
    fn to_writer_pretty(&self, writer: &mut impl Write) -> fmt::Result {
        let name = &self.name;
        write!(writer, "{name}")
    }
}

impl ToWriterPretty for Contexts {
    fn to_writer_pretty(&self, writer: &mut impl Write) -> fmt::Result {
        writeln!(writer, "CreationContext")?;
        for field in &self.creation_context {
            write!(writer, "  ")?;
            field.to_writer_pretty(writer)?;
            writeln!(writer)?;
        }
        writeln!(writer, "CycleContext")?;
        for field in &self.cycle_context {
            write!(writer, "  ")?;
            field.to_writer_pretty(writer)?;
            writeln!(writer)?;
        }
        writeln!(writer, "MainOutputs")?;
        for field in &self.main_outputs {
            write!(writer, "  ")?;
            field.to_writer_pretty(writer)?;
            writeln!(writer)?;
        }
        Ok(())
    }
}

impl ToWriterPretty for Field {
    fn to_writer_pretty(&self, writer: &mut impl Write) -> fmt::Result {
        match self {
            Field::AdditionalOutput { name, .. } => write!(writer, "{name}: AdditfmtnalOutput"),
            Field::CyclerState { name, .. } => write!(writer, "{name}: CyclerState"),
            Field::HardwareInterface { name, .. } => write!(writer, "{name}: HardwareInterface"),
            Field::HistoricInput { name, .. } => write!(writer, "{name}: HistoricInput"),
            Field::Input { name, .. } => write!(writer, "{name}: Input"),
            Field::MainOutput { name, .. } => write!(writer, "{name}: MainOutput"),
            Field::Parameter { name, .. } => write!(writer, "{name}: Parameter"),
            Field::PerceptionInput { name, .. } => write!(writer, "{name}: PerceptfmtnInput"),
            Field::RequiredInput { name, .. } => write!(writer, "{name}: RequiredInput"),
        }
    }
}
