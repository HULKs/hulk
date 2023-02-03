use std::{collections::HashMap, env::args, fs::read_to_string};
use toml::Value;

fn main() {
    // let root_crate = args()
    //     .skip(1)
    //     .next()
    //     .expect("expected root crate as argument");
    let lock_file = read_to_string("Cargo.lock").expect("failed to read Cargo.lock");
    let lock = lock_file
        .parse::<Value>()
        .expect("failed to parse Cargo.lock");
    let Value::Array(lock_packages) = lock.get("package").expect("failed to get `package` array") else {
        panic!("expected `package` to be an array");
    };
    let mut packages: HashMap<String, HashMap<String, Package>> = HashMap::new();
    for package in lock_packages {
        let name = package
            .get("name")
            .expect("expected `name` field")
            .as_str()
            .expect("expected `name` field of type string")
            .to_string();
        let version = package
            .get("version")
            .expect("expected `version` field")
            .as_str()
            .expect("expected `version` field of type string")
            .to_string();
        let dependencies = package
            .get("dependencies")
            .unwrap_or(&Value::Array(Default::default()))
            .as_array()
            .expect("expected `dependencies` field of type array")
            .iter()
            .map(|dependency| {
                let dependency = dependency
                    .as_str()
                    .expect("expected `dependencies` items of type string");
                match dependency.split_once(' ') {
                    Some((name, version)) => (name.to_string(), Some(version.to_string())),
                    None => (dependency.to_string(), None),
                }
            })
            .collect();
        packages.entry(name).or_default().insert(
            version,
            match package.get("source") {
                Some(_) => Package::Remote { dependencies },
                None => Package::Local { dependencies },
            },
        );
    }
    // let mut pending_packages = vec![(root_crate, None)];
    // let mut visited_packages = HashSet::new();
    // let mut collected_packages = vec![];
    // while !pending_packages.is_empty() {
    //     let (name, version) = pending_packages.pop().unwrap();
    //     let versions = &packages[&name];
    //     let (version, dependencies) = match version {
    //         Some(version) => {
    //             let package = &versions[&version];
    //             (version, package)
    //         }
    //         None => versions
    //             .iter()
    //             .map(|(version, package)| (version.clone(), package))
    //             .next()
    //             .unwrap(),
    //     };
    //     let dependencies = match dependencies {
    //         Package::Local { dependencies } => dependencies,
    //         Package::Remote { dependencies } => {
    //             collected_packages.push((name, version));
    //             dependencies
    //         }
    //     };
    //     for dependency in dependencies {
    //         if visited_packages.insert(dependency.clone()) {
    //             pending_packages.push(dependency.clone());
    //         }
    //     }
    // }
    // for (name, version) in collected_packages {
    //     println!("    crate://crates.io/{name}/{version} \\");
    // }
    for (name, versions) in packages {
        for (version, dependencies) in versions {
            match dependencies {
                Package::Local { .. } => {}
                Package::Remote { .. } => println!("    crate://crates.io/{name}/{version} \\"),
            }
        }
    }
}

enum Package {
    Local {
        dependencies: Vec<(String, Option<String>)>,
    },
    Remote {
        dependencies: Vec<(String, Option<String>)>,
    },
}
