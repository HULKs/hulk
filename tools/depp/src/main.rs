use color_eyre::Result;
use std::fs::read_to_string;
use toml::Value;

fn main() -> Result<()> {
    let lock_file = read_to_string("Cargo.lock")?;
    let lock = lock_file.parse::<Value>()?;
    let packages = lock.get("package").unwrap();

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
