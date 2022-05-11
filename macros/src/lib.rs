use module::process_module_implementation;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use require_some::process_require_some;
use serialize_hierarchy::process_serialize_hierarchy_implementation;

mod module;
mod require_some;
mod serialize_hierarchy;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn module(attributes: TokenStream, input: TokenStream) -> TokenStream {
    process_module_implementation(attributes, input)
}

#[proc_macro_derive(SerializeHierarchy, attributes(leaf, dont_serialize))]
#[proc_macro_error]
pub fn serialize_hierarchy(input: TokenStream) -> TokenStream {
    process_serialize_hierarchy_implementation(input)
}

#[proc_macro]
pub fn require_some(input: TokenStream) -> TokenStream {
    process_require_some(input)
}
