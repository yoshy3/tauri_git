<script>
  import { tick } from "svelte";
  import { _ } from "svelte-i18n";

  export let repository = null;
  export let error = "";
  export let historyCommits = [];
  export let historyLoading = false;
  export let historyLoadedAll = false;
  export let selectedCommitOid = "";
  export let selectedCommitDetail = null;
  export let selectedCommitDetailLoading = false;
  export let onSelectCommit = () => {};
  export let onCloseCommitDetail = () => {};

  const graphLaneSpacing = 14;
  const graphPadding = 10;
  const graphRowHeight = 32;
  const graphCenterY = 16;
  const graphOverlap = 3;
  const graphCurveInset = 5;
  const graphPalette = ["#67b3ff", "#ffb454", "#7be495", "#f7768e", "#c792ea", "#7dcfff"];

  function initials(name) {
    return (name || "?")
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase() || "")
      .join("");
  }

  function formatLocalDateTime(value) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return value;
    }

    return new Intl.DateTimeFormat(undefined, {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    }).format(date);
  }

  function formatDetailPatchLineClass(line) {
    if (line.startsWith("@@")) {
      return "patch-line patch-line-hunk";
    }
    if (line.startsWith("+")) {
      return "patch-line patch-line-added";
    }
    if (line.startsWith("-")) {
      return "patch-line patch-line-removed";
    }
    return "patch-line";
  }

  function clampDetailPanelHeight(height) {
    const maxHeight = Math.max(detailPanelMinHeight, windowHeight - 220);
    return Math.min(Math.max(height, detailPanelMinHeight), maxHeight);
  }

  function ensureSelectedCommitVisible() {
    if (!selectedCommitOid || !historyRowsElement) {
      return;
    }

    const rowButton = historyRowsElement.querySelector(`[data-commit-oid="${selectedCommitOid}"]`);
    if (!rowButton) {
      return;
    }

    const containerRect = historyRowsElement.getBoundingClientRect();
    const rowRect = rowButton.getBoundingClientRect();

    if (rowRect.top < containerRect.top) {
      historyRowsElement.scrollTop -= containerRect.top - rowRect.top + 8;
      return;
    }

    if (rowRect.bottom > containerRect.bottom) {
      historyRowsElement.scrollTop += rowRect.bottom - containerRect.bottom + 8;
    }
  }

  function handleSplitterPointerDown(event) {
    if (!detailPanelVisible) {
      return;
    }

    detailPanelDragging = true;
    detailPanelHeightManuallySet = true;
    detailDragStartY = event.clientY;
    detailDragStartHeight = activeDetailPanelHeight;
  }

  function handleWindowPointerMove(event) {
    if (!detailPanelDragging) {
      return;
    }

    detailPanelHeight = clampDetailPanelHeight(detailDragStartHeight - (event.clientY - detailDragStartY));
  }

  function handleWindowPointerUp() {
    detailPanelDragging = false;
  }

  function dedupeLaneEntries(entries) {
    const seen = new Set();
    const result = [];

    entries.forEach((entry) => {
      if (seen.has(entry.oid)) {
        return;
      }

      seen.add(entry.oid);
      result.push(entry);
    });

    return result;
  }

  function buildVerticalPath(x, startY, endY) {
    return `M ${x} ${startY} L ${x} ${endY}`;
  }

  function buildLaneTransitionPath(startX, startY, endX, endY) {
    if (startX === endX) {
      return buildVerticalPath(startX, startY, endY);
    }

    const direction = endY > startY ? 1 : -1;
    const turnStartY = startY + graphCurveInset * direction;
    const turnEndY = endY - graphCurveInset * direction;
    const midY = (turnStartY + turnEndY) / 2;

    return [
      `M ${startX} ${startY}`,
      `L ${startX} ${turnStartY}`,
      `C ${startX} ${midY} ${endX} ${midY} ${endX} ${turnEndY}`,
      `L ${endX} ${endY}`,
    ].join(" ");
  }

  function buildGraphRows(commits) {
    let lanes = [];
    let maxLaneCount = 1;
    let colorCursor = 0;
    const rows = [];

    function nextColor() {
      const color = graphPalette[colorCursor % graphPalette.length];
      colorCursor += 1;
      return color;
    }

    commits.forEach((commit, rowIndex) => {
      const existingIndex = lanes.findIndex((lane) => lane.oid === commit.oid);
      const inserted = existingIndex === -1;
      const before = inserted
        ? [...lanes, { oid: commit.oid, color: nextColor() }]
        : lanes.map((lane) => ({ ...lane }));
      const laneIndex = inserted ? before.length - 1 : existingIndex;
      const nodeLane = before[laneIndex];

      const after = before.map((lane) => ({ ...lane }));
      if (commit.parent_ids.length === 0) {
        after.splice(laneIndex, 1);
      } else {
        after[laneIndex] = { oid: commit.parent_ids[0], color: nodeLane.color };
        for (let index = 1; index < commit.parent_ids.length; index += 1) {
          const parentId = commit.parent_ids[index];
          const existingParentLane = before.find((lane) => lane.oid === parentId);
          after.splice(laneIndex + index, 0, {
            oid: parentId,
            color: existingParentLane?.color ?? nextColor(),
          });
        }
      }

      const dedupedAfter = dedupeLaneEntries(after);
      const laneCount = Math.max(before.length, dedupedAfter.length, 1);
      maxLaneCount = Math.max(maxLaneCount, laneCount);
      const lines = [];
      const topY = -graphOverlap;
      const bottomY = graphRowHeight + graphOverlap;
      const nodeX = graphPadding + laneIndex * graphLaneSpacing;
      const nodeColor = nodeLane.color;

      before.forEach((lane, index) => {
        if (index === laneIndex) {
          return;
        }

        const currentX = graphPadding + index * graphLaneSpacing;
        const stroke = lane.color;
        if (commit.parent_ids.includes(lane.oid)) {
          lines.push({
            d: buildLaneTransitionPath(currentX, topY, nodeX, graphCenterY),
            stroke,
          });
          return;
        }

        const nextIndex = dedupedAfter.findIndex((nextLane) => nextLane.oid === lane.oid);

        if (nextIndex === -1) {
          lines.push({
            d: buildVerticalPath(currentX, topY, graphCenterY),
            stroke,
          });
          return;
        }

        const nextX = graphPadding + nextIndex * graphLaneSpacing;
        lines.push({
          d: buildLaneTransitionPath(currentX, topY, nextX, bottomY),
          stroke,
        });
      });

      if (!inserted) {
        lines.push({
          d: buildVerticalPath(nodeX, topY, graphCenterY),
          stroke: nodeColor,
        });
      }

      commit.parent_ids.forEach((parentId) => {
        const nextIndex = dedupedAfter.findIndex((lane) => lane.oid === parentId);
        if (nextIndex === -1) {
          return;
        }
        const nextX = graphPadding + nextIndex * graphLaneSpacing;
        const nextLaneColor = dedupedAfter[nextIndex].color;
        lines.push({
          d:
            nextX === nodeX
              ? buildVerticalPath(nodeX, graphCenterY, bottomY)
              : buildLaneTransitionPath(nodeX, graphCenterY, nextX, bottomY),
          stroke: nextLaneColor,
        });
      });

      lanes = dedupedAfter;

      rows.push({
        ...commit,
        graphLines: lines,
        graphNodeX: nodeX,
        graphNodeColor: nodeColor,
        graphIsHead: rowIndex === 0,
      });
    });

    const graphWidth = maxLaneCount * graphLaneSpacing + graphPadding * 2;
    return rows.map((row) => ({
      ...row,
      graphWidth,
    }));
  }

  $: historyGraphRows = buildGraphRows(historyCommits);
  $: historyGraphWidth = Math.max(
    historyGraphRows.length > 0 ? historyGraphRows[0].graphWidth : 0,
    56,
  );
  const detailPanelMinHeight = 340;
  let windowHeight = 900;
  let historyRowsElement;
  let detailPanelHeight = 0;
  let detailPanelHeightManuallySet = false;
  let detailPanelDragging = false;
  let detailDragStartY = 0;
  let detailDragStartHeight = 0;
  let previousDetailScrollKey = "";
  $: detailPanelVisible = !!selectedCommitOid;
  $: defaultDetailPanelHeight = clampDetailPanelHeight(Math.round(windowHeight * 0.44));
  $: activeDetailPanelHeight =
    detailPanelVisible && detailPanelHeightManuallySet
      ? clampDetailPanelHeight(detailPanelHeight)
      : defaultDetailPanelHeight;
  let selectedDetailPath = "";
  $: detailFiles = selectedCommitDetail?.files ?? [];
  $: if (detailFiles.length === 0) {
    selectedDetailPath = "";
  } else if (!selectedDetailPath || !detailFiles.some((file) => file.path === selectedDetailPath)) {
    selectedDetailPath = detailFiles[0].path;
  }
  $: selectedDetailFile =
    detailFiles.find((file) => file.path === selectedDetailPath) ?? detailFiles[0] ?? null;
  $: selectedDetailPatchLines = selectedDetailFile ? selectedDetailFile.patch.split("\n") : [];
  $: detailScrollKey = `${selectedCommitOid}:${detailPanelVisible ? "open" : "closed"}`;
  $: if (detailScrollKey !== previousDetailScrollKey) {
    previousDetailScrollKey = detailScrollKey;
    if (detailPanelVisible) {
      void tick().then(ensureSelectedCommitVisible);
    }
  }
