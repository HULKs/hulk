use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    process::Command,
};

use module_attributes2::{Attribute, Module};
use quote::quote;
use syn::{
    parse_file, punctuated::Punctuated, Ident, Item, Path, PathArguments, PathSegment, ReturnType,
    Type, TypeParamBound, UseTree,
};

fn extract_uses(mut prefix: Vec<Ident>, tree: &UseTree) -> HashMap<Ident, Vec<Ident>> {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.clone());
            extract_uses(prefix, &path.tree)
        }
        UseTree::Name(name) => {
            prefix.push(name.ident.clone());
            HashMap::from([(name.ident.clone(), prefix)])
        }
        UseTree::Rename(rename) => {
            prefix.push(rename.ident.clone());
            HashMap::from([(rename.rename.clone(), prefix)])
        }
        UseTree::Glob(_) => HashMap::new(),
        UseTree::Group(group) => group
            .items
            .iter()
            .map(|tree| extract_uses(prefix.clone(), tree))
            .flatten()
            .collect(),
    }
}

fn main() {
    let mut file = File::open("src/spl_network2/message_receiver.rs").unwrap();
    let mut buffer = String::new();
    file.read_to_string(&mut buffer).unwrap();
    let ast = parse_file(&buffer).unwrap();
    let uses: HashMap<_, _> = ast
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Use(use_item) => Some(extract_uses(vec![], &use_item.tree)),
            _ => None,
        })
        .flatten()
        .collect();
    println!("uses: {uses:?}");
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
            Ok(module) => {
                println!("module: {:#?}", module);
                generate_main_outputs_database(&module, &uses)
            }
            Err(error) => println!("{:#?} {}", error.span(), error.to_compile_error()),
        }
    }
}

fn to_absolute_path(relative_path: &Path, uses: &HashMap<Ident, Vec<Ident>>) -> Path {
    let prefix = relative_path
        .segments
        .first()
        .and_then(|first_segment| uses.get(&first_segment.ident));
    Path {
        leading_colon: relative_path.leading_colon,
        segments: match prefix {
            Some(prefix) => Punctuated::from_iter(
                prefix
                    .iter()
                    .map(|identifier| PathSegment {
                        ident: identifier.clone(),
                        arguments: PathArguments::None,
                    })
                    .chain(relative_path.segments.iter().skip(1).cloned()),
            ),
            None => relative_path.segments.clone(),
        },
    }
}

fn to_absolute_type(relative_data_type: &Type, uses: &HashMap<Ident, Vec<Ident>>) -> Type {
    let mut data_type = relative_data_type.clone();
    match &mut data_type {
        Type::Array(array) => {
            array.elem = Box::new(to_absolute_type(&array.elem, uses));
        }
        Type::BareFn(function) => {
            for input in function.inputs.iter_mut() {
                input.ty = to_absolute_type(&input.ty, uses);
            }
            if let ReturnType::Type(_arrow, return_type) = &mut function.output {
                *return_type = Box::new(to_absolute_type(&return_type, uses));
            }
        }
        Type::Group(group) => {
            group.elem = Box::new(to_absolute_type(&group.elem, uses));
        }
        Type::ImplTrait(trait_implementation) => {
            for bound in trait_implementation.bounds.iter_mut() {
                if let TypeParamBound::Trait(trait_bound) = bound {
                    trait_bound.path = to_absolute_path(&trait_bound.path, uses);
                }
            }
        }
        Type::Infer(_) => {}
        Type::Macro(macro_type) => {
            macro_type.mac.path = to_absolute_path(&macro_type.mac.path, uses);
        }
        Type::Never(_) => {}
        Type::Paren(parenthesized) => {
            parenthesized.elem = Box::new(to_absolute_type(&parenthesized.elem, uses));
        }
        Type::Path(path) => {
            if let Some(qself) = &mut path.qself {
                qself.ty = Box::new(to_absolute_type(&qself.ty, uses));
            }
            path.path = to_absolute_path(&path.path, uses);
        }
        Type::Ptr(pointer) => {
            pointer.elem = Box::new(to_absolute_type(&pointer.elem, uses));
        }
        Type::Reference(reference) => {
            reference.elem = Box::new(to_absolute_type(&reference.elem, uses));
        }
        Type::Slice(slice) => {
            slice.elem = Box::new(to_absolute_type(&slice.elem, uses));
        }
        Type::TraitObject(trait_object) => {
            for bound in trait_object.bounds.iter_mut() {
                if let TypeParamBound::Trait(trait_bound) = bound {
                    trait_bound.path = to_absolute_path(&trait_bound.path, uses);
                }
            }
        }
        Type::Tuple(tuple) => {
            for element in tuple.elems.iter_mut() {
                *element = to_absolute_type(&element, uses);
            }
        }
        Type::Verbatim(_) => {}
        _ => panic!("Type not implemented"),
    }
    data_type
}

fn generate_main_outputs_database(module: &Module, uses: &HashMap<Ident, Vec<Ident>>) {
    let main_outputs = module
        .attributes
        .iter()
        .filter_map(|attribute| match attribute {
            Attribute::MainOutput { data_type, name } => {
                let data_type = to_absolute_type(&data_type, uses);
                Some(quote! { #name: Option<#data_type> })
            }
            _ => None,
        });
    let database = quote! {
        struct MainOutputs {
            #(#main_outputs,)*
        }
    };

    {
        let mut file = File::create("database.rs").unwrap();
        write!(file, "{}", database).unwrap();
    }

    let status = Command::new("rustfmt")
        .arg("database.rs")
        .status()
        .expect("Failed to execute rustfmt");
    if !status.success() {
        panic!("rustfmt did not exit with success");
    }
}
