use git2::{
    Cred, DiffFormat, DiffOptions, FetchOptions, Oid, RemoteCallbacks, Repository, Signature,
    Status, StatusOptions,
};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
#[cfg(target_os = "linux")]
use std::env;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn window_state_flags() -> tauri_plugin_window_state::StateFlags {
    #[cfg(target_os = "linux")]
    let mut flags = tauri_plugin_window_state::StateFlags::all();
    #[cfg(not(target_os = "linux"))]
    let flags = tauri_plugin_window_state::StateFlags::all();

    #[cfg(target_os = "linux")]
    if is_wayland_session() {
        flags.remove(tauri_plugin_window_state::StateFlags::POSITION);
        flags.remove(tauri_plugin_window_state::StateFlags::VISIBLE);
    }

    flags
}

#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    matches!(env::var("XDG_SESSION_TYPE").ok().as_deref(), Some("wayland"))
        || env::var_os("WAYLAND_DISPLAY").is_some()
}

async fn run_blocking<T, F>(job: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|error| format!("バックグラウンド処理の実行に失敗しました: {error}"))?
}

#[derive(Serialize)]
struct GitStatusEntry {
    path: String,
    working_tree_status: String,
    index_status: String,
}

#[derive(Serialize)]
struct GitStatusResponse {
    repo_name: String,
    repo_path: String,
    branch: String,
    head_oid: Option<String>,
    history_revision: String,
    has_origin_remote: bool,
    can_push_current_branch: bool,
    current_branch_upstream_name: Option<String>,
    ahead_count: usize,
    behind_count: usize,
    is_clean: bool,
    entries: Vec<GitStatusEntry>,
    head_summary: Option<String>,
    local_branches: Vec<String>,
    local_branch_syncs: Vec<GitLocalBranchSync>,
    remote_groups: Vec<GitRemoteGroup>,
    tags: Vec<String>,
    stashes: Vec<GitStashEntry>,
    submodules: Vec<GitSubmoduleEntry>,
}

#[derive(Serialize)]
struct GitRemoteGroup {
    name: String,
    branches: Vec<String>,
}

#[derive(Serialize)]
struct GitLocalBranchSync {
    name: String,
    ahead_count: usize,
    behind_count: usize,
}

#[derive(Serialize)]
struct GitStashEntry {
    index: usize,
    name: String,
    message: String,
}

#[derive(Serialize)]
struct GitSubmoduleEntry {
    name: String,
    path: String,
}

#[derive(Serialize)]
struct GitCommitSummary {
    oid: String,
    id: String,
    summary: String,
    author: String,
    committed_at: String,
    parent_ids: Vec<String>,
    on_current_branch: bool,
    labels: Vec<GitRefLabel>,
}

struct PushTarget {
    remote_name: String,
    remote_branch_name: String,
    set_upstream: bool,
}

#[derive(Clone, Serialize)]
struct GitRefLabel {
    name: String,
    scope: String,
    is_current: bool,
}

#[derive(Serialize)]
struct GitCommitHistoryChunk {
    commits: Vec<GitCommitSummary>,
    has_more: bool,
}

#[derive(Serialize)]
struct GitCommitDetail {
    oid: String,
    id: String,
    summary: String,
    message: String,
    author: GitCommitPerson,
    committer: GitCommitPerson,
    parents: Vec<GitCommitParent>,
    labels: Vec<GitRefLabel>,
    files: Vec<GitCommitFileDiff>,
}

#[derive(Serialize)]
struct GitCommitPerson {
    name: String,
    email: String,
    committed_at: String,
}

#[derive(Serialize)]
struct GitCommitParent {
    oid: String,
    id: String,
}

#[derive(Serialize)]
struct GitCommitFileDiff {
    path: String,
    status: String,
    patch: String,
}

#[derive(Serialize)]
struct GitReferenceTarget {
    oid: String,
}

#[derive(Serialize)]
struct GitWorktreeFileDiff {
    path: String,
    patch: String,
}

