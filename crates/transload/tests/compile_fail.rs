// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compile-fail coverage for the exporter's tier dispatch.
//!
//! Regenerate the expected output after an intentional message change with:
//! `TRYBUILD=overwrite cargo test -p transload --test compile_fail`.

#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
