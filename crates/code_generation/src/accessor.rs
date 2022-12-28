use convert_case::{Case, Casing};
use proc_macro2::{Delimiter, Group, Punct, Spacing, TokenStream, TokenTree};
use quote::{format_ident, TokenStreamExt};
use source_analyzer::PathSegment;

use super::reference_type::ReferenceType;

pub fn path_to_accessor_token_stream(
    prefix_token_stream: TokenStream,
    path: &[PathSegment],
    reference_type: ReferenceType,
    instance: TokenStream,
    cycler_instance_prefix: TokenStream,
    cycler_instances: &[String],
) -> TokenStream {
    fn path_to_accessor_token_stream_with_cycler_instance(
        prefix_token_stream: TokenStream,
        path: &[PathSegment],
        reference_type: ReferenceType,
        cycler_instance: Option<&str>,
    ) -> TokenStream {
        let mut token_stream = TokenStream::default();
        let mut token_stream_within_method = None;

        let path_contains_optional = path.iter().any(|segment| segment.is_optional);
        if !path_contains_optional {
            token_stream.append(TokenTree::Punct(Punct::new('&', Spacing::Alone)));
            if ReferenceType::Mutable == reference_type {
                token_stream.append(TokenTree::Ident(format_ident!("mut")));
            }
        }

        token_stream.extend(prefix_token_stream);

        for (index, segment) in path.iter().enumerate() {
            {
                let token_stream = token_stream_within_method
                    .as_mut()
                    .unwrap_or(&mut token_stream);

                token_stream.append(TokenTree::Punct(Punct::new('.', Spacing::Alone)));
                match (segment.is_variable, cycler_instance) {
                    (true, Some(cycler_instance)) => {
                        token_stream.append(TokenTree::Ident(format_ident!(
                            "{}",
                            cycler_instance.to_case(Case::Snake)
                        )));
                    }
                    _ => {
                        token_stream.append(TokenTree::Ident(format_ident!("{}", segment.name)));
                    }
                }
            }

            let is_last_segment = index == path.len() - 1;
            if segment.is_optional {
                match token_stream_within_method.take() {
                    Some(mut token_stream_within_method) => {
                        token_stream_within_method
                            .append(TokenTree::Punct(Punct::new('.', Spacing::Alone)));
                        match reference_type {
                            ReferenceType::Immutable => token_stream_within_method
                                .append(TokenTree::Ident(format_ident!("as_ref"))),
                            ReferenceType::Mutable => token_stream_within_method
                                .append(TokenTree::Ident(format_ident!("as_mut"))),
                        }
                        token_stream_within_method.append(TokenTree::Group(Group::new(
                            Delimiter::Parenthesis,
                            TokenStream::default(),
                        )));

                        token_stream.append(TokenTree::Group(Group::new(
                            Delimiter::Parenthesis,
                            token_stream_within_method,
                        )));
                    }
                    None => {
                        token_stream.append(TokenTree::Punct(Punct::new('.', Spacing::Alone)));
                        match reference_type {
                            ReferenceType::Immutable => {
                                token_stream.append(TokenTree::Ident(format_ident!("as_ref")))
                            }
                            ReferenceType::Mutable => {
                                token_stream.append(TokenTree::Ident(format_ident!("as_mut")))
                            }
                        }
                        token_stream.append(TokenTree::Group(Group::new(
                            Delimiter::Parenthesis,
                            TokenStream::default(),
                        )));
                    }
                }

                if !is_last_segment {
                    token_stream.append(TokenTree::Punct(Punct::new('.', Spacing::Alone)));
                    let next_segments_contain_optional = path
                        .iter()
                        .skip(index + 1)
                        .any(|segment| segment.is_optional);
                    let method_name = match next_segments_contain_optional {
                        true => "and_then",
                        false => "map",
                    };
                    token_stream.append(TokenTree::Ident(format_ident!("{}", method_name)));

                    let mut new_token_stream_within_method = TokenStream::default();
                    new_token_stream_within_method
                        .append(TokenTree::Punct(Punct::new('|', Spacing::Alone)));
                    new_token_stream_within_method
                        .append(TokenTree::Ident(format_ident!("{}", segment.name)));
                    new_token_stream_within_method
                        .append(TokenTree::Punct(Punct::new('|', Spacing::Alone)));
                    if !next_segments_contain_optional {
                        new_token_stream_within_method
                            .append(TokenTree::Punct(Punct::new('&', Spacing::Alone)));
                        if ReferenceType::Mutable == reference_type {
                            new_token_stream_within_method
                                .append(TokenTree::Ident(format_ident!("mut")));
                        }
                    }
                    new_token_stream_within_method
                        .append(TokenTree::Ident(format_ident!("{}", segment.name)));
                    token_stream_within_method = Some(new_token_stream_within_method);
                }
            }
        }

        if let Some(token_stream_within_method) = token_stream_within_method.take() {
            token_stream.append(TokenTree::Group(Group::new(
                Delimiter::Parenthesis,
                token_stream_within_method,
            )));
        }

        token_stream
    }

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
            token_stream_within_match.extend(path_to_accessor_token_stream_with_cycler_instance(
                prefix_token_stream.clone(),
                path,
                reference_type,
                Some(cycler_instance),
            ));
            token_stream_within_match.append(TokenTree::Punct(Punct::new(',', Spacing::Alone)));
        }
        token_stream.append(TokenTree::Group(Group::new(
            Delimiter::Brace,
            token_stream_within_match,
        )));
        token_stream
    } else {
        path_to_accessor_token_stream_with_cycler_instance(
            prefix_token_stream,
            path,
            reference_type,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use quote::quote;

    #[test]
    fn paths_with_optionals_result_in_correct_accessor_token_streams() {
        let cases = [
            ("a", ReferenceType::Immutable, quote! { &prefix.a }),
            (
                "$cycler_instance",
                ReferenceType::Immutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => &prefix.instance_a, CyclerInstance::InstanceB => &prefix.instance_b, } },
            ),
            ("a", ReferenceType::Mutable, quote! { &mut prefix.a }),
            (
                "$cycler_instance",
                ReferenceType::Mutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => &mut prefix.instance_a, CyclerInstance::InstanceB => &mut prefix.instance_b, } },
            ),
            ("a/b", ReferenceType::Immutable, quote! { &prefix.a.b }),
            (
                "a/$cycler_instance",
                ReferenceType::Immutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => &prefix.a.instance_a, CyclerInstance::InstanceB => &prefix.a.instance_b, } },
            ),
            ("a/b", ReferenceType::Mutable, quote! { &mut prefix.a.b }),
            (
                "a/$cycler_instance",
                ReferenceType::Mutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => &mut prefix.a.instance_a, CyclerInstance::InstanceB => &mut prefix.a.instance_b, } },
            ),
            ("a/b/c", ReferenceType::Immutable, quote! { &prefix.a.b.c }),
            (
                "a/b/c",
                ReferenceType::Mutable,
                quote! { &mut prefix.a.b.c },
            ),
            (
                "a?/b/c",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().map(|a| &a.b.c) },
            ),
            (
                "a?/b/c",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().map(|a| &mut a.b.c) },
            ),
            ("a?", ReferenceType::Immutable, quote! { prefix.a.as_ref() }),
            (
                "$cycler_instance?",
                ReferenceType::Immutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => prefix.instance_a.as_ref(), CyclerInstance::InstanceB => prefix.instance_b.as_ref(), } },
            ),
            ("a?", ReferenceType::Mutable, quote! { prefix.a.as_mut() }),
            (
                "$cycler_instance?",
                ReferenceType::Mutable,
                quote! { match self.instance_name { CyclerInstance::InstanceA => prefix.instance_a.as_mut(), CyclerInstance::InstanceB => prefix.instance_b.as_mut(), } },
            ),
            (
                "a?/b?/c",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.as_ref()).map(|b| &b.c) },
            ),
            (
                "a?/b?/c",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.as_mut()).map(|b| &mut b.c) },
            ),
            (
                "a?/b?/c?",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.as_ref()).and_then(|b| b.c.as_ref()) },
            ),
            (
                "a?/b?/c?",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.as_mut()).and_then(|b| b.c.as_mut()) },
            ),
            (
                "a?/b?/c?/d",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.as_ref()).and_then(|b| b.c.as_ref()).map(|c| &c.d) },
            ),
            (
                "a?/b?/c?/d",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.as_mut()).and_then(|b| b.c.as_mut()).map(|c| &mut c.d) },
            ),
            (
                "a?/b?/c?/d?",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.as_ref()).and_then(|b| b.c.as_ref()).and_then(|c| c.d.as_ref()) },
            ),
            (
                "a?/b?/c?/d?",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.as_mut()).and_then(|b| b.c.as_mut()).and_then(|c| c.d.as_mut()) },
            ),
            (
                "a?/b/c/d?",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.c.d.as_ref()) },
            ),
            (
                "a?/b/c/d?",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.c.d.as_mut()) },
            ),
            (
                "a?/b/c/d",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().map(|a| &a.b.c.d) },
            ),
            (
                "a?/b/c/d",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().map(|a| &mut a.b.c.d) },
            ),
            (
                "a?/b/c?/d",
                ReferenceType::Immutable,
                quote! { prefix.a.as_ref().and_then(|a| a.b.c.as_ref()).map(|c| &c.d) },
            ),
            (
                "a?/b/c?/d",
                ReferenceType::Mutable,
                quote! { prefix.a.as_mut().and_then(|a| a.b.c.as_mut()).map(|c| &mut c.d) },
            ),
            (
                "a/b/c?/d",
                ReferenceType::Immutable,
                quote! { prefix.a.b.c.as_ref().map(|c| &c.d) },
            ),
            (
                "a/b/c?/d",
                ReferenceType::Mutable,
                quote! { prefix.a.b.c.as_mut().map(|c| &mut c.d) },
            ),
            (
                "a/b/c/d",
                ReferenceType::Immutable,
                quote! { &prefix.a.b.c.d },
            ),
            (
                "a/b/c/d",
                ReferenceType::Mutable,
                quote! { &mut prefix.a.b.c.d },
            ),
            (
                "a/b?/c?/d",
                ReferenceType::Immutable,
                quote! { prefix.a.b.as_ref().and_then(|b| b.c.as_ref()).map(|c| &c.d) },
            ),
            (
                "a/b?/c?/d",
                ReferenceType::Mutable,
                quote! { prefix.a.b.as_mut().and_then(|b| b.c.as_mut()).map(|c| &mut c.d) },
            ),
            (
                "a/b?/c?/d?",
                ReferenceType::Immutable,
                quote! { prefix.a.b.as_ref().and_then(|b| b.c.as_ref()).and_then(|c| c.d.as_ref()) },
            ),
            (
                "a/b?/c?/d?",
                ReferenceType::Mutable,
                quote! { prefix.a.b.as_mut().and_then(|b| b.c.as_mut()).and_then(|c| c.d.as_mut()) },
            ),
            (
                "a/b/c/d/e/f?/g/i/j/k/l/m/n",
                ReferenceType::Immutable,
                quote! { prefix.a.b.c.d.e.f.as_ref().map(|f| &f.g.i.j.k.l.m.n) },
            ),
            (
                "a/b/c/d/e/f?/g/i/j/k/l/m/n",
                ReferenceType::Mutable,
                quote! { prefix.a.b.c.d.e.f.as_mut().map(|f| &mut f.g.i.j.k.l.m.n) },
            ),
            (
                "a/b/c/d/e/f?/g/i/j/k/l/m/n?",
                ReferenceType::Immutable,
                quote! { prefix.a.b.c.d.e.f.as_ref().and_then(|f| f.g.i.j.k.l.m.n.as_ref()) },
            ),
            (
                "a/b/c/d/e/f?/g/i/j/k/l/m/n?",
                ReferenceType::Mutable,
                quote! { prefix.a.b.c.d.e.f.as_mut().and_then(|f| f.g.i.j.k.l.m.n.as_mut()) },
            ),
        ];

        for (path, reference_type, expected_token_stream) in cases {
            let path_segments: Vec<_> = path.split('/').map(PathSegment::from).collect();

            let token_stream = path_to_accessor_token_stream(
                quote! { prefix },
                &path_segments,
                reference_type,
                quote! { self.instance_name },
                quote! { CyclerInstance:: },
                &["InstanceA".to_string(), "InstanceB".to_string()],
            );
            assert_eq!(
                token_stream.to_string(),
                expected_token_stream.to_string(),
                "path: {path:?}"
            );
        }
    }
}
