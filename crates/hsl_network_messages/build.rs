#[cfg(feature = "ros_z")]
use std::{env, path::PathBuf};

#[cfg(feature = "ros_z")]
use bindgen::Builder;

#[cfg(feature = "ros_z")]
fn main() {
    let bindings = Builder::default()
        .header("headers/RoboCupGameControlData.hpp")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .fit_macro_constants(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}

#[cfg(not(feature = "ros_z"))]
fn main() {}
