use std::path::PathBuf;

use module_attributes2::Module;
use syn::File;

#[derive(Clone, Debug)]
pub enum Node {
    AdditionalOutputs { cycler_module: String },
    Configuration,
    CyclerInstance { instance: String },
    CyclerModule { module: String, path: PathBuf },
    HardwareInterface,
    MainOutputs { cycler_module: String },
    Module { module: Module },
    ParsedRustFile { file: File },
    PersistentState { cycler_module: String },
    RustFilePath { path: PathBuf },
}
