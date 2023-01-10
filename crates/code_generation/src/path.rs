use std::iter::once;

use itertools::intersperse;
use proc_macro2::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use quote::{format_ident, TokenStreamExt};
use source_analyzer::PathSegment;

pub fn path_to_path_string_token_stream(
    path: &[PathSegment],
    path_prefix: &str,
    instance: TokenStream,
    cycler_instance_prefix: TokenStream,
    cycler_instances: &[String],
) -> TokenStream {
    for segment in path.iter() {
        if segment.is_variable && segment.name != "cycler_instance" {
            unimplemented!("only $cycler_instance is implemented");
        }
    }
    let path_contains_variable = path.iter().any(|segment| segment.is_variable);
    if path_contains_variable {
        let mut token_stream = TokenStream::default();
        token_stream.append(TokenTree::Ident(format_ident!("match")));
        token_stream.extend(instance);
        let mut token_stream_within_match = TokenStream::default();
        for cycler_instance in cycler_instances {
            token_stream_within_match.extend(cycler_instance_prefix.clone());
            token_stream_within_match.append(format_ident!("{}", cycler_instance));
            token_stream_within_match.append(TokenTree::Punct(Punct::new('=', Spacing::Joint)));
            token_stream_within_match.append(TokenTree::Punct(Punct::new('>', Spacing::Alone)));
            token_stream_within_match.extend(
                path_to_path_string_token_stream_with_cycler_instance(
                    path,
                    path_prefix,
                    Some(cycler_instance),
                ),
            );
            token_stream_within_match.append(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
        }
        token_stream.append(TokenTree::Group(Group::new(
            Delimiter::Brace,
            token_stream_within_match,
        )));
        token_stream
    } else {
        path_to_path_string_token_stream_with_cycler_instance(path, path_prefix, None)
    }
}

fn path_to_path_string_token_stream_with_cycler_instance(
    path: &[PathSegment],
    path_prefix: &str,
    cycler_instance: Option<&str>,
) -> TokenStream {
    let path_string: String = intersperse(
        once(path_prefix.to_string()).chain(path.iter().map(|segment| match segment.is_variable {
            true => cycler_instance.unwrap().to_string(),
            false => segment.name.clone(),
        })),
        ".".to_string(),
    )
    .collect();
    TokenTree::Literal(Literal::string(&path_string)).into()
}
