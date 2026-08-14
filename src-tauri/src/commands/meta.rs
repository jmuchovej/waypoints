use cartograph::BuildInfo;

/// The compiled build's identity — version, channel, commit, build time, and
/// ref. Every value is baked in at compile time by `build.rs`.
#[tauri::command]
pub fn build_info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        channel: env!("WAYPOINTS_CHANNEL").to_string(),
        sha: env!("WAYPOINTS_GIT_SHA").to_string(),
        built_at: env!("WAYPOINTS_BUILT_AT").to_string(),
        branch: env!("WAYPOINTS_GIT_REF").to_string(),
    }
}
