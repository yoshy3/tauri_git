use git2::{Cred, FetchOptions, RemoteCallbacks, Repository, Signature, Status, StatusOptions};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;

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
    has_origin_remote: bool,
    is_clean: bool,
    entries: Vec<GitStatusEntry>,
    head_summary: Option<String>,
    local_branches: Vec<String>,
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

#[tauri::command]
fn open_repository(path: String) -> Result<GitStatusResponse, String> {
    get_repository_status(path)
}

#[tauri::command]
fn get_repository_status(path: String) -> Result<GitStatusResponse, String> {
    let mut repository = open_repo(&path)?;
    build_repository_status(&mut repository)
}

#[tauri::command]
fn commit_all(path: String, message: String) -> Result<GitStatusResponse, String> {
    let mut repository = open_repo(&path)?;
    create_commit(&repository, &message)?;
    build_repository_status(&mut repository)
}

#[tauri::command]
fn fetch_origin(path: String) -> Result<GitStatusResponse, String> {
    let mut repository = open_repo(&path)?;
    fetch_default_remote(&repository)?;
    build_repository_status(&mut repository)
}

#[tauri::command]
fn pull_current_branch(path: String) -> Result<GitStatusResponse, String> {
    let mut repository = open_repo(&path)?;
    pull_current_branch_ff_only(&repository)?;
    build_repository_status(&mut repository)
}

#[tauri::command]
fn push_current_branch(path: String) -> Result<GitStatusResponse, String> {
    let mut repository = open_repo(&path)?;
    push_current_branch_to_origin(&repository)?;
    build_repository_status(&mut repository)
}

#[tauri::command]
fn checkout_branch(
    path: String,
    branch_name: String,
    remote_name: Option<String>,
) -> Result<GitStatusResponse, String> {
    let mut repository = open_repo(&path)?;
    checkout_repository_branch(&repository, &branch_name, remote_name.as_deref())?;
    build_repository_status(&mut repository)
}

#[tauri::command]
fn stash_changes(
    path: String,
    message: Option<String>,
    selected_paths: Vec<String>,
) -> Result<GitStatusResponse, String> {
    let mut repository = open_repo(&path)?;
    create_stash(&mut repository, message.as_deref(), &selected_paths)?;
    build_repository_status(&mut repository)
}

#[tauri::command]
fn apply_stash(path: String, index: usize) -> Result<GitStatusResponse, String> {
    let mut repository = open_repo(&path)?;
    apply_stash_entry(&mut repository, index)?;
    build_repository_status(&mut repository)
}

#[tauri::command]
fn pop_stash(path: String, index: Option<usize>) -> Result<GitStatusResponse, String> {
    let mut repository = open_repo(&path)?;
    pop_stash_entry(&mut repository, index.unwrap_or(0))?;
    build_repository_status(&mut repository)
}

#[tauri::command]
fn get_commit_history_chunk(
    path: String,
    offset: usize,
    limit: usize,
) -> Result<GitCommitHistoryChunk, String> {
    let repository = open_repo(&path)?;
    load_commit_history_chunk(&repository, offset, limit)
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

    let branch = repository
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(ToOwned::to_owned))
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

    let local_branches = load_local_branches(repository)?;
    let remote_groups = load_remote_groups(repository)?;
    let tags = load_tags(repository)?;
    let stashes = load_stashes(repository)?;
    let submodules = load_submodules(repository)?;
    let has_origin_remote = has_remote(repository, "origin")?;

    Ok(GitStatusResponse {
        repo_name,
        repo_path,
        branch,
        has_origin_remote,
        is_clean: entries.is_empty(),
        entries,
        head_summary,
        local_branches,
        remote_groups,
        tags,
        stashes,
        submodules,
    })
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
        .commit(Some("HEAD"), &signature, &signature, message.trim(), &tree, &parents)
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

    let output = Command::new("git")
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

