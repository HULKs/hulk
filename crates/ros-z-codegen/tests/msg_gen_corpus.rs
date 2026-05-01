//! Parser + resolver + hash golden tests for the test_interface_files corpus.
//!
//! These tests exercise the full codegen pipeline (parser → resolver → hash)
//! against the vendored test_interface_files message definitions without
//! requiring a ROS 2 installation or running Zenoh router.

use std::{collections::HashMap, path::PathBuf};

use ros_z_codegen::{
    discovery::{discover_actions, discover_messages, discover_services},
    resolver::Resolver,
    types::{
        ArrayType, ParsedAction, ParsedMessage, ParsedService, ResolvedAction, ResolvedMessage,
        ResolvedService,
    },
};

/// Path to the test_interface_files assets directory.
fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/jazzy/test_interface_files")
}

/// Parse all messages from the corpus.
fn parse_corpus_messages() -> Vec<ParsedMessage> {
    discover_messages(&corpus_dir(), "test_interface_files")
        .expect("Failed to discover test_interface_files messages")
}

/// Parse all services from the corpus.
fn parse_corpus_services() -> Vec<ParsedService> {
    discover_services(&corpus_dir(), "test_interface_files")
        .expect("Failed to discover test_interface_files services")
}

/// Parse all actions from the corpus.
fn parse_corpus_actions() -> Vec<ParsedAction> {
    discover_actions(&corpus_dir(), "test_interface_files")
        .expect("Failed to discover test_interface_files actions")
}

/// Get a parsed message by name.
fn get_msg<'a>(msgs: &'a [ParsedMessage], name: &str) -> &'a ParsedMessage {
    msgs.iter()
        .find(|m| m.name == name)
        .unwrap_or_else(|| panic!("Message '{name}' not found in corpus"))
}

/// Get a parsed service by name.
fn get_srv<'a>(srvs: &'a [ParsedService], name: &str) -> &'a ParsedService {
    srvs.iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("Service '{name}' not found in corpus"))
}

/// Get a parsed action by name.
fn get_action<'a>(actions: &'a [ParsedAction], name: &str) -> &'a ParsedAction {
    actions
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("Action '{name}' not found in corpus"))
}

// ============================================================================
// Parser tests
// ============================================================================

#[test]
fn test_parse_basic_types() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "BasicTypes");

    assert_eq!(msg.package, "test_interface_files");
    // 13 primitive fields: bool, byte, char, float32, float64,
    // int8, uint8, int16, uint16, int32, uint32, int64, uint64
    assert_eq!(msg.fields.len(), 13, "BasicTypes should have 13 fields");
    assert!(
        msg.constants.is_empty(),
        "BasicTypes should have no constants"
    );

    let field_names: Vec<&str> = msg.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"bool_value"));
    assert!(field_names.contains(&"float64_value"));
    assert!(field_names.contains(&"int64_value"));

    // All fields should be scalar (Single array type)
    for field in &msg.fields {
        assert_eq!(
            field.field_type.array,
            ArrayType::Single,
            "BasicTypes field '{}' should be scalar",
            field.name
        );
    }
}

#[test]
fn test_parse_arrays() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "Arrays");

    // 32 fields: 17 plain (13 primitives + string + BasicTypes + Constants + Defaults),
    // 14 with array defaults (bool..string), plus alignment_check (scalar).
    assert_eq!(msg.fields.len(), 32, "Arrays should have 32 fields");

    // First 17 plain array fields are all Fixed(3)
    for field in &msg.fields[..17] {
        assert_eq!(
            field.field_type.array,
            ArrayType::Fixed(3),
            "Arrays plain field '{}' should be Fixed(3)",
            field.name
        );
    }
    // alignment_check is the last field and is scalar
    let last = msg.fields.last().unwrap();
    assert_eq!(last.name, "alignment_check");
    assert_eq!(last.field_type.array, ArrayType::Single);
}

#[test]
fn test_parse_bounded_sequences() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "BoundedSequences");

    // 32 fields: 17 plain + 14 with defaults + alignment_check
    assert_eq!(
        msg.fields.len(),
        32,
        "BoundedSequences should have 32 fields"
    );

    // First 17 are Bounded(3)
    for field in &msg.fields[..17] {
        assert_eq!(
            field.field_type.array,
            ArrayType::Bounded(3),
            "BoundedSequences plain field '{}' should be Bounded(3)",
            field.name
        );
    }
    let last = msg.fields.last().unwrap();
    assert_eq!(last.name, "alignment_check");
    assert_eq!(last.field_type.array, ArrayType::Single);
}

