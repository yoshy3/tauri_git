<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { confirm, open } from "@tauri-apps/plugin-dialog";
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
  let stashing = false;
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
  let branchDialogOpen = false;
  let branchNameDraft = "";
  let branchSwitchAfterCreate = true;
  let deleteDialogOpen = false;
  let deleteBranchNameDraft = "";
  let deleteTargetRef = null;
  let deleteForceEnabled = false;

  const topActions = ["Fetch", "Pull", "Push", "Stash"];
  const implementedTopActions = ["Fetch", "Pull", "Push", "Stash"];
  const lastRepositoryKey = "tauri-git:last-repository-path";
  const historyBatchSize = 100;

  function t(key, values) {
    return get(_)(key, { values });
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

  function closeBranchMenu() {
    branchMenuOpenKey = "";
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
    closeBranchMenu();
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
      closeBranchMenu();
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

  async function handleTopAction(action) {
    if (!repository || !implementedTopActions.includes(action)) {
      return;
    }

    if (action === "Stash") {
      rightPaneExpanded = true;
      rightPaneTab = "stash";
      return;
    }

    if (action === "Push" && !repository.has_origin_remote) {
      await confirm(t("push.missingOriginMessage"), {
        title: t("push.missingOriginTitle"),
        kind: "warning",
        okLabel: t("push.dialogOk"),
        cancelLabel: t("push.dialogCancel"),
      });
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
      } else if (action === "Push") {
        repository = await invoke("push_current_branch", {
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
    await checkoutSelectedRef();
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

  async function loadCommitHistory(path) {
    const requestId = historyRequestId + 1;
    historyRequestId = requestId;
    historyCommits = [];
    historyLoading = true;
    historyLoadedAll = false;

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
      }
    }
  }

  onMount(() => {
    const savedPath = localStorage.getItem(lastRepositoryKey);
    if (!savedPath) {
      return;
    }

    void openRepositoryAt(savedPath, { remember: false, clearSavedOnError: true });
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
    {topActions}
    implementedActions={implementedTopActions}
    activeAction={topbarBusyAction}
    onAction={handleTopAction}
    onRefresh={refreshRepository}
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
    />

    <CommitPane
      {repository}
      changedEntries={changedEntries}
      expanded={rightPaneExpanded}
      activeTab={rightPaneTab}
      {committing}
      {stashing}
      onToggle={toggleRightPane}
      onSelectTab={(tab) => (rightPaneTab = tab)}
      onStash={stashChanges}
      onCommit={commitChanges}
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
    background: rgba(4, 10, 16, 0.66);
    backdrop-filter: blur(8px);
    z-index: 20;
  }

  .dialog-card {
    width: min(100%, 420px);
    display: grid;
    gap: 14px;
    padding: 18px;
    border-radius: 14px;
    background: linear-gradient(180deg, rgba(11, 23, 36, 0.98), rgba(10, 21, 33, 0.97));
    border: 1px solid rgba(120, 148, 177, 0.14);
    box-shadow: 0 18px 48px rgba(0, 0, 0, 0.34);
  }

  .dialog-copy h2 {
    margin: 0;
    color: #f4f8fc;
    font-size: 1rem;
  }

  .dialog-copy p {
    margin: 6px 0 0;
    color: #9cb1c7;
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
    color: #cfe0f2;
    font-size: 0.78rem;
  }

  .dialog-field input {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid rgba(120, 148, 177, 0.14);
    border-radius: 10px;
    background: #040a10;
    color: #e8eef5;
    padding: 11px 12px;
  }

  .dialog-field input:focus {
    outline: none;
    border-color: rgba(84, 155, 233, 0.7);
    box-shadow: 0 0 0 3px rgba(35, 101, 168, 0.18);
    background: #06101a;
  }

  .dialog-checkbox {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dialog-checkbox input[type="checkbox"] {
    accent-color: #4da0ff;
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
    background: rgba(134, 63, 43, 0.12);
    border: 1px solid rgba(190, 104, 76, 0.2);
  }

  .dialog-warning-label,
  .dialog-helper {
    color: #cfe0f2;
    font-size: 0.74rem;
    line-height: 1.4;
  }

  .dialog-warning code {
    color: #ffd8cb;
    font-size: 0.8rem;
    word-break: break-word;
  }

  .dialog-button {
    border: 0;
    border-radius: 10px;
    background: linear-gradient(180deg, #1e68b0, #0d57a0);
    color: #eef5ff;
    padding: 10px 14px;
    font-size: 0.76rem;
    font-weight: 700;
    letter-spacing: 0.03em;
  }

  .dialog-button.dialog-button-muted {
    background: rgba(17, 30, 43, 0.92);
    color: #c8d6e4;
    border: 1px solid rgba(120, 148, 177, 0.12);
  }

  .dialog-button.dialog-button-danger {
    background: linear-gradient(180deg, #9a4a34, #7d3526);
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
