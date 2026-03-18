/// <reference types="node" />

import assert from 'node:assert/strict'
import test from 'node:test'

import { DEFAULT_SYSTEM_CAPABILITIES } from './system.defaults.ts'

test('DEFAULT_SYSTEM_CAPABILITIES keeps capability-gated flows off before the real edition payload loads', () => {
  assert.equal(DEFAULT_SYSTEM_CAPABILITIES.edition, 'personal')
  assert.equal(DEFAULT_SYSTEM_CAPABILITIES.billing_mode, 'cost_report_only')
  assert.equal(DEFAULT_SYSTEM_CAPABILITIES.features.multi_tenant, false)
  assert.equal(DEFAULT_SYSTEM_CAPABILITIES.features.tenant_portal, false)
  assert.equal(DEFAULT_SYSTEM_CAPABILITIES.features.tenant_self_service, false)
  assert.equal(DEFAULT_SYSTEM_CAPABILITIES.features.tenant_recharge, false)
  assert.equal(DEFAULT_SYSTEM_CAPABILITIES.features.credit_billing, false)
  assert.equal(DEFAULT_SYSTEM_CAPABILITIES.features.cost_reports, true)
})
