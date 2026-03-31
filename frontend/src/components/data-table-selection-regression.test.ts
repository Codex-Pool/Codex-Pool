/// <reference types="node" />

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));

test("DataTable keeps batch selection aligned with filtered rows and exposes explicit cross-page selection", async () => {
  const source = await readFile(path.join(ROOT, "components/DataTable.tsx"), "utf8");

  assert.match(
    source,
    /const filteredRowIds = useMemo/,
    "DataTable should track the current filtered row ids so stale selections can be cleaned up",
  );
  assert.match(
    source,
    /new Set\(\[\.\.\.resolvedSelectedKeys\]\.filter\(\(id\) => validIds\.has\(id\)\)\)/,
    "DataTable should drop selected ids that are no longer part of the filtered dataset",
  );
  assert.match(
    source,
    /common\.table\.selectFiltered/,
    "DataTable should expose a separate filtered-results selection control for cross-page bulk actions",
  );
  assert.match(
    source,
    /w-full rounded-large border border-default-200 bg-default-100\/80/,
    "DataTable batch toolbar should stretch to the full table width instead of hugging its contents",
  );
});
