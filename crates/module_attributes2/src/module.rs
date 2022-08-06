use std::collections::HashMap;
use std::mem::take;

use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
use quote::{format_ident, ToTokens};
use syn::{parse2, spanned::Spanned, Error, Ident, ItemImpl, Type};

use crate::attribute::Path;
use crate::to_absolute::ToAbsolute;
use crate::{Attribute, Uses};

#[derive(Clone, Debug)]
pub struct Module {
    pub attributes: Vec<Attribute>,
    pub module_identifier: Ident,
    pub module_identifier_snake_case: Ident,
    pub new_context_identifier: Ident,
    pub cycle_context_identifier: Ident,
    pub main_outputs_identifier: Ident,
    pub implementation: ItemImpl,
}

impl Module {
    pub fn from_implementation(mut implementation: ItemImpl) -> syn::Result<Self> {
        let attributes = take(&mut implementation.attrs)
            .into_iter()
            .map(|attribute| parse2(attribute.to_token_stream()))
            .collect::<Result<_, _>>()?;

        let type_path = match *implementation.self_ty {
            Type::Path(ref type_path) => type_path,
            _ => {
                return Err(Error::new(
                    implementation.self_ty.span(),
                    format!("Expected type path"),
                ))
            }
        };
        let module_identifier = type_path
            .path
            .get_ident()
            .ok_or_else(|| Error::new(type_path.path.span(), format!("Expected identifier")))?
            .clone();
        let module_identifier_snake_case =
            format_ident!("{}", module_identifier.to_string().to_case(Case::Snake));

        Ok(Self {
            attributes,
            module_identifier,
            module_identifier_snake_case,
            new_context_identifier: format_ident!("NewContext"),
            cycle_context_identifier: format_ident!("CycleContext"),
            main_outputs_identifier: format_ident!("MainOutputs"),
            implementation,
        })
    }

    pub fn generate_main_output_fields(&self, uses: &Uses) -> Vec<TokenStream> {
        self.attributes
            .iter()
            .filter_map(|attribute| match attribute {
                Attribute::MainOutput { data_type, name } => {
                    let data_type = data_type.to_absolute(uses);
                    Some(quote! { #name: Option<#data_type> })
                }
                _ => None,
            })
            .collect()
    }

    pub fn generate_persistent_state_fields(&self, uses: &Uses) -> HashMap<Path, Type> {
        self.attributes
            .iter()
            .filter_map(|attribute| match attribute {
                Attribute::PersistentState {
                    data_type,
                    name: _,
                    path,
                } => Some((path.clone(), data_type.to_absolute(uses))),
                _ => None,
            })
            .collect()
    }
}