#[tauri::command]
async fn open_repository(path: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn get_repository_status(path: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn commit_all(path: String, message: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        create_commit(&repository, &message)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn commit_and_push(
    path: String,
    message: String,
    create_upstream_if_missing: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        create_commit(&repository, &message)?;
        push_current_branch_to_target(&repository, create_upstream_if_missing)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn fetch_origin(path: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        fetch_default_remote(&repository)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn pull_current_branch(path: String) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        pull_current_branch_ff_only(&repository)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn push_current_branch(
    path: String,
    create_upstream_if_missing: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        push_current_branch_to_target(&repository, create_upstream_if_missing)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn checkout_branch(
    path: String,
    branch_name: String,
    remote_name: Option<String>,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        checkout_repository_branch(&repository, &branch_name, remote_name.as_deref())?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn create_branch(
    path: String,
    branch_name: String,
    source_name: String,
    source_kind: String,
    source_remote_name: Option<String>,
    switch_after_create: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        create_branch_from_source(
            &repository,
            &branch_name,
            &source_name,
            &source_kind,
            source_remote_name.as_deref(),
            switch_after_create,
        )?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn delete_branch(
    path: String,
    branch_name: String,
    branch_kind: String,
    remote_name: Option<String>,
    force_delete: bool,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        delete_repository_branch(
            &repository,
            &branch_name,
            &branch_kind,
            remote_name.as_deref(),
            force_delete,
        )?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn stash_changes(
    path: String,
    message: Option<String>,
    selected_paths: Vec<String>,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        create_stash(&mut repository, message.as_deref(), &selected_paths)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn discard_changes(
    path: String,
    selected_paths: Vec<String>,
) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        discard_selected_changes(&repository, &selected_paths)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn apply_stash(path: String, index: usize) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        apply_stash_entry(&mut repository, index)?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn pop_stash(path: String, index: Option<usize>) -> Result<GitStatusResponse, String> {
    run_blocking(move || {
        let mut repository = open_repo(&path)?;
        pop_stash_entry(&mut repository, index.unwrap_or(0))?;
        build_repository_status(&mut repository)
    })
    .await
}

#[tauri::command]
async fn get_worktree_file_diff(
    path: String,
    file_path: String,
) -> Result<GitWorktreeFileDiff, String> {
    run_blocking(move || {
        let repository = open_repo(&path)?;
        load_worktree_file_diff(&repository, &file_path)
    })
    .await
}

#[tauri::command]
async fn get_commit_history_chunk(
    path: String,
    offset: usize,
    limit: usize,
) -> Result<GitCommitHistoryChunk, String> {
    run_blocking(move || {
        let repository = open_repo(&path)?;
        load_commit_history_chunk(&repository, offset, limit)
    })
    .await
}

#[tauri::command]
async fn get_commit_detail(path: String, oid: String) -> Result<GitCommitDetail, String> {
    run_blocking(move || {
        let repository = open_repo(&path)?;
        load_commit_detail(&repository, &oid)
    })
    .await
}

#[tauri::command]
async fn resolve_tag_target(path: String, tag_name: String) -> Result<GitReferenceTarget, String> {
    run_blocking(move || {
        let repository = open_repo(&path)?;
        resolve_tag_target_oid(&repository, &tag_name)
    })
    .await
}

fn open_repo(path: &str) -> Result<Repository, String> {
    let normalized = PathBuf::from(path);
    Repository::discover(&normalized).map_err(|error| {
        format!(
            "Git リポジトリを開けませんでした: {} ({})",
            normalized.display(),
            error.message()
        )
    })
}

fn build_repository_status(repository: &mut Repository) -> Result<GitStatusResponse, String> {
    let repo_root = repository_root(repository)?;
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository")
        .to_string();
    let repo_path = repo_root.display().to_string();

    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = repository
        .statuses(Some(&mut status_options))
        .map_err(|error| format!("ステータスを取得できませんでした: {}", error.message()))?;

    let mut entries = Vec::new();

    for entry in statuses.iter() {
        let Some(path) = entry.path() else {
            continue;
        };

        let status = entry.status();
        if status.is_empty() {
            continue;
        }

        entries.push(GitStatusEntry {
            path: path.to_string(),
            index_status: index_status_code(status).to_string(),
            working_tree_status: worktree_status_code(status).to_string(),
        });
    }

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    drop(statuses);

    let current_branch_name = current_local_branch_name(repository);
    let branch = current_branch_name
        .clone()
        .unwrap_or_else(|| "DETACHED".to_string());

    let head_summary = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|commit| {
            let short_id = commit
                .as_object()
                .short_id()
                .ok()
                .and_then(|buf| buf.as_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| commit.id().to_string().chars().take(7).collect());

            let summary = commit.summary().unwrap_or("(no summary)");
            format!("{short_id} {summary}")
        });
    let head_oid = repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .map(|oid| oid.to_string());
    let history_revision = build_history_revision(repository)?;
    let local_branch_syncs = load_local_branch_syncs(repository)?;
    let current_branch_upstream_name = current_branch_name
        .as_deref()
        .map(|branch_name| {
            repository
                .find_branch(branch_name, git2::BranchType::Local)
                .map_err(|error| format!("current branch could not be loaded: {}", error.message()))
                .and_then(|branch| load_branch_upstream_name(&branch))
        })
        .transpose()?
        .flatten();
    let (ahead_count, behind_count) = local_branch_syncs
        .iter()
        .find(|entry| entry.name == branch)
        .map(|entry| (entry.ahead_count, entry.behind_count))
        .unwrap_or((0, 0));
    let local_branches = local_branch_syncs
        .iter()
        .map(|entry| entry.name.clone())
        .collect();
    let remote_groups = load_remote_groups(repository)?;
    let tags = load_tags(repository)?;
    let stashes = load_stashes(repository)?;
    let submodules = load_submodules(repository)?;
    let has_origin_remote = has_remote(repository, "origin")?;
    let can_push_current_branch = current_branch_name.is_some()
        && (current_branch_upstream_name.is_some() || has_origin_remote);

    Ok(GitStatusResponse {
        repo_name,
        repo_path,
        branch,
        head_oid,
        history_revision,
        has_origin_remote,
        can_push_current_branch,
        current_branch_upstream_name,
        ahead_count,
        behind_count,
        is_clean: entries.is_empty(),
        entries,
        head_summary,
        local_branches,
        local_branch_syncs,
        remote_groups,
        tags,
        stashes,
        submodules,
    })
}

fn load_branch_upstream_sync_counts(
    repository: &Repository,
    branch: &git2::Branch<'_>,
) -> Result<(usize, usize), String> {
    let upstream = match branch.upstream() {
        Ok(upstream) => upstream,
        Err(_) => return Ok((0, 0)),
    };

    let local_oid = match branch.get().target() {
        Some(oid) => oid,
        None => return Ok((0, 0)),
    };
    let upstream_oid = match upstream.get().target() {
        Some(oid) => oid,
        None => return Ok((0, 0)),
    };

    repository
        .graph_ahead_behind(local_oid, upstream_oid)
        .map_err(|error| {
            format!(
                "upstream との差分を取得できませんでした: {}",
                error.message()
            )
        })
}

fn current_local_branch_name(repository: &Repository) -> Option<String> {
    repository
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(ToOwned::to_owned))
        .filter(|name| !name.is_empty() && name != "HEAD")
}

fn load_branch_upstream_name(branch: &git2::Branch<'_>) -> Result<Option<String>, String> {
    let upstream = match branch.upstream() {
        Ok(upstream) => upstream,
        Err(_) => return Ok(None),
    };

    upstream
        .name()
        .map(|name| name.map(ToOwned::to_owned))
        .map_err(|error| {
            format!(
                "upstream branch name could not be read: {}",
                error.message()
            )
        })
}

fn parse_remote_branch_ref(reference_name: &str) -> Option<(String, String)> {
    let shorthand = reference_name
        .strip_prefix("refs/remotes/")
        .unwrap_or(reference_name);
    let (remote_name, branch_name) = shorthand.split_once('/')?;

    if remote_name.is_empty() || branch_name.is_empty() {
        return None;
    }

    Some((remote_name.to_string(), branch_name.to_string()))
}

fn resolve_push_target(
    repository: &Repository,
    create_upstream_if_missing: bool,
) -> Result<(String, PushTarget), String> {
    let local_branch_name = current_local_branch_name(repository)
        .ok_or_else(|| "current branch could not be determined".to_string())?;
    let branch = repository
        .find_branch(&local_branch_name, git2::BranchType::Local)
        .map_err(|error| format!("current branch could not be loaded: {}", error.message()))?;

    if let Ok(upstream) = branch.upstream() {
        if let Some(reference_name) = upstream.get().name() {
            if let Some((remote_name, remote_branch_name)) = parse_remote_branch_ref(reference_name)
            {
                return Ok((
                    local_branch_name,
                    PushTarget {
                        remote_name,
                        remote_branch_name,
                        set_upstream: false,
                    },
                ));
            }
        }

        if let Some(shorthand) = upstream.name().map_err(|error| {
            format!(
                "upstream branch name could not be read: {}",
                error.message()
            )
        })? {
            if let Some((remote_name, remote_branch_name)) = parse_remote_branch_ref(shorthand) {
                return Ok((
                    local_branch_name,
                    PushTarget {
                        remote_name,
                        remote_branch_name,
                        set_upstream: false,
                    },
                ));
            }
        }

        return Err("upstream branch could not be resolved".to_string());
    }

    if !create_upstream_if_missing {
        return Err("current branch does not have an upstream branch".to_string());
    }

    if !has_remote(repository, "origin")? {
        return Err("origin remote is not configured".to_string());
    }

    Ok((
        local_branch_name.clone(),
        PushTarget {
            remote_name: "origin".to_string(),
            remote_branch_name: local_branch_name,
            set_upstream: true,
        },
    ))
}

fn create_commit(repository: &Repository, message: &str) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("コミットメッセージが空です。".to_string());
    }

    let mut index = repository
        .index()
        .map_err(|error| format!("インデックスを開けませんでした: {}", error.message()))?;

    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|error| format!("変更をステージングできませんでした: {}", error.message()))?;

    index
        .write()
        .map_err(|error| format!("インデックスを書き込めませんでした: {}", error.message()))?;

    let tree_id = index
        .write_tree()
        .map_err(|error| format!("ツリーを書き込めませんでした: {}", error.message()))?;
    let tree = repository
        .find_tree(tree_id)
        .map_err(|error| format!("ツリーを読み込めませんでした: {}", error.message()))?;

    let signature = repository
        .signature()
        .or_else(|_| Signature::now("Tauri Git", "tauri-git@example.local"))
        .map_err(|error| format!("コミット署名を作れませんでした: {}", error.message()))?;

    let parent_commit = repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repository.find_commit(oid).ok());

    if tree_is_unchanged(&tree, parent_commit.as_ref())? {
        return Err("コミット対象の変更がありません。".to_string());
    }

    let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();

    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message.trim(),
            &tree,
            &parents,
        )
        .map_err(|error| format!("コミットに失敗しました: {}", error.message()))?;

    repository
        .checkout_head(None)
        .map_err(|error| format!("作業ツリーを更新できませんでした: {}", error.message()))?;

    Ok(())
}

fn fetch_default_remote(repository: &Repository) -> Result<(), String> {
    let mut remote = repository
        .find_remote("origin")
        .map_err(|error| format!("origin リモートを開けませんでした: {}", error.message()))?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, _allowed_types| {
        let config = repository.config()?;

        Cred::credential_helper(&config, url, username_from_url).or_else(|_| {
            username_from_url
                .map(Cred::ssh_key_from_agent)
                .unwrap_or_else(|| Err(git2::Error::from_str("利用できる認証情報がありません。")))
        })
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    remote
        .fetch(&[] as &[&str], Some(&mut fetch_options), None)
        .map_err(|error| format!("Fetch に失敗しました: {}", error.message()))?;

    Ok(())
}

fn pull_current_branch_ff_only(repository: &Repository) -> Result<(), String> {
    let repo_root = repository_root(repository)?;

    let output = git_command()
        .current_dir(repo_root)
        .arg("pull")
        .arg("--ff-only")
        .output()
        .map_err(|error| format!("Failed to run git pull: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "git pull failed without output".to_string()
        };

        return Err(format!("git pull failed: {detail}"));
    }

    Ok(())
}

fn push_current_branch_to_target(
    repository: &Repository,
    create_upstream_if_missing: bool,
) -> Result<(), String> {
    let (branch, target) = resolve_push_target(repository, create_upstream_if_missing)?;
    let output = git_command()
        .current_dir(repository_root(repository)?)
        .arg("push")
        .args(target.set_upstream.then_some("-u"))
        .arg(&target.remote_name)
        .arg(format!("{branch}:{}", target.remote_branch_name))
        .output()
        .map_err(|error| format!("Failed to run git push: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "git push failed without output".to_string()
        };

        return Err(format!("git push failed: {detail}"));
    }

    Ok(())
}

fn checkout_repository_branch(
    repository: &Repository,
    branch_name: &str,
    remote_name: Option<&str>,
) -> Result<(), String> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        return Err("branch name is empty".to_string());
    }

    let current_branch = repository
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(ToOwned::to_owned));

    if remote_name.is_none() && current_branch.as_deref() == Some(branch_name) {
        return Ok(());
    }

    let repo_root = repository_root(repository)?;
    let mut command = git_command();
    command.current_dir(repo_root);

    match remote_name {
        Some(remote) if !remote.trim().is_empty() => {
            let remote_branch = format!("{remote}/{branch_name}");
            let has_local_branch = repository
                .find_branch(branch_name, git2::BranchType::Local)
                .is_ok();

            command.arg("checkout");
            if has_local_branch {
                command.arg(branch_name);
            } else {
                command
                    .arg("-b")
                    .arg(branch_name)
                    .arg("--track")
                    .arg(remote_branch);
            }
        }
        _ => {
            command.arg("checkout").arg(branch_name);
        }
    }

    let output = command
        .output()
        .map_err(|error| format!("Failed to run git checkout: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "git checkout failed without output".to_string()
        };

        return Err(format!("git checkout failed: {detail}"));
    }

    Ok(())
}

fn create_branch_from_source(
    repository: &Repository,
    branch_name: &str,
    source_name: &str,
    source_kind: &str,
    source_remote_name: Option<&str>,
    switch_after_create: bool,
) -> Result<(), String> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        return Err("branch name is empty".to_string());
    }

    let source_name = source_name.trim();
    if source_name.is_empty() {
        return Err("branch source is empty".to_string());
    }

    let source_ref = match source_kind {
        "remote_branch" => {
            let remote_name = source_remote_name
                .filter(|name| !name.trim().is_empty())
                .ok_or_else(|| "remote source is missing its remote name".to_string())?;
            format!("{remote_name}/{source_name}")
        }
        "local_branch" | "tag" => source_name.to_string(),
        _ => return Err(format!("unsupported branch source kind: {source_kind}")),
    };

    let repo_root = repository_root(repository)?;
    let mut command = git_command();
    command.current_dir(repo_root);

    if switch_after_create {
        if source_kind == "remote_branch" {
            command
                .arg("checkout")
                .arg("-b")
                .arg(branch_name)
                .arg("--track")
                .arg(&source_ref);
        } else {
            command
                .arg("checkout")
                .arg("-b")
                .arg(branch_name)
                .arg(&source_ref);
        }
    } else if source_kind == "remote_branch" {
        command
            .arg("branch")
            .arg("--track")
            .arg(branch_name)
            .arg(&source_ref);
    } else {
        command.arg("branch").arg(branch_name).arg(&source_ref);
    }

    let output = command
        .output()
        .map_err(|error| format!("Failed to run git branch creation: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "git branch creation failed without output".to_string()
        };

        return Err(format!("git branch creation failed: {detail}"));
    }

    Ok(())
}

