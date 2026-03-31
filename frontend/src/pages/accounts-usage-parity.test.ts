/// <reference types="node" />

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));

test("extractRateLimitDisplaysFromSnapshots keeps used_percent to remainingPercent conversion in shared utils", async () => {
  const source = await readFile(path.join(ROOT, "features/accounts/utils.ts"), "utf8");

  assert.match(
    source,
    /export function extractRateLimitDisplaysFromSnapshots/,
    "accounts utils should expose a shared snapshot-to-usage extraction helper",
  );
  assert.match(
    source,
    /remainingPercent:\s*toRemainingPercent\(fiveHours\.used_percent\)/,
    "five-hour usage should convert used_percent into remainingPercent",
  );
  assert.match(
    source,
    /remainingPercent:\s*toRemainingPercent\(oneWeek\.used_percent\)/,
    "weekly usage should convert used_percent into remainingPercent",
  );
});

test("Accounts uses the shared usage helper and HeroUI progress bars for account pool usage", async () => {
  const source = await readFile(path.join(ROOT, "pages/Accounts.tsx"), "utf8");

  assert.match(
    source,
    /extractRateLimitDisplaysFromSnapshots/,
    "Accounts should reuse the shared rate-limit snapshot extraction helper",
  );
  assert.match(
    source,
    /<Progress/,
    "Accounts should render usage visually with HeroUI Progress",
  );
  assert.match(
    source,
    /compactLabel:/,
    "Accounts should expose compact bucket labels for the condensed usage cards",
  );
  assert.doesNotMatch(
    source,
    /function formatRateLimits/,
    "Accounts should no longer stringify raw used_percent snapshots with a local formatter",
  );
  assert.match(
    source,
    /CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5"/,
    "Accounts records card header should pin its title block to the left edge instead of inheriting HeroUI's centered cross-axis alignment",
  );
});

test("Accounts localizes reason codes instead of falling back to raw backend values", async () => {
  const source = await readFile(path.join(ROOT, "pages/Accounts.tsx"), "utf8");

  assert.match(
    source,
    /getReasonCodeLabel\(record\.reason_code, t\)/,
    "Accounts table rows should localize account-pool reason codes",
  );
  assert.match(
    source,
    /getReasonCodeLabel\(selectedRecord\.reason_code, t\)/,
    "Accounts detail modal should localize account-pool reason codes",
  );
  assert.doesNotMatch(
    source,
    /reason_code \?\?/,
    "Accounts should not directly render raw reason_code values anymore",
  );
});

test("accounts plan labels flow through i18n-backed plan value keys", async () => {
  const source = await readFile(path.join(ROOT, "features/accounts/utils.ts"), "utf8");

  assert.match(
    source,
    /return t\(`accounts\.planValues\.\$\{value\}`,\s*\{ defaultValue: value \}\)/,
    "getPlanLabel should delegate plan values through i18n keys before falling back to the raw value",
  );
});

