use proc_macro_error::abort;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    Ident, Token, TypePath,
};

#[derive(Debug)]
pub struct ParameterAttribute {
    pub data_type: TypePath,
    pub path: Vec<Ident>,
    pub path_is_relative_to_cycler: bool,
    pub change_callback_name: Option<Ident>,
    pub name: Ident,
}

impl Parse for ParameterAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        parenthesized!(content in input);

        let mut data_type = None;
        let mut identifier_name = None;
        let mut path: Option<Vec<Ident>> = None;
        let mut path_is_relative_to_cycler = false;
        let mut change_callback_name = None;
        loop {
            let argument_name = content.parse::<Ident>()?;
            content.parse::<Token![=]>()?;
            match argument_name.to_string().as_str() {
                "data_type" => data_type = Some(content.parse::<TypePath>()?),
                "name" => identifier_name = Some(content.parse::<Ident>()?),
                "path" => {
                    let mut segments = Vec::new();
                    if content.peek(Token![$]) {
                        content.parse::<Token![$]>()?;
                        let capture = content.parse::<Ident>()?;
                        if capture != "this_cycler" {
                            abort!(capture, "unexpected capture"; help = "only `$this_cycler` is implemented");
                        }
                        content.parse::<Token![.]>()?;
                        path_is_relative_to_cycler = true;
                    }
                    loop {
                        segments.push(content.parse()?);
                        if !content.peek(Token![.]) {
                            break;
                        }
                        content.parse::<Token![.]>()?;
                    }
                    path = Some(segments);
                }
                "on_changed" => change_callback_name = Some(content.parse::<Ident>()?),
                _ => abort!(argument_name, "unexpected `{}` argument", argument_name),
            }
            if content.is_empty() {
                break;
            }
            content.parse::<Token![,]>()?;
        }

        let data_type = data_type
            .unwrap_or_else(|| abort!(content.span(), "missing required `data_type` argument"));
        let path =
            path.unwrap_or_else(|| abort!(content.span(), "missing required `path` argument"));
        let name = identifier_name.unwrap_or_else(|| path.last().unwrap().clone());
        Ok(Self {
            data_type,
            path,
            path_is_relative_to_cycler,
            change_callback_name,
            name,
        })
    }
}