fn delete_repository_branch(
    repository: &Repository,
    branch_name: &str,
    branch_kind: &str,
    remote_name: Option<&str>,
    force_delete: bool,
) -> Result<(), String> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        return Err("branch name is empty".to_string());
    }

    let repo_root = repository_root(repository)?;
    let mut command = git_command();
    command.current_dir(repo_root);

    match branch_kind {
        "local_branch" => {
            let current_branch = repository
                .head()
                .ok()
                .and_then(|head| head.shorthand().map(ToOwned::to_owned));
            if current_branch.as_deref() == Some(branch_name) {
                return Err("cannot delete the currently checked out branch".to_string());
            }

            command
                .arg("branch")
                .arg(if force_delete { "-D" } else { "-d" })
                .arg(branch_name);
        }
        "remote_branch" => {
            let remote_name = remote_name
                .filter(|name| !name.trim().is_empty())
                .ok_or_else(|| "remote branch delete requires a remote name".to_string())?;

            command
                .arg("push")
                .arg(remote_name)
                .arg("--delete")
                .arg(branch_name);
        }
        _ => return Err(format!("unsupported branch delete kind: {branch_kind}")),
    }

    let output = command
        .output()
        .map_err(|error| format!("Failed to run git branch delete: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "git branch delete failed without output".to_string()
        };

        return Err(format!("git branch delete failed: {detail}"));
    }

    Ok(())
}

