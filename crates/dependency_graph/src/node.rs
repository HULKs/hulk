use std::path::PathBuf;

use module_attributes2::Module;
use syn::{File, Ident};

#[derive(Clone, Debug)]
pub enum Node {
    AdditionalOutputs { cycler_module: Ident },
    Configuration,
    CyclerInstance { instance: Ident },
    CyclerModule { module: Ident },
    HardwareInterface,
    MainOutputs { cycler_module: Ident },
    Module { module: Module },
    ParsedRustFile { file: File },
    PersistentState { cycler_module: Ident },
    RustFilePath { path: PathBuf },
}
