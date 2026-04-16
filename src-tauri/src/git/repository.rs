use super::*;

pub(crate) fn open_repo(path: &str) -> Result<Repository, String> {
    let normalized = PathBuf::from(path);
    Repository::discover(&normalized).map_err(|error| {
        bilingual_with_detail(
            format!("Git リポジトリを開けませんでした ({})", normalized.display()),
            format!("Failed to open Git repository ({})", normalized.display()),
            error.message(),
        )
    })
}


pub(super) fn repository_root(repository: &Repository) -> Result<PathBuf, String> {
    repository
        .workdir()
        .or_else(|| repository.path().parent())
        .ok_or_else(|| {
            bilingual(
                "リポジトリのルートパスを解決できませんでした。",
                "Failed to resolve the repository root path.",
            )
        })
        .map(|path| path.to_path_buf())
}


pub(super) fn tree_is_unchanged(
    new_tree: &git2::Tree<'_>,
    parent_commit: Option<&git2::Commit<'_>>,
) -> Result<bool, String> {
    let Some(parent_commit) = parent_commit else {
        return Ok(new_tree.is_empty());
    };

    let parent_tree = parent_commit.tree().map_err(|error| {
        bilingual_with_detail(
            "親コミットのツリーを取得できませんでした",
            "Failed to read the parent commit tree",
            error.message(),
        )
    })?;

    Ok(parent_tree.id() == new_tree.id())
}


