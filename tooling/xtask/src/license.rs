// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The project's licensing policy.

/// The license of every first-party crate and source file.
pub const POLICY: &str = "AGPL-3.0-or-later";

/// The copyright holder a first-party source file names.
pub const HOLDER: &str = "John Muchovej";

/// The tag a copyright line opens with, comment syntax aside.
pub const COPYRIGHT_TAG: &str = "SPDX-FileCopyrightText:";

/// The year of the first copyright line in `text`, which a file keeps for life.
pub fn copyright_year(text: &str) -> Option<&str> {
    text.lines()
        .find_map(|line| line.split_once(COPYRIGHT_TAG))
        .and_then(|(_, rest)| rest.split_whitespace().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_year_comes_from_the_first_copyright_line() {
        let text =
            "// marker\n// SPDX-FileCopyrightText: 2025 One\n// SPDX-FileCopyrightText: 2026 Two\n";
        assert_eq!(copyright_year(text), Some("2025"));
    }

    #[test]
    fn a_file_without_a_copyright_line_has_no_year() {
        assert_eq!(copyright_year("// marker\nexport type A = string;\n"), None);
    }
}
