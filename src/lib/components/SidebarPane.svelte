<script>
  export let repository = null;
  export let loading = false;
  export let onSelectRepository = () => {};

  let sidebarFilter = "";
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

  function matchesSidebarFilter(value) {
    if (!sidebarFilterTerm) {
      return true;
    }

    return String(value || "").toLowerCase().includes(sidebarFilterTerm);
  }

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
</script>

<aside class="sidebar">
  <div class="sidebar-toolbar">
    <button class="sidebar-tool sidebar-tool-active" type="button" disabled>Refs</button>
    <button class="sidebar-tool" type="button" on:click={onSelectRepository} disabled={loading}>
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

<style>
  .sidebar {
    min-height: 0;
    padding: 8px;
    border-right: 1px solid rgba(114, 144, 175, 0.06);
    background: linear-gradient(180deg, rgba(10, 17, 26, 0.98), rgba(8, 15, 23, 0.96));
    display: grid;
    grid-template-rows: auto auto auto 1fr;
    gap: 8px;
    overflow: hidden;
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
    background: var(--panel-background);
    border: 1px solid var(--panel-border);
    border-radius: 10px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
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
    color: #f4f8fc;
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

  .path,
  .muted {
    margin: 0;
    color: #6c849c;
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
    border: 1px solid rgba(120, 148, 177, 0.12);
    border-radius: 10px;
    background: rgba(5, 10, 16, 0.92);
    color: #e8eef5;
    padding: 10px 12px;
    transition: border-color 120ms ease, box-shadow 120ms ease, background 120ms ease;
  }

  .sidebar-filter:focus {
    outline: none;
    border-color: rgba(84, 155, 233, 0.7);
    box-shadow: 0 0 0 3px rgba(35, 101, 168, 0.18);
    background: #06101a;
  }

  .sidebar-tree {
    min-height: 0;
    overflow: auto;
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

  @media (max-width: 860px) {
    .sidebar {
      padding: 14px;
    }
  }
</style>
