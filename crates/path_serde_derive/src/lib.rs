use deserialize::derive_path_deserialize;
use introspect::derive_path_introspect;
use proc_macro_error::proc_macro_error;
use serialize::derive_path_serialize;
use syn::{parse_macro_input, DeriveInput};

mod bound;
mod container;
mod deserialize;
mod introspect;
mod serialize;

#[proc_macro_derive(PathSerialize, attributes(path_serde))]
#[proc_macro_error]
pub fn path_serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_path_serialize(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(PathDeserialize, attributes(path_serde))]
#[proc_macro_error]
pub fn path_deserialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_path_deserialize(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(PathIntrospect, attributes(path_serde))]
#[proc_macro_error]
pub fn path_introspect(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_path_introspect(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
