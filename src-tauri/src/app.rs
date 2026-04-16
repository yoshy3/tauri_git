use tauri::Manager;

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
    let window = app.get_webview_window("main").unwrap();
    let title = format!("Tauri Git v{}", env!("CARGO_PKG_VERSION"));
    window.set_title(&title)?;
    Ok(())
}
