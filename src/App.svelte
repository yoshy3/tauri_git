<script>
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";

  let commitSummary = "";
  let commitDescription = "";
  let repository = null;
  let error = "";
  let loading = false;
  let committing = false;
  let rightPaneExpanded = false;
  let lastOpenedPath = "";
  let historyCommits = [];
  let historyLoading = false;
  let historyLoadedAll = false;
  let historyRequestId = 0;
  let sidebarFilter = "";
  let sidebarSections = {
    branches: true,
    remotes: true,
    tags: false,
    stashes: true,
    submodules: false,
  };
  let remoteSections = {};

  const topActions = ["Fetch", "Pull", "Push", "Stash", "Pop"];
  const lastRepositoryKey = "tauri-git:last-repository-path";
  const historyBatchSize = 100;
  const graphLaneSpacing = 14;
  const graphPadding = 10;
  const graphRowHeight = 32;
  const graphCenterY = 16;
  const graphOverlap = 3;
  const graphCurveInset = 5;
  const graphPalette = ["#67b3ff", "#ffb454", "#7be495", "#f7768e", "#c792ea", "#7dcfff"];

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
      lastOpenedPath = repository.repo_path;
      if (remember) {
        localStorage.setItem(lastRepositoryKey, repository.repo_path);
      }
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      if (remember || clearSavedOnError) {
        localStorage.removeItem(lastRepositoryKey);
        lastOpenedPath = "";
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
      title: "Open Git Repository",
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
      lastOpenedPath = repository.repo_path;
      rightPaneExpanded = false;
      void loadCommitHistory(repository.repo_path);
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
  }

  async function commitChanges() {
    if (!repository) {
      error = "先にリポジトリを読み込んでください。";
      return;
    }

    const summary = commitSummary.trim();
    const description = commitDescription.trim();
    if (!summary) {
      error = "コミット summary を入力してください。";
      return;
    }

    const message = description ? `${summary}\n\n${description}` : summary;

    committing = true;
    error = "";
    try {
      const updated = await invoke("commit_all", {
        path: repository.repo_path,
        message,
      });
      repository = updated;
      rightPaneExpanded = false;
      commitSummary = "";
      commitDescription = "";
      void loadCommitHistory(repository.repo_path);
    } catch (messageText) {
      error = String(messageText);
    } finally {
      committing = false;
    }
  }

  function statusLabel(entry) {
    const staged = entry.index_status === "." ? "" : entry.index_status;
    const unstaged = entry.working_tree_status === "." ? "" : entry.working_tree_status;

    if (staged && unstaged) {
      return `${staged}/${unstaged}`;
    }

    return staged || unstaged || "clean";
  }

  function initials(name) {
    return (name || "?")
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() || "")
      .join("");
  }

  function shortPath(path) {
    const parts = path.split("/");
    if (parts.length <= 2) {
      return path;
    }

    return `${parts.slice(0, 2).join("/")}/.../${parts.at(-1)}`;
  }

  function formatLocalDateTime(value) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return value;
    }

    return new Intl.DateTimeFormat(undefined, {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    }).format(date);
  }

  function toggleRightPane() {
    rightPaneExpanded = !rightPaneExpanded;
  }

  function toggleSidebarSection(section) {
    sidebarSections = {
      ...sidebarSections,
      [section]: !sidebarSections[section],
    };
  }

  function toggleRemoteSection(name) {
    remoteSections = {
      ...remoteSections,
      [name]: !isRemoteSectionExpanded(name),
    };
  }

  function setRemoteSection(name, expanded) {
    remoteSections = {
      ...remoteSections,
      [name]: expanded,
    };
  }

  function isRemoteSectionExpanded(name) {
    return remoteSections[name] ?? true;
  }

  function matchesSidebarFilter(value) {
    if (!sidebarFilterTerm) {
      return true;
    }

    return String(value || "").toLowerCase().includes(sidebarFilterTerm);
  }

  function dedupeLaneEntries(entries) {
    const seen = new Set();
    const result = [];

    entries.forEach((entry) => {
      if (seen.has(entry.oid)) {
        return;
      }

      seen.add(entry.oid);
      result.push(entry);
    });

    return result;
  }

  function buildVerticalPath(x, startY, endY) {
    return `M ${x} ${startY} L ${x} ${endY}`;
  }

  function buildLaneTransitionPath(startX, startY, endX, endY) {
    if (startX === endX) {
      return buildVerticalPath(startX, startY, endY);
    }

    const direction = endY > startY ? 1 : -1;
    const turnStartY = startY + graphCurveInset * direction;
    const turnEndY = endY - graphCurveInset * direction;
    const midY = (turnStartY + turnEndY) / 2;

    return [
      `M ${startX} ${startY}`,
      `L ${startX} ${turnStartY}`,
      `C ${startX} ${midY} ${endX} ${midY} ${endX} ${turnEndY}`,
      `L ${endX} ${endY}`,
    ].join(" ");
  }

  function buildGraphRows(commits) {
    let lanes = [];
    let maxLaneCount = 1;
    let colorCursor = 0;
    const rows = [];

    function nextColor() {
      const color = graphPalette[colorCursor % graphPalette.length];
      colorCursor += 1;
      return color;
    }

    commits.forEach((commit, rowIndex) => {
      const existingIndex = lanes.findIndex((lane) => lane.oid === commit.oid);
      const inserted = existingIndex === -1;
      const before = inserted
        ? [...lanes, { oid: commit.oid, color: nextColor() }]
        : lanes.map((lane) => ({ ...lane }));
      const laneIndex = inserted ? before.length - 1 : existingIndex;
      const nodeLane = before[laneIndex];

      const after = before.map((lane) => ({ ...lane }));
      if (commit.parent_ids.length === 0) {
        after.splice(laneIndex, 1);
      } else {
        after[laneIndex] = { oid: commit.parent_ids[0], color: nodeLane.color };
        for (let index = 1; index < commit.parent_ids.length; index += 1) {
          const parentId = commit.parent_ids[index];
          const existingParentLane = before.find((lane) => lane.oid === parentId);
          after.splice(laneIndex + index, 0, {
            oid: parentId,
            color: existingParentLane?.color ?? nextColor(),
          });
        }
      }

      const dedupedAfter = dedupeLaneEntries(after);
      const laneCount = Math.max(before.length, dedupedAfter.length, 1);
      maxLaneCount = Math.max(maxLaneCount, laneCount);
      const lines = [];
      const topY = -graphOverlap;
      const bottomY = graphRowHeight + graphOverlap;
      const nodeX = graphPadding + laneIndex * graphLaneSpacing;
      const nodeColor = nodeLane.color;

      before.forEach((lane, index) => {
        if (index === laneIndex) {
          return;
        }

        const currentX = graphPadding + index * graphLaneSpacing;
        const stroke = lane.color;
        if (commit.parent_ids.includes(lane.oid)) {
          lines.push({
            d: buildLaneTransitionPath(currentX, topY, nodeX, graphCenterY),
            stroke,
          });
          return;
        }

        const nextIndex = dedupedAfter.findIndex((nextLane) => nextLane.oid === lane.oid);

        if (nextIndex === -1) {
          lines.push({
            d: buildVerticalPath(currentX, topY, graphCenterY),
            stroke,
          });
          return;
        }

        const nextX = graphPadding + nextIndex * graphLaneSpacing;
        lines.push({
          d: buildLaneTransitionPath(currentX, topY, nextX, bottomY),
          stroke,
        });
      });

      if (!inserted) {
        lines.push({
          d: buildVerticalPath(nodeX, topY, graphCenterY),
          stroke: nodeColor,
        });
      }

      commit.parent_ids.forEach((parentId) => {
        const nextIndex = dedupedAfter.findIndex((lane) => lane.oid === parentId);
        if (nextIndex === -1) {
          return;
        }
        const nextX = graphPadding + nextIndex * graphLaneSpacing;
        const nextLaneColor = dedupedAfter[nextIndex].color;
        lines.push({
          d:
            nextX === nodeX
              ? buildVerticalPath(nodeX, graphCenterY, bottomY)
              : buildLaneTransitionPath(nodeX, graphCenterY, nextX, bottomY),
          stroke: nextLaneColor,
        });
      });

      lanes = dedupedAfter;

      rows.push({
        ...commit,
        graphLines: lines,
        graphNodeX: nodeX,
        graphNodeColor: nodeColor,
        graphIsHead: rowIndex === 0,
      });
    });

    const graphWidth = maxLaneCount * graphLaneSpacing + graphPadding * 2;
    return rows.map((row) => ({
      ...row,
      graphWidth,
    }));
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

    lastOpenedPath = savedPath;
    void openRepositoryAt(savedPath, { remember: false, clearSavedOnError: true });
  });

  $: changedEntries = repository ? repository.entries : [];
  $: sidebarFilterTerm = sidebarFilter.trim().toLowerCase();
  $: filteredLocalBranches = repository
    ? repository.local_branches.filter((branchName) => matchesSidebarFilter(branchName))
    : [];
  $: filteredRemoteGroups = repository
    ? repository.remote_groups
        .map((group) => {
          const branches = group.branches.filter((branchName) =>
            matchesSidebarFilter(`${group.name}/${branchName}`),
          );

          if (matchesSidebarFilter(group.name) || branches.length > 0) {
            return {
              ...group,
              branches,
            };
          }

          return null;
        })
        .filter(Boolean)
    : [];
  $: filteredTags = repository
    ? repository.tags.filter((tagName) => matchesSidebarFilter(tagName))
    : [];
  $: filteredStashes = repository
    ? repository.stashes.filter((stash) =>
        matchesSidebarFilter(`${stash.name} ${stash.message}`),
      )
    : [];
  $: filteredSubmodules = repository
    ? repository.submodules.filter((submodule) =>
        matchesSidebarFilter(`${submodule.name} ${submodule.path}`),
      )
    : [];
  $: historyGraphRows = buildGraphRows(historyCommits);
  $: historyGraphWidth = Math.max(
    historyGraphRows.length > 0 ? historyGraphRows[0].graphWidth : 0,
    56,
  );