fn create_stash(
    repository: &mut Repository,
    message: Option<&str>,
    selected_paths: &[String],
) -> Result<(), String> {
    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = repository
        .statuses(Some(&mut status_options))
        .map_err(|error| format!("stash 対象を確認できませんでした: {}", error.message()))?;

    if statuses.is_empty() {
        return Err("stash する変更がありません。".to_string());
    }
    drop(statuses);

    if selected_paths.is_empty() {
        return Err("stash 対象のファイルを選択してください。".to_string());
    }

    let repo_root = repository_root(repository)?;
    let stash_message = match message.map(str::trim) {
        Some("") | None => "tauri-git stash",
        Some(text) => text,
    };

    let mut command = git_command();
    command
        .current_dir(repo_root)
        .arg("stash")
        .arg("push")
        .arg("--include-untracked")
        .arg("-m")
        .arg(stash_message)
        .arg("--");

    for path in selected_paths {
        command.arg(path);
    }

    let output = command
        .output()
        .map_err(|error| format!("stash コマンドを実行できませんでした: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "詳細なエラーを取得できませんでした。".to_string()
        };

        return Err(format!("stash に失敗しました: {detail}"));
    }

    Ok(())
}

fn discard_selected_changes(
    repository: &Repository,
    selected_paths: &[String],
) -> Result<(), String> {
    if selected_paths.is_empty() {
        return Err("discard する変更がありません。".to_string());
    }

    let mut status_options = StatusOptions::new();
    status_options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);

    let statuses = repository
        .statuses(Some(&mut status_options))
        .map_err(|error| format!("discard 対象を確認できませんでした: {}", error.message()))?;

    if statuses.is_empty() {
        return Err("discard できる変更がありません。".to_string());
    }

    let selected_path_set = selected_paths
        .iter()
        .map(|path| path.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut restore_paths = Vec::new();
    let mut remove_paths = Vec::new();

    for entry in statuses.iter() {
        let Some(path) = entry.path() else {
            continue;
        };

        if !selected_path_set.contains(path) {
            continue;
        }

        let status = entry.status();
        if status.contains(Status::INDEX_NEW) || status.contains(Status::WT_NEW) {
            remove_paths.push(path.to_string());
        } else {
            restore_paths.push(path.to_string());
        }
    }

    if restore_paths.is_empty() && remove_paths.is_empty() {
        return Err("discard 対象のファイルを選択してください。".to_string());
    }

    let repo_root = repository_root(repository)?;

    if !restore_paths.is_empty() {
        let mut command = git_command();
        command
            .current_dir(&repo_root)
            .arg("restore")
            .arg("--source=HEAD")
            .arg("--staged")
            .arg("--worktree")
            .arg("--");

        for path in &restore_paths {
            command.arg(path);
        }

        run_git_command(command, "discard")?;
    }

    if !remove_paths.is_empty() {
        let mut tracked_removals = Vec::new();
        let mut untracked_removals = Vec::new();

        for path in &remove_paths {
            if repository.find_path_in_head(path).unwrap_or(false) {
                tracked_removals.push(path.clone());
            } else {
                untracked_removals.push(path.clone());
            }
        }

        if !tracked_removals.is_empty() {
            let mut command = git_command();
            command
                .current_dir(&repo_root)
                .arg("rm")
                .arg("-f")
                .arg("--");

            for path in &tracked_removals {
                command.arg(path);
            }

            run_git_command(command, "discard")?;
        }

        if !untracked_removals.is_empty() {
            let mut command = git_command();
            command
                .current_dir(&repo_root)
                .arg("clean")
                .arg("-fd")
                .arg("--");

            for path in &untracked_removals {
                command.arg(path);
            }

            run_git_command(command, "discard")?;
        }
    }

    Ok(())
}

