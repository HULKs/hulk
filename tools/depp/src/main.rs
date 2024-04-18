use color_eyre::{eyre::Context, Result};
use std::fs::read_to_string;
use toml::Value;

fn main() -> Result<()> {
    let lock_file = read_to_string("Cargo.lock").wrap_err("failed to read Cargo.lock")?;
    let lock = lock_file
        .parse::<Value>()
        .wrap_err("failed to parse Cargo.lock")?;
    let packages = lock
        .get("package")
        .expect("Cargo.lock should alway contain a package section");

    let lock_packages = match packages {
        Value::Array(array) => array.iter().map(|package| {
            (
                package.get("name").unwrap().as_str().unwrap(),
                package.get("version").unwrap().as_str().unwrap(),
                package.get("checksum"),
            )
        }),
        _ => unreachable!(),
    };

    let mut dependencies = String::new();
    let mut checksums = String::new();

    for (name, version, checksum) in lock_packages {
        if let Some(checksum) = checksum {
            dependencies.push_str(&format!(
                "            crate://crates.io/{name}/{version} \\\n"
            ));
            checksums.push_str(&format!(
                "SRC_URI[{name}-{version}.sha256sum] = {checksum}\n"
            ));
        }
    }

    print!("SRC_URI += \"\\\n{dependencies}           \"\n\n{checksums}");

    Ok(())
}
