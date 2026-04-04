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
  let rightPaneExpanded = false;
  let historyCommits = [];
  let historyLoading = false;
  let historyLoadedAll = false;
  let historyRequestId = 0;

  const topActions = ["Fetch", "Pull", "Push", "Stash", "Pop"];
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
    error = "";
    try {
      repository = await invoke("get_repository_status", {
        path: repository.repo_path,
      });
      rightPaneExpanded = false;
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
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
      void loadCommitHistory(repository.repo_path);
      return true;
    } catch (messageText) {
      error = String(messageText);
      return false;
    } finally {
      committing = false;
    }
  }

  function toggleRightPane() {
    rightPaneExpanded = !rightPaneExpanded;
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
</script>

<svelte:head>
  <title>{t("app.title")}</title>
</svelte:head>

<div class="app-shell">
  <TopBar {repository} {loading} {topActions} onRefresh={refreshRepository} />

  <main class:workspace-collapsed={!rightPaneExpanded} class="workspace">
    <SidebarPane {repository} {loading} onSelectRepository={selectRepository} />

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
      {committing}
      onToggle={toggleRightPane}
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
