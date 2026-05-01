//! Tests for public generated Rust API and protocol wiring.
//!
//! Run `cargo insta review -p ros-z-codegen` to inspect and accept snapshot changes.

use std::{collections::HashSet, path::PathBuf};

use quote::ToTokens;
use ros_z_codegen::{
    generator::rust::{
        GenerationContext, generate_action_impl, generate_message_impl_with_cdr,
        generate_service_impl,
    },
    resolver::Resolver,
    types::ResolvedMessage,
};

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/jazzy/test_interface_files")
}

fn resolve_corpus() -> Vec<ResolvedMessage> {
    let parsed = ros_z_codegen::discovery::discover_messages(&corpus_dir(), "test_interface_files")
        .expect("discover messages")
        .into_iter()
        .filter(|m| m.name != "WStrings")
        .collect::<Vec<_>>();

    let mut resolver = Resolver::new();
    resolver.resolve_messages(parsed).expect("resolve messages")
}

fn ctx() -> GenerationContext {
    let mut local = HashSet::new();
    local.insert("test_interface_files".to_string());
    GenerationContext::new(None, local)
}

fn format_tokens(ts: proc_macro2::TokenStream) -> String {
    let file: syn::File = syn::parse2(ts).expect("parse generated TokenStream");
    prettyplease::unparse(&file)
}

fn generated_file_for_message(msgs: &[ResolvedMessage], name: &str) -> syn::File {
    let msg = msgs
        .iter()
        .find(|m| m.parsed.name == name)
        .unwrap_or_else(|| panic!("ResolvedMessage '{name}' not found"));
    let tokens = generate_message_impl_with_cdr(msg, &ctx(), &HashSet::new())
        .unwrap_or_else(|error| panic!("generate {name}: {error}"));
    syn::parse2(tokens).expect("generated message should parse as Rust")
}

fn public_fields(file: &syn::File, struct_name: &str) -> Vec<(String, String)> {
    let item = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == struct_name => Some(item),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing generated struct {struct_name}"));

    item.fields
        .iter()
        .map(|field| {
            let name = field
                .ident
                .as_ref()
                .expect("generated field should be named")
                .to_string();
            let ty = field.ty.to_token_stream().to_string().replace(' ', "");
            (name, ty)
        })
        .collect()
}

fn impls_trait(file: &syn::File, self_ty: &str, trait_path_suffix: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Impl(item) => {
            item.self_ty.to_token_stream().to_string().replace(' ', "") == self_ty
                && item
                    .trait_
                    .as_ref()
                    .map(|(_, path, _)| {
                        path.to_token_stream()
                            .to_string()
                            .replace(' ', "")
                            .ends_with(trait_path_suffix)
                    })
                    .unwrap_or(false)
        }
        _ => false,
    })
}

#[test]
fn generated_basic_types_exposes_public_fields_and_traits() {
    let msgs = resolve_corpus();
    let file = generated_file_for_message(&msgs, "BasicTypes");

    assert_eq!(
        public_fields(&file, "BasicTypes"),
        vec![
            ("bool_value".to_string(), "bool".to_string()),
            ("byte_value".to_string(), "u8".to_string()),
            ("char_value".to_string(), "u8".to_string()),
            ("float32_value".to_string(), "f32".to_string()),
            ("float64_value".to_string(), "f64".to_string()),
            ("int8_value".to_string(), "i8".to_string()),
            ("uint8_value".to_string(), "u8".to_string()),
            ("int16_value".to_string(), "i16".to_string()),
            ("uint16_value".to_string(), "u16".to_string()),
            ("int32_value".to_string(), "i32".to_string()),
            ("uint32_value".to_string(), "u32".to_string()),
            ("int64_value".to_string(), "i64".to_string()),
            ("uint64_value".to_string(), "u64".to_string()),
        ]
    );
    assert!(impls_trait(&file, "BasicTypes", "::ros_z::Message"));
    assert!(impls_trait(&file, "BasicTypes", "::ros_z_cdr::CdrEncode"));
    assert!(impls_trait(&file, "BasicTypes", "::ros_z_cdr::CdrDecode"));
}

#[test]
fn generated_string_and_array_messages_expose_public_container_types() {
    let msgs = resolve_corpus();
    let strings = generated_file_for_message(&msgs, "Strings");
    let arrays = generated_file_for_message(&msgs, "Arrays");
    let nested = generated_file_for_message(&msgs, "Nested");

    assert_eq!(
        public_fields(&strings, "Strings"),
        vec![
            (
                "string_value".to_string(),
                "::std::string::String".to_string(),
            ),
            (
                "bounded_string_value".to_string(),
                "::std::string::String".to_string(),
            ),
            (
                "unbounded_string_array".to_string(),
                "::std::vec::Vec<::std::string::String>".to_string(),
            ),
            (
                "string_array_three".to_string(),
                "[::std::string::String;3]".to_string(),
            ),
            (
                "bounded_string_sequence".to_string(),
                "::std::vec::Vec<::std::string::String>".to_string(),
            ),
            (
                "unbounded_bounded_string_array".to_string(),
                "::std::vec::Vec<::std::string::String>".to_string(),
            ),
            (
                "bounded_string_array_three".to_string(),
                "[::std::string::String;3]".to_string(),
            ),
            (
                "bounded_string_bounded_sequence".to_string(),
                "::std::vec::Vec<::std::string::String>".to_string(),
            ),
        ]
    );

    let array_fields = public_fields(&arrays, "Arrays");
    assert!(array_fields.contains(&("bool_values".to_string(), "[bool;3]".to_string())));
    assert!(array_fields.contains(&("byte_values".to_string(), "[u8;3]".to_string())));
    assert!(array_fields.contains(&(
        "string_values".to_string(),
        "[::std::string::String;3]".to_string()
    )));
    assert!(array_fields.contains(&(
        "basic_types_values".to_string(),
        "[super::test_interface_files::BasicTypes;3]".to_string()
    )));
    assert!(array_fields.contains(&("alignment_check".to_string(), "i32".to_string())));

    assert_eq!(
        public_fields(&nested, "Nested"),
        vec![(
            "basic_types_value".to_string(),
            "super::test_interface_files::BasicTypes".to_string()
        )]
    );
}

// ── Service snapshot ──────────────────────────────────────────────────────────

#[test]
fn snapshot_service_basic_types() {
    let parsed = ros_z_codegen::discovery::discover_services(&corpus_dir(), "test_interface_files")
        .expect("discover services");

    let mut resolver = Resolver::new();
    let resolved = resolver.resolve_services(parsed).expect("resolve services");

    let srv = resolved
        .iter()
        .find(|s| s.parsed.name == "BasicTypes")
        .expect("BasicTypes service not found");

    let tokens = generate_service_impl(srv).expect("generate service");
    insta::assert_snapshot!("service_basic_types", format_tokens(tokens));
}

// ── Action snapshot ───────────────────────────────────────────────────────────

#[test]
fn snapshot_action_fibonacci() {
    let parsed = ros_z_codegen::discovery::discover_actions(&corpus_dir(), "test_interface_files")
        .expect("discover actions");

    let mut resolver = Resolver::new();
    let resolved = resolver.resolve_actions(parsed).expect("resolve actions");

    let action = resolved
        .iter()
        .find(|a| a.parsed.name == "Fibonacci")
        .expect("Fibonacci action not found");

    let tokens = generate_action_impl(action).expect("generate action");
    insta::assert_snapshot!("action_fibonacci", format_tokens(tokens));
}
