import {
  type HTMLAttributes,
  type ReactNode,
  useCallback,
  useMemo,
  useState,
} from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@heroui/react";
import {
  Ban,
  Check,
  ChevronDown,
  ChevronUp,
  RotateCw,
  Save,
  Sparkles,
  SquarePen,
  Trash2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  aiErrorLearningApi,
  type AiErrorLearningSettings,
  type BuiltinErrorTemplateKind,
  type BuiltinErrorTemplateRecord,
  type LocalizedErrorTemplates,
  type SupportedErrorTemplateLocale,
  type UpdateBuiltinErrorTemplateRequest,
  type UpdateUpstreamErrorTemplateRequest,
  type UpstreamErrorAction,
  type UpstreamErrorRetryScope,
  type UpstreamErrorTemplateRecord,
  type UpstreamErrorTemplateStatus,
} from "@/api/aiErrorLearning";
import { modelsApi, type ModelSchema } from "@/api/models";
import {
  modelRoutingApi,
  type ClaudeCodeEffortFallbackMode,
  type ClaudeCodeEffortRoutingSettings,
  type ClaudeCodeFamilyEffortRouting,
  type ClaudeCodeRoutingSettings,
  type ModelRoutingSettings,
  type ModelRoutingTriggerMode,
  type ModelRoutingPolicy,
  type RoutingProfile,
  type UpstreamAuthProvider,
  type UpstreamMode,
} from "@/api/modelRouting";
import { localizeApiErrorDisplay } from "@/api/errorI18n";
import { ModelSelector } from "@/components/model-routing/model-selector";
import { getPublishedVersionWindow } from "@/components/model-routing/model-selector-utils";
import {
  AntigravityDialogActions,
  AntigravityDialogBody,
  AntigravityDialogPanel,
  AntigravityDialogShell,
} from "@/components/layout/dialog-archetypes";
import {
  DockedPageIntro,
  PageContent,
  PagePanel,
  SectionHeader,
} from "@/components/layout/page-archetypes";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { LoadingOverlay } from "@/components/ui/loading-overlay";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  SurfaceCard as BaseSurfaceCard,
  SurfaceCardBody,
  SurfaceCode,
  SurfaceInset,
} from "@/components/ui/surface";
import { Textarea } from "@/components/ui/textarea";
import { notify } from "@/lib/notification";
import { cn } from "@/lib/utils";

type ProfileFormState = {
  id?: string;
  name: string;
  description: string;
  enabled: boolean;
  priority: string;
  planTypes: string;
  modes: UpstreamMode[];
  authProviders: UpstreamAuthProvider[];
  includeAccountIds: string;
  excludeAccountIds: string;
};

type PolicyFormState = {
  id?: string;
  name: string;
  family: string;
  exactModels: string[];
  modelPrefixes: string;
  fallbackProfileIds: string[];
  enabled: boolean;
  priority: string;
};

type SettingsFormState = {
  enabled: boolean;
  autoPublish: boolean;
  plannerModelChain: string[];
  triggerMode: ModelRoutingTriggerMode;
  killSwitch: boolean;
};

type ClaudeCodeRoutingFormState = {
  enabled: boolean;
  opusTargetModel: string | null;
  sonnetTargetModel: string | null;
  haikuTargetModel: string | null;
  effortRouting: ClaudeCodeEffortRoutingSettings;
};

type ClaudeCodeEffortFamilyKey = "opus" | "sonnet" | "haiku";

type ErrorLearningSettingsFormState = {
  enabled: boolean;
  firstSeenTimeoutMs: string;
  reviewHitThreshold: string;
};

type TemplateLocaleDraft = Record<SupportedErrorTemplateLocale, string>;

type UpstreamErrorTemplateDraft = {
  semanticErrorCode: string;
  action: UpstreamErrorAction;
  retryScope: UpstreamErrorRetryScope;
  templates: TemplateLocaleDraft;
};

type BuiltinErrorTemplateDraft = {
  templates: TemplateLocaleDraft;
};

type UpdateTemplateMutationPayload = {
  templateId: string;
  payload: UpdateUpstreamErrorTemplateRequest;
};

type UpdateBuiltinTemplateMutationPayload = {
  kind: BuiltinErrorTemplateKind;
  code: string;
  payload: UpdateBuiltinErrorTemplateRequest;
};

const DEFAULT_PROFILE_FORM: ProfileFormState = {
  name: "",
  description: "",
  enabled: true,
  priority: "100",
  planTypes: "",
  modes: [],
  authProviders: [],
  includeAccountIds: "",
  excludeAccountIds: "",
};

const DEFAULT_POLICY_FORM: PolicyFormState = {
  name: "",
  family: "",
  exactModels: [],
  modelPrefixes: "",
  fallbackProfileIds: [],
  enabled: true,
  priority: "100",
};

const ERROR_TEMPLATE_LOCALES: SupportedErrorTemplateLocale[] = ["en", "zh-CN"];

const ERROR_TEMPLATE_ACTIONS: UpstreamErrorAction[] = [
  "return_failure",
  "retry_same_account",
  "retry_cross_account",
];

const ERROR_TEMPLATE_RETRY_SCOPES: UpstreamErrorRetryScope[] = [
  "none",
  "same_account",
  "cross_account",
];

const CLAUDE_CODE_TARGET_EFFORTS = ["low", "medium", "high", "xhigh"] as const;
const CLAUDE_CODE_EFFORT_OPTIONS = [null, ...CLAUDE_CODE_TARGET_EFFORTS] as const;
const CLAUDE_CODE_KNOWN_SOURCE_EFFORTS = {
  opus: ["low", "medium", "high", "max"],
  sonnet: ["low", "medium", "high"],
  haiku: [],
} as const;

function createRecommendedClaudeCodeEffortRouting(): ClaudeCodeEffortRoutingSettings {
  return {
    fallback_mode: "clamp_down",
    opus: {
      source_to_target: {
        low: "low",
        medium: "medium",
        high: "high",
        max: "xhigh",
      },
      default_target_effort: "xhigh",
    },
    sonnet: {
      source_to_target: {
        low: "low",
        medium: "medium",
        high: "high",
      },
      default_target_effort: "high",
    },
    haiku: {
      source_to_target: {},
      default_target_effort: null,
    },
  };
}

function createRecommendedClaudeCodeRoutingDraft(
  enabled: boolean,
): ClaudeCodeRoutingFormState {
  return {
    enabled,
    opusTargetModel: "gpt-5.4",
    sonnetTargetModel: "gpt-5.4-mini",
    haikuTargetModel: "gpt-5.4-mini",
    effortRouting: createRecommendedClaudeCodeEffortRouting(),
  };
}

