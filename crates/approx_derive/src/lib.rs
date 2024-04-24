use proc_macro2::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Result, Type};

#[proc_macro_derive(AbsDiffEq, attributes(abs_diff_eq))]
#[proc_macro_error]
pub fn abs_diff_eq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_abs_diff_eq(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn derive_abs_diff_eq(input: DeriveInput) -> Result<TokenStream> {
    let fields = match input.data {
        Data::Struct(data) => data.fields,
        Data::Enum(data) => abort!(
            data.enum_token,
            "`AbsDiffEq` can only be derived for `struct`",
        ),
        Data::Union(data) => abort!(
            data.union_token,
            "`AbsDiffEq` can only be derived for `struct`",
        ),
    };
    let epsilon_type = extract_epsilon_type(&input.attrs)?;
    let name = input.ident;

    let conditions = fields.into_iter().map(|field| {
        let identifier = field
            .ident
            .clone()
            .unwrap_or_else(|| abort!(field, "field has to be named"));
        quote! {
            self.#identifier.abs_diff_eq(&other.#identifier, epsilon)
        }
    });

    Ok(quote! {
        impl approx::AbsDiffEq for #name {
            type Epsilon = #epsilon_type;

            fn default_epsilon() -> Self::Epsilon {
                Self::Epsilon::default_epsilon()
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
                #(#conditions&&)*
                true
            }
        }
    })
}

fn extract_epsilon_type(attributes: &[Attribute]) -> Result<Option<Type>> {
    let mut epsilon_type = None;

    for attribute in attributes {
        if !attribute.path().is_ident("abs_diff_eq") {
            continue;
        }
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("epsilon_type") {
                let value = meta.value()?;
                epsilon_type = Some(value.parse()?);
                Ok(())
            } else {
                Err(meta.error("unknown attribute"))
            }
        })?;
    }

    Ok(epsilon_type)
}

#[proc_macro_derive(RelativeEq)]
#[proc_macro_error]
pub fn relative_eq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    generate_relative_eq(input).into()
}

fn generate_relative_eq(input: DeriveInput) -> TokenStream {
    let fields = match input.data {
        Data::Struct(data) => data.fields,
        Data::Enum(data) => abort!(
            data.enum_token,
            "`RelativeEq` can only be derived for `struct`",
        ),
        Data::Union(data) => abort!(
            data.union_token,
            "`RelativeEq` can only be derived for `struct`",
        ),
    };
    let name = input.ident;
    let conditions = fields.into_iter().map(|field| {
        let identifier = field
            .ident
            .clone()
            .unwrap_or_else(|| abort!(field, "field has to be named"));
        quote! {
            self.#identifier.relative_eq(&other.#identifier, epsilon, max_relative)
        }
    });

    quote! {
        impl approx::RelativeEq for #name {
            fn default_max_relative() -> Self::Epsilon {
                Self::Epsilon::default_max_relative()
            }

            fn relative_eq(
                &self,
                other: &Self,
                epsilon: Self::Epsilon,
                max_relative: Self::Epsilon,
            ) -> bool {
                #(#conditions&&)*
                true
            }
        }
    }
}
