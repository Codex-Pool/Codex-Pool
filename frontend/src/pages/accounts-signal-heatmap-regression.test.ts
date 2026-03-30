/// <reference types="node" />

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));

test("Accounts wires backend-backed recent signal heatmaps into the list and detail views", async () => {
  const source = await readFile(path.join(ROOT, "pages/Accounts.tsx"), "utf8");

  assert.match(
    source,
    /accountPoolApi\.getSignalHeatmap/,
    "Accounts detail view should request the dedicated account signal heatmap endpoint",
  );
  assert.match(
    source,
    /accountPool\.recentSignal\.window12h/,
    "Accounts list should label the compact recent signal heatmap as the last 12 hours",
  );
  assert.match(
    source,
    /<SignalHeatmapMini[\s\S]*visibleCount=\{12\}/,
    "Accounts list should render the compact recent signal heatmap through the shared mini component",
  );
  assert.match(
    source,
    /<SignalHeatmapCanvas[\s\S]*bucketMinutes=\{selectedSignalHeatmap\.bucket_minutes\}/,
    "Accounts detail should render the expanded recent signal heatmap through the shared canvas component",
  );
  assert.match(
    source,
    /accountPool\.detail\.sections\.recentSignal/,
    "Accounts detail modal should expose a dedicated recent signal heatmap section",
  );
});

test("accounts API exposes recent signal heatmap types and endpoint", async () => {
  const source = await readFile(path.join(ROOT, "api/accounts.ts"), "utf8");

  assert.match(
    source,
    /export interface AccountSignalHeatmapSummary/,
    "accounts API should expose the compact recent signal heatmap summary type",
  );
  assert.match(
    source,
    /recent_signal_heatmap\?: AccountSignalHeatmapSummary/,
    "AccountPoolRecord should include the recent signal heatmap summary field",
  );
  assert.match(
    source,
    /getSignalHeatmap: async \(recordId: string\)/,
    "accountPoolApi should expose a signal heatmap detail fetcher",
  );
  assert.match(
    source,
    /\/account-pool\/accounts\/\$\{recordId\}\/signal-heatmap/,
    "accountPoolApi should target the dedicated account-pool signal heatmap endpoint",
  );
});
