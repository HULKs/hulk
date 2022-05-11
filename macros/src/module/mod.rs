use convert_case::{Case, Casing};
use proc_macro_error::abort_call_site;
use quote::format_ident;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, ItemImpl, TypePath,
};

use self::{
    control::generate_for_control, module_information::ModuleInformation,
    vision::generate_for_vision,
};

mod attributes;
mod control;
mod generators;
mod module_information;
mod vision;

fn to_snake_case(identifier: &TypePath) -> Ident {
    format_ident!(
        "{}",
        &match identifier.path.get_ident() {
            Some(identifier) => identifier,
            None => &identifier.path.segments.last().unwrap().ident,
        }
        .to_string()
        .to_case(Case::Snake)
    )
}

#[derive(Debug)]
struct Arguments {
    cycler_module: Ident,
}

impl Parse for Arguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            cycler_module: input.parse()?,
        })
    }
}

pub fn process_module_implementation(
    attributes: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let arguments = parse_macro_input!(attributes as Arguments);
    let mut module_implementation = parse_macro_input!(input as ItemImpl);

    let cycle_method = ModuleInformation::from_module_implementation(&mut module_implementation);

    match arguments.cycler_module.to_string().as_str() {
        "control" => generate_for_control(
            &arguments.cycler_module,
            module_implementation,
            cycle_method,
        ),
        "vision" => generate_for_vision(
            &arguments.cycler_module,
            module_implementation,
            cycle_method,
        ),
        _ => abort_call_site!("Unknown cycler module"),
    }
}
