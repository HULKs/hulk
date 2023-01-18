use std::env;
use std::path::PathBuf;

use bindgen::{Builder, CargoCallbacks};

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(CargoCallbacks))
        .generate()
        .expect("Failed to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}