fn push_current_branch_to_origin(repository: &Repository) -> Result<(), String> {
    if !has_remote(repository, "origin")? {
        return Err("origin remote is not configured".to_string());
    }

    let branch = repository
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(ToOwned::to_owned))
        .filter(|name| !name.is_empty() && name != "HEAD")
        .ok_or_else(|| "current branch could not be determined".to_string())?;

    let output = Command::new("git")
        .current_dir(repository_root(repository)?)
        .arg("push")
        .arg("-u")
        .arg("origin")
        .arg(&branch)
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
    let mut command = Command::new("git");
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

    let mut command = Command::new("git");
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
            index
                .read_tree(&tree)
                .map_err(|error| format!("インデックスを HEAD に戻せませんでした: {}", error.message()))?;
        } else {
            index
                .clear()
                .map_err(|error| format!("インデックスを初期化できませんでした: {}", error.message()))?;
        }
    } else {
        index
            .clear()
            .map_err(|error| format!("インデックスを初期化できませんでした: {}", error.message()))?;
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

fn has_remote(repository: &Repository, remote_name: &str) -> Result<bool, String> {
    let remotes = repository
        .remotes()
        .map_err(|error| format!("failed to inspect remotes: {}", error.message()))?;

    Ok(remotes.iter().flatten().any(|name| name == remote_name))
}

fn tree_is_unchanged(
    new_tree: &git2::Tree<'_>,
    parent_commit: Option<&git2::Commit<'_>>,
) -> Result<bool, String> {
    let Some(parent_commit) = parent_commit else {
        return Ok(new_tree.is_empty());
    };

    let parent_tree = parent_commit
        .tree()
        .map_err(|error| format!("親コミットのツリーを取得できませんでした: {}", error.message()))?;

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
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)
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
        let parent_ids = commit.parent_ids().map(|parent_id| parent_id.to_string()).collect();
        let on_current_branch = current_head_oid
            .map(|head_oid| head_oid == oid || repository.graph_descendant_of(head_oid, oid).unwrap_or(false))
            .unwrap_or(false);

        commits.push(GitCommitSummary {
            oid: oid.to_string(),
            id: oid.to_string().chars().take(7).collect(),
            summary: commit.summary().unwrap_or("(no summary)").to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            committed_at,
            parent_ids,
            on_current_branch,
            labels: reference_labels.get(&oid.to_string()).cloned().unwrap_or_default(),
        });
    }

    Ok(GitCommitHistoryChunk { commits, has_more })
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
        let reference = reference_result
            .map_err(|error| format!("参照 {} の読み込みに失敗しました: {}", pattern, error.message()))?;
        let Some(oid) = reference.target() else {
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
            .entry(oid.to_string())
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
    _repository: &Repository,
    revwalk: &mut git2::Revwalk<'_>,
) -> Result<(), String> {
    let mut pushed_any = false;

    for pattern in ["refs/heads/*", "refs/remotes/origin/*"] {
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
        revwalk
            .push_head()
            .map_err(|error| format!("HEAD を起点に履歴を辿れませんでした: {}", error.message()))?;
    }

    Ok(())
}

fn load_local_branches(repository: &Repository) -> Result<Vec<String>, String> {
    let branches = repository
        .branches(Some(git2::BranchType::Local))
        .map_err(|error| format!("ブランチ一覧を取得できませんでした: {}", error.message()))?;

    let mut names = Vec::new();

    for branch_result in branches {
        let (branch, _) =
            branch_result.map_err(|error| format!("ブランチ情報を読み込めませんでした: {}", error.message()))?;
        if let Some(name) = branch
            .name()
            .map_err(|error| format!("ブランチ名を取得できませんでした: {}", error.message()))?
        {
            names.push(name.to_string());
        }
    }

    names.sort();
    Ok(names)
}

fn load_remote_groups(repository: &Repository) -> Result<Vec<GitRemoteGroup>, String> {
    let branches = repository
        .branches(Some(git2::BranchType::Remote))
        .map_err(|error| format!("リモートブランチ一覧を取得できませんでした: {}", error.message()))?;

    let mut grouped = BTreeMap::<String, Vec<String>>::new();

    for branch_result in branches {
        let (branch, _) = branch_result
            .map_err(|error| format!("リモートブランチ情報を読み込めませんでした: {}", error.message()))?;
        let Some(name) = branch
            .name()
            .map_err(|error| format!("リモートブランチ名を取得できませんでした: {}", error.message()))?
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

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_repository,
            get_repository_status,
            commit_all,
            fetch_origin,
            pull_current_branch,
            push_current_branch,
            checkout_branch,
            stash_changes,
            apply_stash,
            pop_stash,
            get_commit_history_chunk
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
