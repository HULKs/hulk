use proc_macro2::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::{
    parenthesized, parse_macro_input, punctuated::Punctuated, Data, DataStruct, DeriveInput,
    Generics, Ident, LitStr, Result, Token, Type, WherePredicate,
};

#[proc_macro_derive(SerializeHierarchy, attributes(serialize_hierarchy))]
#[proc_macro_error]
pub fn serialize_hierarchy(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_serialize_hierarchy(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn derive_serialize_hierarchy(mut input: DeriveInput) -> syn::Result<TokenStream> {
    let container = Container::try_from_ast(&input)?;
    container.extend_generics(&mut input.generics);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let path_serializations = container.generate_path_serializations();
    let leaf_serializations = container.generate_leaf_serializations();
    let path_deserializations = container.generate_path_deserializations();
    let leaf_deserializations = container.generate_leaf_deserializations();
    let path_exists = container.generate_path_exists();
    let leaf_exists = container.generate_leaf_exists();
    let extend_with_paths = container.generate_extend_with_paths();
    let extend_with_leafs = container.generate_extend_with_leafs();

    let serialize_path = quote! {
        fn serialize_path<S>(
            &self,
            path: &str,
            serializer: S,
        ) -> Result<S::Ok, serialize_hierarchy::Error<S::Error>>
        where
            S: serde::Serializer,
        {
            let split = path.split_once('.');
            match split {
                Some((name, suffix)) => match name {
                    #(#path_serializations,)*
                    segment => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                        segment: segment.to_string(),
                    }),
                },
                None => {
                    match path {
                        #(#leaf_serializations,)*
                        segment => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                            segment: segment.to_string(),
                        }),
                    }
                }
            }
        }
    };
    let deserialize_path = quote! {
        fn deserialize_path<'de, D>(
            &mut self,
            path: &str,
            deserializer: D,
        ) -> Result<(), serialize_hierarchy::Error<D::Error>>
        where
            D: serde::Deserializer<'de>,
        {
            let split = path.split_once('.');
            match split {
                Some((name, suffix)) => match name {
                    #(#path_deserializations,)*
                    name => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                        segment: name.to_string(),
                    }),
                },
                None => match path {
                    #(#leaf_deserializations,)*
                    name => Err(serialize_hierarchy::Error::UnexpectedPathSegment {
                        segment: name.to_string(),
                    }),
                },
            }
        }
    };
    let exists = quote! {
        fn exists(path: &str) -> bool {
            let split = path.split_once('.');
            match split {
                Some((name, suffix)) => match name {
                    #(#path_exists,)*
                    _ => false,
                },
                None => match path {
                    #(#leaf_exists,)*
                    _ => false,
                },
            }
        }
    };
    let extend_with_fields = quote! {
        fn extend_with_fields(fields: &mut std::collections::BTreeSet<String>, prefix: &str)  {
            #(#extend_with_paths)*
            #(#extend_with_leafs)*
        }
    };

    Ok(quote! {
        impl #impl_generics serialize_hierarchy::SerializeHierarchy for #name #ty_generics #where_clause {
            #serialize_path
            #deserialize_path
            #exists
            #extend_with_fields
        }
    })
}

fn read_fields(data: &DataStruct) -> Result<Vec<Field>> {
    data.fields.iter().map(Field::try_from_ast).collect()
}

struct ComputedLeaf {
    identifier: Ident,
    into_type: Type,
}

struct Container {
    fields: Vec<Field>,
    bounds: Option<Vec<WherePredicate>>,
    computed_leafs: Vec<ComputedLeaf>,
}

