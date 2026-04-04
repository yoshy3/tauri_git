<script>
  export let nodes = [];
  export let loading = false;
  export let currentBranch = "";
  export let remoteName = null;
  export let onCheckoutLocalBranch = () => {};
  export let onCheckoutRemoteBranch = () => {};

  function isCurrentBranch(node) {
    if (!node.isBranch) {
      return false;
    }

    if (remoteName) {
      return remoteName === "origin" && node.fullName === currentBranch;
    }

    return node.fullName === currentBranch;
  }

  function handleBranchClick(node) {
    if (!node.isBranch || loading) {
      return;
    }

    if (remoteName) {
      onCheckoutRemoteBranch(remoteName, node.fullName);
      return;
    }

    if (node.fullName !== currentBranch) {
      onCheckoutLocalBranch(node.fullName);
    }
  }
</script>

<ul class="branch-tree-list">
  {#each nodes as node (node.key)}
    <li class="branch-tree-node">
      {#if node.isBranch}
        <button
          class:tree-item-current={isCurrentBranch(node)}
          class="tree-item tree-item-button"
          type="button"
          disabled={loading}
          on:click={() => handleBranchClick(node)}
        >
          <span class="tree-item-icon tree-item-branch"></span>
          <span class="tree-item-label">{node.label}</span>
        </button>
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
              {onCheckoutLocalBranch}
              {onCheckoutRemoteBranch}
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

  .tree-item,
  .tree-group-toggle {
    width: 100%;
    border: 0;
    background: transparent;
    color: #abbcce;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 8px;
    align-items: center;
    min-height: 24px;
    padding: 3px 8px 3px 4px;
    border-radius: 8px;
    box-sizing: border-box;
    font-size: 0.78rem;
    text-align: left;
  }

  .tree-group-toggle {
    grid-template-columns: auto auto minmax(0, 1fr);
  }

  .tree-item-button {
    cursor: pointer;
  }

  .tree-item-button:hover:enabled,
  .tree-group-toggle:hover {
    background: rgba(255, 255, 255, 0.04);
    color: #eef5fb;
  }

  .tree-item-button:disabled {
    cursor: default;
  }

  .tree-item.tree-item-current {
    background: rgba(101, 168, 239, 0.2);
    color: #eef7ff;
    font-weight: 700;
    box-shadow: inset 0 0 0 1px rgba(108, 177, 248, 0.28);
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
    margin-left: 12px;
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
