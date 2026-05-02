#[test]
fn message_derive_rejects_only_still_unsupported_shapes() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/message_derive/enum.rs");
    cases.pass("tests/ui/message_derive/option_field.rs");
    cases.pass("tests/ui/message_derive/unit_struct.rs");
    cases.compile_fail("tests/ui/message_derive/const_generic.rs");
    cases.compile_fail("tests/ui/message_derive/empty_type_name.rs");
    cases.compile_fail("tests/ui/message_derive/generic_enum.rs");
    cases.compile_fail("tests/ui/message_derive/generic_tuple_struct.rs");
    cases.compile_fail("tests/ui/message_derive/lifetime_generic.rs");
    cases.compile_fail("tests/ui/message_derive/missing_serde.rs");
    cases.compile_fail("tests/ui/message_derive/unsupported_map_key.rs");
    cases.compile_fail("tests/ui/message_derive/tuple_struct.rs");
}
