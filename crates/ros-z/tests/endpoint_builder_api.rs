#[test]
fn endpoint_builders_expose_single_question_mark_api() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/endpoint_builder/new_api_pass.rs");
    cases.compile_fail("tests/ui/endpoint_builder/dynamic_subscriber_cache.rs");
}
