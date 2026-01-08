use std::path::Path;

use color_eyre::Result;
use walking_inference::inference::WalkingInference;

fn main() -> Result<()> {
    println!("Hello, world!");
    WalkingInference::new(Path::new("etc/neural_network/T1.onnx"), &Default::default())?;

    Ok(())
}
