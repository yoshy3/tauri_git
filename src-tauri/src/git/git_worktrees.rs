use super::*;
use crate::models::GitWorktreeEntry;

pub(crate) fn list_worktrees(repository: &Repository) -> Result<Vec<GitWorktreeEntry>, String> {
    let repo_root = repository_root(repository)?;
    let mut command = git_command();
    command
        .current_dir(&repo_root)
        .arg("worktree")
        .arg("list")
        .arg("--porcelain");

    let output = command.output().map_err(|error| {
        bilingual_with_detail(
            "git worktree list を実行できませんでした",
            "Failed to run git worktree list",
            error,
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = command_output_detail(&stderr, &stdout);
        return Err(bilingual_with_detail(
            "git worktree list に失敗しました",
            "git worktree list failed",
            detail,
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    parse_worktree_list_output(&stdout)
}

fn parse_worktree_list_output(output: &str) -> Result<Vec<GitWorktreeEntry>, String> {
    let mut entries = Vec::new();
    let mut is_first = true;

    for block in output.split("\n\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }

        let mut path: Option<String> = None;
        let mut head_oid: Option<String> = None;
        let mut branch: Option<String> = None;
        let mut is_locked = false;
        let mut is_prunable = false;

        for line in block.lines() {
            if let Some(value) = line.strip_prefix("worktree ") {
                path = Some(value.to_string());
            } else if let Some(value) = line.strip_prefix("HEAD ") {
                head_oid = Some(value.chars().take(7).collect());
            } else if let Some(value) = line.strip_prefix("branch ") {
                let name = value
                    .strip_prefix("refs/heads/")
                    .unwrap_or(value)
                    .to_string();
                branch = Some(name);
            } else if line == "detached" || line == "bare" {
                branch = None;
            } else if line.starts_with("locked") {
                is_locked = true;
            } else if line.starts_with("prunable") {
                is_prunable = true;
            }
        }

        if let Some(path) = path {
            entries.push(GitWorktreeEntry {
                path,
                branch,
                head_oid,
                is_main: is_first,
                is_locked,
                is_prunable,
            });
            is_first = false;
        }
    }

    Ok(entries)
}

pub(crate) fn add_worktree(
    repository: &Repository,
    worktree_path: &str,
    branch: &str,
    create_new_branch: bool,
) -> Result<Vec<GitWorktreeEntry>, String> {
    let worktree_path = worktree_path.trim();
    if worktree_path.is_empty() {
        return Err(bilingual(
            "ワークツリーのパスが空です。",
            "Worktree path is empty.",
        ));
    }

    let branch = branch.trim();
    if branch.is_empty() {
        return Err(bilingual("ブランチ名が空です。", "Branch name is empty."));
    }

    let repo_root = repository_root(repository)?;
    let mut command = git_command();
    command.current_dir(&repo_root).arg("worktree").arg("add");

    if create_new_branch {
        command.arg("-b").arg(branch).arg(worktree_path);
    } else {
        command.arg(worktree_path).arg(branch);
    }

    run_git_command(command, "git worktree add")?;
    list_worktrees(repository)
}

pub(crate) fn remove_worktree(
    repository: &Repository,
    worktree_path: &str,
) -> Result<Vec<GitWorktreeEntry>, String> {
    let worktree_path = worktree_path.trim();
    if worktree_path.is_empty() {
        return Err(bilingual(
            "ワークツリーのパスが空です。",
            "Worktree path is empty.",
        ));
    }

    let repo_root = repository_root(repository)?;
    let mut command = git_command();
    command
        .current_dir(&repo_root)
        .arg("worktree")
        .arg("remove")
        .arg(worktree_path);

    run_git_command(command, "git worktree remove")?;
    list_worktrees(repository)
}

pub(crate) fn prune_worktrees(repository: &Repository) -> Result<Vec<GitWorktreeEntry>, String> {
    let repo_root = repository_root(repository)?;
    let mut command = git_command();
    command
        .current_dir(&repo_root)
        .arg("worktree")
        .arg("prune");

    run_git_command(command, "git worktree prune")?;
    list_worktrees(repository)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_main_worktree_with_branch() {
        let output = "worktree /repo\nHEAD abc1234567890\nbranch refs/heads/main\n\n";
        let entries = parse_worktree_list_output(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/repo");
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert_eq!(entries[0].head_oid.as_deref(), Some("abc1234"));
        assert!(entries[0].is_main);
        assert!(!entries[0].is_locked);
        assert!(!entries[0].is_prunable);
    }

    #[test]
    fn parses_detached_head_worktree() {
        let output = "worktree /repo\nHEAD abc1234567890\nbranch refs/heads/main\n\nworktree /repo/linked\nHEAD def5678901234\ndetached\n\n";
        let entries = parse_worktree_list_output(output).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_main);
        assert!(!entries[1].is_main);
        assert!(entries[1].branch.is_none());
    }

    #[test]
    fn parses_locked_and_prunable_worktrees() {
        let output = "worktree /repo\nHEAD abc1234567890\nbranch refs/heads/main\n\nworktree /repo/locked\nHEAD def5678901234\nbranch refs/heads/feat\nlocked reason\n\nworktree /repo/old\nHEAD ghi9012345678\nbranch refs/heads/old\nprunable gitdir points nowhere\n\n";
        let entries = parse_worktree_list_output(output).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries[1].is_locked);
        assert!(!entries[1].is_prunable);
        assert!(!entries[2].is_locked);
        assert!(entries[2].is_prunable);
    }

    #[test]
    fn strips_refs_heads_prefix_from_branch() {
        let output = "worktree /repo\nHEAD abc1234567890\nbranch refs/heads/feature/my-branch\n\n";
        let entries = parse_worktree_list_output(output).unwrap();
        assert_eq!(entries[0].branch.as_deref(), Some("feature/my-branch"));
    }

    #[test]
    fn returns_empty_for_blank_output() {
        let entries = parse_worktree_list_output("").unwrap();
        assert!(entries.is_empty());
    }
}
