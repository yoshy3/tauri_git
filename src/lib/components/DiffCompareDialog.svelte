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
  $: if (open && rows.length > 0) {
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

      <div class="diff-dialog-column-head">
        <div>{$_("diffDialog.before")}</div>
        <div>{$_("diffDialog.after")}</div>
      </div>

      {#if rows.length > 0}
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
    background: rgba(2, 7, 12, 0.72);
    backdrop-filter: blur(6px);
    display: grid;
    place-items: center;
    padding: 24px;
    box-sizing: border-box;
  }

  .diff-dialog {
    width: min(1280px, 100%);
    height: min(820px, calc(100vh - 48px));
    display: grid;
    grid-template-rows: auto auto 1fr;
    overflow: hidden;
    border-radius: 14px;
    background: linear-gradient(180deg, rgba(10, 19, 28, 0.98), rgba(8, 16, 24, 0.98));
    border: 1px solid rgba(120, 148, 177, 0.12);
    box-shadow: 0 28px 64px rgba(0, 0, 0, 0.48);
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
    border-bottom: 1px solid rgba(120, 148, 177, 0.08);
  }

  .diff-dialog-copy {
    min-width: 0;
  }

  .diff-dialog-copy h2 {
    margin: 0;
    color: #eef5fb;
    font-size: 1rem;
  }

  .diff-dialog-copy p {
    margin: 6px 0 0;
    color: #8aa0b8;
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
    background: rgba(144, 92, 14, 0.22);
    color: #ffd48a;
    font-size: 0.72rem;
    font-weight: 700;
  }

  .diff-dialog-close {
    width: 32px;
    height: 32px;
    border: 1px solid rgba(120, 148, 177, 0.12);
    border-radius: 999px;
    background: rgba(12, 24, 38, 0.96);
    color: #eef5fb;
    font-size: 1rem;
  }

  .diff-dialog-close:hover {
    background: rgba(20, 35, 49, 1);
  }

  .diff-dialog-column-head {
    grid-template-columns: 1fr 1fr;
    gap: 0;
    padding: 0;
    border-bottom: 1px solid rgba(120, 148, 177, 0.08);
    background: rgba(8, 16, 24, 0.62);
  }

  .diff-dialog-column-head div {
    padding: 10px 18px;
    color: #8aa0b8;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .diff-dialog-column-head div + div {
    border-left: 1px solid rgba(120, 148, 177, 0.08);
  }

  .diff-dialog-body {
    overflow: auto;
    background: #0a1118;
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
      rgba(13, 22, 31, 0.96),
      rgba(15, 27, 39, 0.98)
    );
    border-top: 1px solid rgba(120, 148, 177, 0.08);
    border-bottom: 1px solid rgba(120, 148, 177, 0.08);
  }

  .diff-gap-line {
    height: 1px;
    background: linear-gradient(
      90deg,
      rgba(120, 148, 177, 0),
      rgba(120, 148, 177, 0.24),
      rgba(120, 148, 177, 0)
    );
  }

  .diff-gap-label {
    color: #88a5c2;
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
    border-left: 1px solid rgba(120, 148, 177, 0.08);
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
    color: #688099;
    user-select: none;
    border-right: 1px solid rgba(120, 148, 177, 0.06);
  }

  .diff-line-text {
    color: #dce7f2;
  }

  .diff-inline-changed {
    padding: 0 1px;
    border-radius: 3px;
  }

  .diff-side-added {
    background: rgba(74, 163, 108, 0.16);
  }

  .diff-side-added .diff-line-number {
    color: #83d3a1;
    background: rgba(46, 96, 62, 0.28);
  }

  .diff-side-added .diff-line-text {
    color: #dff5e7;
  }

  .diff-side-removed {
    background: rgba(168, 76, 76, 0.16);
  }

  .diff-side-removed .diff-line-number {
    color: #f1a6a6;
    background: rgba(104, 42, 42, 0.3);
  }

  .diff-side-removed .diff-line-text {
    color: #f6e1e1;
  }

  .diff-side-modified-left {
    background: rgba(122, 95, 42, 0.16);
  }

  .diff-side-modified-left .diff-line-number {
    color: #e8c587;
    background: rgba(93, 70, 29, 0.32);
  }

  .diff-side-modified-left .diff-line-text {
    color: #f2e7c9;
  }

  .diff-side-modified-right {
    background: rgba(64, 119, 84, 0.18);
  }

  .diff-side-modified-right .diff-line-number {
    color: #8ed5a6;
    background: rgba(39, 84, 54, 0.34);
  }

  .diff-side-modified-right .diff-line-text {
    color: #ddf5e6;
  }

  .diff-side-removed .diff-inline-changed {
    background: rgba(255, 129, 129, 0.28);
    color: #fff0f0;
  }

  .diff-side-added .diff-inline-changed {
    background: rgba(118, 221, 150, 0.28);
    color: #f2fff7;
  }

  .diff-side-modified-left .diff-inline-changed {
    background: rgba(248, 196, 92, 0.34);
    color: #fff4d6;
  }

  .diff-side-modified-right .diff-inline-changed {
    background: rgba(129, 225, 159, 0.3);
    color: #f0fff4;
  }

  .diff-dialog-empty {
    padding: 20px 18px;
    color: #8aa0b8;
  }

  .diff-dialog-empty p {
    margin: 0;
  }

  @media (max-width: 900px) {
    .diff-dialog {
      height: min(860px, calc(100vh - 24px));
    }

    .diff-row,
    .diff-dialog-column-head {
      grid-template-columns: 1fr;
    }

    .diff-side + .diff-side,
    .diff-dialog-column-head div + div {
      border-left: 0;
      border-top: 1px solid rgba(120, 148, 177, 0.08);
    }
  }
</style>
