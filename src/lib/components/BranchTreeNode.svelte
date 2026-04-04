<script>
  export let nodes = [];
  export let loading = false;
  export let currentBranch = "";
  export let remoteName = null;
  export let menuOpenKey = "";
  export let onToggleMenu = () => {};
  export let onCheckoutReference = () => {};
  export let onCreateBranchFromReference = () => {};
  export let onDeleteReference = () => {};

  function isCurrentBranch(node) {
    if (!node.isBranch) {
      return false;
    }

    if (remoteName) {
      return remoteName === "origin" && node.fullName === currentBranch;
    }

    return node.fullName === currentBranch;
  }

  function branchRef(node) {
    return {
      key: node.key,
      kind: remoteName ? "remote_branch" : "local_branch",
      name: node.fullName,
      remoteName,
      displayName: remoteName ? `${remoteName}/${node.fullName}` : node.fullName,
      canCheckout: true,
      canCreateBranch: true,
      canDelete: true,
    };
  }

  function openMenu(node) {
    if (loading || !node.isBranch) {
      return;
    }

    onToggleMenu(menuOpenKey === node.key ? "" : node.key);
  }

  function checkout(node) {
    onToggleMenu("");
    onCheckoutReference(branchRef(node));
  }

  function createBranch(node) {
    onToggleMenu("");
    onCreateBranchFromReference(branchRef(node));
  }

  function deleteBranch(node) {
    onToggleMenu("");
    onDeleteReference(branchRef(node));
  }
</script>

<ul class="branch-tree-list">
  {#each nodes as node (node.key)}
    <li class="branch-tree-node">
      {#if node.isBranch}
        <div class:tree-item-current={isCurrentBranch(node)} class="tree-item-row">
          <div class="tree-item-copy">
            <span class="tree-item-icon tree-item-branch"></span>
            <span class="tree-item-label">{node.label}</span>
          </div>

          <div class:menu-open={menuOpenKey === node.key} class="tree-item-actions">
            <button
              class="tree-item-kebab"
              type="button"
              aria-label="Branch actions"
              disabled={loading}
              on:click={() => openMenu(node)}
            >
              <span></span>
              <span></span>
              <span></span>
            </button>

            {#if menuOpenKey === node.key}
              <div class="tree-item-menu">
                <button class="tree-item-menu-button" type="button" on:click={() => checkout(node)} disabled={loading}>
                  Checkout
                </button>
                <button class="tree-item-menu-button" type="button" on:click={() => createBranch(node)} disabled={loading}>
                  New Branch
                </button>
                <button class="tree-item-menu-button tree-item-menu-button-danger" type="button" on:click={() => deleteBranch(node)} disabled={loading || (!remoteName && isCurrentBranch(node))}>
                  Delete Branch
                </button>
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <details class="branch-folder" open>
          <summary class="tree-group-toggle branch-folder-toggle">
            <span class="tree-chevron"></span>
            <span class="tree-item-icon tree-item-folder"></span>
            <span class="tree-item-label">{node.label}</span>
          </summary>

          <div class="branch-folder-children">
            <svelte:self
              nodes={node.children}
              {loading}
              {currentBranch}
              {remoteName}
              {menuOpenKey}
              {onToggleMenu}
              {onCheckoutReference}
              {onCreateBranchFromReference}
              {onDeleteReference}
            />
          </div>
        </details>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .branch-tree-list {
    display: grid;
    gap: 0;
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .branch-tree-node {
    display: grid;
    gap: 0;
  }

  .tree-item-row,
  .tree-group-toggle {
    width: 100%;
    border: 0;
    background: transparent;
    color: #abbcce;
    display: grid;
    align-items: center;
    min-height: 24px;
    border-radius: 8px;
    box-sizing: border-box;
    font-size: 0.78rem;
    text-align: left;
  }

  .tree-item-row {
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px;
    padding: 3px 4px;
    position: relative;
  }

  .tree-item-copy {
    min-width: 0;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 8px;
    align-items: center;
    padding: 0 4px 0 0;
  }

  .tree-item-row:hover,
  .tree-group-toggle:hover {
    background: rgba(255, 255, 255, 0.04);
    color: #eef5fb;
  }

  .tree-item-row.tree-item-current {
    background: rgba(101, 168, 239, 0.2);
    color: #eef7ff;
    font-weight: 700;
    box-shadow: inset 0 0 0 1px rgba(108, 177, 248, 0.28);
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
    background: rgba(10, 20, 31, 0.92);
    color: #dce8f4;
    padding: 0;
  }

  .tree-item-kebab:hover:enabled {
    background: rgba(25, 50, 75, 0.95);
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
    background: rgba(9, 18, 28, 0.98);
    border: 1px solid rgba(120, 148, 177, 0.16);
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.35);
    z-index: 8;
  }

  .tree-item-menu-button {
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: #dce8f4;
    padding: 8px 10px;
    font-size: 0.74rem;
    text-align: left;
  }

  .tree-item-menu-button:hover:enabled {
    background: rgba(255, 255, 255, 0.05);
  }

  .tree-item-menu-button.tree-item-menu-button-danger {
    color: #ffd3cc;
  }

  .tree-item-menu-button.tree-item-menu-button-danger:hover:enabled {
    background: rgba(156, 70, 46, 0.18);
  }

  .tree-group-toggle {
    grid-template-columns: auto auto minmax(0, 1fr);
    gap: 8px;
    padding: 3px 4px;
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

  .tree-item-folder {
    border-radius: 3px;
    background: linear-gradient(135deg, #6f859c, #93a9c0);
    box-shadow: 0 0 0 1px rgba(131, 152, 175, 0.14);
  }

  .tree-item-label {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .branch-folder {
    display: grid;
    gap: 0;
  }

  .branch-folder > summary {
    list-style: none;
  }

  .branch-folder > summary::-webkit-details-marker {
    display: none;
  }

  .branch-folder-toggle {
    color: #d9e5f1;
    font-weight: 600;
  }

  .branch-folder-children {
    margin-left: 8px;
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

  .branch-folder[open] .tree-chevron {
    transform: rotate(45deg) translateY(-1px);
  }
</style>
