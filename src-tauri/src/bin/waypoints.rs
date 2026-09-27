#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
        let build = waypoints::build_info();
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
        return;
    }

    waypoints::run();
}
