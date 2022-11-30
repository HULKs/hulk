use color_eyre::Result;
use std::fs::read_to_string;
use toml::Value;

fn main() -> Result<()> {
    let manifest_file = read_to_string("Cargo.toml")?;
    let lock_file = read_to_string("Cargo.lock")?;

    let manifest = manifest_file.parse::<Value>()?;
    let lock = lock_file.parse::<Value>()?;

    let package_name = manifest
        .get("package")
        .unwrap()
        .as_table()
        .unwrap()
        .get("name")
        .unwrap()
        .as_str()
        .unwrap();
    let packages = lock.get("package").unwrap();

    let lock_packages = match packages {
        Value::Array(array) => array.iter().map(|package| {
            (
                package.get("name").unwrap().as_str().unwrap(),
                package.get("version").unwrap().as_str().unwrap(),
            )
        }),
        _ => unreachable!(),
    };

    for (name, version) in lock_packages {
        if name == package_name {
            continue;
        }

        println!("    crate://crates.io/{}/{} \\", name, version);
    }

    Ok(())
}
