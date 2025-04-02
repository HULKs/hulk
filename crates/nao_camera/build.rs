use std::path::PathBuf;
use std::{env, error::Error};

use bindgen::Builder;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = Builder::default()
        .rust_target("1.80".parse()?)
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");

    Ok(())
}
