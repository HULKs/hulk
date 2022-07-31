use quote::ToTokens;
use syn::{parse2, ItemImpl};

use crate::Attribute;

#[derive(Debug)]
pub struct Module {}

impl Module {
    pub fn from_implementation(implementation: ItemImpl) -> syn::Result<Self> {
        println!("attributes: {:#?}", implementation.attrs);
        let foo = implementation.attrs.first().unwrap().clone();
        let foo = foo.to_token_stream();
        println!("foo: {:#?}", foo);
        let attribute: Attribute = parse2(foo)?;
        println!("attribute: {:#?}", attribute);
        Ok(Self {})
    }
}
