import { apiClient } from './client'

export type UpstreamMode = 'open_ai_api_key' | 'chat_gpt_session' | 'codex_oauth'
export type UpstreamAuthProvider = 'legacy_bearer' | 'oauth_refresh_token'
export type AiRoutingTriggerMode = 'hybrid' | 'scheduled_only' | 'event_only'

export interface RoutingProfileSelector {
  plan_types: string[]
  modes: UpstreamMode[]
  auth_providers: UpstreamAuthProvider[]
  include_account_ids: string[]
  exclude_account_ids: string[]
}

export interface RoutingProfile {
  id: string
  name: string
  description?: string | null
  enabled: boolean
  priority: number
  selector: RoutingProfileSelector
  created_at: string
  updated_at: string
}

export interface ModelRoutingPolicy {
  id: string
  name: string
  family: string
  exact_models: string[]
  model_prefixes: string[]
  fallback_profile_ids: string[]
  enabled: boolean
  priority: number
  created_at: string
  updated_at: string
}

export interface CompiledRoutingProfile {
  id: string
  name: string
  account_ids: string[]
}

export interface CompiledModelRoutingPolicy {
  id: string
  name: string
  family: string
  exact_models: string[]
  model_prefixes: string[]
  fallback_segments: CompiledRoutingProfile[]
}

export interface CompiledRoutingPlan {
  version_id: string
  published_at: string
  trigger_reason?: string | null
  default_route: CompiledRoutingProfile[]
  policies: CompiledModelRoutingPolicy[]
}

export interface AiRoutingSettings {
  enabled: boolean
  auto_publish: boolean
  planner_model_chain: string[]
  trigger_mode: AiRoutingTriggerMode
  kill_switch: boolean
  updated_at: string
}

export interface RoutingPlanVersion {
  id: string
  reason?: string | null
  published_at: string
  compiled_plan: CompiledRoutingPlan
}

export interface RoutingProfilesResponse {
  profiles?: RoutingProfile[]
}

export interface ModelRoutingPoliciesResponse {
  policies?: ModelRoutingPolicy[]
}

export interface AiRoutingSettingsResponse {
  settings: AiRoutingSettings
}

export interface RoutingPlanVersionsResponse {
  versions?: RoutingPlanVersion[]
}

export interface UpsertRoutingProfileRequest {
  id?: string
  name: string
  description?: string | null
  enabled: boolean
  priority: number
  selector: RoutingProfileSelector
}

export interface UpsertModelRoutingPolicyRequest {
  id?: string
  name: string
  family: string
  exact_models: string[]
  model_prefixes: string[]
  fallback_profile_ids: string[]
  enabled: boolean
  priority: number
}

export interface UpdateAiRoutingSettingsRequest {
  enabled: boolean
  auto_publish: boolean
  planner_model_chain: string[]
  trigger_mode: AiRoutingTriggerMode
  kill_switch: boolean
}

export const aiRoutingApi = {
  listProfiles: () =>
    apiClient.get<RoutingProfilesResponse>('/admin/ai-routing/profiles'),
  upsertProfile: (payload: UpsertRoutingProfileRequest) =>
    apiClient.post<RoutingProfile>('/admin/ai-routing/profiles', payload),
  deleteProfile: (profileId: string) =>
    apiClient.delete<void>(`/admin/ai-routing/profiles/${profileId}`),
  listPolicies: () =>
    apiClient.get<ModelRoutingPoliciesResponse>('/admin/ai-routing/model-policies'),
  upsertPolicy: (payload: UpsertModelRoutingPolicyRequest) =>
    apiClient.post<ModelRoutingPolicy>('/admin/ai-routing/model-policies', payload),
  deletePolicy: (policyId: string) =>
    apiClient.delete<void>(`/admin/ai-routing/model-policies/${policyId}`),
  getSettings: () =>
    apiClient.get<AiRoutingSettingsResponse>('/admin/ai-routing/settings'),
  updateSettings: (payload: UpdateAiRoutingSettingsRequest) =>
    apiClient.put<AiRoutingSettingsResponse>('/admin/ai-routing/settings', payload),
  listVersions: () =>
    apiClient.get<RoutingPlanVersionsResponse>('/admin/ai-routing/versions'),
}
