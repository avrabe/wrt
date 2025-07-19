//! WRT Core Memory Safety Tests - MOVED
//!
//! The memory safety tests for the wrt crate have been consolidated into
//! the main test suite at: wrt-tests/integration/memory/
//!
//! For the complete memory safety test suite, use:
//! ```
//! cargo test -p wrt-tests memory
//! ```
//!
//! Previously, wrt memory tests were in:
//! - wrt/tests/memory_fix_test.rs (MOVED)
//! - wrt/tests/memory_safe_test.rs (MOVED)
//! - wrt/tests/memory_safety_test.rs (MOVED)
//!
//! All functionality is now available in the consolidated test suite.

#[test]
fn wrt_memory_tests_moved_notice() {
    println!("WRT memory safety tests have been moved to wrt-tests/integration/memory/";
    println!("Run: cargo test -p wrt-tests memory";
}