fn apply_stash_entry(repository: &mut Repository, index: usize) -> Result<(), String> {
    match repository.stash_apply(index, None) {
        Ok(()) => {
            reset_index_to_head(repository)?;
            Ok(())
        }
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            Err("適用できる stash がありません。".to_string())
        }
        Err(error) => Err(format!("stash apply に失敗しました: {}", error.message())),
    }
}

fn pop_stash_entry(repository: &mut Repository, index: usize) -> Result<(), String> {
    match repository.stash_pop(index, None) {
        Ok(()) => {
            reset_index_to_head(repository)?;
            Ok(())
        }
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            Err("適用できる stash がありません。".to_string())
        }
        Err(error) => Err(format!("stash pop に失敗しました: {}", error.message())),
    }
}

fn reset_index_to_head(repository: &Repository) -> Result<(), String> {
    let mut index = repository
        .index()
        .map_err(|error| format!("インデックスを開けませんでした: {}", error.message()))?;

    if let Ok(head) = repository.head() {
        if let Ok(tree) = head.peel_to_tree() {
            index.read_tree(&tree).map_err(|error| {
                format!(
                    "インデックスを HEAD に戻せませんでした: {}",
                    error.message()
                )
            })?;
        } else {
            index.clear().map_err(|error| {
                format!("インデックスを初期化できませんでした: {}", error.message())
            })?;
        }
    } else {
        index.clear().map_err(|error| {
            format!("インデックスを初期化できませんでした: {}", error.message())
        })?;
    }

    index
        .write()
        .map_err(|error| format!("インデックスを書き込めませんでした: {}", error.message()))?;

    Ok(())
}

fn repository_root(repository: &Repository) -> Result<PathBuf, String> {
    repository
        .workdir()
        .or_else(|| repository.path().parent())
        .ok_or_else(|| "リポジトリのルートパスを解決できませんでした。".to_string())
        .map(|path| path.to_path_buf())
}

fn git_command() -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new("git");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

fn run_git_command(mut command: Command, action_name: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("{action_name} コマンドを実行できませんでした: {}", error))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "詳細なエラーを取得できませんでした。".to_string()
    };

    Err(format!("{action_name} に失敗しました: {detail}"))
}

fn run_git_diff_command(mut command: Command, action_name: &str) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("{action_name} コマンドを実行できませんでした: {}", error))?;

    match output.status.code() {
        Some(0) | Some(1) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                "詳細なエラーを取得できませんでした。".to_string()
            };

            Err(format!("{action_name} に失敗しました: {detail}"))
        }
    }
}

fn load_worktree_file_diff(
    repository: &Repository,
    file_path: &str,
) -> Result<GitWorktreeFileDiff, String> {
    let repo_root = repository_root(repository)?;
    let patch = if repository.find_path_in_head(file_path)? {
        run_git_diff_command(
            {
                let mut command = git_command();
                command
                    .current_dir(&repo_root)
                    .arg("diff")
                    .arg("--no-ext-diff")
                    .arg("--no-color")
                    .arg("HEAD")
                    .arg("--")
                    .arg(file_path);
                command
            },
            "変更差分の取得",
        )?
    } else {
        run_git_diff_command(
            {
                let mut command = git_command();
                command
                    .current_dir(&repo_root)
                    .arg("diff")
                    .arg("--no-ext-diff")
                    .arg("--no-color")
                    .arg("--no-index")
                    .arg("--")
                    .arg("/dev/null")
                    .arg(file_path);
                command
            },
            "新規ファイル差分の取得",
        )?
    };

    Ok(GitWorktreeFileDiff {
        path: file_path.to_string(),
        patch,
    })
}

trait RepositoryHeadExt {
    fn find_path_in_head(&self, path: &str) -> Result<bool, String>;
}

impl RepositoryHeadExt for Repository {
    fn find_path_in_head(&self, path: &str) -> Result<bool, String> {
        let head = match self.head() {
            Ok(head) => head,
            Err(_) => return Ok(false),
        };
        let tree = match head.peel_to_tree() {
            Ok(tree) => tree,
            Err(_) => return Ok(false),
        };

        match tree.get_path(std::path::Path::new(path)) {
            Ok(_) => Ok(true),
            Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(false),
            Err(error) => Err(format!(
                "HEAD のファイル状態を確認できませんでした: {}",
                error.message()
            )),
        }
    }
}

fn has_remote(repository: &Repository, remote_name: &str) -> Result<bool, String> {
    let remotes = repository
        .remotes()
        .map_err(|error| format!("failed to inspect remotes: {}", error.message()))?;

    Ok(remotes.iter().flatten().any(|name| name == remote_name))
}

fn build_history_revision(repository: &Repository) -> Result<String, String> {
    let mut entries = Vec::new();

    if let Ok(head) = repository.head() {
        if let Some(target) = head.target() {
            entries.push(format!("HEAD={target}"));
        }
    }

    let references = repository.references().map_err(|error| {
        format!(
            "履歴リビジョン情報を取得できませんでした: {}",
            error.message()
        )
    })?;

    for reference_result in references {
        let reference = reference_result
            .map_err(|error| format!("参照情報を読み込めませんでした: {}", error.message()))?;
        let Some(name) = reference.name() else {
            continue;
        };

        if !(name.starts_with("refs/heads/")
            || name.starts_with("refs/remotes/")
            || name.starts_with("refs/tags/"))
        {
            continue;
        }

        let target = reference
            .target()
            .map(|oid| oid.to_string())
            .unwrap_or_else(|| "none".to_string());

        entries.push(format!("{name}={target}"));
    }

    entries.sort();
    Ok(entries.join("|"))
}

