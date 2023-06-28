use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use source_analyzer::{
    cycler::{Cycler, InstanceName},
    path::Path,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReferenceKind {
    Immutable,
    Mutable,
}

pub fn path_to_accessor_token_stream(
    prefix: TokenStream,
    path: &Path,
    reference_type: ReferenceKind,
    cycler: &Cycler,
) -> TokenStream {
    if let Some(segment) = path
        .segments
        .iter()
        .find(|segment| segment.is_variable && segment.name != "cycler_instance")
    {
        let name = &segment.name;
        unimplemented!("unexpected `${name}` only $cycler_instance can be used as variable in path",)
    }
    if path.contains_variable() {
        let variants = cycler.instances.iter().map(|instance| {
            let instance_name = format_ident!("{}", instance.name);
            let accessor_path = path_to_accessor_token_stream_with_cycler_instance(
                prefix.clone(),
                path,
                reference_type,
                Some(&instance.name),
            );
            quote! {
                CyclerInstance::#instance_name => #accessor_path,
            }
        });
        quote! {
            match instance {
                #(#variants)*
            }
        }
    } else {
        path_to_accessor_token_stream_with_cycler_instance(prefix, path, reference_type, None)
    }
}

fn path_to_accessor_token_stream_with_cycler_instance(
    prefix: TokenStream,
    path: &Path,
    reference_type: ReferenceKind,
    cycler_instance: Option<&InstanceName>,
) -> TokenStream {
    let segments = path.segments.iter().map(|segment| {
        let field = match segment.is_variable {
            true => format_ident!("{}", cycler_instance.unwrap().to_case(Case::Snake)),
            false => format_ident!("{}", segment.name),
        };
        match segment.is_optional {
            true => match reference_type {
                ReferenceKind::Immutable => quote! { #field.as_ref()? },
                ReferenceKind::Mutable => quote! { #field.as_mut()? },
            },
            false => quote! { #field },
        }
    });
    if path.contains_optional() {
        quote! {
            (|| Some(#prefix . #(#segments).*))()
        }
    } else {
        let reference = match reference_type {
            ReferenceKind::Immutable => quote! {& },
            ReferenceKind::Mutable => quote! {&mut },
        };
        quote! {
            #reference #prefix . #(#segments).*
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use quote::quote;
    use source_analyzer::cycler::{CyclerKind, Instance};

    #[test]
    fn paths_with_optionals_result_in_correct_accessor_token_streams() {
        let cases = [
            ("a", ReferenceKind::Immutable, quote! { &prefix.a }),
            (
                "$cycler_instance",
                ReferenceKind::Immutable,
                quote! { match instance { CyclerInstance::InstanceA => &prefix.instance_a, CyclerInstance::InstanceB => &prefix.instance_b, } },
            ),
            ("a", ReferenceKind::Mutable, quote! { &mut prefix.a }),
            (
                "$cycler_instance",
                ReferenceKind::Mutable,
                quote! { match instance { CyclerInstance::InstanceA => &mut prefix.instance_a, CyclerInstance::InstanceB => &mut prefix.instance_b, } },
            ),
            ("a.b", ReferenceKind::Immutable, quote! { &prefix.a.b }),
            (
                "a.$cycler_instance",
                ReferenceKind::Immutable,
                quote! { match instance { CyclerInstance::InstanceA => &prefix.a.instance_a, CyclerInstance::InstanceB => &prefix.a.instance_b, } },
            ),
            ("a.b", ReferenceKind::Mutable, quote! { &mut prefix.a.b }),
            (
                "a.$cycler_instance",
                ReferenceKind::Mutable,
                quote! { match instance { CyclerInstance::InstanceA => &mut prefix.a.instance_a, CyclerInstance::InstanceB => &mut prefix.a.instance_b, } },
            ),
            ("a.b.c", ReferenceKind::Immutable, quote! { &prefix.a.b.c }),
            (
                "a.b.c",
                ReferenceKind::Mutable,
                quote! { &mut prefix.a.b.c },
            ),
            (
                "a?.b.c",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.as_ref()?.b.c)) () },
            ),
            (
                "a?.b.c",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.as_mut()?.b.c)) () },
            ),
            (
                "a?",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.as_ref()?)) () },
            ),
            (
                "$cycler_instance?",
                ReferenceKind::Immutable,
                quote! { match instance { CyclerInstance::InstanceA => (|| Some(prefix.instance_a.as_ref()?)) (), CyclerInstance::InstanceB => (|| Some(prefix.instance_b.as_ref()?)) (), } },
            ),
            (
                "a?",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.as_mut()?)) () },
            ),
            (
                "$cycler_instance?",
                ReferenceKind::Mutable,
                quote! { match instance { CyclerInstance::InstanceA => (|| Some(prefix.instance_a.as_mut()?)) (), CyclerInstance::InstanceB => (|| Some(prefix.instance_b.as_mut()?)) (), } },
            ),
            (
                "a?.b?.c",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.as_ref()?.b.as_ref()?.c)) () },
            ),
            (
                "a?.b?.c",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.as_mut()?.b.as_mut()?.c)) () },
            ),
            (
                "a?.b?.c?",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.as_ref()?.b.as_ref()?.c.as_ref()?)) () },
            ),
            (
                "a?.b?.c?",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.as_mut()?.b.as_mut()?.c.as_mut()?)) () },
            ),
            (
                "a?.b?.c?.d",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.as_ref()?.b.as_ref()?.c.as_ref()?.d)) () },
            ),
            (
                "a?.b?.c?.d",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.as_mut()?.b.as_mut()?.c.as_mut()?.d)) () },
            ),
            (
                "a?.b?.c?.d?",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.as_ref()?.b.as_ref()?.c.as_ref()?.d.as_ref()?)) () },
            ),
            (
                "a?.b?.c?.d?",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.as_mut()?.b.as_mut()?.c.as_mut()?.d.as_mut()?)) () },
            ),
            (
                "a?.b.c.d?",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.as_ref()?.b.c.d.as_ref()?)) () },
            ),
            (
                "a?.b.c.d?",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.as_mut()?.b.c.d.as_mut()?)) () },
            ),
            (
                "a?.b.c.d",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.as_ref()?.b.c.d)) () },
            ),
            (
                "a?.b.c.d",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.as_mut()?.b.c.d)) () },
            ),
            (
                "a?.b.c?.d",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.as_ref()?.b.c.as_ref()?.d)) () },
            ),
            (
                "a?.b.c?.d",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.as_mut()?.b.c.as_mut()?.d)) () },
            ),
            (
                "a.b.c?.d",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.b.c.as_ref()?.d)) () },
            ),
            (
                "a.b.c?.d",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.b.c.as_mut()?.d)) () },
            ),
            (
                "a.b.c.d",
                ReferenceKind::Immutable,
                quote! { &prefix.a.b.c.d },
            ),
            (
                "a.b.c.d",
                ReferenceKind::Mutable,
                quote! { &mut prefix.a.b.c.d },
            ),
            (
                "a.b?.c?.d",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.b.as_ref()?.c.as_ref()?.d)) () },
            ),
            (
                "a.b?.c?.d",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.b.as_mut()?.c.as_mut()?.d)) () },
            ),
            (
                "a.b?.c?.d?",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.b.as_ref()?.c.as_ref()?.d.as_ref()?)) () },
            ),
            (
                "a.b?.c?.d?",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.b.as_mut()?.c.as_mut()?.d.as_mut()?)) () },
            ),
            (
                "a.b.c.d.e.f?.g.i.j.k.l.m.n",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.b.c.d.e.f.as_ref()?.g.i.j.k.l.m.n)) () },
            ),
            (
                "a.b.c.d.e.f?.g.i.j.k.l.m.n",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.b.c.d.e.f.as_mut()?.g.i.j.k.l.m.n)) () },
            ),
            (
                "a.b.c.d.e.f?.g.i.j.k.l.m.n?",
                ReferenceKind::Immutable,
                quote! { (|| Some(prefix.a.b.c.d.e.f.as_ref()?.g.i.j.k.l.m.n.as_ref()?)) () },
            ),
            (
                "a.b.c.d.e.f?.g.i.j.k.l.m.n?",
                ReferenceKind::Mutable,
                quote! { (|| Some(prefix.a.b.c.d.e.f.as_mut()?.g.i.j.k.l.m.n.as_mut()?)) () },
            ),
        ];
        let cycler = Cycler {
            name: "TestCycler".to_string(),
            kind: CyclerKind::RealTime,
            instances: vec![
                Instance {
                    name: "InstanceA".to_string(),
                },
                Instance {
                    name: "InstanceB".to_string(),
                },
            ],
            module: "test_cycler".to_string(),
            nodes: vec![],
        };

        for (path, reference_type, expected_token_stream) in cases {
            let path = Path::from(path);

            let token_stream =
                path_to_accessor_token_stream(quote! { prefix }, &path, reference_type, &cycler);

            assert_eq!(
                token_stream.to_string(),
                expected_token_stream.to_string(),
                "path: {path:?}"
            );
        }
    }
}
