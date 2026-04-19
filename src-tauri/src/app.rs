use tauri::Manager;

#[cfg(target_os = "macos")]
use tauri::menu::{AboutMetadata, Menu, PredefinedMenuItem, Submenu};

pub(crate) fn window_state_flags() -> tauri_plugin_window_state::StateFlags {
    #[cfg(target_os = "linux")]
    let mut flags = tauri_plugin_window_state::StateFlags::all();
    #[cfg(not(target_os = "linux"))]
    let flags = tauri_plugin_window_state::StateFlags::all();

    #[cfg(target_os = "linux")]
    {
        flags.remove(tauri_plugin_window_state::StateFlags::MAXIMIZED);
        flags.remove(tauri_plugin_window_state::StateFlags::FULLSCREEN);
        flags.remove(tauri_plugin_window_state::StateFlags::DECORATIONS);
        flags.remove(tauri_plugin_window_state::StateFlags::VISIBLE);
    }

    println!("window state flags: {flags:?}");
    flags
}

pub(crate) fn setup_main_window(
    app: &mut tauri::App,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    configure_macos_app_menu(app)?;

    let window = app.get_webview_window("main").unwrap();
    let title = format!("Tauri Git v{}", env!("CARGO_PKG_VERSION"));
    window.set_title(&title)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn configure_macos_app_menu(app: &mut tauri::App) -> tauri::Result<()> {
    let app_handle = app.handle();
    let pkg_info = app_handle.package_info();
    let config = app_handle.config();

    let about_metadata = AboutMetadata {
        name: Some(pkg_info.name.clone()),
        version: Some(pkg_info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config.bundle.publisher.clone().map(|publisher| vec![publisher]),
        ..Default::default()
    };

    let menu = Menu::with_items(
        app_handle,
        &[
            &Submenu::with_items(
                app_handle,
                pkg_info.name.clone(),
                true,
                &[
                    &PredefinedMenuItem::about(app_handle, None, Some(about_metadata))?,
                    &PredefinedMenuItem::separator(app_handle)?,
                    &PredefinedMenuItem::services(app_handle, None)?,
                    &PredefinedMenuItem::separator(app_handle)?,
                    &PredefinedMenuItem::hide(app_handle, None)?,
                    &PredefinedMenuItem::hide_others(app_handle, None)?,
                    &PredefinedMenuItem::separator(app_handle)?,
                    &PredefinedMenuItem::quit(app_handle, None)?,
                ],
            )?,
            &Submenu::with_items(
                app_handle,
                "File",
                true,
                &[&PredefinedMenuItem::close_window(app_handle, None)?],
            )?,
            &Submenu::with_items(
                app_handle,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app_handle, None)?,
                    &PredefinedMenuItem::redo(app_handle, None)?,
                    &PredefinedMenuItem::separator(app_handle)?,
                    &PredefinedMenuItem::cut(app_handle, None)?,
                    &PredefinedMenuItem::copy(app_handle, None)?,
                    &PredefinedMenuItem::paste(app_handle, None)?,
                    &PredefinedMenuItem::select_all(app_handle, None)?,
                ],
            )?,
            &Submenu::with_items(
                app_handle,
                "View",
                true,
                &[&PredefinedMenuItem::fullscreen(app_handle, None)?],
            )?,
            &Submenu::with_items(
                app_handle,
                "Window",
                true,
                &[
                    &PredefinedMenuItem::minimize(app_handle, None)?,
                    &PredefinedMenuItem::maximize(app_handle, None)?,
                    &PredefinedMenuItem::separator(app_handle)?,
                    &PredefinedMenuItem::close_window(app_handle, None)?,
                ],
            )?,
        ],
    )?;

    app.set_menu(menu)?;
    Ok(())
}
