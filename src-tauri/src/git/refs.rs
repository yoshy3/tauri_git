use super::*;

pub(super) fn load_reference_labels(
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


pub(super) fn push_history_refs(
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


pub(crate) fn resolve_tag_target_oid(
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



