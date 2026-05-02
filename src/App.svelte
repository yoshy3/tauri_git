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
  let rebaseDialogOpen = false;
  let rebaseTargetRef = null;
  let resetDialogOpen = false;
  let resetTargetCommit = null;
  let resetModeDraft = "mixed";
  let revertDialogOpen = false;
  let revertTargetCommit = null;
  let revertMessageDraft = "";
  let tagDialogOpen = false;
  let tagNameDraft = "";
  let tagMessageDraft = "";
  let tagPushAfterCreate = false;
  let tagTargetRevision = "HEAD";
  let tagTargetLabel = "HEAD";
  let deleteTagDialogOpen = false;
  let deleteTagNameDraft = "";
  let deleteTargetTagName = "";
  let deleteDialogOpen = false;
  let deleteBranchNameDraft = "";
  let deleteTargetRef = null;
  let deleteForceEnabled = false;
  let discardDialogOpen = false;
  let discardPendingPaths = [];
  let pushDialogOpen = false;
  let pendingPushAction = null;
  let pushForceWithLease = false;
  let theme = "dark";
  let leftPaneWidth = 246;
  let rightPaneWidth = 332;
  let workspaceElement;
  let resizeCleanup = null;
  let viewportWidth = 0;
  let recentRepositoryPaths = [];

  const topActions = ["Fetch", "Pull", "Push", "Stash", "Discard"];
  const implementedTopActions = ["Fetch", "Pull", "Push", "Stash", "Discard"];
  const lastRepositoryKey = "tauri-git:last-repository-path";
  const repositoryHistoryKey = "tauri-git:repository-history";
  const themeStorageKey = "tauri-git:theme";
  const paneLayoutStorageKey = "tauri-git:pane-layout";
  const maxRecentRepositories = 10;
  const historyBatchSize = 100;
  const autoRefreshIntervalMs = 2500;
  const minLeftPaneWidth = 180;
  const maxLeftPaneWidth = 420;
  const minCenterPaneWidth = 360;
  const minRightPaneWidth = 260;
  const maxRightPaneWidth = 560;
  const collapsedRightPaneWidth = 54;
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
      head_message: status.head_message ?? null,
      head_is_pushed: status.head_is_pushed ?? false,
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
      rebaseDialogOpen ||
      resetDialogOpen ||
      revertDialogOpen ||
      tagDialogOpen ||
      deleteDialogOpen ||
      deleteTagDialogOpen ||
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

  function resetRebaseDialog() {
    rebaseDialogOpen = false;
    rebaseTargetRef = null;
  }

  function resetResetDialog() {
    resetDialogOpen = false;
    resetTargetCommit = null;
    resetModeDraft = "mixed";
  }

  function resetRevertDialog() {
    revertDialogOpen = false;
    revertTargetCommit = null;
    revertMessageDraft = "";
  }

  function resetDeleteDialog() {
    deleteDialogOpen = false;
    deleteBranchNameDraft = "";
    deleteTargetRef = null;
    deleteForceEnabled = false;
  }

  function resetTagDialog() {
    tagDialogOpen = false;
    tagNameDraft = "";
    tagMessageDraft = "";
    tagPushAfterCreate = false;
    tagTargetRevision = "HEAD";
    tagTargetLabel = "HEAD";
  }

  function resetDeleteTagDialog() {
    deleteTagDialogOpen = false;
    deleteTagNameDraft = "";
    deleteTargetTagName = "";
  }

  function resetDiscardDialog() {
    discardDialogOpen = false;
    discardPendingPaths = [];
  }

  function resetPushDialog(result = false) {
    pendingPushAction?.resolve?.(result);
    pushDialogOpen = false;
    pendingPushAction = null;
    pushForceWithLease = false;
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

  function clamp(value, min, max) {
    return Math.min(Math.max(value, min), max);
  }

  function savePaneLayout() {
    localStorage.setItem(
      paneLayoutStorageKey,
      JSON.stringify({
        leftPaneWidth,
        rightPaneWidth,
      }),
    );
  }

  function restorePaneLayout() {
    const savedLayout = localStorage.getItem(paneLayoutStorageKey);
    if (!savedLayout) {
      return;
    }

    try {
      const parsed = JSON.parse(savedLayout);
      if (typeof parsed.leftPaneWidth === "number") {
        leftPaneWidth = clamp(parsed.leftPaneWidth, minLeftPaneWidth, maxLeftPaneWidth);
      }
      if (typeof parsed.rightPaneWidth === "number") {
        rightPaneWidth = clamp(parsed.rightPaneWidth, minRightPaneWidth, maxRightPaneWidth);
      }
    } catch (_error) {
      localStorage.removeItem(paneLayoutStorageKey);
    }
  }

  function loadRecentRepositoryPaths() {
    const savedHistory = localStorage.getItem(repositoryHistoryKey);
    if (!savedHistory) {
      const lastPath = localStorage.getItem(lastRepositoryKey);
      recentRepositoryPaths = lastPath ? [lastPath] : [];
      return;
    }

    try {
      const parsed = JSON.parse(savedHistory);
      if (Array.isArray(parsed)) {
        recentRepositoryPaths = parsed
          .map((value) => String(value || "").trim())
          .filter(Boolean)
          .slice(0, maxRecentRepositories);
        return;
      }
    } catch (_error) {
      localStorage.removeItem(repositoryHistoryKey);
    }

    const lastPath = localStorage.getItem(lastRepositoryKey);
    recentRepositoryPaths = lastPath ? [lastPath] : [];
  }

  function saveRecentRepositoryPaths() {
    localStorage.setItem(repositoryHistoryKey, JSON.stringify(recentRepositoryPaths));
  }

  function rememberRepositoryPath(path) {
    const trimmed = String(path || "").trim();
    if (!trimmed) {
      return;
    }

    recentRepositoryPaths = [
      trimmed,
      ...recentRepositoryPaths.filter((entry) => entry !== trimmed),
    ].slice(0, maxRecentRepositories);
    localStorage.setItem(lastRepositoryKey, trimmed);
    saveRecentRepositoryPaths();
  }

  function forgetRepositoryPath(path) {
    const trimmed = String(path || "").trim();
    if (!trimmed) {
      return;
    }

    recentRepositoryPaths = recentRepositoryPaths.filter((entry) => entry !== trimmed);
    saveRecentRepositoryPaths();

    if (localStorage.getItem(lastRepositoryKey) === trimmed) {
      if (recentRepositoryPaths.length > 0) {
        localStorage.setItem(lastRepositoryKey, recentRepositoryPaths[0]);
      } else {
        localStorage.removeItem(lastRepositoryKey);
      }
    }
  }

  function applyPaneConstraints() {
    const workspaceWidth = workspaceElement?.clientWidth ?? 0;
    if (!workspaceWidth || viewportWidth <= 1180) {
      return;
    }

    leftPaneWidth = clamp(leftPaneWidth, minLeftPaneWidth, maxLeftPaneWidth);
    rightPaneWidth = clamp(rightPaneWidth, minRightPaneWidth, maxRightPaneWidth);

    const rightWidth = rightPaneExpanded ? rightPaneWidth : collapsedRightPaneWidth;
    const maxLeftFromViewport = workspaceWidth - rightWidth - minCenterPaneWidth;
    leftPaneWidth = clamp(leftPaneWidth, minLeftPaneWidth, Math.max(minLeftPaneWidth, maxLeftFromViewport));

    if (rightPaneExpanded) {
      const maxRightFromViewport = workspaceWidth - leftPaneWidth - minCenterPaneWidth;
      rightPaneWidth = clamp(rightPaneWidth, minRightPaneWidth, Math.max(minRightPaneWidth, maxRightFromViewport));
    }
  }

  function updatePaneLayout(mutator) {
    mutator();
    applyPaneConstraints();
    savePaneLayout();
  }

  function beginPaneResize(pane, event) {
    if (viewportWidth <= 1180) {
      return;
    }

    const workspaceWidth = workspaceElement?.clientWidth ?? 0;
    if (!workspaceWidth) {
      return;
    }

    const startX = event.clientX;
    const startLeftWidth = leftPaneWidth;
    const startRightWidth = rightPaneWidth;

    const handlePointerMove = (moveEvent) => {
      if (pane === "left") {
        updatePaneLayout(() => {
          leftPaneWidth = startLeftWidth + (moveEvent.clientX - startX);
        });
        return;
      }

      updatePaneLayout(() => {
        rightPaneWidth = startRightWidth - (moveEvent.clientX - startX);
      });
    };

    const stopResize = () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", stopResize);
      window.removeEventListener("pointercancel", stopResize);
      document.body.classList.remove("pane-resizing");
      resizeCleanup = null;
      savePaneLayout();
    };

    resizeCleanup?.();
    resizeCleanup = stopResize;
    document.body.classList.add("pane-resizing");
    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", stopResize);
    window.addEventListener("pointercancel", stopResize);
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
    resetRebaseDialog();
    resetResetDialog();
    resetTagDialog();
    resetDeleteTagDialog();
    loading = true;

    try {
      repository = await invoke("open_repository", { path: trimmed });
      if (remember) {
        rememberRepositoryPath(repository.repo_path);
      }
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      if (remember || clearSavedOnError) {
        forgetRepositoryPath(trimmed);
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

  async function openRecentRepository(path) {
    await openRepositoryAt(path, { clearSavedOnError: true });
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
      resetRebaseDialog();
      resetResetDialog();
      resetTagDialog();
      resetDeleteTagDialog();
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

  function openPushDialog(kind, payload = {}) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return Promise.resolve(false);
    }

    if (currentBranchHasUpstream() && !payload.allowForcePush) {
      return Promise.resolve(true);
    }

    if (!repository.has_origin_remote || !repository.can_push_current_branch) {
      error = pushUnavailableMessage();
      return Promise.resolve(false);
    }

    resetPushDialog(false);
    error = "";

    return new Promise((resolve) => {
      pendingPushAction = { kind, payload, resolve };
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
        forceWithLease: pushForceWithLease,
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

  async function runCommitAndPushChanges(message, amend = false, createUpstreamIfMissing = false) {
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
        amend,
        forceWithLease: pushForceWithLease,
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
        await openPushDialog("push", { createUpstreamIfMissing: true });
        return;
      }

      if (repository.ahead_count > 0 && repository.behind_count > 0) {
        await openPushDialog("push", { allowForcePush: true });
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

  async function commitChanges(message, amend = false) {
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
        amend,
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

  async function commitAndPushChanges(message, amend = false) {
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
      return await openPushDialog("commit_and_push", {
        message,
        amend,
        createUpstreamIfMissing: true,
      });
    }

    if (amend && repository.head_is_pushed) {
      return await openPushDialog("commit_and_push", {
        message,
        amend,
        allowForcePush: true,
      });
    }

    return await runCommitAndPushChanges(message, amend, false);
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

    if (pendingPushAction?.payload?.allowForcePush) {
      if (pendingPushAction?.kind === "commit_and_push") {
        return t("push.forceAfterCommitDescription", {
          branch: repository.branch,
          remote: repository.current_branch_upstream_name ?? `origin/${repository.branch}`,
        });
      }

      return t("push.forceDescription", {
        branch: repository.branch,
        remote: repository.current_branch_upstream_name ?? `origin/${repository.branch}`,
      });
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

  function pushDialogTitle() {
    if (pendingPushAction?.payload?.allowForcePush) {
      return t("push.forceTitle");
    }

    return t("push.noUpstreamTitle");
  }

  function pushDialogConfirmLabel() {
    if (pendingPushAction?.payload?.allowForcePush) {
      if (pendingPushAction?.kind === "commit_and_push") {
        return commitAndPushing ? t("push.commitForcePushing") : t("push.commitForcePush");
      }

      return loading ? t("push.forcePushing") : t("push.forcePush");
    }

    if (pendingPushAction?.kind === "commit_and_push") {
      return commitAndPushing ? t("push.commitCreatingAndPushing") : t("push.commitCreateAndPush");
    }

    return loading ? t("push.creatingAndPushing") : t("push.createAndPush");
  }

  async function confirmPushDialog() {
    if (!pendingPushAction) {
      resetPushDialog(false);
      return;
    }

    const success =
      pendingPushAction.kind === "commit_and_push"
        ? await runCommitAndPushChanges(
            pendingPushAction.payload?.message ?? "",
            pendingPushAction.payload?.amend ?? false,
            pendingPushAction.payload?.createUpstreamIfMissing ?? false,
          )
        : await runPushCurrentBranch(pendingPushAction.payload?.createUpstreamIfMissing ?? false);

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
      canDelete: true,
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

  function buildTagTarget() {
    if (selectedCommitOid) {
      const selectedCommitSummary =
        selectedCommitDetail?.oid === selectedCommitOid
          ? selectedCommitDetail.summary
          : historyCommits.find((commit) => commit.oid === selectedCommitOid)?.summary ?? "";

      return {
        revision: selectedCommitOid,
        label: selectedCommitSummary
          ? `${selectedCommitOid.slice(0, 7)} (${t("history.details.sha")}) - ${selectedCommitSummary}`
          : `${selectedCommitOid.slice(0, 7)} (${t("history.details.sha")})`,
      };
    }

    if (selectedRef?.kind === "remote_branch") {
      return {
        revision: `${selectedRef.remoteName}/${selectedRef.name}`,
        label: selectedRef.displayName,
      };
    }

    if (selectedRef?.name) {
      return {
        revision: selectedRef.name,
        label: selectedRef.displayName ?? selectedRef.name,
      };
    }

    return {
      revision: "HEAD",
      label: repository?.branch ? `HEAD (${repository.branch})` : "HEAD",
    };
  }

  function openCreateTagDialog() {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return;
    }

    closeBranchMenu();
    const target = buildTagTarget();
    tagTargetRevision = target.revision;
    tagTargetLabel = target.label;
    tagNameDraft = "";
    tagMessageDraft = "";
    tagPushAfterCreate = false;
    tagDialogOpen = true;
    error = "";
  }

  async function createTag() {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return;
    }

    if (!tagNameDraft.trim()) {
      error = t("errors.tagNameEmpty");
      return;
    }

    loading = true;
    error = "";

    try {
      repository = await invoke("create_tag", {
        path: repository.repo_path,
        tagName: tagNameDraft.trim(),
        target: tagTargetRevision,
        message: tagMessageDraft.trim() || null,
        pushAfterCreate: tagPushAfterCreate,
      });

      resetTagDialog();
      void loadCommitHistory(repository.repo_path, { preserveSelection: true });
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
  }

  function openDeleteTagDialog(tagName = selectedRef?.kind === "tag" ? selectedRef.name : "") {
    if (!repository || !tagName) {
      return;
    }

    closeBranchMenu();
    deleteTargetTagName = tagName;
    deleteTagNameDraft = "";
    deleteTagDialogOpen = true;
    error = "";
  }

  async function confirmDeleteTag() {
    if (!repository || !deleteTargetTagName) {
      return;
    }

    if (deleteTagNameDraft.trim() !== deleteTargetTagName) {
      error = t("tagDelete.nameMismatch");
      return;
    }

    loading = true;
    error = "";

    try {
      repository = await invoke("delete_tag", {
        path: repository.repo_path,
        tagName: deleteTargetTagName,
      });

      if (selectedRef?.kind === "tag" && selectedRef.name === deleteTargetTagName) {
        selectedRef = null;
        pendingRefCommitOid = "";
      }
      resetDeleteTagDialog();
      void loadCommitHistory(repository.repo_path, { preserveSelection: true });
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
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

  function openRebaseDialog(ref = selectedRef) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return;
    }

    if (!ref?.canRebase) {
      return;
    }

    selectedRef = ref;
    closeBranchMenu();
    rebaseTargetRef = ref;
    rebaseDialogOpen = true;
    error = "";
  }

  async function confirmRebaseReference() {
    if (!repository || !rebaseTargetRef?.canRebase) {
      return;
    }

    loading = true;
    error = "";

    try {
      repository = await invoke("rebase_current_branch", {
        path: repository.repo_path,
        targetName: rebaseTargetRef.name,
        targetKind: rebaseTargetRef.kind,
        targetRemoteName: rebaseTargetRef.remoteName ?? null,
      });
      selectedRef = rebaseTargetRef;
      pendingRefCommitOid = "";
      selectedStashIndex = null;
      resetRebaseDialog();
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
  }

  function openResetCommitDialog(commit = selectedCommitDetail) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return;
    }

    const targetCommit =
      commit?.oid
        ? commit
        : selectedCommitOid
          ? selectedCommitDetail?.oid === selectedCommitOid
            ? selectedCommitDetail
            : historyCommits.find((entry) => entry.oid === selectedCommitOid) ?? null
          : null;

    if (!targetCommit?.oid) {
      return;
    }

    closeBranchMenu();
    resetTargetCommit = {
      oid: targetCommit.oid,
      id: targetCommit.id ?? targetCommit.oid.slice(0, 7),
      summary: targetCommit.summary ?? "",
    };
    resetModeDraft = "mixed";
    resetDialogOpen = true;
    error = "";
  }

  function resetDialogDescription() {
    if (!repository || !resetTargetCommit) {
      return "";
    }

    return t("resetDialog.description", {
      branch: repository.branch,
      target: resetTargetCommit.id,
    });
  }

  function resetModeDescription(mode) {
    return t(`resetDialog.modes.${mode}.description`);
  }

  async function confirmResetCommit() {
    if (!repository || !resetTargetCommit?.oid) {
      return;
    }

    loading = true;
    error = "";

    try {
      repository = await invoke("reset_current_branch", {
        path: repository.repo_path,
        target: resetTargetCommit.oid,
        resetMode: resetModeDraft,
      });
      selectedCommitOid = resetTargetCommit.oid;
      selectedCommitDetail = null;
      selectedStashIndex = null;
      pendingRefCommitOid = "";
      resetResetDialog();
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      void loadCommitHistory(repository.repo_path, { preserveSelection: true });
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
  }

  function openRevertCommitDialog(commit = selectedCommitDetail) {
    if (!repository) {
      error = t("errors.openRepositoryFirst");
      return;
    }

    const targetCommit =
      commit?.oid
        ? commit
        : selectedCommitOid
          ? selectedCommitDetail?.oid === selectedCommitOid
            ? selectedCommitDetail
            : historyCommits.find((entry) => entry.oid === selectedCommitOid) ?? null
          : null;

    if (!targetCommit?.oid) {
      return;
    }

    closeBranchMenu();
    revertTargetCommit = {
      oid: targetCommit.oid,
      id: targetCommit.id ?? targetCommit.oid.slice(0, 7),
      summary: targetCommit.summary ?? "",
    };
    revertMessageDraft = `Revert "${revertTargetCommit.summary}"`;
    revertDialogOpen = true;
    error = "";
  }

  async function confirmRevertCommit() {
    if (!repository || !revertTargetCommit?.oid) {
      return;
    }

    loading = true;
    error = "";

    try {
      repository = await invoke("revert_commit", {
        path: repository.repo_path,
        target: revertTargetCommit.oid,
        message: revertMessageDraft,
      });
      selectedCommitDetail = null;
      selectedStashIndex = null;
      pendingRefCommitOid = "";
      resetRevertDialog();
      rightPaneExpanded = false;
      rightPaneTab = "commit";
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
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
    restorePaneLayout();
    loadRecentRepositoryPaths();
    applyPaneConstraints();

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

    const handleResize = () => {
      viewportWidth = window.innerWidth;
      applyPaneConstraints();
      savePaneLayout();
    };

    const handleGlobalPointerDown = (event) => {
      const target = event.target;
      if (!(target instanceof Element)) {
        return;
      }

      if (branchMenuOpenKey && !target.closest(".tree-item-actions")) {
        closeBranchMenu();
      }

      if (
        selectedStashIndex !== null &&
        !target.closest(".stash-actions") &&
        !target.closest(".tree-item-stack")
      ) {
        selectedStashIndex = null;
      }
    };

    viewportWidth = window.innerWidth;
    appVisible = document.visibilityState === "visible";
    const intervalId = window.setInterval(() => {
      void autoRefreshRepository();
    }, autoRefreshIntervalMs);
    document.addEventListener("visibilitychange", handleVisibilityChange);
    document.addEventListener("pointerdown", handleGlobalPointerDown);
    window.addEventListener("focus", handleWindowFocus);
    window.addEventListener("resize", handleResize);

    const savedPath = recentRepositoryPaths[0] ?? localStorage.getItem(lastRepositoryKey);
    if (!savedPath) {
      return () => {
        resizeCleanup?.();
        window.clearInterval(intervalId);
        document.removeEventListener("visibilitychange", handleVisibilityChange);
        document.removeEventListener("pointerdown", handleGlobalPointerDown);
        window.removeEventListener("focus", handleWindowFocus);
        window.removeEventListener("resize", handleResize);
      };
    }

    void openRepositoryAt(savedPath, { remember: false, clearSavedOnError: true });

    return () => {
      resizeCleanup?.();
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      document.removeEventListener("pointerdown", handleGlobalPointerDown);
      window.removeEventListener("focus", handleWindowFocus);
      window.removeEventListener("resize", handleResize);
    };
  });

  $: paneLayoutStyle =
    viewportWidth <= 1180
      ? undefined
      : `--left-pane-width: ${leftPaneWidth}px; --right-pane-width: ${
          rightPaneExpanded ? rightPaneWidth : collapsedRightPaneWidth
        }px;`;
  $: if (workspaceElement) {
    applyPaneConstraints();
  }

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
    recentRepositoryPaths={recentRepositoryPaths}
    {topActions}
    implementedActions={implementedTopActions}
    activeAction={topbarBusyAction}
    canResetSelectedCommit={Boolean(selectedCommitOid)}
    onAction={handleTopAction}
    onSelectRepository={selectRepository}
    onOpenRecentRepository={openRecentRepository}
    onResetSelectedCommit={() => openResetCommitDialog()}
    onRefresh={refreshRepository}
    onToggleTheme={toggleTheme}
  />

  <main bind:this={workspaceElement} class:workspace-collapsed={!rightPaneExpanded} class="workspace" style={paneLayoutStyle}>
    <SidebarPane
      {repository}
      {loading}
      selectedStashIndex={selectedStashIndex}
      {selectedRef}
      stashBusyAction={stashBusyAction}
      onSelectStash={(index) => (selectedStashIndex = index)}
      onSelectTag={selectTag}
      onOpenCreateTagDialog={openCreateTagDialog}
      onOpenDeleteTagDialog={openDeleteTagDialog}
      menuOpenKey={branchMenuOpenKey}
      onToggleMenu={(key) => (branchMenuOpenKey = key)}
      onCheckoutReference={checkoutReference}
      onCreateBranchFromReference={openCreateBranchDialog}
      onRebaseReference={openRebaseDialog}
      onDeleteReference={deleteReference}
      onCancelSelectedStash={() => (selectedStashIndex = null)}
      onApplySelectedStash={applySelectedStash}
      onPopSelectedStash={popSelectedStash}
    />

    <div
      aria-label="Resize left pane"
      class="pane-resizer pane-resizer-left"
      on:pointerdown={(event) => beginPaneResize("left", event)}
      role="separator"
      tabindex="-1"
    ></div>

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
      onOpenResetCommitDialog={openResetCommitDialog}
      onOpenRevertCommitDialog={openRevertCommitDialog}
      onCloseCommitDetail={closeCommitDetail}
    />

    <div
      aria-hidden={!rightPaneExpanded}
      aria-label="Resize right pane"
      class:pane-resizer-disabled={!rightPaneExpanded}
      class="pane-resizer pane-resizer-right"
      on:pointerdown={(event) => beginPaneResize("right", event)}
      role="separator"
      tabindex="-1"
    ></div>

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

  {#if rebaseDialogOpen && rebaseTargetRef && repository}
    <div class="dialog-backdrop" role="presentation" on:click={(event) => event.target === event.currentTarget && !loading && resetRebaseDialog()}>
      <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="rebase-dialog-title">
        <div class="dialog-copy">
          <h2 id="rebase-dialog-title">{t("rebaseDialog.title")}</h2>
          <p>{t("rebaseDialog.description", { branch: repository.branch, target: rebaseTargetRef.displayName })}</p>
        </div>

        <div class="dialog-warning">
          <span class="dialog-warning-label">{t("rebaseDialog.targetLabel")}</span>
          <code>{rebaseTargetRef.displayName}</code>
        </div>

        <p class="dialog-helper">{t("rebaseDialog.warning")}</p>

        <div class="dialog-actions">
          <button class="dialog-button dialog-button-muted" type="button" on:click={resetRebaseDialog} disabled={loading}>
            {t("rebaseDialog.cancel")}
          </button>
          <button class="dialog-button" type="button" on:click={confirmRebaseReference} disabled={loading}>
            {loading ? t("rebaseDialog.rebasing") : t("rebaseDialog.confirm")}
          </button>
        </div>
      </section>
    </div>
  {/if}

  {#if resetDialogOpen && resetTargetCommit && repository}
    <div class="dialog-backdrop" role="presentation" on:click={(event) => event.target === event.currentTarget && !loading && resetResetDialog()}>
      <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="reset-dialog-title">
        <div class="dialog-copy">
          <h2 id="reset-dialog-title">{t("resetDialog.title")}</h2>
          <p>{resetDialogDescription()}</p>
        </div>

        <div class="dialog-warning">
          <span class="dialog-warning-label">{t("resetDialog.targetLabel")}</span>
          <code>{resetTargetCommit.id} {resetTargetCommit.summary ? `- ${resetTargetCommit.summary}` : ""}</code>
        </div>

        <div class="dialog-radio-group" role="radiogroup" aria-label={t("resetDialog.modeLabel")}>
          {#each ["soft", "mixed", "hard"] as mode}
            <label class:dialog-radio-option-danger={mode === "hard"} class="dialog-radio-option">
              <input type="radio" name="reset-mode" bind:group={resetModeDraft} value={mode} disabled={loading} />
              <span class="dialog-radio-copy">
                <strong>{t(`resetDialog.modes.${mode}.label`)}</strong>
                <span>{resetModeDescription(mode)}</span>
              </span>
            </label>
          {/each}
        </div>

        <p class="dialog-helper">{t(`resetDialog.warnings.${resetModeDraft}`)}</p>

        <div class="dialog-actions">
          <button class="dialog-button dialog-button-muted" type="button" on:click={resetResetDialog} disabled={loading}>
            {t("resetDialog.cancel")}
          </button>
          <button class:dialog-button-danger={resetModeDraft === "hard"} class="dialog-button" type="button" on:click={confirmResetCommit} disabled={loading}>
            {loading ? t("resetDialog.resetting") : t("resetDialog.confirm")}
          </button>
        </div>
      </section>
    </div>
  {/if}

  {#if revertDialogOpen && revertTargetCommit}
    <div class="dialog-backdrop" role="presentation" on:click={(event) => event.target === event.currentTarget && !loading && resetRevertDialog()}>
      <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="revert-dialog-title">
        <div class="dialog-copy">
          <h2 id="revert-dialog-title">{t("revertDialog.title")}</h2>
          <p>{t("revertDialog.description", { id: revertTargetCommit.id })}</p>
        </div>

        <div class="dialog-warning">
          <span class="dialog-warning-label">{t("revertDialog.targetLabel")}</span>
          <code>{revertTargetCommit.id} {revertTargetCommit.summary ? `- ${revertTargetCommit.summary}` : ""}</code>
        </div>

        <label class="dialog-field">
          <span>{t("revertDialog.messageLabel")}</span>
          <textarea bind:value={revertMessageDraft} rows="3" disabled={loading}></textarea>
        </label>

        <div class="dialog-actions">
          <button class="dialog-button dialog-button-muted" type="button" on:click={resetRevertDialog} disabled={loading}>
            {t("revertDialog.cancel")}
          </button>
          <button class="dialog-button" type="button" on:click={confirmRevertCommit} disabled={loading || !revertMessageDraft.trim()}>
            {loading ? t("revertDialog.reverting") : t("revertDialog.confirm")}
          </button>
        </div>
      </section>
    </div>
  {/if}

  {#if tagDialogOpen}
    <div class="dialog-backdrop" role="presentation" on:click={(event) => event.target === event.currentTarget && !loading && resetTagDialog()}>
      <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="tag-dialog-title">
        <div class="dialog-copy">
          <h2 id="tag-dialog-title">{t("tagDialog.title")}</h2>
          <p>{t("tagDialog.description", { target: tagTargetLabel })}</p>
        </div>

        <div class="dialog-warning">
          <span class="dialog-warning-label">{t("tagDialog.targetLabel")}</span>
          <code>{tagTargetLabel}</code>
        </div>

        <label class="dialog-field">
          <span>{t("tagDialog.nameLabel")}</span>
          <input bind:value={tagNameDraft} placeholder={t("tagDialog.namePlaceholder")} disabled={loading} />
        </label>

        <label class="dialog-field">
          <span>{t("tagDialog.messageLabel")}</span>
          <textarea bind:value={tagMessageDraft} rows="4" placeholder={t("tagDialog.messagePlaceholder")} disabled={loading}></textarea>
        </label>

        <label class="dialog-checkbox">
          <input type="checkbox" bind:checked={tagPushAfterCreate} disabled={loading || !repository?.has_origin_remote} />
          <span>{t("tagDialog.pushAfterCreate")}</span>
        </label>

        {#if repository && !repository.has_origin_remote}
          <p class="dialog-helper">{t("tagDialog.pushUnavailable")}</p>
        {/if}

        <div class="dialog-actions">
          <button class="dialog-button dialog-button-muted" type="button" on:click={resetTagDialog} disabled={loading}>
            {t("tagDialog.cancel")}
          </button>
          <button class="dialog-button" type="button" on:click={createTag} disabled={loading}>
            {loading ? t("tagDialog.creating") : t("tagDialog.create")}
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

  {#if deleteTagDialogOpen && deleteTargetTagName}
    <div class="dialog-backdrop" role="presentation" on:click={(event) => event.target === event.currentTarget && !loading && resetDeleteTagDialog()}>
      <section class="dialog-card" role="dialog" aria-modal="true" aria-labelledby="delete-tag-dialog-title">
        <div class="dialog-copy">
          <h2 id="delete-tag-dialog-title">{t("tagDelete.title")}</h2>
          <p>{t("tagDelete.description", { tag: deleteTargetTagName })}</p>
        </div>

        <div class="dialog-warning">
          <span class="dialog-warning-label">{t("tagDelete.targetLabel")}</span>
          <code>{deleteTargetTagName}</code>
        </div>

        <label class="dialog-field">
          <span>{t("tagDelete.inputLabel")}</span>
          <input bind:value={deleteTagNameDraft} placeholder={deleteTargetTagName} disabled={loading} />
        </label>

        <p class="dialog-helper">{t("tagDelete.inputHint", { tag: deleteTargetTagName })}</p>

        <div class="dialog-actions">
          <button class="dialog-button dialog-button-muted" type="button" on:click={resetDeleteTagDialog} disabled={loading}>
            {t("tagDelete.cancel")}
          </button>
          <button
            class="dialog-button dialog-button-danger"
            type="button"
            on:click={confirmDeleteTag}
            disabled={loading || deleteTagNameDraft.trim() !== deleteTargetTagName}
          >
            {loading ? t("tagDelete.deleting") : t("tagDelete.delete")}
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
          <h2 id="push-dialog-title">{pushDialogTitle()}</h2>
          <p>{pushDialogDescription()}</p>
        </div>

        <div class="dialog-warning">
          <span class="dialog-warning-label">{t("push.targetLabel")}</span>
          <code>{pushDialogTarget()}</code>
        </div>

        {#if pendingPushAction?.payload?.allowForcePush}
          <label class="dialog-checkbox">
            <input type="checkbox" bind:checked={pushForceWithLease} disabled={loading || commitAndPushing} />
            <span>{t("push.forceOption")}</span>
          </label>
          <p class="dialog-helper">{t("push.forceHelper")}</p>
        {/if}

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
            {pushDialogConfirmLabel()}
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
    grid-template-columns: var(--left-pane-width, 246px) 8px minmax(0, 1fr) 8px var(--right-pane-width, 332px);
    overflow: hidden;
    transition: grid-template-columns 160ms ease;
  }

  .workspace.workspace-collapsed {
    grid-template-columns: var(--left-pane-width, 246px) 8px minmax(0, 1fr) 8px 54px;
  }

  .pane-resizer {
    position: relative;
    min-width: 8px;
    height: 100%;
    cursor: col-resize;
    touch-action: none;
  }

  .pane-resizer::before {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 1px;
    transform: translateX(-50%);
    background: color-mix(in srgb, var(--panel-border) 78%, transparent);
    transition: background 120ms ease, box-shadow 120ms ease;
  }

  .pane-resizer:hover::before {
    background: color-mix(in srgb, var(--accent) 50%, var(--panel-border));
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 16%, transparent);
  }

  .pane-resizer.pane-resizer-disabled {
    cursor: default;
    pointer-events: none;
  }

  .pane-resizer.pane-resizer-disabled::before {
    opacity: 0;
  }

  :global(body.pane-resizing) {
    cursor: col-resize;
    user-select: none;
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

  .dialog-field textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--surface-border);
    border-radius: 10px;
    background: var(--input-background);
    color: var(--text-secondary);
    padding: 11px 12px;
    resize: vertical;
    min-height: 96px;
  }

  .dialog-field input:focus {
    outline: none;
    border-color: var(--input-border-focus);
    box-shadow: var(--focus-ring);
    background: var(--input-background-focus);
  }

  .dialog-field textarea:focus {
    outline: none;
    border-color: var(--input-border-focus);
    box-shadow: var(--focus-ring);
    background: var(--input-background-focus);
  }

  .dialog-radio-group {
    display: grid;
    gap: 8px;
  }

  .dialog-radio-option {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 10px 12px;
    border: 1px solid var(--surface-border);
    border-radius: 10px;
    background: var(--surface-background);
  }

  .dialog-radio-option-danger {
    border-color: color-mix(in srgb, var(--danger-border) 65%, var(--surface-border));
  }

  .dialog-radio-option input[type="radio"] {
    margin-top: 2px;
  }

  .dialog-radio-copy {
    display: grid;
    gap: 3px;
  }

  .dialog-radio-copy strong {
    color: var(--text-primary);
    font-size: 0.82rem;
  }

  .dialog-radio-copy span {
    color: var(--text-secondary);
    font-size: 0.76rem;
    line-height: 1.4;
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

    .pane-resizer {
      display: none;
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
