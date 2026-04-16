use super::*;

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
        .ok_or_else(|| {
            bilingual(
                "現在のブランチを特定できませんでした。",
                "The current branch could not be determined.",
            )
        })?;
    let branch = repository
        .find_branch(&local_branch_name, git2::BranchType::Local)
        .map_err(|error| {
            bilingual_with_detail(
                "現在のブランチ情報を読み込めませんでした",
                "Failed to load the current branch",
                error.message(),
            )
        })?;

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
            bilingual_with_detail(
                "upstream ブランチ名を取得できませんでした",
                "Failed to read the upstream branch name",
                error.message(),
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

        return Err(bilingual(
            "upstream ブランチを解決できませんでした。",
            "The upstream branch could not be resolved.",
        ));
    }

    if !create_upstream_if_missing {
        return Err(bilingual(
            "現在のブランチに upstream ブランチがありません。",
            "The current branch does not have an upstream branch.",
        ));
    }

    if !has_remote(repository, "origin")? {
        return Err(bilingual(
            "origin リモートが設定されていません。",
            "The origin remote is not configured.",
        ));
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


pub(crate) fn fetch_default_remote(repository: &Repository) -> Result<(), String> {
    let mut remote = repository
        .find_remote("origin")
        .map_err(|error| {
            bilingual_with_detail(
                "origin リモートを開けませんでした",
                "Failed to open the origin remote",
                error.message(),
            )
        })?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|url, username_from_url, _allowed_types| {
        let config = repository.config()?;

        Cred::credential_helper(&config, url, username_from_url).or_else(|_| {
            username_from_url
                .map(Cred::ssh_key_from_agent)
                .unwrap_or_else(|| {
                    Err(git2::Error::from_str(&bilingual(
                        "利用できる認証情報がありません。",
                        "No usable credentials were available.",
                    )))
                })
        })
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    remote
        .fetch(&[] as &[&str], Some(&mut fetch_options), None)
        .map_err(|error| {
            bilingual_with_detail(
                "Fetch に失敗しました",
                "Fetch failed",
                error.message(),
            )
        })?;

    Ok(())
}


pub(crate) fn pull_current_branch_ff_only(repository: &Repository) -> Result<(), String> {
    let repo_root = repository_root(repository)?;

    let output = git_command()
        .current_dir(repo_root)
        .arg("pull")
        .arg("--ff-only")
        .output()
        .map_err(|error| {
            bilingual_with_detail(
                "git pull を実行できませんでした",
                "Failed to run git pull",
                error,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = command_output_detail(&stderr, &stdout);
        return Err(bilingual_with_detail(
            "git pull に失敗しました",
            "git pull failed",
            detail,
        ));
    }

    Ok(())
}


pub(crate) fn push_current_branch_to_target(
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
        .map_err(|error| {
            bilingual_with_detail(
                "git push を実行できませんでした",
                "Failed to run git push",
                error,
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = command_output_detail(&stderr, &stdout);
        return Err(bilingual_with_detail(
            "git push に失敗しました",
            "git push failed",
            detail,
        ));
    }

    Ok(())
}


pub(super) fn has_remote(repository: &Repository, remote_name: &str) -> Result<bool, String> {
    let remotes = repository
        .remotes()
        .map_err(|error| {
            bilingual_with_detail(
                "リモート設定を確認できませんでした",
                "Failed to inspect remotes",
                error.message(),
            )
        })?;

    Ok(remotes.iter().flatten().any(|name| name == remote_name))
}


