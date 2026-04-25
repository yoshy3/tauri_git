<script>
  import { _, locale } from "svelte-i18n";
  import { tick } from "svelte";
  import { setAppLocale } from "../i18n";

  export let repository = null;
  export let loading = false;
  export let theme = "dark";
  export let recentRepositoryPaths = [];
  export let topActions = [];
  export let implementedActions = [];
  export let activeAction = "";
  export let canResetSelectedCommit = false;
  export let onAction = () => {};
  export let onSelectRepository = () => {};
  export let onOpenRecentRepository = () => {};
  export let onResetSelectedCommit = () => {};
  export let onRefresh = () => {};
  export let onToggleTheme = () => {};
  let recentMenuOpen = false;
  let recentMenuElement;

  function actionKey(action) {
    return `topbar.${action.toLowerCase()}`;
  }

  function iconType(action) {
    return action.toLowerCase();
  }

  function actionBadgeCount(action) {
    if (!repository) {
      return 0;
    }

    if (action === "Pull") {
      return repository.behind_count ?? 0;
    }

    if (action === "Push") {
      return repository.ahead_count ?? 0;
    }

    return 0;
  }

  $: themeToggleLabel = theme === "dark" ? $_("topbar.switchToLight") : $_("topbar.switchToDark");
  $: themeCaption = $_("topbar.theme");

  async function toggleRecentMenu() {
    recentMenuOpen = !recentMenuOpen;
    if (recentMenuOpen) {
      await tick();
      recentMenuElement?.querySelector("button")?.focus();
    }
  }

  function closeRecentMenu() {
    recentMenuOpen = false;
  }

  async function openRecent(path) {
    closeRecentMenu();
    await onOpenRecentRepository(path);
  }

  function handleDocumentPointerDown(event) {
    if (!recentMenuOpen) {
      return;
    }

    const target = event.target;
    if (!(target instanceof Element)) {
      return;
    }

    if (!target.closest(".open-split-button")) {
      closeRecentMenu();
    }
  }
</script>

<svelte:document on:pointerdown={handleDocumentPointerDown} />

