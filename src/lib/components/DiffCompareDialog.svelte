<script>
  import { tick } from "svelte";
  import { _ } from "svelte-i18n";
  import { parseUnifiedDiff } from "../diff/sideBySideDiff";

  export let open = false;
  export let filePath = "";
  export let patchText = "";
  export let status = "";
  export let onClose = () => {};

  let bodyElement;

  function handleBackdropClick(event) {
    if (event.target === event.currentTarget) {
      onClose();
    }
  }

  async function scrollToFirstChange() {
    if (!open || !bodyElement || rows.length === 0) {
      return;
    }

    await tick();

    const firstChangedRow = bodyElement.querySelector(
      '[data-diff-kind="modified"], [data-diff-kind="added"], [data-diff-kind="removed"]',
    );

    if (!(firstChangedRow instanceof HTMLElement)) {
      bodyElement.scrollTop = 0;
      return;
    }

    bodyElement.scrollTop = Math.max(firstChangedRow.offsetTop - 72, 0);
  }

  $: rows = parseUnifiedDiff(patchText);
  $: hasComparableRows = rows.length > 0;
  $: hasRawPatch = patchText.trim().length > 0;
  $: if (open && hasComparableRows) {
    scrollToFirstChange();
  }
</script>

<svelte:window
  on:keydown={(event) => {
    if (open && event.key === "Escape") {
      onClose();
    }
  }}
/>

