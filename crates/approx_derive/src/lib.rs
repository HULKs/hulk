use proc_macro2::TokenStream;
use proc_macro_error::{abort, proc_macro_error, OptionExt, ResultExt};
use quote::quote;
use syn::{
    parse_macro_input,
    punctuated::{self},
    Attribute, Data, DeriveInput, Lit, Meta, MetaNameValue, NestedMeta, Type,
};

#[proc_macro_derive(AbsDiffEq, attributes(abs_diff_eq))]
#[proc_macro_error]
pub fn abs_diff_eq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    generate_abs_diff_eq(input).into()
}

fn generate_abs_diff_eq(input: DeriveInput) -> TokenStream {
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
    let epsilon = extract_epsilon(&input.attrs).expect_or_abort("`epsilon` not specified");
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

    quote! {
        impl approx::AbsDiffEq for #name {
            type Epsilon = #epsilon;

            fn default_epsilon() -> Self::Epsilon {
                Self::Epsilon::default_epsilon()
            }

            fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
                #(#conditions)&&*
            }
        }
    }
}

fn extract_epsilon(attrs: &[Attribute]) -> Option<Type> {
    attrs
        .iter()
        .filter_map(parse_meta_items)
        .flatten()
        .find_map(|meta| match meta {
            NestedMeta::Meta(Meta::NameValue(MetaNameValue { path, lit, .. }))
                if path.is_ident("epsilon") =>
            {
                let string = match lit {
                    Lit::Str(string) => string,
                    _ => abort!(lit, "expected string literal"),
                };
                let epsilon = string
                    .parse()
                    .expect_or_abort("failed to parse epsilon type");
                Some(epsilon)
            }
            _ => None,
        })
}

fn parse_meta_items(attribute: &Attribute) -> Option<punctuated::IntoIter<NestedMeta>> {
    if !attribute.path.is_ident("abs_diff_eq") {
        return None;
    }
    match attribute.parse_meta() {
        Ok(Meta::List(meta)) => Some(meta.nested.into_iter()),
        Ok(other) => abort!(other, "expected `#[abs_diff_eq(...)]`",),
        Err(error) => abort!(error.span(), error.to_string()),
    }
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
                #(#conditions)&&*
            }
        }
    }
}