impl Container {
    fn try_from_ast(item: &DeriveInput) -> Result<Self> {
        let mut bounds = None;
        let mut computed_leafs = Vec::new();

        for attribute in &item.attrs {
            if !attribute.path().is_ident("serialize_hierarchy") {
                continue;
            }
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("bound") {
                    let value = meta.value()?;
                    let string: LitStr = value.parse()?;
                    let where_predicates = string
                        .parse_with(Punctuated::<WherePredicate, Token![,]>::parse_terminated)?;
                    bounds = Some(Vec::from_iter(where_predicates));
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
            Data::Union(data) => {
                abort!(
                    data.union_token,
                    "`SerializeHierarchy` can only be derived for `struct` or `enum`",
                )
            }
        };
        Ok(Container {
            computed_leafs,
            bounds,
            fields,
        })
    }

    fn extend_generics(&self, generics: &mut Generics) {
        if let Some(bounds) = &self.bounds {
            generics
                .make_where_clause()
                .predicates
                .extend(bounds.clone());
        }
    }

    fn generate_path_serializations(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .filter(|field| !field.skip && !field.leaf)
            .map(|field| {
                let identifier = &field.identifier;
                let pattern = identifier.to_string();
                quote! {
                    #pattern => self.#identifier.serialize_path(suffix, serializer)
                }
            })
            .collect()
    }

    fn generate_leaf_serializations(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .filter(|field| !field.skip)
            .map(|field| {
                let identifier = &field.identifier;
                let pattern = identifier.to_string();
                quote! {
                    #pattern => serde::Serialize::serialize(&self.#identifier, serializer).map_err(serialize_hierarchy::Error::SerializationFailed)
                }
            })
            .chain(self.computed_leafs.iter().map(|leaf| {
                let identifier = &leaf.identifier;
                let into_type = &leaf.into_type;
                let pattern = identifier.to_string();
                quote! {
                    #pattern => {
                        std::convert::TryInto::<#into_type>::try_into(self)
                            .map_err(serde::ser::Error::custom)
                            .and_then(|leaf| serde::Serialize::serialize(&leaf, serializer))
                            .map_err(serialize_hierarchy::Error::SerializationFailed)
                    }
                }
            }))
            .collect()
    }

    fn generate_path_deserializations(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .filter(|field| !field.skip && !field.leaf)
            .map(|field| {
                let identifier = &field.identifier;
                let pattern = identifier.to_string();
                quote! {
                    #pattern => self.#identifier.deserialize_path(suffix, deserializer)
                }
            })
            .collect()
    }

    fn generate_leaf_deserializations(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .filter(|field| !field.skip)
            .map(|field| {
                let identifier = &field.identifier;
                let pattern = identifier.to_string();
                let ty = &field.ty;
                quote! {
                    #pattern => {
                        self.#identifier = <#ty as serde::Deserialize>::deserialize(deserializer).map_err(serialize_hierarchy::Error::DeserializationFailed)?;
                        Ok(())
                    }
                }
            })
            .collect()
    }

    fn generate_path_exists(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .filter(|field| !field.skip && !field.leaf)
            .map(|field| {
                let identifier = &field.identifier;
                let pattern = identifier.to_string();
                let ty = &field.ty;
                quote! {
                    #pattern => <#ty as serialize_hierarchy::SerializeHierarchy>::exists(suffix)
                }
            })
            .collect()
    }

    fn generate_leaf_exists(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .filter(|field| !field.skip)
            .map(|field| {
                let identifier = &field.identifier;
                let pattern = identifier.to_string();
                quote! {
                    #pattern => true
                }
            })
            .chain(self.computed_leafs.iter().map(|leaf| {
                let identifier = &leaf.identifier;
                let pattern = identifier.to_string();
                quote! {
                    #pattern => true
                }
            }))
            .collect()
    }

    fn generate_extend_with_paths(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .filter(|field| !field.skip && !field.leaf)
            .map(|field| {
                let field_name = &field.identifier.to_string();
                let ty = &field.ty;
                quote! {
                    <#ty as serialize_hierarchy::SerializeHierarchy>::extend_with_fields(fields, &format!("{prefix}{}.", #field_name));
                }
            })
            .collect()
    }

    fn generate_extend_with_leafs(&self) -> Vec<TokenStream> {
        self.fields
            .iter()
            .filter(|field| !field.skip)
            .map(|field| {
                let field_name = &field.identifier.to_string();
                quote! {
                    fields.insert(format!("{prefix}{}", #field_name));
                }
            })
            .chain(self.computed_leafs.iter().map(|leaf| {
                let field_name = &leaf.identifier.to_string();
                quote! {
                    fields.insert(format!("{prefix}{}", #field_name));
                }
            }))
            .collect()
    }
}

#[derive(Debug)]
struct Field {
    skip: bool,
    leaf: bool,
    identifier: Ident,
    ty: Type,
}

impl Field {
    fn try_from_ast(field: &syn::Field) -> Result<Self> {
        let mut skip = false;
        let mut leaf = false;

        for attribute in &field.attrs {
            if !attribute.path().is_ident("serialize_hierarchy") {
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