function parseCsvInput(raw: string): string[] {
  return raw
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function toggleItem<T extends string>(items: T[], target: T): T[] {
  return items.includes(target)
    ? items.filter((item) => item !== target)
    : [...items, target];
}

function formatDateTime(value?: string | null) {
  if (!value) return "-";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString();
}

function parseIntegerInput(raw: string, fallback: number) {
  const value = Number(raw);
  return Number.isFinite(value) && value >= 0 ? Math.trunc(value) : fallback;
}

function selectorFilterSummary(
  profile: RoutingProfile,
  t: ReturnType<typeof useTranslation>["t"],
) {
  return t("modelRoutingPage.profiles.summary", {
    plans: profile.selector.plan_types.length,
    modes: profile.selector.modes.length,
    authProviders: profile.selector.auth_providers.length,
    include: profile.selector.include_account_ids.length,
    exclude: profile.selector.exclude_account_ids.length,
  });
}

function triggerModeLabel(
  mode: ModelRoutingTriggerMode,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (mode === "scheduled_only")
    return t("modelRoutingPage.triggerModes.scheduledOnly");
  if (mode === "event_only")
    return t("modelRoutingPage.triggerModes.eventOnly");
  return t("modelRoutingPage.triggerModes.hybrid");
}

function modeLabel(
  mode: UpstreamMode,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (mode === "open_ai_api_key") return t("modelRoutingPage.modes.apiKey");
  if (mode === "chat_gpt_session")
    return t("modelRoutingPage.modes.chatGptSession");
  return t("modelRoutingPage.modes.codexOauth");
}

function authProviderLabel(
  provider: UpstreamAuthProvider,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (provider === "oauth_refresh_token") {
    return t("modelRoutingPage.authProviders.oauthRefreshToken");
  }
  return t("modelRoutingPage.authProviders.legacyBearer");
}

function settingsStatusVariant(
  settings: ModelRoutingSettings | null,
): "success" | "warning" | "secondary" | "destructive" {
  if (!settings) return "secondary";
  if (settings.kill_switch) return "destructive";
  if (!settings.enabled) return "secondary";
  if (!settings.auto_publish) return "warning";
  return "success";
}

function fallbackProfileSummary(
  policy: ModelRoutingPolicy,
  profileNames: Map<string, string>,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (policy.fallback_profile_ids.length === 0) {
    return t("modelRoutingPage.common.none");
  }
  return policy.fallback_profile_ids
    .map(
      (profileId) =>
        profileNames.get(profileId) ??
        t("modelRoutingPage.common.deletedProfile"),
    )
    .join(" -> ");
}

function createSettingsDraft(
  settings: ModelRoutingSettings,
): SettingsFormState {
  return {
    enabled: settings.enabled,
    autoPublish: settings.auto_publish,
    plannerModelChain: settings.planner_model_chain ?? [],
    triggerMode: settings.trigger_mode,
    killSwitch: settings.kill_switch,
  };
}

function createClaudeCodeRoutingDraft(
  settings: ClaudeCodeRoutingSettings,
): ClaudeCodeRoutingFormState {
  return {
    enabled: settings.enabled,
    opusTargetModel: settings.opus_target_model,
    sonnetTargetModel: settings.sonnet_target_model,
    haikuTargetModel: settings.haiku_target_model,
    effortRouting: {
      fallback_mode: settings.effort_routing.fallback_mode,
      opus: cloneClaudeCodeFamilyEffortRouting(settings.effort_routing.opus),
      sonnet: cloneClaudeCodeFamilyEffortRouting(settings.effort_routing.sonnet),
      haiku: cloneClaudeCodeFamilyEffortRouting(settings.effort_routing.haiku),
    },
  };
}

function claudeCodeDraftEquals(
  draft: ClaudeCodeRoutingFormState,
  settings: ClaudeCodeRoutingSettings,
) {
  return (
    draft.enabled === settings.enabled &&
    draft.opusTargetModel === settings.opus_target_model &&
    draft.sonnetTargetModel === settings.sonnet_target_model &&
    draft.haikuTargetModel === settings.haiku_target_model &&
    claudeCodeEffortRoutingEquals(draft.effortRouting, settings.effort_routing)
  );
}

function cloneClaudeCodeFamilyEffortRouting(
  routing: ClaudeCodeFamilyEffortRouting,
): ClaudeCodeFamilyEffortRouting {
  return {
    source_to_target: Object.fromEntries(
      Object.entries(routing.source_to_target ?? {}).sort(([left], [right]) =>
        left.localeCompare(right),
      ),
    ),
    default_target_effort: routing.default_target_effort ?? null,
  };
}

function normalizeClaudeCodeFamilyEffortRouting(
  routing: ClaudeCodeFamilyEffortRouting,
): ClaudeCodeFamilyEffortRouting {
  return {
    source_to_target: Object.fromEntries(
      Object.entries(routing.source_to_target ?? {})
        .map(([source, target]) => [
          source.trim().toLowerCase(),
          typeof target === "string" && target.trim().length > 0
            ? target.trim().toLowerCase()
            : null,
        ] as [string, string | null])
        .filter(
          (entry): entry is [string, string | null] => entry[0].length > 0,
        )
        .sort(([left], [right]) => left.localeCompare(right)),
    ),
    default_target_effort:
      typeof routing.default_target_effort === "string" &&
      routing.default_target_effort.trim().length > 0
        ? routing.default_target_effort.trim().toLowerCase()
        : null,
  };
}

function normalizeClaudeCodeEffortRouting(
  routing: ClaudeCodeEffortRoutingSettings,
): ClaudeCodeEffortRoutingSettings {
  return {
    fallback_mode: routing.fallback_mode === "omit" ? "omit" : "clamp_down",
    opus: normalizeClaudeCodeFamilyEffortRouting(routing.opus),
    sonnet: normalizeClaudeCodeFamilyEffortRouting(routing.sonnet),
    haiku: normalizeClaudeCodeFamilyEffortRouting(routing.haiku),
  };
}

function claudeCodeEffortRoutingEquals(
  left: ClaudeCodeEffortRoutingSettings,
  right: ClaudeCodeEffortRoutingSettings,
) {
  const normalizedLeft = normalizeClaudeCodeEffortRouting(left);
  const normalizedRight = normalizeClaudeCodeEffortRouting(right);
  return JSON.stringify(normalizedLeft) === JSON.stringify(normalizedRight);
}

function createErrorLearningSettingsDraft(
  settings: AiErrorLearningSettings,
): ErrorLearningSettingsFormState {
  return {
    enabled: settings.enabled,
    firstSeenTimeoutMs: String(settings.first_seen_timeout_ms),
    reviewHitThreshold: String(settings.review_hit_threshold),
  };
}

function createUpstreamErrorTemplateDraft(
  template: UpstreamErrorTemplateRecord,
): UpstreamErrorTemplateDraft {
  return {
    semanticErrorCode: template.semantic_error_code,
    action: template.action,
    retryScope: template.retry_scope,
    templates: {
      en: template.templates.en ?? "",
      "zh-CN": template.templates["zh-CN"] ?? "",
    },
  };
}

function createBuiltinErrorTemplateDraft(
  template: BuiltinErrorTemplateRecord,
): BuiltinErrorTemplateDraft {
  return {
    templates: {
      en: template.templates.en ?? "",
      "zh-CN": template.templates["zh-CN"] ?? "",
    },
  };
}

function createLocalizedErrorTemplatesPayload(
  templates: TemplateLocaleDraft,
): LocalizedErrorTemplates {
  return {
    en: templates.en.trim() || null,
    "zh-CN": templates["zh-CN"].trim() || null,
  };
}

function selectSingleModel(next: string[]) {
  return next.length > 0 ? (next[next.length - 1] ?? null) : null;
}

function templateStatusVariant(
  status: UpstreamErrorTemplateStatus,
): "success" | "warning" | "secondary" | "destructive" | "info" {
  if (status === "approved") return "success";
  if (status === "review_pending") return "warning";
  if (status === "rejected") return "destructive";
  return "info";
}

function templateStatusLabel(
  status: UpstreamErrorTemplateStatus,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (status === "review_pending")
    return t("modelRoutingPage.errorLearning.statuses.reviewPending");
  if (status === "approved")
    return t("modelRoutingPage.errorLearning.statuses.approved");
  if (status === "rejected")
    return t("modelRoutingPage.errorLearning.statuses.rejected");
  return t("modelRoutingPage.errorLearning.statuses.provisionalLive");
}

function upstreamErrorActionLabel(
  action: UpstreamErrorAction,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (action === "retry_same_account") {
    return t("modelRoutingPage.errorLearning.actionValues.retrySameAccount");
  }
  if (action === "retry_cross_account") {
    return t("modelRoutingPage.errorLearning.actionValues.retryCrossAccount");
  }
  return t("modelRoutingPage.errorLearning.actionValues.returnFailure");
}

function upstreamErrorRetryScopeLabel(
  scope: UpstreamErrorRetryScope,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (scope === "same_account") {
    return t("modelRoutingPage.errorLearning.retryScopes.sameAccount");
  }
  if (scope === "cross_account") {
    return t("modelRoutingPage.errorLearning.retryScopes.crossAccount");
  }
  return t("modelRoutingPage.errorLearning.retryScopes.none");
}

function localeLabel(
  locale: SupportedErrorTemplateLocale,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (locale === "zh-CN")
    return t("modelRoutingPage.errorLearning.locales.zhCN");
  return t("modelRoutingPage.errorLearning.locales.en");
}

function builtinErrorTemplateKindLabel(
  kind: BuiltinErrorTemplateKind,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (kind === "gateway_error") {
    return t(
      "modelRoutingPage.errorLearning.builtinTemplates.kinds.gatewayError",
    );
  }
  return t(
    "modelRoutingPage.errorLearning.builtinTemplates.kinds.heuristicUpstream",
  );
}

function builtinErrorTemplateKey(template: BuiltinErrorTemplateRecord) {
  return `${template.kind}:${template.code}`;
}

function SurfaceCard({
  className,
  contentClassName,
  children,
}: {
  className?: string;
  contentClassName?: string;
  children: ReactNode;
}) {
  return (
    <BaseSurfaceCard tone="muted" shadow="none" className={className}>
      <SurfaceCardBody className={cn("space-y-3 p-4", contentClassName)}>
        {children}
      </SurfaceCardBody>
    </BaseSurfaceCard>
  );
}

function EmptyState({
  className,
  children,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return (
    <SurfaceInset
      className={cn("px-4 py-4 text-sm text-muted-foreground", className)}
      {...props}
    >
      {children}
    </SurfaceInset>
  );
}

export default function ModelRouting() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [profileDialogOpen, setProfileDialogOpen] = useState(false);
  const [policyDialogOpen, setPolicyDialogOpen] = useState(false);
  const [profileForm, setProfileForm] =
    useState<ProfileFormState>(DEFAULT_PROFILE_FORM);
  const [policyForm, setPolicyForm] =
    useState<PolicyFormState>(DEFAULT_POLICY_FORM);
  const [settingsDraftOverride, setSettingsDraftOverride] =
    useState<SettingsFormState | null>(null);
  const [claudeCodeDraftOverride, setClaudeCodeDraftOverride] =
    useState<ClaudeCodeRoutingFormState | null>(null);
  const [errorLearningDraftOverride, setErrorLearningDraftOverride] =
    useState<ErrorLearningSettingsFormState | null>(null);
  const [editingTemplateId, setEditingTemplateId] = useState<string | null>(
    null,
  );
  const [editingTemplateDraft, setEditingTemplateDraft] =
    useState<UpstreamErrorTemplateDraft | null>(null);
  const [editingBuiltinTemplateKey, setEditingBuiltinTemplateKey] = useState<
    string | null
  >(null);
  const [editingBuiltinTemplateDraft, setEditingBuiltinTemplateDraft] =
    useState<BuiltinErrorTemplateDraft | null>(null);
  const [versionsExpanded, setVersionsExpanded] = useState(false);
  const [builtinTemplatesExpanded, setBuiltinTemplatesExpanded] =
    useState(false);

  const resolveErrorLabel = useCallback(
    (err: unknown, fallback: string) =>
      localizeApiErrorDisplay(t, err, fallback).label,
    [t],
  );

  const notifySuccess = useCallback((title: string) => {
    notify({
      variant: "success",
      title,
    });
  }, []);

  const notifyError = useCallback(
    (error: unknown, fallback: string) => {
      const description = resolveErrorLabel(error, fallback);
      notify({
        variant: "error",
        title: fallback,
        description: description !== fallback ? description : undefined,
      });
    },
    [resolveErrorLabel],
  );

  const { data, isLoading, isFetching } = useQuery({
    queryKey: ["adminModelRouting"],
    queryFn: async () => {
      const [
        profilesPayload,
        policiesPayload,
        settingsPayload,
        claudeCodeSettingsPayload,
        versionsPayload,
        errorLearningSettingsPayload,
        upstreamErrorTemplatesPayload,
        builtinErrorTemplatesPayload,
      ] = await Promise.all([
        modelRoutingApi.listProfiles(),
        modelRoutingApi.listPolicies(),
        modelRoutingApi.getSettings(),
        modelRoutingApi.getClaudeCodeSettings(),
        modelRoutingApi.listVersions(),
        aiErrorLearningApi.getSettings(),
        aiErrorLearningApi.listTemplates(),
        aiErrorLearningApi.listBuiltinTemplates(),
      ]);
      return {
        profiles: profilesPayload.profiles ?? [],
        policies: policiesPayload.policies ?? [],
        settings: settingsPayload.settings,
        claudeCodeSettings: claudeCodeSettingsPayload.settings,
        versions: versionsPayload.versions ?? [],
        errorLearningSettings: errorLearningSettingsPayload.settings,
        upstreamErrorTemplates: upstreamErrorTemplatesPayload.templates ?? [],
        builtinErrorTemplates: builtinErrorTemplatesPayload.templates ?? [],
      };
    },
    staleTime: 30_000,
  });
  const { data: modelsPayload } = useQuery({
    queryKey: ["models"],
    queryFn: modelsApi.listModels,
    staleTime: 60_000,
  });

  const profiles = useMemo(() => data?.profiles ?? [], [data?.profiles]);
  const policies = useMemo(() => data?.policies ?? [], [data?.policies]);
  const settings = data?.settings ?? null;
  const claudeCodeSettings = data?.claudeCodeSettings ?? null;
  const versions = useMemo(() => data?.versions ?? [], [data?.versions]);
  const availableModels = useMemo<ModelSchema[]>(
    () => modelsPayload?.data ?? [],
    [modelsPayload?.data],
  );
  const errorLearningSettings = data?.errorLearningSettings ?? null;
  const upstreamErrorTemplates = useMemo(() => {
    const items = [...(data?.upstreamErrorTemplates ?? [])];
    const statusRank: Record<UpstreamErrorTemplateStatus, number> = {
      review_pending: 0,
      provisional_live: 1,
      approved: 2,
      rejected: 3,
    };
    items.sort((left, right) => {
      if (statusRank[left.status] !== statusRank[right.status]) {
        return statusRank[left.status] - statusRank[right.status];
      }
      if (right.hit_count !== left.hit_count) {
        return right.hit_count - left.hit_count;
      }
      return (
        new Date(right.last_seen_at).getTime() -
        new Date(left.last_seen_at).getTime()
      );
    });
    return items;
  }, [data?.upstreamErrorTemplates]);
  const builtinErrorTemplates = useMemo(() => {
    const items = [...(data?.builtinErrorTemplates ?? [])];
    items.sort((left, right) => {
      if (left.kind !== right.kind) {
        return left.kind.localeCompare(right.kind);
      }
      if (left.is_overridden !== right.is_overridden) {
        return left.is_overridden ? -1 : 1;
      }
      return left.code.localeCompare(right.code);
    });
    return items;
  }, [data?.builtinErrorTemplates]);
  const settingsDraft = useMemo(
    () =>
      settingsDraftOverride ??
      (settings ? createSettingsDraft(settings) : null),
    [settingsDraftOverride, settings],
  );
  const claudeCodeDraft = useMemo(
    () =>
      claudeCodeDraftOverride ??
      (claudeCodeSettings
        ? createClaudeCodeRoutingDraft(claudeCodeSettings)
        : null),
    [claudeCodeDraftOverride, claudeCodeSettings],
  );
  const errorLearningSettingsDraft = useMemo(
    () =>
      errorLearningDraftOverride ??
      (errorLearningSettings
        ? createErrorLearningSettingsDraft(errorLearningSettings)
        : null),
    [errorLearningDraftOverride, errorLearningSettings],
  );
  const visibleVersions = useMemo(
    () => getPublishedVersionWindow(versions, versionsExpanded, 5),
    [versions, versionsExpanded],
  );
  const visibleBuiltinTemplates = useMemo(
    () =>
      getPublishedVersionWindow(
        builtinErrorTemplates,
        builtinTemplatesExpanded,
        5,
      ),
    [builtinErrorTemplates, builtinTemplatesExpanded],
  );
  const modelSelectorLabels = useMemo(
    () => ({
      addModel: t("modelRoutingPage.modelSelector.addModel"),
      searchPlaceholder: t("modelRoutingPage.modelSelector.searchPlaceholder"),
      emptyCatalog: t("modelRoutingPage.modelSelector.emptyCatalog"),
      emptySelection: t("modelRoutingPage.modelSelector.emptySelection"),
      noMatches: t("modelRoutingPage.modelSelector.noMatches"),
      unknownModel: t("modelRoutingPage.modelSelector.unknownModel"),
      moveUp: t("modelRoutingPage.modelSelector.moveUp"),
      moveDown: t("modelRoutingPage.modelSelector.moveDown"),
      remove: t("modelRoutingPage.modelSelector.remove"),
      available: t("models.availability.available"),
      unavailable: t("models.availability.unavailable"),
      unknown: t("models.availability.unknown"),
    }),
    [t],
  );
  const claudeCodeModelSelectorLabels = useMemo(
    () => ({
      ...modelSelectorLabels,
      emptySelection: t("modelRoutingPage.claudeCode.emptySelection"),
    }),
    [modelSelectorLabels, t],
  );
  const claudeCodeFamilies = useMemo(
    () => [
      {
        key: "opusTargetModel" as const,
        effortKey: "opus" as const,
        label: t("modelRoutingPage.claudeCode.families.opus"),
        sourceEfforts: [...CLAUDE_CODE_KNOWN_SOURCE_EFFORTS.opus],
      },
      {
        key: "sonnetTargetModel" as const,
        effortKey: "sonnet" as const,
        label: t("modelRoutingPage.claudeCode.families.sonnet"),
        sourceEfforts: [...CLAUDE_CODE_KNOWN_SOURCE_EFFORTS.sonnet],
      },
      {
        key: "haikuTargetModel" as const,
        effortKey: "haiku" as const,
        label: t("modelRoutingPage.claudeCode.families.haiku"),
        sourceEfforts: [...CLAUDE_CODE_KNOWN_SOURCE_EFFORTS.haiku],
      },
    ],
    [t],
  );
  const claudeCodeFallbackModeOptions = useMemo(
    () => [
      {
        value: "clamp_down" as ClaudeCodeEffortFallbackMode,
        label: t("modelRoutingPage.claudeCode.fallbackModes.clampDown"),
      },
      {
        value: "omit" as ClaudeCodeEffortFallbackMode,
        label: t("modelRoutingPage.claudeCode.fallbackModes.omit"),
      },
    ],
    [t],
  );
  const claudeCodeTargetEffortOptions = useMemo(
    () =>
      CLAUDE_CODE_EFFORT_OPTIONS.map((value) => ({
        value,
        label: value
          ? t(`modelRoutingPage.claudeCode.effortLabels.${value}`)
          : t("modelRoutingPage.claudeCode.effortLabels.omit"),
      })),
    [t],
  );
  const claudeCodeHasChanges = useMemo(
    () =>
      claudeCodeDraft !== null &&
      claudeCodeSettings !== null &&
      !claudeCodeDraftEquals(claudeCodeDraft, claudeCodeSettings),
    [claudeCodeDraft, claudeCodeSettings],
  );

  const updateSettingsDraft = useCallback(
    (updater: (current: SettingsFormState) => SettingsFormState) => {
      const base =
        settingsDraft ?? (settings ? createSettingsDraft(settings) : null);
      if (!base) {
        return;
      }
      setSettingsDraftOverride(updater(base));
    },
    [settings, settingsDraft],
  );

  const updateClaudeCodeDraft = useCallback(
    (
      updater: (
        current: ClaudeCodeRoutingFormState,
      ) => ClaudeCodeRoutingFormState,
    ) => {
      const base =
        claudeCodeDraft ??
        (claudeCodeSettings
          ? createClaudeCodeRoutingDraft(claudeCodeSettings)
          : null);
      if (!base) {
        return;
      }
      setClaudeCodeDraftOverride(updater(base));
    },
    [claudeCodeDraft, claudeCodeSettings],
  );

  const updateClaudeCodeFallbackMode = useCallback(
    (fallbackMode: ClaudeCodeEffortFallbackMode) => {
      updateClaudeCodeDraft((prev) => ({
        ...prev,
        effortRouting: {
          ...prev.effortRouting,
          fallback_mode: fallbackMode,
        },
      }));
    },
    [updateClaudeCodeDraft],
  );

  const updateClaudeCodeFamilyDefaultEffort = useCallback(
    (family: ClaudeCodeEffortFamilyKey, nextEffort: string | null) => {
      updateClaudeCodeDraft((prev) => ({
        ...prev,
        effortRouting: {
          ...prev.effortRouting,
          [family]: {
            ...prev.effortRouting[family],
            default_target_effort: nextEffort,
          },
        },
      }));
    },
    [updateClaudeCodeDraft],
  );

  const updateClaudeCodeFamilyMappedEffort = useCallback(
    (
      family: ClaudeCodeEffortFamilyKey,
      sourceEffort: string,
      nextEffort: string | null,
    ) => {
      updateClaudeCodeDraft((prev) => ({
        ...prev,
        effortRouting: {
          ...prev.effortRouting,
          [family]: {
            ...prev.effortRouting[family],
            source_to_target: {
              ...prev.effortRouting[family].source_to_target,
              [sourceEffort]: nextEffort,
            },
          },
        },
      }));
    },
    [updateClaudeCodeDraft],
  );

  const updateErrorLearningDraft = useCallback(
    (
      updater: (
        current: ErrorLearningSettingsFormState,
      ) => ErrorLearningSettingsFormState,
    ) => {
      const base =
        errorLearningSettingsDraft ??
        (errorLearningSettings
          ? createErrorLearningSettingsDraft(errorLearningSettings)
          : null);
      if (!base) {
        return;
      }
      setErrorLearningDraftOverride(updater(base));
    },
    [errorLearningSettings, errorLearningSettingsDraft],
  );

  const profileNames = useMemo(() => {
    return new Map(profiles.map((profile) => [profile.id, profile.name]));
  }, [profiles]);

  const saveSettingsMutation = useMutation({
    mutationFn: async () => {
      if (!settingsDraft) {
        throw new Error("settings_missing");
      }
      return modelRoutingApi.updateSettings({
        enabled: settingsDraft.enabled,
        auto_publish: settingsDraft.autoPublish,
        planner_model_chain: settingsDraft.plannerModelChain,
        trigger_mode: settingsDraft.triggerMode,
        kill_switch: settingsDraft.killSwitch,
      });
    },
    onSuccess: () => {
      notifySuccess(t("modelRoutingPage.messages.settingsSaved"));
      setSettingsDraftOverride(null);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(err, t("modelRoutingPage.messages.settingsSaveFailed"));
    },
  });

  const saveClaudeCodeMutation = useMutation({
    mutationFn: async () => {
      if (!claudeCodeDraft) {
        throw new Error("claude_code_settings_missing");
      }
      return modelRoutingApi.updateClaudeCodeSettings({
        enabled: claudeCodeDraft.enabled,
        opus_target_model: claudeCodeDraft.opusTargetModel,
        sonnet_target_model: claudeCodeDraft.sonnetTargetModel,
        haiku_target_model: claudeCodeDraft.haikuTargetModel,
        effort_routing: normalizeClaudeCodeEffortRouting(
          claudeCodeDraft.effortRouting,
        ),
      });
    },
    onSuccess: () => {
      notifySuccess(t("modelRoutingPage.messages.claudeCodeSettingsSaved"));
      setClaudeCodeDraftOverride(null);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(
        err,
        t("modelRoutingPage.messages.claudeCodeSettingsSaveFailed"),
      );
    },
  });

  const saveErrorLearningSettingsMutation = useMutation({
    mutationFn: async () => {
      if (!errorLearningSettingsDraft || !errorLearningSettings) {
        throw new Error("error_learning_settings_missing");
      }
      return aiErrorLearningApi.updateSettings({
        enabled: errorLearningSettingsDraft.enabled,
        first_seen_timeout_ms: parseIntegerInput(
          errorLearningSettingsDraft.firstSeenTimeoutMs,
          errorLearningSettings.first_seen_timeout_ms,
        ),
        review_hit_threshold: parseIntegerInput(
          errorLearningSettingsDraft.reviewHitThreshold,
          errorLearningSettings.review_hit_threshold,
        ),
      });
    },
    onSuccess: () => {
      notifySuccess(t("modelRoutingPage.messages.errorLearningSettingsSaved"));
      setErrorLearningDraftOverride(null);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(
        err,
        t("modelRoutingPage.messages.errorLearningSettingsSaveFailed"),
      );
    },
  });

  const upsertProfileMutation = useMutation({
    mutationFn: () =>
      modelRoutingApi.upsertProfile({
        id: profileForm.id,
        name: profileForm.name.trim(),
        description: profileForm.description.trim() || undefined,
        enabled: profileForm.enabled,
        priority: Number(profileForm.priority) || 0,
        selector: {
          plan_types: parseCsvInput(profileForm.planTypes),
          modes: profileForm.modes,
          auth_providers: profileForm.authProviders,
          include_account_ids: parseCsvInput(profileForm.includeAccountIds),
          exclude_account_ids: parseCsvInput(profileForm.excludeAccountIds),
        },
      }),
    onSuccess: (profile) => {
      notifySuccess(
        t("modelRoutingPage.messages.profileSaved", { name: profile.name }),
      );
      setProfileDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(err, t("modelRoutingPage.messages.profileSaveFailed"));
    },
  });

  const deleteProfileMutation = useMutation({
    mutationFn: (profileId: string) => modelRoutingApi.deleteProfile(profileId),
    onSuccess: () => {
      notifySuccess(t("modelRoutingPage.messages.profileDeleted"));
      setProfileDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(err, t("modelRoutingPage.messages.profileDeleteFailed"));
    },
  });

  const upsertPolicyMutation = useMutation({
    mutationFn: () =>
      modelRoutingApi.upsertPolicy({
        id: policyForm.id,
        name: policyForm.name.trim(),
        family: policyForm.family.trim(),
        exact_models: policyForm.exactModels,
        model_prefixes: parseCsvInput(policyForm.modelPrefixes),
        fallback_profile_ids: policyForm.fallbackProfileIds,
        enabled: policyForm.enabled,
        priority: Number(policyForm.priority) || 0,
      }),
    onSuccess: (policy) => {
      notifySuccess(
        t("modelRoutingPage.messages.policySaved", { name: policy.name }),
      );
      setPolicyDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(err, t("modelRoutingPage.messages.policySaveFailed"));
    },
  });

  const deletePolicyMutation = useMutation({
    mutationFn: (policyId: string) => modelRoutingApi.deletePolicy(policyId),
    onSuccess: () => {
      notifySuccess(t("modelRoutingPage.messages.policyDeleted"));
      setPolicyDialogOpen(false);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(err, t("modelRoutingPage.messages.policyDeleteFailed"));
    },
  });

  const updateTemplateMutation = useMutation({
    mutationFn: ({ templateId, payload }: UpdateTemplateMutationPayload) =>
      aiErrorLearningApi.updateTemplate(templateId, payload),
    onSuccess: () => {
      notifySuccess(t("modelRoutingPage.messages.templateSaved"));
      setEditingTemplateId(null);
      setEditingTemplateDraft(null);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(err, t("modelRoutingPage.messages.templateSaveFailed"));
    },
  });

  const approveTemplateMutation = useMutation({
    mutationFn: (templateId: string) =>
      aiErrorLearningApi.approveTemplate(templateId),
    onSuccess: ({ template }) => {
      notifySuccess(t("modelRoutingPage.messages.templateApproved"));
      if (editingTemplateId === template.id) {
        setEditingTemplateId(null);
      }
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(err, t("modelRoutingPage.messages.templateApproveFailed"));
    },
  });

  const rejectTemplateMutation = useMutation({
    mutationFn: (templateId: string) =>
      aiErrorLearningApi.rejectTemplate(templateId),
    onSuccess: ({ template }) => {
      notifySuccess(t("modelRoutingPage.messages.templateRejected"));
      if (editingTemplateId === template.id) {
        setEditingTemplateId(null);
      }
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(err, t("modelRoutingPage.messages.templateRejectFailed"));
    },
  });

  const rewriteTemplateMutation = useMutation({
    mutationFn: (templateId: string) =>
      aiErrorLearningApi.rewriteTemplate(templateId),
    onSuccess: ({ template }) => {
      notifySuccess(t("modelRoutingPage.messages.templateRewritten"));
      setEditingTemplateId(template.id);
      setEditingTemplateDraft(createUpstreamErrorTemplateDraft(template));
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(err, t("modelRoutingPage.messages.templateRewriteFailed"));
    },
  });

  const updateBuiltinTemplateMutation = useMutation({
    mutationFn: ({
      kind,
      code,
      payload,
    }: UpdateBuiltinTemplateMutationPayload) =>
      aiErrorLearningApi.updateBuiltinTemplate(kind, code, payload),
    onSuccess: () => {
      notifySuccess(t("modelRoutingPage.messages.builtinTemplateSaved"));
      setEditingBuiltinTemplateKey(null);
      setEditingBuiltinTemplateDraft(null);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(
        err,
        t("modelRoutingPage.messages.builtinTemplateSaveFailed"),
      );
    },
  });

  const rewriteBuiltinTemplateMutation = useMutation({
    mutationFn: ({
      kind,
      code,
    }: {
      kind: BuiltinErrorTemplateKind;
      code: string;
    }) => aiErrorLearningApi.rewriteBuiltinTemplate(kind, code),
    onSuccess: ({ template }) => {
      notifySuccess(t("modelRoutingPage.messages.builtinTemplateRewritten"));
      setEditingBuiltinTemplateKey(builtinErrorTemplateKey(template));
      setEditingBuiltinTemplateDraft(createBuiltinErrorTemplateDraft(template));
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(
        err,
        t("modelRoutingPage.messages.builtinTemplateRewriteFailed"),
      );
    },
  });

  const resetBuiltinTemplateMutation = useMutation({
    mutationFn: ({
      kind,
      code,
    }: {
      kind: BuiltinErrorTemplateKind;
      code: string;
    }) => aiErrorLearningApi.resetBuiltinTemplate(kind, code),
    onSuccess: () => {
      notifySuccess(t("modelRoutingPage.messages.builtinTemplateReset"));
      setEditingBuiltinTemplateKey(null);
      setEditingBuiltinTemplateDraft(null);
      queryClient.invalidateQueries({ queryKey: ["adminModelRouting"] });
    },
    onError: (err) => {
      notifyError(
        err,
        t("modelRoutingPage.messages.builtinTemplateResetFailed"),
      );
    },
  });

  const openCreateProfileDialog = () => {
    setProfileForm(DEFAULT_PROFILE_FORM);
    setProfileDialogOpen(true);
  };

  const openEditProfileDialog = (profile: RoutingProfile) => {
    setProfileForm({
      id: profile.id,
      name: profile.name,
      description: profile.description ?? "",
      enabled: profile.enabled,
      priority: String(profile.priority),
      planTypes: profile.selector.plan_types.join(", "),
      modes: profile.selector.modes,
      authProviders: profile.selector.auth_providers,
      includeAccountIds: profile.selector.include_account_ids.join(", "),
      excludeAccountIds: profile.selector.exclude_account_ids.join(", "),
    });
    setProfileDialogOpen(true);
  };

  const openCreatePolicyDialog = () => {
    setPolicyForm(DEFAULT_POLICY_FORM);
    setPolicyDialogOpen(true);
  };

  const openEditPolicyDialog = (policy: ModelRoutingPolicy) => {
    setPolicyForm({
      id: policy.id,
      name: policy.name,
      family: policy.family,
      exactModels: policy.exact_models,
      modelPrefixes: policy.model_prefixes.join(", "),
      fallbackProfileIds: policy.fallback_profile_ids,
      enabled: policy.enabled,
      priority: String(policy.priority),
    });
    setPolicyDialogOpen(true);
  };

  const openTemplateEditor = (template: UpstreamErrorTemplateRecord) => {
    setEditingTemplateId(template.id);
    setEditingTemplateDraft(createUpstreamErrorTemplateDraft(template));
  };

  const cancelTemplateEditor = () => {
    setEditingTemplateId(null);
    setEditingTemplateDraft(null);
  };

  const updateTemplateDraft = useCallback(
    (
      updater: (
        current: UpstreamErrorTemplateDraft,
      ) => UpstreamErrorTemplateDraft,
    ) => {
      setEditingTemplateDraft((current) =>
        current ? updater(current) : current,
      );
    },
    [],
  );

  const openBuiltinTemplateEditor = (template: BuiltinErrorTemplateRecord) => {
    setEditingBuiltinTemplateKey(builtinErrorTemplateKey(template));
    setEditingBuiltinTemplateDraft(createBuiltinErrorTemplateDraft(template));
  };

  const cancelBuiltinTemplateEditor = () => {
    setEditingBuiltinTemplateKey(null);
    setEditingBuiltinTemplateDraft(null);
  };

  const updateBuiltinTemplateDraft = useCallback(
    (
      updater: (
        current: BuiltinErrorTemplateDraft,
      ) => BuiltinErrorTemplateDraft,
    ) => {
      setEditingBuiltinTemplateDraft((current) =>
        current ? updater(current) : current,
      );
    },
    [],
  );

  const modeOptions: UpstreamMode[] = [
    "codex_oauth",
    "chat_gpt_session",
    "open_ai_api_key",
  ];
  const authProviderOptions: UpstreamAuthProvider[] = [
    "oauth_refresh_token",
    "legacy_bearer",
  ];
  const triggerModeOptions: ModelRoutingTriggerMode[] = [
    "hybrid",
    "scheduled_only",
    "event_only",
  ];
  const anyTemplateMutationPending =
    updateTemplateMutation.isPending ||
    approveTemplateMutation.isPending ||
    rejectTemplateMutation.isPending ||
    rewriteTemplateMutation.isPending;
  const anyBuiltinTemplateMutationPending =
    updateBuiltinTemplateMutation.isPending ||
    rewriteBuiltinTemplateMutation.isPending ||
    resetBuiltinTemplateMutation.isPending;

  return (
    <PageContent className="overflow-y-auto">
      <LoadingOverlay show={isLoading} title={t("common.loading")} />

      <div className="space-y-4 md:space-y-5">
        <DockedPageIntro
          archetype="workspace"
          title={t("nav.modelRouting")}
          description={t("modelRoutingPage.subtitle")}
          actions={
            <>
              <Button
                onClick={() => {
                  queryClient.invalidateQueries({
                    queryKey: ["adminModelRouting"],
                  });
                  queryClient.invalidateQueries({ queryKey: ["models"] });
                }}
                disabled={isFetching}
              >
                <RotateCw
                  className={`mr-2 h-4 w-4 ${isFetching ? "animate-spin" : ""}`}
                />
                {t("modelRoutingPage.actions.refresh")}
              </Button>
              <Button variant="flat" onClick={openCreateProfileDialog}>
                {t("modelRoutingPage.actions.createProfile")}
              </Button>
              <Button color="primary" onClick={openCreatePolicyDialog}>
                {t("modelRoutingPage.actions.createPolicy")}
              </Button>
            </>
          }
        />

        <div className="grid gap-4 xl:grid-cols-[minmax(0,1.28fr)_minmax(18rem,0.72fr)]">
          <div className="space-y-4 md:space-y-5">
            <PagePanel className="relative overflow-hidden space-y-5">
              <SectionHeader
                title={t("modelRoutingPage.settings.title")}
                description={t("modelRoutingPage.settings.description")}
              />
              <div className="flex flex-wrap items-center gap-2">
                <Badge variant={settingsStatusVariant(settings)}>
                  {settings?.kill_switch
                    ? t("modelRoutingPage.status.killSwitchOn")
                    : settings?.enabled
                      ? t("modelRoutingPage.status.enabled")
                      : t("modelRoutingPage.status.disabled")}
                </Badge>
                <Badge variant={settings?.auto_publish ? "info" : "secondary"}>
                  {settings?.auto_publish
                    ? t("modelRoutingPage.status.autoPublishOn")
                    : t("modelRoutingPage.status.autoPublishOff")}
                </Badge>
                <Badge variant="outline">
                  {t("modelRoutingPage.settings.updatedAt", {
                    value: formatDateTime(settings?.updated_at),
                  })}
                </Badge>
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                <SurfaceCard contentClassName="p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="font-medium">
                        {t("modelRoutingPage.settings.enabled")}
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {t("modelRoutingPage.settings.enabledHint")}
                      </div>
                    </div>
                    <Checkbox
                      checked={settingsDraft?.enabled ?? false}
                      onCheckedChange={(checked) =>
                        updateSettingsDraft((prev) => ({
                          ...prev,
                          enabled: checked === true,
                        }))
                      }
                    />
                  </div>
                </SurfaceCard>

                <SurfaceCard contentClassName="p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="font-medium">
                        {t("modelRoutingPage.settings.autoPublish")}
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {t("modelRoutingPage.settings.autoPublishHint")}
                      </div>
                    </div>
                    <Checkbox
                      checked={settingsDraft?.autoPublish ?? false}
                      onCheckedChange={(checked) =>
                        updateSettingsDraft((prev) => ({
                          ...prev,
                          autoPublish: checked === true,
                        }))
                      }
                    />
                  </div>
                </SurfaceCard>

                <SurfaceCard contentClassName="p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="font-medium">
                        {t("modelRoutingPage.settings.killSwitch")}
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {t("modelRoutingPage.settings.killSwitchHint")}
                      </div>
                    </div>
                    <Checkbox
                      checked={settingsDraft?.killSwitch ?? false}
                      onCheckedChange={(checked) =>
                        updateSettingsDraft((prev) => ({
                          ...prev,
                          killSwitch: checked === true,
                        }))
                      }
                    />
                  </div>
                </SurfaceCard>

                <SurfaceCard contentClassName="p-4">
                  <div className="mb-2 font-medium">
                    {t("modelRoutingPage.settings.triggerMode")}
                  </div>
                  <Select
                    value={settingsDraft?.triggerMode ?? "hybrid"}
                    onValueChange={(value) =>
                      updateSettingsDraft((prev) => ({
                        ...prev,
                        triggerMode: value as ModelRoutingTriggerMode,
                      }))
                    }
                  >
                    <SelectTrigger
                      className="w-full"
                      aria-label={t("modelRoutingPage.settings.triggerMode")}
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {triggerModeOptions.map((mode) => (
                        <SelectItem key={mode} value={mode}>
                          {triggerModeLabel(mode, t)}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </SurfaceCard>
              </div>

              <SurfaceCard contentClassName="p-4">
                <label className="mb-2 block font-medium">
                  {t("modelRoutingPage.settings.plannerModelChain")}
                </label>
                <ModelSelector
                  catalog={availableModels}
                  value={settingsDraft?.plannerModelChain ?? []}
                  onChange={(next) =>
                    updateSettingsDraft((prev) => ({
                      ...prev,
                      plannerModelChain: next,
                    }))
                  }
                  reorderable
                  labels={modelSelectorLabels}
                />
                <p className="mt-2 text-xs text-muted-foreground">
                  {t("modelRoutingPage.settings.plannerModelChainHint")}
                </p>
              </SurfaceCard>

              <div className="flex justify-end">
                <Button
                  color="primary"
                  onClick={() => saveSettingsMutation.mutate()}
                  disabled={saveSettingsMutation.isPending}
                >
                  <Save className="mr-2 h-4 w-4" />
                  {t("modelRoutingPage.actions.saveSettings")}
                </Button>
              </div>
            </PagePanel>

            <PagePanel className="space-y-5">
              <SectionHeader
                title={t("modelRoutingPage.claudeCode.title")}
                description={t("modelRoutingPage.claudeCode.description")}
              />
              <div className="flex flex-wrap items-center gap-2">
                <Badge
                  variant={
                    claudeCodeSettings?.enabled ? "success" : "secondary"
                  }
                >
                  {claudeCodeSettings?.enabled
                    ? t("modelRoutingPage.status.enabled")
                    : t("modelRoutingPage.status.disabled")}
                </Badge>
                <Badge variant="outline">
                  {t("modelRoutingPage.claudeCode.updatedAt", {
                    value: formatDateTime(claudeCodeSettings?.updated_at),
                  })}
                </Badge>
              </div>

              <SurfaceCard contentClassName="p-4">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="font-medium">
                      {t("modelRoutingPage.claudeCode.enabled")}
                    </div>
                    <div className="text-sm text-muted-foreground">
                      {t("modelRoutingPage.claudeCode.enabledHint")}
                    </div>
                  </div>
                  <Checkbox
                    checked={claudeCodeDraft?.enabled ?? false}
                    onCheckedChange={(checked) =>
                      updateClaudeCodeDraft((prev) => ({
                        ...prev,
                        enabled: checked === true,
                      }))
                    }
                  />
                </div>
              </SurfaceCard>

              <SurfaceCard contentClassName="space-y-3 p-4">
                <div>
                  <div className="font-medium">
                    {t("modelRoutingPage.claudeCode.fallbackMode")}
                  </div>
                  <div className="text-sm text-muted-foreground">
                    {t("modelRoutingPage.claudeCode.fallbackModeHint")}
                  </div>
                </div>
                <Select
                  value={
                    claudeCodeDraft?.effortRouting.fallback_mode ?? "clamp_down"
                  }
                  onValueChange={(next) =>
                    updateClaudeCodeFallbackMode(
                      next as ClaudeCodeEffortFallbackMode,
                    )
                  }
                >
                  <SelectTrigger>
                    <SelectValue
                      placeholder={t(
                        "modelRoutingPage.claudeCode.fallbackMode",
                      )}
                    />
                  </SelectTrigger>
                  <SelectContent>
                    {claudeCodeFallbackModeOptions.map((option) => (
                      <SelectItem key={option.value} value={option.value}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </SurfaceCard>

              <div className="grid gap-4 xl:grid-cols-3">
                {claudeCodeFamilies.map((family) => {
                  const selectedModel = claudeCodeDraft?.[family.key] ?? null;
                  const familyRouting =
                    claudeCodeDraft?.effortRouting[family.effortKey];

                  return (
                    <SurfaceCard
                      key={family.key}
                      contentClassName="space-y-4 p-4"
                    >
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <div className="font-medium">{family.label}</div>
                        {!selectedModel ? (
                          <Badge variant="secondary">
                            {t("modelRoutingPage.claudeCode.unmapped")}
                          </Badge>
                        ) : null}
                      </div>
                      <p className="text-sm text-muted-foreground">
                        {t("modelRoutingPage.claudeCode.familyHint", {
                          family: family.label,
                        })}
                      </p>
                      <ModelSelector
                        catalog={availableModels}
                        value={selectedModel ? [selectedModel] : []}
                        onChange={(next) =>
                          updateClaudeCodeDraft((prev) => ({
                            ...prev,
                            [family.key]: selectSingleModel(next),
                          }))
                        }
                        labels={claudeCodeModelSelectorLabels}
                      />

                      <div className="space-y-3 rounded-xl border border-border/60 bg-muted/20 p-3">
                        <div>
                          <div className="text-sm font-medium">
                            {t("modelRoutingPage.claudeCode.effortRoutingTitle")}
                          </div>
                          <div className="text-xs text-muted-foreground">
                            {t("modelRoutingPage.claudeCode.effortRoutingHint")}
                          </div>
                        </div>

                        <div className="grid gap-2">
                          <div className="grid gap-2 md:grid-cols-[minmax(0,1fr),minmax(0,1fr)] md:items-center">
                            <div className="text-sm">
                              {t(
                                "modelRoutingPage.claudeCode.unknownEffortLabel",
                              )}
                            </div>
                            <Select
                              value={
                                familyRouting?.default_target_effort ??
                                "__omit__"
                              }
                              onValueChange={(next) =>
                                updateClaudeCodeFamilyDefaultEffort(
                                  family.effortKey,
                                  next === "__omit__" ? null : next,
                                )
                              }
                            >
                              <SelectTrigger>
                                <SelectValue
                                  placeholder={t(
                                    "modelRoutingPage.claudeCode.effortLabels.omit",
                                  )}
                                />
                              </SelectTrigger>
                              <SelectContent>
                                {claudeCodeTargetEffortOptions.map((option) => (
                                  <SelectItem
                                    key={option.value ?? "__omit__"}
                                    value={option.value ?? "__omit__"}
                                  >
                                    {option.label}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          </div>

                          {family.sourceEfforts.length === 0 ? (
                            <div className="text-xs text-muted-foreground">
                              {t(
                                "modelRoutingPage.claudeCode.noFixedEffortsHint",
                              )}
                            </div>
                          ) : (
                            family.sourceEfforts.map((sourceEffort) => (
                              <div
                                key={`${family.key}-${sourceEffort}`}
                                className="grid gap-2 md:grid-cols-[minmax(0,1fr),minmax(0,1fr)] md:items-center"
                              >
                                <div className="text-sm">
                                  {t(
                                    `modelRoutingPage.claudeCode.effortLabels.${sourceEffort}`,
                                  )}
                                </div>
                                <Select
                                  value={
                                    familyRouting?.source_to_target?.[
                                      sourceEffort
                                    ] ?? "__omit__"
                                  }
                                  onValueChange={(next) =>
                                    updateClaudeCodeFamilyMappedEffort(
                                      family.effortKey,
                                      sourceEffort,
                                      next === "__omit__" ? null : next,
                                    )
                                  }
                                >
                                  <SelectTrigger>
                                    <SelectValue
                                      placeholder={t(
                                        "modelRoutingPage.claudeCode.effortLabels.omit",
                                      )}
                                    />
                                  </SelectTrigger>
                                  <SelectContent>
                                    {claudeCodeTargetEffortOptions.map(
                                      (option) => (
                                        <SelectItem
                                          key={option.value ?? "__omit__"}
                                          value={option.value ?? "__omit__"}
                                        >
                                          {option.label}
                                        </SelectItem>
                                      ),
                                    )}
                                  </SelectContent>
                                </Select>
                              </div>
                            ))
                          )}
                        </div>
                      </div>
                    </SurfaceCard>
                  );
                })}
              </div>

              <div className="flex justify-end gap-2">
                <Button
                  variant="flat"
                  onClick={() =>
                    setClaudeCodeDraftOverride(
                      createRecommendedClaudeCodeRoutingDraft(
                        claudeCodeDraft?.enabled ?? claudeCodeSettings?.enabled ?? false,
                      ),
                    )
                  }
                  disabled={saveClaudeCodeMutation.isPending}
                >
                  {t("modelRoutingPage.actions.useClaudeCodeDefaults")}
                </Button>
                <Button
                  variant="flat"
                  onClick={() => setClaudeCodeDraftOverride(null)}
                  disabled={
                    !claudeCodeHasChanges || saveClaudeCodeMutation.isPending
                  }
                >
                  {t("modelRoutingPage.actions.resetClaudeCode")}
                </Button>
                <Button
                  color="primary"
                  onClick={() => saveClaudeCodeMutation.mutate()}
                  disabled={
                    !claudeCodeHasChanges || saveClaudeCodeMutation.isPending
                  }
                >
                  <Save className="mr-2 h-4 w-4" />
                  {t("modelRoutingPage.actions.saveClaudeCode")}
                </Button>
              </div>
            </PagePanel>
          </div>

          <PagePanel tone="secondary" className="space-y-4">
            <SectionHeader
              title={t("modelRoutingPage.versions.title")}
              description={t("modelRoutingPage.versions.description")}
            />
            {versions.length === 0 ? (
              <EmptyState>{t("modelRoutingPage.versions.empty")}</EmptyState>
            ) : (
              <>
                <div
                  className={
                    versionsExpanded
                      ? "max-h-[30rem] space-y-3 overflow-y-auto pr-2"
                      : "space-y-3"
                  }
                >
                  {visibleVersions.visibleItems.map((version) => {
                    const reason =
                      version.reason || version.compiled_plan.trigger_reason;
                    return (
                      <SurfaceCard
                        key={version.id}
                        contentClassName="space-y-3 p-4"
                      >
                        <div className="flex flex-wrap items-center justify-between gap-2">
                          <div className="font-medium">
                            {formatDateTime(version.published_at)}
                          </div>
                          <Badge variant="outline">
                            {version.id.slice(0, 8)}
                          </Badge>
                        </div>
                        <div className="text-sm text-muted-foreground">
                          {reason?.trim() ||
                            t("modelRoutingPage.versions.noReason")}
                        </div>
                        <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
                          <span>
                            {t("modelRoutingPage.versions.defaultSegments", {
                              count: version.compiled_plan.default_route.length,
                            })}
                          </span>
                          <span>
                            {t("modelRoutingPage.versions.policyCount", {
                              count: version.compiled_plan.policies.length,
                            })}
                          </span>
                        </div>
                      </SurfaceCard>
                    );
                  })}
                </div>
                {visibleVersions.canToggle ? (
                  <div className="flex justify-end">
                    <Button
                      type="button"
                      variant="light"
                      size="sm"
                      onClick={() => setVersionsExpanded((current) => !current)}
                    >
                      {versionsExpanded ? (
                        <ChevronUp className="mr-2 h-4 w-4" />
                      ) : (
                        <ChevronDown className="mr-2 h-4 w-4" />
                      )}
                      {versionsExpanded
                        ? t("modelRoutingPage.versions.showLess")
                        : t("modelRoutingPage.versions.showMore", {
                            count: visibleVersions.hiddenCount,
                          })}
                    </Button>
                  </div>
                ) : null}
              </>
            )}
          </PagePanel>
        </div>

        <div className="grid gap-6 xl:grid-cols-[minmax(320px,0.75fr)_minmax(0,1.25fr)]">
          <PagePanel className="space-y-6">
            <SectionHeader
              title={t("modelRoutingPage.errorLearning.settings.title")}
              description={t(
                "modelRoutingPage.errorLearning.settings.description",
              )}
            />
            <div className="space-y-6">
              <div className="flex flex-wrap items-center gap-2">
                <Badge
                  variant={
                    errorLearningSettings?.enabled ? "success" : "secondary"
                  }
                >
                  {errorLearningSettings?.enabled
                    ? t("modelRoutingPage.status.enabled")
                    : t("modelRoutingPage.status.disabled")}
                </Badge>
                <Badge variant="outline">
                  {t("modelRoutingPage.errorLearning.settings.updatedAt", {
                    value: formatDateTime(errorLearningSettings?.updated_at),
                  })}
                </Badge>
              </div>

              <SurfaceCard contentClassName="space-y-5 p-4">
                <SurfaceCard contentClassName="p-4">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <div className="font-medium">
                        {t("modelRoutingPage.errorLearning.settings.enabled")}
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {t(
                          "modelRoutingPage.errorLearning.settings.enabledHint",
                        )}
                      </div>
                    </div>
                    <Checkbox
                      checked={errorLearningSettingsDraft?.enabled ?? false}
                      onCheckedChange={(checked) =>
                        updateErrorLearningDraft((prev) => ({
                          ...prev,
                          enabled: checked === true,
                        }))
                      }
                    />
                  </div>
                </SurfaceCard>

                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <label className="text-sm font-medium">
                      {t(
                        "modelRoutingPage.errorLearning.settings.firstSeenTimeoutMs",
                      )}
                    </label>
                    <Input
                      type="number"
                      value={
                        errorLearningSettingsDraft?.firstSeenTimeoutMs ?? ""
                      }
                      onChange={(event) =>
                        updateErrorLearningDraft((prev) => ({
                          ...prev,
                          firstSeenTimeoutMs: event.target.value,
                        }))
                      }
                    />
                    <p className="text-xs text-muted-foreground">
                      {t(
                        "modelRoutingPage.errorLearning.settings.firstSeenTimeoutMsHint",
                      )}
                    </p>
                  </div>

                  <div className="space-y-2">
                    <label className="text-sm font-medium">
                      {t(
                        "modelRoutingPage.errorLearning.settings.reviewHitThreshold",
                      )}
                    </label>
                    <Input
                      type="number"
                      value={
                        errorLearningSettingsDraft?.reviewHitThreshold ?? ""
                      }
                      onChange={(event) =>
                        updateErrorLearningDraft((prev) => ({
                          ...prev,
                          reviewHitThreshold: event.target.value,
                        }))
                      }
                    />
                    <p className="text-xs text-muted-foreground">
                      {t(
                        "modelRoutingPage.errorLearning.settings.reviewHitThresholdHint",
                      )}
                    </p>
                  </div>
                </div>
              </SurfaceCard>

              <div className="flex justify-end">
                <Button
                  color="primary"
                  onClick={() => saveErrorLearningSettingsMutation.mutate()}
                  disabled={saveErrorLearningSettingsMutation.isPending}
                >
                  <Save className="mr-2 h-4 w-4" />
                  {t("modelRoutingPage.errorLearning.actions.saveSettings")}
                </Button>
              </div>
            </div>
          </PagePanel>

          <PagePanel className="space-y-4">
            <SectionHeader
              title={t("modelRoutingPage.errorLearning.templates.title")}
              description={t(
                "modelRoutingPage.errorLearning.templates.description",
              )}
            />
            <div className="space-y-4">
              {upstreamErrorTemplates.length === 0 ? (
                <EmptyState>
                  {t("modelRoutingPage.errorLearning.templates.empty")}
                </EmptyState>
              ) : (
                upstreamErrorTemplates.map((template) => {
                  const isEditing =
                    editingTemplateId === template.id &&
                    editingTemplateDraft !== null;

                  return (
                    <SurfaceCard
                      key={template.id}
                      contentClassName="space-y-4 p-4"
                    >
                      <div className="flex flex-wrap items-start justify-between gap-3">
                        <div className="space-y-2">
                          <div className="flex flex-wrap items-center gap-2">
                            <Badge
                              variant={templateStatusVariant(template.status)}
                            >
                              {templateStatusLabel(template.status, t)}
                            </Badge>
                            <Badge variant="outline">{template.provider}</Badge>
                            <Badge variant="outline">
                              {t(
                                "modelRoutingPage.errorLearning.templates.normalizedStatusCode",
                                {
                                  value: template.normalized_status_code,
                                },
                              )}
                            </Badge>
                            <Badge variant="secondary">
                              {t(
                                "modelRoutingPage.errorLearning.templates.hitCount",
                                {
                                  count: template.hit_count,
                                },
                              )}
                            </Badge>
                          </div>
                          <div className="text-xs text-muted-foreground">
                            {t(
                              "modelRoutingPage.errorLearning.templates.fingerprint",
                            )}
                          </div>
                          <SurfaceCode>{template.fingerprint}</SurfaceCode>
                        </div>

                        <div className="flex flex-wrap gap-2">
                          <Button
                            size="sm"
                            onClick={() => openTemplateEditor(template)}
                            disabled={anyTemplateMutationPending}
                          >
                            <SquarePen className="mr-2 h-4 w-4" />
                            {t("modelRoutingPage.actions.edit")}
                          </Button>
                          <Button
                            size="sm"
                            onClick={() =>
                              rewriteTemplateMutation.mutate(template.id)
                            }
                            disabled={anyTemplateMutationPending}
                          >
                            <Sparkles className="mr-2 h-4 w-4" />
                            {t(
                              "modelRoutingPage.errorLearning.actions.rewrite",
                            )}
                          </Button>
                          <Button
                            size="sm"
                            onClick={() =>
                              approveTemplateMutation.mutate(template.id)
                            }
                            disabled={
                              anyTemplateMutationPending ||
                              template.status === "approved"
                            }
                          >
                            <Check className="mr-2 h-4 w-4" />
                            {t(
                              "modelRoutingPage.errorLearning.actions.approve",
                            )}
                          </Button>
                          <Button
                            color="danger"
                            variant="light"
                            size="sm"
                            onClick={() =>
                              rejectTemplateMutation.mutate(template.id)
                            }
                            disabled={
                              anyTemplateMutationPending ||
                              template.status === "rejected"
                            }
                          >
                            <Ban className="mr-2 h-4 w-4" />
                            {t("modelRoutingPage.errorLearning.actions.reject")}
                          </Button>
                        </div>
                      </div>

                      <div className="grid gap-3 text-sm text-muted-foreground md:grid-cols-3">
                        <div>
                          <div className="text-xs font-medium uppercase tracking-wide">
                            {t(
                              "modelRoutingPage.errorLearning.templates.semanticErrorCode",
                            )}
                          </div>
                          {isEditing ? (
                            <Input
                              value={editingTemplateDraft.semanticErrorCode}
                              onChange={(event) =>
                                updateTemplateDraft((prev) => ({
                                  ...prev,
                                  semanticErrorCode: event.target.value,
                                }))
                              }
                            />
                          ) : (
                            <div className="mt-1 font-mono text-foreground">
                              {template.semantic_error_code || "-"}
                            </div>
                          )}
                        </div>

                        <div>
                          <div className="text-xs font-medium uppercase tracking-wide">
                            {t(
                              "modelRoutingPage.errorLearning.templates.action",
                            )}
                          </div>
                          {isEditing ? (
                            <Select
                              value={editingTemplateDraft.action}
                              onValueChange={(value) =>
                                updateTemplateDraft((prev) => ({
                                  ...prev,
                                  action: value as UpstreamErrorAction,
                                }))
                              }
                            >
                              <SelectTrigger className="mt-1 w-full">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                {ERROR_TEMPLATE_ACTIONS.map((action) => (
                                  <SelectItem key={action} value={action}>
                                    {upstreamErrorActionLabel(action, t)}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          ) : (
                            <div className="mt-1 text-foreground">
                              {upstreamErrorActionLabel(template.action, t)}
                            </div>
                          )}
                        </div>

                        <div>
                          <div className="text-xs font-medium uppercase tracking-wide">
                            {t(
                              "modelRoutingPage.errorLearning.templates.retryScope",
                            )}
                          </div>
                          {isEditing ? (
                            <Select
                              value={editingTemplateDraft.retryScope}
                              onValueChange={(value) =>
                                updateTemplateDraft((prev) => ({
                                  ...prev,
                                  retryScope: value as UpstreamErrorRetryScope,
                                }))
                              }
                            >
                              <SelectTrigger className="mt-1 w-full">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                {ERROR_TEMPLATE_RETRY_SCOPES.map((scope) => (
                                  <SelectItem key={scope} value={scope}>
                                    {upstreamErrorRetryScopeLabel(scope, t)}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          ) : (
                            <div className="mt-1 text-foreground">
                              {upstreamErrorRetryScopeLabel(
                                template.retry_scope,
                                t,
                              )}
                            </div>
                          )}
                        </div>
                      </div>

                      <div className="grid gap-3 text-sm text-muted-foreground md:grid-cols-3">
                        <div>
                          <div className="text-xs font-medium uppercase tracking-wide">
                            {t(
                              "modelRoutingPage.errorLearning.templates.firstSeenAt",
                            )}
                          </div>
                          <div className="mt-1 text-foreground">
                            {formatDateTime(template.first_seen_at)}
                          </div>
                        </div>
                        <div>
                          <div className="text-xs font-medium uppercase tracking-wide">
                            {t(
                              "modelRoutingPage.errorLearning.templates.lastSeenAt",
                            )}
                          </div>
                          <div className="mt-1 text-foreground">
                            {formatDateTime(template.last_seen_at)}
                          </div>
                        </div>
                        <div>
                          <div className="text-xs font-medium uppercase tracking-wide">
                            {t(
                              "modelRoutingPage.errorLearning.templates.updatedAt",
                            )}
                          </div>
                          <div className="mt-1 text-foreground">
                            {formatDateTime(template.updated_at)}
                          </div>
                        </div>
                      </div>

                      <div className="space-y-2">
                        <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                          {t(
                            "modelRoutingPage.errorLearning.templates.representativeSamples",
                          )}
                        </div>
                        {template.representative_samples.length === 0 ? (
                          <EmptyState className="px-3 py-3">
                            {t(
                              "modelRoutingPage.errorLearning.templates.samplesEmpty",
                            )}
                          </EmptyState>
                        ) : (
                          <div className="space-y-2">
                            {template.representative_samples.map(
                              (sample, index) => (
                                <SurfaceCode
                                  key={`${template.id}-sample-${index}`}
                                >
                                  {sample}
                                </SurfaceCode>
                              ),
                            )}
                          </div>
                        )}
                      </div>

                      <div className="space-y-3">
                        <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                          {t(
                            "modelRoutingPage.errorLearning.templates.localizedTemplates",
                          )}
                        </div>
                        <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
                          {ERROR_TEMPLATE_LOCALES.map((key) => (
                            <div
                              key={`${template.id}-${key}`}
                              className="space-y-2"
                            >
                              <div className="text-xs font-medium text-muted-foreground">
                                {localeLabel(key, t)}
                              </div>
                              {isEditing ? (
                                <Textarea
                                  rows={3}
                                  value={editingTemplateDraft.templates[key]}
                                  onChange={(event) =>
                                    updateTemplateDraft((prev) => ({
                                      ...prev,
                                      templates: {
                                        ...prev.templates,
                                        [key]: event.target.value,
                                      },
                                    }))
                                  }
                                />
                              ) : (
                                <SurfaceCode className="min-h-24">
                                  {template.templates[key]?.trim() ||
                                    t(
                                      "modelRoutingPage.errorLearning.templates.localeEmpty",
                                    )}
                                </SurfaceCode>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>

                      {isEditing ? (
                        <div className="flex flex-wrap justify-end gap-2">
                          <Button
                            size="sm"
                            onClick={cancelTemplateEditor}
                            disabled={updateTemplateMutation.isPending}
                          >
                            <X className="mr-2 h-4 w-4" />
                            {t("modelRoutingPage.errorLearning.actions.cancel")}
                          </Button>
                          <Button
                            size="sm"
                            color="primary"
                            onClick={() => {
                              if (!editingTemplateDraft) {
                                return;
                              }
                              updateTemplateMutation.mutate({
                                templateId: template.id,
                                payload: {
                                  semantic_error_code:
                                    editingTemplateDraft.semanticErrorCode.trim(),
                                  action: editingTemplateDraft.action,
                                  retry_scope: editingTemplateDraft.retryScope,
                                  templates:
                                    createLocalizedErrorTemplatesPayload(
                                      editingTemplateDraft.templates,
                                    ),
                                },
                              });
                            }}
                            disabled={updateTemplateMutation.isPending}
                          >
                            <Save className="mr-2 h-4 w-4" />
                            {t(
                              "modelRoutingPage.errorLearning.actions.saveTemplate",
                            )}
                          </Button>
                        </div>
                      ) : null}
                    </SurfaceCard>
                  );
                })
              )}
            </div>
          </PagePanel>

          <PagePanel className="space-y-4 xl:col-span-2">
            <SectionHeader
              title={t("modelRoutingPage.errorLearning.builtinTemplates.title")}
              description={t(
                "modelRoutingPage.errorLearning.builtinTemplates.description",
              )}
            />
            <div className="space-y-4">
              {builtinErrorTemplates.length === 0 ? (
                <EmptyState>
                  {t("modelRoutingPage.errorLearning.builtinTemplates.empty")}
                </EmptyState>
              ) : (
                <>
                  <div
                    className={
                      builtinTemplatesExpanded
                        ? "max-h-[30rem] space-y-4 overflow-y-auto pr-2"
                        : "space-y-4"
                    }
                  >
                    {visibleBuiltinTemplates.visibleItems.map((template) => {
                      const templateKey = builtinErrorTemplateKey(template);
                      const isEditing =
                        editingBuiltinTemplateKey === templateKey &&
                        editingBuiltinTemplateDraft !== null;

                      return (
                        <SurfaceCard
                          key={templateKey}
                          contentClassName="space-y-4 p-4"
                        >
                          <div className="flex flex-wrap items-start justify-between gap-3">
                            <div className="space-y-2">
                              <div className="flex flex-wrap items-center gap-2">
                                <Badge variant="outline">
                                  {builtinErrorTemplateKindLabel(
                                    template.kind,
                                    t,
                                  )}
                                </Badge>
                                <Badge
                                  variant={
                                    template.is_overridden
                                      ? "warning"
                                      : "secondary"
                                  }
                                >
                                  {template.is_overridden
                                    ? t(
                                        "modelRoutingPage.errorLearning.builtinTemplates.overridden",
                                      )
                                    : t(
                                        "modelRoutingPage.errorLearning.builtinTemplates.defaultState",
                                      )}
                                </Badge>
                                <Badge variant="secondary">
                                  {template.code}
                                </Badge>
                              </div>
                              <div className="text-xs text-muted-foreground">
                                {t(
                                  "modelRoutingPage.errorLearning.builtinTemplates.updatedAt",
                                  {
                                    value: formatDateTime(template.updated_at),
                                  },
                                )}
                              </div>
                            </div>

                            <div className="flex flex-wrap gap-2">
                              <Button
                                size="sm"
                                onClick={() =>
                                  openBuiltinTemplateEditor(template)
                                }
                                disabled={anyBuiltinTemplateMutationPending}
                              >
                                <SquarePen className="mr-2 h-4 w-4" />
                                {t("modelRoutingPage.actions.edit")}
                              </Button>
                              <Button
                                size="sm"
                                onClick={() =>
                                  rewriteBuiltinTemplateMutation.mutate({
                                    kind: template.kind,
                                    code: template.code,
                                  })
                                }
                                disabled={anyBuiltinTemplateMutationPending}
                              >
                                <Sparkles className="mr-2 h-4 w-4" />
                                {t(
                                  "modelRoutingPage.errorLearning.actions.rewrite",
                                )}
                              </Button>
                              <Button
                                size="sm"
                                onClick={() =>
                                  resetBuiltinTemplateMutation.mutate({
                                    kind: template.kind,
                                    code: template.code,
                                  })
                                }
                                disabled={
                                  anyBuiltinTemplateMutationPending ||
                                  !template.is_overridden
                                }
                              >
                                <RotateCw className="mr-2 h-4 w-4" />
                                {t(
                                  "modelRoutingPage.errorLearning.builtinTemplates.reset",
                                )}
                              </Button>
                            </div>
                          </div>

                          <div className="grid gap-3 text-sm text-muted-foreground md:grid-cols-3">
                            <div>
                              <div className="text-xs font-medium uppercase tracking-wide">
                                {t(
                                  "modelRoutingPage.errorLearning.builtinTemplates.kind",
                                )}
                              </div>
                              <div className="mt-1 text-foreground">
                                {builtinErrorTemplateKindLabel(
                                  template.kind,
                                  t,
                                )}
                              </div>
                            </div>
                            <div>
                              <div className="text-xs font-medium uppercase tracking-wide">
                                {t(
                                  "modelRoutingPage.errorLearning.builtinTemplates.code",
                                )}
                              </div>
                              <div className="mt-1 font-mono text-foreground">
                                {template.code}
                              </div>
                            </div>
                            {template.kind === "heuristic_upstream" ? (
                              <div className="grid gap-3 md:col-span-1 md:grid-cols-2">
                                <div>
                                  <div className="text-xs font-medium uppercase tracking-wide">
                                    {t(
                                      "modelRoutingPage.errorLearning.templates.action",
                                    )}
                                  </div>
                                  <div className="mt-1 text-foreground">
                                    {template.action
                                      ? upstreamErrorActionLabel(
                                          template.action,
                                          t,
                                        )
                                      : t("modelRoutingPage.common.none")}
                                  </div>
                                </div>
                                <div>
                                  <div className="text-xs font-medium uppercase tracking-wide">
                                    {t(
                                      "modelRoutingPage.errorLearning.templates.retryScope",
                                    )}
                                  </div>
                                  <div className="mt-1 text-foreground">
                                    {template.retry_scope
                                      ? upstreamErrorRetryScopeLabel(
                                          template.retry_scope,
                                          t,
                                        )
                                      : t("modelRoutingPage.common.none")}
                                  </div>
                                </div>
                              </div>
                            ) : (
                              <div>
                                <div className="text-xs font-medium uppercase tracking-wide">
                                  {t(
                                    "modelRoutingPage.errorLearning.builtinTemplates.scope",
                                  )}
                                </div>
                                <div className="mt-1 text-foreground">
                                  {t(
                                    "modelRoutingPage.errorLearning.builtinTemplates.gatewayOnly",
                                  )}
                                </div>
                              </div>
                            )}
                          </div>

                          <div className="space-y-3">
                            <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                              {t(
                                "modelRoutingPage.errorLearning.builtinTemplates.localizedTemplates",
                              )}
                            </div>
                            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
                              {ERROR_TEMPLATE_LOCALES.map((key) => (
                                <div
                                  key={`${templateKey}-${key}`}
                                  className="space-y-2"
                                >
                                  <div className="text-xs font-medium text-muted-foreground">
                                    {localeLabel(key, t)}
                                  </div>
                                  {isEditing ? (
                                    <Textarea
                                      rows={3}
                                      value={
                                        editingBuiltinTemplateDraft.templates[
                                          key
                                        ]
                                      }
                                      onChange={(event) =>
                                        updateBuiltinTemplateDraft((prev) => ({
                                          ...prev,
                                          templates: {
                                            ...prev.templates,
                                            [key]: event.target.value,
                                          },
                                        }))
                                      }
                                    />
                                  ) : (
                                    <SurfaceCode className="min-h-24">
                                      {template.templates[key]?.trim() ||
                                        t(
                                          "modelRoutingPage.errorLearning.templates.localeEmpty",
                                        )}
                                    </SurfaceCode>
                                  )}
                                </div>
                              ))}
                            </div>
                          </div>

                          <div className="space-y-3">
                            <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                              {t(
                                "modelRoutingPage.errorLearning.builtinTemplates.defaultTemplates",
                              )}
                            </div>
                            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
                              {ERROR_TEMPLATE_LOCALES.map((key) => (
                                <div
                                  key={`${templateKey}-default-${key}`}
                                  className="space-y-2"
                                >
                                  <div className="text-xs font-medium text-muted-foreground">
                                    {localeLabel(key, t)}
                                  </div>
                                  <SurfaceCode className="min-h-24">
                                    {template.default_templates[key]?.trim() ||
                                      t(
                                        "modelRoutingPage.errorLearning.templates.localeEmpty",
                                      )}
                                  </SurfaceCode>
                                </div>
                              ))}
                            </div>
                          </div>

                          {isEditing ? (
                            <div className="flex flex-wrap justify-end gap-2">
                              <Button
                                size="sm"
                                onClick={cancelBuiltinTemplateEditor}
                                disabled={
                                  updateBuiltinTemplateMutation.isPending
                                }
                              >
                                <X className="mr-2 h-4 w-4" />
                                {t(
                                  "modelRoutingPage.errorLearning.actions.cancel",
                                )}
                              </Button>
                              <Button
                                size="sm"
                                color="primary"
                                onClick={() =>
                                  updateBuiltinTemplateMutation.mutate({
                                    kind: template.kind,
                                    code: template.code,
                                    payload: {
                                      templates:
                                        createLocalizedErrorTemplatesPayload(
                                          editingBuiltinTemplateDraft.templates,
                                        ),
                                    },
                                  })
                                }
                                disabled={
                                  updateBuiltinTemplateMutation.isPending
                                }
                              >
                                <Save className="mr-2 h-4 w-4" />
                                {t(
                                  "modelRoutingPage.errorLearning.builtinTemplates.save",
                                )}
                              </Button>
                            </div>
                          ) : null}
                        </SurfaceCard>
                      );
                    })}
                  </div>
                  {visibleBuiltinTemplates.canToggle ? (
                    <div className="flex justify-end">
                      <Button
                        type="button"
                        variant="light"
                        size="sm"
                        onClick={() =>
                          setBuiltinTemplatesExpanded((current) => !current)
                        }
                      >
                        {builtinTemplatesExpanded ? (
                          <ChevronUp className="mr-2 h-4 w-4" />
                        ) : (
                          <ChevronDown className="mr-2 h-4 w-4" />
                        )}
                        {builtinTemplatesExpanded
                          ? t("modelRoutingPage.versions.showLess")
                          : t("modelRoutingPage.versions.showMore", {
                              count: visibleBuiltinTemplates.hiddenCount,
                            })}
                      </Button>
                    </div>
                  ) : null}
                </>
              )}
            </div>
          </PagePanel>
        </div>

        <div className="mt-6 grid gap-6 xl:grid-cols-2">
          <PagePanel className="space-y-3">
            <SectionHeader
              title={t("modelRoutingPage.profiles.title")}
              description={t("modelRoutingPage.profiles.description")}
            />
            <div className="space-y-3">
              {profiles.length === 0 ? (
                <EmptyState>{t("modelRoutingPage.profiles.empty")}</EmptyState>
              ) : (
                profiles.map((profile) => (
                  <SurfaceCard
                    key={profile.id}
                    contentClassName="space-y-3 p-4"
                  >
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div className="space-y-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <div className="font-medium">{profile.name}</div>
                          <Badge
                            variant={profile.enabled ? "success" : "secondary"}
                          >
                            {profile.enabled
                              ? t("modelRoutingPage.status.enabled")
                              : t("modelRoutingPage.status.disabled")}
                          </Badge>
                          <Badge variant="outline">
                            {t("modelRoutingPage.common.priority", {
                              value: profile.priority,
                            })}
                          </Badge>
                        </div>
                        {profile.description ? (
                          <div className="text-sm text-muted-foreground">
                            {profile.description}
                          </div>
                        ) : null}
                        <div className="text-sm text-muted-foreground">
                          {selectorFilterSummary(profile, t)}
                        </div>
                      </div>
                      <div className="flex gap-2">
                        <Button
                          variant="flat"
                          size="sm"
                          onClick={() => openEditProfileDialog(profile)}
                        >
                          <SquarePen className="mr-2 h-4 w-4" />
                          {t("modelRoutingPage.actions.edit")}
                        </Button>
                        <Button
                          color="danger"
                          variant="light"
                          size="sm"
                          onClick={() =>
                            deleteProfileMutation.mutate(profile.id)
                          }
                        >
                          <Trash2 className="mr-2 h-4 w-4" />
                          {t("modelRoutingPage.actions.delete")}
                        </Button>
                      </div>
                    </div>
                    <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
                      {profile.selector.modes.length > 0 ? (
                        <span>
                          {profile.selector.modes
                            .map((mode) => modeLabel(mode, t))
                            .join(" / ")}
                        </span>
                      ) : (
                        <span>{t("modelRoutingPage.profiles.anyMode")}</span>
                      )}
                      {profile.selector.auth_providers.length > 0 ? (
                        <span>
                          {profile.selector.auth_providers
                            .map((provider) => authProviderLabel(provider, t))
                            .join(" / ")}
                        </span>
                      ) : null}
                    </div>
                  </SurfaceCard>
                ))
              )}
            </div>
          </PagePanel>

          <PagePanel className="space-y-3">
            <SectionHeader
              title={t("modelRoutingPage.policies.title")}
              description={t("modelRoutingPage.policies.description")}
            />
            <div className="space-y-3">
              {policies.length === 0 ? (
                <EmptyState>{t("modelRoutingPage.policies.empty")}</EmptyState>
              ) : (
                policies.map((policy) => (
                  <SurfaceCard key={policy.id} contentClassName="space-y-3 p-4">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div className="space-y-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <div className="font-medium">{policy.name}</div>
                          <Badge
                            variant={policy.enabled ? "success" : "secondary"}
                          >
                            {policy.enabled
                              ? t("modelRoutingPage.status.enabled")
                              : t("modelRoutingPage.status.disabled")}
                          </Badge>
                          <Badge variant="outline">
                            {t("modelRoutingPage.common.priority", {
                              value: policy.priority,
                            })}
                          </Badge>
                          <Badge variant="info">{policy.family}</Badge>
                        </div>
                        <div className="text-sm text-muted-foreground">
                          {t("modelRoutingPage.policies.summary", {
                            exact: policy.exact_models.length,
                            prefixes: policy.model_prefixes.length,
                            fallbacks: policy.fallback_profile_ids.length,
                          })}
                        </div>
                        <div className="text-sm text-muted-foreground">
                          {t("modelRoutingPage.policies.fallbackChain", {
                            value: fallbackProfileSummary(
                              policy,
                              profileNames,
                              t,
                            ),
                          })}
                        </div>
                      </div>
                      <div className="flex gap-2">
                        <Button
                          variant="flat"
                          size="sm"
                          onClick={() => openEditPolicyDialog(policy)}
                        >
                          <SquarePen className="mr-2 h-4 w-4" />
                          {t("modelRoutingPage.actions.edit")}
                        </Button>
                        <Button
                          color="danger"
                          variant="light"
                          size="sm"
                          onClick={() => deletePolicyMutation.mutate(policy.id)}
                        >
                          <Trash2 className="mr-2 h-4 w-4" />
                          {t("modelRoutingPage.actions.delete")}
                        </Button>
                      </div>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {policy.exact_models.slice(0, 4).map((model) => (
                        <Badge key={model} variant="outline">
                          {model}
                        </Badge>
                      ))}
                      {policy.exact_models.length > 4 ? (
                        <Badge variant="secondary">
                          {t("modelRoutingPage.policies.moreExactModels", {
                            count: policy.exact_models.length - 4,
                          })}
                        </Badge>
                      ) : null}
                    </div>
                  </SurfaceCard>
                ))
              )}
            </div>
          </PagePanel>
        </div>

        <Dialog open={profileDialogOpen} onOpenChange={setProfileDialogOpen}>
          <AntigravityDialogShell
            size="lg"
            title={
              profileForm.id
                ? t("modelRoutingPage.dialogs.editProfile")
                : t("modelRoutingPage.dialogs.createProfile")
            }
            description={t("modelRoutingPage.dialogs.profileDescription")}
            footer={
              <AntigravityDialogActions className="justify-between">
                {profileForm.id ? (
                  <Button
                    color="danger"
                    variant="light"
                    onClick={() =>
                      deleteProfileMutation.mutate(profileForm.id!)
                    }
                    disabled={deleteProfileMutation.isPending}
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    {t("modelRoutingPage.actions.deleteProfile")}
                  </Button>
                ) : (
                  <div />
                )}
                <Button
                  color="primary"
                  onClick={() => upsertProfileMutation.mutate()}
                  disabled={upsertProfileMutation.isPending}
                >
                  <Save className="mr-2 h-4 w-4" />
                  {t("modelRoutingPage.actions.saveProfile")}
                </Button>
              </AntigravityDialogActions>
            }
          >
            <AntigravityDialogBody className="grid gap-4 md:grid-cols-2">
              <AntigravityDialogPanel className="space-y-4 md:col-span-2">
                <div className="space-y-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.name")}
                  </label>
                  <Input
                    value={profileForm.name}
                    onChange={(event) =>
                      setProfileForm((prev) => ({
                        ...prev,
                        name: event.target.value,
                      }))
                    }
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.description")}
                  </label>
                  <Textarea
                    value={profileForm.description}
                    onChange={(event) =>
                      setProfileForm((prev) => ({
                        ...prev,
                        description: event.target.value,
                      }))
                    }
                    rows={3}
                  />
                </div>
                <div className="grid gap-4 md:grid-cols-[minmax(0,0.7fr)_minmax(0,1fr)]">
                  <div className="space-y-2">
                    <label className="text-sm font-medium">
                      {t("modelRoutingPage.form.priority")}
                    </label>
                    <Input
                      type="number"
                      value={profileForm.priority}
                      onChange={(event) =>
                        setProfileForm((prev) => ({
                          ...prev,
                          priority: event.target.value,
                        }))
                      }
                    />
                  </div>
                  <SurfaceCard contentClassName="p-4">
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <div className="font-medium">
                          {t("modelRoutingPage.form.enabled")}
                        </div>
                        <div className="text-sm text-muted-foreground">
                          {t("modelRoutingPage.form.enabledHint")}
                        </div>
                      </div>
                      <Checkbox
                        checked={profileForm.enabled}
                        onCheckedChange={(checked) =>
                          setProfileForm((prev) => ({
                            ...prev,
                            enabled: checked === true,
                          }))
                        }
                      />
                    </div>
                  </SurfaceCard>
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.planTypes")}
                  </label>
                  <Input
                    value={profileForm.planTypes}
                    onChange={(event) =>
                      setProfileForm((prev) => ({
                        ...prev,
                        planTypes: event.target.value,
                      }))
                    }
                    placeholder={t(
                      "modelRoutingPage.form.planTypesPlaceholder",
                    )}
                  />
                </div>
              </AntigravityDialogPanel>

              <AntigravityDialogPanel>
                <div className="text-sm font-medium">
                  {t("modelRoutingPage.form.modes")}
                </div>
                <SurfaceCard contentClassName="space-y-2 p-3">
                  {modeOptions.map((mode) => (
                    <label
                      key={mode}
                      className="flex items-center justify-between gap-3 text-sm"
                    >
                      <span>{modeLabel(mode, t)}</span>
                      <Checkbox
                        checked={profileForm.modes.includes(mode)}
                        onCheckedChange={() =>
                          setProfileForm((prev) => ({
                            ...prev,
                            modes: toggleItem(prev.modes, mode),
                          }))
                        }
                      />
                    </label>
                  ))}
                </SurfaceCard>
              </AntigravityDialogPanel>

              <AntigravityDialogPanel>
                <div className="text-sm font-medium">
                  {t("modelRoutingPage.form.authProviders")}
                </div>
                <SurfaceCard contentClassName="space-y-2 p-3">
                  {authProviderOptions.map((provider) => (
                    <label
                      key={provider}
                      className="flex items-center justify-between gap-3 text-sm"
                    >
                      <span>{authProviderLabel(provider, t)}</span>
                      <Checkbox
                        checked={profileForm.authProviders.includes(provider)}
                        onCheckedChange={() =>
                          setProfileForm((prev) => ({
                            ...prev,
                            authProviders: toggleItem(
                              prev.authProviders,
                              provider,
                            ),
                          }))
                        }
                      />
                    </label>
                  ))}
                </SurfaceCard>
              </AntigravityDialogPanel>

              <AntigravityDialogPanel>
                <div className="space-y-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.includeAccounts")}
                  </label>
                  <Textarea
                    value={profileForm.includeAccountIds}
                    onChange={(event) =>
                      setProfileForm((prev) => ({
                        ...prev,
                        includeAccountIds: event.target.value,
                      }))
                    }
                    rows={4}
                    placeholder={t(
                      "modelRoutingPage.form.includeAccountsPlaceholder",
                    )}
                  />
                </div>
              </AntigravityDialogPanel>

              <AntigravityDialogPanel>
                <div className="space-y-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.excludeAccounts")}
                  </label>
                  <Textarea
                    value={profileForm.excludeAccountIds}
                    onChange={(event) =>
                      setProfileForm((prev) => ({
                        ...prev,
                        excludeAccountIds: event.target.value,
                      }))
                    }
                    rows={4}
                    placeholder={t(
                      "modelRoutingPage.form.excludeAccountsPlaceholder",
                    )}
                  />
                </div>
              </AntigravityDialogPanel>
            </AntigravityDialogBody>
          </AntigravityDialogShell>
        </Dialog>

        <Dialog open={policyDialogOpen} onOpenChange={setPolicyDialogOpen}>
          <AntigravityDialogShell
            size="lg"
            title={
              policyForm.id
                ? t("modelRoutingPage.dialogs.editPolicy")
                : t("modelRoutingPage.dialogs.createPolicy")
            }
            description={t("modelRoutingPage.dialogs.policyDescription")}
            footer={
              <AntigravityDialogActions className="justify-between">
                {policyForm.id ? (
                  <Button
                    color="danger"
                    variant="light"
                    onClick={() => deletePolicyMutation.mutate(policyForm.id!)}
                    disabled={deletePolicyMutation.isPending}
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    {t("modelRoutingPage.actions.deletePolicy")}
                  </Button>
                ) : (
                  <div />
                )}
                <Button
                  color="primary"
                  onClick={() => upsertPolicyMutation.mutate()}
                  disabled={upsertPolicyMutation.isPending}
                >
                  <Save className="mr-2 h-4 w-4" />
                  {t("modelRoutingPage.actions.savePolicy")}
                </Button>
              </AntigravityDialogActions>
            }
          >
            <AntigravityDialogBody className="grid gap-4 md:grid-cols-2">
              <AntigravityDialogPanel className="space-y-4">
                <div className="space-y-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.name")}
                  </label>
                  <Input
                    value={policyForm.name}
                    onChange={(event) =>
                      setPolicyForm((prev) => ({
                        ...prev,
                        name: event.target.value,
                      }))
                    }
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.family")}
                  </label>
                  <Input
                    value={policyForm.family}
                    onChange={(event) =>
                      setPolicyForm((prev) => ({
                        ...prev,
                        family: event.target.value,
                      }))
                    }
                    placeholder={t("modelRoutingPage.form.familyPlaceholder")}
                  />
                </div>
                <div className="space-y-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.priority")}
                  </label>
                  <Input
                    type="number"
                    value={policyForm.priority}
                    onChange={(event) =>
                      setPolicyForm((prev) => ({
                        ...prev,
                        priority: event.target.value,
                      }))
                    }
                  />
                </div>
                <SurfaceCard contentClassName="p-4">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="font-medium">
                        {t("modelRoutingPage.form.enabled")}
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {t("modelRoutingPage.form.policyEnabledHint")}
                      </div>
                    </div>
                    <Checkbox
                      checked={policyForm.enabled}
                      onCheckedChange={(checked) =>
                        setPolicyForm((prev) => ({
                          ...prev,
                          enabled: checked === true,
                        }))
                      }
                    />
                  </div>
                </SurfaceCard>
              </AntigravityDialogPanel>

              <AntigravityDialogPanel className="space-y-4 md:col-span-2">
                <div className="space-y-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.exactModels")}
                  </label>
                  <ModelSelector
                    catalog={availableModels}
                    value={policyForm.exactModels}
                    onChange={(next) =>
                      setPolicyForm((prev) => ({ ...prev, exactModels: next }))
                    }
                    labels={modelSelectorLabels}
                  />
                  <p className="text-xs text-muted-foreground">
                    {t("modelRoutingPage.form.exactModelsHint")}
                  </p>
                </div>
                <div className="space-y-2 md:col-span-2">
                  <label className="text-sm font-medium">
                    {t("modelRoutingPage.form.modelPrefixes")}
                  </label>
                  <Textarea
                    value={policyForm.modelPrefixes}
                    onChange={(event) =>
                      setPolicyForm((prev) => ({
                        ...prev,
                        modelPrefixes: event.target.value,
                      }))
                    }
                    rows={4}
                    placeholder={t(
                      "modelRoutingPage.form.modelPrefixesPlaceholder",
                    )}
                  />
                  <p className="text-xs text-muted-foreground">
                    {t("modelRoutingPage.form.modelPrefixesHint")}
                  </p>
                </div>
              </AntigravityDialogPanel>

              <AntigravityDialogPanel className="space-y-4 md:col-span-2">
                <div className="space-y-2">
                  <div className="text-sm font-medium">
                    {t("modelRoutingPage.form.fallbackProfiles")}
                  </div>
                  <SurfaceCard contentClassName="space-y-2 p-3">
                    {profiles.length === 0 ? (
                      <div className="text-sm text-muted-foreground">
                        {t("modelRoutingPage.form.noProfilesAvailable")}
                      </div>
                    ) : (
                      profiles.map((profile) => (
                        <label
                          key={profile.id}
                          className="flex items-center justify-between gap-3 text-sm"
                        >
                          <span>{profile.name}</span>
                          <Checkbox
                            checked={policyForm.fallbackProfileIds.includes(
                              profile.id,
                            )}
                            onCheckedChange={() =>
                              setPolicyForm((prev) => ({
                                ...prev,
                                fallbackProfileIds: toggleItem(
                                  prev.fallbackProfileIds,
                                  profile.id,
                                ),
                              }))
                            }
                          />
                        </label>
                      ))
                    )}
                  </SurfaceCard>
                </div>
              </AntigravityDialogPanel>
            </AntigravityDialogBody>
          </AntigravityDialogShell>
        </Dialog>
      </div>
    </PageContent>
  );
}
