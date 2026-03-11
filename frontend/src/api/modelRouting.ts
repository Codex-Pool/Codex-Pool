import { apiClient } from './client'

export type UpstreamMode = 'open_ai_api_key' | 'chat_gpt_session' | 'codex_oauth'
export type UpstreamAuthProvider = 'legacy_bearer' | 'oauth_refresh_token'
export type ModelRoutingTriggerMode = 'hybrid' | 'scheduled_only' | 'event_only'

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

export interface ModelRoutingSettings {
  enabled: boolean
  auto_publish: boolean
  planner_model_chain: string[]
  trigger_mode: ModelRoutingTriggerMode
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

export interface ModelRoutingSettingsResponse {
  settings: ModelRoutingSettings
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

export interface UpdateModelRoutingSettingsRequest {
  enabled: boolean
  auto_publish: boolean
  planner_model_chain: string[]
  trigger_mode: ModelRoutingTriggerMode
  kill_switch: boolean
}

function normalizeStringArray(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === 'string')
    : []
}

function normalizeProfileSelector(
  selector?: Partial<RoutingProfileSelector> | null,
): RoutingProfileSelector {
  return {
    plan_types: normalizeStringArray(selector?.plan_types),
    modes: Array.isArray(selector?.modes) ? selector.modes : [],
    auth_providers: Array.isArray(selector?.auth_providers) ? selector.auth_providers : [],
    include_account_ids: normalizeStringArray(selector?.include_account_ids),
    exclude_account_ids: normalizeStringArray(selector?.exclude_account_ids),
  }
}

function normalizeProfile(profile: RoutingProfile): RoutingProfile {
  return {
    ...profile,
    selector: normalizeProfileSelector(profile.selector),
  }
}

function normalizePolicy(policy: ModelRoutingPolicy): ModelRoutingPolicy {
  return {
    ...policy,
    exact_models: normalizeStringArray(policy.exact_models),
    model_prefixes: normalizeStringArray(policy.model_prefixes),
    fallback_profile_ids: normalizeStringArray(policy.fallback_profile_ids),
  }
}

function normalizeCompiledProfile(profile: CompiledRoutingProfile): CompiledRoutingProfile {
  return {
    ...profile,
    account_ids: normalizeStringArray(profile.account_ids),
  }
}

function normalizeCompiledPolicy(
  policy: CompiledModelRoutingPolicy,
): CompiledModelRoutingPolicy {
  return {
    ...policy,
    exact_models: normalizeStringArray(policy.exact_models),
    model_prefixes: normalizeStringArray(policy.model_prefixes),
    fallback_segments: Array.isArray(policy.fallback_segments)
      ? policy.fallback_segments.map(normalizeCompiledProfile)
      : [],
  }
}

function normalizeCompiledPlan(plan: CompiledRoutingPlan): CompiledRoutingPlan {
  return {
    ...plan,
    default_route: Array.isArray(plan.default_route)
      ? plan.default_route.map(normalizeCompiledProfile)
      : [],
    policies: Array.isArray(plan.policies) ? plan.policies.map(normalizeCompiledPolicy) : [],
  }
}

function normalizeSettings(settings: ModelRoutingSettings): ModelRoutingSettings {
  return {
    ...settings,
    planner_model_chain: normalizeStringArray(settings.planner_model_chain),
  }
}

function normalizeVersion(version: RoutingPlanVersion): RoutingPlanVersion {
  return {
    ...version,
    compiled_plan: normalizeCompiledPlan(version.compiled_plan),
  }
}

export const modelRoutingApi = {
  listProfiles: async () => {
    const response = await apiClient.get<RoutingProfilesResponse>('/admin/model-routing/profiles')
    return {
      profiles: Array.isArray(response.profiles) ? response.profiles.map(normalizeProfile) : [],
    }
  },
  upsertProfile: (payload: UpsertRoutingProfileRequest) =>
    apiClient.post<RoutingProfile>('/admin/model-routing/profiles', payload),
  deleteProfile: (profileId: string) =>
    apiClient.delete<void>(`/admin/model-routing/profiles/${profileId}`),
  listPolicies: async () => {
    const response = await apiClient.get<ModelRoutingPoliciesResponse>(
      '/admin/model-routing/model-policies',
    )
    return {
      policies: Array.isArray(response.policies) ? response.policies.map(normalizePolicy) : [],
    }
  },
  upsertPolicy: (payload: UpsertModelRoutingPolicyRequest) =>
    apiClient.post<ModelRoutingPolicy>('/admin/model-routing/model-policies', payload),
  deletePolicy: (policyId: string) =>
    apiClient.delete<void>(`/admin/model-routing/model-policies/${policyId}`),
  getSettings: async () => {
    const response = await apiClient.get<ModelRoutingSettingsResponse>(
      '/admin/model-routing/settings',
    )
    return {
      settings: normalizeSettings(response.settings),
    }
  },
  updateSettings: (payload: UpdateModelRoutingSettingsRequest) =>
    apiClient.put<ModelRoutingSettingsResponse>('/admin/model-routing/settings', payload),
  listVersions: async () => {
    const response = await apiClient.get<RoutingPlanVersionsResponse>(
      '/admin/model-routing/versions',
    )
    return {
      versions: Array.isArray(response.versions) ? response.versions.map(normalizeVersion) : [],
    }
  },
}
