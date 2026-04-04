<script>
  export let repository = null;
  export let changedEntries = [];
  export let expanded = false;
  export let committing = false;
  export let onToggle = () => {};
  export let onCommit = async () => false;

  let commitSummary = "";
  let commitDescription = "";
  let currentRepoPath = "";

  function statusLabel(entry) {
    const staged = entry.index_status === "." ? "" : entry.index_status;
    const unstaged = entry.working_tree_status === "." ? "" : entry.working_tree_status;

    if (staged && unstaged) {
      return `${staged}/${unstaged}`;
    }

    return staged || unstaged || "clean";
  }

  function shortPath(path) {
    const parts = path.split("/");
    if (parts.length <= 2) {
      return path;
    }

    return `${parts.slice(0, 2).join("/")}/.../${parts.at(-1)}`;
  }

  async function handleCommit() {
    const summary = commitSummary.trim();
    const description = commitDescription.trim();

    if (!summary) {
      const success = await onCommit("");
      if (success) {
        commitSummary = "";
        commitDescription = "";
      }
      return;
    }

    const message = description ? `${summary}\n\n${description}` : summary;
    const success = await onCommit(message);

    if (success) {
      commitSummary = "";
      commitDescription = "";
    }
  }

  $: nextRepoPath = repository?.repo_path ?? "";
  $: if (nextRepoPath !== currentRepoPath) {
    currentRepoPath = nextRepoPath;
    commitSummary = "";
    commitDescription = "";
  }
</script>

<aside class:collapsed-pane={!expanded} class="right-pane">
  <button class:attention={changedEntries.length > 0} class="pane-toggle" on:click={onToggle}>
    <span class="pane-toggle-label">
      {expanded
        ? "Close Commit Panel"
        : changedEntries.length > 0
          ? `Open Commit Panel (${changedEntries.length})`
          : "Open Commit Panel"}
    </span>
  </button>

  {#if expanded}
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

      <button class="primary wide" on:click={handleCommit} disabled={!repository || committing || repository.is_clean}>
        {committing ? "Committing..." : `Commit to ${repository ? repository.branch : "branch"}`}
      </button>
    </section>
  {/if}
</aside>

<style>
  .right-pane {
    min-height: 0;
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

  .changes-panel,
  .commit-panel {
    background: var(--panel-background);
    border: 1px solid var(--panel-border);
    border-radius: 10px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
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
    margin: 0;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: #f4f8fc;
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

  .changes-group ul {
    list-style: none;
    padding: 0;
    margin: 0;
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

  .changes-group p,
  .empty-side {
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
    padding: 10px;
    border-radius: 8px;
    overflow: auto;
  }

  .commit-panel label {
    display: grid;
    gap: 5px;
  }

  .commit-panel span {
    color: #8aa0b8;
    font-size: 0.78rem;
  }

  .commit-panel input,
  .commit-panel textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid rgba(120, 148, 177, 0.12);
    border-radius: 8px;
    background: #040a10;
    color: #e8eef5;
    padding: 11px 12px;
    transition: border-color 120ms ease, box-shadow 120ms ease, background 120ms ease;
  }

  .commit-panel input:focus,
  .commit-panel textarea:focus {
    outline: none;
    border-color: rgba(84, 155, 233, 0.7);
    box-shadow: 0 0 0 3px rgba(35, 101, 168, 0.18);
    background: #06101a;
  }

  .commit-panel textarea {
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

  .primary:hover:enabled,
  .pane-toggle:hover:enabled {
    transform: translateY(-1px);
  }

  .primary:hover:enabled {
    background: linear-gradient(180deg, #2673bf, #1160ae);
  }

  .wide {
    width: 100%;
    padding: 10px 12px;
    margin-top: 8px;
  }

  @media (max-width: 1180px) {
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
    .right-pane {
      padding: 14px;
      grid-template-columns: 1fr;
      overflow: visible;
    }
  }
</style>