{#if open}
  <div class="diff-dialog-backdrop" role="presentation" on:click={handleBackdropClick}>
    <section class="diff-dialog" role="dialog" aria-modal="true" aria-labelledby="diff-dialog-title">
      <header class="diff-dialog-header">
        <div class="diff-dialog-copy">
          <h2 id="diff-dialog-title">{$_("diffDialog.title")}</h2>
          <p title={filePath}>{filePath}</p>
        </div>

        <div class="diff-dialog-actions">
          {#if status}
            <span class="diff-dialog-status">{status}</span>
          {/if}
          <button class="diff-dialog-close" type="button" aria-label={$_("diffDialog.close")} on:click={onClose}>
            ×
          </button>
        </div>
      </header>

      {#if hasComparableRows}
        <div class="diff-dialog-column-head">
          <div>{$_("diffDialog.before")}</div>
          <div>{$_("diffDialog.after")}</div>
        </div>

        <div class="diff-dialog-body" bind:this={bodyElement}>
          {#each rows as row}
            {#if row.kind === "gap"}
              <div class="diff-gap-row" role="presentation">
                <span class="diff-gap-line" />
                <span class="diff-gap-label">{row.leftText}</span>
                <span class="diff-gap-line" />
              </div>
            {:else}
              <div class="diff-row" data-diff-kind={row.kind}>
                <div
                  class:diff-side-modified-left={row.kind === "modified"}
                  class:diff-side-removed={row.kind === "removed"}
                  class="diff-side"
                >
                  <span class="diff-line-number">{row.leftNumber}</span>
                  <span class="diff-line-text">
                    {#each row.leftChunks as chunk}
                      <span class:diff-inline-changed={chunk.changed}>{chunk.text}</span>
                    {/each}
                  </span>
                </div>

                <div
                  class:diff-side-added={row.kind === "added"}
                  class:diff-side-modified-right={row.kind === "modified"}
                  class="diff-side"
                >
                  <span class="diff-line-number">{row.rightNumber}</span>
                  <span class="diff-line-text">
                    {#each row.rightChunks as chunk}
                      <span class:diff-inline-changed={chunk.changed}>{chunk.text}</span>
                    {/each}
                  </span>
                </div>
              </div>
            {/if}
          {/each}
        </div>
      {:else if hasRawPatch}
        <div class="diff-dialog-body diff-dialog-body-raw">
          <pre class="diff-dialog-raw-patch">{patchText}</pre>
        </div>
      {:else}
        <div class="diff-dialog-empty">
          <p>{$_("diffDialog.empty")}</p>
        </div>
      {/if}
    </section>
  </div>
{/if}

<style>
  .diff-dialog-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: var(--modal-backdrop);
    backdrop-filter: blur(6px);
    display: grid;
    place-items: center;
    padding: 24px;
    box-sizing: border-box;
  }

  .diff-dialog {
    width: max(0px, calc(100vw - 48px));
    height: max(0px, calc(100vh - 48px));
    max-width: none;
    max-height: none;
    display: grid;
    grid-template-rows: auto auto 1fr;
    overflow: hidden;
    border-radius: 14px;
    background: var(--dialog-background);
    border: 1px solid var(--surface-border);
    box-shadow: var(--modal-shadow);
  }

  .diff-dialog-header,
  .diff-dialog-column-head {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 12px;
    align-items: center;
    padding: 14px 18px;
  }

  .diff-dialog-header {
    border-bottom: 1px solid var(--panel-border);
  }

  .diff-dialog-copy {
    min-width: 0;
  }

  .diff-dialog-copy h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
  }

  .diff-dialog-copy p {
    margin: 6px 0 0;
    color: var(--text-muted);
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .diff-dialog-actions {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }

  .diff-dialog-status {
    padding: 4px 9px;
    border-radius: 999px;
    background: var(--warning-soft);
    color: var(--warning-text);
    font-size: 0.72rem;
    font-weight: 700;
  }

  .diff-dialog-close {
    width: 32px;
    height: 32px;
    border: 1px solid var(--surface-border);
    border-radius: 999px;
    background: var(--surface-background-strong);
    color: var(--text-primary);
    font-size: 1rem;
  }

  .diff-dialog-close:hover {
    background: var(--surface-background-hover);
  }

  .diff-dialog-column-head {
    grid-template-columns: 1fr 1fr;
    gap: 0;
    padding: 0;
    border-bottom: 1px solid var(--panel-border);
    background: var(--panel-soft-background);
  }

  .diff-dialog-column-head div {
    padding: 10px 18px;
    color: var(--text-muted);
    font-size: 0.72rem;
    letter-spacing: 0.08em;
  }

  .diff-dialog-column-head div + div {
    border-left: 1px solid var(--panel-border);
  }

  .diff-dialog-body {
    overflow: auto;
    background: var(--patch-background);
  }

  .diff-dialog-body-raw {
    padding: 14px 18px 18px;
  }

  .diff-dialog-raw-patch {
    margin: 0;
    color: var(--text-secondary);
    font-family: "SFMono-Regular", "Menlo", monospace;
    font-size: 0.76rem;
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .diff-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }

  .diff-gap-row {
    display: grid;
    grid-template-columns: minmax(32px, 1fr) auto minmax(32px, 1fr);
    gap: 12px;
    align-items: center;
    padding: 10px 18px;
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--panel-soft-background) 88%, transparent),
      color-mix(in srgb, var(--surface-background) 92%, transparent)
    );
    border-top: 1px solid var(--panel-border);
    border-bottom: 1px solid var(--panel-border);
  }

  .diff-gap-line {
    height: 1px;
    background: linear-gradient(
      90deg,
      rgba(120, 148, 177, 0),
      var(--surface-border-strong),
      rgba(120, 148, 177, 0)
    );
  }

  .diff-gap-label {
    color: var(--text-muted);
    font-size: 0.72rem;
    letter-spacing: 0.03em;
    white-space: nowrap;
  }

  .diff-side {
    min-width: 0;
    display: grid;
    grid-template-columns: 54px minmax(0, 1fr);
    gap: 0;
    font-family: "SFMono-Regular", "Menlo", monospace;
    font-size: 0.76rem;
    line-height: 1.45;
  }

  .diff-side + .diff-side {
    border-left: 1px solid var(--panel-border);
  }

  .diff-line-number,
  .diff-line-text {
    display: block;
    padding: 0 10px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .diff-line-number {
    text-align: right;
    color: var(--text-subtle);
    user-select: none;
    border-right: 1px solid var(--row-border);
  }

  .diff-line-text {
    color: var(--text-secondary);
  }

  .diff-inline-changed {
    padding: 0 2px;
    margin: 0 -1px;
    border-radius: 3px;
    box-decoration-break: clone;
    -webkit-box-decoration-break: clone;
  }

  .diff-side-added {
    background: var(--success-soft-row);
  }

  .diff-side-added .diff-line-number {
    color: var(--success-text);
    background: color-mix(in srgb, var(--success-soft-row) 75%, black 10%);
  }

  .diff-side-added .diff-line-text {
    color: var(--text-primary);
  }

  .diff-side-removed {
    background: var(--danger-soft-row);
  }

  .diff-side-removed .diff-line-number {
    color: var(--danger-text);
    background: color-mix(in srgb, var(--danger-soft-row) 78%, black 8%);
  }

  .diff-side-removed .diff-line-text {
    color: var(--text-primary);
  }

  .diff-side-modified-left {
    background: var(--warning-soft-row);
  }

  .diff-side-modified-left .diff-line-number {
    color: var(--warning-text);
    background: color-mix(in srgb, var(--warning-soft-row) 78%, black 8%);
  }

  .diff-side-modified-left .diff-line-text {
    color: var(--text-primary);
  }

  .diff-side-modified-right {
    background: var(--modified-soft-row);
  }

  .diff-side-modified-right .diff-line-number {
    color: var(--success-text);
    background: color-mix(in srgb, var(--modified-soft-row) 78%, black 8%);
  }

  .diff-side-modified-right .diff-line-text {
    color: var(--text-primary);
  }

  .diff-side-removed .diff-inline-changed {
    background: var(--diff-inline-removed-bg);
    box-shadow: inset 0 0 0 1px var(--diff-inline-removed-border);
    color: var(--text-primary);
  }

  .diff-side-added .diff-inline-changed {
    background: var(--diff-inline-added-bg);
    box-shadow: inset 0 0 0 1px var(--diff-inline-added-border);
    color: var(--text-primary);
  }

  .diff-side-modified-left .diff-inline-changed {
    background: var(--diff-inline-modified-left-bg);
    box-shadow: inset 0 0 0 1px var(--diff-inline-modified-left-border);
    color: var(--text-primary);
  }

  .diff-side-modified-right .diff-inline-changed {
    background: var(--diff-inline-modified-right-bg);
    box-shadow: inset 0 0 0 1px var(--diff-inline-modified-right-border);
    color: var(--text-primary);
  }

  .diff-dialog-empty {
    padding: 20px 18px;
    color: var(--text-muted);
  }

  .diff-dialog-empty p {
    margin: 0;
  }

  @media (max-width: 900px) {
    .diff-dialog {
      width: max(0px, calc(100vw - 24px));
      height: max(0px, calc(100vh - 24px));
    }

    .diff-row,
    .diff-dialog-column-head {
      grid-template-columns: 1fr;
    }

    .diff-side + .diff-side,
    .diff-dialog-column-head div + div {
      border-left: 0;
      border-top: 1px solid var(--panel-border);
    }
  }
</style>
