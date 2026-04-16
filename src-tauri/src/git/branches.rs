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
            format!(
                "upstream との差分を取得できませんでした: {}",
                error.message()
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
            format!(
                "upstream branch name could not be read: {}",
                error.message()
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


pub(crate) fn delete_repository_branch(
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


