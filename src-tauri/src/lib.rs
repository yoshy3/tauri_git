use git2::{Repository, Signature, Status, StatusOptions};
use serde::Serialize;
use std::path::PathBuf;

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
    is_clean: bool,
    entries: Vec<GitStatusEntry>,
    head_summary: Option<String>,
    local_branches: Vec<String>,
}

#[derive(Serialize)]
struct GitCommitSummary {
    oid: String,
    id: String,
    summary: String,
    author: String,
    committed_at: String,
    parent_ids: Vec<String>,
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
    let repository = open_repo(&path)?;
    build_repository_status(&repository)
}

#[tauri::command]
fn commit_all(path: String, message: String) -> Result<GitStatusResponse, String> {
    let repository = open_repo(&path)?;
    create_commit(&repository, &message)?;
    build_repository_status(&repository)
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

fn build_repository_status(repository: &Repository) -> Result<GitStatusResponse, String> {
    let repo_root = repository
        .workdir()
        .or_else(|| repository.path().parent())
        .ok_or_else(|| "リポジトリのルートパスを解決できませんでした。".to_string())?;
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository")
        .to_string();

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

    Ok(GitStatusResponse {
        repo_name,
        repo_path: repo_root.display().to_string(),
        branch,
        is_clean: entries.is_empty(),
        entries,
        head_summary,
        local_branches,
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
        '?'
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
    let mut revwalk = repository
        .revwalk()
        .map_err(|error| format!("コミット履歴を読み込めませんでした: {}", error.message()))?;

    revwalk
        .push_head()
        .map_err(|error| format!("HEAD を起点に履歴を辿れませんでした: {}", error.message()))?;
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

        commits.push(GitCommitSummary {
            oid: oid.to_string(),
            id: oid.to_string().chars().take(7).collect(),
            summary: commit.summary().unwrap_or("(no summary)").to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            committed_at,
            parent_ids,
        });
    }

    Ok(GitCommitHistoryChunk { commits, has_more })
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

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_repository,
            get_repository_status,
            commit_all,
            get_commit_history_chunk
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
