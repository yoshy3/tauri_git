use super::*;

pub(crate) fn load_commit_history_chunk(
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


pub(crate) fn load_commit_detail(repository: &Repository, oid: &str) -> Result<GitCommitDetail, String> {
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

    maybe_fill_git_crypt_commit_patches(
        repository,
        parent_tree.as_ref(),
        &commit_tree,
        &mut files,
    )?;

    Ok(files)
}


