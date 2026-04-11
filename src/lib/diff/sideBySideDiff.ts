export type DiffChunk = {
  text: string;
  changed: boolean;
};

export type SideBySideDiffRow = {
  kind: "context" | "added" | "removed" | "modified" | "gap";
  leftNumber: string;
  rightNumber: string;
  leftText: string;
  rightText: string;
  leftChunks: DiffChunk[];
  rightChunks: DiffChunk[];
};

function parseHunkHeader(line: string) {
  const match = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/.exec(line);
  if (!match) {
    return null;
  }

  return {
    oldLine: Number(match[1]),
    oldCount: Number(match[2] ?? "1"),
    newLine: Number(match[3]),
    newCount: Number(match[4] ?? "1"),
  };
}

function isMetadataLine(line: string) {
  return (
    line.startsWith("diff --git") ||
    line.startsWith("index ") ||
    line.startsWith("--- ") ||
    line.startsWith("+++ ")
  );
}

function buildPlainChunks(text: string): DiffChunk[] {
  return [{ text, changed: false }];
}

function splitChangedChunks(leftText: string, rightText: string) {
  let prefixLength = 0;
  const maxPrefix = Math.min(leftText.length, rightText.length);
  while (
    prefixLength < maxPrefix &&
    leftText[prefixLength] === rightText[prefixLength]
  ) {
    prefixLength += 1;
  }

  let suffixLength = 0;
  const maxSuffix = Math.min(leftText.length - prefixLength, rightText.length - prefixLength);
  while (
    suffixLength < maxSuffix &&
    leftText[leftText.length - 1 - suffixLength] ===
      rightText[rightText.length - 1 - suffixLength]
  ) {
    suffixLength += 1;
  }

  const leftPrefix = leftText.slice(0, prefixLength);
  const rightPrefix = rightText.slice(0, prefixLength);
  const leftChanged = leftText.slice(prefixLength, leftText.length - suffixLength);
  const rightChanged = rightText.slice(prefixLength, rightText.length - suffixLength);
  const leftSuffix = suffixLength > 0 ? leftText.slice(leftText.length - suffixLength) : "";
  const rightSuffix = suffixLength > 0 ? rightText.slice(rightText.length - suffixLength) : "";

  const leftChunks: DiffChunk[] = [];
  const rightChunks: DiffChunk[] = [];

  if (leftPrefix) {
    leftChunks.push({ text: leftPrefix, changed: false });
  }
  if (rightPrefix) {
    rightChunks.push({ text: rightPrefix, changed: false });
  }

  leftChunks.push({ text: leftChanged || " ", changed: true });
  rightChunks.push({ text: rightChanged || " ", changed: true });

  if (leftSuffix) {
    leftChunks.push({ text: leftSuffix, changed: false });
  }
  if (rightSuffix) {
    rightChunks.push({ text: rightSuffix, changed: false });
  }

  return { leftChunks, rightChunks };
}

function lcsLength(leftText: string, rightText: string) {
  const left = Array.from(leftText);
  const right = Array.from(rightText);
  const dp = Array.from({ length: left.length + 1 }, () =>
    new Array(right.length + 1).fill(0),
  );

  for (let leftIndex = 1; leftIndex <= left.length; leftIndex += 1) {
    for (let rightIndex = 1; rightIndex <= right.length; rightIndex += 1) {
      dp[leftIndex][rightIndex] =
        left[leftIndex - 1] === right[rightIndex - 1]
          ? dp[leftIndex - 1][rightIndex - 1] + 1
          : Math.max(dp[leftIndex - 1][rightIndex], dp[leftIndex][rightIndex - 1]);
    }
  }

  return dp[left.length][right.length];
}

function normalizeWhitespace(text: string) {
  return text.trim().replace(/\s+/g, " ");
}

function tokenSet(text: string) {
  return new Set(
    (text.match(/[A-Za-z_][A-Za-z0-9_]*|\d+|[^\s]/g) ?? []).filter(Boolean),
  );
}

function tokenSimilarity(leftText: string, rightText: string) {
  const leftTokens = tokenSet(leftText);
  const rightTokens = tokenSet(rightText);

  if (leftTokens.size === 0 && rightTokens.size === 0) {
    return 1;
  }

  let intersection = 0;
  for (const token of leftTokens) {
    if (rightTokens.has(token)) {
      intersection += 1;
    }
  }

  return (2 * intersection) / (leftTokens.size + rightTokens.size);
}

