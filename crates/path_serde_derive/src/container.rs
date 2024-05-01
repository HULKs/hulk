use proc_macro_error::abort;
use syn::{
    parenthesized, parse::Parse as _, punctuated::Punctuated, Data, DataStruct, DeriveInput, Ident,
    Result, Token, Type, WherePredicate,
};

pub struct Container {
    pub fields: Vec<Field>,
    pub bounds: Option<Punctuated<WherePredicate, Token![,]>>,
    pub computed_leafs: Vec<ComputedLeaf>,
}

impl Container {
    pub fn try_from_ast(item: &DeriveInput) -> Result<Self> {
        let mut bounds = None;
        let mut computed_leafs = Vec::new();

        for attribute in &item.attrs {
            if !attribute.path().is_ident("path_serde") {
                continue;
            }
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("bound") {
                    let value = meta.value()?;
                    bounds = Some(value.parse_terminated(WherePredicate::parse, Token![,])?);
                    Ok(())
                } else if meta.path.is_ident("add_leaf") {
                    let content;
                    parenthesized!(content in meta.input);
                    let identifier: Ident = content.parse()?;
                    content.parse::<Token![:]>()?;
                    let into_type = content.parse::<Type>()?;
                    computed_leafs.push(ComputedLeaf {
                        identifier,
                        into_type,
                    });
                    Ok(())
                } else {
                    Err(meta.error("unknown attribute"))
                }
            })?;
        }

        let fields = match &item.data {
            Data::Struct(data) => read_fields(data)?,
            Data::Enum(..) => Vec::new(),
            Data::Union(..) => Vec::new(),
        };
        Ok(Container {
            computed_leafs,
            bounds,
            fields,
        })
    }
}

#[derive(Debug)]
pub struct Field {
    pub skip: bool,
    pub leaf: bool,
    pub identifier: Ident,
    pub ty: Type,
}

impl Field {
    fn try_from_ast(field: &syn::Field) -> Result<Self> {
        let mut skip = false;
        let mut leaf = false;

        for attribute in &field.attrs {
            if !attribute.path().is_ident("path_serde") {
                continue;
            }
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    skip = true;
                    Ok(())
                } else if meta.path.is_ident("leaf") {
                    leaf = true;
                    Ok(())
                } else {
                    Err(meta.error("unknown attribute"))
                }
            })?;
        }

        let identifier = field
            .ident
            .clone()
            .unwrap_or_else(|| abort!(field, "field has to be named"));
        let ty = field.ty.clone();

        Ok(Field {
            skip,
            leaf,
            identifier,
            ty,
        })
    }
}

fn read_fields(data: &DataStruct) -> Result<Vec<Field>> {
    data.fields.iter().map(Field::try_from_ast).collect()
}

pub struct ComputedLeaf {
    pub identifier: Ident,
    pub into_type: Type,
}
