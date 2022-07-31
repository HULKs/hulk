use quote::ToTokens;
use syn::{parse2, ItemImpl};

use crate::Attribute;

#[derive(Debug)]
pub struct Module {
    attributes: Vec<Attribute>,
}

impl Module {
    pub fn from_implementation(mut implementation: ItemImpl) -> syn::Result<Self> {
        let attributes = implementation
            .attrs
            .into_iter()
            .map(|attribute| parse2(attribute.to_token_stream()))
            .collect::<Result<_, _>>()?;
        Ok(Self { attributes })
    }
}
