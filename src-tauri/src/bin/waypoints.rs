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
