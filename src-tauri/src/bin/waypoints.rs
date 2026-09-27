#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Answered before any runtime starts, so a packaged build can be identified
    // without a display. Matched exactly, so a forwarded `waypoints://` URL or
    // any other launch argument still falls through to the app.
    if std::env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V")
    {
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
