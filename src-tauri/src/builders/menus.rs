// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

#[cfg(debug_assertions)]
use cartograph::WaypointsRuntime;
#[cfg(debug_assertions)]
use std::sync::Arc;
use tauri::{
    App, AppHandle, Emitter, Manager, Runtime, async_runtime,
    menu::{
        AboutMetadata, AboutMetadataBuilder, MenuBuilder, MenuEvent, MenuItemBuilder, Submenu,
        SubmenuBuilder,
    },
};
#[cfg(debug_assertions)]
use tauri::{image::Image, menu::IconMenuItemBuilder};
use tauri_plugin_updater::UpdaterExt;

type SetupResult<T> = Result<T, Box<dyn std::error::Error>>;
type SubMenuResult<R> = SetupResult<Submenu<R>>;

pub fn setup(app: &mut App) -> SetupResult<()> {
    let app_handle = app.app_handle();
    let menu = MenuBuilder::new(app);

    #[cfg(target_os = "macos")]
    let menu = menu.item(&setup_macos(app_handle)?);

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let menu = menu.item(&setup_file(app_handle)?);

    let menu = menu.item(&setup_edit(app_handle)?);
    let menu = menu.item(&setup_view(app_handle)?);
    let menu = menu.item(&setup_window(app_handle)?);

    #[cfg(debug_assertions)]
    let menu = menu.item(&setup_developer(app_handle)?);

    let menu = menu.item(&setup_help(app_handle)?);

    app.set_menu(menu.build()?)?;

    // Updater menu handler
    let handle = app_handle.clone();
    app_handle.on_menu_event(move |_app, event: MenuEvent| {
        if event.id().0.as_str() == "check-for-updates" {
            let handle = handle.clone();
            async_runtime::spawn(async move {
                check_for_updates(handle).await;
            });
        }
    });

    Ok(())
}

fn setup_macos<R: tauri::Runtime>(app_handle: &AppHandle<R>) -> SubMenuResult<R> {
    let pkg_name = app_handle.package_info().name.clone();
    let metadata = setup_about(app_handle)?;
    Ok(SubmenuBuilder::new(app_handle, pkg_name)
        .about(Some(metadata))
        .separator()
        .item(
            &MenuItemBuilder::with_id("check-for-updates", "Check for Updates...")
                .build(app_handle)?,
        )
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .separator()
        .close_window()
        .quit()
        .build()?)
}

fn setup_view<R: Runtime>(app_handle: &AppHandle<R>) -> SubMenuResult<R> {
    let builder = SubmenuBuilder::new(app_handle, "View");

    #[cfg(debug_assertions)]
    let builder = {
        let handle = app_handle.clone();
        app_handle.on_menu_event(move |_app, event: MenuEvent| {
            let Some(window) = handle.get_webview_window("main") else {
                return;
            };
            match event.id().0.as_str() {
                "reload" => {
                    let _ = window.eval("window.location.reload()");
                }
                "hard-reload" => {
                    let _ = window.clear_all_browsing_data();
                    let _ = window.eval("window.location.reload()");
                }
                "devtools" => {
                    window.open_devtools();
                }
                _ => {}
            }
        });

        builder
            .item(
                &IconMenuItemBuilder::with_id("reload", "Reload")
                    .icon(Image::from_bytes(include_bytes!(
                        "../icons/lucide-rotate-ccw-light.png"
                    ))?)
                    .accelerator("CmdOrCtrl+R")
                    .build(app_handle)?,
            )
            .item(
                &MenuItemBuilder::with_id("hard-reload", "Hard Reload")
                    .accelerator("CmdOrCtrl+Shift+R")
                    .build(app_handle)?,
            )
            .separator()
            .item(
                &IconMenuItemBuilder::with_id("devtools", "Developer Tools")
                    .icon(Image::from_bytes(include_bytes!(
                        "../icons/lucide-tool-case-light.png"
                    ))?)
                    .accelerator("CmdOrCtrl+Alt+I")
                    .build(app_handle)?,
            )
    };

    #[cfg(target_os = "macos")]
    let builder = builder.fullscreen_with_text("Fullscreen");

    Ok(builder.build()?)
}

fn setup_edit<R: Runtime>(app_handle: &AppHandle<R>) -> SubMenuResult<R> {
    let handle = app_handle.clone();
    app_handle.on_menu_event(move |_app, event: MenuEvent| {
        let Some(window) = handle.get_webview_window("main") else {
            return;
        };
        match event.id().0.as_str() {
            "undo" => {
                let _ = window.emit("app:edit:undo", ());
            }
            "redo" => {
                let _ = window.emit("app:edit:redo", ());
            }
            _ => {}
        }
    });

    let builder = SubmenuBuilder::new(app_handle, "Edit")
        .item(
            &MenuItemBuilder::with_id("undo", "Undo")
                .accelerator("CmdOrCtrl+Z")
                .build(app_handle)?,
        )
        .item(
            &MenuItemBuilder::with_id("redo", "Redo")
                .accelerator("CmdOrCtrl+Shift+Z")
                .build(app_handle)?,
        )
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all();

    Ok(builder.build()?)
}

