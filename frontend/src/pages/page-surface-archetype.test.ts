/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = fileURLToPath(new URL('.', import.meta.url))

const DASHBOARD_FAMILY_PAGES = [
  'Dashboard.tsx',
  'Usage.tsx',
  'Billing.tsx',
  'System.tsx',
  'Config.tsx',
  'ImportJobs.tsx',
  'Proxies.tsx',
] as const

test('dashboard-family pages should use shared antigravity surface primitives instead of legacy card skins', async () => {
  for (const relativePath of DASHBOARD_FAMILY_PAGES) {
    const source = await readFile(path.join(ROOT, relativePath), 'utf8')

    assert.doesNotMatch(
      source,
      /border border-default-100 bg-content1 shadow-md/,
      `${relativePath} should not keep the legacy page Card skin`,
    )
  }
})
