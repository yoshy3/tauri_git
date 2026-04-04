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
  let rightPaneExpanded = false;
  let rightPaneTab = "commit";
  let historyCommits = [];
  let historyLoading = false;
  let historyLoadedAll = false;
  let historyRequestId = 0;

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

  async function openRepositoryAt(path, options = {}) {
    const { remember = true, resetPane = true, clearSavedOnError = false } = options;
    const trimmed = path.trim();
    if (!trimmed) {
      return;
    }

    error = "";
    repository = null;
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
      stashBusyAction={stashBusyAction}
      onSelectRepository={selectRepository}
      onSelectStash={(index) => (selectedStashIndex = index)}
      onCheckoutLocalBranch={(branchName) => checkoutBranch(branchName)}
      onCheckoutRemoteBranch={(remoteName, branchName) => checkoutBranch(branchName, remoteName)}
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
