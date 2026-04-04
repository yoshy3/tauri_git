<script>
  export let repository = null;
  export let loading = false;
  export let topActions = [];
  export let onRefresh = () => {};
</script>

<header class="topbar">
  <div class="brand">
    <span class="brand-mark">G</span>
    <div>
      <strong>Tauri Git</strong>
      <p>Minimal git client</p>
    </div>
  </div>

  <div class="toolbar">
    {#each topActions as action}
      <button class="toolbar-button" disabled={!repository}>
        {action}
      </button>
    {/each}
    <button class="toolbar-button active" on:click={onRefresh} disabled={!repository || loading}>
      {loading ? "Syncing..." : "Refresh"}
    </button>
  </div>
</header>

<style>
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

  .toolbar-button {
    background: transparent;
    border: 0;
    color: #8aa0b8;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 0.72rem;
    padding: 10px 12px;
    border-radius: 8px;
  }

  .toolbar-button:hover:enabled {
    background: rgba(255, 255, 255, 0.03);
    color: #dce8f4;
  }

  .toolbar-button.active {
    color: #f2f7fb;
    background: rgba(32, 84, 138, 0.22);
    box-shadow: inset 0 -2px 0 #4da0ff;
  }

  @media (max-width: 860px) {
    .topbar {
      align-items: stretch;
      flex-direction: column;
      padding: 12px 14px;
      gap: 10px;
    }
  }
</style>
