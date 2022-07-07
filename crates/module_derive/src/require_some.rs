use proc_macro::TokenStream;
use quote::quote;

pub fn process_require_some(input: TokenStream) -> TokenStream {
    let field = proc_macro2::TokenStream::from(input);
    let expanded = quote! {
        match #field {
            Some(data) => data,
            None => return Ok(MainOutputs::none()),
        }
    };
    expanded.into()
}
