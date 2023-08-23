use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input,
    punctuated::{Pair, Punctuated},
    spanned::Spanned,
    token::Mut,
    AngleBracketedGenericArguments, Expr, ExprLit, GenericArgument, GenericParam, ItemStruct,
    Lifetime, LifetimeDef, Lit, Path, PathArguments, PathSegment, Type, TypeParam, TypePath,
    TypeReference,
};

#[proc_macro_attribute]
#[proc_macro_error]
pub fn context(_attributes: TokenStream, input: TokenStream) -> TokenStream {
    let mut struct_item = parse_macro_input!(input as ItemStruct);

    let struct_name = struct_item.ident.to_string();
    let allowed_member_types = match struct_name.as_str() {
        "CreationContext" => ["HardwareInterface", "Parameter", "PersistentState"].as_slice(),
        "CycleContext" => [
            "AdditionalOutput",
            "HardwareInterface",
            "HistoricInput",
            "Input",
            "Parameter",
            "PerceptionInput",
            "PersistentState",
            "RequiredInput",
        ]
        .as_slice(),
        "MainOutputs" => ["MainOutput"].as_slice(),
        _ => abort!(
            struct_item.ident,
            "unexpected context name, try one of `CreationContext`, `CycleContext`, `MainOutputs`"
        ),
    };

    let mut requires_lifetime_parameter = false;
    let mut requires_hardware_interface_parameter = false;

    for field in struct_item.fields.iter_mut() {
        match &mut field.ty {
            Type::Path(path) => {
                let first_segment = match path.path.segments.first_mut() {
                    Some(segment) => segment,
                    None => abort!(path, "expected type path with at least one segment"),
                };
                let field_type = first_segment.ident.to_string();
                if !allowed_member_types.contains(&field_type.as_str()) {
                    abort!(
                        field,
                        format!("{struct_name} may not contain members of type {field_type}")
                    );
                };

                match field_type.as_str() {
                    "PerceptionInput" => match &mut first_segment.arguments {
                        PathArguments::AngleBracketed(arguments) if arguments.args.len() == 3 => {
                            pop_string_argument(arguments);
                            pop_string_argument(arguments);
                            let data_type = get_data_type(arguments);
                            into_reference_with_lifetime(data_type, None);
                            requires_lifetime_parameter = true;
                            embed_into_vec(data_type);
                        }
                        _ => abort!(first_segment, "expected exactly three generic parameters"),
                    },
                    "Input" | "RequiredInput" => match &mut first_segment.arguments {
                        PathArguments::AngleBracketed(arguments)
                            if arguments.args.len() == 2 || arguments.args.len() == 3 =>
                        {
                            let has_additional_argument = arguments.args.len() == 3;
                            pop_string_argument(arguments);
                            if has_additional_argument {
                                pop_string_argument(arguments);
                            }
                            if first_segment.ident == "RequiredInput" {
                                let data_type = get_data_type(arguments);
                                unwrap_option(data_type);
                            }
                            let data_type = get_data_type(arguments);
                            into_reference_with_lifetime(data_type, None);
                            requires_lifetime_parameter = true;
                            field.ty = data_type.clone();
                        }
                        _ => abort!(
                            first_segment,
                            "expected exactly two or three generic parameters"
                        ),
                    },
                    "Parameter" | "PersistentState" => match &mut first_segment.arguments {
                        PathArguments::AngleBracketed(arguments) if arguments.args.len() == 2 => {
                            pop_string_argument(arguments);
                            let data_type = get_data_type(arguments);
                            into_reference_with_lifetime(
                                data_type,
                                (first_segment.ident == "PersistentState").then(Mut::default),
                            );
                            requires_lifetime_parameter = true;
                            field.ty = data_type.clone();
                        }
                        _ => abort!(first_segment, "expected exactly two generic parameters"),
                    },
                    "AdditionalOutput" | "HistoricInput" => {
                        requires_lifetime_parameter = true;
                        match &mut first_segment.arguments {
                            PathArguments::AngleBracketed(arguments)
                                if arguments.args.len() == 2 =>
                            {
                                pop_string_argument(arguments);
                                if first_segment.ident == "HistoricInput" {
                                    let data_type = get_data_type(arguments);
                                    into_reference_with_lifetime(data_type, None);
                                } else {
                                    prepend_lifetime_argument(arguments);
                                }
                            }
                            _ => abort!(first_segment, "expected exactly two generic parameters"),
                        }
                    }
                    "MainOutput" => {}
                    "HardwareInterface" => {
                        requires_lifetime_parameter = true;
                        requires_hardware_interface_parameter = true;
                        field.ty = Type::Reference(TypeReference {
                            and_token: Default::default(),
                            lifetime: Some(Lifetime::new("'context", Span::call_site())),
                            mutability: None,
                            elem: Box::new(Type::Path(TypePath {
                                qself: None,
                                path: Path {
                                    leading_colon: None,
                                    segments: Punctuated::from_iter([
                                        PathSegment {
                                            ident: format_ident!("std"),
                                            arguments: PathArguments::None,
                                        },
                                        PathSegment {
                                            ident: format_ident!("sync"),
                                            arguments: PathArguments::None,
                                        },
                                        PathSegment {
                                            ident: format_ident!("Arc"),
                                            arguments: PathArguments::AngleBracketed(
                                                AngleBracketedGenericArguments {
                                                    colon2_token: None,
                                                    lt_token: Default::default(),
                                                    args: Punctuated::from_iter([
                                                        GenericArgument::Type(Type::Path(
                                                            TypePath {
                                                                qself: None,
                                                                path: Path {
                                                                    leading_colon: None,
                                                                    segments: Punctuated::from_iter(
                                                                        [PathSegment {
                                                                            ident: format_ident!(
                                                                                "Interface"
                                                                            ),
                                                                            arguments:
                                                                                PathArguments::None,
                                                                        }],
                                                                    ),
                                                                },
                                                            },
                                                        )),
                                                    ]),
                                                    gt_token: Default::default(),
                                                },
                                            ),
                                        },
                                    ]),
                                },
                            })),
                        });
                    }
                    _ => {
                        abort!(first_segment.ident, "unexpected identifier")
                    }
                }
            }
            _ => abort!(field.ty, "expected type path"),
        }
    }

    if requires_lifetime_parameter {
        struct_item.generics.params.insert(
            0,
            GenericParam::Lifetime(LifetimeDef::new(Lifetime::new(
                "'context",
                Span::call_site(),
            ))),
        );
    }
    if requires_hardware_interface_parameter {
        struct_item
            .generics
            .params
            .push(GenericParam::Type(TypeParam {
                attrs: Default::default(),
                ident: format_ident!("Interface"),
                colon_token: None,
                bounds: Default::default(),
                eq_token: None,
                default: None,
            }));
    }

    let struct_name = struct_item.ident.clone();
    let struct_generics = struct_item.generics.clone();
    let field_names: Vec<_> = struct_item
        .fields
        .iter()
        .map(|field| field.ident.clone())
        .collect();
    let field_types: Vec<_> = struct_item
        .fields
        .iter()
        .map(|field| field.ty.clone())
        .collect();
    let new_method_stream = quote! {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            #(#field_names: #field_types),*
        ) -> Self {
            Self {
                #(#field_names),*
            }
        }
    };

    let struct_stream = struct_item.into_token_stream();
    quote! {
        #struct_stream

        impl #struct_generics #struct_name #struct_generics {
            #new_method_stream
        }
    }
    .into()
}

