use cartograph::BuildInfo;

/// The compiled build's identity — version, channel, commit, build time, and
/// ref. Every value is baked in at compile time by `build.rs`.
#[tauri::command]
pub fn build_info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        channel: env!("WAYPOINTS_CHANNEL").to_string(),
        sha: env!("WAYPOINTS_GIT_SHA").to_string(),
        built_at: built_at(),
        branch: env!("WAYPOINTS_GIT_BRANCH").to_string(),
    }
}

/// Format the baked build epoch as RFC 3339 UTC; empty for dev builds (epoch 0).
fn built_at() -> String {
    match env!("WAYPOINTS_BUILT_AT_EPOCH").parse::<i64>() {
        Ok(0) | Err(_) => String::new(),
        Ok(epoch) => chrono::DateTime::from_timestamp(epoch, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default(),
    }
}
