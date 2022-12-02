use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, ToTokens};
use syn::{
    parse_macro_input,
    punctuated::{Pair, Punctuated},
    spanned::Spanned,
    token::Mut,
    AngleBracketedGenericArguments, Expr, ExprLit, GenericArgument, GenericParam, ItemStruct,
    Lifetime, LifetimeDef, Lit, Path, PathArguments, PathSegment, PredicateType, TraitBound,
    TraitBoundModifier, Type, TypeParam, TypeParamBound, TypePath, TypeReference, WhereClause,
    WherePredicate,
};

#[proc_macro_attribute]
#[proc_macro_error]
pub fn context(_attributes: TokenStream, input: TokenStream) -> TokenStream {
    let mut struct_item = parse_macro_input!(input as ItemStruct);

    let mut requires_lifetime_parameter = false;
    let mut requires_hardware_interface_parameter = false;

    for field in struct_item.fields.iter_mut() {
        match &mut field.ty {
            Type::Path(path) => {
                let first_segment = match path.path.segments.first_mut() {
                    Some(segment) => segment,
                    None => abort!(path, "expected type path with at least one segment"),
                };
                match first_segment.ident.to_string().as_str() {
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
                            pop_string_argument(arguments);
                            let has_additional_argument = arguments.args.len() == 2;
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
                                if first_segment.ident == "PersistentState" {
                                    Some(Default::default())
                                } else {
                                    None
                                },
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
                    "CyclerInstance" | "MainOutput" => {}
                    "HardwareInterface" => {
                        // TODO: maybe remove reference of Arc
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
        struct_item.generics.where_clause = Some(WhereClause {
            where_token: Default::default(),
            predicates: Punctuated::from_iter([WherePredicate::Type(PredicateType {
                lifetimes: None,
                bounded_ty: Type::Path(TypePath {
                    qself: None,
                    path: Path {
                        leading_colon: None,
                        segments: Punctuated::from_iter([PathSegment {
                            ident: format_ident!("Interface"),
                            arguments: PathArguments::None,
                        }]),
                    },
                }),
                colon_token: Default::default(),
                bounds: Punctuated::from_iter([TypeParamBound::Trait(TraitBound {
                    paren_token: None,
                    modifier: TraitBoundModifier::None,
                    lifetimes: None,
                    path: Path {
                        leading_colon: None,
                        segments: Punctuated::from_iter([
                            PathSegment {
                                ident: format_ident!("types"),
                                arguments: PathArguments::None,
                            },
                            PathSegment {
                                ident: format_ident!("hardware"),
                                arguments: PathArguments::None,
                            },
                            PathSegment {
                                ident: format_ident!("Interface"),
                                arguments: PathArguments::None,
                            },
                        ]),
                    },
                })]),
            })]),
        })
    }

    struct_item.into_token_stream().into()
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