<header class="topbar">
  <div class="topbar-leading">
    <div class="brand">
      <span class="brand-mark">G</span>
      <div>
        <strong>Tauri Git</strong>
        <p>{$_("topbar.subtitle")}</p>
      </div>
    </div>

    <div class="open-split-button">
      <button class="toolbar-button toolbar-button-open" type="button" on:click={onSelectRepository} disabled={loading}>
        <span class="button-icon-wrap">
          <span class="button-icon" aria-hidden="true">
            <svg class="toolbar-svg toolbar-svg-open" viewBox="0 0 16 16" fill="none">
              <path d="M2.75 4.25h3l1.25 1.5h6.25v6.5H2.75z" />
              <path d="M2.75 6.25h10.5" />
            </svg>
          </span>
        </span>
        <span class="button-label">{loading ? $_("topbar.opening") : $_("topbar.open")}</span>
      </button>
      <button
        class="toolbar-button toolbar-button-split-toggle"
        type="button"
        aria-label={$_("topbar.openRecent")}
        aria-expanded={recentMenuOpen}
        on:click={toggleRecentMenu}
        disabled={loading}
      >
        <span class="button-icon-wrap">
          <span class="button-icon button-icon-chevron" aria-hidden="true">▾</span>
        </span>
      </button>

      {#if recentMenuOpen}
        <div bind:this={recentMenuElement} class="open-recent-menu">
          {#if recentRepositoryPaths.length > 0}
            {#each recentRepositoryPaths as path}
              <button class="open-recent-item" type="button" on:click={() => openRecent(path)} title={path}>
                <span class="open-recent-primary">{path.split("/").filter(Boolean).at(-1) ?? path}</span>
                <span class="open-recent-secondary">{path}</span>
              </button>
            {/each}
          {:else}
            <div class="open-recent-empty">{$_("topbar.noRecentRepositories")}</div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <div class="toolbar-cluster">
    <div class="toolbar">
      {#each topActions as action}
        {@const badgeCount = actionBadgeCount(action)}
        <button
          class:active={activeAction === action}
          class="toolbar-button"
          disabled={!repository || loading || !implementedActions.includes(action)}
          on:click={() => onAction(action)}
        >
          <span class="button-icon-wrap">
            <span class="button-icon" aria-hidden="true">
              {#if iconType(action) === "fetch"}
                <svg class="toolbar-svg toolbar-svg-fetch" viewBox="0 0 16 16" fill="none">
                  <path d="M8 2.5v8" />
                  <path d="M5.25 7.75 8 10.5l2.75-2.75" />
                  <path d="M3 12.5h10" />
                </svg>
              {:else if iconType(action) === "pull"}
                <svg class="toolbar-svg toolbar-svg-pull" viewBox="0 0 16 16" fill="none">
                  <path d="M8 2.5v8" />
                  <path d="M5.25 7.75 8 10.5l2.75-2.75" />
                  <path d="M4 5.25a4 4 0 0 1 8 0" />
                </svg>
              {:else if iconType(action) === "push"}
                <svg class="toolbar-svg toolbar-svg-push" viewBox="0 0 16 16" fill="none">
                  <path d="M8 13.5v-8" />
                  <path d="M10.75 8.25 8 5.5 5.25 8.25" />
                  <path d="M4 10.75a4 4 0 0 0 8 0" />
                </svg>
              {:else if iconType(action) === "stash"}
                <svg class="toolbar-svg toolbar-svg-stash" viewBox="0 0 16 16" fill="none">
                  <path d="M3 5.25 8 2.5l5 2.75-5 2.75-5-2.75Z" />
                  <path d="M3 8.25 8 11l5-2.75" />
                  <path d="M3 11.25 8 14l5-2.75" />
                </svg>
              {:else if iconType(action) === "discard"}
                <svg class="toolbar-svg toolbar-svg-discard" viewBox="0 0 16 16" fill="none">
                  <path d="M3.75 4.5h8.5" />
                  <path d="M6.25 2.75h3.5" />
                  <path d="M5 4.5v7.25" />
                  <path d="M8 4.5v7.25" />
                  <path d="M11 4.5v7.25" />
                  <path d="M4.5 4.5l.6 8.1c.04.5.45.9.95.9h3.9c.5 0 .91-.4.95-.9l.6-8.1" />
                </svg>
              {/if}
            </span>
            {#if badgeCount > 0}
              <span class="toolbar-badge" aria-label={`${action} ${badgeCount} commits pending`}>
                {badgeCount}
              </span>
            {/if}
          </span>
          <span class="button-label">{activeAction === action ? $_("topbar.syncing") : $_(actionKey(action))}</span>
        </button>
      {/each}
      <button class="toolbar-button" disabled={!repository || loading || !canResetSelectedCommit} on:click={onResetSelectedCommit}>
        <span class="button-icon-wrap">
          <span class="button-icon" aria-hidden="true">
            <svg class="toolbar-svg toolbar-svg-reset" viewBox="0 0 16 16" fill="none">
              <path d="M4.25 5.5A4.75 4.75 0 1 1 3.5 8" />
              <path d="M3.5 3.75v3h3" />
            </svg>
          </span>
        </span>
        <span class="button-label">{$_("topbar.reset")}</span>
      </button>
      <button class:active={activeAction === "Refresh"} class="toolbar-button" on:click={onRefresh} disabled={!repository || loading}>
        <span class="button-icon-wrap">
          <span class="button-icon" aria-hidden="true">
            <svg class="toolbar-svg toolbar-svg-refresh" viewBox="0 0 16 16" fill="none">
              <path d="M12.75 6A5 5 0 1 0 13 8" />
              <path d="M10.75 3.25h2v2" />
            </svg>
          </span>
        </span>
        <span class="button-label">{activeAction === "Refresh" ? $_("topbar.syncing") : $_("topbar.refresh")}</span>
      </button>
    </div>

    <div class="locale-area">
      <span class="locale-label">{themeCaption}:</span>
      <button
        class="theme-toggle"
        type="button"
        on:click={onToggleTheme}
        aria-label={themeToggleLabel}
        title={themeToggleLabel}
      >
        <span class="button-icon" aria-hidden="true">
          {#if theme === "dark"}
            <svg class="theme-svg" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="3.2" />
              <path d="M8 1.75v1.5" />
              <path d="M8 12.75v1.5" />
              <path d="M12.25 8h1.5" />
              <path d="M2.25 8h1.5" />
              <path d="m12.42 3.58-1.06 1.06" />
              <path d="M4.64 11.36 3.58 12.42" />
              <path d="m12.42 12.42-1.06-1.06" />
              <path d="M4.64 4.64 3.58 3.58" />
            </svg>
          {:else}
            <svg class="theme-svg" viewBox="0 0 16 16" fill="none">
              <path d="M11.9 10.47A5.25 5.25 0 0 1 5.53 4.1 5.5 5.5 0 1 0 11.9 10.47Z" />
            </svg>
          {/if}
        </span>
      </button>
    </div>

    <div class="locale-area">
      <span class="locale-label">{$_("topbar.language")}:</span>
      <div class="locale-switcher" aria-label={$_("topbar.language")}>
        <button
          class:locale-active={$locale === "en"}
          class="locale-button"
          type="button"
          on:click={() => setAppLocale("en")}
          title="English"
          aria-label="English"
        >
          <span class="button-icon" aria-hidden="true">
            <span class="locale-glyph">A</span>
          </span>
        </button>
        <button
          class:locale-active={$locale === "ja"}
          class="locale-button"
          type="button"
          on:click={() => setAppLocale("ja")}
          title="日本誁E
          aria-label="日本誁E
        >
          <span class="button-icon" aria-hidden="true">
            <span class="locale-glyph locale-glyph-ja">日</span>
          </span>
        </button>
      </div>
    </div>
  </div>
</header>

<style>
  .topbar {
    position: relative;
    z-index: 20;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 14px 0 10px;
    border-bottom: 1px solid var(--header-border);
    background: var(--header-background);
    backdrop-filter: blur(20px);
    box-shadow: inset 0 -1px 0 rgba(255, 255, 255, 0.02);
  }

  .topbar-leading {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
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
    color: var(--text-subtle);
    font-size: 0.75rem;
  }

  .brand-mark {
    width: 32px;
    height: 32px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, var(--accent-strong), var(--accent));
    color: white;
    font-weight: 700;
    box-shadow: 0 8px 18px rgba(32, 108, 184, 0.28);
  }

  .toolbar-cluster {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .toolbar {
    display: flex;
    gap: 2px;
    flex-wrap: wrap;
    align-items: stretch;
  }

  .toolbar-button {
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    gap: 7px;
    background: transparent;
    border: 0;
    color: var(--text-muted);
    letter-spacing: 0.08em;
    font-size: 0.72rem;
    min-width: 66px;
    min-height: 64px;
    padding: 7px 10px 10px;
    border-radius: 8px;
  }

  .toolbar-button:hover:enabled {
    background: var(--hover-overlay-soft);
    color: var(--text-secondary);
  }

  .toolbar-button.active {
    color: var(--text-primary);
    background: var(--accent-soft);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .open-split-button {
    position: relative;
    z-index: 21;
    display: inline-flex;
    align-items: stretch;
    gap: 0;
    border-radius: 8px;
  }

  .toolbar-button-open {
    min-width: 70px;
    padding-right: 4px;
    border-radius: 0;
  }

  .toolbar-button-split-toggle {
    min-width: 24px;
    width: 24px;
    padding: 0;
    justify-content: center;
    gap: 0;
    border-radius: 0;
    position: relative;
    margin-left: -8px;
  }

  .toolbar-button-split-toggle::before {
    content: "";
    position: absolute;
    left: 0;
    top: 12px;
    bottom: 12px;
    width: 1px;
    background: color-mix(in srgb, var(--surface-border) 80%, transparent);
  }

  .button-icon-chevron {
    font-size: 0.9rem;
    line-height: 1;
    align-items: center;
    justify-content: center;
    transform: translateY(0);
  }

  .open-recent-menu {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    width: min(360px, 60vw);
    max-height: 280px;
    overflow: auto;
    display: grid;
    gap: 4px;
    padding: 8px;
    border: 1px solid var(--surface-border);
    border-radius: 12px;
    background: var(--panel-background);
    box-shadow: var(--panel-shadow);
    z-index: 22;
  }

  .open-recent-item {
    width: 100%;
    display: grid;
    gap: 3px;
    text-align: left;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: inherit;
    padding: 8px 10px;
  }

  .open-recent-item:hover {
    background: var(--hover-overlay-soft);
  }

  .open-recent-primary {
    color: var(--text-primary);
    font-size: 0.8rem;
    font-weight: 600;
  }

  .open-recent-secondary {
    color: var(--text-muted);
    font-size: 0.7rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .open-recent-empty {
    color: var(--text-muted);
    font-size: 0.78rem;
    padding: 8px 10px;
  }

  .locale-switcher {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border-radius: 10px;
    background: var(--surface-background);
    border: 1px solid var(--surface-border);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
  }

  .locale-area {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .locale-label {
    color: var(--text-muted);
    font-size: 0.74rem;
    font-weight: 600;
    letter-spacing: 0.04em;
  }

  .locale-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 34px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--text-muted);
    padding: 8px;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.08em;
  }

  .locale-button:hover {
    background: var(--hover-overlay-soft);
    color: var(--text-secondary);
  }

  .locale-button.locale-active {
    background: var(--accent-soft);
    color: var(--text-primary);
    box-shadow: inset 0 -2px 0 var(--accent);
  }

  .theme-toggle {
    display: inline-grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border: 1px solid var(--surface-border);
    border-radius: 10px;
    background: var(--surface-background);
    color: var(--text-secondary);
    padding: 0;
  }

  .theme-toggle:hover {
    background: var(--surface-background-hover);
  }

  .button-icon {
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .button-icon-wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    flex-shrink: 0;
  }

  .button-label {
    display: flex;
    align-items: flex-start;
    justify-content: center;
    min-height: 1.2em;
    line-height: 1.1;
    text-align: center;
  }

  .toolbar-svg,
  .theme-svg {
    width: 22px;
    height: 22px;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .toolbar-svg-fetch {
    transform: translateY(1px);
  }

  .toolbar-svg-pull {
    transform: translateY(3.5px);
  }

  .toolbar-svg-push {
    transform: translateY(-1.5px);
  }

  .toolbar-svg-stash {
    transform: translateY(0.5px);
  }

  .toolbar-svg-discard {
    transform: translateY(0.5px);
  }

  .toolbar-svg-refresh {
    transform: translateY(0.5px);
  }

  .toolbar-svg-reset {
    transform: translateY(0.5px);
  }

  .toolbar-svg-open {
    transform: translateY(0.5px);
  }

  .toolbar-badge {
    position: absolute;
    top: -1px;
    right: -9px;
    min-width: 14px;
    height: 14px;
    padding: 0 2px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 999px;
    background: #e5484d;
    color: #fff8f8;
    font-size: 0.5rem;
    font-weight: 800;
    line-height: 1;
    letter-spacing: 0;
    box-shadow: 0 0 0 2px var(--surface-background-strong);
    pointer-events: none;
  }

  .locale-glyph {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    font-size: 0.82rem;
    font-weight: 800;
    line-height: 1;
    letter-spacing: 0;
  }

  .locale-glyph-ja {
    font-size: 0.76rem;
  }

  @media (max-width: 860px) {
    .topbar {
      align-items: stretch;
      flex-direction: column;
      padding: 12px 14px;
      gap: 10px;
    }

    .topbar-leading {
      justify-content: space-between;
    }

    .open-recent-menu {
      width: min(320px, calc(100vw - 28px));
    }

    .toolbar-cluster {
      flex-direction: column;
      align-items: stretch;
    }
  }
</style>
