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

export interface AdminModelSectionItem {
  key: string
  label: string
  detail?: string | null
  status?: string | null
  icon_svg?: string | null
}

export interface AdminModelSnapshotItem {
  alias: string
  label: string
  latest_snapshot?: string | null
  versions: string[]
}

export interface AdminModelOfficialInfo {
  title: string
  display_name?: string | null
  tagline?: string | null
  family?: string | null
  family_label?: string | null
  description?: string | null
  avatar_url?: string | null
  deprecated?: boolean | null
  context_window_tokens?: number | null
  max_input_tokens?: number | null
  max_output_tokens?: number | null
  knowledge_cutoff?: string | null
  reasoning_token_support?: boolean | null
  input_price_microcredits?: number | null
  cached_input_price_microcredits?: number | null
  output_price_microcredits?: number | null
  pricing_notes?: string | null
  pricing_note_items: string[]
  input_modalities: string[]
  output_modalities: string[]
  endpoints: string[]
  supported_features: string[]
  supported_tools: string[]
  snapshots: string[]
  modality_items: AdminModelSectionItem[]
  endpoint_items: AdminModelSectionItem[]
  feature_items: AdminModelSectionItem[]
  tool_items: AdminModelSectionItem[]
  snapshot_items: AdminModelSnapshotItem[]
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
  listModels: async (): Promise<ListModelsResponse> => {
    const response = await apiClient.get<ListModelsResponse>('/admin/models', {
      timeout: 30000,
    })
    return response.data
  },
  syncOpenAiCatalog: async (): Promise<OpenAiModelsSyncResponse> => {
    const response = await apiClient.post<OpenAiModelsSyncResponse>('/admin/models/sync-openai', {})
    return response.data
  },
  probeModels: async (payload: ProbeModelsRequest = {}): Promise<ListModelsResponse> => {
    const response = await apiClient.post<ListModelsResponse>('/admin/models/probe', payload, {
      timeout: 120000,
    })
    return response.data
  },
  listModelPricing: async (): Promise<ModelPricingItem[]> => {
    const response = await apiClient.get<ModelPricingItem[]>('/admin/model-pricing')
    return response.data
  },
  upsertModelPricing: async (payload: ModelPricingUpsertRequest): Promise<ModelPricingItem> => {
    const response = await apiClient.post<ModelPricingItem>('/admin/model-pricing', payload)
    return response.data
  },
  deleteModelPricing: async (pricingId: string): Promise<void> => {
    await apiClient.delete<void>(`/admin/model-pricing/${pricingId}`)
  },
}
