<script>
  import { invoke } from "@tauri-apps/api/core";

  let repoPath = "";
  let commitSummary = "";
  let commitDescription = "";
  let repository = null;
  let error = "";
  let loading = false;
  let committing = false;

  const topActions = ["Fetch", "Pull", "Push", "Stash", "Pop"];
  const navItems = ["Branches", "Remotes", "Tags", "Stashes"];

  async function loadRepository() {
    error = "";
    repository = null;

    const trimmed = repoPath.trim();
    if (!trimmed) {
      error = "Git リポジトリのパスを入力してください。";
      return;
    }

    loading = true;
    try {
      repository = await invoke("open_repository", { path: trimmed });
      repoPath = repository.repo_path;
    } catch (message) {
      error = String(message);
    } finally {
      loading = false;
    }
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
      commitSummary = "";
      commitDescription = "";
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

  function selectEntries(kind) {
    if (!repository) {
      return [];
    }

    return repository.entries.filter((entry) =>
      kind === "staged" ? entry.index_status !== "." : entry.working_tree_status !== ".",
    );
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

  $: stagedEntries = selectEntries("staged");
  $: unstagedEntries = selectEntries("unstaged");
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

  <main class="workspace">
    <aside class="sidebar">
      <section class="repo-card">
        <p class="sidebar-label">Repository</p>
        {#if repository}
          <div class="repo-icon-row">
            <span class="repo-icon"></span>
            <span class="repo-chip">mainline</span>
          </div>
          <h1>{repository.repo_name}</h1>
          <p class="muted">{repository.branch}</p>
          <p class="path">{repository.repo_path}</p>
        {:else}
          <h1>No repository</h1>
          <p class="muted">ローカル repo を開いてください</p>
        {/if}
      </section>

      <section class="open-panel">
        <label>
          <span>Path</span>
          <input
            bind:value={repoPath}
            placeholder="/path/to/repository"
            on:keydown={(event) => event.key === "Enter" && loadRepository()}
          />
        </label>
        <button class="primary wide" on:click={loadRepository} disabled={loading}>
          {loading ? "Opening..." : "Open Repository"}
        </button>
      </section>

      <button class="new-branch" disabled={!repository}>New Branch</button>

      <nav class="side-nav">
        {#each navItems as item}
          <button class:item-active={item === "Branches"}>
            {item}
          </button>
        {/each}
      </nav>

      <section class="branch-panel">
        <div class="panel-heading">
          <p class="sidebar-label">Local</p>
        </div>

        {#if repository && repository.local_branches.length > 0}
          <ul class="branch-list">
            {#each repository.local_branches as branchName}
              <li class:current-branch={branchName === repository.branch}>
                <span class="branch-dot"></span>
                <span>{branchName}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="empty-side">ブランチ情報はまだありません。</p>
        {/if}
      </section>

      <footer class="sidebar-footer">
        <button>Settings</button>
        <button>Terminal</button>
      </footer>
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
            <span>{repository.recent_commits.length} commits</span>
          {/if}
        </div>
      </div>

      {#if repository && repository.head_summary}
        <div class="head-pill">HEAD {repository.head_summary}</div>
      {/if}

      {#if error}
        <div class="banner error-banner">{error}</div>
      {/if}

      <section class="history-table">
        <div class="history-head">
          <span>Graph</span>
          <span>Subject</span>
          <span>Author</span>
          <span>Hash</span>
          <span>Date</span>
        </div>

        {#if repository && repository.recent_commits.length > 0}
          <ul class="history-rows">
            {#each repository.recent_commits as commit, index}
              <li>
                <div class="graph-cell">
                  <span class="graph-line"></span>
                  <span class:graph-node-main={index === 0} class="graph-node"></span>
                  {#if index < repository.recent_commits.length - 1}
                    <span class="graph-tail"></span>
                  {/if}
                </div>

                <div class="subject-cell">
                  <strong>{commit.summary}</strong>
                  {#if index === 0}
                    <div class="history-tags">
                      <span>main</span>
                      <span>HEAD</span>
                    </div>
                  {/if}
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
        {:else}
          <div class="empty-history">
            <p>表示できるコミット履歴がありません。</p>
            <p class="muted">まずリポジトリを開くか、最初のコミットを作成してください。</p>
          </div>
        {/if}
      </section>
    </section>

    <aside class="right-pane">
      <section class="changes-panel">
        <div class="changes-group">
          <div class="changes-head">
            <h2>Unstaged ({unstagedEntries.length})</h2>
            <button disabled>Stage all</button>
          </div>

          {#if unstagedEntries.length > 0}
            <ul>
              {#each unstagedEntries as entry}
                <li>
                  <span class="file-status warning">{statusLabel(entry)}</span>
                  <div>
                    <strong>{shortPath(entry.path)}</strong>
                    <p>{entry.path}</p>
                  </div>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-side">未ステージの変更はありません。</p>
          {/if}
        </div>

        <div class="changes-group">
          <div class="changes-head">
            <h2>Staged ({stagedEntries.length})</h2>
            <button disabled>Unstage all</button>
          </div>

          {#if stagedEntries.length > 0}
            <ul>
              {#each stagedEntries as entry}
                <li>
                  <span class="file-status ok">{statusLabel(entry)}</span>
                  <div>
                    <strong>{shortPath(entry.path)}</strong>
                    <p>{entry.path}</p>
                  </div>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-side">ステージ済みの変更はありません。</p>
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

  .toolbar-button,
  .side-nav button,
  .sidebar-footer button,
  .changes-head button {
    background: transparent;
    border: 0;
    color: #8aa0b8;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 0.72rem;
    padding: 10px 12px;
    border-radius: 8px;
  }

  .toolbar-button:hover:enabled,
  .side-nav button:hover:enabled,
  .sidebar-footer button:hover:enabled {
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
  }

  .sidebar,
  .center-pane,
  .right-pane {
    min-height: 0;
  }

  .sidebar {
    padding: 8px 8px 10px;
    border-right: 1px solid rgba(114, 144, 175, 0.06);
    background: linear-gradient(180deg, rgba(8, 18, 29, 0.96), rgba(8, 16, 25, 0.92));
    display: grid;
    grid-template-rows: auto auto auto auto 1fr auto;
    gap: 8px;
    overflow: auto;
  }

  .repo-card,
  .open-panel,
  .branch-panel,
  .changes-panel,
  .commit-panel,
  .history-table {
    background: linear-gradient(180deg, rgba(11, 23, 36, 0.98), rgba(11, 22, 34, 0.95));
    border: 1px solid rgba(120, 148, 177, 0.08);
    border-radius: 10px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
  }

  .repo-card,
  .open-panel,
  .branch-panel,
  .commit-panel {
    padding: 10px;
  }

  .sidebar-label {
    margin: 0 0 5px;
    color: #5f7891;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }

  h1,
  h2 {
    margin: 0;
    font-size: 1rem;
    color: #f4f8fc;
  }

  .repo-card h1 {
    font-size: 1.08rem;
    margin-top: 5px;
  }

  .muted,
  .path,
  .empty-side {
    color: #6c849c;
  }

  .path {
    font-size: 0.75rem;
    line-height: 1.35;
    word-break: break-all;
  }

  .repo-icon-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .repo-icon {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    background: linear-gradient(180deg, #3d6ea3, #1d4875);
    box-shadow: 0 0 0 1px rgba(126, 164, 201, 0.18);
  }

  .repo-chip {
    padding: 2px 7px;
    border-radius: 999px;
    background: rgba(55, 88, 119, 0.45);
    color: #bcd0e5;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .open-panel label,
  .commit-panel label {
    display: grid;
    gap: 5px;
  }

  .open-panel span,
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

  .primary,
  .new-branch {
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

  .new-branch {
    padding: 10px 12px;
    text-transform: uppercase;
    font-size: 0.78rem;
    border-radius: 6px;
  }

  .side-nav {
    display: grid;
    gap: 2px;
  }

  .side-nav button {
    text-align: left;
    padding: 8px 10px;
    border-radius: 6px;
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.9rem;
    position: relative;
  }

  .side-nav button.item-active {
    background: linear-gradient(180deg, rgba(25, 74, 126, 0.5), rgba(20, 62, 106, 0.45));
    color: #f1f7ff;
    box-shadow: inset 3px 0 0 #4da0ff;
  }

  .branch-panel {
    display: grid;
    gap: 6px;
  }

  .branch-list,
  .history-rows,
  .changes-group ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .branch-list {
    display: grid;
    gap: 2px;
  }

  .branch-list li {
    display: flex;
    align-items: center;
    gap: 9px;
    color: #a8bbcf;
    font-size: 0.87rem;
    padding: 3px 2px;
  }

  .branch-list li.current-branch {
    color: #f5f9fd;
  }

  .branch-dot {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: #4da0ff;
  }

  .sidebar-footer {
    display: grid;
    gap: 2px;
    align-self: end;
  }

  .sidebar-footer button {
    text-align: left;
    padding: 6px 4px;
    font-size: 0.82rem;
    letter-spacing: 0;
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
    grid-template-columns: 72px minmax(0, 1.5fr) minmax(140px, 0.8fr) 92px 132px;
    gap: 14px;
    align-items: center;
  }

  .history-head {
    padding: 8px 14px;
    color: #60788f;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    border-bottom: 1px solid rgba(120, 148, 177, 0.06);
    background: rgba(8, 16, 25, 0.55);
  }

  .history-rows {
    overflow: auto;
  }

  .history-rows li {
    padding: 8px 14px;
    border-bottom: 1px solid rgba(120, 148, 177, 0.04);
    transition: background 120ms ease;
    height: 40px;
    box-sizing: border-box;
    overflow: hidden;
  }

  .history-rows li:hover {
    background: rgba(255, 255, 255, 0.02);
  }

  .graph-cell {
    position: relative;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .graph-line,
  .graph-tail {
    position: absolute;
    width: 2px;
    background: rgba(110, 143, 176, 0.38);
    left: calc(50% - 1px);
  }

  .graph-line {
    top: -8px;
    height: 10px;
  }

  .graph-tail {
    top: 16px;
    bottom: -8px;
  }

  .graph-node {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: #ffb54a;
    z-index: 1;
    box-shadow: 0 0 0 4px rgba(255, 181, 74, 0.08);
  }

  .graph-node.graph-node-main {
    width: 10px;
    height: 10px;
    background: #66b0ff;
    box-shadow: 0 0 0 4px rgba(102, 176, 255, 0.1);
  }

  .subject-cell {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
  }

  .subject-cell strong {
    display: block;
    color: #eef5fb;
    line-height: 1;
    font-size: 0.89rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .history-tags {
    display: flex;
    gap: 6px;
    margin-top: 0;
    flex-shrink: 0;
  }

  .history-tags span {
    padding: 3px 7px;
    border-radius: 999px;
    background: rgba(43, 71, 98, 0.72);
    color: #c7d9eb;
    font-size: 0.68rem;
    text-transform: uppercase;
  }

  .author-cell {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 0.84rem;
    overflow: hidden;
  }

  .author-cell span:last-child {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .avatar {
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    border-radius: 999px;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, #2b6aa8, #70b8ff);
    color: #fff;
    font-size: 0.68rem;
    font-weight: 700;
  }

  .hash-cell {
    display: flex;
    align-items: center;
    color: #7990a7;
    font-size: 0.76rem;
    overflow: hidden;
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
    font-size: 0.76rem;
    overflow: hidden;
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
    grid-template-rows: 1fr auto;
    gap: 10px;
    overflow: hidden;
  }

  .changes-panel {
    display: grid;
    grid-template-rows: 1fr 1fr;
    overflow: hidden;
  }

  .changes-group {
    padding: 12px;
    overflow: auto;
  }

  .changes-group + .changes-group {
    border-top: 1px solid rgba(120, 148, 177, 0.08);
  }

  .changes-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 10px;
  }

  .changes-head h2 {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
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
  .new-branch:hover:enabled {
    transform: translateY(-1px);
    background: linear-gradient(180deg, #2673bf, #1160ae);
  }

  @media (max-width: 1180px) {
    .workspace {
      grid-template-columns: 220px minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) auto;
    }

    .right-pane {
      grid-column: 1 / -1;
      padding: 0 16px 16px;
      grid-template-columns: 1fr 320px;
      grid-template-rows: none;
      overflow: auto;
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
      grid-template-columns: 48px minmax(0, 1fr);
    }

    .history-rows li {
      height: 36px;
      padding: 6px 10px;
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
