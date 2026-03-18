export interface AdminSystemCounts {
  total_accounts: number
  enabled_accounts: number
  oauth_accounts: number
  api_keys: number
  tenants: number
}

export interface DataPlaneDebugState {
  snapshot_revision?: number
  account_total?: number
  active_account_total?: number
  auth_mode?: string
  auth_fail_open?: boolean
  allowlist_api_key_total?: number
  auth_validator_enabled?: boolean
  sticky_session_total?: number
  sticky_hit_count?: number
  sticky_miss_count?: number
  sticky_rebind_count?: number
  sticky_mapping_total?: number
  sticky_hit_ratio?: number
  failover_enabled?: boolean
  same_account_quick_retry_max?: number
  request_failover_wait_ms?: number
  retry_poll_interval_ms?: number
  sticky_prefer_non_conflicting?: boolean
  shared_routing_cache_enabled?: boolean
  failover_attempt_total?: number
  failover_success_total?: number
  failover_exhausted_total?: number
  same_account_retry_total?: number
  billing_authorize_total?: number
  routing_cache_local_sticky_hit_total?: number
  routing_cache_local_sticky_miss_total?: number
  routing_cache_shared_sticky_hit_total?: number
  routing_cache_shared_sticky_miss_total?: number
  billing_preauth_error_ratio_count_total?: number
  billing_preauth_error_ratio_avg?: number
  billing_preauth_error_ratio_p50?: number
  billing_preauth_error_ratio_p95?: number
  billing_preauth_capture_missing_total?: number
  billing_settle_complete_total?: number
  billing_release_without_capture_total?: number
  billing_settle_complete_ratio?: number
  billing_release_without_capture_ratio?: number
  billing_preauth_model_error_stats?: {
    model: string
    sample_count: number
    avg_ratio: number
    p50_ratio: number
    p95_ratio: number
  }[]
}

export interface ControlPlaneDebugState {
  billing_reconcile_scanned_total?: number
  billing_reconcile_adjust_total?: number
  billing_reconcile_failed_total?: number
  billing_reconcile_released_total?: number
}

export interface RuntimeConfigSnapshot {
  control_plane_listen: string
  data_plane_base_url: string
  auth_validate_url: string
  oauth_refresh_enabled: boolean
  oauth_refresh_interval_sec: number
  database_url?: string
  redis_url?: string
  clickhouse_url?: string
  notes?: string
}

export type ProductEdition = 'personal' | 'team' | 'business'

export type BillingMode = 'cost_report_only' | 'credit_enforced'

export interface EditionFeatures {
  multi_tenant: boolean
  tenant_portal: boolean
  tenant_self_service: boolean
  tenant_recharge: boolean
  credit_billing: boolean
  cost_reports: boolean
}

export interface SystemCapabilitiesResponse {
  edition: ProductEdition
  billing_mode: BillingMode
  features: EditionFeatures
}

export interface RuntimeConfigUpdateRequest {
  data_plane_base_url?: string
  auth_validate_url?: string
  oauth_refresh_enabled?: boolean
  oauth_refresh_interval_sec?: number
  notes?: string
}

export interface AdminSystemStateResponse {
  generated_at: string
  started_at: string
  uptime_sec: number
  usage_repo_available: boolean
  config: RuntimeConfigSnapshot
  counts: AdminSystemCounts
  control_plane_debug?: ControlPlaneDebugState
  data_plane_debug?: DataPlaneDebugState
  data_plane_error?: string
}

export interface UsageSummaryQueryResponse {
  start_ts: number
  end_ts: number
  account_total_requests: number
  tenant_api_key_total_requests: number
  unique_account_count: number
  unique_tenant_api_key_count: number
  estimated_cost_microusd?: number
  dashboard_metrics?: UsageDashboardMetrics
}

export interface HourlyUsageTotalPoint {
  hour_start: number
  request_count: number
}

