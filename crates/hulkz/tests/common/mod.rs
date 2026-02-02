//! Shared test utilities for hulkz integration tests.

/// Helper to create a unique namespace for test isolation.
pub fn test_namespace(name: &str) -> String {
    format!("test_{}_{}", name, std::process::id())
}
