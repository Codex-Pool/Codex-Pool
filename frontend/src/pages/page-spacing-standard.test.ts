/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('..', import.meta.url))

const PAGE_CONTENT_FILES = [
  'pages/Dashboard.tsx',
  'pages/Usage.tsx',
  'pages/Billing.tsx',
  'pages/Accounts.tsx',
  'pages/Models.tsx',
  'pages/AdminApiKeys.tsx',
  'pages/Proxies.tsx',
  'pages/Groups.tsx',
  'pages/ImportJobs.tsx',
  'pages/OAuthImport.tsx',
  'pages/ModelRouting.tsx',
  'pages/Config.tsx',
  'pages/Logs.tsx',
  'pages/System.tsx',
  'pages/Tenants.tsx',
  'tenant/pages/DashboardPage.tsx',
  'tenant/pages/UsagePage.tsx',
  'tenant/pages/BillingPage.tsx',
  'tenant/pages/LogsPage.tsx',
  'tenant/pages/ApiKeysPage.tsx',
  'features/billing/admin-cost-report.tsx',
  'features/billing/tenant-cost-report.tsx',
] as const

test('all routed surfaces should use the shared PageContent gutter primitive', async () => {
  for (const relativePath of PAGE_CONTENT_FILES) {
    const source = await readFile(path.join(ROOT, relativePath), 'utf8')

    assert.match(
      source,
      /PageContent/,
      `${relativePath} should use the shared PageContent primitive for outer gutters`,
    )
  }
})

test('routed surfaces should no longer hardcode 32px desktop gutters or missing page padding', async () => {
  for (const relativePath of PAGE_CONTENT_FILES) {
    const source = await readFile(path.join(ROOT, relativePath), 'utf8')

    assert.doesNotMatch(
      source,
      /\blg:p-8\b/,
      `${relativePath} should not hardcode 32px desktop page gutters`,
    )
  }
})
