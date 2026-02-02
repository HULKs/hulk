//! Shared test utilities for hulkz integration tests.

use hulkz::Session;

/// Path to test parameters file relative to crate root.
#[allow(dead_code)]
const TEST_PARAMETERS: &str = "tests/fixtures/parameters.json5";

/// Helper to create a unique namespace for test isolation.
pub fn test_namespace(name: &str) -> String {
    format!("test_{}_{}", name, std::process::id())
}

/// Creates a test session with the test parameters file loaded.
///
/// Use this for tests that need parameter configuration values.
/// Tests that don't use parameters can use `Session::create()` directly.
#[allow(dead_code)]
pub async fn test_session(name: &str) -> hulkz::Result<Session> {
    Session::builder(test_namespace(name))
        .parameters_file(TEST_PARAMETERS)
        .build()
        .await
}
