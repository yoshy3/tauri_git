mod app;
mod commands;
mod git;
mod models;

pub fn run() {
    let state_flags = app::window_state_flags();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(state_flags)
                .build(),
        )
        .setup(app::setup_main_window)
        .invoke_handler(tauri::generate_handler![
            commands::open_repository,
            commands::get_repository_status,
            commands::commit_all,
            commands::commit_and_push,
            commands::fetch_origin,
            commands::pull_current_branch,
            commands::push_current_branch,
            commands::checkout_branch,
            commands::create_branch,
            commands::rebase_current_branch,
            commands::reset_current_branch,
            commands::delete_branch,
            commands::create_tag,
            commands::delete_tag,
            commands::stash_changes,
            commands::discard_changes,
            commands::apply_stash,
            commands::pop_stash,
            commands::get_worktree_file_diff,
            commands::get_commit_history_chunk,
            commands::get_commit_detail,
            commands::resolve_tag_target
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
