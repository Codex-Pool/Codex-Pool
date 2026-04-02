import { apiClient } from "./client";

export type UpstreamMode =
  | "open_ai_api_key"
  | "chat_gpt_session"
  | "codex_oauth";
export type UpstreamAuthProvider = "legacy_bearer" | "oauth_refresh_token";
export type ModelRoutingTriggerMode =
  | "hybrid"
  | "scheduled_only"
  | "event_only";
export type ClaudeCodeEffortFallbackMode = "clamp_down" | "omit";

export interface RoutingProfileSelector {
  plan_types: string[];
  modes: UpstreamMode[];
  auth_providers: UpstreamAuthProvider[];
  include_account_ids: string[];
  exclude_account_ids: string[];
}

export interface RoutingProfile {
  id: string;
  name: string;
  description?: string | null;
  enabled: boolean;
  priority: number;
  selector: RoutingProfileSelector;
  created_at: string;
  updated_at: string;
}

export interface ModelRoutingPolicy {
  id: string;
  name: string;
  family: string;
  exact_models: string[];
  model_prefixes: string[];
  fallback_profile_ids: string[];
  enabled: boolean;
  priority: number;
  created_at: string;
  updated_at: string;
}

export interface CompiledRoutingProfile {
  id: string;
  name: string;
  account_ids: string[];
}

export interface CompiledModelRoutingPolicy {
  id: string;
  name: string;
  family: string;
  exact_models: string[];
  model_prefixes: string[];
  fallback_segments: CompiledRoutingProfile[];
}

export interface CompiledRoutingPlan {
  version_id: string;
  published_at: string;
  trigger_reason?: string | null;
  default_route: CompiledRoutingProfile[];
  policies: CompiledModelRoutingPolicy[];
}

export interface ModelRoutingSettings {
  enabled: boolean;
  auto_publish: boolean;
  planner_model_chain: string[];
  trigger_mode: ModelRoutingTriggerMode;
  kill_switch: boolean;
  updated_at: string;
}

export interface ClaudeCodeRoutingSettings {
  enabled: boolean;
  opus_target_model: string | null;
  sonnet_target_model: string | null;
  haiku_target_model: string | null;
  effort_routing: ClaudeCodeEffortRoutingSettings;
  updated_at: string;
}

export interface ClaudeCodeFamilyEffortRouting {
  source_to_target: Record<string, string | null>;
  default_target_effort: string | null;
}

export interface ClaudeCodeEffortRoutingSettings {
  fallback_mode: ClaudeCodeEffortFallbackMode;
  opus: ClaudeCodeFamilyEffortRouting;
  sonnet: ClaudeCodeFamilyEffortRouting;
  haiku: ClaudeCodeFamilyEffortRouting;
}

export interface RoutingPlanVersion {
  id: string;
  reason?: string | null;
  published_at: string;
  compiled_plan: CompiledRoutingPlan;
}

export interface RoutingProfilesResponse {
  profiles?: RoutingProfile[];
}

export interface ModelRoutingPoliciesResponse {
  policies?: ModelRoutingPolicy[];
}

export interface ModelRoutingSettingsResponse {
  settings: ModelRoutingSettings;
}

export interface ClaudeCodeRoutingSettingsResponse {
  settings: ClaudeCodeRoutingSettings;
}

export interface RoutingPlanVersionsResponse {
  versions?: RoutingPlanVersion[];
}

export interface UpsertRoutingProfileRequest {
  id?: string;
  name: string;
  description?: string | null;
  enabled: boolean;
  priority: number;
  selector: RoutingProfileSelector;
}

export interface UpsertModelRoutingPolicyRequest {
  id?: string;
  name: string;
  family: string;
  exact_models: string[];
  model_prefixes: string[];
  fallback_profile_ids: string[];
  enabled: boolean;
  priority: number;
}

export interface UpdateModelRoutingSettingsRequest {
  enabled: boolean;
  auto_publish: boolean;
  planner_model_chain: string[];
  trigger_mode: ModelRoutingTriggerMode;
  kill_switch: boolean;
}

export interface UpdateClaudeCodeRoutingSettingsRequest {
  enabled: boolean;
  opus_target_model: string | null;
  sonnet_target_model: string | null;
  haiku_target_model: string | null;
  effort_routing?: ClaudeCodeEffortRoutingSettings;
}