fn tree_is_unchanged(
    new_tree: &git2::Tree<'_>,
    parent_commit: Option<&git2::Commit<'_>>,
) -> Result<bool, String> {
    let Some(parent_commit) = parent_commit else {
        return Ok(new_tree.is_empty());
    };

    let parent_tree = parent_commit.tree().map_err(|error| {
        format!(
            "親コミットのツリーを取得できませんでした: {}",
            error.message()
        )
    })?;

    Ok(parent_tree.id() == new_tree.id())
}

fn index_status_code(status: Status) -> char {
    if status.contains(Status::INDEX_NEW) {
        'A'
    } else if status.contains(Status::INDEX_MODIFIED) {
        'M'
    } else if status.contains(Status::INDEX_DELETED) {
        'D'
    } else if status.contains(Status::INDEX_RENAMED) {
        'R'
    } else if status.contains(Status::INDEX_TYPECHANGE) {
        'T'
    } else {
        '.'
    }
}

fn worktree_status_code(status: Status) -> char {
    if status.contains(Status::WT_NEW) {
        'A'
    } else if status.contains(Status::WT_MODIFIED) {
        'M'
    } else if status.contains(Status::WT_DELETED) {
        'D'
    } else if status.contains(Status::WT_RENAMED) {
        'R'
    } else if status.contains(Status::WT_TYPECHANGE) {
        'T'
    } else {
        '.'
    }
}

fn load_commit_history_chunk(
    repository: &Repository,
    offset: usize,
    limit: usize,
) -> Result<GitCommitHistoryChunk, String> {
    let current_head_oid = repository.head().ok().and_then(|head| head.target());
    let current_branch_name = repository
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(ToOwned::to_owned));
    let reference_labels = load_reference_labels(repository, current_branch_name.as_deref())?;
    let mut revwalk = repository
        .revwalk()
        .map_err(|error| format!("コミット履歴を読み込めませんでした: {}", error.message()))?;

    push_history_refs(repository, &mut revwalk)?;
    revwalk
        .set_sorting(git2::Sort::TOPOLOGICAL)
        .map_err(|error| format!("コミット履歴の並び替えに失敗しました: {}", error.message()))?;

    let mut commits = Vec::new();
    let mut has_more = false;

    for (index, oid_result) in revwalk.enumerate() {
        if index < offset {
            continue;
        }

        if commits.len() >= limit {
            has_more = true;
            break;
        }

        let oid = oid_result
            .map_err(|error| format!("コミット ID を取得できませんでした: {}", error.message()))?;
        let commit = repository
            .find_commit(oid)
            .map_err(|error| format!("コミットを読み込めませんでした: {}", error.message()))?;

        let timestamp = commit.time().seconds();
        let committed_at = chrono::DateTime::from_timestamp(timestamp, 0)
            .map(|datetime| datetime.format("%Y-%m-%dT%H:%M:%S").to_string())
            .unwrap_or_else(|| "unknown time".to_string());
        let parent_ids = commit
            .parent_ids()
            .map(|parent_id| parent_id.to_string())
            .collect();
        let on_current_branch = current_head_oid
            .map(|head_oid| {
                head_oid == oid
                    || repository
                        .graph_descendant_of(head_oid, oid)
                        .unwrap_or(false)
            })
            .unwrap_or(false);

        commits.push(GitCommitSummary {
            oid: oid.to_string(),
            id: oid.to_string().chars().take(7).collect(),
            summary: commit.summary().unwrap_or("(no summary)").to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            committed_at,
            parent_ids,
            on_current_branch,
            labels: reference_labels
                .get(&oid.to_string())
                .cloned()
                .unwrap_or_default(),
        });
    }

    Ok(GitCommitHistoryChunk { commits, has_more })
}

fn load_commit_detail(repository: &Repository, oid: &str) -> Result<GitCommitDetail, String> {
    let current_branch_name = repository
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(ToOwned::to_owned));
    let reference_labels = load_reference_labels(repository, current_branch_name.as_deref())?;
    let oid = Oid::from_str(oid)
        .map_err(|error| format!("コミット ID が不正です: {}", error.message()))?;
    let commit = repository
        .find_commit(oid)
        .map_err(|error| format!("コミットを読み込めませんでした: {}", error.message()))?;

    let files = load_commit_file_diffs(repository, &commit)?;

    Ok(GitCommitDetail {
        oid: oid.to_string(),
        id: oid.to_string().chars().take(7).collect(),
        summary: commit.summary().unwrap_or("(no summary)").to_string(),
        message: commit.message().unwrap_or("").trim_end().to_string(),
        author: build_commit_person(commit.author()),
        committer: build_commit_person(commit.committer()),
        parents: commit
            .parent_ids()
            .map(|parent_oid| GitCommitParent {
                oid: parent_oid.to_string(),
                id: parent_oid.to_string().chars().take(7).collect(),
            })
            .collect(),
        labels: reference_labels
            .get(&oid.to_string())
            .cloned()
            .unwrap_or_default(),
        files,
    })
}

fn build_commit_person(signature: git2::Signature<'_>) -> GitCommitPerson {
    GitCommitPerson {
        name: signature.name().unwrap_or("Unknown").to_string(),
        email: signature.email().unwrap_or("").to_string(),
        committed_at: format_signature_time(signature.when()),
    }
}

fn format_signature_time(time: git2::Time) -> String {
    let timestamp = time.seconds();
    let offset_seconds = time.offset_minutes() * 60;
    let Some(offset) = chrono::FixedOffset::east_opt(offset_seconds) else {
        return "unknown time".to_string();
    };
    let Some(datetime) = chrono::DateTime::from_timestamp(timestamp, 0) else {
        return "unknown time".to_string();
    };

    datetime
        .with_timezone(&offset)
        .format("%Y-%m-%d %H:%M:%S %:z")
        .to_string()
}

