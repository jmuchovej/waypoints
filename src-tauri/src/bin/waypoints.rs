// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of Waypoints.
//
// Waypoints is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Waypoints is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with Waypoints. If not, see <https://www.gnu.org/licenses/>.
//
// Additional permission under GNU AGPL version 3 section 7: this Program
// may be conveyed through application stores on the terms set out in
// docs/licensing/COPYING.exceptions.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Writes the build identity to stdout for `--version`.
///
/// The one place in the app where stdout is the contract rather than a mistake:
/// a `--version` caller reads it, and the `disallowed_macros` reasoning — that
/// stdout is unstructured and swallowed under the Tauri runtime — does not apply
/// on a path that returns before the runtime starts. A function rather than an
/// attribute on the statement, because the lint reports at the macro's
/// expansion span and so only an item-level level reaches it.
#[expect(
    clippy::disallowed_macros,
    reason = "stdout is this flag's output; no runtime exists yet to swallow it"
)]
fn print_build_identity(build: &cartograph::BuildInfo) {
    // `built_at` is empty unless the build opted into a wall-clock stamp, so
    // omit the clause rather than inventing one.
    let stamped = if build.built_at.is_empty() {
        String::new()
    } else {
        format!(", built {}", build.built_at)
    };
    println!(
        "waypoints {} ({}, {}{stamped})",
        build.version, build.channel, build.sha,
    );
}

fn main() {
    // Answered before any runtime starts, so a packaged build can be identified
    // without a display.
    //
    // Matched as the *sole* argument rather than anywhere in argv: a Tauri app
    // receives arguments it must not interpret — deep links arrive this way on
    // Linux and Windows and are forwarded by the single-instance plugin, and
    // macOS passes process serial numbers on some launch paths. Anything else,
    // including `waypoints://… --version`, falls through to the app.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.as_slice(), [flag] if flag == "--version" || flag == "-V") {
        print_build_identity(&waypoints::build_info());
        return;
    }

    waypoints::run();
}
