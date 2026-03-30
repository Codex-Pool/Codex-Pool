/// <reference types="node" />

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL(".", import.meta.url));

test("models and tenants trim repeated workbench copy from primary surfaces", async () => {
  const models = await readFile(path.join(ROOT, "pages/Models.tsx"), "utf8");
  const tenants = await readFile(path.join(ROOT, "pages/Tenants.tsx"), "utf8");

  assert.doesNotMatch(
    models,
    /meta=\{t\("models\.description"\)\}/,
    "Models should not repeat the page purpose in DockedPageIntro meta",
  );
  assert.doesNotMatch(
    models,
    /models\.antigravity\.(summaryDescription|catalogDescription|directoryDescription)/,
    "Models should not repeat explanatory section descriptions on the first-run workbench surfaces",
  );
  assert.doesNotMatch(
    models,
    /<CardBody className="gap-4 px-5 pb-5 pt-1">[\s\S]*?<Divider \/>[\s\S]*?models\.antigravity\.cacheUpdatedAt/,
    "Models catalog summary should not keep a divider that only separated header copy from the metrics grid",
  );

  assert.match(
    tenants,
    /<DockedPageIntro[\s\S]*actions=\{\(/,
    "Tenants should dock refresh actions in the shared page intro",
  );
  assert.doesNotMatch(
    tenants,
    /<SectionHeader[\s\S]*description=\{t\('tenants\.subtitle'/,
    "Tenants create panel should not repeat the page subtitle",
  );
  assert.doesNotMatch(
    tenants,
    /<SectionHeader[\s\S]*description=\{t\('tenants\.list\.searchPlaceholder'/,
    "Tenants list panel should not repeat the table search placeholder as section copy",
  );
});

test("tenant dashboard and billing keep section headers concise", async () => {
  const dashboard = await readFile(
    path.join(ROOT, "tenant/pages/DashboardPage.tsx"),
    "utf8",
  );
  const billing = await readFile(path.join(ROOT, "tenant/pages/BillingPage.tsx"), "utf8");

  assert.doesNotMatch(
    dashboard,
    /dashboard\.filters\.description|tenantDashboard\.filters\.apiKeyHint|tenantDashboard\.groupOverview\.(singleDescription|allDescription)|tenantDashboard\.tokenTrend\.description|tenantDashboard\.modelDistribution\.description/,
    "Tenant dashboard should keep first-screen sections concise and avoid repeated explanatory copy",
  );

  assert.doesNotMatch(
    billing,
    /tenantBilling\.(trend|groupPricing|snapshot|ledger)\.description/,
    "Tenant billing should not repeat the page subtitle across the first group of workbench panels",
  );
});
