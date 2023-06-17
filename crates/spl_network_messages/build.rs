use std::{env, path::PathBuf};

use bindgen::{Builder, CargoCallbacks};

fn main() {
    let bindings = Builder::default()
        .header("headers/RoboCupGameControlData.hpp")
        .header("headers/VisualRefereeChallenge.hpp")
        .parse_callbacks(Box::new(CargoCallbacks))
        .layout_tests(false)
        .fit_macro_constants(true)
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Failed to write bindings");
}
