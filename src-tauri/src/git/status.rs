use super::*;

pub(crate) fn build_repository_status(repository: &mut Repository) -> Result<GitStatusResponse, String> {
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
        .map_err(|error| {
            bilingual_with_detail(
                "ステータスを取得できませんでした",
                "Failed to load repository status",
                error.message(),
            )
        })?;

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
    let head_message = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .and_then(|commit| commit.message().map(ToOwned::to_owned));
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
                .map_err(|error| {
                    bilingual_with_detail(
                        "現在のブランチ情報を読み込めませんでした",
                        "Failed to load the current branch",
                        error.message(),
                    )
                })
                .and_then(|branch| load_branch_upstream_name(&branch))
        })
        .transpose()?
        .flatten();
    let (ahead_count, behind_count) = local_branch_syncs
        .iter()
        .find(|entry| entry.name == branch)
        .map(|entry| (entry.ahead_count, entry.behind_count))
        .unwrap_or((0, 0));
    let head_is_pushed = current_branch_upstream_name.is_some() && ahead_count == 0;
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
        head_message,
        head_is_pushed,
        local_branches,
        local_branch_syncs,
        remote_groups,
        tags,
        stashes,
        submodules,
    })
}


fn build_history_revision(repository: &Repository) -> Result<String, String> {
    let mut entries = Vec::new();

    if let Ok(head) = repository.head() {
        if let Some(target) = head.target() {
            entries.push(format!("HEAD={target}"));
        }
    }

    let references = repository.references().map_err(|error| {
        bilingual_with_detail(
            "履歴リビジョン情報を取得できませんでした",
            "Failed to load history revision information",
            error.message(),
        )
    })?;

    for reference_result in references {
        let reference = reference_result
            .map_err(|error| {
                bilingual_with_detail(
                    "参照情報を読み込めませんでした",
                    "Failed to load reference information",
                    error.message(),
                )
            })?;
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


fn load_local_branch_syncs(repository: &Repository) -> Result<Vec<GitLocalBranchSync>, String> {
    let branches = repository
        .branches(Some(git2::BranchType::Local))
        .map_err(|error| {
            bilingual_with_detail(
                "ブランチ一覧を取得できませんでした",
                "Failed to load local branches",
                error.message(),
            )
        })?;

    let mut syncs = Vec::new();

    for branch_result in branches {
        let (branch, _) = branch_result
            .map_err(|error| {
                bilingual_with_detail(
                    "ブランチ情報を読み込めませんでした",
                    "Failed to read branch information",
                    error.message(),
                )
            })?;
        if let Some(name) = branch
            .name()
            .map_err(|error| {
                bilingual_with_detail(
                    "ブランチ名を取得できませんでした",
                    "Failed to read the branch name",
                    error.message(),
                )
            })?
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
            bilingual_with_detail(
                "リモートブランチ一覧を取得できませんでした",
                "Failed to load remote branches",
                error.message(),
            )
        })?;

    let mut grouped = BTreeMap::<String, Vec<String>>::new();

    for branch_result in branches {
        let (branch, _) = branch_result.map_err(|error| {
            bilingual_with_detail(
                "リモートブランチ情報を読み込めませんでした",
                "Failed to read remote branch information",
                error.message(),
            )
        })?;
        let Some(name) = branch.name().map_err(|error| {
            bilingual_with_detail(
                "リモートブランチ名を取得できませんでした",
                "Failed to read the remote branch name",
                error.message(),
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
        .map_err(|error| {
            bilingual_with_detail(
                "タグ一覧を取得できませんでした",
                "Failed to load tags",
                error.message(),
            )
        })?;

    let mut tags = tag_names
        .iter()
        .flatten()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    tags.sort();
    Ok(tags)
}


fn load_submodules(repository: &Repository) -> Result<Vec<GitSubmoduleEntry>, String> {
    let submodules = repository
        .submodules()
        .map_err(|error| {
            bilingual_with_detail(
                "submodule 一覧を取得できませんでした",
                "Failed to load submodules",
                error.message(),
            )
        })?;

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


