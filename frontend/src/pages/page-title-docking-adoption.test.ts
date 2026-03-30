/// <reference types="node" />

import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));

const DOCKED_PAGE_FILES = [
  "pages/Dashboard.tsx",
  "pages/Usage.tsx",
  "pages/Billing.tsx",
  "pages/Accounts.tsx",
  "pages/Models.tsx",
  "pages/AdminApiKeys.tsx",
  "pages/Proxies.tsx",
  "pages/Groups.tsx",
  "pages/ImportJobs.tsx",
  "pages/OAuthImport.tsx",
  "pages/ModelRouting.tsx",
  "pages/Config.tsx",
  "pages/Logs.tsx",
  "pages/System.tsx",
  "pages/Tenants.tsx",
  "tenant/pages/DashboardPage.tsx",
  "tenant/pages/UsagePage.tsx",
  "tenant/pages/BillingPage.tsx",
  "tenant/pages/LogsPage.tsx",
  "tenant/pages/ApiKeysPage.tsx",
  "features/billing/admin-cost-report.tsx",
  "features/billing/tenant-cost-report.tsx",
] as const;

const WORKBENCH_ARCHETYPE_EXPECTATIONS = [
  ["pages/Dashboard.tsx", "workspace"],
  ["pages/Usage.tsx", "workspace"],
  ["pages/Billing.tsx", "workspace"],
  ["pages/Accounts.tsx", "workspace"],
  ["pages/Models.tsx", "workspace"],
  ["pages/AdminApiKeys.tsx", "workspace"],
  ["pages/Proxies.tsx", "workspace"],
  ["pages/Groups.tsx", "workspace"],
  ["pages/ImportJobs.tsx", "workspace"],
  ["pages/OAuthImport.tsx", "workspace"],
  ["pages/ModelRouting.tsx", "workspace"],
  ["pages/Logs.tsx", "workspace"],
  ["pages/System.tsx", "workspace"],
  ["pages/Tenants.tsx", "workspace"],
  ["pages/Config.tsx", "settings"],
] as const;

test("all routed surfaces use the shared docked page intro primitive", async () => {
  for (const relativePath of DOCKED_PAGE_FILES) {
    const source = await readFile(path.join(ROOT, relativePath), "utf8");

    assert.match(
      source,
      /DockedPageIntro/,
      `${relativePath} should use the shared DockedPageIntro primitive`,
    );
  }
});

test("DataTable-driven pages should no longer keep top-level titles inside the table toolbar", async () => {
  for (const relativePath of ["pages/Models.tsx", "pages/Logs.tsx"] as const) {
    const source = await readFile(path.join(ROOT, relativePath), "utf8");

    assert.doesNotMatch(
      source,
      /<DataTable[\s\S]*?\s+title=\{t\(/,
      `${relativePath} should move the page title out of the DataTable toolbar`,
    );
    assert.doesNotMatch(
      source,
      /<DataTable[\s\S]*?\s+subtitle=\{t\(/,
      `${relativePath} should move the page subtitle out of the DataTable toolbar`,
    );
  }
});

test("main workbench pages keep their docked title intro on the correct archetype", async () => {
  for (const [relativePath, archetype] of WORKBENCH_ARCHETYPE_EXPECTATIONS) {
    const source = await readFile(path.join(ROOT, relativePath), "utf8");

    assert.match(
      source,
      new RegExp(`archetype="${archetype}"`),
      `${relativePath} should declare archetype="${archetype}" for the shared docked title layout`,
    );
  }
});

test("AdminApiKeys stays on HeroUI-native workbench primitives", async () => {
  const source = await readFile(path.join(ROOT, "pages/AdminApiKeys.tsx"), "utf8");

  assert.match(
    source,
    /from ["']@heroui\/react["']/,
    "AdminApiKeys should use HeroUI native components for its workbench surface",
  );
  assert.doesNotMatch(
    source,
    /components\/DataTable/,
    "AdminApiKeys should not depend on the legacy DataTable wrapper",
  );
  assert.doesNotMatch(
    source,
    /components\/ui\/badge|components\/ui\/button|components\/ui\/input|components\/ui\/surface/,
    "AdminApiKeys should not depend on legacy shadcn-style wrappers",
  );
});

test("Groups stays on HeroUI-native workbench primitives", async () => {
  const source = await readFile(path.join(ROOT, "pages/Groups.tsx"), "utf8");

  assert.match(
    source,
    /from ["']@heroui\/react["']/,
    "Groups should use HeroUI native components for its workbench surface",
  );
  assert.doesNotMatch(
    source,
    /components\/DataTable/,
    "Groups should not depend on the legacy DataTable wrapper",
  );
  assert.doesNotMatch(
    source,
    /components\/ui\/badge|components\/ui\/button|components\/ui\/checkbox|components\/ui\/input|components\/ui\/select|components\/ui\/textarea/,
    "Groups should not depend on legacy shadcn-style form and action wrappers",
  );
});

test("Models avoids rendering raw technical error fields directly into operator surfaces", async () => {
  const source = await readFile(path.join(ROOT, "pages/Models.tsx"), "utf8");

  assert.doesNotMatch(
    source,
    /\{meta\.catalog_last_error\}/,
    "Models should not render raw catalog error text directly in the status card",
  );
  assert.doesNotMatch(
    source,
    /\{selectedModel\.availability_error/,
    "Models should not render raw availability error text directly in the detail surface",
  );
});

test("workbench list and ledger card headers pin their title blocks to the left edge", async () => {
  const expectations = [
    ["pages/Groups.tsx", /CardHeader className="flex flex-col items-start gap-4 px-5 pb-4 pt-5"/],
    ["pages/AdminApiKeys.tsx", /CardHeader className="flex flex-col items-start gap-4 px-5 pb-4 pt-5"/],
    ["pages/Models.tsx", /CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5"/],
    ["pages/Billing.tsx", /CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5"/],
    ["pages/Usage.tsx", /CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5"/],
    ["features/billing/admin-cost-report.tsx", /CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5"/],
  ] as const;

  for (const [relativePath, pattern] of expectations) {
    const source = await readFile(path.join(ROOT, relativePath), "utf8");

    assert.match(
      source,
      pattern,
      `${relativePath} should opt out of HeroUI CardHeader cross-axis centering for vertically stacked title blocks`,
    );
  }
});