fn setup_file<R: Runtime>(app_handle: &AppHandle<R>) -> SubMenuResult<R> {
    let handle = app_handle.clone();
    app_handle.on_menu_event(move |_app, event: MenuEvent| {
        if event.id().0.as_str() == "new"
            && let Some(window) = handle.get_webview_window("main")
        {
            let _ = window.emit("app:mark:new", ());
        }
    });

    let builder = SubmenuBuilder::new(app_handle, "File");

    let builder = builder.item(
        &MenuItemBuilder::with_id("new", "New")
            .accelerator("CmdOrCtrl+N")
            .build(app_handle)?,
    );
    let builder = builder.item(
        &MenuItemBuilder::with_id("new-window", "New Window")
            .accelerator("CmdOrCtrl+Shift+N")
            .build(app_handle)?,
    );

    let builder = builder.separator().close_window();

    #[cfg(not(target_os = "macos"))]
    let builder = builder.quit();

    Ok(builder.build()?)
}

fn setup_window<R: Runtime>(app_handle: &AppHandle<R>) -> SubMenuResult<R> {
    let builder = SubmenuBuilder::new(app_handle, "Window")
        .minimize()
        .maximize();

    Ok(builder.build()?)
}

fn setup_help<R: Runtime>(app_handle: &AppHandle<R>) -> SubMenuResult<R> {
    let builder = SubmenuBuilder::new(app_handle, "Help");

    #[cfg(not(target_os = "macos"))]
    let builder = builder
        .item(
            &MenuItemBuilder::with_id("check-for-updates", "Check for Updates...")
                .build(app_handle)?,
        )
        .separator()
        .about(setup_about(app_handle)?.into());

    Ok(builder.build()?)
}

/// Mirrors the `cfg` on the call site in [`setup`]: the Developer menu is a
/// debug-only affordance, so in a release build the function is not merely
/// unused, it is not wanted. Gating the definition rather than silencing the
/// warning keeps the two in step — an `allow(dead_code)` would still compile
/// it into shipped binaries.
#[cfg(debug_assertions)]
fn setup_developer<R: Runtime>(app_handle: &AppHandle<R>) -> SubMenuResult<R> {
    let builder = SubmenuBuilder::new(app_handle, "Developer");
    let handle = app_handle.clone();

    app_handle.on_menu_event(move |_app, event: MenuEvent| {
        if event.id().0.as_str() == "reset-database" {
            let handle = handle.clone();
            async_runtime::spawn(async move {
                let rt = handle.state::<Arc<WaypointsRuntime>>();

                if let Err(e) = rt.reset_db().await {
                    tracing::error!("database reset failed: {e}");
                    return;
                }

                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.eval("window.location.reload()");
                }
            });
        }
    });

    let builder = builder.item(
        &IconMenuItemBuilder::with_id("reset-database", "Reset Database")
            .icon(Image::from_bytes(include_bytes!(
                "../icons/lucide-database-zap-light.png"
            ))?)
            .build(app_handle)?,
    );

    Ok(builder.build()?)
}

fn setup_about<R: Runtime>(app_handle: &AppHandle<R>) -> SetupResult<AboutMetadata<'_>> {
    let pkg_info = app_handle.package_info();
    let config = app_handle.config();
    let about = AboutMetadataBuilder::new()
        .name("Waypoints".into())
        .copyright(config.bundle.copyright.clone())
        .authors(config.bundle.publisher.clone().map(|p| vec![p]))
        .version(pkg_info.version.to_string().into())
        .website("https://waypoints.so".into());

    Ok(about.build())
}