fn pop_string_argument(arguments: &mut AngleBracketedGenericArguments) {
    match arguments.args.pop() {
        Some(
            Pair::End(GenericArgument::Const(Expr::Lit(ExprLit {
                lit: Lit::Str(_), ..
            })))
            | Pair::Punctuated(
                GenericArgument::Const(Expr::Lit(ExprLit {
                    lit: Lit::Str(_), ..
                })),
                _,
            ),
        ) => {}
        Some(argument) => {
            abort!(argument, "expected string literal");
        }
        _ => {
            abort!(arguments, "expected exactly at least one generic parameter");
        }
    }
}

fn prepend_lifetime_argument(arguments: &mut AngleBracketedGenericArguments) {
    arguments.args.insert(
        0,
        GenericArgument::Lifetime(Lifetime::new("'context", Span::call_site())),
    );
}

fn get_data_type(arguments: &mut AngleBracketedGenericArguments) -> &mut Type {
    let span = arguments.span();
    match arguments.args.first_mut().unwrap() {
        GenericArgument::Type(data_type) => data_type,
        _ => abort!(span, "expected type path in first generic parameter"),
    }
}

fn into_reference_with_lifetime(data_type: &mut Type, mutability: Option<Mut>) {
    let data_type = match data_type {
        Type::Path(TypePath {
            path: Path { segments, .. },
            ..
        }) if !segments.is_empty() && segments.last().unwrap().ident == "Option" => {
            match &mut segments.last_mut().unwrap().arguments {
                PathArguments::AngleBracketed(arguments) if arguments.args.len() == 1 => {
                    match arguments.args.first_mut().unwrap() {
                        GenericArgument::Type(data_type) => data_type,
                        _ => data_type,
                    }
                }
                _ => data_type,
            }
        }
        _ => data_type,
    };
    *data_type = Type::Reference(TypeReference {
        and_token: Default::default(),
        lifetime: Some(Lifetime::new("'context", Span::call_site())),
        mutability,
        elem: Box::new(data_type.clone()),
    });
}

fn embed_into_vec(data_type: &mut Type) {
    *data_type = Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments: Punctuated::from_iter([PathSegment {
                ident: format_ident!("Vec"),
                arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    colon2_token: None,
                    lt_token: Default::default(),
                    args: Punctuated::from_iter([GenericArgument::Type(data_type.clone())]),
                    gt_token: Default::default(),
                }),
            }]),
        },
    });
}

fn unwrap_option(data_type: &mut Type) {
    *data_type =
        match data_type {
            Type::Path(TypePath {
                path: syn::Path { segments, .. },
                ..
            }) if !segments.is_empty() && segments.last().unwrap().ident == "Option" => {
                match &segments.last().unwrap().arguments {
                    PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                        args, ..
                    }) if args.len() == 1 => match args.first().unwrap() {
                        GenericArgument::Type(nested_data_type) => nested_data_type.clone(),
                        _ => abort!(
                            args.first(),
                            "unexpected generic argument, expected type argument in data type"
                        ),
                    },
                    arguments => abort!(
                        arguments,
                        "expected exactly one generic type argument in data type"
                    ),
                }
            }
            _ => abort!(data_type, "expected Option<T> as data type"),
        };
}