export interface UsageHourlyTrendsResponse {
  start_ts: number
  end_ts: number
  account_totals: HourlyUsageTotalPoint[]
  tenant_api_key_totals: HourlyUsageTotalPoint[]
  dashboard_metrics?: UsageDashboardMetrics
}

export interface UsageDashboardTokenBreakdown {
  input_tokens: number
  cached_input_tokens: number
  output_tokens: number
  reasoning_tokens: number
  total_tokens: number
}

export interface UsageDashboardTokenTrendPoint {
  hour_start: number
  request_count: number
  input_tokens: number
  cached_input_tokens: number
  output_tokens: number
  reasoning_tokens: number
  total_tokens: number
  estimated_cost_microusd?: number
}

export interface UsageDashboardModelDistributionItem {
  model: string
  request_count: number
  total_tokens: number
}

export interface UsageDashboardMetrics {
  total_requests: number
  estimated_cost_microusd?: number
  token_breakdown: UsageDashboardTokenBreakdown
  avg_first_token_latency_ms?: number
  token_trends: UsageDashboardTokenTrendPoint[]
  model_request_distribution: UsageDashboardModelDistributionItem[]
  model_token_distribution: UsageDashboardModelDistributionItem[]
}

export interface HourlyTenantUsageTotalPoint {
  tenant_id: string
  hour_start: number
  request_count: number
}

export interface UsageHourlyTenantTrendsResponse {
  start_ts: number
  end_ts: number
  items: HourlyTenantUsageTotalPoint[]
}

export interface TenantUsageLeaderboardItem {
  tenant_id: string
  total_requests: number
}

export interface AccountUsageLeaderboardItem {
  account_id: string
  total_requests: number
}

export interface ApiKeyUsageLeaderboardItem {
  tenant_id: string
  api_key_id: string
  total_requests: number
}

export interface UsageLeaderboardOverviewResponse {
  start_ts: number
  end_ts: number
  tenants: TenantUsageLeaderboardItem[]
  accounts: AccountUsageLeaderboardItem[]
  api_keys: ApiKeyUsageLeaderboardItem[]
  summary?: UsageSummaryQueryResponse
}

export interface TenantUsageLeaderboardResponse {
  start_ts: number
  end_ts: number
  items: TenantUsageLeaderboardItem[]
}

export interface AccountUsageLeaderboardResponse {
  start_ts: number
  end_ts: number
  items: AccountUsageLeaderboardItem[]
}

export interface ApiKeyUsageLeaderboardResponse {
  start_ts: number
  end_ts: number
  items: ApiKeyUsageLeaderboardItem[]
}

export interface AdminLogEntry {
  id: number
  ts: string
  level: string
  action: string
  message: string
}

export interface AdminLogsResponse {
  items: AdminLogEntry[]
}

export type ProxyFailMode = 'strict_proxy' | 'allow_direct_fallback'

export interface AdminProxyPoolSettings {
  enabled: boolean
  fail_mode: ProxyFailMode
  updated_at: string
}

export interface AdminProxyNode {
  id: string
  label: string
  proxy_url_masked: string
  scheme: string
  has_auth: boolean
  enabled: boolean
  weight: number
  last_test_status?: string
  last_latency_ms?: number
  last_error?: string
  last_tested_at?: string
  updated_at: string
}

export interface AdminProxyPoolResponse {
  settings: AdminProxyPoolSettings
  nodes: AdminProxyNode[]
}

export interface CreateAdminProxyNodeRequest {
  label: string
  proxy_url: string
  enabled?: boolean
  weight?: number
}

export interface UpdateAdminProxyNodeRequest {
  label?: string
  proxy_url?: string
  enabled?: boolean
  weight?: number
}

export interface UpdateAdminProxyPoolSettingsRequest {
  enabled: boolean
  fail_mode: ProxyFailMode
}

export interface AdminProxyNodeMutationResponse {
  node: AdminProxyNode
}

export interface AdminProxyPoolSettingsResponse {
  settings: AdminProxyPoolSettings
}

export interface AdminProxyTestResponse {
  tested: number
  results: AdminProxyNode[]
}
