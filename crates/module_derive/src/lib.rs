use module::process_module_implementation;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use require_some::process_require_some;

mod module;
mod require_some;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn module(attributes: TokenStream, input: TokenStream) -> TokenStream {
    process_module_implementation(attributes, input)
}

#[proc_macro]
pub fn require_some(input: TokenStream) -> TokenStream {
    process_require_some(input)
}
