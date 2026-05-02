use crate::git;
use crate::models::{
    GitCommitDetail, GitCommitHistoryChunk, GitReferenceTarget, GitStatusResponse,
    GitWorktreeFileDiff,
};

async fn run_blocking<T, F>(job: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|error| {
            git::bilingual_with_detail(
                "バックグラウンド処理の実行に失敗しました",
                "Failed to run background task",
                error,
            )
        })?
}

#[tauri::command]
pub(crate) async fn open_repository(path: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn get_repository_status(path: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn commit_all(
    path: String,
    message: String,
    amend: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::create_commit(&repository, &message, amend)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn commit_and_push(
    path: String,
    message: String,
    create_upstream_if_missing: bool,
    amend: bool,
    force_with_lease: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::create_commit(&repository, &message, amend)?;
        git::push_current_branch_to_target(&repository, create_upstream_if_missing, force_with_lease)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn fetch_origin(path: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::fetch_default_remote(&repository)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn pull_current_branch(path: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::pull_current_branch_ff_only(&repository)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn push_current_branch(
    path: String,
    create_upstream_if_missing: bool,
    force_with_lease: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::push_current_branch_to_target(&repository, create_upstream_if_missing, force_with_lease)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn checkout_branch(
    path: String,
    branch_name: String,
    remote_name: Option<String>,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::checkout_repository_branch(&repository, &branch_name, remote_name.as_deref())?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn create_branch(
    path: String,
    branch_name: String,
    source_name: String,
    source_kind: String,
    source_remote_name: Option<String>,
    switch_after_create: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::create_branch_from_source(
            &repository,
            &branch_name,
            &source_name,
            &source_kind,
            source_remote_name.as_deref(),
            switch_after_create,
        )?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn rebase_current_branch(
    path: String,
    target_name: String,
    target_kind: String,
    target_remote_name: Option<String>,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::rebase_current_branch_onto_reference(
            &repository,
            &target_name,
            &target_kind,
            target_remote_name.as_deref(),
        )?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn reset_current_branch(
    path: String,
    target: String,
    reset_mode: String,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::reset_current_branch_to_commit(&repository, &target, &reset_mode)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn revert_commit(
    path: String,
    target: String,
    message: String,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::revert_commit(&repository, &target, &message)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn delete_branch(
    path: String,
    branch_name: String,
    branch_kind: String,
    remote_name: Option<String>,
    force_delete: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::delete_repository_branch(
            &repository,
            &branch_name,
            &branch_kind,
            remote_name.as_deref(),
            force_delete,
        )?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn create_tag(
    path: String,
    tag_name: String,
    target: String,
    message: Option<String>,
    push_after_create: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::create_repository_tag(&repository, &tag_name, &target, message.as_deref())?;
        if push_after_create {
            git::push_tag_to_origin(&repository, &tag_name)?;
        }
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn delete_tag(path: String, tag_name: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::delete_repository_tag(&repository, &tag_name)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn stash_changes(
    path: String,
    message: Option<String>,
    selected_paths: Vec<String>,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::create_stash(&mut repository, message.as_deref(), &selected_paths)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn discard_changes(
    path: String,
    selected_paths: Vec<String>,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::discard_selected_changes(&repository, &selected_paths)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn apply_stash(
    path: String,
    index: usize,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::apply_stash_entry(&mut repository, index)?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn pop_stash(
    path: String,
    index: Option<usize>,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = git::open_repo(&path)?;
        git::pop_stash_entry(&mut repository, index.unwrap_or(0))?;
        git::build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
pub(crate) async fn get_worktree_file_diff(
    path: String,
    file_path: String,
) -> Result<GitWorktreeFileDiff, String> {
    run_blocking(move || {
        let repository = git::open_repo(&path)?;
        git::load_worktree_file_diff(&repository, &file_path)
    })
    .await
}

#[tauri::command]
pub(crate) async fn get_commit_history_chunk(
    path: String,
    offset: usize,
    limit: usize,
) -> Result<GitCommitHistoryChunk, String> {
    run_blocking(move || {
        let repository = git::open_repo(&path)?;
        git::load_commit_history_chunk(&repository, offset, limit)
    })
    .await
}

#[tauri::command]
pub(crate) async fn get_commit_detail(
    path: String,
    oid: String,
) -> Result<GitCommitDetail, String> {
    run_blocking(move || {
        let repository = git::open_repo(&path)?;
        git::load_commit_detail(&repository, &oid)
    })
    .await
}

#[tauri::command]
pub(crate) async fn resolve_tag_target(
    path: String,
    tag_name: String,
) -> Result<GitReferenceTarget, String> {
    run_blocking(move || {
        let repository = git::open_repo(&path)?;
        git::resolve_tag_target_oid(&repository, &tag_name)
    })
    .await
}
