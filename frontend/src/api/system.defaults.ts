import type { SystemCapabilitiesResponse } from './types'

export const DEFAULT_SYSTEM_CAPABILITIES: SystemCapabilitiesResponse = {
  edition: 'personal',
  billing_mode: 'cost_report_only',
  features: {
    multi_tenant: false,
    tenant_portal: false,
    tenant_self_service: false,
    tenant_recharge: false,
    credit_billing: false,
    cost_reports: true,
  },
}
