use crate::models::{
    GitCommitDetail, GitCommitFileDiff, GitCommitHistoryChunk, GitCommitParent,
    GitCommitPerson, GitCommitSummary, GitLocalBranchSync, GitRefLabel, GitReferenceTarget,
    GitRemoteGroup, GitStashEntry, GitStatusEntry, GitStatusResponse, GitSubmoduleEntry,
    GitWorktreeFileDiff,
};
use git2::{
    Cred, DiffFormat, DiffOptions, FetchOptions, Oid, RemoteCallbacks, Repository, Signature,
    Status, StatusOptions,
};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::fs;
use std::io::Write;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod branches;
mod diff;
mod history;
mod refs;
mod remotes;
mod repository;
mod status;
mod worktree;

pub(crate) use branches::{
    checkout_repository_branch, create_branch_from_source, delete_repository_branch,
    rebase_current_branch_onto_reference, reset_current_branch_to_commit, revert_commit,
};
use branches::{
    current_local_branch_name, load_branch_upstream_name, load_branch_upstream_sync_counts,
};
pub(crate) use diff::load_worktree_file_diff;
use diff::{
    append_patch_line, diff_delta_path, diff_delta_status, is_no_newline_marker,
    git_command, maybe_fill_git_crypt_commit_patches, run_git_command, RepositoryHeadExt,
};
pub(crate) use history::{load_commit_detail, load_commit_history_chunk};
use refs::{load_reference_labels, push_history_refs};
pub(crate) use refs::{create_repository_tag, delete_repository_tag, resolve_tag_target_oid};
pub(crate) use remotes::{
    fetch_default_remote, pull_current_branch_ff_only, push_current_branch_to_target, push_tag_to_origin,
};
use remotes::has_remote;
pub(crate) use repository::open_repo;
use repository::{repository_root, tree_is_unchanged};
pub(crate) use status::build_repository_status;
pub(crate) use worktree::{
    apply_stash_entry, create_commit, create_stash, discard_selected_changes, pop_stash_entry,
};
use worktree::load_stashes;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

struct PushTarget {
    remote_name: String,
    remote_branch_name: String,
    set_upstream: bool,
}

pub(crate) fn bilingual(ja: impl Into<String>, en: impl Into<String>) -> String {
    format!("{} / {}", ja.into(), en.into())
}

pub(crate) fn bilingual_with_detail(
    ja: impl Into<String>,
    en: impl Into<String>,
    detail: impl Display,
) -> String {
    format!("{} / {}: {}", ja.into(), en.into(), detail)
}

fn command_output_detail(stderr: &str, stdout: &str) -> String {
    if !stderr.is_empty() {
        stderr.to_string()
    } else if !stdout.is_empty() {
        stdout.to_string()
    } else {
        bilingual(
            "詳細なエラー情報はありません。",
            "No detailed error output was available.",
        )
    }
}
