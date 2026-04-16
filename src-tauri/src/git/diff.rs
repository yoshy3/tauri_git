use super::*;

pub(super) fn git_command() -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new("git");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}


pub(super) fn run_git_command(mut command: Command, action_name: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("{action_name} コマンドを実行できませんでした: {}", error))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "詳細なエラーを取得できませんでした。".to_string()
    };

    Err(format!("{action_name} に失敗しました: {detail}"))
}


fn run_git_diff_command(mut command: Command, action_name: &str) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("{action_name} コマンドを実行できませんでした: {}", error))?;

    match output.status.code() {
        Some(0) | Some(1) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr.is_empty() {
                stderr
            } else if !stdout.is_empty() {
                stdout
            } else {
                "詳細なエラーを取得できませんでした。".to_string()
            };

            Err(format!("{action_name} に失敗しました: {detail}"))
        }
    }
}


fn run_worktree_diff_variant(
    repo_root: &Path,
    file_path: &str,
    include_head: bool,
    extra_args: &[&str],
    action_name: &str,
) -> Result<String, String> {
    run_git_diff_command(
        {
            let mut command = git_command();
            command.current_dir(repo_root).arg("diff");
            for arg in extra_args {
                command.arg(arg);
            }
            command.arg("--no-ext-diff").arg("--no-color");
            if include_head {
                command.arg("HEAD").arg("--").arg(file_path);
            } else {
                command.arg("--no-index").arg("--").arg("/dev/null").arg(file_path);
            }
            command
        },
        action_name,
    )
}


fn run_worktree_diff_with_fallbacks(
    repo_root: &Path,
    file_path: &str,
    include_head: bool,
    action_name: &str,
) -> Result<String, String> {
    let variants: &[&[&str]] = if include_head {
        &[&[], &["--textconv"], &["--text"]]
    } else {
        &[&[], &["--text"]]
    };

    let mut last_patch = String::new();

    for extra_args in variants {
        let patch =
            run_worktree_diff_variant(repo_root, file_path, include_head, extra_args, action_name)?;
        if !patch.trim().is_empty() {
            return Ok(patch);
        }
        last_patch = patch;
    }

    Ok(last_patch)
}


fn split_command_line(command_line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in command_line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}


fn run_filter_command(
    repo_root: &Path,
    command_line: &str,
    input: &[u8],
    action_name: &str,
) -> Result<Vec<u8>, String> {
    let parts = split_command_line(command_line);
    let executable = parts
        .first()
        .ok_or_else(|| format!("{action_name} command is empty"))?;

    let mut command = Command::new(executable);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .current_dir(repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    for arg in parts.iter().skip(1) {
        command.arg(arg);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("{action_name} process could not be started: {error}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(input)
            .map_err(|error| format!("{action_name} input could not be written: {error}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|error| format!("{action_name} result could not be read: {error}"))?;

    if output.status.success() {
        return Ok(output.stdout);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "unknown error".to_string()
    };

    Err(format!("{action_name} failed: {detail}"))
}


fn maybe_build_git_crypt_diff(
    repository: &Repository,
    repo_root: &Path,
    file_path: &str,
    include_head: bool,
) -> Result<Option<String>, String> {
    let config = repository
        .config()
        .map_err(|error| format!("failed to inspect git config: {}", error.message()))?;
    let smudge_command = match config.get_string("filter.git-crypt.smudge") {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let worktree_path = repo_root.join(file_path);
    let new_bytes = fs::read(&worktree_path)
        .map_err(|error| format!("failed to read worktree file {}: {}", worktree_path.display(), error))?;

    let old_bytes = if include_head {
        let head = match repository.head() {
            Ok(head) => head,
            Err(_) => return Ok(None),
        };
        let tree = match head.peel_to_tree() {
            Ok(tree) => tree,
            Err(_) => return Ok(None),
        };
        let entry = match tree.get_path(Path::new(file_path)) {
            Ok(entry) => entry,
            Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "failed to resolve HEAD entry for {}: {}",
                    file_path,
                    error.message()
                ))
            }
        };
        let blob = repository
            .find_blob(entry.id())
            .map_err(|error| format!("failed to read HEAD blob for {}: {}", file_path, error.message()))?;
        Some(
            run_filter_command(repo_root, &smudge_command, blob.content(), "git-crypt smudge")?,
        )
    } else {
        None
    };

    let old_text = String::from_utf8_lossy(old_bytes.as_deref().unwrap_or_default()).to_string();
    let new_text = String::from_utf8_lossy(&new_bytes).to_string();
    if old_text == new_text {
        return Ok(None);
    }

    Ok(Some(build_full_file_unified_diff(
        file_path, &old_text, &new_text,
    )))
}


fn load_tree_entry_plaintext(
    repository: &Repository,
    tree: Option<&git2::Tree<'_>>,
    file_path: &str,
    smudge_command: &str,
    repo_root: &Path,
) -> Result<Option<String>, String> {
    let Some(tree) = tree else {
        return Ok(None);
    };

    let entry = match tree.get_path(Path::new(file_path)) {
        Ok(entry) => entry,
        Err(error) if error.code() == git2::ErrorCode::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to resolve tree entry for {}: {}",
                file_path,
                error.message()
            ))
        }
    };

    let blob = repository
        .find_blob(entry.id())
        .map_err(|error| format!("failed to read blob for {}: {}", file_path, error.message()))?;
    let plaintext = run_filter_command(repo_root, smudge_command, blob.content(), "git-crypt smudge")?;
    Ok(Some(String::from_utf8_lossy(&plaintext).to_string()))
}