async fn check_for_updates(handle: AppHandle) {
    use cartograph::SelfUpdatePolicy;
    use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

    let instance = handle.state::<crate::launch::Instance>();
    let status = match crate::commands::self_update::status(&handle, &instance) {
        Ok(status) => status,
        Err(e) => {
            tracing::error!("failed to read the self-update setting: {e}");
            return;
        }
    };

    tracing::info!(policy = ?status.effective, "checking for updates");

    let updater = match handle.updater() {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("failed to create updater: {e}");
            return;
        }
    };

    match decide(status.effective, &updater).await {
        Outcome::Skipped => {
            tracing::info!("self-update is off; not checking");
        }
        Outcome::UpToDate => {
            tracing::info!("no update available");
        }
        Outcome::Failed(e) => {
            tracing::error!("update check failed: {e}");
        }
        Outcome::Install(update) => {
            tracing::info!("update available: v{}", update.version);

            if let Err(e) = update
                .download_and_install(
                    |downloaded, total| {
                        tracing::info!("downloading update: {downloaded}/{total:?} bytes");
                    },
                    || {
                        tracing::info!("download complete, installing");
                    },
                )
                .await
            {
                tracing::error!("update failed: {e}");
                return;
            }

            tracing::info!("update installed, restarting");
            handle.restart();
        }
        // Surface the new version, never download or install: either the
        // package that installed this copy capped it, or the user asked to be
        // told and nothing more.
        Outcome::Notify(update) => {
            tracing::info!("update available: v{} (notify only)", update.version);

            let why = if status.cap < SelfUpdatePolicy::On {
                "This copy is managed by a package manager, which delivers updates itself."
            } else {
                "Self-updating is set to notify only; change it in Settings to install updates automatically."
            };

            handle
                .dialog()
                .message(format!(
                    "Waypoints v{} is available (you have v{}).\n\n{why}",
                    update.version, update.current_version,
                ))
                .title("Update available")
                .kind(MessageDialogKind::Info)
                .show(|_| {});
        }
    }
}

/// The one call the policy gates, behind a trait so the gate is testable
/// without an updater.
pub(crate) trait UpdateCheck {
    type Update;
    type Error: std::fmt::Display;

    async fn check(&self) -> Result<Option<Self::Update>, Self::Error>;
}

impl UpdateCheck for tauri_plugin_updater::Updater {
    type Update = tauri_plugin_updater::Update;
    type Error = tauri_plugin_updater::Error;

    async fn check(&self) -> Result<Option<Self::Update>, Self::Error> {
        tauri_plugin_updater::Updater::check(self).await
    }
}

#[derive(Debug)]
pub(crate) enum Outcome<U> {
    /// The policy is `off`: the endpoint was not contacted.
    Skipped,
    UpToDate,
    /// A newer version, under `notify`.
    Notify(U),
    /// A newer version, under `on`.
    Install(U),
    Failed(String),
}

/// What to do about an update under `policy`: nothing at all under `off`, and
/// otherwise whatever the check finds, acted on only if the policy is `on`.
pub(crate) async fn decide<C: UpdateCheck>(
    policy: cartograph::SelfUpdatePolicy,
    checker: &C,
) -> Outcome<C::Update> {
    use cartograph::SelfUpdatePolicy::{Notify, Off, On};

    match policy {
        Off => Outcome::Skipped,
        Notify | On => match checker.check().await {
            Ok(None) => Outcome::UpToDate,
            Ok(Some(update)) if policy == On => Outcome::Install(update),
            Ok(Some(update)) => Outcome::Notify(update),
            Err(e) => Outcome::Failed(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cartograph::SelfUpdatePolicy::{Notify, Off, On};
    use std::cell::Cell;

    /// Counts checks and answers with a fixed result.
    struct FakeUpdater {
        checks: Cell<usize>,
        found: Option<&'static str>,
    }

    impl FakeUpdater {
        fn finding(found: Option<&'static str>) -> Self {
            Self {
                checks: Cell::new(0),
                found,
            }
        }
    }

    impl UpdateCheck for FakeUpdater {
        type Update = &'static str;
        type Error = String;

        async fn check(&self) -> Result<Option<&'static str>, String> {
            self.checks.set(self.checks.get() + 1);
            Ok(self.found)
        }
    }

    #[tokio::test]
    async fn off_never_calls_check() {
        let updater = FakeUpdater::finding(Some("v9.9.9"));
        assert!(matches!(decide(Off, &updater).await, Outcome::Skipped));
        assert_eq!(updater.checks.get(), 0);
    }

    #[tokio::test]
    async fn notify_checks_and_announces_without_installing() {
        let updater = FakeUpdater::finding(Some("v9.9.9"));
        assert!(matches!(
            decide(Notify, &updater).await,
            Outcome::Notify("v9.9.9")
        ));
        assert_eq!(updater.checks.get(), 1);
    }

    #[tokio::test]
    async fn on_checks_and_installs() {
        let updater = FakeUpdater::finding(Some("v9.9.9"));
        assert!(matches!(
            decide(On, &updater).await,
            Outcome::Install("v9.9.9")
        ));
        assert_eq!(updater.checks.get(), 1);
    }

    #[tokio::test]
    async fn nothing_newer_is_up_to_date_under_both_checking_policies() {
        for policy in [Notify, On] {
            let updater = FakeUpdater::finding(None);
            assert!(matches!(decide(policy, &updater).await, Outcome::UpToDate));
            assert_eq!(updater.checks.get(), 1);
        }
    }
}
