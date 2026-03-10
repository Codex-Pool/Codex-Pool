import { apiClient } from './client'

export type ModelAvailabilityStatus = 'unknown' | 'available' | 'unavailable'

export interface ModelPricingItem {
  id: string
  model: string
  input_price_microcredits: number
  cached_input_price_microcredits: number
  output_price_microcredits: number
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface AdminModelPricingView {
  input_price_microcredits?: number | null
  cached_input_price_microcredits?: number | null
  output_price_microcredits?: number | null
  source: string
}

export interface AdminModelOfficialInfo {
  title: string
  description?: string | null
  context_window_tokens?: number | null
  max_output_tokens?: number | null
  knowledge_cutoff?: string | null
  reasoning_token_support?: boolean | null
  input_price_microcredits?: number | null
  cached_input_price_microcredits?: number | null
  output_price_microcredits?: number | null
  pricing_notes?: string | null
  input_modalities: string[]
  output_modalities: string[]
  endpoints: string[]
  source_url: string
  synced_at: string
  raw_text?: string | null
}

export interface ModelSchema {
  id: string
  owned_by: string
  availability_status: ModelAvailabilityStatus
  availability_checked_at?: string | null
  availability_http_status?: number | null
  availability_error?: string | null
  official: AdminModelOfficialInfo
  override_pricing?: ModelPricingItem | null
  effective_pricing: AdminModelPricingView
}

export interface ModelsMeta {
  probe_cache_ttl_sec: number
  probe_cache_stale: boolean
  probe_cache_updated_at?: string | null
  probe_source_account_label?: string | null
  catalog_synced_at?: string | null
  catalog_sync_required: boolean
  catalog_last_error?: string | null
}

export interface ListModelsResponse {
  object: string
  data: ModelSchema[]
  meta: ModelsMeta
}

export interface ProbeModelsRequest {
  force?: boolean
  models?: string[]
}

export interface ModelPricingUpsertRequest {
  model: string
  input_price_microcredits: number
  cached_input_price_microcredits: number
  output_price_microcredits: number
  enabled: boolean
}

export interface OpenAiModelsSyncResponse {
  models_total: number
  created_or_updated: number
  deleted_catalog_rows: number
  cleared_custom_entities: number
  cleared_billing_rules: number
  deleted_legacy_pricing_rows: number
  synced_at: string
}

export const modelsApi = {
  listModels: () =>
    apiClient.get<ListModelsResponse>('/admin/models', {
      timeout: 30000,
    }),
  syncOpenAiCatalog: () =>
    apiClient.post<OpenAiModelsSyncResponse>('/admin/models/sync-openai', {}),
  probeModels: (payload: ProbeModelsRequest = {}) =>
    apiClient.post<ListModelsResponse>('/admin/models/probe', payload, {
      timeout: 120000,
    }),
  listModelPricing: () => apiClient.get<ModelPricingItem[]>('/admin/model-pricing'),
  upsertModelPricing: (payload: ModelPricingUpsertRequest) =>
    apiClient.post<ModelPricingItem>('/admin/model-pricing', payload),
  deleteModelPricing: (pricingId: string) =>
    apiClient.delete<void>(`/admin/model-pricing/${pricingId}`),
}
