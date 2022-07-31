use std::{fs::File, io::Read};

use module_attributes2::Module;
use syn::{parse_file, Item};

fn main() {
    let mut file = File::open("src/spl_network2/message_receiver.rs").unwrap();
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();
    let ast = parse_file(&buffer).unwrap();
    for item in ast.items {
        let impl_item = match item {
            Item::Impl(impl_item) => impl_item,
            _ => continue,
        };
        if impl_item.attrs.is_empty() {
            continue;
        }
        let first_is_module_identifier = impl_item
            .attrs
            .first()
            .unwrap()
            .path
            .get_ident()
            .map_or(false, |identifier| identifier == "perception_module");
        if !first_is_module_identifier {
            continue;
        }
        match Module::from_implementation(impl_item) {
            Ok(module) => println!("module: {:#?}", module),
            Err(error) => println!("{:#?} {}", error.span(), error.to_compile_error()),
        }
    }
}