function normalizeStringArray(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

function normalizeNullableString(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function normalizeNullableEffort(value: unknown): string | null {
  return typeof value === "string" && value.trim().length > 0
    ? value.trim().toLowerCase()
    : null;
}

function normalizeClaudeCodeFamilyEffortRouting(
  value?: Partial<ClaudeCodeFamilyEffortRouting> | null,
): ClaudeCodeFamilyEffortRouting {
  const sourceToTarget = Object.fromEntries(
    Object.entries(value?.source_to_target ?? {})
      .map(([source, target]) => [
        source.trim().toLowerCase(),
        normalizeNullableEffort(target),
      ] as [string, string | null])
      .filter(
        (entry): entry is [string, string | null] => entry[0].length > 0,
      )
      .sort(([left], [right]) => left.localeCompare(right)),
  );

  return {
    source_to_target: sourceToTarget,
    default_target_effort: normalizeNullableEffort(value?.default_target_effort),
  };
}

function normalizeClaudeCodeEffortRouting(
  value?: Partial<ClaudeCodeEffortRoutingSettings> | null,
): ClaudeCodeEffortRoutingSettings {
  return {
    fallback_mode: value?.fallback_mode === "omit" ? "omit" : "clamp_down",
    opus: normalizeClaudeCodeFamilyEffortRouting(value?.opus),
    sonnet: normalizeClaudeCodeFamilyEffortRouting(value?.sonnet),
    haiku: normalizeClaudeCodeFamilyEffortRouting(value?.haiku),
  };
}

function normalizeProfileSelector(
  selector?: Partial<RoutingProfileSelector> | null,
): RoutingProfileSelector {
  return {
    plan_types: normalizeStringArray(selector?.plan_types),
    modes: Array.isArray(selector?.modes) ? selector.modes : [],
    auth_providers: Array.isArray(selector?.auth_providers)
      ? selector.auth_providers
      : [],
    include_account_ids: normalizeStringArray(selector?.include_account_ids),
    exclude_account_ids: normalizeStringArray(selector?.exclude_account_ids),
  };
}

function normalizeProfile(profile: RoutingProfile): RoutingProfile {
  return {
    ...profile,
    selector: normalizeProfileSelector(profile.selector),
  };
}

function normalizePolicy(policy: ModelRoutingPolicy): ModelRoutingPolicy {
  return {
    ...policy,
    exact_models: normalizeStringArray(policy.exact_models),
    model_prefixes: normalizeStringArray(policy.model_prefixes),
    fallback_profile_ids: normalizeStringArray(policy.fallback_profile_ids),
  };
}

function normalizeCompiledProfile(
  profile: CompiledRoutingProfile,
): CompiledRoutingProfile {
  return {
    ...profile,
    account_ids: normalizeStringArray(profile.account_ids),
  };
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
  };
}

function normalizeCompiledPlan(plan: CompiledRoutingPlan): CompiledRoutingPlan {
  return {
    ...plan,
    default_route: Array.isArray(plan.default_route)
      ? plan.default_route.map(normalizeCompiledProfile)
      : [],
    policies: Array.isArray(plan.policies)
      ? plan.policies.map(normalizeCompiledPolicy)
      : [],
  };
}

function normalizeSettings(
  settings: ModelRoutingSettings,
): ModelRoutingSettings {
  return {
    ...settings,
    planner_model_chain: normalizeStringArray(settings.planner_model_chain),
  };
}

function normalizeClaudeCodeSettings(
  settings: ClaudeCodeRoutingSettings,
): ClaudeCodeRoutingSettings {
  return {
    ...settings,
    opus_target_model: normalizeNullableString(settings.opus_target_model),
    sonnet_target_model: normalizeNullableString(settings.sonnet_target_model),
    haiku_target_model: normalizeNullableString(settings.haiku_target_model),
    effort_routing: normalizeClaudeCodeEffortRouting(settings.effort_routing),
  };
}

function normalizeVersion(version: RoutingPlanVersion): RoutingPlanVersion {
  return {
    ...version,
    compiled_plan: normalizeCompiledPlan(version.compiled_plan),
  };
}

export const modelRoutingApi = {
  listProfiles: async () => {
    const response = await apiClient.get<RoutingProfilesResponse>(
      "/admin/model-routing/profiles",
    );
    return {
      profiles: Array.isArray(response.data.profiles)
        ? response.data.profiles.map(normalizeProfile)
        : [],
    };
  },
  upsertProfile: async (payload: UpsertRoutingProfileRequest) => {
    const response = await apiClient.post<RoutingProfile>(
      "/admin/model-routing/profiles",
      payload,
    );
    return response.data;
  },
  deleteProfile: (profileId: string) =>
    apiClient.delete<void>(`/admin/model-routing/profiles/${profileId}`),
  listPolicies: async () => {
    const response = await apiClient.get<ModelRoutingPoliciesResponse>(
      "/admin/model-routing/model-policies",
    );
    return {
      policies: Array.isArray(response.data.policies)
        ? response.data.policies.map(normalizePolicy)
        : [],
    };
  },
  upsertPolicy: async (payload: UpsertModelRoutingPolicyRequest) => {
    const response = await apiClient.post<ModelRoutingPolicy>(
      "/admin/model-routing/model-policies",
      payload,
    );
    return response.data;
  },
  deletePolicy: (policyId: string) =>
    apiClient.delete<void>(`/admin/model-routing/model-policies/${policyId}`),
  getSettings: async () => {
    const response = await apiClient.get<ModelRoutingSettingsResponse>(
      "/admin/model-routing/settings",
    );
    return {
      settings: normalizeSettings(response.data.settings),
    };
  },
  updateSettings: async (payload: UpdateModelRoutingSettingsRequest) => {
    const response = await apiClient.put<ModelRoutingSettingsResponse>(
      "/admin/model-routing/settings",
      payload,
    );
    return response.data;
  },
  getClaudeCodeSettings: async () => {
    const response = await apiClient.get<ClaudeCodeRoutingSettingsResponse>(
      "/admin/model-routing/claude-code",
    );
    return {
      settings: normalizeClaudeCodeSettings(response.data.settings),
    };
  },
  updateClaudeCodeSettings: async (
    payload: UpdateClaudeCodeRoutingSettingsRequest,
  ) => {
    const response = await apiClient.put<ClaudeCodeRoutingSettingsResponse>(
      "/admin/model-routing/claude-code",
      payload,
    );
    return {
      settings: normalizeClaudeCodeSettings(response.data.settings),
    };
  },
  listVersions: async () => {
    const response = await apiClient.get<RoutingPlanVersionsResponse>(
      "/admin/model-routing/versions",
    );
    return {
      versions: Array.isArray(response.data.versions)
        ? response.data.versions.map(normalizeVersion)
        : [],
    };
  },
};