function bigramSimilarity(leftText: string, rightText: string) {
  const left = Array.from(normalizeWhitespace(leftText));
  const right = Array.from(normalizeWhitespace(rightText));

  if (left.length === 0 && right.length === 0) {
    return 1;
  }

  const leftBigrams = new Map<string, number>();
  const rightBigrams = new Map<string, number>();

  const fill = (source: string[], target: Map<string, number>) => {
    if (source.length < 2) {
      if (source.length === 1) {
        target.set(source[0], 1);
      }
      return;
    }

    for (let index = 0; index < source.length - 1; index += 1) {
      const gram = `${source[index]}${source[index + 1]}`;
      target.set(gram, (target.get(gram) ?? 0) + 1);
    }
  };

  fill(left, leftBigrams);
  fill(right, rightBigrams);

  let overlap = 0;
  for (const [gram, count] of leftBigrams.entries()) {
    overlap += Math.min(count, rightBigrams.get(gram) ?? 0);
  }

  const leftCount = Array.from(leftBigrams.values()).reduce((sum, count) => sum + count, 0);
  const rightCount = Array.from(rightBigrams.values()).reduce((sum, count) => sum + count, 0);

  if (leftCount === 0 && rightCount === 0) {
    return 1;
  }

  return (2 * overlap) / (leftCount + rightCount);
}

function prefixSuffixSimilarity(leftText: string, rightText: string) {
  const maxLength = Math.max(leftText.length, rightText.length, 1);
  let prefixLength = 0;
  while (
    prefixLength < leftText.length &&
    prefixLength < rightText.length &&
    leftText[prefixLength] === rightText[prefixLength]
  ) {
    prefixLength += 1;
  }

  let suffixLength = 0;
  while (
    suffixLength < leftText.length - prefixLength &&
    suffixLength < rightText.length - prefixLength &&
    leftText[leftText.length - 1 - suffixLength] === rightText[rightText.length - 1 - suffixLength]
  ) {
    suffixLength += 1;
  }

  return (prefixLength + suffixLength) / maxLength;
}

function indentationSimilarity(leftText: string, rightText: string) {
  const leftIndent = leftText.match(/^\s*/)?.[0].length ?? 0;
  const rightIndent = rightText.match(/^\s*/)?.[0].length ?? 0;
  const maxIndent = Math.max(leftIndent, rightIndent, 1);
  return 1 - Math.abs(leftIndent - rightIndent) / maxIndent;
}

function lineSimilarity(leftText: string, rightText: string) {
  if (!leftText && !rightText) {
    return 1;
  }

  const normalizedLeft = normalizeWhitespace(leftText);
  const normalizedRight = normalizeWhitespace(rightText);
  const common = lcsLength(normalizedLeft, normalizedRight);
  const lcsScore =
    normalizedLeft.length + normalizedRight.length === 0
      ? 1
      : (2 * common) / (normalizedLeft.length + normalizedRight.length);
  const bigramScore = bigramSimilarity(leftText, rightText);
  const tokenScore = tokenSimilarity(leftText, rightText);
  const prefixSuffixScore = prefixSuffixSimilarity(leftText, rightText);
  const indentScore = indentationSimilarity(leftText, rightText);

  return (
    lcsScore * 0.35 +
    bigramScore * 0.25 +
    tokenScore * 0.22 +
    prefixSuffixScore * 0.12 +
    indentScore * 0.06
  );
}

function alignChangedLines(removedLines: string[], addedLines: string[]) {
  const gapPenalty = -0.38;
  const pairThreshold = 0.28;
  const rowCount = removedLines.length;
  const columnCount = addedLines.length;
  const score = Array.from({ length: rowCount + 1 }, () =>
    new Array(columnCount + 1).fill(0),
  );
  const move = Array.from({ length: rowCount + 1 }, () =>
    new Array<"pair" | "remove" | "add" | null>(columnCount + 1).fill(null),
  );

  for (let row = 1; row <= rowCount; row += 1) {
    score[row][0] = row * gapPenalty;
    move[row][0] = "remove";
  }

  for (let column = 1; column <= columnCount; column += 1) {
    score[0][column] = column * gapPenalty;
    move[0][column] = "add";
  }

  for (let row = 1; row <= rowCount; row += 1) {
    for (let column = 1; column <= columnCount; column += 1) {
      const similarity = lineSimilarity(removedLines[row - 1], addedLines[column - 1]);
      const pairScore = score[row - 1][column - 1] + similarity;
      const removeScore = score[row - 1][column] + gapPenalty;
      const addScore = score[row][column - 1] + gapPenalty;

      if (pairScore >= removeScore && pairScore >= addScore) {
        score[row][column] = pairScore;
        move[row][column] = "pair";
      } else if (removeScore >= addScore) {
        score[row][column] = removeScore;
        move[row][column] = "remove";
      } else {
        score[row][column] = addScore;
        move[row][column] = "add";
      }
    }
  }

  const aligned: Array<{ type: "pair" | "remove" | "add"; left?: string; right?: string }> = [];
  let row = rowCount;
  let column = columnCount;

  while (row > 0 || column > 0) {
    const action = move[row][column];
    if (action === "pair" && row > 0 && column > 0) {
      const left = removedLines[row - 1];
      const right = addedLines[column - 1];
      if (lineSimilarity(left, right) >= pairThreshold) {
        aligned.push({ type: "pair", left, right });
      } else {
        aligned.push({ type: "add", right });
        aligned.push({ type: "remove", left });
      }
      row -= 1;
      column -= 1;
      continue;
    }

    if (action === "remove" && row > 0) {
      aligned.push({ type: "remove", left: removedLines[row - 1] });
      row -= 1;
      continue;
    }

    if (column > 0) {
      aligned.push({ type: "add", right: addedLines[column - 1] });
      column -= 1;
      continue;
    }

    if (row > 0) {
      aligned.push({ type: "remove", left: removedLines[row - 1] });
      row -= 1;
    }
  }

  return aligned.reverse();
}

