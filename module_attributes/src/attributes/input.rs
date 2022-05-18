use proc_macro_error::abort;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    Ident, Token, TypePath,
};

#[derive(Debug)]
pub struct InputAttribute {
    pub path: Vec<Ident>,
    pub data_type: TypePath,
    pub cycler: Option<Ident>,
    pub name: Ident,
}

impl Parse for InputAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        parenthesized!(content in input);

        let mut path: Option<Vec<Ident>> = None;
        let mut data_type = None;
        let mut cycler = None;
        let mut identifier_name = None;
        loop {
            let argument_name = content.parse::<Ident>()?;
            content.parse::<Token![=]>()?;
            match argument_name.to_string().as_str() {
                "path" => {
                    let mut segments = Vec::new();
                    loop {
                        segments.push(content.parse()?);
                        if !content.peek(Token![.]) {
                            break;
                        }
                        content.parse::<Token![.]>()?;
                    }
                    path = Some(segments);
                }
                "data_type" => data_type = Some(content.parse::<TypePath>()?),
                "cycler" => cycler = Some(content.parse::<Ident>()?),
                "name" => identifier_name = Some(content.parse::<Ident>()?),
                _ => abort!(argument_name, "unexpected `{}` argument", argument_name),
            }
            if content.is_empty() {
                break;
            }
            content.parse::<Token![,]>()?;
        }

        let path =
            path.unwrap_or_else(|| abort!(content.span(), "missing required `path` argument"));
        let data_type = data_type
            .unwrap_or_else(|| abort!(content.span(), "missing required `data_type` argument"));
        let name = identifier_name.unwrap_or_else(|| path.last().unwrap().clone());
        Ok(Self {
            path,
            data_type,
            cycler,
            name,
        })
    }
}
