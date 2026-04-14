<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { _ } from "svelte-i18n";
  import { get } from "svelte/store";
  import TopBar from "./lib/components/TopBar.svelte";
  import SidebarPane from "./lib/components/SidebarPane.svelte";
  import HistoryPane from "./lib/components/HistoryPane.svelte";
  import CommitPane from "./lib/components/CommitPane.svelte";

  let repository = null;
  let error = "";
  let loading = false;
  let committing = false;
  let commitAndPushing = false;
  let stashing = false;
  let discarding = false;
  let topbarBusyAction = "";
  let stashBusyAction = "";
  let selectedStashIndex = null;
  let selectedRef = null;
  let branchMenuOpenKey = "";
  let rightPaneExpanded = false;
  let rightPaneTab = "commit";
  let historyCommits = [];
  let historyLoading = false;
  let historyLoadedAll = false;
  let historyRequestId = 0;
  let selectedCommitOid = "";
  let selectedCommitScrollToken = 0;
  let pendingRefCommitOid = "";
  let selectedCommitDetail = null;
  let selectedCommitDetailLoading = false;
  let commitDetailRequestId = 0;
  let branchDialogOpen = false;
  let branchNameDraft = "";
  let branchSwitchAfterCreate = true;
  let deleteDialogOpen = false;
  let deleteBranchNameDraft = "";
  let deleteTargetRef = null;
  let deleteForceEnabled = false;
  let discardDialogOpen = false;
  let discardPendingPaths = [];
  let pushDialogOpen = false;
  let pendingPushAction = null;
  let theme = "dark";

  const topActions = ["Fetch", "Pull", "Push", "Stash", "Discard"];
  const implementedTopActions = ["Fetch", "Pull", "Push", "Stash", "Discard"];
  const lastRepositoryKey = "tauri-git:last-repository-path";
  const themeStorageKey = "tauri-git:theme";
  const historyBatchSize = 100;
  const autoRefreshIntervalMs = 2500;
  let autoRefreshInFlight = false;
  let appVisible = true;

  function t(key, values) {
    return get(_)(key, { values });
  }

  function buildRepositoryStatusFingerprint(status) {
    if (!status) {
      return "";
    }

    return JSON.stringify({
      branch: status.branch,
      head_oid: status.head_oid ?? null,
      head_summary: status.head_summary ?? null,
      history_revision: status.history_revision ?? "",
      has_origin_remote: status.has_origin_remote,
      can_push_current_branch: status.can_push_current_branch ?? false,
      current_branch_upstream_name: status.current_branch_upstream_name ?? null,
      ahead_count: status.ahead_count ?? 0,
      behind_count: status.behind_count ?? 0,
      is_clean: status.is_clean,
      entries: status.entries ?? [],
      local_branches: status.local_branches ?? [],
      local_branch_syncs: status.local_branch_syncs ?? [],
      remote_groups: status.remote_groups ?? [],
      tags: status.tags ?? [],
      stashes: status.stashes ?? [],
      submodules: status.submodules ?? [],
    });
  }

  function isInteractionBusy() {
    return (
      loading ||
      committing ||
      commitAndPushing ||
      stashing ||
      discarding ||
      autoRefreshInFlight ||
      Boolean(topbarBusyAction) ||
      Boolean(stashBusyAction) ||
      branchDialogOpen ||
      deleteDialogOpen ||
      discardDialogOpen ||
      pushDialogOpen
    );
  }

  function resetHistoryState() {
    historyRequestId += 1;
    historyCommits = [];
    historyLoading = false;
    historyLoadedAll = false;
  }

  function resetBranchDialog() {
    branchDialogOpen = false;
    branchNameDraft = "";
    branchSwitchAfterCreate = true;
  }

  function resetDeleteDialog() {
    deleteDialogOpen = false;
    deleteBranchNameDraft = "";
    deleteTargetRef = null;
    deleteForceEnabled = false;
  }

  function resetDiscardDialog() {
    discardDialogOpen = false;
    discardPendingPaths = [];
  }

  function resetPushDialog(result = false) {
    pendingPushAction?.resolve?.(result);
    pushDialogOpen = false;
    pendingPushAction = null;
  }

  function closeBranchMenu() {
    branchMenuOpenKey = "";
  }

  function applyTheme(nextTheme) {
    theme = nextTheme === "light" ? "light" : "dark";
    document.documentElement.dataset.theme = theme;
    localStorage.setItem(themeStorageKey, theme);
  }

  function toggleTheme() {
    applyTheme(theme === "dark" ? "light" : "dark");
  }

  async function openRepositoryAt(path, options = {}) {
    const { remember = true, resetPane = true, clearSavedOnError = false } = options;
    const trimmed = path.trim();
    if (!trimmed) {
      return;
    }

    error = "";
    repository = null;
    selectedRef = null;
    selectedCommitOid = "";
    selectedCommitScrollToken = 0;
    pendingRefCommitOid = "";
    selectedCommitDetail = null;
    selectedCommitDetailLoading = false;
    closeBranchMenu();
    resetPushDialog();
    resetHistoryState();
    if (resetPane) {
      rightPaneExpanded = false;
      rightPaneTab = "commit";
    }
    loading = true;

    try {
      repository = await invoke("open_repository", { path: trimmed });
      if (remember) {
        localStorage.setItem(lastRepositoryKey, repository.repo_path);
      }
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      if (remember || clearSavedOnError) {
        localStorage.removeItem(lastRepositoryKey);
      }
      error = String(message);
    } finally {
      loading = false;
    }
  }

  async function selectRepository() {
    error = "";
    const selected = await open({
      directory: true,
      multiple: false,
      title: t("dialog.openRepository"),
    });

    if (!selected) {
      return;
    }

    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) {
      return;
    }

    await openRepositoryAt(path);
  }

  async function refreshRepository() {
    if (!repository) {
      return;
    }

    loading = true;
    topbarBusyAction = "Refresh";
    error = "";
    try {
      repository = await invoke("get_repository_status", {
        path: repository.repo_path,
      });
      selectedRef = null;
      selectedCommitOid = "";
      selectedCommitScrollToken = 0;
      pendingRefCommitOid = "";
      selectedCommitDetail = null;
      selectedCommitDetailLoading = false;
      closeBranchMenu();
      resetPushDialog();
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
      topbarBusyAction = "";
    }
  }

  function currentBranchHasUpstream() {
    return Boolean(repository?.current_branch_upstream_name);
  }

  function pushUnavailableMessage() {
    if (!repository?.has_origin_remote && !currentBranchHasUpstream()) {
      return t("push.missingOriginMessage");
    }

    return t("push.currentBranchUnavailableMessage");
  }

  function openPushDialog(kind, message = "") {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return Promise.resolve(false);
    }

    if (currentBranchHasUpstream()) {
      return Promise.resolve(true);
    }

    if (!repository.has_origin_remote || !repository.can_push_current_branch) {
      error = pushUnavailableMessage();
      return Promise.resolve(false);
    }

    resetPushDialog(false);
    error = "";

    return new Promise((resolve) => {
      pendingPushAction = { kind, message, resolve };
      pushDialogOpen = true;
    });
  }

  async function runPushCurrentBranch(createUpstreamIfMissing = false) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return false;
    }

    loading = true;
    topbarBusyAction = "Push";
    error = "";

    try {
      repository = await invoke("push_current_branch", {
        path: repository.repo_path,
        createUpstreamIfMissing,
      });
      void loadCommitHistory(repository.repo_path);
      return true;
    } catch (message) {
      error = String(message);
      return false;
    } finally {
      loading = false;
      topbarBusyAction = "";
    }
  }

  async function runCommitAndPushChanges(message, createUpstreamIfMissing = false) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return false;
    }

    commitAndPushing = true;
    error = "";
    try {
      const updated = await invoke("commit_and_push", {
        path: repository.repo_path,
        message,
        createUpstreamIfMissing,
      });
      repository = updated;
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      void loadCommitHistory(repository.repo_path);
      return true;
    } catch (messageText) {
      error = String(messageText);
      return false;
    } finally {
      commitAndPushing = false;
    }
  }

  async function handleTopAction(action) {
    if (!repository || !implementedTopActions.includes(action)) {
      return;
    }

    if (action === "Stash" || action === "Discard") {
      rightPaneExpanded = true;
      rightPaneTab = action.toLowerCase();
      return;
    }

    if (action === "Push") {
      if (!repository.can_push_current_branch) {
        error = pushUnavailableMessage();
        return;
      }

      if (!currentBranchHasUpstream()) {
        await openPushDialog("push");
        return;
      }

      await runPushCurrentBranch(false);
      return;
    }

    loading = true;
    topbarBusyAction = action;
    error = "";

    try {
      if (action === "Fetch") {
        repository = await invoke("fetch_origin", {
          path: repository.repo_path,
        });
      } else if (action === "Pull") {
        repository = await invoke("pull_current_branch", {
          path: repository.repo_path,
        });
      }

      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
      topbarBusyAction = "";
    }
  }

  async function commitChanges(message) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return false;
    }

    if (!message.trim()) {
      error = t("errors.commitMessageEmpty");
      return false;
    }

    committing = true;
    error = "";
    try {
      const updated = await invoke("commit_all", {
        path: repository.repo_path,
        message,
      });
      repository = updated;
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      void loadCommitHistory(repository.repo_path);
      return true;
    } catch (messageText) {
      error = String(messageText);
      return false;
    } finally {
      committing = false;
    }
  }

  async function stashChanges(message, selectedPaths) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return false;
    }

    if (!selectedPaths.length) {
      error = t("errors.stashFilesEmpty");
      return false;
    }

    stashing = true;
    error = "";

    try {
      repository = await invoke("stash_changes", {
        path: repository.repo_path,
        message,
        selectedPaths,
      });
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      selectedStashIndex = null;
      void loadCommitHistory(repository.repo_path);
      return true;
    } catch (messageText) {
      error = String(messageText);
      return false;
    } finally {
      stashing = false;
    }
  }

  function toggleRightPane() {
    if (rightPaneExpanded) {
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      return;
    }

    rightPaneExpanded = true;
    rightPaneTab = "commit";
  }

  async function commitAndPushChanges(message) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return false;
    }

    if (!message.trim()) {
      error = t("errors.commitMessageEmpty");
      return false;
    }

    if (!repository.can_push_current_branch) {
      error = pushUnavailableMessage();
      return false;
    }

    if (!currentBranchHasUpstream()) {
      return await openPushDialog("commit_and_push", message);
    }

    return await runCommitAndPushChanges(message, false);
  }

  async function checkoutBranch(branchName, remoteName = null) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return;
    }

    loading = true;
    error = "";

    try {
      repository = await invoke("checkout_branch", {
        path: repository.repo_path,
        branchName,
        remoteName,
      });
      selectedStashIndex = null;
      selectedRef = null;
      closeBranchMenu();
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
  }

  async function discardChanges(selectedPaths) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return false;
    }

    if (!selectedPaths.length) {
      error = t("errors.discardFilesEmpty");
      return false;
    }

    discardPendingPaths = [...selectedPaths];
    discardDialogOpen = true;
    error = "";
    return false;
  }

  async function confirmDiscardChanges() {
    if (!repository || discardPendingPaths.length === 0) {
      resetDiscardDialog();
      return;
    }

    discarding = true;
    error = "";

    try {
      repository = await invoke("discard_changes", {
        path: repository.repo_path,
        selectedPaths: discardPendingPaths,
      });
      resetDiscardDialog();
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      void loadCommitHistory(repository.repo_path);
    } catch (messageText) {
      error = String(messageText);
    } finally {
      discarding = false;
    }
  }

  function pushDialogTarget() {
    if (!repository) {
      return "origin";
    }

    return `origin/${repository.branch}`;
  }

  function pushDialogDescription() {
    if (!repository) {
      return "";
    }

    if (pendingPushAction?.kind === "commit_and_push") {
      return t("push.createBranchAfterCommitDescription", {
        branch: repository.branch,
        remote: "origin",
      });
    }

    return t("push.createBranchDescription", {
      branch: repository.branch,
      remote: "origin",
    });
  }

  async function confirmPushDialog() {
    if (!pendingPushAction) {
      resetPushDialog(false);
      return;
    }

    const success =
      pendingPushAction.kind === "commit_and_push"
        ? await runCommitAndPushChanges(pendingPushAction.message, true)
        : await runPushCurrentBranch(true);

    if (success) {
      resetPushDialog(true);
    }
  }

  async function checkoutSelectedRef() {
    if (!selectedRef || !selectedRef.canCheckout) {
      return;
    }

    if (selectedRef.kind === "local_branch") {
      await checkoutBranch(selectedRef.name);
      return;
    }

    if (selectedRef.kind === "remote_branch") {
      await checkoutBranch(selectedRef.name, selectedRef.remoteName);
    }
  }

  async function checkoutReference(ref) {
    selectedRef = ref;
    pendingRefCommitOid = "";
    await checkoutSelectedRef();
  }

  async function selectTag(tagName) {
    if (!repository || !tagName) {
      return;
    }

    selectedStashIndex = null;
    closeBranchMenu();
    selectedRef = {
      key: `tag:${tagName}`,
      kind: "tag",
      name: tagName,
      displayName: tagName,
      canCheckout: false,
      canCreateBranch: true,
      canDelete: false,
    };
    error = "";

    try {
      const target = await invoke("resolve_tag_target", {
        path: repository.repo_path,
        tagName,
      });

      if (selectedRef?.kind !== "tag" || selectedRef.name !== tagName) {
        return;
      }

      pendingRefCommitOid = target.oid;
      if (historyCommits.some((commit) => commit.oid === target.oid)) {
        selectCommit(target.oid);
      } else {
        selectedCommitScrollToken += 1;
      }
    } catch (message) {
      if (selectedRef?.kind === "tag" && selectedRef.name === tagName) {
        pendingRefCommitOid = "";
      }
      error = String(message);
    }
  }

  async function deleteReference(ref) {
    if (!repository || !ref?.canDelete) {
      return;
    }
    closeBranchMenu();
    deleteTargetRef = ref;
    deleteBranchNameDraft = "";
    deleteForceEnabled = false;
    deleteDialogOpen = true;
  }

  async function confirmDeleteReference() {
    if (!repository || !deleteTargetRef?.canDelete) {
      return;
    }

    if (deleteBranchNameDraft.trim() !== deleteTargetRef.displayName) {
      error = t("branchDelete.nameMismatch");
      return;
    }

    loading = true;
    error = "";

    try {
      repository = await invoke("delete_branch", {
        path: repository.repo_path,
        branchName: deleteTargetRef.name,
        branchKind: deleteTargetRef.kind,
        remoteName: deleteTargetRef.remoteName ?? null,
        forceDelete: deleteForceEnabled,
      });
      if (selectedRef?.key === deleteTargetRef.key) {
        selectedRef = null;
        pendingRefCommitOid = "";
      }
      resetDeleteDialog();
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
  }

  function openCreateBranchDialog(ref = selectedRef) {
    if (!ref?.canCreateBranch) {
      return;
    }

    selectedRef = ref;
    closeBranchMenu();
    branchNameDraft = "";
    branchSwitchAfterCreate = true;
    branchDialogOpen = true;
  }

  async function createBranchFromSelectedRef() {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return;
    }

    if (!selectedRef?.canCreateBranch) {
      return;
    }

    if (!branchNameDraft.trim()) {
      error = t("errors.branchNameEmpty");
      return;
    }

    loading = true;
    error = "";

    try {
      repository = await invoke("create_branch", {
        path: repository.repo_path,
        branchName: branchNameDraft.trim(),
        sourceName: selectedRef.name,
        sourceKind: selectedRef.kind,
        sourceRemoteName: selectedRef.remoteName ?? null,
        switchAfterCreate: branchSwitchAfterCreate,
      });
      selectedRef = null;
      pendingRefCommitOid = "";
      selectedStashIndex = null;
      resetBranchDialog();
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
  }

  async function applySelectedStash(index) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return;
    }

    loading = true;
    stashBusyAction = "apply";
    error = "";

    try {
      repository = await invoke("apply_stash", {
        path: repository.repo_path,
        index,
      });
      selectedStashIndex = null;
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
      stashBusyAction = "";
    }
  }

  async function popSelectedStash(index) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return;
    }

    loading = true;
    stashBusyAction = "pop";
    error = "";

    try {
      repository = await invoke("pop_stash", {
        path: repository.repo_path,
        index,
      });
      selectedStashIndex = null;
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
      stashBusyAction = "";
    }
  }

  async function loadCommitHistory(path, options = {}) {
    const { preserveSelection = false } = options;
    const requestId = historyRequestId + 1;
    historyRequestId = requestId;
    const previousSelectedCommitOid = preserveSelection ? selectedCommitOid : "";
    historyCommits = [];
    historyLoading = true;
    historyLoadedAll = false;
    if (!preserveSelection) {
      selectedCommitOid = "";
      selectedCommitScrollToken = 0;
      pendingRefCommitOid = "";
      selectedCommitDetail = null;
      selectedCommitDetailLoading = false;
    }

    let offset = 0;

    try {
      while (true) {
        const chunk = await invoke("get_commit_history_chunk", {
          path,
          offset,
          limit: historyBatchSize,
        });

        if (requestId !== historyRequestId) {
          return;
        }

        if (chunk.commits.length > 0) {
          historyCommits = [...historyCommits, ...chunk.commits];
          offset += chunk.commits.length;
        }

        if (!chunk.has_more) {
          historyLoadedAll = true;
          break;
        }

        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    } catch (message) {
      if (requestId !== historyRequestId) {
        return;
      }

      error = String(message);
    } finally {
      if (requestId === historyRequestId) {
        historyLoading = false;
        if (preserveSelection && previousSelectedCommitOid && !historyCommits.some((commit) => commit.oid === previousSelectedCommitOid)) {
          selectedCommitOid = "";
          selectedCommitDetail = null;
          selectedCommitDetailLoading = false;
        }
      }
    }
  }

  async function autoRefreshRepository() {
    if (!repository || !appVisible || isInteractionBusy()) {
      return;
    }

    const repoPath = repository.repo_path;
    const previousStatus = repository;
    autoRefreshInFlight = true;

    try {
      const updated = await invoke("get_repository_status", {
        path: repoPath,
      });

      if (!repository || repository.repo_path !== repoPath) {
        return;
      }

      const previousFingerprint = buildRepositoryStatusFingerprint(previousStatus);
      const nextFingerprint = buildRepositoryStatusFingerprint(updated);
      const historyChanged =
        previousStatus.history_revision !== updated.history_revision ||
        previousStatus.head_oid !== updated.head_oid;

      if (previousFingerprint !== nextFingerprint) {
        repository = updated;
      }

      if (historyChanged) {
        void loadCommitHistory(repoPath, { preserveSelection: true });
      }
    } catch (_message) {
      // Ignore transient auto-refresh failures and keep the current UI state.
    } finally {
      autoRefreshInFlight = false;
    }
  }

  onMount(() => {
    applyTheme(localStorage.getItem(themeStorageKey) ?? "dark");

    const handleVisibilityChange = () => {
      appVisible = document.visibilityState === "visible";
      if (appVisible) {
        void autoRefreshRepository();
      }
    };

    const handleWindowFocus = () => {
      appVisible = true;
      void autoRefreshRepository();
    };

    appVisible = document.visibilityState === "visible";
    const intervalId = window.setInterval(() => {
      void autoRefreshRepository();
    }, autoRefreshIntervalMs);
    document.addEventListener("visibilitychange", handleVisibilityChange);
    window.addEventListener("focus", handleWindowFocus);

    const savedPath = localStorage.getItem(lastRepositoryKey);
    if (!savedPath) {
      return () => {
        window.clearInterval(intervalId);
        document.removeEventListener("visibilitychange", handleVisibilityChange);
        window.removeEventListener("focus", handleWindowFocus);
      };
    }

    void openRepositoryAt(savedPath, { remember: false, clearSavedOnError: true });

    return () => {
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.removeEventListener("focus", handleWindowFocus);
    };
  });

  $: changedEntries = repository ? repository.entries : [];
  $: if (repository && selectedStashIndex !== null) {
    const stillExists = repository.stashes?.some((stash) => stash.index === selectedStashIndex);
    if (!stillExists) {
      selectedStashIndex = null;
    }
  }
  $: if (repository && selectedRef) {
    const stillExists =
      (selectedRef.kind === "local_branch" && repository.local_branches?.includes(selectedRef.name)) ||
      (selectedRef.kind === "remote_branch" &&
        repository.remote_groups?.some(
          (group) => group.name === selectedRef.remoteName && group.branches.includes(selectedRef.name),
        )) ||
      (selectedRef.kind === "tag" && repository.tags?.includes(selectedRef.name));
    if (!stillExists) {
      selectedRef = null;
      pendingRefCommitOid = "";
    }
  }
  $: if (pendingRefCommitOid && historyCommits.some((commit) => commit.oid === pendingRefCommitOid)) {
    selectCommit(pendingRefCommitOid);
    selectedCommitScrollToken += 1;
    pendingRefCommitOid = "";
  }
  $: if (selectedCommitOid && !historyCommits.some((commit) => commit.oid === selectedCommitOid)) {
    selectedCommitOid = "";
    selectedCommitScrollToken = 0;
    selectedCommitDetail = null;
    selectedCommitDetailLoading = false;
  }

  async function loadCommitDetail(path, oid) {
    const requestId = commitDetailRequestId + 1;
    commitDetailRequestId = requestId;
    selectedCommitDetailLoading = true;

    try {
      const detail = await invoke("get_commit_detail", {
        path,
        oid,
      });

      if (requestId !== commitDetailRequestId || selectedCommitOid !== oid) {
        return;
      }

      selectedCommitDetail = detail;
    } catch (message) {
      if (requestId !== commitDetailRequestId) {
        return;
      }

      error = String(message);
    } finally {
      if (requestId === commitDetailRequestId) {
        selectedCommitDetailLoading = false;
      }
    }
  }

  function selectCommit(oid) {
    if (!repository || !oid) {
      return;
    }

    if (selectedCommitOid === oid) {
      selectedCommitScrollToken += 1;
      return;
    }

    selectedCommitOid = oid;
    selectedCommitDetail = null;
    void loadCommitDetail(repository.repo_path, oid);
  }

  function closeCommitDetail() {
    selectedCommitOid = "";
    selectedCommitDetail = null;
    selectedCommitDetailLoading = false;
  }

  async function loadWorktreeCompareDiff(entry) {
    if (!repository || !entry?.path) {
      error = t("errors.openRepositoryFirst");
      return null;
    }

    error = "";

    try {
      return await invoke("get_worktree_file_diff", {
        path: repository.repo_path,
        filePath: entry.path,
      });
    } catch (message) {
      error = String(message);
      return null;
    }
  }
</script>

<svelte:head>
  <title>{t("app.title")}</title>
</svelte:head>

<div class="app-shell">
  <TopBar
    {repository}
    {loading}
    {theme}
    {topActions}
    implementedActions={implementedTopActions}
    activeAction={topbarBusyAction}
    onAction={handleTopAction}
    onRefresh={refreshRepository}
    onToggleTheme={toggleTheme}
  />

  <main class:workspace-collapsed={!rightPaneExpanded} class="workspace">
    <SidebarPane
      {repository}
      {loading}
      selectedStashIndex={selectedStashIndex}
      {selectedRef}
      stashBusyAction={stashBusyAction}
      onSelectRepository={selectRepository}
      onSelectStash={(index) => (selectedStashIndex = index)}
      onSelectTag={selectTag}
      menuOpenKey={branchMenuOpenKey}
      onToggleMenu={(key) => (branchMenuOpenKey = key)}
      onCheckoutReference={checkoutReference}
      onCreateBranchFromReference={openCreateBranchDialog}
      onDeleteReference={deleteReference}
      onCancelSelectedStash={() => (selectedStashIndex = null)}
      onApplySelectedStash={applySelectedStash}
      onPopSelectedStash={popSelectedStash}
    />

    <HistoryPane
      {repository}
      {error}
      {historyCommits}
      {historyLoading}
      {historyLoadedAll}
      {selectedCommitOid}
      {selectedCommitScrollToken}
      {selectedCommitDetail}
      {selectedCommitDetailLoading}
      onSelectCommit={selectCommit}
      onCloseCommitDetail={closeCommitDetail}
    />

    <CommitPane
      {repository}
      changedEntries={changedEntries}
      expanded={rightPaneExpanded}
      activeTab={rightPaneTab}
      {committing}
      {commitAndPushing}
      {stashing}
      {discarding}
      onToggle={toggleRightPane}
      onSelectTab={(tab) => (rightPaneTab = tab)}
      onStash={stashChanges}
      onDiscard={discardChanges}
      onCommit={commitChanges}
      onCommitAndPush={commitAndPushChanges}
      onLoadCompareDiff={loadWorktreeCompareDiff}
    />
  </main>

  {#if branchDialogOpen && selectedRef}
    <div class="dialog-backdrop" role="presentation" on:click={(event) => event.target === event.currentTarget && !loading && resetBranchDialog()}>
      <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="branch-dialog-title">
        <div class="dialog-copy">
          <h2 id="branch-dialog-title">{t("branchDialog.title")}</h2>
          <p>{t("branchDialog.fromSource", { source: selectedRef.displayName })}</p>
        </div>

        <label class="dialog-field">
          <span>{t("branchDialog.nameLabel")}</span>
          <input bind:value={branchNameDraft} placeholder={t("branchDialog.namePlaceholder")} disabled={loading} />
        </label>

        <label class="dialog-checkbox">
          <input type="checkbox" bind:checked={branchSwitchAfterCreate} disabled={loading} />
          <span>{t("branchDialog.switchAfterCreate")}</span>
        </label>

        <div class="dialog-actions">
          <button class="dialog-button dialog-button-muted" type="button" on:click={resetBranchDialog} disabled={loading}>
            {t("branchDialog.cancel")}
          </button>
          <button class="dialog-button" type="button" on:click={createBranchFromSelectedRef} disabled={loading}>
            {loading ? t("branchDialog.creating") : t("branchDialog.create")}
          </button>
        </div>
      </section>
    </div>
  {/if}

  {#if deleteDialogOpen && deleteTargetRef}
    <div class="dialog-backdrop" role="presentation" on:click={(event) => event.target === event.currentTarget && !loading && resetDeleteDialog()}>
      <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="delete-dialog-title">
        <div class="dialog-copy">
          <h2 id="delete-dialog-title">{t("branchDelete.title")}</h2>
          <p>{t("branchDelete.description", { branch: deleteTargetRef.displayName })}</p>
        </div>

        <div class="dialog-warning">
          <span class="dialog-warning-label">{t("branchDelete.targetLabel")}</span>
          <code>{deleteTargetRef.displayName}</code>
        </div>

        <label class="dialog-field">
          <span>{t("branchDelete.inputLabel")}</span>
          <input bind:value={deleteBranchNameDraft} placeholder={deleteTargetRef.displayName} disabled={loading} />
        </label>

        <p class="dialog-helper">{t("branchDelete.inputHint", { branch: deleteTargetRef.displayName })}</p>

        {#if deleteTargetRef.kind === "local_branch"}
          <label class="dialog-checkbox">
            <input type="checkbox" bind:checked={deleteForceEnabled} disabled={loading} />
            <span>{t("branchDelete.forceOption")}</span>
          </label>
        {/if}

        <div class="dialog-actions">
          <button class="dialog-button dialog-button-muted" type="button" on:click={resetDeleteDialog} disabled={loading}>
            {t("branchDelete.cancel")}
          </button>
          <button
            class="dialog-button dialog-button-danger"
            type="button"
            on:click={confirmDeleteReference}
            disabled={loading || deleteBranchNameDraft.trim() !== deleteTargetRef.displayName}
          >
            {loading ? t("branchDelete.deleting") : t("branchDelete.delete")}
          </button>
        </div>
      </section>
    </div>
  {/if}

  {#if discardDialogOpen}
    <div class="dialog-backdrop" role="presentation" on:click={(event) => event.target === event.currentTarget && !discarding && resetDiscardDialog()}>
      <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="discard-dialog-title">
        <div class="dialog-copy">
          <h2 id="discard-dialog-title">{t("discardDialog.title")}</h2>
          <p>{t("discardDialog.description", { count: discardPendingPaths.length })}</p>
        </div>

        <div class="dialog-warning">
          <span class="dialog-warning-label">{t("discardDialog.targetLabel")}</span>
          <code>{t("discardDialog.selectionCount", { count: discardPendingPaths.length })}</code>
        </div>

        <p class="dialog-helper">{t("discardDialog.warning")}</p>

        <div class="dialog-actions">
          <button class="dialog-button dialog-button-muted" type="button" on:click={resetDiscardDialog} disabled={discarding}>
            {t("discardDialog.cancel")}
          </button>
          <button class="dialog-button dialog-button-danger" type="button" on:click={confirmDiscardChanges} disabled={discarding}>
            {discarding ? t("discardDialog.discarding") : t("discardDialog.confirm")}
          </button>
        </div>
      </section>
    </div>
  {/if}

  {#if pushDialogOpen && repository}
    <div
      class="dialog-backdrop"
      role="presentation"
      on:click={(event) => event.target === event.currentTarget && !loading && !commitAndPushing && resetPushDialog(false)}
    >
      <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="push-dialog-title">
        <div class="dialog-copy">
          <h2 id="push-dialog-title">{t("push.noUpstreamTitle")}</h2>
          <p>{pushDialogDescription()}</p>
        </div>

        <div class="dialog-warning">
          <span class="dialog-warning-label">{t("push.targetLabel")}</span>
          <code>{pushDialogTarget()}</code>
        </div>

        <div class="dialog-actions">
          <button
            class="dialog-button dialog-button-muted"
            type="button"
            on:click={() => resetPushDialog(false)}
            disabled={loading || commitAndPushing}
          >
            {t("push.dialogCancel")}
          </button>
          <button class="dialog-button" type="button" on:click={confirmPushDialog} disabled={loading || commitAndPushing}>
            {pendingPushAction?.kind === "commit_and_push"
              ? commitAndPushing
                ? t("push.commitCreatingAndPushing")
                : t("push.commitCreateAndPush")
              : loading
                ? t("push.creatingAndPushing")
                : t("push.createAndPush")}
          </button>
        </div>
      </section>
    </div>
  {/if}
</div>

<style>
  .app-shell {
    height: 100vh;
    display: grid;
    grid-template-rows: 60px 1fr;
    overflow: hidden;
  }

  .workspace {
    min-height: 0;
    height: 100%;
    display: grid;
    grid-template-columns: 246px minmax(0, 1fr) 332px;
    overflow: hidden;
    transition: grid-template-columns 160ms ease;
  }

  .workspace.workspace-collapsed {
    grid-template-columns: 246px minmax(0, 1fr) 54px;
  }

  .dialog-backdrop {
    position: fixed;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 20px;
    background: var(--backdrop-background);
    backdrop-filter: blur(8px);
    z-index: 20;
  }

  .dialog-card {
    width: min(100%, 420px);
    display: grid;
    gap: 14px;
    padding: 18px;
    border-radius: 14px;
    background: var(--dialog-background);
    border: 1px solid var(--surface-border-strong);
    box-shadow: var(--dialog-shadow);
  }

  .dialog-copy h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .dialog-copy p {
    margin: 6px 0 0;
    color: var(--text-muted);
    font-size: 0.82rem;
    line-height: 1.4;
    word-break: break-word;
  }

  .dialog-field {
    display: grid;
    gap: 6px;
  }

  .dialog-field span,
  .dialog-checkbox span {
    color: var(--text-secondary);
    font-size: 0.78rem;
  }

  .dialog-field input {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--surface-border);
    border-radius: 10px;
    background: var(--input-background);
    color: var(--text-secondary);
    padding: 11px 12px;
  }

  .dialog-field input:focus {
    outline: none;
    border-color: var(--input-border-focus);
    box-shadow: var(--focus-ring);
    background: var(--input-background-focus);
  }

  .dialog-checkbox {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dialog-checkbox input[type="checkbox"] {
    accent-color: var(--accent);
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .dialog-warning {
    display: grid;
    gap: 4px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--danger-soft);
    border: 1px solid var(--danger-border);
  }

  .dialog-warning-label,
  .dialog-helper {
    color: var(--text-secondary);
    font-size: 0.74rem;
    line-height: 1.4;
  }

  .dialog-warning code {
    color: var(--danger-text);
    font-size: 0.8rem;
    word-break: break-word;
  }

  .dialog-button {
    border: 0;
    border-radius: 10px;
    background: linear-gradient(180deg, var(--accent-strong), var(--accent-strong-2));
    color: var(--accent-contrast);
    padding: 10px 14px;
    font-size: 0.76rem;
    font-weight: 700;
    letter-spacing: 0.03em;
  }

  .dialog-button.dialog-button-muted {
    background: var(--surface-background-muted);
    color: var(--text-secondary);
    border: 1px solid var(--surface-border);
  }

  .dialog-button.dialog-button-danger {
    background: linear-gradient(180deg, var(--danger-strong), var(--danger-strong-2));
  }

  @media (max-width: 1180px) {
    .workspace {
      grid-template-columns: 220px minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) auto;
    }

    .workspace.workspace-collapsed {
      grid-template-columns: 220px minmax(0, 1fr);
    }
  }

  @media (max-width: 860px) {
    .app-shell {
      height: auto;
      min-height: 100vh;
      grid-template-rows: auto 1fr;
      overflow: auto;
    }

    .workspace {
      grid-template-columns: 1fr;
      height: auto;
      overflow: visible;
    }
  }
</style>
