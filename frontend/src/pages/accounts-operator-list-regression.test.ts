/// <reference types="node" />

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));

test("Accounts consolidates operator-facing table columns and actions", async () => {
  const source = await readFile(path.join(ROOT, "pages/Accounts.tsx"), "utf8");

  assert.match(
    source,
    /<TableColumn[^>]*>\s*\{t\('accountPool\.columns\.operationalStatus'\)\}\s*<\/TableColumn>/,
    "Accounts should expose the consolidated operational status column",
  );
  assert.match(
    source,
    /<TableColumn[^>]*>\s*\{t\('accountPool\.columns\.recentSignal'\)\}\s*<\/TableColumn>/,
    "Accounts should expose the recent signal column",
  );
  assert.doesNotMatch(
    source,
    /<TableColumn[^>]*>\s*\{t\('accountPool\.columns\.reason'\)\}\s*<\/TableColumn>|<TableColumn[^>]*>\s*\{t\('accountPool\.columns\.credentials'\)\}\s*<\/TableColumn>/,
    "Accounts should no longer keep reason and credentials as standalone list columns",
  );
  assert.match(
    source,
    /<DropdownMenu[\s\S]*accountPool\.actions\.reprobe[\s\S]*accountPool\.actions\.restore[\s\S]*accountPool\.actions\.delete/,
    "Accounts should collapse secondary row actions into a HeroUI dropdown menu",
  );
  assert.match(
    source,
    /accountPool\.actions\.more/,
    "Accounts should label the row action dropdown through i18n",
  );
  assert.match(
    source,
    /isIconOnly[\s\S]*accountPool\.actions\.inspect[\s\S]*<Eye className="h-4 w-4" \/>/,
    "Accounts row inspect action should collapse to an icon-only button",
  );
  assert.match(
    source,
    /isIconOnly[\s\S]*accountPool\.actions\.more[\s\S]*<MoreHorizontal className="h-4 w-4" \/>/,
    "Accounts row overflow action should collapse to an icon-only button",
  );
});

test("Accounts summary cards can drive state and reason filters", async () => {
  const source = await readFile(path.join(ROOT, "pages/Accounts.tsx"), "utf8");

  assert.match(
    source,
    /setStateFilter\(\(current\) => \(current === card\.key \? 'all' : card\.key\)\)/,
    "Accounts state overview cards should toggle the state filter directly",
  );
  assert.match(
    source,
    /setReasonClassFilter\(\(current\) => \(current === card\.key \? 'all' : card\.key\)\)/,
    "Accounts reason overview cards should toggle the reason-class filter directly",
  );
  assert.match(
    source,
    /isPressable/,
    "Accounts overview cards should become pressable filter surfaces",
  );
});

test("Accounts list avoids showing raw account ids in the secondary identity line", async () => {
  const source = await readFile(path.join(ROOT, "pages/Accounts.tsx"), "utf8");

  assert.doesNotMatch(
    source,
    /const accountId = record\.chatgpt_account_id\?\.trim\(\)|record\.chatgpt_account_id \?\? '-'/,
    "Accounts list should not expose raw ChatGPT account ids in the secondary identity line",
  );
});

test("Accounts batch selection separates current-page selection from filtered-result selection", async () => {
  const source = await readFile(path.join(ROOT, "pages/Accounts.tsx"), "utf8");
  const zh = await readFile(path.join(ROOT, "locales/zh-CN.ts"), "utf8");
  const en = await readFile(path.join(ROOT, "locales/en.ts"), "utf8");

  assert.match(
    source,
    /const currentPageRecordIds = useMemo\(\(\) => paginatedRecords\.map\(\(record\) => record\.id\),\s*\[paginatedRecords\]\)/,
    "Accounts should derive a dedicated current-page record id set for page-scoped selection",
  );
  assert.match(
    source,
    /togglePageSelection/,
    "Accounts should route the header checkbox through explicit page-only selection logic",
  );
  assert.match(
    source,
    /toggleFilteredSelection/,
    "Accounts should expose an explicit filtered-results selection control for cross-page bulk actions",
  );
  assert.match(
    source,
    /w-full rounded-large border border-default-200 bg-default-100\/80/,
    "Accounts batch action bar should stretch to the same width as the records workbench",
  );
  assert.match(
    zh,
    /selectFiltered:\s*"全选当前筛选结果（\{\{count\}\} 条）"/,
    "zh-CN should localize the cross-page filtered selection label",
  );
  assert.match(
    en,
    /selectFiltered:\s*"Select all filtered results \(\{\{count\}\}\)"/,
    "en should localize the cross-page filtered selection label",
  );
  assert.doesNotMatch(
    zh,
    /selectedCount:\s*"已选 \{\{count\}\} \/ \{\{total\}\} 条"/,
    "zh-CN should not keep the stale selected-count copy that still expects an unused total placeholder",
  );
  assert.doesNotMatch(
    en,
    /selectedCount:\s*"\{\{count\}\} of \{\{total\}\} selected"/,
    "en should not keep the stale selected-count copy that still expects an unused total placeholder",
  );
});