#[test]
fn test_parse_unbounded_sequences() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "UnboundedSequences");

    // 32 fields: 17 plain + 14 with defaults + alignment_check
    assert_eq!(
        msg.fields.len(),
        32,
        "UnboundedSequences should have 32 fields"
    );

    // First 17 are Unbounded
    for field in &msg.fields[..17] {
        assert_eq!(
            field.field_type.array,
            ArrayType::Unbounded,
            "UnboundedSequences plain field '{}' should be Unbounded",
            field.name
        );
    }
    let last = msg.fields.last().unwrap();
    assert_eq!(last.name, "alignment_check");
    assert_eq!(last.field_type.array, ArrayType::Single);
}

#[test]
fn test_parse_constants() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "Constants");

    // Constants.msg has 13 constants (bool..uint64), no fields, no string constant
    assert!(msg.fields.is_empty(), "Constants.msg should have no fields");
    assert_eq!(
        msg.constants.len(),
        13,
        "Constants.msg should have 13 constants"
    );

    let const_names: Vec<&str> = msg.constants.iter().map(|c| c.name.as_str()).collect();
    assert!(const_names.contains(&"BOOL_CONST"));
    assert!(const_names.contains(&"INT64_CONST"));
    assert!(const_names.contains(&"UINT64_CONST"));
}

#[test]
fn test_parse_defaults() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "Defaults");

    // Defaults.msg has 13 fields (bool..uint64) with default values, no string field
    assert_eq!(msg.fields.len(), 13, "Defaults.msg should have 13 fields");

    for field in &msg.fields {
        assert!(
            field.default.is_some(),
            "Defaults.msg field '{}' should have a default value",
            field.name
        );
    }
}

#[test]
fn test_parse_empty() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "Empty");

    assert!(msg.fields.is_empty(), "Empty.msg should have no fields");
    assert!(
        msg.constants.is_empty(),
        "Empty.msg should have no constants"
    );
}

#[test]
fn test_parse_nested() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "Nested");

    assert_eq!(msg.fields.len(), 1, "Nested.msg should have 1 field");
    assert_eq!(msg.fields[0].name, "basic_types_value");
    assert_eq!(msg.fields[0].field_type.base_type, "BasicTypes");
    assert_eq!(msg.fields[0].field_type.array, ArrayType::Single);
}

#[test]
fn test_parse_multi_nested() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "MultiNested");

    // 9 fields: Arrays/BoundedSequences/UnboundedSequences × 3 array kinds (single, fixed[3], unbounded)
    assert_eq!(msg.fields.len(), 9, "MultiNested.msg should have 9 fields");

    // First 3 are scalar references to the three complex types
    assert_eq!(msg.fields[0].name, "array_of_arrays");
    assert_eq!(msg.fields[0].field_type.base_type, "Arrays");
    assert_eq!(msg.fields[0].field_type.array, ArrayType::Single);
}

#[test]
fn test_parse_strings() {
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "Strings");

    // 8 fields: string_value, bounded_string_value, and 6 array variants
    assert_eq!(msg.fields.len(), 8, "Strings.msg should have 8 fields");

    // bounded_string_value should have string_bound = Some(10)
    let bounded = msg
        .fields
        .iter()
        .find(|f| f.name == "bounded_string_value")
        .expect("Should have bounded_string_value field");
    assert_eq!(
        bounded.field_type.string_bound,
        Some(10),
        "bounded_string_value should have string_bound=10"
    );

    // unbounded_string_array should be Unbounded
    let arr = msg
        .fields
        .iter()
        .find(|f| f.name == "unbounded_string_array")
        .unwrap();
    assert_eq!(arr.field_type.array, ArrayType::Unbounded);

    // string_array_three should be Fixed(3)
    let arr3 = msg
        .fields
        .iter()
        .find(|f| f.name == "string_array_three")
        .unwrap();
    assert_eq!(arr3.field_type.array, ArrayType::Fixed(3));
}

#[test]
fn test_parse_wstrings_filtered() {
    // WStrings.msg should parse without panicking; has wstring fields.
    // The filter_messages function in InterfaceGenerator would exclude it,
    // but the parser itself should succeed.
    let msgs = parse_corpus_messages();
    let msg = get_msg(&msgs, "WStrings");

    // All fields have wstring base type
    assert!(!msg.fields.is_empty(), "WStrings.msg should have fields");
    for field in &msg.fields {
        assert!(
            field.field_type.base_type.contains("wstring"),
            "WStrings.msg field '{}' should have wstring type",
            field.name
        );
    }
}

