use std::sync::Arc;

#[cfg(target_os = "macos")]
use tauri::menu::Submenu;
use tauri::{
    App, AppHandle, Emitter, Manager, Runtime, async_runtime,
    image::Image,
    menu::{
        AboutMetadata, AboutMetadataBuilder, IconMenuItemBuilder, MenuBuilder, MenuEvent,
        MenuItemBuilder, SubmenuBuilder,
    },
};

use cartograph::WaypointsRuntime;

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

    Ok(())
}

fn setup_macos<R: tauri::Runtime>(app_handle: &AppHandle<R>) -> SubMenuResult<R> {
    let pkg_name = app_handle.package_info().name.clone();
    let metadata = setup_about(app_handle)?;
    Ok(SubmenuBuilder::new(app_handle, pkg_name)
        .about(Some(metadata))
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
        if event.id().0.as_str() == "new" {
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.emit("app:mark:new", ());
            }
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
    let builder = builder.about(setup_about(app_handle)?.into());

    Ok(builder.build()?)
}

fn setup_developer<R: Runtime>(app_handle: &AppHandle<R>) -> SubMenuResult<R> {
    let builder = SubmenuBuilder::new(app_handle, "Developer");
    let handle = app_handle.clone();

    app_handle.on_menu_event(move |_app, event: MenuEvent| match event.id().0.as_str() {
        "reset-database" => {
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
        _ => {}
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