fn load_commit_file_diffs(
    repository: &Repository,
    commit: &git2::Commit<'_>,
) -> Result<Vec<GitCommitFileDiff>, String> {
    let commit_tree = commit
        .tree()
        .map_err(|error| format!("コミットツリーを取得できませんでした: {}", error.message()))?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(
            commit
                .parent(0)
                .and_then(|parent| parent.tree())
                .map_err(|error| {
                    format!(
                        "親コミットのツリーを取得できませんでした: {}",
                        error.message()
                    )
                })?,
        )
    } else {
        None
    };

    let mut diff_options = DiffOptions::new();
    diff_options
        .context_lines(3)
        .interhunk_lines(1)
        .include_untracked(true)
        .recurse_untracked_dirs(true);

    let diff = repository
        .diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&commit_tree),
            Some(&mut diff_options),
        )
        .map_err(|error| format!("コミット差分を読み込めませんでした: {}", error.message()))?;

    let mut files = diff
        .deltas()
        .map(|delta| GitCommitFileDiff {
            path: diff_delta_path(&delta),
            status: diff_delta_status(&delta).to_string(),
            patch: String::new(),
        })
        .collect::<Vec<_>>();

    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        let path = diff_delta_path(&delta);
        if let Some(file) = files.iter_mut().find(|file| file.path == path) {
            let origin = line.origin();
            let content = String::from_utf8_lossy(line.content());

            match origin {
                ' ' | '+' | '-' => append_patch_line(&mut file.patch, Some(origin), &content),
                '=' => append_patch_line(&mut file.patch, Some(' '), &content),
                '>' => {
                    if !is_no_newline_marker(&content) {
                        append_patch_line(&mut file.patch, Some('+'), &content);
                    }
                }
                '<' => {
                    if !is_no_newline_marker(&content) {
                        append_patch_line(&mut file.patch, Some('-'), &content);
                    }
                }
                'H' => append_patch_line(&mut file.patch, None, &content),
                _ => {}
            }
        }
        true
    })
    .map_err(|error| format!("コミットパッチを読み込めませんでした: {}", error.message()))?;

    Ok(files)
}

fn append_patch_line(buffer: &mut String, prefix: Option<char>, content: &str) {
    if let Some(prefix) = prefix {
        buffer.push(prefix);
    }

    buffer.push_str(content);

    if !content.ends_with('\n') {
        buffer.push('\n');
    }
}

fn is_no_newline_marker(content: &str) -> bool {
    content.trim() == "\\ No newline at end of file"
}

fn diff_delta_path(delta: &git2::DiffDelta<'_>) -> String {
    delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(unknown file)".to_string())
}

fn diff_delta_status(delta: &git2::DiffDelta<'_>) -> char {
    match delta.status() {
        git2::Delta::Added => 'A',
        git2::Delta::Deleted => 'D',
        git2::Delta::Modified => 'M',
        git2::Delta::Renamed => 'R',
        git2::Delta::Copied => 'C',
        git2::Delta::Typechange => 'T',
        _ => 'M',
    }
}

fn load_reference_labels(
    repository: &Repository,
    current_branch_name: Option<&str>,
) -> Result<HashMap<String, Vec<GitRefLabel>>, String> {
    let mut labels_by_oid = HashMap::new();

    append_reference_labels(
        repository,
        "refs/heads/*",
        "refs/heads/",
        "local",
        current_branch_name,
        &mut labels_by_oid,
    )?;
    append_reference_labels(
        repository,
        "refs/remotes/origin/*",
        "refs/remotes/",
        "remote",
        None,
        &mut labels_by_oid,
    )?;
    append_reference_labels(
        repository,
        "refs/tags/*",
        "refs/tags/",
        "tag",
        None,
        &mut labels_by_oid,
    )?;

    for labels in labels_by_oid.values_mut() {
        labels.sort_by(|left, right| {
            right
                .is_current
                .cmp(&left.is_current)
                .then_with(|| left.scope.cmp(&right.scope))
                .then_with(|| left.name.cmp(&right.name))
        });
    }

    Ok(labels_by_oid)
}

fn append_reference_labels(
    repository: &Repository,
    pattern: &str,
    prefix: &str,
    scope: &str,
    current_branch_name: Option<&str>,
    labels_by_oid: &mut HashMap<String, Vec<GitRefLabel>>,
) -> Result<(), String> {
    let references = match repository.references_glob(pattern) {
        Ok(references) => references,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "参照 {} を読み込めませんでした: {}",
                pattern,
                error.message()
            ))
        }
    };

    for reference_result in references {
        let reference = reference_result.map_err(|error| {
            format!(
                "参照 {} の読み込みに失敗しました: {}",
                pattern,
                error.message()
            )
        })?;
        let Ok(commit) = reference.peel_to_commit() else {
            continue;
        };
        let Some(name) = reference.name() else {
            continue;
        };
        if scope == "remote" && name.ends_with("/HEAD") {
            continue;
        }

        let display_name = name.strip_prefix(prefix).unwrap_or(name).to_string();
        let is_current = scope == "local" && current_branch_name == Some(display_name.as_str());

        labels_by_oid
            .entry(commit.id().to_string())
            .or_default()
            .push(GitRefLabel {
                name: display_name,
                scope: scope.to_string(),
                is_current,
            });
    }

    Ok(())
}

fn push_history_refs(
    repository: &Repository,
    revwalk: &mut git2::Revwalk<'_>,
) -> Result<(), String> {
    let mut pushed_any = false;

    for pattern in ["refs/heads/*", "refs/remotes/origin/*", "refs/tags/*"] {
        match revwalk.push_glob(pattern) {
            Ok(()) => pushed_any = true,
            Err(error) if error.code() == git2::ErrorCode::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "履歴参照 {} を追加できませんでした: {}",
                    pattern,
                    error.message()
                ))
            }
        }
    }

    if !pushed_any {
        let references = match repository.references_glob("refs/tags/*") {
            Ok(references) => Some(references),
            Err(error) if error.code() == git2::ErrorCode::NotFound => None,
            Err(error) => {
                return Err(format!(
                    "タグ参照を読み込めませんでした: {}",
                    error.message()
                ))
            }
        };

        if let Some(references) = references {
            for reference_result in references {
                let reference = reference_result.map_err(|error| {
                    format!("タグ参照の読み込みに失敗しました: {}", error.message())
                })?;
                let Ok(commit) = reference.peel_to_commit() else {
                    continue;
                };
                revwalk.push(commit.id()).map_err(|error| {
                    format!(
                        "タグ {} を履歴起点に追加できませんでした: {}",
                        commit.id(),
                        error.message()
                    )
                })?;
                pushed_any = true;
            }
        }
    }

    if !pushed_any {
        revwalk
            .push_head()
            .map_err(|error| format!("HEAD を起点に履歴を辿れませんでした: {}", error.message()))?;
    }

    Ok(())
}

