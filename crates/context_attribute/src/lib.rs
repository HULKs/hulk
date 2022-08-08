use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_error::{abort, proc_macro_error};
use quote::ToTokens;
use syn::{
    parse_macro_input, punctuated::Pair, token::Pub, Expr, ExprLit, GenericArgument, GenericParam,
    ItemStruct, Lifetime, LifetimeDef, Lit, PathArguments, Type, VisPublic, Visibility,
};

#[proc_macro_attribute]
#[proc_macro_error]
pub fn context(_attributes: TokenStream, input: TokenStream) -> TokenStream {
    let mut struct_item = parse_macro_input!(input as ItemStruct);

    match &mut struct_item.vis {
        Visibility::Public(..) => {}
        _ => {
            struct_item.vis = Visibility::Public(VisPublic {
                pub_token: Pub {
                    span: Span::call_site(),
                },
            });
        }
    }

    let mut requires_lifetime_parameter = false;

    for field in struct_item.fields.iter_mut() {
        match &mut field.vis {
            Visibility::Public(..) => {}
            _ => {
                field.vis = Visibility::Public(VisPublic {
                    pub_token: Pub {
                        span: Span::call_site(),
                    },
                });
            }
        }

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

    struct_item.into_token_stream().into()
}
