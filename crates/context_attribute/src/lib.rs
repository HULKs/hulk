use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{abort, proc_macro_error};
use quote::{format_ident, ToTokens};
use syn::{
    parse_macro_input,
    punctuated::{Pair, Punctuated},
    AngleBracketedGenericArguments, Expr, ExprLit, GenericArgument, GenericParam, ItemStruct,
    Lifetime, LifetimeDef, Lit, Path, PathArguments, PathSegment, PredicateType, TraitBound,
    TraitBoundModifier, Type, TypeParam, TypeParamBound, TypePath, WhereClause, WherePredicate,
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
                    "PerceptionInput" => {
                        requires_lifetime_parameter = true;
                        match &mut first_segment.arguments {
                            PathArguments::AngleBracketed(arguments) => {
                                if arguments.args.len() != 3 {
                                    abort!(arguments, "expected exactly three generic parameters");
                                }
                                match arguments.args.pop() {
                                    Some(Pair::End(GenericArgument::Const(Expr::Lit(
                                        ExprLit {
                                            lit: Lit::Str(_), ..
                                        },
                                    )))) => {}
                                    Some(argument) => {
                                        abort!(
                                            argument,
                                            "expected string literal in third generic parameter"
                                        );
                                    }
                                    _ => {
                                        abort!(
                                            arguments,
                                            "expected exactly three generic parameters"
                                        );
                                    }
                                }
                                match arguments.args.pop() {
                                    Some(Pair::End(GenericArgument::Const(Expr::Lit(
                                        ExprLit {
                                            lit: Lit::Str(_), ..
                                        },
                                    )))) => {}
                                    Some(argument) => {
                                        abort!(
                                            argument,
                                            "expected string literal in second generic parameter"
                                        );
                                    }
                                    _ => {
                                        abort!(
                                            arguments,
                                            "expected exactly three generic parameters"
                                        );
                                    }
                                }
                                arguments.args.insert(
                                    0,
                                    GenericArgument::Lifetime(Lifetime::new(
                                        "'context",
                                        Span::call_site(),
                                    )),
                                );
                            }
                            _ => abort!(first_segment, "expected exactly three generic parameters"),
                        }
                    }
                    "AdditionalOutput" | "HistoricInput" | "RequiredInput" | "OptionalInput"
                    | "Parameter" | "PersistentState" => {
                        requires_lifetime_parameter = true;
                        match &mut first_segment.arguments {
                            PathArguments::AngleBracketed(arguments) => {
                                if arguments.args.len() != 2 {
                                    abort!(arguments, "expected exactly two generic parameters");
                                }
                                match arguments.args.pop() {
                                    Some(Pair::End(GenericArgument::Const(Expr::Lit(
                                        ExprLit {
                                            lit: Lit::Str(_), ..
                                        },
                                    )))) => {}
                                    Some(argument) => {
                                        abort!(
                                            argument,
                                            "expected string literal in second generic parameter"
                                        );
                                    }
                                    _ => {
                                        abort!(
                                            arguments,
                                            "expected exactly two generic parameters"
                                        );
                                    }
                                }
                                arguments.args.insert(
                                    0,
                                    GenericArgument::Lifetime(Lifetime::new(
                                        "'context",
                                        Span::call_site(),
                                    )),
                                );
                            }
                            _ => abort!(first_segment, "expected exactly two generic parameters"),
                        }
                    }
                    "MainOutput" => {}
                    "HardwareInterface" => {
                        requires_lifetime_parameter = true;
                        requires_hardware_interface_parameter = true;
                        first_segment.arguments =
                            PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                                colon2_token: None,
                                lt_token: Default::default(),
                                args: Punctuated::from_iter([
                                    GenericArgument::Lifetime(Lifetime::new(
                                        "'context",
                                        Span::call_site(),
                                    )),
                                    GenericArgument::Type(Type::Path(TypePath {
                                        qself: None,
                                        path: Path {
                                            leading_colon: None,
                                            segments: Punctuated::from_iter([PathSegment {
                                                ident: format_ident!("Interface"),
                                                arguments: PathArguments::None,
                                            }]),
                                        },
                                    })),
                                ]),
                                gt_token: Default::default(),
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
                                ident: format_ident!("hardware"),
                                arguments: PathArguments::None,
                            },
                            PathSegment {
                                ident: format_ident!("HardwareInterface"),
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
