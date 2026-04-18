use super::*;

pub(crate) fn create_commit(repository: &Repository, message: &str, amend: bool) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err(bilingual(
            "コミットメッセージが空です。",
            "Commit message is empty.",
        ));
    }

    let mut index = repository
        .index()
        .map_err(|error| {
            bilingual_with_detail(
                "インデックスを開けませんでした",
                "Failed to open the index",
                error.message(),
            )
        })?;

    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|error| {
            bilingual_with_detail(
                "変更をステージングできませんでした",
                "Failed to stage changes",
                error.message(),
            )
        })?;

    index
        .write()
        .map_err(|error| {
            bilingual_with_detail(
                "インデックスを書き込めませんでした",
                "Failed to write the index",
                error.message(),
            )
        })?;

    let tree_id = index
        .write_tree()
        .map_err(|error| {
            bilingual_with_detail(
                "ツリーを書き込めませんでした",
                "Failed to write the tree",
                error.message(),
            )
        })?;
    let tree = repository
        .find_tree(tree_id)
        .map_err(|error| {
            bilingual_with_detail(
                "ツリーを読み込めませんでした",
                "Failed to load the tree",
                error.message(),
            )
        })?;

    let signature = repository
        .signature()
        .or_else(|_| Signature::now("Tauri Git", "tauri-git@example.local"))
        .map_err(|error| {
            bilingual_with_detail(
                "コミット署名を作れませんでした",
                "Failed to create the commit signature",
                error.message(),
            )
        })?;

    let parent_commit = repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|oid| repository.find_commit(oid).ok());

    if tree_is_unchanged(&tree, parent_commit.as_ref())? {
        return Err(bilingual(
            "コミット対象の変更がありません。",
            "There are no changes to commit.",
        ));
    }

    if amend {
        let parent_commit = parent_commit.ok_or_else(|| {
            bilingual(
                "amend する直前コミットがありません。",
                "There is no previous commit to amend.",
            )
        })?;

        parent_commit
            .amend(
                Some("HEAD"),
                Some(&parent_commit.author()),
                Some(&signature),
                None,
                Some(message.trim()),
                Some(&tree),
            )
            .map_err(|error| {
                bilingual_with_detail(
                    "amend に失敗しました",
                    "Amend failed",
                    error.message(),
                )
            })?;
    } else {
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
            .map_err(|error| {
                bilingual_with_detail(
                    "コミットに失敗しました",
                    "Commit failed",
                    error.message(),
                )
            })?;
    }

    repository
        .checkout_head(None)
        .map_err(|error| {
            bilingual_with_detail(
                "作業ツリーを更新できませんでした",
                "Failed to refresh the working tree",
                error.message(),
            )
        })?;

    Ok(())
}


pub(crate) fn create_stash(
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
        .map_err(|error| {
            bilingual_with_detail(
                "stash 対象を確認できませんでした",
                "Failed to inspect stash candidates",
                error.message(),
            )
        })?;

    if statuses.is_empty() {
        return Err(bilingual(
            "stash する変更がありません。",
            "There are no changes to stash.",
        ));
    }
    drop(statuses);

    if selected_paths.is_empty() {
        return Err(bilingual(
            "stash 対象のファイルを選択してください。",
            "Select at least one file to stash.",
        ));
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
        .map_err(|error| {
            bilingual_with_detail(
                "stash コマンドを実行できませんでした",
                "Failed to run the stash command",
                error,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = command_output_detail(&stderr, &stdout);
        return Err(bilingual_with_detail(
            "stash に失敗しました",
            "Stash failed",
            detail,
        ));
    }

    Ok(())
}


pub(crate) fn discard_selected_changes(
    repository: &Repository,
    selected_paths: &[String],
) -> Result<(), String> {
    if selected_paths.is_empty() {
        return Err(bilingual(
            "discard する変更がありません。",
            "There are no changes to discard.",
        ));
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
        .map_err(|error| {
            bilingual_with_detail(
                "discard 対象を確認できませんでした",
                "Failed to inspect discard candidates",
                error.message(),
            )
        })?;

    if statuses.is_empty() {
        return Err(bilingual(
            "discard できる変更がありません。",
            "There are no changes that can be discarded.",
        ));
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
        return Err(bilingual(
            "discard 対象のファイルを選択してください。",
            "Select at least one file to discard.",
        ));
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


pub(crate) fn apply_stash_entry(repository: &mut Repository, index: usize) -> Result<(), String> {
    match repository.stash_apply(index, None) {
        Ok(()) => {
            reset_index_to_head(repository)?;
            Ok(())
        }
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            Err(bilingual(
                "適用できる stash がありません。",
                "There is no stash entry that can be applied.",
            ))
        }
        Err(error) => Err(bilingual_with_detail(
            "stash apply に失敗しました",
            "stash apply failed",
            error.message(),
        )),
    }
}


pub(crate) fn pop_stash_entry(repository: &mut Repository, index: usize) -> Result<(), String> {
    match repository.stash_pop(index, None) {
        Ok(()) => {
            reset_index_to_head(repository)?;
            Ok(())
        }
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            Err(bilingual(
                "適用できる stash がありません。",
                "There is no stash entry that can be popped.",
            ))
        }
        Err(error) => Err(bilingual_with_detail(
            "stash pop に失敗しました",
            "stash pop failed",
            error.message(),
        )),
    }
}


fn reset_index_to_head(repository: &Repository) -> Result<(), String> {
    let mut index = repository
        .index()
        .map_err(|error| {
            bilingual_with_detail(
                "インデックスを開けませんでした",
                "Failed to open the index",
                error.message(),
            )
        })?;

    if let Ok(head) = repository.head() {
        if let Ok(tree) = head.peel_to_tree() {
            index.read_tree(&tree).map_err(|error| {
                bilingual_with_detail(
                    "インデックスを HEAD に戻せませんでした",
                    "Failed to restore the index to HEAD",
                    error.message(),
                )
            })?;
        } else {
            index.clear().map_err(|error| {
                bilingual_with_detail(
                    "インデックスを初期化できませんでした",
                    "Failed to clear the index",
                    error.message(),
                )
            })?;
        }
    } else {
        index.clear().map_err(|error| {
            bilingual_with_detail(
                "インデックスを初期化できませんでした",
                "Failed to clear the index",
                error.message(),
            )
        })?;
    }

    index
        .write()
        .map_err(|error| {
            bilingual_with_detail(
                "インデックスを書き込めませんでした",
                "Failed to write the index",
                error.message(),
            )
        })?;

    Ok(())
}


pub(super) fn load_stashes(repository: &mut Repository) -> Result<Vec<GitStashEntry>, String> {
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


