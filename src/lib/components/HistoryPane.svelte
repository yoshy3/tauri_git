<script>
  import { _ } from "svelte-i18n";

  export let repository = null;
  export let error = "";
  export let historyCommits = [];
  export let historyLoading = false;
  export let historyLoadedAll = false;

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
</script>

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
      <ul class="history-rows">
        {#each historyGraphRows as commit}
          <li class:muted-history-row={!commit.on_current_branch}>
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
</section>

<style>
  .center-pane {
    min-height: 0;
    min-width: 0;
    padding: 12px;
    display: grid;
    grid-template-rows: auto auto 1fr;
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
  .history-rows li {
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
    padding: 0 12px;
    border-bottom: 1px solid rgba(120, 148, 177, 0.04);
    transition: background 120ms ease;
    height: 32px;
    box-sizing: border-box;
  }

  .history-rows li:hover {
    background: rgba(255, 255, 255, 0.02);
  }

  .history-rows li.muted-history-row {
    background: rgba(255, 255, 255, 0.01);
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

  @media (max-width: 860px) {
    .center-pane {
      padding: 14px;
    }

    .history-toolbar {
      align-items: stretch;
      flex-direction: column;
    }

    .history-head,
    .history-rows li {
      grid-template-columns: 84px minmax(0, 1fr);
    }

    .history-rows li {
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
  }
</style>