</script>

<svelte:head>
  <title>Tauri Git</title>
</svelte:head>

<div class="app-shell">
  <header class="topbar">
    <div class="brand">
      <span class="brand-mark">G</span>
      <div>
        <strong>Tauri Git</strong>
        <p>Minimal Tauri client</p>
      </div>
    </div>

    <div class="toolbar">
      {#each topActions as action}
        <button class="toolbar-button" disabled={!repository}>
          {action}
        </button>
      {/each}
      <button class="toolbar-button active" on:click={refreshRepository} disabled={!repository || loading}>
        {loading ? "Syncing..." : "Refresh"}
      </button>
    </div>
  </header>

  <main class:workspace-collapsed={!rightPaneExpanded} class="workspace">
    <aside class="sidebar">
      <div class="sidebar-toolbar">
        <button class="sidebar-tool sidebar-tool-active" type="button" disabled>Refs</button>
        <button class="sidebar-tool" type="button" on:click={selectRepository} disabled={loading}>
          {loading ? "Opening..." : "Open"}
        </button>
      </div>

      <section class="sidebar-summary">
        {#if repository}
          <div class="sidebar-summary-top">
            <div class="sidebar-summary-copy">
              <h1>{repository.repo_name}</h1>
              <p class="sidebar-branch-name">{repository.branch}</p>
            </div>
            <span class:repo-status-clean={repository.is_clean} class="repo-status-indicator"></span>
          </div>
          <p class="path">{repository.repo_path}</p>
        {:else}
          <h1>No repository</h1>
          <p class="muted">フォルダ選択ダイアログから Git リポジトリを開いてください。</p>
        {/if}

      </section>

      <div class="sidebar-filter-wrap">
        <input
          class="sidebar-filter"
          placeholder="Filter"
          bind:value={sidebarFilter}
          disabled={!repository}
        />
      </div>

      <section class="sidebar-tree">
        {#if !repository}
          <p class="tree-empty">左上の Open からリポジトリを選択してください。</p>
        {:else}
          <div class="tree-section">
            <button class="tree-section-toggle" type="button" on:click={() => toggleSidebarSection("branches")}>
              <span class:expanded={sidebarSections.branches} class="tree-chevron"></span>
              <span>Branches</span>
            </button>

            {#if sidebarSections.branches}
              {#if filteredLocalBranches.length > 0}
                <ul class="tree-list tree-section-children">
                  {#each filteredLocalBranches as branchName}
                    <li class:tree-item-current={branchName === repository.branch} class="tree-item">
                      <span class="tree-item-icon tree-item-branch"></span>
                      <span class="tree-item-label">{branchName}</span>
                    </li>
                  {/each}
                </ul>
              {:else}
                <p class="tree-empty">表示できるブランチはありません。</p>
              {/if}
            {/if}
          </div>

          <div class="tree-section">
            <button class="tree-section-toggle" type="button" on:click={() => toggleSidebarSection("remotes")}>
              <span class:expanded={sidebarSections.remotes} class="tree-chevron"></span>
              <span>Remotes</span>
            </button>

            {#if sidebarSections.remotes}
              {#if filteredRemoteGroups.length > 0}
                <ul class="tree-list tree-section-children">
                  {#each filteredRemoteGroups as group}
                    <li class="tree-group">
                      <details
                        class:tree-group-expanded={isRemoteSectionExpanded(group.name)}
                        class="tree-group-details"
                        open={isRemoteSectionExpanded(group.name)}
                        on:toggle={(event) => setRemoteSection(group.name, event.currentTarget.open)}
                      >
                        <summary class="tree-group-toggle">
                          <span class="tree-chevron"></span>
                          <span class="tree-item-icon tree-item-remote"></span>
                          <span class="tree-item-label">{group.name}</span>
                        </summary>

                        {#if group.branches.length > 0}
                          <ul class="tree-list tree-nested-list">
                            {#each group.branches as branchName}
                              <li
                                class:tree-item-current={group.name === "origin" && branchName === repository.branch}
                                class="tree-item tree-item-nested"
                              >
                                <span class="tree-item-icon tree-item-branch tree-item-branch-muted"></span>
                                <span class="tree-item-label">{branchName}</span>
                              </li>
                            {/each}
                          </ul>
                        {:else}
                          <p class="tree-empty tree-empty-nested">一致するブランチはありません。</p>
                        {/if}
                      </details>
                    </li>
                  {/each}
                </ul>
              {:else}
                <p class="tree-empty">表示できるリモートはありません。</p>
              {/if}
            {/if}
          </div>

          <div class="tree-section">
            <button class="tree-section-toggle" type="button" on:click={() => toggleSidebarSection("tags")}>
              <span class:expanded={sidebarSections.tags} class="tree-chevron"></span>
              <span>Tags</span>
            </button>

            {#if sidebarSections.tags}
              {#if filteredTags.length > 0}
                <ul class="tree-list tree-section-children">
                  {#each filteredTags as tagName}
                    <li class="tree-item">
                      <span class="tree-item-icon tree-item-tag"></span>
                      <span class="tree-item-label">{tagName}</span>
                    </li>
                  {/each}
                </ul>
              {:else}
                <p class="tree-empty">表示できるタグはありません。</p>
              {/if}
            {/if}
          </div>

          <div class="tree-section">
            <button class="tree-section-toggle" type="button" on:click={() => toggleSidebarSection("stashes")}>
              <span class:expanded={sidebarSections.stashes} class="tree-chevron"></span>
              <span>Stashes</span>
            </button>

            {#if sidebarSections.stashes}
              {#if filteredStashes.length > 0}
                <ul class="tree-list tree-section-children">
                  {#each filteredStashes as stash}
                    <li class="tree-item tree-item-stack">
                      <span class="tree-item-icon tree-item-stash"></span>
                      <span class="tree-item-label">{stash.name}</span>
                      <span class="tree-item-detail">{stash.message}</span>
                    </li>
                  {/each}
                </ul>
              {:else}
                <p class="tree-empty">stash はありません。</p>
              {/if}
            {/if}
          </div>

          <div class="tree-section">
            <button class="tree-section-toggle" type="button" on:click={() => toggleSidebarSection("submodules")}>
              <span class:expanded={sidebarSections.submodules} class="tree-chevron"></span>
              <span>Submodules</span>
            </button>

            {#if sidebarSections.submodules}
              {#if filteredSubmodules.length > 0}
                <ul class="tree-list tree-section-children">
                  {#each filteredSubmodules as submodule}
                    <li class="tree-item tree-item-stack">
                      <span class="tree-item-icon tree-item-submodule"></span>
                      <span class="tree-item-label">{submodule.name}</span>
                      <span class="tree-item-detail">{submodule.path}</span>
                    </li>
                  {/each}
                </ul>
              {:else}
                <p class="tree-empty">submodule はありません。</p>
              {/if}
            {/if}
          </div>
        {/if}
      </section>
    </aside>

    <section class="center-pane">
      <div class="history-toolbar">
        <input
          class="search"
          placeholder="Search commits..."
          disabled={!repository}
        />
        <div class="history-meta">
          {#if repository}
            <span>{repository.branch}</span>
            <span>{historyCommits.length} commits</span>
            <span>{historyLoading ? "loading..." : historyLoadedAll ? "complete" : ""}</span>
          {/if}
        </div>
      </div>

      {#if repository && repository.head_summary}
        <div class="head-pill">HEAD {repository.head_summary}</div>
      {/if}

      {#if error}
        <div class="banner error-banner">{error}</div>
      {/if}

      <section class="history-table" style={`--graph-column-width: ${historyGraphWidth}px;`}>
        <div class="history-head">
          <span>Graph</span>
          <span>Subject</span>
          <span>Author</span>
          <span>Hash</span>
          <span>Date</span>
        </div>

        {#if repository && historyCommits.length > 0}
          <ul class="history-rows">
            {#each historyGraphRows as commit, index}
              <li class:muted-history-row={!commit.on_current_branch}>
                <div class="graph-cell">
                  <svg
                    class="graph-svg"
                    viewBox={`0 0 ${commit.graphWidth} ${graphRowHeight}`}
                    width={commit.graphWidth}
                    height={graphRowHeight}
                    preserveAspectRatio="xMinYMid meet"
                    aria-hidden="true"
                  >
                    {#each commit.graphLines as line}
                      <path
                        d={line.d}
                        stroke={line.stroke}
                        stroke-width="2"
                        fill="none"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                      />
                    {/each}
                    <circle
                      cx={commit.graphNodeX}
                      cy={graphCenterY}
                      r={commit.graphIsHead ? 4.5 : 3.5}
                      fill={commit.graphNodeColor}
                    />
                  </svg>
                </div>

                <div class="subject-cell">
                  {#if commit.labels.length > 0}
                    <div class="history-tags">
                      {#each commit.labels as label}
                        <span
                          class:history-tag-local={label.scope === "local"}
                          class:history-tag-remote={label.scope === "remote"}
                          class:history-tag-current={label.is_current}
                        >
                          {label.name}
                        </span>
                      {/each}
                    </div>
                  {/if}
                  <strong>{commit.summary}</strong>
                </div>

                <div class="author-cell">
                  <span class="avatar">{initials(commit.author)}</span>
                  <span>{commit.author}</span>
                </div>

                <div class="hash-cell">
                  <span>{commit.id}</span>
                </div>

                <div class="date-cell">
                  <span>{formatLocalDateTime(commit.committed_at)}</span>
                </div>
              </li>
            {/each}
          </ul>
        {:else if repository && historyLoading}
          <div class="empty-history">
            <p>コミット履歴を読み込み中です。</p>
            <p class="muted">読み込んだところから順次表示します。</p>
          </div>
        {:else}
          <div class="empty-history">
            <p>表示できるコミット履歴がありません。</p>
            <p class="muted">まずリポジトリを開くか、最初のコミットを作成してください。</p>
          </div>
        {/if}
      </section>
    </section>

    <aside class:collapsed-pane={!rightPaneExpanded} class="right-pane">
      <button
        class:attention={changedEntries.length > 0}
        class="pane-toggle"
        on:click={toggleRightPane}
      >
        <span class="pane-toggle-label">
          {rightPaneExpanded ? "Close Commit Panel" : changedEntries.length > 0 ? `Open Commit Panel (${changedEntries.length})` : "Open Commit Panel"}
        </span>
      </button>

      {#if rightPaneExpanded}
      <section class="changes-panel">
        <div class="changes-summary">
          <div class="changes-summary-copy">
            <h2>Commit Files ({changedEntries.length})</h2>
            <p class="changes-caption">変更は自動でステージしてまとめてコミットします</p>
          </div>
        </div>

        <div class="changes-group unified">
          {#if changedEntries.length > 0}
            <ul>
              {#each changedEntries as entry}
                <li>
                  <span class:file-status={true} class:warning={entry.working_tree_status !== "."} class:ok={entry.working_tree_status === "."}>
                    {statusLabel(entry)}
                  </span>
                  <div>
                    <strong>{shortPath(entry.path)}</strong>
                    <p>{entry.path}</p>
                  </div>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-side">コミット対象の変更はありません。</p>
          {/if}
        </div>
      </section>

      <section class="commit-panel">
        <label>
          <span>Summary (required)</span>
          <input bind:value={commitSummary} placeholder="Short commit summary" />
        </label>

        <label>
          <span>Description</span>
          <textarea bind:value={commitDescription} rows="6" placeholder="Optional longer description"></textarea>
        </label>

        <button
          class="primary wide"
          on:click={commitChanges}
          disabled={!repository || committing || repository.is_clean}
        >
          {committing ? "Committing..." : `Commit to ${repository ? repository.branch : "branch"}`}
        </button>
      </section>
      {/if}
    </aside>
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    background:
      radial-gradient(circle at top left, rgba(18, 49, 82, 0.5), transparent 22%),
      radial-gradient(circle at bottom right, rgba(16, 38, 64, 0.35), transparent 24%),
      linear-gradient(180deg, #07111a 0%, #0a1621 100%);
    color: #d5deea;
    overflow: hidden;
  }

  :global(html),
  :global(body),
  :global(#app) {
    height: 100%;
  }

  .app-shell {
    height: 100vh;
    display: grid;
    grid-template-rows: 60px 1fr;
    overflow: hidden;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 14px 0 10px;
    border-bottom: 1px solid rgba(114, 144, 175, 0.1);
    background: linear-gradient(180deg, rgba(6, 14, 23, 0.98), rgba(8, 17, 27, 0.93));
    backdrop-filter: blur(20px);
    box-shadow: inset 0 -1px 0 rgba(255, 255, 255, 0.02);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .brand strong {
    display: block;
    font-size: 1.08rem;
    letter-spacing: 0.01em;
  }

  .brand p {
    margin: 2px 0 0;
    color: #6f859c;
    font-size: 0.75rem;
  }

  .brand-mark {
    width: 32px;
    height: 32px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, #1f5b94, #4ca4ff);
    color: white;
    font-weight: 700;
    box-shadow: 0 8px 18px rgba(32, 108, 184, 0.28);
  }

  .toolbar {
    display: flex;
    gap: 2px;
    flex-wrap: wrap;
  }

  .toolbar-button {
    background: transparent;
    border: 0;
    color: #8aa0b8;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 0.72rem;
    padding: 10px 12px;
    border-radius: 8px;
  }

  .toolbar-button:hover:enabled {
    background: rgba(255, 255, 255, 0.03);
    color: #dce8f4;
  }

  .toolbar-button.active {
    color: #f2f7fb;
    background: rgba(32, 84, 138, 0.22);
    box-shadow: inset 0 -2px 0 #4da0ff;
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

  .sidebar,
  .center-pane,
  .right-pane {
    min-height: 0;
  }

  .sidebar {
    padding: 8px;
    border-right: 1px solid rgba(114, 144, 175, 0.06);
    background: linear-gradient(180deg, rgba(10, 17, 26, 0.98), rgba(8, 15, 23, 0.96));
    display: grid;
    grid-template-rows: auto auto auto 1fr;
    gap: 8px;
    overflow: hidden;
  }

  .changes-panel,
  .commit-panel,
  .history-table {
    background: linear-gradient(180deg, rgba(11, 23, 36, 0.98), rgba(11, 22, 34, 0.95));
    border: 1px solid rgba(120, 148, 177, 0.08);
    border-radius: 10px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
  }

  .sidebar-summary,
  .sidebar-tree,
  .commit-panel {
    padding: 10px;
  }

  h1,
  h2 {
    margin: 0;
    font-size: 1rem;
    color: #f4f8fc;
  }

  .muted,
  .path,
  .empty-side {
    color: #6c849c;
  }

  .path {
    font-size: 0.72rem;
    line-height: 1.35;
    word-break: break-all;
  }

  .commit-panel label {
    display: grid;
    gap: 5px;
  }

  .commit-panel span {
    color: #8aa0b8;
    font-size: 0.78rem;
  }

  input,
  textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid rgba(120, 148, 177, 0.12);
    border-radius: 8px;
    background: #040a10;
    color: #e8eef5;
    padding: 11px 12px;
    transition: border-color 120ms ease, box-shadow 120ms ease, background 120ms ease;
  }

  input:focus,
  textarea:focus {
    outline: none;
    border-color: rgba(84, 155, 233, 0.7);
    box-shadow: 0 0 0 3px rgba(35, 101, 168, 0.18);
    background: #06101a;
  }

  textarea {
    resize: vertical;
  }

  .primary {
    border: 0;
    border-radius: 8px;
    background: linear-gradient(180deg, #1e68b0, #0d57a0);
    color: #eef5ff;
    font-weight: 700;
    letter-spacing: 0.03em;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.12);
  }

  .wide {
    width: 100%;
    padding: 10px 12px;
    margin-top: 8px;
  }

  .history-rows,
  .changes-group ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .sidebar-toolbar {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
  }

  .sidebar-tool {
    border: 1px solid rgba(120, 148, 177, 0.12);
    border-radius: 10px;
    background: rgba(12, 23, 35, 0.82);
    color: #95a7ba;
    padding: 9px 12px;
    font-size: 0.76rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .sidebar-tool-active {
    background: rgba(18, 37, 56, 0.9);
    color: #eef5fb;
    box-shadow: inset 0 -2px 0 #4da0ff;
  }

  .sidebar-summary,
  .sidebar-tree {
    background: linear-gradient(180deg, rgba(11, 23, 36, 0.98), rgba(11, 22, 34, 0.95));
    border: 1px solid rgba(120, 148, 177, 0.08);
    border-radius: 10px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
  }

  .sidebar-summary {
    display: grid;
    gap: 4px;
  }

  .sidebar-summary-top {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    align-items: start;
  }

  .sidebar-summary-copy {
    min-width: 0;
  }

  .sidebar-summary h1 {
    font-size: 1rem;
    line-height: 1.15;
  }

  .sidebar-branch-name {
    margin: 4px 0 0;
    color: #d8e6f3;
    font-size: 0.84rem;
    font-weight: 600;
  }

  .repo-status-indicator {
    width: 10px;
    height: 10px;
    flex-shrink: 0;
    margin-top: 4px;
    border-radius: 999px;
    background: #f2a95c;
    box-shadow: 0 0 0 1px rgba(242, 169, 92, 0.16);
  }

  .repo-status-indicator.repo-status-clean {
    background: #63d39c;
    box-shadow: 0 0 0 1px rgba(99, 211, 156, 0.16);
  }

  .sidebar-filter-wrap {
    min-height: 0;
  }

  .sidebar-filter {
    border-radius: 10px;
    padding: 10px 12px;
    background: rgba(5, 10, 16, 0.92);
  }

  .sidebar-tree {
    min-height: 0;
    overflow: auto;
    display: grid;
    gap: 0;
    align-content: start;
  }

  .tree-section {
    display: grid;
    gap: 0;
  }

  .tree-section-toggle,
  .tree-group-toggle {
    width: 100%;
    border: 0;
    background: transparent;
    color: #eef5fb;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 4px;
    text-align: left;
    font-size: 0.78rem;
    font-weight: 700;
    letter-spacing: 0.01em;
    cursor: pointer;
  }

  .tree-group-toggle {
    color: #d9e5f1;
    font-size: 0.77rem;
    font-weight: 600;
    padding-left: 0;
    border-radius: 8px;
    min-height: 24px;
  }

  .tree-group-toggle:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .tree-chevron {
    width: 7px;
    height: 7px;
    flex-shrink: 0;
    border-right: 2px solid #7f93a8;
    border-bottom: 2px solid #7f93a8;
    transform: rotate(-45deg) translateY(-1px);
    transform-origin: 50% 50%;
    transition: transform 160ms ease;
  }

  .tree-chevron.expanded {
    transform: rotate(45deg) translateY(-1px);
  }

  .tree-group-details[open] .tree-chevron {
    transform: rotate(45deg) translateY(-1px);
  }

  .tree-list {
    display: grid;
    gap: 0;
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .tree-section-children {
    margin-left: 12px;
  }

  .tree-nested-list {
    margin-left: 12px;
  }

  .tree-group {
    display: grid;
    gap: 0;
  }

  .tree-group-details {
    display: grid;
    gap: 0;
  }

  .tree-group-details > summary {
    list-style: none;
  }

  .tree-group-details > summary::-webkit-details-marker {
    display: none;
  }

  .tree-group-expanded > .tree-group-toggle,
  .tree-group-details.tree-group-expanded > .tree-group-toggle {
    color: #eef5fb;
  }

  .tree-item {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 8px;
    align-items: center;
    min-height: 24px;
    padding: 3px 8px 3px 4px;
    border-radius: 8px;
    color: #abbcce;
    font-size: 0.78rem;
    box-sizing: border-box;
  }

  .tree-item.tree-item-current {
    background: rgba(255, 255, 255, 0.08);
    color: #f8fbff;
    font-weight: 700;
  }

  .tree-item-nested {
    margin-left: 0;
    padding-left: 0;
  }

  .tree-item-stack {
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    padding-top: 3px;
    padding-bottom: 3px;
  }

  .tree-item-icon {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    border-radius: 999px;
    margin-top: 1px;
  }

  .tree-item-branch {
    background: linear-gradient(135deg, #77b7ff, #4a9cff);
    box-shadow: 0 0 0 1px rgba(110, 172, 234, 0.14);
  }

  .tree-item-branch-muted {
    opacity: 0.62;
  }

  .tree-item-remote {
    border-radius: 3px;
    background: linear-gradient(135deg, #8f98a3, #c0cad6);
    box-shadow: 0 0 0 1px rgba(150, 163, 179, 0.14);
  }

  .tree-item-tag {
    border-radius: 4px;
    background: linear-gradient(135deg, #f1c56f, #f09a42);
    box-shadow: 0 0 0 1px rgba(239, 169, 72, 0.14);
  }

  .tree-item-stash {
    background: linear-gradient(135deg, #9b8eff, #7f71eb);
    box-shadow: 0 0 0 1px rgba(144, 130, 241, 0.14);
  }

  .tree-item-submodule {
    border-radius: 3px;
    background: linear-gradient(135deg, #6fd9d3, #4ca9ba);
    box-shadow: 0 0 0 1px rgba(88, 178, 187, 0.14);
  }

  .tree-item-label,
  .tree-item-detail {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tree-item-detail {
    grid-column: 2;
    color: #6f859c;
    font-size: 0.69rem;
    display: none;
  }

  .tree-empty {
    margin: 0;
    padding: 0 4px 2px 12px;
    color: #6f859c;
    font-size: 0.72rem;
    line-height: 1.35;
  }

  .tree-empty.tree-empty-nested {
    padding-left: 24px;
  }

  .center-pane {
    padding: 12px;
    display: grid;
    grid-template-rows: auto auto 1fr;
    gap: 10px;
    min-width: 0;
    min-height: 0;
    background: linear-gradient(180deg, rgba(11, 22, 34, 0.6), rgba(10, 20, 31, 0.36));
    overflow: hidden;
  }

  .history-toolbar {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: center;
    padding: 2px 2px 0;
  }

  .search {
    max-width: 520px;
    border-radius: 6px;
    background: #04080d;
  }

  .history-meta {
    display: flex;
    gap: 12px;
    color: #688099;
    font-size: 0.85rem;
  }

  .head-pill {
    align-self: start;
    padding: 6px 10px;
    border-radius: 999px;
    background: rgba(23, 44, 66, 0.76);
    color: #a9c1d8;
    font-size: 0.76rem;
    border: 1px solid rgba(96, 132, 169, 0.14);
  }

  .banner {
    border-radius: 10px;
    padding: 12px 14px;
  }

  .error-banner {
    background: rgba(132, 32, 38, 0.18);
    color: #ffb8bc;
    border: 1px solid rgba(198, 84, 90, 0.18);
  }

  .history-table {
    overflow: hidden;
    display: grid;
    grid-template-rows: auto 1fr;
    min-height: 0;
    border-radius: 8px;
  }

  .history-head,
  .history-rows li {
    display: grid;
    grid-template-columns: var(--graph-column-width, 108px) minmax(0, 1.5fr) minmax(140px, 0.8fr) 92px 132px;
    gap: 14px;
    align-items: center;
  }

  .history-head {
    padding: 4px 12px;
    color: #60788f;
    font-size: 0.64rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    border-bottom: 1px solid rgba(120, 148, 177, 0.06);
    background: rgba(8, 16, 25, 0.55);
  }

  .history-rows {
    overflow: auto;
  }

  .history-rows li {
    padding: 0 12px;
    border-bottom: 1px solid rgba(120, 148, 177, 0.04);
    transition: background 120ms ease;
    height: 32px;
    box-sizing: border-box;
  }

  .history-rows li:hover {
    background: rgba(255, 255, 255, 0.02);
  }

  .history-rows li.muted-history-row {
    background: rgba(255, 255, 255, 0.01);
  }

  .graph-cell {
    position: relative;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    overflow: visible;
  }

  .graph-svg {
    display: block;
    overflow: visible;
    flex: 0 0 auto;
  }

  .subject-cell {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    height: 100%;
  }

  .subject-cell strong {
    display: block;
    color: #eef5fb;
    line-height: 1.2;
    font-size: 0.78rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .muted-history-row .subject-cell strong,
  .muted-history-row .author-cell,
  .muted-history-row .hash-cell,
  .muted-history-row .date-cell {
    color: #627388;
  }

  .history-tags {
    display: flex;
    gap: 3px;
    margin-top: 0;
    flex-shrink: 0;
    align-items: center;
  }

  .history-tags span {
    padding: 1px 7px;
    border-radius: 999px;
    background: rgba(43, 71, 98, 0.72);
    color: #c7d9eb;
    font-size: 0.68rem;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .muted-history-row .history-tags span {
    background: rgba(43, 71, 98, 0.32);
    color: #8ca1b8;
  }

  .history-tags span.history-tag-local {
    background: rgba(144, 92, 14, 0.22);
    color: #ffd48a;
    box-shadow: inset 0 0 0 1px rgba(218, 146, 33, 0.35);
  }

  .history-tags span.history-tag-remote {
    background: rgba(60, 74, 94, 0.5);
    color: #d4dde7;
    box-shadow: inset 0 0 0 1px rgba(114, 138, 167, 0.2);
  }

  .history-tags span.history-tag-current {
    background: rgba(164, 102, 11, 0.28);
    color: #fff4d9;
    box-shadow: inset 0 0 0 1px rgba(241, 170, 55, 0.75);
    font-weight: 700;
  }

  .author-cell {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 0.74rem;
    overflow: hidden;
    height: 100%;
  }

  .author-cell span:last-child {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .avatar {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
    border-radius: 999px;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, #2b6aa8, #70b8ff);
    color: #fff;
    font-size: 0.5rem;
    font-weight: 700;
  }

  .muted-history-row .avatar {
    background: linear-gradient(135deg, #44566d, #738399);
    color: #dbe4ed;
  }

  .hash-cell {
    display: flex;
    align-items: center;
    color: #7990a7;
    font-size: 0.68rem;
    overflow: hidden;
    height: 100%;
  }

  .hash-cell span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .date-cell {
    display: flex;
    align-items: center;
    color: #7990a7;
    font-size: 0.68rem;
    overflow: hidden;
    height: 100%;
  }

  .date-cell span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .empty-history {
    padding: 28px 20px;
  }

  .right-pane {
    padding: 12px 12px 12px 0;
    display: grid;
    grid-template-rows: auto 1fr;
    gap: 10px;
    overflow: hidden;
    width: 332px;
    justify-self: end;
    transition: width 160ms ease, padding 160ms ease;
  }

  .right-pane.collapsed-pane {
    width: 54px;
    padding-left: 0;
    padding-right: 8px;
  }

  .pane-toggle {
    border: 1px solid rgba(120, 148, 177, 0.16);
    border-radius: 12px;
    background: rgba(12, 24, 38, 0.96);
    color: #c7d7e8;
    min-height: 54px;
    padding: 10px 12px;
    text-align: left;
    font-size: 0.76rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    writing-mode: horizontal-tb;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
  }

  .collapsed-pane .pane-toggle {
    height: 100%;
    min-height: 160px;
    padding: 14px 8px;
    writing-mode: vertical-rl;
    text-orientation: mixed;
    justify-self: stretch;
    justify-content: center;
    border-radius: 10px;
  }

  .pane-toggle.attention {
    background: linear-gradient(180deg, rgba(30, 104, 176, 0.92), rgba(13, 87, 160, 0.92));
    border-color: rgba(104, 173, 244, 0.42);
    color: #eef6ff;
    box-shadow: 0 0 0 1px rgba(88, 161, 237, 0.16), 0 8px 20px rgba(15, 88, 160, 0.28);
  }

  .pane-toggle-label {
    display: inline-block;
  }

  .changes-panel {
    display: grid;
    grid-template-rows: auto 1fr;
    overflow: hidden;
  }

  .changes-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 12px;
    min-height: 68px;
    box-sizing: border-box;
  }

  .changes-summary-copy {
    min-width: 0;
  }

  .changes-summary h2 {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .changes-caption {
    margin: 4px 0 0;
    color: #6f859c;
    font-size: 0.76rem;
    line-height: 1.3;
  }

  .changes-group {
    padding: 12px;
    overflow: auto;
  }

  .changes-group.unified {
    border-top: 1px solid rgba(120, 148, 177, 0.08);
  }

  .changes-group li {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px;
    align-items: start;
    padding: 10px 2px;
    border-bottom: 1px solid rgba(120, 148, 177, 0.04);
  }

  .changes-group strong {
    color: #eef5fb;
    display: block;
    font-size: 0.9rem;
  }

  .changes-group p {
    margin: 2px 0 0;
    color: #6f859c;
    font-size: 0.78rem;
    word-break: break-all;
  }

  .file-status {
    min-width: 34px;
    border-radius: 999px;
    padding: 4px 8px;
    text-align: center;
    font-size: 0.68rem;
    font-weight: 700;
  }

  .file-status.warning {
    background: rgba(232, 162, 74, 0.18);
    color: #ffc46e;
  }

  .file-status.ok {
    background: rgba(70, 144, 92, 0.2);
    color: #8fdfab;
  }

  .commit-panel {
    display: grid;
    gap: 12px;
    border-radius: 8px;
    overflow: auto;
  }

  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  button {
    transition: background 120ms ease, color 120ms ease, opacity 120ms ease, transform 120ms ease;
  }

  .primary:hover:enabled,
  .pane-toggle:hover:enabled {
    transform: translateY(-1px);
  }

  .primary:hover:enabled {
    background: linear-gradient(180deg, #2673bf, #1160ae);
  }

  @media (max-width: 1180px) {
    .workspace {
      grid-template-columns: 220px minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) auto;
    }

    .workspace.workspace-collapsed {
      grid-template-columns: 220px minmax(0, 1fr);
    }

    .right-pane {
      grid-column: 1 / -1;
      padding: 0 16px 16px;
      grid-template-columns: 1fr 320px;
      grid-template-rows: auto;
      overflow: auto;
      width: auto;
    }

    .right-pane.collapsed-pane {
      width: auto;
      padding-left: 16px;
    }

    .collapsed-pane .pane-toggle {
      min-height: 54px;
      writing-mode: horizontal-tb;
      text-orientation: initial;
    }
  }

  @media (max-width: 860px) {
    .app-shell {
      height: auto;
      min-height: 100vh;
      grid-template-rows: auto 1fr;
      overflow: auto;
    }

    .topbar,
    .history-toolbar {
      align-items: stretch;
      flex-direction: column;
    }

    .workspace {
      grid-template-columns: 1fr;
      height: auto;
      overflow: visible;
    }

    .right-pane,
    .center-pane,
    .sidebar {
      padding: 14px;
    }

    .right-pane {
      grid-template-columns: 1fr;
      overflow: visible;
    }

    .history-head,
    .history-rows li {
      grid-template-columns: 84px minmax(0, 1fr);
    }

    .history-rows li {
      height: 30px;
      padding: 0 10px;
    }

    .history-head span:nth-child(3),
    .history-head span:nth-child(4),
    .history-head span:nth-child(5),
    .author-cell,
    .hash-cell,
    .date-cell {
      display: none;
    }
  }
</style>
