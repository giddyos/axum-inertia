//! Compile-time macro contract tests.

#[test]
fn embed_frontend_ui() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/ui/pass/*.rs");
    tests.compile_fail("tests/ui/fail/*.rs");
}
