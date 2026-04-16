use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct GitStatusEntry {
    pub(crate) path: String,
    pub(crate) working_tree_status: String,
    pub(crate) index_status: String,
}

#[derive(Serialize)]
pub(crate) struct GitStatusResponse {
    pub(crate) repo_name: String,
    pub(crate) repo_path: String,
    pub(crate) branch: String,
    pub(crate) head_oid: Option<String>,
    pub(crate) history_revision: String,
    pub(crate) has_origin_remote: bool,
    pub(crate) can_push_current_branch: bool,
    pub(crate) current_branch_upstream_name: Option<String>,
    pub(crate) ahead_count: usize,
    pub(crate) behind_count: usize,
    pub(crate) is_clean: bool,
    pub(crate) entries: Vec<GitStatusEntry>,
    pub(crate) head_summary: Option<String>,
    pub(crate) local_branches: Vec<String>,
    pub(crate) local_branch_syncs: Vec<GitLocalBranchSync>,
    pub(crate) remote_groups: Vec<GitRemoteGroup>,
    pub(crate) tags: Vec<String>,
    pub(crate) stashes: Vec<GitStashEntry>,
    pub(crate) submodules: Vec<GitSubmoduleEntry>,
}

#[derive(Serialize)]
pub(crate) struct GitRemoteGroup {
    pub(crate) name: String,
    pub(crate) branches: Vec<String>,
}

#[derive(Serialize)]
pub(crate) struct GitLocalBranchSync {
    pub(crate) name: String,
    pub(crate) ahead_count: usize,
    pub(crate) behind_count: usize,
}

#[derive(Serialize)]
pub(crate) struct GitStashEntry {
    pub(crate) index: usize,
    pub(crate) name: String,
    pub(crate) message: String,
}

#[derive(Serialize)]
pub(crate) struct GitSubmoduleEntry {
    pub(crate) name: String,
    pub(crate) path: String,
}

#[derive(Serialize)]
pub(crate) struct GitCommitSummary {
    pub(crate) oid: String,
    pub(crate) id: String,
    pub(crate) summary: String,
    pub(crate) author: String,
    pub(crate) committed_at: String,
    pub(crate) parent_ids: Vec<String>,
    pub(crate) on_current_branch: bool,
    pub(crate) labels: Vec<GitRefLabel>,
}

#[derive(Clone, Serialize)]
pub(crate) struct GitRefLabel {
    pub(crate) name: String,
    pub(crate) scope: String,
    pub(crate) is_current: bool,
}

#[derive(Serialize)]
pub(crate) struct GitCommitHistoryChunk {
    pub(crate) commits: Vec<GitCommitSummary>,
    pub(crate) has_more: bool,
}

#[derive(Serialize)]
pub(crate) struct GitCommitDetail {
    pub(crate) oid: String,
    pub(crate) id: String,
    pub(crate) summary: String,
    pub(crate) message: String,
    pub(crate) author: GitCommitPerson,
    pub(crate) committer: GitCommitPerson,
    pub(crate) parents: Vec<GitCommitParent>,
    pub(crate) labels: Vec<GitRefLabel>,
    pub(crate) files: Vec<GitCommitFileDiff>,
}

#[derive(Serialize)]
pub(crate) struct GitCommitPerson {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) committed_at: String,
}

#[derive(Serialize)]
pub(crate) struct GitCommitParent {
    pub(crate) oid: String,
    pub(crate) id: String,
}

#[derive(Serialize)]
pub(crate) struct GitCommitFileDiff {
    pub(crate) path: String,
    pub(crate) status: String,
    pub(crate) patch: String,
}

#[derive(Serialize)]
pub(crate) struct GitReferenceTarget {
    pub(crate) oid: String,
}

#[derive(Serialize)]
pub(crate) struct GitWorktreeFileDiff {
    pub(crate) path: String,
    pub(crate) patch: String,
}
