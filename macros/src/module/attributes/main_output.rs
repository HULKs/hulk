use proc_macro_error::{abort, abort_call_site};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    Ident, Token, TypePath,
};

use crate::module::to_snake_case;

#[derive(Debug)]
pub struct MainOutputAttribute {
    pub data_type: TypePath,
    pub name: Ident,
}

impl Parse for MainOutputAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        parenthesized!(content in input);

        let mut data_type = None;
        let mut identifier_name = None;
        loop {
            let argument_name = content.parse::<Ident>()?;
            content.parse::<Token![=]>()?;
            match argument_name.to_string().as_str() {
                "data_type" => data_type = Some(content.parse::<TypePath>()?),
                "name" => identifier_name = Some(content.parse::<Ident>()?),
                _ => abort!(argument_name, "unexpected `{}` argument", argument_name),
            }
            if content.is_empty() {
                break;
            }
            content.parse::<Token![,]>()?;
        }

        let data_type =
            data_type.unwrap_or_else(|| abort_call_site!("missing required `data_type` argument"));
        let name = identifier_name.unwrap_or_else(|| to_snake_case(&data_type));
        Ok(Self { data_type, name })
    }
}
