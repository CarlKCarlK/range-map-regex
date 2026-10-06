//! Compile-fail tests for the `declare` macros' diagnostics.

#[test]
fn declare_ui() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