</script>

<svelte:window bind:innerHeight={windowHeight} on:pointermove={handleWindowPointerMove} on:pointerup={handleWindowPointerUp} />

<section class="center-pane">
  <div class="history-toolbar">
    <input class="search" placeholder={$_("history.searchPlaceholder")} disabled={!repository} />
    <div class="history-meta">
      {#if repository}
        <span>{repository.branch}</span>
        <span>{$_("history.commitCount", { values: { count: historyCommits.length } })}</span>
        <span>{historyLoading ? $_("history.loading") : historyLoadedAll ? $_("history.complete") : ""}</span>
      {/if}
    </div>
  </div>

  {#if repository && repository.head_summary}
    <div class="head-pill">{$_("history.headSummary", { values: { summary: repository.head_summary } })}</div>
  {/if}

  {#if error}
    <div class="banner error-banner">{error}</div>
  {/if}

  <section class="history-table" style={`--graph-column-width: ${historyGraphWidth}px;`}>
    <div class="history-head">
      <span>{$_("history.columns.graph")}</span>
      <span>{$_("history.columns.subject")}</span>
      <span>{$_("history.columns.author")}</span>
      <span>{$_("history.columns.hash")}</span>
      <span>{$_("history.columns.date")}</span>
    </div>

    {#if repository && historyCommits.length > 0}
      <ul bind:this={historyRowsElement} class="history-rows">
        {#each historyGraphRows as commit}
          <li class:history-row-selected={selectedCommitOid === commit.oid} class:muted-history-row={!commit.on_current_branch}>
            <button class="history-row-button" data-commit-oid={commit.oid} type="button" on:click={() => onSelectCommit(commit.oid)}>
              <div class="graph-cell">
                <svg
                  class="graph-svg"
                  viewBox={`0 0 ${commit.graphWidth} ${graphRowHeight}`}
                  width={commit.graphWidth}
                  height={graphRowHeight}
                  preserveAspectRatio="xMinYMid meet"
                  aria-hidden="true"
                >
                  {#each commit.graphLines as line}
                    <path
                      d={line.d}
                      stroke={line.stroke}
                      stroke-width="2"
                      fill="none"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  {/each}
                  <circle
                    cx={commit.graphNodeX}
                    cy={graphCenterY}
                    r={commit.graphIsHead ? 4.5 : 3.5}
                    fill={commit.graphNodeColor}
                  />
                </svg>
              </div>

              <div class="subject-cell">
                {#if commit.labels.length > 0}
                  <div class="history-tags">
                    {#each commit.labels as label}
                      <span
                        class:history-tag-local={label.scope === "local"}
                        class:history-tag-remote={label.scope === "remote"}
                        class:history-tag-current={label.is_current}
                      >
                        {label.name}
                      </span>
                    {/each}
                  </div>
                {/if}
                <strong>{commit.summary}</strong>
              </div>

              <div class="author-cell">
                <span class="avatar">{initials(commit.author)}</span>
                <span>{commit.author}</span>
              </div>

              <div class="hash-cell">
                <span>{commit.id}</span>
              </div>

              <div class="date-cell">
                <span>{formatLocalDateTime(commit.committed_at)}</span>
              </div>
            </button>
          </li>
        {/each}
      </ul>
    {:else if repository && historyLoading}
      <div class="empty-history">
        <p>{$_("history.loadingTitle")}</p>
        <p class="muted">{$_("history.loadingBody")}</p>
      </div>
    {:else}
      <div class="empty-history">
        <p>{$_("history.emptyTitle")}</p>
        <p class="muted">{$_("history.emptyBody")}</p>
      </div>
    {/if}
  </section>

  {#if detailPanelVisible}
    <div
      class:commit-details-splitter-active={detailPanelDragging}
      class="commit-details-splitter"
      role="separator"
      aria-orientation="horizontal"
      on:pointerdown={handleSplitterPointerDown}
    ></div>

    <section class="commit-details" style={`height: ${activeDetailPanelHeight}px;`}>
      {#if selectedCommitDetailLoading}
        <div class="commit-details-empty">
          <p>{$_("history.details.loadingTitle")}</p>
          <p class="muted">{$_("history.details.loadingBody")}</p>
        </div>
      {:else if selectedCommitDetail}
        <div class="commit-details-header">
          <div class="commit-details-header-top">
            <div class="commit-details-party-grid">
              <div class="commit-party-card">
                <div class="commit-party-label">{$_("history.details.author")}</div>
                <div class="commit-party-main">
                  <span class="commit-party-avatar">{initials(selectedCommitDetail.author.name)}</span>
                  <div class="commit-party-copy">
                    <strong>{selectedCommitDetail.author.name}</strong>
                    <span>{selectedCommitDetail.author.email}</span>
                    <span>{selectedCommitDetail.author.committed_at}</span>
                  </div>
                </div>
              </div>

              <div class="commit-party-card">
                <div class="commit-party-label">{$_("history.details.committer")}</div>
                <div class="commit-party-main">
                  <span class="commit-party-avatar">{initials(selectedCommitDetail.committer.name)}</span>
                  <div class="commit-party-copy">
                    <strong>{selectedCommitDetail.committer.name}</strong>
                    <span>{selectedCommitDetail.committer.email}</span>
                    <span>{selectedCommitDetail.committer.committed_at}</span>
                  </div>
                </div>
              </div>
            </div>

            <button class="commit-details-close" type="button" aria-label={$_("history.details.close")} on:click={onCloseCommitDetail}>
              ×
            </button>
          </div>

          <div class="commit-detail-meta">
            <div class="commit-detail-meta-row">
              <span>{$_("history.details.refs")}</span>
              <div class="history-tags">
                {#if selectedCommitDetail.labels.length > 0}
                  {#each selectedCommitDetail.labels as label}
                    <span
                      class:history-tag-local={label.scope === "local"}
                      class:history-tag-remote={label.scope === "remote"}
                      class:history-tag-current={label.is_current}
                    >
                      {label.name}
                    </span>
                  {/each}
                {:else}
                  <span class="commit-detail-meta-empty">{$_("history.details.none")}</span>
                {/if}
              </div>
            </div>

            <div class="commit-detail-meta-row">
              <span>{$_("history.details.sha")}</span>
              <code>{selectedCommitDetail.oid}</code>
            </div>

            <div class="commit-detail-meta-row">
              <span>{$_("history.details.parents")}</span>
              <div class="commit-parent-links">
                {#if selectedCommitDetail.parents.length > 0}
                  {#each selectedCommitDetail.parents as parent}
                    <button class="commit-inline-link" type="button" on:click={() => onSelectCommit(parent.oid)}>
                      {parent.id}
                    </button>
                  {/each}
                {:else}
                  <span class="commit-detail-meta-empty">{$_("history.details.none")}</span>
                {/if}
              </div>
            </div>
          </div>

          <div class="commit-message-block">
            <h3>{selectedCommitDetail.summary}</h3>
            {#if selectedCommitDetail.message && selectedCommitDetail.message !== selectedCommitDetail.summary}
              <pre>{selectedCommitDetail.message}</pre>
            {/if}
          </div>
        </div>

        <div class="commit-details-body">
          <aside class="commit-files-list">
            <div class="commit-files-header">{$_("history.details.files", { values: { count: detailFiles.length } })}</div>
            {#if detailFiles.length > 0}
              <ul>
                {#each detailFiles as file}
                  <li>
                    <button
                      class:commit-file-selected={selectedDetailFile && selectedDetailFile.path === file.path}
                      class="commit-file-button"
                      type="button"
                      on:click={() => (selectedDetailPath = file.path)}
                    >
                      <span class="commit-file-status">{file.status}</span>
                      <span class="commit-file-path">{file.path}</span>
                    </button>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="commit-details-empty muted">{$_("history.details.filesEmpty")}</p>
            {/if}
          </aside>

          <section class="commit-diff-view">
            <div class="commit-files-header">
              {selectedDetailFile ? selectedDetailFile.path : $_("history.details.diff")}
            </div>

            {#if selectedDetailFile}
              <pre class="patch-view">{#each selectedDetailPatchLines as line, index}<span class={formatDetailPatchLineClass(line)}>{line}{index < selectedDetailPatchLines.length - 1 ? "\n" : ""}</span>{/each}</pre>
            {:else}
              <div class="commit-details-empty">
                <p>{$_("history.details.diffEmpty")}</p>
              </div>
            {/if}
          </section>
        </div>
      {:else}
        <div class="commit-details-empty">
          <p>{$_("history.details.loadingTitle")}</p>
          <p class="muted">{$_("history.details.loadingBody")}</p>
        </div>
      {/if}
    </section>
  {/if}
</section>

<style>
  .center-pane {
    min-height: 0;
    min-width: 0;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: linear-gradient(180deg, rgba(11, 22, 34, 0.6), rgba(10, 20, 31, 0.36));
    overflow: hidden;
  }

  .history-toolbar {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    align-items: center;
    padding: 2px 2px 0;
  }

  .search {
    width: 100%;
    max-width: 520px;
    box-sizing: border-box;
    border: 1px solid rgba(120, 148, 177, 0.12);
    border-radius: 6px;
    background: #04080d;
    color: #e8eef5;
    padding: 11px 12px;
    transition: border-color 120ms ease, box-shadow 120ms ease, background 120ms ease;
  }

  .search:focus {
    outline: none;
    border-color: rgba(84, 155, 233, 0.7);
    box-shadow: 0 0 0 3px rgba(35, 101, 168, 0.18);
    background: #06101a;
  }

  .history-meta {
    display: flex;
    gap: 12px;
    color: #688099;
    font-size: 0.85rem;
  }

  .head-pill {
    align-self: start;
    padding: 6px 10px;
    border-radius: 999px;
    background: rgba(23, 44, 66, 0.76);
    color: #a9c1d8;
    font-size: 0.76rem;
    border: 1px solid rgba(96, 132, 169, 0.14);
  }

  .banner {
    border-radius: 10px;
    padding: 12px 14px;
  }

  .error-banner {
    background: rgba(132, 32, 38, 0.18);
    color: #ffb8bc;
    border: 1px solid rgba(198, 84, 90, 0.18);
  }

  .history-table {
    flex: 1 1 auto;
    overflow: hidden;
    display: grid;
    grid-template-rows: auto 1fr;
    min-height: 0;
    border-radius: 8px;
    background: var(--panel-background);
    border: 1px solid var(--panel-border);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
  }

  .history-head,
  .history-row-button {
    display: grid;
    grid-template-columns: var(--graph-column-width, 108px) minmax(0, 1.5fr) minmax(140px, 0.8fr) 92px 132px;
    gap: 14px;
    align-items: center;
  }

  .history-head {
    padding: 4px 12px;
    color: #60788f;
    font-size: 0.64rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    border-bottom: 1px solid rgba(120, 148, 177, 0.06);
    background: rgba(8, 16, 25, 0.55);
  }

  .history-rows {
    overflow: auto;
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .history-rows li {
    list-style: none;
  }

  .history-row-button {
    width: 100%;
    border: 0;
    background: transparent;
    color: inherit;
    text-align: left;
    padding: 0 12px;
    border-bottom: 1px solid rgba(120, 148, 177, 0.04);
    transition: background 120ms ease, box-shadow 120ms ease;
    height: 32px;
    box-sizing: border-box;
    cursor: pointer;
  }

  .history-row-button:hover {
    background: rgba(255, 255, 255, 0.02);
  }

  .history-rows li.muted-history-row .history-row-button {
    background: rgba(255, 255, 255, 0.01);
  }

  .history-rows li.history-row-selected .history-row-button {
    background: rgba(35, 82, 135, 0.22);
    box-shadow: inset 0 0 0 1px rgba(77, 160, 255, 0.22);
  }

  .graph-cell {
    position: relative;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    overflow: visible;
  }

  .graph-svg {
    display: block;
    overflow: visible;
    flex: 0 0 auto;
  }

  .subject-cell {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    height: 100%;
  }

  .subject-cell strong {
    display: block;
    color: #eef5fb;
    line-height: 1.2;
    font-size: 0.78rem;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .muted,
  .muted-history-row .subject-cell strong,
  .muted-history-row .author-cell,
  .muted-history-row .hash-cell,
  .muted-history-row .date-cell {
    color: #627388;
  }

  .history-tags {
    display: flex;
    gap: 3px;
    flex-shrink: 0;
    align-items: center;
  }

  .history-tags span {
    padding: 1px 7px;
    border-radius: 999px;
    background: rgba(43, 71, 98, 0.72);
    color: #c7d9eb;
    font-size: 0.68rem;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .muted-history-row .history-tags span {
    background: rgba(43, 71, 98, 0.32);
    color: #8ca1b8;
  }

  .history-tags span.history-tag-local {
    background: rgba(144, 92, 14, 0.22);
    color: #ffd48a;
    box-shadow: inset 0 0 0 1px rgba(218, 146, 33, 0.35);
  }

  .history-tags span.history-tag-remote {
    background: rgba(60, 74, 94, 0.5);
    color: #d4dde7;
    box-shadow: inset 0 0 0 1px rgba(114, 138, 167, 0.2);
  }

  .history-tags span.history-tag-current {
    background: rgba(164, 102, 11, 0.28);
    color: #fff4d9;
    box-shadow: inset 0 0 0 1px rgba(241, 170, 55, 0.75);
    font-weight: 700;
  }

  .author-cell {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 0.74rem;
    overflow: hidden;
    height: 100%;
  }

  .author-cell span:last-child,
  .hash-cell span,
  .date-cell span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .avatar {
    width: 15px;
    height: 15px;
    flex-shrink: 0;
    border-radius: 999px;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, #2b6aa8, #70b8ff);
    color: #fff;
    font-size: 0.5rem;
    font-weight: 700;
  }

  .muted-history-row .avatar {
    background: linear-gradient(135deg, #44566d, #738399);
    color: #dbe4ed;
  }

  .hash-cell,
  .date-cell {
    display: flex;
    align-items: center;
    color: #7990a7;
    font-size: 0.68rem;
    overflow: hidden;
    height: 100%;
  }

  .empty-history {
    padding: 28px 20px;
  }

  .empty-history p {
    margin: 0;
  }

  .empty-history p + p {
    margin-top: 4px;
  }

  .commit-details {
    flex: 0 0 auto;
    min-height: 340px;
    display: grid;
    grid-template-rows: auto 1fr;
    overflow: hidden;
    border-radius: 8px;
    background: var(--panel-background);
    border: 1px solid var(--panel-border);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.02);
  }

  .commit-details-splitter {
    flex: 0 0 10px;
    margin: -2px 0;
    cursor: row-resize;
    position: relative;
  }

  .commit-details-splitter::before {
    content: "";
    position: absolute;
    left: 50%;
    top: 50%;
    width: 64px;
    height: 4px;
    border-radius: 999px;
    transform: translate(-50%, -50%);
    background: rgba(120, 148, 177, 0.28);
  }

  .commit-details-splitter:hover::before,
  .commit-details-splitter.commit-details-splitter-active::before {
    background: rgba(77, 160, 255, 0.68);
  }

  .commit-details-header {
    display: grid;
    gap: 12px;
    padding: 14px 16px 12px;
    border-bottom: 1px solid rgba(120, 148, 177, 0.08);
  }

  .commit-details-header-top {
    display: flex;
    gap: 14px;
    align-items: start;
    justify-content: space-between;
  }

  .commit-details-body {
    min-height: 0;
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr);
  }

  .commit-details-empty {
    padding: 18px 16px;
  }

  .commit-details-empty p {
    margin: 0;
  }

  .commit-details-empty p + p {
    margin-top: 4px;
  }

  .commit-details-party-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .commit-party-card {
    min-width: 0;
  }

  .commit-party-label {
    margin-bottom: 6px;
    color: #8aa0b8;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .commit-party-main {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .commit-party-avatar {
    width: 34px;
    height: 34px;
    flex-shrink: 0;
    border-radius: 10px;
    display: grid;
    place-items: center;
    background: linear-gradient(135deg, #7a56c2, #d3b8f8);
    color: white;
    font-size: 0.8rem;
    font-weight: 700;
  }

  .commit-party-copy {
    min-width: 0;
    display: grid;
    gap: 2px;
  }

  .commit-party-copy strong,
  .commit-party-copy span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .commit-party-copy strong {
    font-size: 0.94rem;
    color: #eef5fb;
  }

  .commit-party-copy span {
    color: #8aa0b8;
    font-size: 0.76rem;
  }

  .commit-detail-meta {
    display: grid;
    gap: 6px;
  }

  .commit-details-close {
    flex: 0 0 auto;
    width: 30px;
    height: 30px;
    border: 1px solid rgba(120, 148, 177, 0.12);
    border-radius: 999px;
    background: rgba(11, 22, 34, 0.82);
    color: #dce7f2;
    font-size: 1rem;
    line-height: 1;
  }

  .commit-details-close:hover {
    background: rgba(22, 38, 54, 0.96);
  }

  .commit-detail-meta-row {
    display: grid;
    grid-template-columns: 72px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
  }

  .commit-detail-meta-row > span:first-child {
    color: #8aa0b8;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .commit-detail-meta-row code {
    color: #d9e7f4;
    font-size: 0.8rem;
    word-break: break-all;
  }

  .commit-parent-links {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }

  .commit-inline-link {
    border: 0;
    padding: 0;
    background: transparent;
    color: #6bb0ff;
    font-size: 0.82rem;
  }

  .commit-inline-link:hover {
    color: #9dc9ff;
    text-decoration: underline;
  }

  .commit-detail-meta-empty {
    color: #627388;
    font-size: 0.76rem;
  }

  .commit-message-block h3 {
    margin: 0;
    color: #eef5fb;
    font-size: 0.94rem;
    font-weight: 600;
  }

  .commit-message-block pre {
    margin: 8px 0 0;
    color: #dce7f2;
    font-size: 0.82rem;
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
  }

  .commit-files-list {
    min-height: 0;
    overflow: auto;
    border-right: 1px solid rgba(120, 148, 177, 0.08);
    background: rgba(7, 14, 22, 0.32);
  }

  .commit-files-header {
    padding: 10px 12px;
    border-bottom: 1px solid rgba(120, 148, 177, 0.08);
    color: #8aa0b8;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .commit-files-list ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .commit-file-button {
    width: 100%;
    border: 0;
    background: transparent;
    color: inherit;
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    align-items: center;
    text-align: left;
    padding: 8px 12px;
  }

  .commit-file-button:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .commit-file-button.commit-file-selected {
    background: rgba(35, 82, 135, 0.22);
    box-shadow: inset 0 0 0 1px rgba(77, 160, 255, 0.18);
  }

  .commit-file-status {
    color: #ffbf69;
    font-size: 0.72rem;
    font-weight: 700;
    text-align: center;
  }

  .commit-file-path {
    color: #e5eef7;
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .commit-diff-view {
    min-height: 0;
    display: grid;
    grid-template-rows: auto 1fr;
    overflow: hidden;
  }

  .patch-view {
    margin: 0;
    min-height: 0;
    overflow: auto;
    padding: 12px 14px;
    background: #0a1118;
    color: #dce7f2;
    font-size: 0.76rem;
    line-height: 1.45;
    font-family: "SFMono-Regular", "Menlo", monospace;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .patch-line {
    display: block;
  }

  .patch-line-hunk {
    color: #8fb9e6;
  }

  .patch-line-added {
    background: rgba(88, 171, 105, 0.24);
    color: #b7f0c3;
  }

  .patch-line-removed {
    background: rgba(180, 78, 78, 0.2);
    color: #ffb5b5;
  }

  @media (max-width: 860px) {
    .center-pane {
      padding: 14px;
    }

    .history-toolbar {
      align-items: stretch;
      flex-direction: column;
    }

    .history-head,
    .history-row-button {
      grid-template-columns: 84px minmax(0, 1fr);
    }

    .history-row-button {
      height: 30px;
      padding: 0 10px;
    }

    .history-head span:nth-child(3),
    .history-head span:nth-child(4),
    .history-head span:nth-child(5),
    .author-cell,
    .hash-cell,
    .date-cell {
      display: none;
    }

    .commit-details {
      min-height: 360px;
    }

    .commit-details-party-grid,
    .commit-details-body {
      grid-template-columns: 1fr;
    }

    .commit-files-list {
      border-right: 0;
      border-bottom: 1px solid rgba(120, 148, 177, 0.08);
      max-height: 140px;
    }
  }
</style>
