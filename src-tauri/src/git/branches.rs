use super::*;

pub(super) fn load_branch_upstream_sync_counts(
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
            bilingual_with_detail(
                "upstream との差分を取得できませんでした",
                "Failed to read ahead/behind counts for the upstream branch",
                error.message(),
            )
        })
}


pub(super) fn current_local_branch_name(repository: &Repository) -> Option<String> {
    repository
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(ToOwned::to_owned))
        .filter(|name| !name.is_empty() && name != "HEAD")
}


pub(super) fn load_branch_upstream_name(branch: &git2::Branch<'_>) -> Result<Option<String>, String> {
    let upstream = match branch.upstream() {
        Ok(upstream) => upstream,
        Err(_) => return Ok(None),
    };

    upstream
        .name()
        .map(|name| name.map(ToOwned::to_owned))
        .map_err(|error| {
            bilingual_with_detail(
                "upstream ブランチ名を取得できませんでした",
                "Failed to read the upstream branch name",
                error.message(),
            )
        })
}


pub(crate) fn checkout_repository_branch(
    repository: &Repository,
    branch_name: &str,
    remote_name: Option<&str>,
) -> Result<(), String> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        return Err(bilingual("ブランチ名が空です。", "Branch name is empty."));
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
        .map_err(|error| {
            bilingual_with_detail(
                "git checkout を実行できませんでした",
                "Failed to run git checkout",
                error,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = command_output_detail(&stderr, &stdout);
        return Err(bilingual_with_detail(
            "git checkout に失敗しました",
            "git checkout failed",
            detail,
        ));
    }

    Ok(())
}


pub(crate) fn create_branch_from_source(
    repository: &Repository,
    branch_name: &str,
    source_name: &str,
    source_kind: &str,
    source_remote_name: Option<&str>,
    switch_after_create: bool,
) -> Result<(), String> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        return Err(bilingual("ブランチ名が空です。", "Branch name is empty."));
    }

    let source_name = source_name.trim();
    if source_name.is_empty() {
        return Err(bilingual("ブランチ元が空です。", "Branch source is empty."));
    }

    let source_ref = match source_kind {
        "remote_branch" => {
            let remote_name = source_remote_name
                .filter(|name| !name.trim().is_empty())
                .ok_or_else(|| {
                    bilingual(
                        "リモート元の名前が指定されていません。",
                        "The remote source name is missing.",
                    )
                })?;
            format!("{remote_name}/{source_name}")
        }
        "local_branch" | "tag" => source_name.to_string(),
        _ => {
            return Err(bilingual(
                format!("未対応のブランチ元種別です: {source_kind}"),
                format!("Unsupported branch source kind: {source_kind}"),
            ))
        }
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
        .map_err(|error| {
            bilingual_with_detail(
                "ブランチ作成コマンドを実行できませんでした",
                "Failed to run branch creation command",
                error,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = command_output_detail(&stderr, &stdout);
        return Err(bilingual_with_detail(
            "ブランチ作成に失敗しました",
            "Branch creation failed",
            detail,
        ));
    }

    Ok(())
}

pub(crate) fn rebase_current_branch_onto_reference(
    repository: &Repository,
    target_name: &str,
    target_kind: &str,
    target_remote_name: Option<&str>,
) -> Result<(), String> {
    let target_name = target_name.trim();
    if target_name.is_empty() {
        return Err(bilingual("rebase 対象が空です。", "The rebase target is empty."));
    }

    let current_branch = current_local_branch_name(repository).ok_or_else(|| {
        bilingual(
            "現在のローカルブランチを特定できません。",
            "The current local branch could not be determined.",
        )
    })?;

    let target_ref = match target_kind {
        "local_branch" => {
            if current_branch == target_name {
                return Ok(());
            }
            target_name.to_string()
        }
        "remote_branch" => {
            let remote_name = target_remote_name
                .filter(|name| !name.trim().is_empty())
                .ok_or_else(|| {
                    bilingual(
                        "リモート rebase 先にはリモート名が必要です。",
                        "Rebasing onto a remote branch requires a remote name.",
                    )
                })?;
            format!("{remote_name}/{target_name}")
        }
        _ => {
            return Err(bilingual(
                format!("未対応の rebase 対象種別です: {target_kind}"),
                format!("Unsupported rebase target kind: {target_kind}"),
            ))
        }
    };

    let repo_root = repository_root(repository)?;
    let mut command = git_command();
    command
        .current_dir(repo_root)
        .arg("rebase")
        .arg(&target_ref);

    run_git_command(command, "git rebase")
}

pub(crate) fn reset_current_branch_to_commit(
    repository: &Repository,
    target: &str,
    reset_mode: &str,
) -> Result<(), String> {
    let target = target.trim();
    if target.is_empty() {
        return Err(bilingual("reset 対象が空です。", "The reset target is empty."));
    }

    current_local_branch_name(repository).ok_or_else(|| {
        bilingual(
            "現在のローカルブランチを特定できません。",
            "The current local branch could not be determined.",
        )
    })?;

    let mode_flag = match reset_mode.trim() {
        "soft" => "--soft",
        "mixed" => "--mixed",
        "hard" => "--hard",
        other => {
            return Err(bilingual(
                format!("未対応の reset モードです: {other}"),
                format!("Unsupported reset mode: {other}"),
            ))
        }
    };

    let repo_root = repository_root(repository)?;
    let mut command = git_command();
    command
        .current_dir(repo_root)
        .arg("reset")
        .arg(mode_flag)
        .arg(target);

    run_git_command(command, "git reset")
}


pub(crate) fn revert_commit(
    repository: &Repository,
    target: &str,
    message: &str,
) -> Result<(), String> {
    let target = target.trim();
    if target.is_empty() {
        return Err(bilingual("revert 対象が空です。", "The revert target is empty."));
    }

    let message = message.trim();
    if message.is_empty() {
        return Err(bilingual(
            "コミットメッセージが空です。",
            "Commit message is empty.",
        ));
    }

    let repo_root = repository_root(repository)?;

    let mut revert_cmd = git_command();
    revert_cmd
        .current_dir(&repo_root)
        .arg("revert")
        .arg("--no-commit")
        .arg(target);
    run_git_command(revert_cmd, "git revert")?;

    let mut commit_cmd = git_command();
    commit_cmd
        .current_dir(&repo_root)
        .arg("commit")
        .arg("-m")
        .arg(message);
    run_git_command(commit_cmd, "git commit")
}


pub(crate) fn delete_repository_branch(
    repository: &Repository,
    branch_name: &str,
    branch_kind: &str,
    remote_name: Option<&str>,
    force_delete: bool,
) -> Result<(), String> {
    let branch_name = branch_name.trim();
    if branch_name.is_empty() {
        return Err(bilingual("ブランチ名が空です。", "Branch name is empty."));
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
                return Err(bilingual(
                    "現在 checkout 中のブランチは削除できません。",
                    "Cannot delete the currently checked out branch.",
                ));
            }

            command
                .arg("branch")
                .arg(if force_delete { "-D" } else { "-d" })
                .arg(branch_name);
        }
        "remote_branch" => {
            let remote_name = remote_name
                .filter(|name| !name.trim().is_empty())
                .ok_or_else(|| {
                    bilingual(
                        "リモートブランチ削除にはリモート名が必要です。",
                        "Deleting a remote branch requires a remote name.",
                    )
                })?;

            command
                .arg("push")
                .arg(remote_name)
                .arg("--delete")
                .arg(branch_name);
        }
        _ => {
            return Err(bilingual(
                format!("未対応のブランチ削除種別です: {branch_kind}"),
                format!("Unsupported branch delete kind: {branch_kind}"),
            ))
        }
    }

    let output = command
        .output()
        .map_err(|error| {
            bilingual_with_detail(
                "ブランチ削除コマンドを実行できませんでした",
                "Failed to run branch delete command",
                error,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = command_output_detail(&stderr, &stdout);
        return Err(bilingual_with_detail(
            "ブランチ削除に失敗しました",
            "Branch deletion failed",
            detail,
        ));
    }

    Ok(())
}