export function parseUnifiedDiff(patchText: string): SideBySideDiffRow[] {
  if (!patchText.trim()) {
    return [];
  }

  const lines = patchText.replace(/\r\n/g, "\n").split("\n");
  const rows: SideBySideDiffRow[] = [];
  let index = 0;
  let oldLine = 0;
  let newLine = 0;

  while (index < lines.length) {
    const line = lines[index];

    if (!line || isMetadataLine(line)) {
      index += 1;
      continue;
    }

    if (line.startsWith("@@")) {
      const parsed = parseHunkHeader(line);
      if (parsed) {
        if (rows.length > 0) {
          const skippedOld = Math.max(parsed.oldLine - oldLine, 0);
          const skippedNew = Math.max(parsed.newLine - newLine, 0);
          const skippedLines = Math.max(skippedOld, skippedNew);

          if (skippedLines > 0) {
            const label = `... ${skippedLines} unchanged line${skippedLines === 1 ? "" : "s"} omitted ...`;
            rows.push({
              kind: "gap",
              leftNumber: "",
              rightNumber: "",
              leftText: label,
              rightText: label,
              leftChunks: buildPlainChunks(label),
              rightChunks: buildPlainChunks(label),
            });
          }
        }

        oldLine = parsed.oldLine;
        newLine = parsed.newLine;
      }
      index += 1;
      continue;
    }

    if (line.startsWith(" ")) {
      const text = line.slice(1);
      rows.push({
        kind: "context",
        leftNumber: String(oldLine),
        rightNumber: String(newLine),
        leftText: text,
        rightText: text,
        leftChunks: buildPlainChunks(text),
        rightChunks: buildPlainChunks(text),
      });
      oldLine += 1;
      newLine += 1;
      index += 1;
      continue;
    }

    if (line.startsWith("-") || line.startsWith("+")) {
      const removedLines: string[] = [];
      const addedLines: string[] = [];

      while (index < lines.length) {
        const currentLine = lines[index];

        if (currentLine === "\\ No newline at end of file") {
          index += 1;
          continue;
        }

        if (currentLine.startsWith("-")) {
          removedLines.push(currentLine.slice(1));
          index += 1;
          continue;
        }

        if (currentLine.startsWith("+")) {
          addedLines.push(currentLine.slice(1));
          index += 1;
          continue;
        }

        break;
      }

      for (const entry of alignChangedLines(removedLines, addedLines)) {
        if (entry.type === "pair") {
          const leftText = entry.left ?? "";
          const rightText = entry.right ?? "";
          const { leftChunks, rightChunks } = splitChangedChunks(leftText, rightText);
          rows.push({
            kind: "modified",
            leftNumber: String(oldLine),
            rightNumber: String(newLine),
            leftText,
            rightText,
            leftChunks,
            rightChunks,
          });
          oldLine += 1;
          newLine += 1;
          continue;
        }

        if (entry.type === "remove") {
          const leftText = entry.left ?? "";
          rows.push({
            kind: "removed",
            leftNumber: String(oldLine),
            rightNumber: "",
            leftText,
            rightText: "",
            leftChunks: [{ text: leftText || " ", changed: true }],
            rightChunks: [{ text: " ", changed: false }],
          });
          oldLine += 1;
          continue;
        }

        const rightText = entry.right ?? "";
        rows.push({
          kind: "added",
          leftNumber: "",
          rightNumber: String(newLine),
          leftText: "",
          rightText,
          leftChunks: [{ text: " ", changed: false }],
          rightChunks: [{ text: rightText || " ", changed: true }],
        });
        newLine += 1;
      }

      continue;
    }

    index += 1;
  }

  return rows;
}
