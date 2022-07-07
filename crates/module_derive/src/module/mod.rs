use module_attributes::ModuleInformation;
use proc_macro_error::abort_call_site;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, ItemImpl,
};

use self::{control::generate_for_control, vision::generate_for_vision};

mod control;
mod generators;
mod vision;

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
    let module_implementation = parse_macro_input!(input as ItemImpl);
    let module_information = ModuleInformation::from_module_implementation(module_implementation);

    match arguments.cycler_module.to_string().as_str() {
        "control" => generate_for_control(&arguments.cycler_module, module_information),
        "vision" => generate_for_vision(&arguments.cycler_module, module_information),
        _ => abort_call_site!("Unknown cycler module"),
    }
}
