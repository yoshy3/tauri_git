<script>
  import { _ } from "svelte-i18n";
  import BranchTreeNode from "./BranchTreeNode.svelte";

  export let repository = null;
  export let loading = false;
  export let selectedStashIndex = null;
  export let selectedRef = null;
  export let stashBusyAction = "";
  export let menuOpenKey = "";
  export let onSelectStash = () => {};
  export let onSelectTag = () => {};
  export let onOpenCreateTagDialog = () => {};
  export let onOpenDeleteTagDialog = () => {};
  export let onToggleMenu = () => {};
  export let onCheckoutReference = () => {};
  export let onCreateBranchFromReference = () => {};
  export let onRebaseReference = () => {};
  export let onDeleteReference = () => {};
  export let onCancelSelectedStash = () => {};
  export let onApplySelectedStash = () => {};
  export let onPopSelectedStash = () => {};

  let branchFilter = "";
  let branchFilterTerm = "";
  let sidebarSections = {
    branches: true,
    remotes: true,
    tags: false,
    stashes: true,
    submodules: false,
  };
  let remoteSections = {};

  function toggleSidebarSection(section) {
    sidebarSections = {
      ...sidebarSections,
      [section]: !sidebarSections[section],
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

  function tagMenuKey(tagName) {
    return `tag:${tagName}`;
  }

  function toggleTagMenu(tagName) {
    const key = tagMenuKey(tagName);
    onToggleMenu(menuOpenKey === key ? "" : key);
  }

  function openDeleteTag(tagName) {
    onToggleMenu("");
    onOpenDeleteTagDialog(tagName);
  }

  function matchesBranchFilter(value) {
    if (!branchFilterTerm) {
      return true;
    }

    return String(value || "").toLowerCase().includes(branchFilterTerm);
  }

  function buildBranchTree(branchNames, branchSyncMap = {}) {
    const root = [];

    branchNames.forEach((branchName) => {
      const segments = String(branchName || "")
        .split("/")
        .filter(Boolean);

      if (segments.length === 0) {
        return;
      }

      let level = root;
      let pathSoFar = "";

      segments.forEach((segment, index) => {
        pathSoFar = pathSoFar ? `${pathSoFar}/${segment}` : segment;
        const isBranch = index === segments.length - 1;
        let node = level.find((entry) => entry.label === segment && entry.isBranch === isBranch);

        if (!node) {
          node = {
            key: pathSoFar,
            label: segment,
            fullName: pathSoFar,
            isBranch,
            aheadCount: isBranch ? branchSyncMap[pathSoFar]?.ahead_count ?? 0 : 0,
            behindCount: isBranch ? branchSyncMap[pathSoFar]?.behind_count ?? 0 : 0,
            children: [],
          };
          level.push(node);
        } else if (isBranch) {
          node.aheadCount = branchSyncMap[pathSoFar]?.ahead_count ?? 0;
          node.behindCount = branchSyncMap[pathSoFar]?.behind_count ?? 0;
        }

        level = node.children;
      });
    });

    return root;
  }

  function filterBranchTree(nodes, filterTerm, parentPath = "") {
    if (!filterTerm) {
      return nodes;
    }

    return nodes
      .map((node) => {
        const nodePath = parentPath ? `${parentPath}/${node.label}` : node.label;

        if (node.isBranch) {
          return matchesBranchFilter(node.fullName) ? node : null;
        }

        const filteredChildren = filterBranchTree(node.children, filterTerm, nodePath);
        if (matchesBranchFilter(nodePath) || filteredChildren.length > 0) {
          return {
            ...node,
            children: filteredChildren,
          };
        }

        return null;
      })
      .filter(Boolean);
  }

  $: branchFilterTerm = branchFilter.trim().toLowerCase();
  $: localBranchSyncMap = repository
    ? Object.fromEntries((repository.local_branch_syncs ?? []).map((entry) => [entry.name, entry]))
    : {};
  $: localBranchTree = repository ? buildBranchTree(repository.local_branches, localBranchSyncMap) : [];
  $: filteredLocalBranchTree = filterBranchTree(localBranchTree, branchFilterTerm);
  $: filteredRemoteGroups = repository
    ? repository.remote_groups
        .map((group) => {
          const fullTree = buildBranchTree(group.branches);
          const tree = matchesBranchFilter(group.name)
            ? fullTree
            : filterBranchTree(fullTree, branchFilterTerm);

          if (matchesBranchFilter(group.name) || tree.length > 0) {
            return {
              ...group,
              tree,
            };
          }

          return null;
        })
        .filter(Boolean)
    : [];
  $: hasBranchFilter = branchFilterTerm.length > 0;
  $: filteredTags = repository ? repository.tags : [];
  $: filteredStashes = repository ? repository.stashes : [];
  $: filteredSubmodules = repository ? repository.submodules : [];
  $: selectedStash =
    repository && selectedStashIndex !== null
      ? repository.stashes.find((stash) => stash.index === selectedStashIndex) ?? null
      : null;
  $: branchesSectionVisible = sidebarSections.branches || hasBranchFilter;
  $: remotesSectionVisible = sidebarSections.remotes || hasBranchFilter;
</script>

<aside class="sidebar">
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
      <h1>{$_("sidebar.noRepository")}</h1>
      <p class="muted">{$_("sidebar.openRepositoryHint")}</p>
    {/if}
  </section>

      <div class="sidebar-filter-wrap">
        <input
          class="sidebar-filter"
          placeholder={$_("sidebar.branchFilter")}
          bind:value={branchFilter}
          disabled={!repository}
        />
      </div>

  <section class="sidebar-tree">
    {#if !repository}
      <p class="tree-empty">{$_("sidebar.emptyState")}</p>
    {:else}
      <div class="tree-section">
        <button class="tree-section-toggle" type="button" on:click={() => toggleSidebarSection("branches")}>
          <span class:expanded={sidebarSections.branches} class="tree-chevron"></span>
          <span>{$_("sidebar.branches")}</span>
        </button>

        {#if branchesSectionVisible}
          {#if filteredLocalBranchTree.length > 0}
            <div class="tree-branch-root">
              {#key `local:${branchFilterTerm}`}
                <BranchTreeNode
                  nodes={filteredLocalBranchTree}
                  {loading}
                  currentBranch={repository.branch}
                  {menuOpenKey}
                  {onToggleMenu}
                  {onCheckoutReference}
                  {onCreateBranchFromReference}
                  {onRebaseReference}
                  {onDeleteReference}
                />
              {/key}
            </div>
          {:else if hasBranchFilter}
            <p class="tree-empty">{$_("sidebar.matchingBranchesEmpty")}</p>
          {:else}
            <p class="tree-empty">{$_("sidebar.branchesEmpty")}</p>
          {/if}
        {/if}
      </div>

      <div class="tree-section">
        <button class="tree-section-toggle" type="button" on:click={() => toggleSidebarSection("remotes")}>
          <span class:expanded={sidebarSections.remotes} class="tree-chevron"></span>
          <span>{$_("sidebar.remotes")}</span>
        </button>

        {#if remotesSectionVisible}
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

                    {#if group.tree.length > 0}
                      <div class="tree-nested-list">
                        {#key `remote:${group.name}:${branchFilterTerm}`}
                          <BranchTreeNode
                            nodes={group.tree}
                            {loading}
                            currentBranch={repository.branch}
                            remoteName={group.name}
                            {menuOpenKey}
                            {onToggleMenu}
                            {onCheckoutReference}
                            {onCreateBranchFromReference}
                            {onRebaseReference}
                            {onDeleteReference}
                          />
                        {/key}
                      </div>
                    {:else}
                      <p class="tree-empty tree-empty-nested">{$_("sidebar.matchingBranchesEmpty")}</p>
                    {/if}
                  </details>
                </li>
              {/each}
            </ul>
          {:else if hasBranchFilter}
            <p class="tree-empty">{$_("sidebar.matchingBranchesEmpty")}</p>
          {:else}
            <p class="tree-empty">{$_("sidebar.remotesEmpty")}</p>
          {/if}
        {/if}
      </div>

      {#if !hasBranchFilter}
      <div class="tree-section">
        <div class="tree-section-header">
          <button class="tree-section-toggle tree-section-toggle-grow" type="button" on:click={() => toggleSidebarSection("tags")}>
            <span class:expanded={sidebarSections.tags} class="tree-chevron"></span>
            <span>{$_("sidebar.tags")}</span>
          </button>
          <button class="tree-section-action" type="button" aria-label={$_("sidebar.newTag")} disabled={loading} on:click={onOpenCreateTagDialog}>
            +
          </button>
        </div>

        {#if sidebarSections.tags}
          {#if filteredTags.length > 0}
            <ul class="tree-list tree-section-children">
              {#each filteredTags as tagName}
                <li>
                  <div class:tree-item-current={selectedRef?.kind === "tag" && selectedRef.name === tagName} class="tree-item-row">
                    <button class="tree-item tree-item-button tree-item-tag-button" type="button" on:click={() => onSelectTag(tagName)}>
                      <span class="tree-item-icon tree-item-tag"></span>
                      <span class="tree-item-label">{tagName}</span>
                    </button>

                    <div class:menu-open={menuOpenKey === tagMenuKey(tagName)} class="tree-item-actions">
                      <button
                        class="tree-item-kebab"
                        type="button"
                        aria-label={$_("sidebar.tagActions")}
                        disabled={loading}
                        on:click={() => toggleTagMenu(tagName)}
                      >
                        <span></span>
                        <span></span>
                        <span></span>
                      </button>

                      {#if menuOpenKey === tagMenuKey(tagName)}
                        <div class="tree-item-menu">
                          <button class="tree-item-menu-button tree-item-menu-button-danger" type="button" on:click={() => openDeleteTag(tagName)}>
                            {$_("sidebar.deleteTag")}
                          </button>
                        </div>
                      {/if}
                    </div>
                  </div>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="tree-empty">{$_("sidebar.tagsEmpty")}</p>
          {/if}
        {/if}
      </div>

      <div class="tree-section">
        <button class="tree-section-toggle" type="button" on:click={() => toggleSidebarSection("stashes")}>
          <span class:expanded={sidebarSections.stashes} class="tree-chevron"></span>
          <span>{$_("sidebar.stashes")}</span>
        </button>

        {#if sidebarSections.stashes}
          {#if selectedStash}
            <div class="stash-actions">
              <div class="stash-actions-copy">
                <strong>{selectedStash.name}</strong>
                <span>{selectedStash.message}</span>
              </div>
              <div class="stash-actions-buttons">
                <button
                  class="stash-action-button stash-action-button-neutral"
                  type="button"
                  disabled={loading}
                  on:click={onCancelSelectedStash}
                >
                  {$_("sidebar.cancel")}
                </button>
                <button
                  class="stash-action-button"
                  type="button"
                  disabled={loading}
                  on:click={() => onApplySelectedStash(selectedStash.index)}
                >
                  {stashBusyAction === "apply" ? $_("sidebar.applying") : $_("sidebar.apply")}
                </button>
                <button
                  class="stash-action-button stash-action-button-danger"
                  type="button"
                  disabled={loading}
                  on:click={() => onPopSelectedStash(selectedStash.index)}
                >
                  {stashBusyAction === "pop" ? $_("sidebar.popping") : $_("sidebar.pop")}
                </button>
              </div>
            </div>
          {/if}

          {#if filteredStashes.length > 0}
            <ul class="tree-list tree-section-children">
              {#each filteredStashes as stash}
                <li>
                  <button
                    class:tree-item-current={stash.index === selectedStashIndex}
                    class="tree-item tree-item-button tree-item-stack"
                    type="button"
                    on:click={() => onSelectStash(stash.index)}
                  >
                    <span class="tree-item-icon tree-item-stash"></span>
                    <span class="tree-item-label">{stash.name}</span>
                    <span class="tree-item-detail">{stash.message}</span>
                  </button>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="tree-empty">{$_("sidebar.stashesEmpty")}</p>
          {/if}
        {/if}
      </div>

      <div class="tree-section">
        <button class="tree-section-toggle" type="button" on:click={() => toggleSidebarSection("submodules")}>
          <span class:expanded={sidebarSections.submodules} class="tree-chevron"></span>
          <span>{$_("sidebar.submodules")}</span>
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
            <p class="tree-empty">{$_("sidebar.submodulesEmpty")}</p>
          {/if}
        {/if}
      </div>
      {/if}
    {/if}
  </section>
</aside>

<style>
  .sidebar {
    min-height: 0;
    padding: 8px;
    border-right: 1px solid var(--row-border);
    background: var(--sidebar-background);
    display: grid;
    grid-template-rows: auto auto 1fr;
    gap: 8px;
    overflow: hidden;
  }

  .sidebar-summary,
  .sidebar-tree {
    background: var(--panel-background);
    border: 1px solid var(--panel-border);
    border-radius: 10px;
    box-shadow: var(--panel-shadow);
  }

  .sidebar-summary {
    display: grid;
    gap: 4px;
    padding: 10px;
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
    margin: 0;
    font-size: 1rem;
    line-height: 1.15;
    color: var(--text-primary);
  }

  .sidebar-branch-name {
    margin: 4px 0 0;
    color: var(--text-secondary);
    font-size: 0.84rem;
    font-weight: 600;
  }

  .repo-status-indicator {
    width: 10px;
    height: 10px;
    flex-shrink: 0;
    margin-top: 4px;
    border-radius: 999px;
    background: var(--warning-fill);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--warning-fill) 25%, transparent);
  }

  .repo-status-indicator.repo-status-clean {
    background: var(--success-fill);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--success-fill) 25%, transparent);
  }

  .path,
  .muted {
    margin: 0;
    color: var(--text-subtle);
  }

  .path {
    font-size: 0.72rem;
    line-height: 1.35;
    word-break: break-all;
  }

  .sidebar-filter-wrap {
    min-height: 0;
  }

  .sidebar-filter {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--input-border);
    border-radius: 10px;
    background: var(--input-background-strong);
    color: var(--text-secondary);
    padding: 10px 12px;
    transition: border-color 120ms ease, box-shadow 120ms ease, background 120ms ease;
  }

  .sidebar-filter:focus {
    outline: none;
    border-color: var(--input-border-focus);
    box-shadow: var(--focus-ring);
    background: var(--input-background-focus);
  }

  .sidebar-tree {
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    display: grid;
    gap: 0;
    align-content: start;
    padding: 10px;
  }

  .tree-section,
  .tree-group,
  .tree-group-details {
    display: grid;
    gap: 0;
  }

  .tree-section-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .tree-section-toggle,
  .tree-group-toggle {
    width: 100%;
    border: 0;
    background: transparent;
    color: var(--text-primary);
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
    color: var(--text-secondary);
    font-size: 0.77rem;
    font-weight: 600;
    padding-left: 0;
    border-radius: 8px;
    min-height: 24px;
  }

  .tree-group-toggle:hover {
    background: var(--hover-overlay-soft);
  }

  .tree-section-toggle-grow {
    min-width: 0;
    flex: 1 1 auto;
  }

  .tree-section-action {
    width: 24px;
    height: 24px;
    flex: 0 0 auto;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--text-secondary);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    line-height: 1;
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease, background 120ms ease, color 120ms ease;
  }

  .tree-section-header:hover .tree-section-action,
  .tree-section-action:focus-visible {
    opacity: 1;
    pointer-events: auto;
  }

  .tree-section-action:hover:enabled {
    background: var(--surface-background-hover);
    color: var(--text-primary);
  }

  .tree-chevron {
    width: 7px;
    height: 7px;
    flex-shrink: 0;
    border-right: 2px solid var(--text-muted);
    border-bottom: 2px solid var(--text-muted);
    transform: rotate(-45deg) translateY(-1px);
    transform-origin: 50% 50%;
    transition: transform 160ms ease;
  }

  .tree-chevron.expanded,
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

  .tree-section-children,
  .tree-nested-list {
    margin-left: 12px;
  }

  .tree-branch-root {
    margin-left: 0;
  }

  .tree-group-details > summary {
    list-style: none;
  }

  .tree-group-details > summary::-webkit-details-marker {
    display: none;
  }

  .tree-group-expanded > .tree-group-toggle,
  .tree-group-details.tree-group-expanded > .tree-group-toggle {
    color: var(--text-primary);
  }

  .tree-item {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 8px;
    align-items: center;
    min-height: 24px;
    padding: 3px 8px 3px 4px;
    border-radius: 8px;
    color: var(--text-muted);
    font-size: 0.78rem;
    box-sizing: border-box;
  }

  .tree-item-button {
    width: 100%;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }

  .tree-item-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px;
    align-items: center;
    min-height: 24px;
    border-radius: 8px;
    padding: 0 4px 0 0;
  }

  .tree-item-row:hover {
    background: var(--hover-overlay);
  }

  .tree-item-row.tree-item-current {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent-soft-border);
  }

  .tree-item-button:hover:enabled {
    background: var(--hover-overlay);
    color: var(--text-primary);
  }

  .tree-item-button:disabled {
    cursor: default;
  }

  .tree-item-stack {
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    padding-top: 3px;
    padding-bottom: 3px;
  }

  .tree-item-tag-button {
    padding-right: 0;
  }

  .tree-item-tag-button:hover:enabled {
    background: transparent;
  }

  .tree-item-icon {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    border-radius: 999px;
    margin-top: 1px;
  }

  .tree-item-tag {
    border-radius: 4px;
    background: linear-gradient(135deg, #f1c56f, #f09a42);
    box-shadow: 0 0 0 1px rgba(239, 169, 72, 0.14);
  }

  .tree-item-remote {
    border-radius: 3px;
    background: linear-gradient(135deg, #8f98a3, #c0cad6);
    box-shadow: 0 0 0 1px rgba(150, 163, 179, 0.14);
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

  .tree-item-detail {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tree-item-label {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tree-item-detail {
    display: none;
  }

  .tree-empty {
    margin: 0;
    padding: 0 4px 2px 12px;
    color: var(--text-subtle);
    font-size: 0.72rem;
    line-height: 1.35;
  }

  .tree-empty.tree-empty-nested {
    padding-left: 24px;
  }

  .tree-item-actions {
    position: relative;
    display: flex;
    align-items: center;
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease;
  }

  .tree-item-row:hover .tree-item-actions,
  .tree-item-actions.menu-open {
    opacity: 1;
    pointer-events: auto;
  }

  .tree-item-kebab {
    width: 24px;
    height: 24px;
    display: inline-flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 3px;
    border: 0;
    border-radius: 7px;
    background: var(--surface-background-strong);
    color: var(--text-secondary);
    padding: 0;
  }

  .tree-item-kebab:hover:enabled {
    background: var(--surface-background-hover);
  }

  .tree-item-kebab span {
    width: 3px;
    height: 3px;
    border-radius: 999px;
    background: currentColor;
  }

  .tree-item-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    min-width: 128px;
    display: grid;
    gap: 2px;
    padding: 6px;
    border-radius: 10px;
    background: var(--dialog-background);
    border: 1px solid var(--surface-border-strong);
    box-shadow: var(--dialog-shadow);
    z-index: 8;
  }

  .tree-item-menu-button {
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--text-secondary);
    padding: 8px 10px;
    font-size: 0.74rem;
    text-align: left;
  }

  .tree-item-menu-button:hover:enabled {
    background: var(--hover-overlay);
  }

  .tree-item-menu-button.tree-item-menu-button-danger {
    color: var(--danger-text);
  }

  .tree-item-menu-button.tree-item-menu-button-danger:hover:enabled {
    background: var(--danger-soft);
  }

  .stash-actions {
    display: grid;
    gap: 8px;
    margin: 2px 0 8px 12px;
    padding: 8px 10px;
    min-width: 0;
    border-radius: 10px;
    background: var(--hover-overlay-soft);
    border: 1px solid var(--panel-border);
  }

  .stash-actions-copy {
    display: grid;
    gap: 2px;
    min-width: 0;
  }

  .stash-actions-copy strong {
    color: var(--text-primary);
    font-size: 0.75rem;
    line-height: 1.2;
  }

  .stash-actions-copy span {
    color: var(--text-muted);
    font-size: 0.69rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .stash-actions-buttons {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 6px;
    min-width: 0;
  }

  .stash-action-button {
    width: 100%;
    min-width: 0;
    border: 0;
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent-contrast);
    padding: 7px 10px;
    font-size: 0.69rem;
    font-weight: 700;
    letter-spacing: 0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .stash-action-button:hover:enabled {
    background: var(--surface-background-hover);
  }

  .stash-action-button-neutral {
    background: var(--surface-background-muted);
    color: var(--text-secondary);
    border: 1px solid var(--surface-border);
  }

  .stash-action-button-neutral:hover:enabled {
    background: var(--surface-background-hover);
  }

  .stash-action-button-danger {
    grid-column: 1 / -1;
    background: var(--danger-soft);
    color: var(--danger-text);
  }

  .stash-action-button-danger:hover:enabled {
    background: color-mix(in srgb, var(--danger-soft) 82%, var(--danger-strong) 18%);
  }

  @media (max-width: 860px) {
    .sidebar {
      padding: 14px;
    }
  }
</style>
