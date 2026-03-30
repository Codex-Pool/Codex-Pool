/// <reference types="node" />

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL(".", import.meta.url));

test("high-priority admin and tenant surfaces avoid direct raw error leaks", async () => {
  const accounts = await readFile(path.join(ROOT, "pages/Accounts.tsx"), "utf8");
  const models = await readFile(path.join(ROOT, "pages/Models.tsx"), "utf8");
  const tenantApp = await readFile(path.join(ROOT, "tenant/TenantApp.tsx"), "utf8");
  const oauthImport = await readFile(path.join(ROOT, "pages/OAuthImport.tsx"), "utf8");
  const dashboard = await readFile(path.join(ROOT, "pages/Dashboard.tsx"), "utf8");

  assert.doesNotMatch(
    accounts,
    /failed\.error\.message|extractApiErrorMessage\(/,
    "Accounts should not surface raw account-pool action messages directly to operators",
  );
  assert.match(
    accounts,
    /localizeApiErrorDisplay\(t, error, fallback\)\.label/,
    "Accounts should use localized API error labels for action failures",
  );

  assert.doesNotMatch(
    models,
    /extractApiErrorMessage\(/,
    "Models should not surface raw API error messages directly in notifications",
  );
  assert.match(
    models,
    /localizeApiErrorDisplay\(/,
    "Models should localize action and clipboard failure messages",
  );

  assert.doesNotMatch(
    tenantApp,
    /extractTenantApiErrorMessage|debug_code/,
    "Tenant auth should not expose raw tenant auth errors or debug codes",
  );
  assert.match(
    tenantApp,
    /localizeApiErrorDisplay\(t, err, t\('tenantApp\.auth\.error\./,
    "Tenant auth failures should flow through the shared API error localization helper",
  );

  assert.doesNotMatch(
    oauthImport,
    /session\.error\.message/,
    "OAuth import should not render raw backend error messages",
  );
  assert.match(
    oauthImport,
    /getPlanLabel\(/,
    "OAuth import should localize plan labels through the shared account plan helper",
  );

  assert.doesNotMatch(
    dashboard,
    /message:\s*systemState\.data_plane_error/,
    "Dashboard should not render the raw data plane error string in alert cards",
  );
});

test("logs range label and import item fallbacks stay localized", async () => {
  const logs = await readFile(path.join(ROOT, "pages/Logs.tsx"), "utf8");
  const importJobs = await readFile(path.join(ROOT, "pages/ImportJobs.tsx"), "utf8");
  const groups = await readFile(path.join(ROOT, "pages/Groups.tsx"), "utf8");
  const config = await readFile(path.join(ROOT, "pages/Config.tsx"), "utf8");
  const tenantApiKeys = await readFile(
    path.join(ROOT, "tenant/pages/ApiKeysPage.tsx"),
    "utf8",
  );
  const tenantBilling = await readFile(
    path.join(ROOT, "tenant/pages/BillingPage.tsx"),
    "utf8",
  );
  const zh = await readFile(path.join(ROOT, "locales/zh-CN.ts"), "utf8");
  const en = await readFile(path.join(ROOT, "locales/en.ts"), "utf8");

  assert.match(
    logs,
    /t\('logs\.events\.filters\.range'\)/,
    "Logs should use the localized events.filters.range label",
  );
  assert.match(
    zh,
    /range:\s*"时间范围"/,
    "zh-CN should define the logs events range label",
  );
  assert.match(
    en,
    /range:\s*"Time range"/,
    "en should define the logs events range label",
  );
  assert.match(
    zh,
    /tokenSegments:\s*\{\s*cached:\s*"缓存",\s*input:\s*"输入",\s*output:\s*"输出"/,
    "zh-CN should expose shared token segment labels",
  );
  assert.match(
    en,
    /tokenSegments:\s*\{\s*cached:\s*"Cached",\s*input:\s*"Input",\s*output:\s*"Output"/,
    "en should expose shared token segment labels",
  );

  assert.match(
    importJobs,
    /function resolveImportIssueLabel/,
    "ImportJobs should resolve import item issues through a localized helper",
  );
  assert.doesNotMatch(
    importJobs,
    /const messageDetail\s*=\s*item\.error_message|item\.admission_source\s*\?\?\s*item\.admission_reason\s*\?\?\s*"-"/,
    "ImportJobs should not render raw item error text or admission source labels in the detail table",
  );
  assert.match(
    groups,
    /common\.tokenSegments\.input|common\.tokenSegments\.cached|common\.tokenSegments\.output/,
    "Groups pricing previews should use shared token segment labels",
  );
  assert.match(
    tenantApiKeys,
    /common\.tokenSegments\.input|common\.tokenSegments\.cached|common\.tokenSegments\.output/,
    "Tenant API keys pricing previews should use shared token segment labels",
  );
  assert.match(
    tenantBilling,
    /common\.tokenSegments\.input|common\.tokenSegments\.cached|common\.tokenSegments\.output/,
    "Tenant billing pricing previews should use shared token segment labels",
  );
  assert.match(
    config,
    /common\.units\.secondsShort/,
    "Config should format refresh interval seconds through shared i18n units",
  );
});

test("remaining operator-facing diagnostics stay localized", async () => {
  const errorI18n = await readFile(path.join(ROOT, "api/errorI18n.ts"), "utf8");
  const logs = await readFile(path.join(ROOT, "pages/Logs.tsx"), "utf8");
  const tenantLogs = await readFile(path.join(ROOT, "tenant/pages/LogsPage.tsx"), "utf8");
  const accountsColumns = await readFile(
    path.join(ROOT, "features/accounts/use-accounts-columns.tsx"),
    "utf8",
  );
  const accountDetail = await readFile(
    path.join(ROOT, "features/accounts/account-detail-dialog.tsx"),
    "utf8",
  );
  const importPanels = await readFile(path.join(ROOT, "features/import-jobs/panels.tsx"), "utf8");
  const proxies = await readFile(path.join(ROOT, "pages/Proxies.tsx"), "utf8");
  const zh = await readFile(path.join(ROOT, "locales/zh-CN.ts"), "utf8");
  const en = await readFile(path.join(ROOT, "locales/en.ts"), "utf8");

  assert.match(
    errorI18n,
    /function buildDiagnosticTooltip\(parts: string\[\]\): string \| undefined \{\s*if \(!import\.meta\.env\.DEV\)/,
    "Shared error localization should keep diagnostic tooltips in development only",
  );

  assert.match(
    logs,
    /function localizeEventType\(/,
    "Logs should localize unified event stream event types through a dedicated mapper",
  );
  assert.match(
    logs,
    /function localizeReasonCode\(/,
    "Logs should localize reason codes through a dedicated mapper",
  );
  assert.match(
    logs,
    /function localizeRoutingDecision\(/,
    "Logs should localize routing decisions through a dedicated mapper",
  );
  assert.doesNotMatch(
    logs,
    /<div className="font-medium text-foreground">\{item\.event_type\}<\/div>|selectedEvent\?\.event_type \?\? t\('logs\.events\.detailTitle'\)|\{item\.reason_code \?\? '-'\}|\{selectedEvent\.auth_provider \?\? '-'\}/,
    "Logs should not render raw event_type, reason_code, or auth metadata directly in operator views",
  );
  assert.match(
    zh,
    /eventTypes:\s*\{[\s\S]*requestReceived:\s*"收到请求"[\s\S]*unknown:\s*"未知事件"/,
    "zh-CN should define localized system event type labels",
  );
  assert.match(
    en,
    /eventTypes:\s*\{[\s\S]*requestReceived:\s*"Request received"[\s\S]*unknown:\s*"Unknown event"/,
    "en should define localized system event type labels",
  );

  assert.match(
    tenantLogs,
    /function localizeTenantAuditTargetType\(/,
    "Tenant logs should localize audit target types",
  );
  assert.match(
    tenantLogs,
    /function summarizeTenantAuditPayload\(/,
    "Tenant logs should summarize audit payload presence instead of dumping raw JSON in the table",
  );
  assert.doesNotMatch(
    tenantLogs,
    /title=\{display\.tooltip\}|title=\{row\.original\.actor_type\}|title=\{row\.original\.action\}|title=\{row\.original\.result_status\}|row\.original\.target_type \?\? '-'|JSON\.stringify\(row\.original\.payload_json\)/,
    "Tenant logs should not leak raw request/audit codes and payloads through table cells or tooltips",
  );
  assert.match(
    zh,
    /targetTypes:\s*\{[\s\S]*requestLogs:\s*"请求日志"[\s\S]*unknown:\s*"未知目标"/,
    "zh-CN should define localized tenant audit target labels",
  );
  assert.match(
    en,
    /targetTypes:\s*\{[\s\S]*requestLogs:\s*"Request logs"[\s\S]*unknown:\s*"Unknown target"/,
    "en should define localized tenant audit target labels",
  );

  assert.doesNotMatch(
    accountsColumns,
    /title=\{errorDisplay\.tooltip\}/,
    "Accounts table should not expose localized OAuth error tooltips directly",
  );
  assert.doesNotMatch(
    accountDetail,
    /title=\{refreshErrorDisplay\.tooltip\}|title=\{rateLimitErrorDisplay\.tooltip\}/,
    "Account detail dialog should not expose raw OAuth error codes via tooltips",
  );
  assert.doesNotMatch(
    importPanels,
    /title=\{errorDisplay\.tooltip\}|title=\{entry\.error_code\}/,
    "Import job panels should not expose raw error codes through titles",
  );

  assert.match(
    proxies,
    /proxies\.antigravity\.lastErrorSummary/,
    "Proxy cards should show a stable localized summary for the latest probe error",
  );
  assert.doesNotMatch(
    proxies,
    /\{proxy\.lastError\}/,
    "Proxy cards should not render raw lastError strings directly to operators",
  );
});
