/// <reference types="node" />

import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const ACCOUNTS_API_PATH = new URL('./accounts.ts', import.meta.url)
const ADMIN_TENANTS_API_PATH = new URL('./adminTenants.ts', import.meta.url)
const GROUPS_API_PATH = new URL('./groups.ts', import.meta.url)
const MODEL_ROUTING_API_PATH = new URL('./modelRouting.ts', import.meta.url)
const SETTINGS_API_PATH = new URL('./settings.ts', import.meta.url)

test('accountsApi exposes the main branch account management surface', () => {
  const requiredMethods = [
    'listAccounts',
    'setEnabled',
    'deleteAccount',
    'listOAuthStatuses',
    'getOAuthStatus',
    'refreshOAuth',
    'refreshOAuthJob',
    'createRateLimitRefreshJob',
    'getRateLimitRefreshJob',
    'disableFamily',
    'enableFamily',
    'batchOperate',
  ]

  return readFile(ACCOUNTS_API_PATH, 'utf8').then((source) => {
    requiredMethods.forEach((method) => {
      assert.match(source, new RegExp(`${method}\\s*:`))
    })
  })
})

test('accountPoolApi exposes the account-pool detail testing surface', async () => {
  const source = await readFile(ACCOUNTS_API_PATH, 'utf8')

  ;[
    'getSummary',
    'listRecords',
    'getRecord',
    'getSignalHeatmap',
    'runAction',
    'testResponses',
  ].forEach((method) => {
    assert.match(source, new RegExp(`${method}\\s*:`))
  })
})

test('adminTenantsApi exposes the main branch tenant management surface', () => {
  const requiredMethods = [
    'listTenants',
    'ensureDefaultTenant',
    'createTenant',
    'patchTenant',
    'rechargeTenant',
    'getTenantCreditBalance',
    'getTenantCreditSummary',
    'getTenantCreditLedger',
    'listModelPricing',
    'upsertModelPricing',
    'createImpersonation',
    'deleteImpersonation',
  ]

  return readFile(ADMIN_TENANTS_API_PATH, 'utf8').then((source) => {
    requiredMethods.forEach((method) => {
      assert.match(source, new RegExp(`${method}\\s*:`))
    })
  })
})

test('groupsApi exposes the main branch admin editing surface', () => {
  const requiredMethods = [
    'adminList',
    'adminUpsert',
    'adminDelete',
    'adminUpsertPolicy',
    'adminDeletePolicy',
  ]

  return readFile(GROUPS_API_PATH, 'utf8').then((source) => {
    requiredMethods.forEach((method) => {
      assert.match(source, new RegExp(`${method}\\s*:`))
    })
  })
})

test('modelRoutingApi exposes the main branch routing management surface', () => {
  const requiredMethods = [
    'listProfiles',
    'upsertProfile',
    'deleteProfile',
    'listPolicies',
    'upsertPolicy',
    'deletePolicy',
    'getSettings',
    'updateSettings',
    'getClaudeCodeSettings',
    'updateClaudeCodeSettings',
    'listVersions',
  ]

  return readFile(MODEL_ROUTING_API_PATH, 'utf8').then((source) => {
    requiredMethods.forEach((method) => {
      assert.match(source, new RegExp(`${method}\\s*:`))
    })
  })
})

test('settings module exposes standalone admin api key management methods', async () => {
  const source = await readFile(SETTINGS_API_PATH, 'utf8')
  assert.match(source, /apiKeysApi/)
  assert.match(source, /listKeys\s*:/)
  assert.match(source, /createKey\s*:/)
  assert.match(source, /updateKeyEnabled\s*:/)
})
