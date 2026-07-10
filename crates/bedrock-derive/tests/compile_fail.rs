//! Compile-fail coverage for the derives' strictness guarantees. Each case in
//! `tests/compile_fail/` must fail to compile with a bedrock-authored message.
//!
//! Regenerate the expected output after intentional message changes with:
//! `TRYBUILD=overwrite cargo test -p bedrock-derive --test compile_fail`.

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
