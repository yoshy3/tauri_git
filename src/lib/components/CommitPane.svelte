<script>
  import { _ } from "svelte-i18n";
  import DiffCompareDialog from "./DiffCompareDialog.svelte";

  export let repository = null;
  export let changedEntries = [];
  export let expanded = false;
  export let activeTab = "commit";
  export let committing = false;
  export let commitAndPushing = false;
  export let stashing = false;
  export let discarding = false;
  export let onToggle = () => {};
  export let onSelectTab = () => {};
  export let onStash = async () => false;
  export let onDiscard = async () => false;
  export let onCommit = async () => false;
  export let onCommitAndPush = async () => false;
  export let onLoadCompareDiff = async () => null;

  let commitSummary = "";
  let commitDescription = "";
  let stashMessage = "";
  let stashSelectedPaths = [];
  let discardSelectedPaths = [];
  let currentRepoPath = "";
  let currentChangeSelectionKey = "";
  let compareDialogOpen = false;
  let compareDialogLoading = false;
  let compareDialogFilePath = "";
  let compareDialogPatchText = "";
  let compareDialogStatus = "";

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

  async function handleCompare(entry) {
    compareDialogFilePath = entry.path;
    compareDialogPatchText = "";
    compareDialogStatus = statusLabel(entry);
    compareDialogLoading = true;
    compareDialogOpen = true;

    const diff = await onLoadCompareDiff(entry);

    if (!diff) {
      compareDialogLoading = false;
      compareDialogOpen = false;
      return;
    }

    compareDialogFilePath = diff.path || entry.path;
    compareDialogPatchText = diff.patch || "";
    compareDialogStatus = statusLabel(entry);
    compareDialogLoading = false;
  }

  function closeCompareDialog() {
    compareDialogOpen = false;
    compareDialogLoading = false;
    compareDialogFilePath = "";
    compareDialogPatchText = "";
    compareDialogStatus = "";
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

  async function handleCommitAndPush() {
    const summary = commitSummary.trim();
    const description = commitDescription.trim();

    if (!summary) {
      const success = await onCommitAndPush("");
      if (success) {
        commitSummary = "";
        commitDescription = "";
      }
      return;
    }

    const message = description ? `${summary}\n\n${description}` : summary;
    const success = await onCommitAndPush(message);

    if (success) {
      commitSummary = "";
      commitDescription = "";
    }
  }

  function isDiscardPathSelected(path) {
    return discardSelectedPaths.includes(path);
  }

  function setDiscardPathSelected(path, selected) {
    if (selected) {
      if (discardSelectedPaths.includes(path)) {
        return;
      }

      discardSelectedPaths = [...discardSelectedPaths, path];
      return;
    }

    discardSelectedPaths = discardSelectedPaths.filter((entryPath) => entryPath !== path);
  }

  function selectAllDiscardPaths() {
    discardSelectedPaths = changedEntries.map((entry) => entry.path);
  }

  function clearSelectedDiscardPaths() {
    discardSelectedPaths = [];
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

  async function handleDiscard() {
    const success = await onDiscard(discardSelectedPaths);

    if (success) {
      discardSelectedPaths = [];
    }
  }

  $: nextRepoPath = repository?.repo_path ?? "";
  $: if (nextRepoPath !== currentRepoPath) {
    currentRepoPath = nextRepoPath;
    commitSummary = "";
    commitDescription = "";
    stashMessage = "";
    stashSelectedPaths = changedEntries.map((entry) => entry.path);
    discardSelectedPaths = changedEntries.map((entry) => entry.path);
  }
  $: changedEntryPaths = changedEntries.map((entry) => entry.path);
  $: selectedStashPathSet = new Set(stashSelectedPaths);
  $: selectedDiscardPathSet = new Set(discardSelectedPaths);
  $: changeSelectionKey = `${currentRepoPath}::${changedEntryPaths.join("\u0001")}`;
  $: if (changeSelectionKey !== currentChangeSelectionKey) {
    currentChangeSelectionKey = changeSelectionKey;
    const validPaths = new Set(changedEntryPaths);
    const preservedPaths = stashSelectedPaths.filter((path) => validPaths.has(path));
    stashSelectedPaths =
      preservedPaths.length > 0 || changedEntryPaths.length === 0
        ? preservedPaths
        : [...changedEntryPaths];
    const preservedDiscardPaths = discardSelectedPaths.filter((path) => validPaths.has(path));
    discardSelectedPaths =
      preservedDiscardPaths.length > 0 || changedEntryPaths.length === 0
        ? preservedDiscardPaths
        : [...changedEntryPaths];
  }
  $: if (!expanded) {
    closeCompareDialog();
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
      <button
        class:panel-tab-active={activeTab === "discard"}
        class="panel-tab"
        type="button"
        role="tab"
        aria-selected={activeTab === "discard"}
        on:click={() => onSelectTab("discard")}
      >
        {$_("commit.discardTab")}
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
                  <button class="compare-button" type="button" on:click={() => handleCompare(entry)}>
                    {$_("history.details.compare")}
                  </button>
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
      {:else if activeTab === "discard"}
        <div class="action-block">
          <div class="action-block-copy">
            <h2>{$_("commit.discardTitle")}</h2>
            <p class="changes-caption">{$_("commit.discardCaption")}</p>
          </div>

          <div class="discard-warning">
            {$_("commit.discardWarning")}
          </div>

          <div class="stash-selection-toolbar">
            <span>{$_("commit.discardSelectionCount", { values: { count: discardSelectedPaths.length } })}</span>
            <div class="stash-selection-actions">
              <button class="secondary" type="button" on:click={selectAllDiscardPaths} disabled={changedEntries.length === 0 || discarding}>
                {$_("commit.selectAll")}
              </button>
              <button class="secondary" type="button" on:click={clearSelectedDiscardPaths} disabled={discardSelectedPaths.length === 0 || discarding}>
                {$_("commit.clear")}
              </button>
            </div>
          </div>

          <div class="stash-selection-list">
            {#if changedEntries.length > 0}
              {#each changedEntries as entry (entry.path)}
                <label class:selected={selectedDiscardPathSet.has(entry.path)} class="stash-selection-item discard-selection-item">
                  <input
                    type="checkbox"
                    checked={isDiscardPathSelected(entry.path)}
                    disabled={discarding}
                    on:change={(event) => setDiscardPathSelected(entry.path, event.currentTarget.checked)}
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
              <p class="empty-side">{$_("commit.discardEmpty")}</p>
            {/if}
          </div>

          <button
            class="primary wide danger"
            type="button"
            on:click={handleDiscard}
            disabled={!repository || discarding || discardSelectedPaths.length === 0}
          >
            {discarding ? $_("commit.discarding") : $_("commit.discardAction")}
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

          <div class="commit-actions">
            <button class="primary wide" on:click={handleCommit} disabled={!repository || committing || commitAndPushing || repository.is_clean}>
              {committing
                ? $_("commit.committing")
                : $_("commit.commitToBranch", {
                    values: { branch: repository ? repository.branch : $_("commit.branchFallback") },
                  })}
            </button>

            <button
              class="primary wide secondary-primary"
              on:click={handleCommitAndPush}
              disabled={!repository || committing || commitAndPushing || repository.is_clean || !repository.can_push_current_branch}
            >
              {commitAndPushing
                ? $_("commit.commitAndPushing")
                : $_("commit.commitAndPushToBranch", {
                    values: { branch: repository ? repository.branch : $_("commit.branchFallback") },
                  })}
            </button>
          </div>
        </div>
      {/if}
    </section>
  {/if}
</aside>

<DiffCompareDialog
  open={compareDialogOpen}
  filePath={compareDialogFilePath}
  patchText={compareDialogPatchText}
  status={compareDialogStatus}
  onClose={closeCompareDialog}
/>

<style>
  .right-pane {
    min-height: 0;
    padding: 12px 12px 12px 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow: hidden;
    width: 100%;
    min-width: 0;
    justify-self: stretch;
    transition: padding 160ms ease;
  }

  .right-pane.collapsed-pane {
    padding-left: 0;
    padding-right: 8px;
  }

  .pane-toggle {
    border: 1px solid var(--surface-border-strong);
    border-radius: 12px;
    background: var(--surface-background-strong);
    color: var(--text-secondary);
    min-height: 54px;
    padding: 10px 12px;
    text-align: left;
    font-size: 0.76rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    writing-mode: horizontal-tb;
    box-shadow: var(--panel-shadow);
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
    background: linear-gradient(180deg, var(--accent-strong), var(--accent-strong-2));
    border-color: var(--accent-soft-border);
    color: var(--accent-contrast);
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
    box-shadow: var(--panel-shadow);
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
    color: var(--text-primary);
  }

  .changes-caption {
    margin: 4px 0 0;
    color: var(--text-subtle);
    font-size: 0.76rem;
    line-height: 1.3;
  }

  .changes-group {
    padding: 12px;
    overflow: auto;
  }

  .changes-group.unified {
    border-top: 1px solid var(--panel-border);
  }

  .changes-group ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .changes-group li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 10px 2px;
    border-bottom: 1px solid var(--row-border);
  }

  .changes-group strong {
    display: block;
    font-size: 0.9rem;
    line-height: 1.35;
    color: var(--text-subtle);
    word-break: break-all;
  }

  .compare-button {
    border: 1px solid var(--surface-border);
    border-radius: 7px;
    background: var(--surface-background);
    color: var(--text-secondary);
    padding: 6px 10px;
    font-size: 0.71rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .compare-button:hover {
    background: var(--surface-background-hover);
    border-color: var(--accent-soft-border);
  }

  .empty-side {
    margin: 2px 0 0;
    color: var(--text-subtle);
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
    background: var(--warning-soft);
    color: var(--warning-text);
  }

  .file-status.ok {
    background: var(--success-soft);
    color: var(--success-text);
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
    grid-template-columns: 1fr 1fr 1fr 48px;
    gap: 6px;
    flex: 0 0 38px;
    align-items: stretch;
  }

  .panel-tab {
    border: 1px solid var(--surface-border);
    border-radius: 8px;
    background: var(--surface-background);
    color: var(--text-muted);
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
    background: var(--surface-background-hover);
    color: var(--text-primary);
    box-shadow: inset 0 -2px 0 var(--accent);
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
    color: var(--text-primary);
  }

  .commit-panel label {
    display: grid;
    gap: 5px;
  }

  .commit-panel span {
    color: var(--text-muted);
    font-size: 0.78rem;
  }

  .commit-panel input,
  .commit-panel textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--input-border);
    border-radius: 8px;
    background: var(--input-background);
    color: var(--text-secondary);
    padding: 11px 12px;
    transition: border-color 120ms ease, box-shadow 120ms ease, background 120ms ease;
  }

  .commit-panel input:focus,
  .commit-panel textarea:focus {
    outline: none;
    border-color: var(--input-border-focus);
    box-shadow: var(--focus-ring);
    background: var(--input-background-focus);
  }

  .commit-panel textarea {
    resize: vertical;
  }

  .stash-selection-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    color: var(--text-muted);
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
    border: 1px solid var(--panel-border);
    border-radius: 8px;
    background: var(--panel-soft-background);
    transition: border-color 120ms ease, background 120ms ease, box-shadow 120ms ease;
  }

  .stash-selection-item.selected {
    border-color: var(--selection-accent-border);
    background: var(--selection-accent-background);
    box-shadow: var(--selection-accent-shadow);
  }

  .discard-warning {
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--danger-soft);
    border: 1px solid var(--danger-border);
    color: var(--danger-text);
    font-size: 0.76rem;
    line-height: 1.45;
  }

  .discard-selection-item.selected {
    border-color: var(--selection-danger-border);
    background: var(--selection-danger-background);
    box-shadow: var(--selection-danger-shadow);
  }

  .stash-selection-item input[type="checkbox"] {
    margin: 3px 0 0;
    accent-color: var(--accent);
  }

  .stash-selection-copy {
    min-width: 0;
  }

  .stash-selection-copy strong {
    color: var(--text-primary);
    display: block;
    font-size: 0.86rem;
    line-height: 1.35;
    word-break: break-all;
  }

  .secondary {
    border: 1px solid var(--surface-border);
    border-radius: 8px;
    background: var(--surface-background);
    color: var(--text-secondary);
    padding: 7px 10px;
    font-size: 0.74rem;
    font-weight: 600;
  }

  .secondary:hover:enabled {
    background: var(--surface-background-hover);
  }

  .primary {
    border: 0;
    border-radius: 8px;
    background: linear-gradient(180deg, var(--accent-strong), var(--accent-strong-2));
    color: var(--accent-contrast);
    font-weight: 700;
    letter-spacing: 0.03em;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.12);
  }

  .primary:hover:enabled,
  .pane-toggle:hover:enabled {
    transform: translateY(-1px);
  }

  .primary:hover:enabled {
    background: linear-gradient(180deg, color-mix(in srgb, var(--accent-strong) 88%, white), color-mix(in srgb, var(--accent-strong-2) 88%, white));
  }

  .primary.danger {
    background: linear-gradient(180deg, var(--danger-strong), var(--danger-strong-2));
  }

  .primary.danger:hover:enabled {
    background: linear-gradient(180deg, color-mix(in srgb, var(--danger-strong) 88%, white), color-mix(in srgb, var(--danger-strong-2) 88%, white));
  }

  .wide {
    width: 100%;
    padding: 10px 12px;
    margin-top: 8px;
  }

  .commit-actions {
    display: grid;
    gap: 8px;
  }

  .secondary-primary {
    background: linear-gradient(180deg, #286b57, #185240);
  }

  .secondary-primary:hover:enabled {
    background: linear-gradient(180deg, #317a64, #1d624d);
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
