//! Example placeholder for dynamic/static interop.

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Dynamic interop uses Schema/TypeShape roots.");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run()
}
