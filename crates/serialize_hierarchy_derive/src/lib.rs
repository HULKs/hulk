use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use serialize_hierarchy::process_serialize_hierarchy_implementation;

mod serialize_hierarchy;

#[proc_macro_derive(SerializeHierarchy, attributes(leaf, dont_serialize))]
#[proc_macro_error]
pub fn serialize_hierarchy(input: TokenStream) -> TokenStream {
    process_serialize_hierarchy_implementation(input)
}