pub(super) fn maybe_fill_git_crypt_commit_patches(
    repository: &Repository,
    parent_tree: Option<&git2::Tree<'_>>,
    commit_tree: &git2::Tree<'_>,
    files: &mut [GitCommitFileDiff],
) -> Result<(), String> {
    if !files.iter().any(|file| file.patch.trim().is_empty()) {
        return Ok(());
    }

    let config = repository
        .config()
        .map_err(|error| format!("failed to inspect git config: {}", error.message()))?;
    let smudge_command = match config.get_string("filter.git-crypt.smudge") {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };
    let repo_root = repository_root(repository)?;

    for file in files.iter_mut().filter(|file| file.patch.trim().is_empty()) {
        let old_text = load_tree_entry_plaintext(
            repository,
            parent_tree,
            &file.path,
            &smudge_command,
            &repo_root,
        )?;
        let new_text = load_tree_entry_plaintext(
            repository,
            Some(commit_tree),
            &file.path,
            &smudge_command,
            &repo_root,
        )?;

        let old_text = old_text.unwrap_or_default();
        let new_text = new_text.unwrap_or_default();
        if old_text == new_text {
            continue;
        }

        file.patch = build_full_file_unified_diff(&file.path, &old_text, &new_text);
    }

    Ok(())
}


fn text_to_diff_lines(text: &str) -> Vec<String> {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .split_terminator('\n')
        .map(|line| line.to_string())
        .collect()
}


fn build_full_file_unified_diff(file_path: &str, old_text: &str, new_text: &str) -> String {
    let old_lines = text_to_diff_lines(old_text);
    let new_lines = text_to_diff_lines(new_text);
    let old_count = old_lines.len();
    let new_count = new_lines.len();
    let old_start = if old_count == 0 { 0 } else { 1 };
    let new_start = if new_count == 0 { 0 } else { 1 };

    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{file_path} b/{file_path}\n"));
    patch.push_str(&format!("--- a/{file_path}\n"));
    patch.push_str(&format!("+++ b/{file_path}\n"));
    patch.push_str(&format!(
        "@@ -{old_start},{old_count} +{new_start},{new_count} @@\n"
    ));

    for line in old_lines {
        patch.push('-');
        patch.push_str(&line);
        patch.push('\n');
    }

    for line in new_lines {
        patch.push('+');
        patch.push_str(&line);
        patch.push('\n');
    }

    patch
}


pub(crate) fn load_worktree_file_diff(
    repository: &Repository,
    file_path: &str,
) -> Result<GitWorktreeFileDiff, String> {
    let repo_root = repository_root(repository)?;
    let include_head = repository.find_path_in_head(file_path)?;
    let mut patch = if include_head {
        run_worktree_diff_with_fallbacks(&repo_root, file_path, true, "worktree diff")?
    } else {
        run_worktree_diff_with_fallbacks(&repo_root, file_path, false, "new file diff")?
    };

    if patch.trim().is_empty() {
        if let Some(git_crypt_patch) =
            maybe_build_git_crypt_diff(repository, &repo_root, file_path, include_head)?
        {
            patch = git_crypt_patch;
        }
    }

    Ok(GitWorktreeFileDiff {
        path: file_path.to_string(),
        patch,
    })
}

pub(super) trait RepositoryHeadExt {
    fn find_path_in_head(&self, path: &str) -> Result<bool, String>;
}

impl RepositoryHeadExt for Repository {
    fn find_path_in_head(&self, path: &str) -> Result<bool, String> {
        let head = match self.head() {
            Ok(head) => head,
            Err(_) => return Ok(false),
        };
        let tree = match head.peel_to_tree() {
            Ok(tree) => tree,
            Err(_) => return Ok(false),
        };

        match tree.get_path(std::path::Path::new(path)) {
            Ok(_) => Ok(true),
            Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(false),
            Err(error) => Err(format!(
                "HEAD tree lookup failed for {}: {}",
                path,
                error.message()
            )),
        }
    }
}


pub(super) fn append_patch_line(buffer: &mut String, prefix: Option<char>, content: &str) {
    if let Some(prefix) = prefix {
        buffer.push(prefix);
    }

    buffer.push_str(content);

    if !content.ends_with('\n') {
        buffer.push('\n');
    }
}


pub(super) fn is_no_newline_marker(content: &str) -> bool {
    content.trim() == "\\ No newline at end of file"
}


pub(super) fn diff_delta_path(delta: &git2::DiffDelta<'_>) -> String {
    delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(unknown file)".to_string())
}


pub(super) fn diff_delta_status(delta: &git2::DiffDelta<'_>) -> char {
    match delta.status() {
        git2::Delta::Added => 'A',
        git2::Delta::Deleted => 'D',
        git2::Delta::Modified => 'M',
        git2::Delta::Renamed => 'R',
        git2::Delta::Copied => 'C',
        git2::Delta::Typechange => 'T',
        _ => 'M',
    }
}


