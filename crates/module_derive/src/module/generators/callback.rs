use module_attributes::ParameterAttribute;
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_change_callback_invocation(attribute: &ParameterAttribute) -> TokenStream {
    if let Some(change_callback_name) = &attribute.change_callback_name {
        let path_string = attribute
            .path
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(".");
        let path_string = quote! { #path_string };
        let path = &attribute.path;
        let changed_member = if attribute.path_is_relative_to_cycler {
            quote! {cycler_configuration.#(#path).*}
        } else {
            quote! {configuration.#(#path).*}
        };
        quote! {
            if changed_parameters.contains(#path_string) {
                self.#change_callback_name(&#changed_member)?;
            }
        }
    } else {
        TokenStream::new()
    }
}
