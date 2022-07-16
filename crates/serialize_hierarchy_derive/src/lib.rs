use proc_macro::TokenStream;
use proc_macro_error::{abort_call_site, proc_macro_error};
use process_enum::process_enum;
use process_struct::process_struct;
use syn::{parse_macro_input, Data, DeriveInput};

mod process_enum;
mod process_struct;

#[proc_macro_derive(SerializeHierarchy, attributes(dont_serialize))]
#[proc_macro_error]
pub fn serialize_hierarchy(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match &input.data {
        Data::Struct(data) => process_struct(&input, data),
        Data::Enum(data) => process_enum(&input, data),
        Data::Union(..) => {
            abort_call_site!("`SerializeHierarchy` can only be derived for `struct`")
        }
    }
}
