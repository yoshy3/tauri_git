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
        <strong>GitPrecision</strong>
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

      {#if error}
        <div class="banner error-banner">{error}</div>
      {/if}

      <section class="history-table">
        <div class="history-head">
          <span>Graph</span>
          <span>Subject</span>
          <span>Author</span>
          <span>Hash / Date</span>
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
                  <span>{commit.committed_at}</span>
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
      radial-gradient(circle at top left, rgba(12, 34, 56, 0.55), transparent 20%),
      linear-gradient(180deg, #08121d 0%, #0e1925 100%);
    color: #d5deea;
  }

  .app-shell {
    min-height: 100vh;
    display: grid;
    grid-template-rows: 60px 1fr;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 18px;
    border-bottom: 1px solid rgba(114, 144, 175, 0.12);
    background: rgba(6, 14, 23, 0.9);
    backdrop-filter: blur(18px);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .brand strong {
    display: block;
    font-size: 1.05rem;
    letter-spacing: 0.02em;
  }

  .brand p {
    margin: 2px 0 0;
    color: #6f859c;
    font-size: 0.75rem;
  }

  .brand-mark {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, #1f5b94, #4ca4ff);
    color: white;
    font-weight: 700;
  }

  .toolbar {
    display: flex;
    gap: 10px;
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
  }

  .toolbar-button.active {
    color: #f2f7fb;
    border-bottom: 2px solid #4da0ff;
    padding-bottom: 8px;
  }

  .workspace {
    min-height: 0;
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr) 320px;
  }

  .sidebar,
  .center-pane,
  .right-pane {
    min-height: 0;
  }

  .sidebar {
    padding: 16px 12px;
    border-right: 1px solid rgba(114, 144, 175, 0.08);
    background: rgba(8, 18, 29, 0.92);
    display: grid;
    grid-template-rows: auto auto auto auto 1fr auto;
    gap: 14px;
  }

  .repo-card,
  .open-panel,
  .branch-panel,
  .changes-panel,
  .commit-panel,
  .history-table {
    background: rgba(12, 24, 38, 0.92);
    border: 1px solid rgba(120, 148, 177, 0.1);
    border-radius: 12px;
  }

  .repo-card,
  .open-panel,
  .branch-panel,
  .commit-panel {
    padding: 14px;
  }

  .sidebar-label {
    margin: 0 0 8px;
    color: #6f859c;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  h1,
  h2 {
    margin: 0;
    font-size: 1rem;
    color: #f4f8fc;
  }

  .repo-card h1 {
    font-size: 1.1rem;
  }

  .muted,
  .path,
  .empty-side {
    color: #7088a2;
  }

  .path {
    font-size: 0.78rem;
    line-height: 1.5;
    word-break: break-all;
  }

  .open-panel label,
  .commit-panel label {
    display: grid;
    gap: 7px;
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
    border: 1px solid rgba(120, 148, 177, 0.14);
    border-radius: 8px;
    background: #050b11;
    color: #e8eef5;
    padding: 11px 12px;
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
  }

  .wide {
    width: 100%;
    padding: 12px 14px;
    margin-top: 12px;
  }

  .new-branch {
    padding: 12px 14px;
    text-transform: uppercase;
    font-size: 0.78rem;
  }

  .side-nav {
    display: grid;
    gap: 4px;
  }

  .side-nav button {
    text-align: left;
    padding: 10px 12px;
    border-radius: 8px;
    text-transform: none;
    letter-spacing: 0;
    font-size: 0.9rem;
  }

  .side-nav button.item-active {
    background: rgba(28, 92, 157, 0.35);
    color: #f1f7ff;
  }

  .branch-panel {
    display: grid;
    gap: 10px;
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
    gap: 8px;
  }

  .branch-list li {
    display: flex;
    align-items: center;
    gap: 9px;
    color: #a8bbcf;
    font-size: 0.92rem;
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
    gap: 6px;
    align-self: end;
  }

  .sidebar-footer button {
    text-align: left;
    padding: 8px 4px;
    font-size: 0.86rem;
    letter-spacing: 0;
  }

  .center-pane {
    padding: 16px;
    display: grid;
    grid-template-rows: auto auto 1fr;
    gap: 14px;
    min-width: 0;
  }

  .history-toolbar {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: center;
  }

  .search {
    max-width: 520px;
  }

  .history-meta {
    display: flex;
    gap: 12px;
    color: #6f859c;
    font-size: 0.85rem;
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
  }

  .history-head,
  .history-rows li {
    display: grid;
    grid-template-columns: 72px minmax(0, 1.5fr) minmax(140px, 0.8fr) 150px;
    gap: 14px;
    align-items: center;
  }

  .history-head {
    padding: 12px 18px;
    color: #6f859c;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    border-bottom: 1px solid rgba(120, 148, 177, 0.08);
  }

  .history-rows {
    overflow: auto;
  }

  .history-rows li {
    padding: 14px 18px;
    border-bottom: 1px solid rgba(120, 148, 177, 0.05);
  }

  .graph-cell {
    position: relative;
    min-height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .graph-line,
  .graph-tail {
    position: absolute;
    width: 2px;
    background: rgba(110, 143, 176, 0.55);
    left: calc(50% - 1px);
  }

  .graph-line {
    top: -14px;
    height: 16px;
  }

  .graph-tail {
    top: 20px;
    bottom: -14px;
  }

  .graph-node {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: #ffb54a;
    z-index: 1;
  }

  .graph-node.graph-node-main {
    width: 10px;
    height: 10px;
    background: #66b0ff;
  }

  .subject-cell {
    min-width: 0;
  }

  .subject-cell strong {
    display: block;
    color: #eef5fb;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .history-tags {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }

  .history-tags span {
    padding: 3px 7px;
    border-radius: 999px;
    background: rgba(43, 71, 98, 0.85);
    color: #c7d9eb;
    font-size: 0.68rem;
    text-transform: uppercase;
  }

  .author-cell {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .avatar {
    width: 26px;
    height: 26px;
    border-radius: 999px;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, #2b6aa8, #70b8ff);
    color: #fff;
    font-size: 0.74rem;
    font-weight: 700;
  }

  .hash-cell {
    display: grid;
    gap: 2px;
    color: #7990a7;
    font-size: 0.82rem;
  }

  .empty-history {
    padding: 28px 20px;
  }

  .right-pane {
    padding: 16px 16px 16px 0;
    display: grid;
    grid-template-rows: 1fr auto;
    gap: 14px;
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
    font-size: 0.86rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .changes-group li {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px;
    align-items: start;
    padding: 10px 0;
    border-bottom: 1px solid rgba(120, 148, 177, 0.05);
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
    font-size: 0.72rem;
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
  }

  button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  @media (max-width: 1180px) {
    .workspace {
      grid-template-columns: 220px minmax(0, 1fr);
    }

    .right-pane {
      grid-column: 1 / -1;
      padding: 0 16px 16px;
      grid-template-columns: 1fr 320px;
      grid-template-rows: none;
    }
  }

  @media (max-width: 860px) {
    .app-shell {
      grid-template-rows: auto 1fr;
    }

    .topbar,
    .history-toolbar {
      align-items: stretch;
      flex-direction: column;
    }

    .workspace {
      grid-template-columns: 1fr;
    }

    .right-pane,
    .center-pane,
    .sidebar {
      padding: 14px;
    }

    .right-pane {
      grid-template-columns: 1fr;
    }

    .history-head,
    .history-rows li {
      grid-template-columns: 48px minmax(0, 1fr);
    }

    .history-head span:nth-child(3),
    .history-head span:nth-child(4),
    .author-cell,
    .hash-cell {
      display: none;
    }
  }
</style>
