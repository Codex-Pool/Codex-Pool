/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'

test('shared copy actions route success feedback through notify with contextual page copy', async () => {
  const tenantCopyUtils = await readFile(new URL('../features/tenants/utils.ts', import.meta.url), 'utf8')
  const adminApiKeys = await readFile(new URL('./AdminApiKeys.tsx', import.meta.url), 'utf8')
  const tenantsPage = await readFile(new URL('./Tenants.tsx', import.meta.url), 'utf8')
  const en = await readFile(new URL('../locales/en.ts', import.meta.url), 'utf8')
  const zh = await readFile(new URL('../locales/zh-CN.ts', import.meta.url), 'utf8')

  assert.match(
    tenantCopyUtils,
    /import \{ notify \} from ['"]@\/lib\/notification['"]/,
    'shared tenant copy helpers should adopt the shared notify utility',
  )
  assert.match(
    tenantCopyUtils,
    /successTitle: string/,
    'shared tenant copy helpers should require contextual success titles',
  )
  assert.match(
    tenantCopyUtils,
    /errorDescription: string/,
    'shared tenant copy helpers should require contextual failure descriptions',
  )
  assert.match(
    tenantCopyUtils,
    /variant: 'success'/,
    'shared tenant copy helpers should emit success notifications after copy succeeds',
  )
  assert.match(
    tenantCopyUtils,
    /variant: 'error'/,
    'shared tenant copy helpers should emit error notifications when copy fails',
  )

  assert.match(
    adminApiKeys,
    /copyText\(createdKey\.plaintext_key,\s*\{[\s\S]*apiKeys\.dialog\.created\.copyPlaintext[\s\S]*apiKeys\.notifications\.copyPlaintextSuccess[\s\S]*apiKeys\.notifications\.copyPlaintextFailed[\s\S]*\}\)/,
    'admin API key creation should notify after plaintext copy succeeds or fails',
  )
  assert.match(
    adminApiKeys,
    /copyText\(key\.key_prefix,\s*\{[\s\S]*apiKeys\.actions\.copyPrefix[\s\S]*apiKeys\.notifications\.copyPrefixSuccess[\s\S]*apiKeys\.notifications\.copyPrefixFailed[\s\S]*\}\)/,
    'admin API key prefix copy actions should notify after copy succeeds or fails',
  )

  assert.match(
    tenantsPage,
    /copyText\(row\.original\.key_prefix,\s*\{[\s\S]*tenants\.keys\.list\.copyPrefix[\s\S]*tenants\.notifications\.copyPrefixSuccess[\s\S]*tenants\.notifications\.copyPrefixFailed[\s\S]*\}\)/,
    'tenant key prefix copy actions should notify after copy succeeds or fails',
  )
  assert.match(
    tenantsPage,
    /copyText\(lastImpersonation\.access_token,\s*\{[\s\S]*tenants\.impersonation\.copyToken[\s\S]*tenants\.notifications\.copyTokenSuccess[\s\S]*tenants\.notifications\.copyTokenFailed[\s\S]*\}\)/,
    'tenant impersonation token copy actions should notify after copy succeeds or fails',
  )
  assert.match(
    tenantsPage,
    /copyText\(createdKey\.plaintext_key,\s*\{[\s\S]*tenants\.keys\.created\.copyPlaintext[\s\S]*tenants\.notifications\.copyPlaintextSuccess[\s\S]*tenants\.notifications\.copyPlaintextFailed[\s\S]*\}\)/,
    'tenant plaintext key copy actions should notify after copy succeeds or fails',
  )

  assert.match(
    en,
    /copyPrefixSuccess: "Copied key prefix\."/,
    'en should define copy-prefix success notification copy',
  )
  assert.match(
    en,
    /copyPlaintextSuccess: "Copied plaintext key\."/,
    'en should define plaintext-key success notification copy',
  )
  assert.match(
    en,
    /copyTokenSuccess: "Copied token\."/,
    'en should define token success notification copy',
  )

  assert.match(
    zh,
    /copyPrefixSuccess: "已复制 key 前缀。"/,
    'zh-CN should define copy-prefix success notification copy',
  )
  assert.match(
    zh,
    /copyPlaintextSuccess: "已复制明文密钥。"/,
    'zh-CN should define plaintext-key success notification copy',
  )
  assert.match(
    zh,
    /copyTokenSuccess: "已复制 token。"/,
    'zh-CN should define token success notification copy',
  )
})