#[test]
fn test_parse_service_basic_types() {
    let srvs = parse_corpus_services();
    let srv = get_srv(&srvs, "BasicTypes");

    assert_eq!(srv.package, "test_interface_files");
    // BasicTypes.srv has 14 fields (13 primitives + string) in both request and response
    assert_eq!(
        srv.request.fields.len(),
        14,
        "Request should have 14 fields"
    );
    assert_eq!(
        srv.response.fields.len(),
        14,
        "Response should have 14 fields"
    );
    assert_eq!(srv.request.name, "BasicTypesRequest");
    assert_eq!(srv.response.name, "BasicTypesResponse");
}

#[test]
fn test_parse_fibonacci_action() {
    let actions = parse_corpus_actions();
    let action = get_action(&actions, "Fibonacci");

    assert_eq!(action.package, "test_interface_files");
    assert_eq!(action.name, "Fibonacci");

    // Goal: int32 order
    assert_eq!(action.goal.fields.len(), 1);
    assert_eq!(action.goal.fields[0].name, "order");
    assert_eq!(action.goal.fields[0].field_type.base_type, "int32");

    // Result: int32[] sequence
    let result = &action.result;
    assert_eq!(result.fields.len(), 1);
    assert_eq!(result.fields[0].name, "sequence");
    assert_eq!(result.fields[0].field_type.array, ArrayType::Unbounded);

    // Feedback: int32[] sequence
    let feedback = &action.feedback;
    assert_eq!(feedback.fields.len(), 1);
    assert_eq!(feedback.fields[0].name, "sequence");
}

#[test]
fn test_parse_service_empty() {
    let srvs = parse_corpus_services();
    let srv = get_srv(&srvs, "Empty");
    assert_eq!(srv.package, "test_interface_files");
    assert!(
        srv.request.fields.is_empty(),
        "Empty.srv request should have no fields"
    );
    assert!(
        srv.response.fields.is_empty(),
        "Empty.srv response should have no fields"
    );
    assert_eq!(srv.request.name, "EmptyRequest");
    assert_eq!(srv.response.name, "EmptyResponse");
}

#[test]
fn test_parse_service_arrays() {
    let srvs = parse_corpus_services();
    let srv = get_srv(&srvs, "Arrays");
    assert_eq!(srv.package, "test_interface_files");
    // Arrays.srv: request and response each have 14 fixed-size array fields
    assert_eq!(
        srv.request.fields.len(),
        14,
        "Arrays.srv request should have 14 fields"
    );
    assert_eq!(
        srv.response.fields.len(),
        14,
        "Arrays.srv response should have 14 fields"
    );
    // Spot-check: first field is bool[3]
    assert_eq!(srv.request.fields[0].name, "bool_values");
    assert_eq!(srv.request.fields[0].field_type.array, ArrayType::Fixed(3));
}

// ============================================================================
// Hash tests
// ============================================================================

/// Resolve corpus messages for hashing. Excludes WStrings because TypeId has no
/// array variants for wstring (WSTRING_ARRAY etc. are not defined in the schema).
fn resolve_corpus() -> HashMap<String, ResolvedMessage> {
    let msgs = parse_corpus_messages()
        .into_iter()
        .filter(|m| m.name != "WStrings")
        .collect();
    let mut resolver = Resolver::new();
    let resolved = resolver
        .resolve_messages(msgs)
        .expect("Failed to resolve test_interface_files messages");
    resolved
        .into_iter()
        .map(|r| (r.parsed.name.clone(), r))
        .collect()
}

/// Resolve all corpus services without loading external service event packages.
fn resolve_corpus_services() -> (Vec<ResolvedService>, Vec<ResolvedMessage>) {
    let msgs = parse_corpus_messages()
        .into_iter()
        .filter(|m| m.name != "WStrings")
        .collect();
    let mut resolver = Resolver::new();
    let resolved_msgs = resolver
        .resolve_messages(msgs)
        .expect("Failed to resolve messages for service resolver");
    let srvs = parse_corpus_services();
    let resolved_srvs = resolver
        .resolve_services(srvs)
        .expect("Failed to resolve test_interface_files services");
    (resolved_srvs, resolved_msgs)
}

/// Resolve all corpus actions without loading external action packages.
fn resolve_corpus_actions() -> Vec<ResolvedAction> {
    let msgs = parse_corpus_messages()
        .into_iter()
        .filter(|m| m.name != "WStrings")
        .collect();
    let mut resolver = Resolver::new();
    resolver
        .resolve_messages(msgs)
        .expect("Failed to resolve messages for action resolver");
    let actions = parse_corpus_actions();
    resolver
        .resolve_actions(actions)
        .expect("Failed to resolve test_interface_files actions")
}

