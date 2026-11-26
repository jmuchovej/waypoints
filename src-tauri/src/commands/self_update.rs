// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::launch::Instance;
use crate::settings::{self, SettingsResult};
use cartograph::{SelfUpdatePolicy, SelfUpdateStatus};
use tauri::{AppHandle, State};

/// The setting, the launch cap, and what they resolve to.
pub(crate) fn status(app: &AppHandle, instance: &Instance) -> SettingsResult<SelfUpdateStatus> {
    Ok(SelfUpdateStatus::new(
        settings::self_update(app)?,
        instance.cap(),
    ))
}

#[tauri::command]
pub fn self_update_status(
    app: AppHandle,
    instance: State<'_, Instance>,
) -> SettingsResult<SelfUpdateStatus> {
    status(&app, &instance)
}

#[tauri::command]
pub fn self_update_set(
    app: AppHandle,
    instance: State<'_, Instance>,
    setting: SelfUpdatePolicy,
) -> SettingsResult<SelfUpdateStatus> {
    settings::set_self_update(&app, setting)?;
    status(&app, &instance)
}
