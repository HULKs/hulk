#[test]
fn message_derive_supports_message_shapes() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/message_derive/const_generic.rs");
    cases.pass("tests/ui/message_derive/enum.rs");
    cases.pass("tests/ui/message_derive/generic_tuple_struct.rs");
    cases.pass("tests/ui/message_derive/option_field.rs");
    cases.pass("tests/ui/message_derive/tuple_struct.rs");
    cases.pass("tests/ui/message_derive/unit_struct.rs");
}