test("locale files expose account-pool usage wording and reason-code mappings", async () => {
  const zh = await readFile(path.join(ROOT, "locales/zh-CN.ts"), "utf8");
  const en = await readFile(path.join(ROOT, "locales/en.ts"), "utf8");

  assert.match(zh, /quota:\s*"用量"/, "zh-CN should label the account-pool usage column as 用量");
  assert.match(en, /quota:\s*"Usage"/, "en should label the account-pool usage column as Usage");
  assert.match(zh, /operationalStatus:\s*"运营状态"/, "zh-CN should expose the consolidated operational status column label");
  assert.match(en, /operationalStatus:\s*"Operational status"/, "en should expose the consolidated operational status column label");
  assert.match(zh, /recentSignal:\s*"最近信号"/, "zh-CN should expose the recent signal column label");
  assert.match(en, /recentSignal:\s*"Recent signal"/, "en should expose the recent signal column label");
  assert.match(zh, /window12h:\s*"近 12h"/, "zh-CN should expose the compact 12-hour heatmap label");
  assert.match(en, /window24h:\s*"Last 24h"/, "en should expose the detail 24-hour heatmap label");
  assert.match(zh, /noHeatmap:\s*"暂无可展示的信号热图"/, "zh-CN should expose the no-heatmap fallback copy");
  assert.match(en, /bucketTooltip:\s*"\{\{time\}\} · \{\{count\}\} signals · active \{\{active\}\} \/ passive \{\{passive\}\}"/, "en should expose the heatmap tooltip copy");
  assert.match(zh, /updatedAt:\s*"更新时间"/, "zh-CN should expose the account-pool updatedAt column label");
  assert.match(en, /updatedAt:\s*"Updated at"/, "en should expose the account-pool updatedAt column label");
  assert.match(zh, /more:\s*"更多操作"/, "zh-CN should expose the account-pool more actions label");
  assert.match(en, /more:\s*"More actions"/, "en should expose the account-pool more actions label");
  assert.match(zh, /fiveHoursShort:\s*"5h"/, "zh-CN should expose the compact five-hour usage bucket label");
  assert.match(en, /oneWeekShort:\s*"7d"/, "en should expose the compact weekly usage bucket label");
  assert.match(zh, /tokenInvalidated:\s*"令牌已失效"/, "zh-CN should localize tokenInvalidated");
  assert.match(en, /tokenInvalidated:\s*"Token invalidated"/, "en should localize tokenInvalidated");
  assert.match(zh, /unknown:\s*"未知阻断原因"/, "zh-CN should include an unknown account-pool reason-code fallback");
  assert.match(en, /unknown:\s*"Unknown blocking reason"/, "en should include an unknown account-pool reason-code fallback");
});

test("Accounts detail modal exposes the runtime-only real responses testing panel through HeroUI primitives", async () => {
  const source = await readFile(path.join(ROOT, "pages/Accounts.tsx"), "utf8");
  const zh = await readFile(path.join(ROOT, "locales/zh-CN.ts"), "utf8");
  const en = await readFile(path.join(ROOT, "locales/en.ts"), "utf8");

  assert.match(
    source,
    /accountPoolApi\.testResponses/,
    "Accounts should call the dedicated account-pool responses test endpoint",
  );
  assert.match(
    source,
    /accountsApi\.getOAuthStatus/,
    "Accounts should prefer runtime account supported_models for the test model options",
  );
  assert.match(
    source,
    /modelsApi\.listModels/,
    "Accounts should fall back to the global models list when account-specific models are unavailable",
  );
  assert.match(
    source,
    /record_scope === 'runtime'/,
    "Accounts should only expose the interactive testing panel for runtime records",
  );
  assert.match(
    source,
    /previousResponseId/,
    "Accounts should keep track of previous_response_id for lightweight in-modal follow-up turns",
  );
  assert.match(
    source,
    /extractApiErrorMessageFrom/,
    "Accounts should surface the raw responses test error message for operator debugging",
  );
  assert.match(
    source,
    /<Textarea/,
    "Accounts should render the test prompt input with HeroUI Textarea",
  );
  assert.match(
    source,
    /<Select/,
    "Accounts should render the test model picker with HeroUI Select",
  );
  assert.match(
    source,
    /accountPool\.detail\.sections\.realTest/,
    "Accounts should localize the new real test section title through accountPool.detail.sections.realTest",
  );
  assert.match(
    source,
    /accountPool\.detail\.test\.rawError/,
    "Accounts should localize the raw error label for the real test debug panel",
  );
  assert.match(
    zh,
    /realTest:\s*"真实测试"/,
    "zh-CN should define the real responses test section title",
  );
  assert.match(
    en,
    /realTest:\s*"Real test"/,
    "en should define the real responses test section title",
  );
  assert.match(
    zh,
    /runtimeOnly:\s*"首版真实测试仅支持 runtime 记录"/,
    "zh-CN should explain the runtime-only limitation for the first version",
  );
  assert.match(
    zh,
    /rawError:\s*"原始报错"/,
    "zh-CN should define the raw responses test error label",
  );
  assert.match(
    en,
    /runtimeOnly:\s*"The first version of real testing only supports runtime records"/,
    "en should explain the runtime-only limitation for the first version",
  );
  assert.match(
    en,
    /rawError:\s*"Raw error"/,
    "en should define the raw responses test error label",
  );
});
