use std::path::PathBuf;

use module_attributes2::Module;
use syn::{File, Type};

use crate::parser::Uses;

#[derive(Clone, Debug)]
pub enum Node {
    Configuration,
    CyclerInstance { instance: String },
    CyclerModule { module: String, path: PathBuf },
    HardwareInterface,
    Module { module: Module },
    ParsedRustFile { file: File },
    RustFilePath { path: PathBuf },
    Struct { name: String, cycler_module: String }, // TODO: remove cycler_module
    StructField { data_type: Type },
    Uses { uses: Uses },
}
