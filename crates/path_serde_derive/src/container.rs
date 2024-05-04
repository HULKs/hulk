use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parenthesized, parse::Parse as _, parse_quote, Data, DataStruct, DeriveInput, Ident, Index,
    Result, Token, Type, WherePredicate,
};

use crate::bound::infer_predicates;

pub struct Container {
    pub fields: Vec<Field>,
    pub serialize_bounds: Vec<WherePredicate>,
    pub deserialize_bounds: Vec<WherePredicate>,
    pub introspect_bounds: Vec<WherePredicate>,
    pub computed_leafs: Vec<ComputedLeaf>,
}

impl Container {
    pub fn try_from_ast(item: &DeriveInput) -> Result<Self> {
        let mut serialize_bounds = None;
        let mut deserialize_bounds = None;
        let mut introspect_bounds = None;
        let mut computed_leafs = Vec::new();

        for attribute in &item.attrs {
            if !attribute.path().is_ident("path_serde") {
                continue;
            }
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("bound") {
                    let bounds = Some(
                        meta.value()?
                            .parse_terminated(WherePredicate::parse, Token![,])?
                            .into_iter()
                            .collect(),
                    );
                    serialize_bounds = bounds.clone();
                    deserialize_bounds = bounds.clone();
                    introspect_bounds = bounds.clone();
                } else if meta.path.is_ident("serialize_bound") {
                    serialize_bounds = Some(
                        meta.value()?
                            .parse_terminated(WherePredicate::parse, Token![,])?
                            .into_iter()
                            .collect(),
                    );
                } else if meta.path.is_ident("deserialize_bound") {
                    deserialize_bounds = Some(
                        meta.value()?
                            .parse_terminated(WherePredicate::parse, Token![,])?
                            .into_iter()
                            .collect(),
                    );
                } else if meta.path.is_ident("introspect_bound") {
                    introspect_bounds = Some(
                        meta.value()?
                            .parse_terminated(WherePredicate::parse, Token![,])?
                            .into_iter()
                            .collect(),
                    );
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
                } else {
                    return Err(meta.error("unknown attribute"));
                }
                Ok(())
            })?;
        }

        let fields = match &item.data {
            Data::Struct(data) => read_fields(data)?,
            Data::Enum(..) => Vec::new(),
            Data::Union(..) => Vec::new(),
        };
        let serialize_bounds = serialize_bounds.unwrap_or_else(|| {
            infer_predicates(
                item,
                parse_quote!(path_serde::PathSerialize + serde::Serialize),
                None,
            )
        });
        let deserialize_bounds = deserialize_bounds.unwrap_or_else(|| {
            infer_predicates(
                item,
                parse_quote!(path_serde::PathDeserialize + serde::Deserialize<'de>),
                Some(parse_quote!(for<'de>)),
            )
        });
        let introspect_bounds = introspect_bounds.unwrap_or_else(|| {
            infer_predicates(item, parse_quote!(path_serde::PathIntrospect), None)
        });

        Ok(Container {
            computed_leafs,
            serialize_bounds,
            deserialize_bounds,
            introspect_bounds,
            fields,
        })
    }
}

#[derive(Debug)]
pub enum Identifier {
    Ident(Ident),
    Index(Index),
}

impl ToTokens for Identifier {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Identifier::Ident(ident) => ident.to_tokens(tokens),
            Identifier::Index(index) => index.to_tokens(tokens),
        }
    }
}

impl Identifier {
    pub fn to_field_name(&self) -> String {
        match self {
            Identifier::Ident(ident) => ident.to_string(),
            Identifier::Index(index) => index.index.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct Field {
    pub skip_serialize: bool,
    pub skip_deserialize: bool,
    pub skip_introspect: bool,
    pub is_leaf: bool,
    pub identifier: Identifier,
    pub ty: Type,
}

impl Field {
    fn try_from_ast(index: usize, field: &syn::Field) -> Result<Self> {
        let mut skip_serialize = false;
        let mut skip_deserialize = false;
        let mut skip_introspect = false;
        let mut is_leaf = false;

        for attribute in &field.attrs {
            if !attribute.path().is_ident("path_serde") {
                continue;
            }
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("skip") {
                    skip_serialize = true;
                    skip_deserialize = true;
                    skip_introspect = true;
                } else if meta.path.is_ident("skip_serialize") {
                    skip_serialize = true;
                } else if meta.path.is_ident("skip_deserialize") {
                    skip_deserialize = true;
                } else if meta.path.is_ident("skip_introspect") {
                    skip_introspect = true;
                } else if meta.path.is_ident("leaf") {
                    is_leaf = true;
                } else {
                    return Err(meta.error("unknown attribute"));
                }
                Ok(())
            })?;
        }

        let identifier = match &field.ident {
            Some(ident) => Identifier::Ident(ident.clone()),
            None => Identifier::Index(Index::from(index)),
        };
        let ty = field.ty.clone();

        Ok(Field {
            skip_serialize,
            skip_deserialize,
            skip_introspect,
            is_leaf,
            identifier,
            ty,
        })
    }
}

fn read_fields(data: &DataStruct) -> Result<Vec<Field>> {
    data.fields
        .iter()
        .enumerate()
        .map(|(index, field)| Field::try_from_ast(index, field))
        .collect()
}

pub struct ComputedLeaf {
    pub identifier: Ident,
    pub into_type: Type,
}