#[test]
fn test_hash_format_basic_types() {
    let resolved = resolve_corpus();
    let hash = resolved["BasicTypes"].schema_hash.to_hash_string();
    assert!(
        hash.starts_with("RZHS01_"),
        "Hash must start with RZHS01_: {hash}"
    );
    assert_eq!(
        hash.len(),
        7 + 64,
        "Hash must be RZHS01_ + 64 hex chars: {hash}"
    );
}

#[test]
fn test_hash_format_empty() {
    let resolved = resolve_corpus();
    let hash = resolved["Empty"].schema_hash.to_hash_string();
    assert!(
        hash.starts_with("RZHS01_"),
        "Hash must start with RZHS01_: {hash}"
    );
    assert_eq!(
        hash.len(),
        7 + 64,
        "Hash must be RZHS01_ + 64 hex chars: {hash}"
    );
}

#[test]
fn test_hash_deterministic() {
    let resolved1 = resolve_corpus();
    let resolved2 = resolve_corpus();
    let h1 = resolved1["BasicTypes"].schema_hash.to_hash_string();
    let h2 = resolved2["BasicTypes"].schema_hash.to_hash_string();
    assert_eq!(h1, h2, "Hash must be deterministic");
}

#[test]
fn test_hashes_differ_between_types() {
    let resolved = resolve_corpus();
    let h_basic = resolved["BasicTypes"].schema_hash.to_hash_string();
    let h_empty = resolved["Empty"].schema_hash.to_hash_string();
    let h_nested = resolved["Nested"].schema_hash.to_hash_string();
    let h_arrays = resolved["Arrays"].schema_hash.to_hash_string();

    let all = [&h_basic, &h_empty, &h_nested, &h_arrays];
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            if i != j {
                assert_ne!(
                    a, b,
                    "Hashes must differ for different types (indices {i},{j})"
                );
            }
        }
    }
}

#[test]
fn test_hash_nested_depends_on_basic_types() {
    // Nested.msg contains BasicTypes; its hash must differ from BasicTypes's hash
    let resolved = resolve_corpus();
    let h_basic = resolved["BasicTypes"].schema_hash.to_hash_string();
    let h_nested = resolved["Nested"].schema_hash.to_hash_string();
    assert_ne!(
        h_basic, h_nested,
        "Nested hash must differ from BasicTypes hash"
    );
}

// ============================================================================
// Golden hash regression tests.
// ============================================================================

#[test]
fn test_golden_hashes() {
    let resolved = resolve_corpus();
    let cases = [
        (
            "Arrays",
            "RZHS01_ec11bf6c387a9a6edd86a9c7df5efd10248dff9145e74591a79e3dc4e513bbaf",
        ),
        (
            "BasicTypes",
            "RZHS01_7a9f4bcd31ee576d5113df2eb086b45ce429038a3c434ccf7286cea75b748ba1",
        ),
        (
            "BoundedPlainSequences",
            "RZHS01_e1efa8c0a79a7c8ba5ca765dfdec8f5fb4145e3e2184b71c68cbe1db82cc3471",
        ),
        (
            "BoundedSequences",
            "RZHS01_230318e2899b0688c445c5e49ee1f31f8c3a893f048efe0f325dda4b3629a5f5",
        ),
        (
            "Constants",
            "RZHS01_aea92366e0896e47bfefce75291f66e2529cf0feca33822d4a7e8a90b810e008",
        ),
        (
            "Defaults",
            "RZHS01_047ec8c4be8e8b649018cffee4295449e62b3f49d709ffcf4051cfab16b3ed79",
        ),
        (
            "Empty",
            "RZHS01_24fa7b85edd72eecbe2d488dca72d3ab1e6f9cdf25b8b0585006d2f604362112",
        ),
        (
            "MultiNested",
            "RZHS01_1874b413e9d8bca8cb6adb962d0ea9c672fe3aaa222926cf010cfb0a0e274f2c",
        ),
        (
            "Nested",
            "RZHS01_3ed08553667ea92e4fec375e255627c6bf43137dd20aa24e331b96d726ee74bc",
        ),
        (
            "Strings",
            "RZHS01_cd845d6e9691e82d2ef563b03a94f2adccf1705ad44fdffdc8e4af95464a765f",
        ),
        (
            "UnboundedSequences",
            "RZHS01_0d622ae8cf0cadd782a99e58a4da1d2f20699232e6f830ea86c97ee43b903f66",
        ),
    ];
    for (name, expected) in cases {
        let hash = resolved[name].schema_hash.to_hash_string();
        assert_eq!(hash, expected, "{name} hash regression");
    }
}