fn load_local_branch_syncs(repository: &Repository) -> Result<Vec<GitLocalBranchSync>, String> {
    let branches = repository
        .branches(Some(git2::BranchType::Local))
        .map_err(|error| format!("ブランチ一覧を取得できませんでした: {}", error.message()))?;

    let mut syncs = Vec::new();

    for branch_result in branches {
        let (branch, _) = branch_result
            .map_err(|error| format!("ブランチ情報を読み込めませんでした: {}", error.message()))?;
        if let Some(name) = branch
            .name()
            .map_err(|error| format!("ブランチ名を取得できませんでした: {}", error.message()))?
        {
            let (ahead_count, behind_count) =
                load_branch_upstream_sync_counts(repository, &branch)?;
            syncs.push(GitLocalBranchSync {
                name: name.to_string(),
                ahead_count,
                behind_count,
            });
        }
    }

    syncs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(syncs)
}

fn load_remote_groups(repository: &Repository) -> Result<Vec<GitRemoteGroup>, String> {
    let branches = repository
        .branches(Some(git2::BranchType::Remote))
        .map_err(|error| {
            format!(
                "リモートブランチ一覧を取得できませんでした: {}",
                error.message()
            )
        })?;

    let mut grouped = BTreeMap::<String, Vec<String>>::new();

    for branch_result in branches {
        let (branch, _) = branch_result.map_err(|error| {
            format!(
                "リモートブランチ情報を読み込めませんでした: {}",
                error.message()
            )
        })?;
        let Some(name) = branch.name().map_err(|error| {
            format!(
                "リモートブランチ名を取得できませんでした: {}",
                error.message()
            )
        })?
        else {
            continue;
        };

        if name.ends_with("/HEAD") {
            continue;
        }

        let Some((remote_name, branch_name)) = name.split_once('/') else {
            continue;
        };

        grouped
            .entry(remote_name.to_string())
            .or_default()
            .push(branch_name.to_string());
    }

    Ok(grouped
        .into_iter()
        .map(|(name, mut branches)| {
            branches.sort();
            branches.dedup();
            GitRemoteGroup { name, branches }
        })
        .collect())
}

fn load_tags(repository: &Repository) -> Result<Vec<String>, String> {
    let tag_names = repository
        .tag_names(None)
        .map_err(|error| format!("タグ一覧を取得できませんでした: {}", error.message()))?;

    let mut tags = tag_names
        .iter()
        .flatten()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    tags.sort();
    Ok(tags)
}

fn load_stashes(repository: &mut Repository) -> Result<Vec<GitStashEntry>, String> {
    let mut stashes = Vec::new();

    match repository.stash_foreach(|index, message, _oid| {
        let (name, detail) = parse_stash_display(message, index);
        stashes.push(GitStashEntry {
            index,
            name,
            message: detail,
        });
        true
    }) {
        Ok(()) => {}
        Err(error) if error.code() == git2::ErrorCode::NotFound => {}
        Err(error) => {
            return Err(format!(
                "stash 一覧を取得できませんでした: {}",
                error.message()
            ))
        }
    }

    Ok(stashes)
}

fn parse_stash_display(message: &str, index: usize) -> (String, String) {
    let trimmed = message.trim();

    if let Some((prefix, title)) = trimmed.rsplit_once(": ") {
        let stash_name = if title.trim().is_empty() {
            format!("stash@{{{index}}}")
        } else {
            title.trim().to_string()
        };

        return (stash_name, prefix.trim().to_string());
    }

    if trimmed.is_empty() {
        (format!("stash@{{{index}}}"), String::new())
    } else {
        (trimmed.to_string(), String::new())
    }
}

fn load_submodules(repository: &Repository) -> Result<Vec<GitSubmoduleEntry>, String> {
    let submodules = repository
        .submodules()
        .map_err(|error| format!("submodule 一覧を取得できませんでした: {}", error.message()))?;

    let mut entries = submodules
        .into_iter()
        .map(|submodule| {
            let path = submodule.path().display().to_string();
            let name = submodule.name().unwrap_or(path.as_str()).to_string();

            GitSubmoduleEntry { name, path }
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn resolve_tag_target_oid(
    repository: &Repository,
    tag_name: &str,
) -> Result<GitReferenceTarget, String> {
    let reference_name = format!("refs/tags/{tag_name}");
    let reference = repository
        .find_reference(&reference_name)
        .map_err(|error| {
            format!(
                "タグ {} を読み込めませんでした: {}",
                tag_name,
                error.message()
            )
        })?;
    let commit = reference.peel_to_commit().map_err(|error| {
        format!(
            "タグ {} のコミットを解決できませんでした: {}",
            tag_name,
            error.message()
        )
    })?;

    Ok(GitReferenceTarget {
        oid: commit.id().to_string(),
    })
}

pub fn run() {
    let state_flags = window_state_flags();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(state_flags)
                .build(),
        )
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let title = format!("Tauri Git v{}", env!("CARGO_PKG_VERSION"));
            window.set_title(&title)?;
            #[cfg(target_os = "linux")]
            if is_wayland_session() {
                window.show()?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_repository,
            get_repository_status,
            commit_all,
            commit_and_push,
            fetch_origin,
            pull_current_branch,
            push_current_branch,
            checkout_branch,
            create_branch,
            delete_branch,
            stash_changes,
            discard_changes,
            apply_stash,
            pop_stash,
            get_worktree_file_diff,
            get_commit_history_chunk,
            get_commit_detail,
            resolve_tag_target
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
