/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'

async function readSource(relativePath: string) {
  return readFile(new URL(relativePath, import.meta.url), 'utf8')
}

test('admin client uses shared auth client contract instead of localStorage redirects', async () => {
  const source = await readSource('./api/client.ts')

  assert.match(source, /createAuthApiClient/, 'admin client should use createAuthApiClient')
  assert.match(source, /getAdminAccessToken/, 'admin client should read token from admin session')
  assert.match(source, /AUTH_REQUIRED_EVENT/, 'admin client should export auth required event')
  assert.match(source, /LOGIN_FAILED_EVENT/, 'admin client should export login failed event')
  assert.match(
    source,
    /\/account-pool\/accounts\/.*\/responses\/test|shouldDispatchAuthRequired/,
    'admin client should keep account-pool real test 401s from triggering global auth expiry handling',
  )
  assert.doesNotMatch(
    source,
    /localStorage\.getItem\('access_token'\)/,
    'admin client should not read tokens from localStorage directly',
  )
  assert.doesNotMatch(
    source,
    /window\.location\.href = '\/login'/,
    'admin client should not force redirect inside interceptor',
  )
})

test('tenant client uses shared auth client contract without mutating session inside interceptor', async () => {
  const source = await readSource('./api/tenantClient.ts')

  assert.match(source, /createAuthApiClient/, 'tenant client should use createAuthApiClient')
  assert.match(source, /getTenantAccessToken/, 'tenant client should read token from tenant session')
  assert.match(source, /TENANT_AUTH_REQUIRED_EVENT/, 'tenant client should export auth required event')
  assert.match(source, /TENANT_LOGIN_FAILED_EVENT/, 'tenant client should export login failed event')
  assert.doesNotMatch(
    source,
    /clearTenantAccessToken/,
    'tenant client should delegate session cleanup to app shell handlers',
  )
})

test('app shell uses admin auth events and in-memory session helpers', async () => {
  const source = await readSource('./App.tsx')

  assert.match(source, /adminAuthApi/, 'App should use adminAuthApi')
  assert.match(source, /setAdminAccessToken/, 'App should set admin session token via helper')
  assert.match(source, /clearAdminAccessToken/, 'App should clear admin session token via helper')
  assert.match(source, /AUTH_REQUIRED_EVENT/, 'App should listen to auth required event')
  assert.match(source, /LOGIN_FAILED_EVENT/, 'App should listen to login failed event')
  assert.match(source, /queryClient\.clear\(\)/, 'App should clear query cache when session changes')
  assert.doesNotMatch(source, /authApi\b/, 'App should not use legacy authApi')
  assert.doesNotMatch(
    source,
    /localStorage\.(getItem|setItem|removeItem)\('access_token'\)/,
    'App should not manipulate admin token through localStorage',
  )
})

test('tenant session is purely in-memory', async () => {
  const source = await readSource('./lib/tenant-session.ts')

  assert.match(source, /let tenantAccessToken: string \| null = null/, 'tenant session should keep in-memory token')
  assert.doesNotMatch(source, /localStorage/, 'tenant session should not persist tokens in localStorage')
})

test('legacy account helpers no longer depend on effective_enabled semantics', async () => {
  const columnsSource = await readSource('./features/accounts/use-accounts-columns.tsx')
  const detailSource = await readSource('./features/accounts/account-detail-dialog.tsx')

  assert.doesNotMatch(
    columnsSource,
    /effective_enabled/,
    'legacy account columns should not read effective_enabled',
  )
  assert.doesNotMatch(
    detailSource,
    /effective_enabled/,
    'legacy account detail should not read effective_enabled',
  )
})