// ============================================================================
// Service and action hash tests
// ============================================================================

#[test]
fn test_service_hash_format() {
    let (srvs, _) = resolve_corpus_services();
    for srv in &srvs {
        let hash = srv.schema_hash.to_hash_string();
        assert!(
            hash.starts_with("RZHS01_"),
            "Service {} hash must start with RZHS01_: {hash}",
            srv.parsed.name
        );
        assert_eq!(
            hash.len(),
            7 + 64,
            "Service {} hash must be RZHS01_ + 64 hex chars: {hash}",
            srv.parsed.name
        );
    }
}

#[test]
fn test_service_hashes_differ_from_messages() {
    let resolved_msgs = resolve_corpus();
    let (srvs, _) = resolve_corpus_services();

    // BasicTypes.srv hash must differ from BasicTypes.msg hash
    let msg_hash = resolved_msgs["BasicTypes"].schema_hash.to_hash_string();
    let srv = srvs.iter().find(|s| s.parsed.name == "BasicTypes").unwrap();
    let srv_hash = srv.schema_hash.to_hash_string();
    assert_ne!(
        msg_hash, srv_hash,
        "BasicTypes.srv hash must differ from BasicTypes.msg hash"
    );
}

#[test]
fn test_action_hash_format() {
    let actions = resolve_corpus_actions();
    for action in &actions {
        let hash = action.schema_hash.to_hash_string();
        assert!(
            hash.starts_with("RZHS01_"),
            "Action {} hash must start with RZHS01_: {hash}",
            action.parsed.name
        );
        assert_eq!(
            hash.len(),
            7 + 64,
            "Action {} hash must be RZHS01_ + 64 hex chars: {hash}",
            action.parsed.name
        );
    }
}

#[test]
fn test_action_goal_result_feedback_hashes_differ() {
    let actions = resolve_corpus_actions();
    let fib = actions
        .iter()
        .find(|a| a.parsed.name == "Fibonacci")
        .unwrap();

    let goal_hash = fib.goal.schema_hash.to_hash_string();
    let result = &fib.result;
    let feedback = &fib.feedback;
    let result_hash = result.schema_hash.to_hash_string();
    let feedback_hash = feedback.schema_hash.to_hash_string();

    assert_ne!(
        goal_hash, result_hash,
        "Fibonacci goal/result hashes must differ"
    );
    assert_ne!(
        goal_hash, feedback_hash,
        "Fibonacci goal/feedback hashes must differ"
    );
}

// ============================================================================
// Parser error path tests
// ============================================================================

mod parser_errors {
    use ros_z_codegen::parser::{
        parse_constant, parse_default_value, parse_field, parse_field_type,
    };

    #[test]
    fn test_parse_field_too_few_tokens() {
        // A field line needs at least "type name"; one token is invalid.
        let err = parse_field("uint8", "pkg", 1).unwrap_err();
        assert!(
            err.to_string().contains("Invalid field format"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_parse_field_type_unclosed_bracket() {
        let err = parse_field_type("uint8[", "pkg").unwrap_err();
        assert!(
            err.to_string().contains("Invalid array syntax"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_parse_field_type_invalid_fixed_size() {
        let err = parse_field_type("uint8[abc]", "pkg").unwrap_err();
        assert!(
            err.to_string().contains("Invalid fixed array size"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_parse_field_type_invalid_bounded_array_size() {
        let err = parse_field_type("uint8[<=xyz]", "pkg").unwrap_err();
        assert!(
            err.to_string().contains("Invalid bounded array size"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_parse_field_type_invalid_string_bound() {
        let err = parse_field_type("string<=xyz", "pkg").unwrap_err();
        assert!(
            err.to_string().contains("Invalid string bound"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_parse_default_value_empty_array() {
        // An empty array literal "[]" is not a valid default.
        let err = parse_default_value("[]").unwrap_err();
        assert!(err.to_string().contains("Empty array"), "unexpected: {err}");
    }

    #[test]
    fn test_parse_constant_missing_value() {
        // A constant line must have "TYPE NAME = VALUE"; missing "= value" is invalid.
        let err = parse_constant("uint8 FOO", 1).unwrap_err();
        assert!(err.to_string().contains("constant"), "unexpected: {err}");
    }
}
