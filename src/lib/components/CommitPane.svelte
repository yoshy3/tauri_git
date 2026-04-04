<script>
  import { _ } from "svelte-i18n";

  export let repository = null;
  export let changedEntries = [];
  export let expanded = false;
  export let activeTab = "commit";
  export let committing = false;
  export let stashing = false;
  export let onToggle = () => {};
  export let onSelectTab = () => {};
  export let onStash = async () => false;
  export let onCommit = async () => false;

  let commitSummary = "";
  let commitDescription = "";
  let stashMessage = "";
  let stashSelectedPaths = [];
  let currentRepoPath = "";
  let currentChangeSelectionKey = "";

  function statusLabel(entry) {
    const staged = entry.index_status === "." ? "" : entry.index_status;
    const unstaged = entry.working_tree_status === "." ? "" : entry.working_tree_status;

    if (staged && unstaged) {
      return `${staged}/${unstaged}`;
    }

    return staged || unstaged || $_("status.clean");
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

  function isStashPathSelected(path) {
    return stashSelectedPaths.includes(path);
  }

  function setStashPathSelected(path, selected) {
    if (selected) {
      if (stashSelectedPaths.includes(path)) {
        return;
      }

      stashSelectedPaths = [...stashSelectedPaths, path];
      return;
    }

    stashSelectedPaths = stashSelectedPaths.filter((entryPath) => entryPath !== path);
  }

  function selectAllStashPaths() {
    stashSelectedPaths = changedEntries.map((entry) => entry.path);
  }

  function clearSelectedStashPaths() {
    stashSelectedPaths = [];
  }

  async function handleStash() {
    if (!stashMessage.trim()) {
      return;
    }

    const success = await onStash(stashMessage.trim(), stashSelectedPaths);

    if (success) {
      stashMessage = "";
      stashSelectedPaths = [];
    }
  }

  $: nextRepoPath = repository?.repo_path ?? "";
  $: if (nextRepoPath !== currentRepoPath) {
    currentRepoPath = nextRepoPath;
    commitSummary = "";
    commitDescription = "";
    stashMessage = "";
    stashSelectedPaths = changedEntries.map((entry) => entry.path);
  }
  $: changedEntryPaths = changedEntries.map((entry) => entry.path);
  $: selectedStashPathSet = new Set(stashSelectedPaths);
  $: changeSelectionKey = `${currentRepoPath}::${changedEntryPaths.join("\u0001")}`;
  $: if (changeSelectionKey !== currentChangeSelectionKey) {
    currentChangeSelectionKey = changeSelectionKey;
    const validPaths = new Set(changedEntryPaths);
    const preservedPaths = stashSelectedPaths.filter((path) => validPaths.has(path));
    stashSelectedPaths =
      preservedPaths.length > 0 || changedEntryPaths.length === 0
        ? preservedPaths
        : [...changedEntryPaths];
  }
</script>

<aside class:collapsed-pane={!expanded} class="right-pane">
  {#if !expanded}
    <button class:attention={changedEntries.length > 0} class="pane-toggle" on:click={onToggle}>
      <span class="pane-toggle-label">
        {changedEntries.length > 0
          ? $_("commit.openPanelWithCount", { values: { count: changedEntries.length } })
          : $_("commit.openPanel")}
      </span>
    </button>
  {:else}
    <div class="panel-tabs" role="tablist" aria-label={$_("commit.panelTabs")}>
      <button
        class:panel-tab-active={activeTab === "commit"}
        class="panel-tab"
        type="button"
        role="tab"
        aria-selected={activeTab === "commit"}
        on:click={() => onSelectTab("commit")}
      >
        {$_("commit.commitTab")}
      </button>
      <button
        class:panel-tab-active={activeTab === "stash"}
        class="panel-tab"
        type="button"
        role="tab"
        aria-selected={activeTab === "stash"}
        on:click={() => onSelectTab("stash")}
      >
        {$_("commit.stashTab")}
      </button>
      <button class="panel-tab panel-tab-close" type="button" aria-label={$_("commit.closePanel")} on:click={onToggle}>
        &gt;&gt;
      </button>
    </div>

    {#if activeTab === "commit"}
      <section class="changes-panel">
        <div class="changes-summary">
          <div class="changes-summary-copy">
            <h2>{$_("commit.filesTitle", { values: { count: changedEntries.length } })}</h2>
            <p class="changes-caption">{$_("commit.filesCaption")}</p>
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
                    <strong title={entry.path}>{entry.path}</strong>
                  </div>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="empty-side">{$_("commit.filesEmpty")}</p>
          {/if}
        </div>
      </section>
    {/if}

    <section class="commit-panel">
      {#if activeTab === "stash"}
        <div class="action-block">
          <div class="action-block-copy">
            <h2>{$_("commit.stashTitle")}</h2>
            <p class="changes-caption">{$_("commit.stashCaption")}</p>
          </div>

          <label>
            <span>{$_("commit.stashMessageLabel")}</span>
            <input bind:value={stashMessage} placeholder={$_("commit.stashMessagePlaceholder")} />
          </label>

          <div class="stash-selection-toolbar">
            <span>{$_("commit.stashSelectionCount", { values: { count: stashSelectedPaths.length } })}</span>
            <div class="stash-selection-actions">
              <button class="secondary" type="button" on:click={selectAllStashPaths} disabled={changedEntries.length === 0 || stashing}>
                {$_("commit.selectAll")}
              </button>
              <button class="secondary" type="button" on:click={clearSelectedStashPaths} disabled={stashSelectedPaths.length === 0 || stashing}>
                {$_("commit.clear")}
              </button>
            </div>
          </div>

          <div class="stash-selection-list">
            {#if changedEntries.length > 0}
              {#each changedEntries as entry (entry.path)}
                <label class:selected={selectedStashPathSet.has(entry.path)} class="stash-selection-item">
                  <input
                    type="checkbox"
                    checked={selectedStashPathSet.has(entry.path)}
                    disabled={stashing}
                    on:change={(event) => setStashPathSelected(entry.path, event.currentTarget.checked)}
                  />
                  <span class:file-status={true} class:warning={entry.working_tree_status !== "."} class:ok={entry.working_tree_status === "."}>
                    {statusLabel(entry)}
                  </span>
                  <div class="stash-selection-copy">
                    <strong title={entry.path}>{entry.path}</strong>
                  </div>
                </label>
              {/each}
            {:else}
              <p class="empty-side">{$_("commit.stashEmpty")}</p>
            {/if}
          </div>

          <button
            class="primary wide"
            type="button"
            on:click={handleStash}
            disabled={!repository || stashing || stashSelectedPaths.length === 0 || !stashMessage.trim()}
          >
            {stashing ? $_("commit.stashing") : $_("commit.stashAction")}
          </button>
        </div>
      {:else}
        <div class="action-block">
          <div class="action-block-copy">
            <h2>{$_("commit.commitTitle")}</h2>
          </div>

          <label>
            <span>{$_("commit.summaryLabel")}</span>
            <input bind:value={commitSummary} placeholder={$_("commit.summaryPlaceholder")} />
          </label>

          <label>
            <span>{$_("commit.descriptionLabel")}</span>
            <textarea bind:value={commitDescription} rows="6" placeholder={$_("commit.descriptionPlaceholder")}></textarea>
          </label>

          <button class="primary wide" on:click={handleCommit} disabled={!repository || committing || repository.is_clean}>
            {committing
              ? $_("commit.committing")
              : $_("commit.commitToBranch", {
                  values: { branch: repository ? repository.branch : $_("commit.branchFallback") },
                })}
          </button>
        </div>
      {/if}
    </section>
  {/if}
</aside>

<style>
  .right-pane {
    min-height: 0;
    padding: 12px 12px 12px 0;
    display: flex;
    flex-direction: column;
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
    padding: 14px 6px;
    writing-mode: horizontal-tb;
    text-orientation: initial;
    justify-self: stretch;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 10px;
    overflow: hidden;
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

  .collapsed-pane .pane-toggle-label {
    line-height: 1;
    white-space: nowrap;
    transform: rotate(90deg);
    transform-origin: center;
    letter-spacing: 0.08em;
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
    line-height: 1.35;
    color: #6f859c;
    word-break: break-all;
  }

  .empty-side {
    margin: 2px 0 0;
    color: #6f859c;
    font-size: 0.78rem;
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

  .action-block {
    display: grid;
    gap: 10px;
  }

  .panel-tabs {
    display: grid;
    grid-template-columns: 1fr 1fr 48px;
    gap: 6px;
    flex: 0 0 38px;
    align-items: stretch;
  }

  .panel-tab {
    border: 1px solid rgba(120, 148, 177, 0.12);
    border-radius: 8px;
    background: rgba(12, 23, 35, 0.76);
    color: #8aa0b8;
    padding: 8px 10px;
    font-size: 0.74rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    height: 38px;
    align-self: start;
    box-sizing: border-box;
  }

  .panel-tab.panel-tab-active {
    background: rgba(18, 37, 56, 0.9);
    color: #eef5fb;
    box-shadow: inset 0 -2px 0 #4da0ff;
  }

  .panel-tab-close {
    padding: 8px 0;
    letter-spacing: 0.02em;
    font-size: 0.86rem;
  }

  .action-block-copy h2 {
    margin: 0;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: #f4f8fc;
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

  .stash-selection-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    color: #8aa0b8;
    font-size: 0.76rem;
  }

  .stash-selection-actions {
    display: inline-flex;
    gap: 6px;
  }

  .stash-selection-list {
    display: grid;
    gap: 6px;
    max-height: 220px;
    overflow: auto;
    padding-right: 2px;
  }

  .stash-selection-item {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    align-items: start;
    gap: 8px;
    padding: 8px 10px;
    border: 1px solid rgba(120, 148, 177, 0.1);
    border-radius: 8px;
    background: rgba(8, 16, 24, 0.64);
    transition: border-color 120ms ease, background 120ms ease, box-shadow 120ms ease;
  }

  .stash-selection-item.selected {
    border-color: rgba(84, 155, 233, 0.58);
    background: linear-gradient(180deg, rgba(18, 40, 61, 0.92), rgba(15, 31, 47, 0.92));
    box-shadow: inset 0 0 0 1px rgba(77, 160, 255, 0.12);
  }

  .stash-selection-item input[type="checkbox"] {
    margin: 3px 0 0;
    accent-color: #4da0ff;
  }

  .stash-selection-copy {
    min-width: 0;
  }

  .stash-selection-copy strong {
    color: #eef5fb;
    display: block;
    font-size: 0.86rem;
    line-height: 1.35;
    word-break: break-all;
  }

  .secondary {
    border: 1px solid rgba(120, 148, 177, 0.14);
    border-radius: 8px;
    background: rgba(12, 23, 35, 0.76);
    color: #d4e1ee;
    padding: 7px 10px;
    font-size: 0.74rem;
    font-weight: 600;
  }

  .secondary:hover:enabled {
    background: rgba(18, 33, 47, 0.88);
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
      display: flex;
      flex-direction: column;
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
      overflow: visible;
    }
  }
</style>
