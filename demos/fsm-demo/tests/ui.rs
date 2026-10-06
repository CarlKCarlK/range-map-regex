//! Compile-fail tests: the diagnostics for malformed `fsm!` declarations.

#[test]
fn ui() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
