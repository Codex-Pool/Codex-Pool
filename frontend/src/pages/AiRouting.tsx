import { useCallback, useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { RotateCw, Save, SquarePen, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import {
  aiRoutingApi,
  type AiRoutingSettings,
  type AiRoutingTriggerMode,
  type ModelRoutingPolicy,
  type RoutingProfile,
  type UpstreamAuthProvider,
  type UpstreamMode,
} from '@/api/aiRouting'
import { localizeApiErrorDisplay } from '@/api/errorI18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { LoadingOverlay } from '@/components/ui/loading-overlay'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { POOL_SECTION_CLASS_NAME } from '@/lib/pool-styles'

type ProfileFormState = {
  id?: string
  name: string
  description: string
  enabled: boolean
  priority: string
  planTypes: string
  modes: UpstreamMode[]
  authProviders: UpstreamAuthProvider[]
  includeAccountIds: string
  excludeAccountIds: string
}

type PolicyFormState = {
  id?: string
  name: string
  family: string
  exactModels: string
  modelPrefixes: string
  fallbackProfileIds: string[]
  enabled: boolean
  priority: string
}

type SettingsFormState = {
  enabled: boolean
  autoPublish: boolean
  plannerModelChain: string
  triggerMode: AiRoutingTriggerMode
  killSwitch: boolean
}

const DEFAULT_PROFILE_FORM: ProfileFormState = {
  name: '',
  description: '',
  enabled: true,
  priority: '100',
  planTypes: '',
  modes: [],
  authProviders: [],
  includeAccountIds: '',
  excludeAccountIds: '',
}

const DEFAULT_POLICY_FORM: PolicyFormState = {
  name: '',
  family: '',
  exactModels: '',
  modelPrefixes: '',
  fallbackProfileIds: [],
  enabled: true,
  priority: '100',
}

function parseCsvInput(raw: string): string[] {
  return raw
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean)
}

function toggleItem<T extends string>(items: T[], target: T): T[] {
  return items.includes(target)
    ? items.filter((item) => item !== target)
    : [...items, target]
}

function formatDateTime(value?: string | null) {
  if (!value) return '-'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return '-'
  return date.toLocaleString()
}

function selectorFilterSummary(profile: RoutingProfile, t: ReturnType<typeof useTranslation>['t']) {
  return t('aiRoutingPage.profiles.summary', {
    plans: profile.selector.plan_types.length,
    modes: profile.selector.modes.length,
    authProviders: profile.selector.auth_providers.length,
    include: profile.selector.include_account_ids.length,
    exclude: profile.selector.exclude_account_ids.length,
  })
}

function triggerModeLabel(
  mode: AiRoutingTriggerMode,
  t: ReturnType<typeof useTranslation>['t'],
) {
  if (mode === 'scheduled_only') return t('aiRoutingPage.triggerModes.scheduledOnly')
  if (mode === 'event_only') return t('aiRoutingPage.triggerModes.eventOnly')
  return t('aiRoutingPage.triggerModes.hybrid')
}

function modeLabel(mode: UpstreamMode, t: ReturnType<typeof useTranslation>['t']) {
  if (mode === 'open_ai_api_key') return t('aiRoutingPage.modes.apiKey')
  if (mode === 'chat_gpt_session') return t('aiRoutingPage.modes.chatGptSession')
  return t('aiRoutingPage.modes.codexOauth')
}

function authProviderLabel(
  provider: UpstreamAuthProvider,
  t: ReturnType<typeof useTranslation>['t'],
) {
  if (provider === 'oauth_refresh_token') {
    return t('aiRoutingPage.authProviders.oauthRefreshToken')
  }
  return t('aiRoutingPage.authProviders.legacyBearer')
}

function settingsStatusVariant(
  settings: AiRoutingSettings | null,
): 'success' | 'warning' | 'secondary' | 'destructive' {
  if (!settings) return 'secondary'
  if (settings.kill_switch) return 'destructive'
  if (!settings.enabled) return 'secondary'
  if (!settings.auto_publish) return 'warning'
  return 'success'
}

function fallbackProfileSummary(
  policy: ModelRoutingPolicy,
  profileNames: Map<string, string>,
  t: ReturnType<typeof useTranslation>['t'],
) {
  if (policy.fallback_profile_ids.length === 0) {
    return t('aiRoutingPage.common.none')
  }
  return policy.fallback_profile_ids
    .map((profileId) => profileNames.get(profileId) ?? t('aiRoutingPage.common.deletedProfile'))
    .join(' -> ')
}

function createSettingsDraft(settings: AiRoutingSettings): SettingsFormState {
  return {
    enabled: settings.enabled,
    autoPublish: settings.auto_publish,
    plannerModelChain: settings.planner_model_chain.join(', '),
    triggerMode: settings.trigger_mode,
    killSwitch: settings.kill_switch,
  }
}

export default function AiRouting() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [profileDialogOpen, setProfileDialogOpen] = useState(false)
  const [policyDialogOpen, setPolicyDialogOpen] = useState(false)
  const [profileForm, setProfileForm] = useState<ProfileFormState>(DEFAULT_PROFILE_FORM)
  const [policyForm, setPolicyForm] = useState<PolicyFormState>(DEFAULT_POLICY_FORM)
  const [settingsDraftOverride, setSettingsDraftOverride] = useState<SettingsFormState | null>(null)

  const resolveErrorLabel = useCallback(
    (err: unknown, fallback: string) => localizeApiErrorDisplay(t, err, fallback).label,
    [t],
  )

  const { data, isLoading, isFetching } = useQuery({
    queryKey: ['adminAiRouting'],
    queryFn: async () => {
      const [profilesPayload, policiesPayload, settingsPayload, versionsPayload] =
        await Promise.all([
          aiRoutingApi.listProfiles(),
          aiRoutingApi.listPolicies(),
          aiRoutingApi.getSettings(),
          aiRoutingApi.listVersions(),
        ])
      return {
        profiles: profilesPayload.profiles ?? [],
        policies: policiesPayload.policies ?? [],
        settings: settingsPayload.settings,
        versions: versionsPayload.versions ?? [],
      }
    },
    staleTime: 30_000,
  })

  const profiles = useMemo(() => data?.profiles ?? [], [data?.profiles])
  const policies = useMemo(() => data?.policies ?? [], [data?.policies])
  const settings = data?.settings ?? null
  const versions = useMemo(() => data?.versions ?? [], [data?.versions])
  const settingsDraft = useMemo(
    () => settingsDraftOverride ?? (settings ? createSettingsDraft(settings) : null),
    [settingsDraftOverride, settings],
  )

  const updateSettingsDraft = useCallback(
    (updater: (current: SettingsFormState) => SettingsFormState) => {
      const base = settingsDraft ?? (settings ? createSettingsDraft(settings) : null)
      if (!base) {
        return
      }
      setSettingsDraftOverride(updater(base))
    },
    [settings, settingsDraft],
  )

  const profileNames = useMemo(() => {
    return new Map(profiles.map((profile) => [profile.id, profile.name]))
  }, [profiles])

  const saveSettingsMutation = useMutation({
    mutationFn: async () => {
      if (!settingsDraft) {
        throw new Error('settings_missing')
      }
      return aiRoutingApi.updateSettings({
        enabled: settingsDraft.enabled,
        auto_publish: settingsDraft.autoPublish,
        planner_model_chain: parseCsvInput(settingsDraft.plannerModelChain),
        trigger_mode: settingsDraft.triggerMode,
        kill_switch: settingsDraft.killSwitch,
      })
    },
    onSuccess: () => {
      setError(null)
      setNotice(t('aiRoutingPage.messages.settingsSaved'))
      setSettingsDraftOverride(null)
      queryClient.invalidateQueries({ queryKey: ['adminAiRouting'] })
    },
    onError: (err) => {
      setError(resolveErrorLabel(err, t('aiRoutingPage.messages.settingsSaveFailed')))
    },
  })

  const upsertProfileMutation = useMutation({
    mutationFn: () =>
      aiRoutingApi.upsertProfile({
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
      setError(null)
      setNotice(t('aiRoutingPage.messages.profileSaved', { name: profile.name }))
      setProfileDialogOpen(false)
      queryClient.invalidateQueries({ queryKey: ['adminAiRouting'] })
    },
    onError: (err) => {
      setError(resolveErrorLabel(err, t('aiRoutingPage.messages.profileSaveFailed')))
    },
  })

  const deleteProfileMutation = useMutation({
    mutationFn: (profileId: string) => aiRoutingApi.deleteProfile(profileId),
    onSuccess: () => {
      setError(null)
      setNotice(t('aiRoutingPage.messages.profileDeleted'))
      setProfileDialogOpen(false)
      queryClient.invalidateQueries({ queryKey: ['adminAiRouting'] })
    },
    onError: (err) => {
      setError(resolveErrorLabel(err, t('aiRoutingPage.messages.profileDeleteFailed')))
    },
  })

  const upsertPolicyMutation = useMutation({
    mutationFn: () =>
      aiRoutingApi.upsertPolicy({
        id: policyForm.id,
        name: policyForm.name.trim(),
        family: policyForm.family.trim(),
        exact_models: parseCsvInput(policyForm.exactModels),
        model_prefixes: parseCsvInput(policyForm.modelPrefixes),
        fallback_profile_ids: policyForm.fallbackProfileIds,
        enabled: policyForm.enabled,
        priority: Number(policyForm.priority) || 0,
      }),
    onSuccess: (policy) => {
      setError(null)
      setNotice(t('aiRoutingPage.messages.policySaved', { name: policy.name }))
      setPolicyDialogOpen(false)
      queryClient.invalidateQueries({ queryKey: ['adminAiRouting'] })
    },
    onError: (err) => {
      setError(resolveErrorLabel(err, t('aiRoutingPage.messages.policySaveFailed')))
    },
  })

  const deletePolicyMutation = useMutation({
    mutationFn: (policyId: string) => aiRoutingApi.deletePolicy(policyId),
    onSuccess: () => {
      setError(null)
      setNotice(t('aiRoutingPage.messages.policyDeleted'))
      setPolicyDialogOpen(false)
      queryClient.invalidateQueries({ queryKey: ['adminAiRouting'] })
    },
    onError: (err) => {
      setError(resolveErrorLabel(err, t('aiRoutingPage.messages.policyDeleteFailed')))
    },
  })

  const openCreateProfileDialog = () => {
    setProfileForm(DEFAULT_PROFILE_FORM)
    setProfileDialogOpen(true)
  }

  const openEditProfileDialog = (profile: RoutingProfile) => {
    setProfileForm({
      id: profile.id,
      name: profile.name,
      description: profile.description ?? '',
      enabled: profile.enabled,
      priority: String(profile.priority),
      planTypes: profile.selector.plan_types.join(', '),
      modes: profile.selector.modes,
      authProviders: profile.selector.auth_providers,
      includeAccountIds: profile.selector.include_account_ids.join(', '),
      excludeAccountIds: profile.selector.exclude_account_ids.join(', '),
    })
    setProfileDialogOpen(true)
  }

  const openCreatePolicyDialog = () => {
    setPolicyForm(DEFAULT_POLICY_FORM)
    setPolicyDialogOpen(true)
  }

  const openEditPolicyDialog = (policy: ModelRoutingPolicy) => {
    setPolicyForm({
      id: policy.id,
      name: policy.name,
      family: policy.family,
      exactModels: policy.exact_models.join(', '),
      modelPrefixes: policy.model_prefixes.join(', '),
      fallbackProfileIds: policy.fallback_profile_ids,
      enabled: policy.enabled,
      priority: String(policy.priority),
    })
    setPolicyDialogOpen(true)
  }

  const modeOptions: UpstreamMode[] = ['codex_oauth', 'chat_gpt_session', 'open_ai_api_key']
  const authProviderOptions: UpstreamAuthProvider[] = ['oauth_refresh_token', 'legacy_bearer']
  const triggerModeOptions: AiRoutingTriggerMode[] = ['hybrid', 'scheduled_only', 'event_only']

  return (
    <div className="flex-1 overflow-y-auto p-8">
      <LoadingOverlay show={isLoading} title={t('common.loading')} />

      <div className="mb-8 flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
        <div className="space-y-2">
          <h2 className="text-3xl font-semibold tracking-tight">{t('aiRoutingPage.title')}</h2>
          <p className="max-w-3xl text-sm text-muted-foreground">
            {t('aiRoutingPage.subtitle')}
          </p>
        </div>
        <div className="flex flex-wrap gap-3">
          <Button
            variant="outline"
            onClick={() => queryClient.invalidateQueries({ queryKey: ['adminAiRouting'] })}
            disabled={isFetching}
          >
            <RotateCw className={`mr-2 h-4 w-4 ${isFetching ? 'animate-spin' : ''}`} />
            {t('aiRoutingPage.actions.refresh')}
          </Button>
          <Button variant="outline" onClick={openCreateProfileDialog}>
            {t('aiRoutingPage.actions.createProfile')}
          </Button>
          <Button onClick={openCreatePolicyDialog}>
            {t('aiRoutingPage.actions.createPolicy')}
          </Button>
        </div>
      </div>

      {error ? (
        <div className="mb-4 rounded-lg border border-destructive/30 bg-destructive/5 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      ) : null}
      {notice ? (
        <div className="mb-4 rounded-lg border border-success/30 bg-success-muted px-4 py-3 text-sm text-success-foreground">
          {notice}
        </div>
      ) : null}

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.3fr)_minmax(320px,0.7fr)]">
        <Card className="relative overflow-hidden border-border/60">
          <CardHeader>
            <CardTitle>{t('aiRoutingPage.settings.title')}</CardTitle>
            <CardDescription>{t('aiRoutingPage.settings.description')}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant={settingsStatusVariant(settings)}>
                {settings?.kill_switch
                  ? t('aiRoutingPage.status.killSwitchOn')
                  : settings?.enabled
                    ? t('aiRoutingPage.status.enabled')
                    : t('aiRoutingPage.status.disabled')}
              </Badge>
              <Badge variant={settings?.auto_publish ? 'info' : 'secondary'}>
                {settings?.auto_publish
                  ? t('aiRoutingPage.status.autoPublishOn')
                  : t('aiRoutingPage.status.autoPublishOff')}
              </Badge>
              <Badge variant="outline">
                {t('aiRoutingPage.settings.updatedAt', {
                  value: formatDateTime(settings?.updated_at),
                })}
              </Badge>
            </div>

            <div className="grid gap-4 md:grid-cols-2">
              <label className={POOL_SECTION_CLASS_NAME}>
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="font-medium">{t('aiRoutingPage.settings.enabled')}</div>
                    <div className="text-sm text-muted-foreground">
                      {t('aiRoutingPage.settings.enabledHint')}
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
              </label>

              <label className={POOL_SECTION_CLASS_NAME}>
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="font-medium">{t('aiRoutingPage.settings.autoPublish')}</div>
                    <div className="text-sm text-muted-foreground">
                      {t('aiRoutingPage.settings.autoPublishHint')}
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
              </label>

              <label className={POOL_SECTION_CLASS_NAME}>
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <div className="font-medium">{t('aiRoutingPage.settings.killSwitch')}</div>
                    <div className="text-sm text-muted-foreground">
                      {t('aiRoutingPage.settings.killSwitchHint')}
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
              </label>

              <div className={POOL_SECTION_CLASS_NAME}>
                <div className="mb-2 font-medium">{t('aiRoutingPage.settings.triggerMode')}</div>
                <Select
                  value={settingsDraft?.triggerMode ?? 'hybrid'}
                  onValueChange={(value) =>
                    updateSettingsDraft((prev) => ({
                      ...prev,
                      triggerMode: value as AiRoutingTriggerMode,
                    }))
                  }
                >
                  <SelectTrigger className="w-full" aria-label={t('aiRoutingPage.settings.triggerMode')}>
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
              </div>
            </div>

            <div className={POOL_SECTION_CLASS_NAME}>
              <label className="mb-2 block font-medium">
                {t('aiRoutingPage.settings.plannerModelChain')}
              </label>
              <Textarea
                value={settingsDraft?.plannerModelChain ?? ''}
                onChange={(event) =>
                  updateSettingsDraft((prev) => ({
                    ...prev,
                    plannerModelChain: event.target.value,
                  }))
                }
                rows={4}
                placeholder={t('aiRoutingPage.settings.plannerModelChainPlaceholder')}
              />
              <p className="mt-2 text-xs text-muted-foreground">
                {t('aiRoutingPage.settings.plannerModelChainHint')}
              </p>
            </div>

            <div className="flex justify-end">
              <Button onClick={() => saveSettingsMutation.mutate()} disabled={saveSettingsMutation.isPending}>
                <Save className="mr-2 h-4 w-4" />
                {t('aiRoutingPage.actions.saveSettings')}
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card className="border-border/60">
          <CardHeader>
            <CardTitle>{t('aiRoutingPage.versions.title')}</CardTitle>
            <CardDescription>{t('aiRoutingPage.versions.description')}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {versions.length === 0 ? (
              <div className="rounded-lg border border-dashed p-4 text-sm text-muted-foreground">
                {t('aiRoutingPage.versions.empty')}
              </div>
            ) : (
              versions.map((version) => {
                const reason = version.reason || version.compiled_plan.trigger_reason
                return (
                  <div key={version.id} className={POOL_SECTION_CLASS_NAME}>
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <div className="font-medium">{formatDateTime(version.published_at)}</div>
                      <Badge variant="outline">{version.id.slice(0, 8)}</Badge>
                    </div>
                    <div className="text-sm text-muted-foreground">
                      {reason?.trim() || t('aiRoutingPage.versions.noReason')}
                    </div>
                    <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
                      <span>
                        {t('aiRoutingPage.versions.defaultSegments', {
                          count: version.compiled_plan.default_route.length,
                        })}
                      </span>
                      <span>
                        {t('aiRoutingPage.versions.policyCount', {
                          count: version.compiled_plan.policies.length,
                        })}
                      </span>
                    </div>
                  </div>
                )
              })
            )}
          </CardContent>
        </Card>
      </div>

      <div className="mt-6 grid gap-6 xl:grid-cols-2">
        <Card className="border-border/60">
          <CardHeader>
            <CardTitle>{t('aiRoutingPage.profiles.title')}</CardTitle>
            <CardDescription>{t('aiRoutingPage.profiles.description')}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {profiles.length === 0 ? (
              <div className="rounded-lg border border-dashed p-4 text-sm text-muted-foreground">
                {t('aiRoutingPage.profiles.empty')}
              </div>
            ) : (
              profiles.map((profile) => (
                <div key={profile.id} className={POOL_SECTION_CLASS_NAME}>
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="space-y-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <div className="font-medium">{profile.name}</div>
                        <Badge variant={profile.enabled ? 'success' : 'secondary'}>
                          {profile.enabled
                            ? t('aiRoutingPage.status.enabled')
                            : t('aiRoutingPage.status.disabled')}
                        </Badge>
                        <Badge variant="outline">
                          {t('aiRoutingPage.common.priority', { value: profile.priority })}
                        </Badge>
                      </div>
                      {profile.description ? (
                        <div className="text-sm text-muted-foreground">{profile.description}</div>
                      ) : null}
                      <div className="text-sm text-muted-foreground">
                        {selectorFilterSummary(profile, t)}
                      </div>
                    </div>
                    <div className="flex gap-2">
                      <Button variant="outline" size="sm" onClick={() => openEditProfileDialog(profile)}>
                        <SquarePen className="mr-2 h-4 w-4" />
                        {t('aiRoutingPage.actions.edit')}
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => deleteProfileMutation.mutate(profile.id)}
                      >
                        <Trash2 className="mr-2 h-4 w-4" />
                        {t('aiRoutingPage.actions.delete')}
                      </Button>
                    </div>
                  </div>
                  <div className="flex flex-wrap gap-2 text-xs text-muted-foreground">
                    {profile.selector.modes.length > 0 ? (
                      <span>
                        {profile.selector.modes
                          .map((mode) => modeLabel(mode, t))
                          .join(' / ')}
                      </span>
                    ) : (
                      <span>{t('aiRoutingPage.profiles.anyMode')}</span>
                    )}
                    {profile.selector.auth_providers.length > 0 ? (
                      <span>
                        {profile.selector.auth_providers
                          .map((provider) => authProviderLabel(provider, t))
                          .join(' / ')}
                      </span>
                    ) : null}
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>

        <Card className="border-border/60">
          <CardHeader>
            <CardTitle>{t('aiRoutingPage.policies.title')}</CardTitle>
            <CardDescription>{t('aiRoutingPage.policies.description')}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {policies.length === 0 ? (
              <div className="rounded-lg border border-dashed p-4 text-sm text-muted-foreground">
                {t('aiRoutingPage.policies.empty')}
              </div>
            ) : (
              policies.map((policy) => (
                <div key={policy.id} className={POOL_SECTION_CLASS_NAME}>
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="space-y-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <div className="font-medium">{policy.name}</div>
                        <Badge variant={policy.enabled ? 'success' : 'secondary'}>
                          {policy.enabled
                            ? t('aiRoutingPage.status.enabled')
                            : t('aiRoutingPage.status.disabled')}
                        </Badge>
                        <Badge variant="outline">
                          {t('aiRoutingPage.common.priority', { value: policy.priority })}
                        </Badge>
                        <Badge variant="info">{policy.family}</Badge>
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {t('aiRoutingPage.policies.summary', {
                          exact: policy.exact_models.length,
                          prefixes: policy.model_prefixes.length,
                          fallbacks: policy.fallback_profile_ids.length,
                        })}
                      </div>
                      <div className="text-sm text-muted-foreground">
                        {t('aiRoutingPage.policies.fallbackChain', {
                          value: fallbackProfileSummary(policy, profileNames, t),
                        })}
                      </div>
                    </div>
                    <div className="flex gap-2">
                      <Button variant="outline" size="sm" onClick={() => openEditPolicyDialog(policy)}>
                        <SquarePen className="mr-2 h-4 w-4" />
                        {t('aiRoutingPage.actions.edit')}
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => deletePolicyMutation.mutate(policy.id)}
                      >
                        <Trash2 className="mr-2 h-4 w-4" />
                        {t('aiRoutingPage.actions.delete')}
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
                        {t('aiRoutingPage.policies.moreExactModels', {
                          count: policy.exact_models.length - 4,
                        })}
                      </Badge>
                    ) : null}
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </div>

      <Dialog open={profileDialogOpen} onOpenChange={setProfileDialogOpen}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>
              {profileForm.id
                ? t('aiRoutingPage.dialogs.editProfile')
                : t('aiRoutingPage.dialogs.createProfile')}
            </DialogTitle>
            <DialogDescription>{t('aiRoutingPage.dialogs.profileDescription')}</DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2 md:col-span-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.name')}</label>
              <Input
                value={profileForm.name}
                onChange={(event) =>
                  setProfileForm((prev) => ({ ...prev, name: event.target.value }))
                }
              />
            </div>
            <div className="space-y-2 md:col-span-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.description')}</label>
              <Textarea
                value={profileForm.description}
                onChange={(event) =>
                  setProfileForm((prev) => ({ ...prev, description: event.target.value }))
                }
                rows={3}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.priority')}</label>
              <Input
                type="number"
                value={profileForm.priority}
                onChange={(event) =>
                  setProfileForm((prev) => ({ ...prev, priority: event.target.value }))
                }
              />
            </div>
            <label className={POOL_SECTION_CLASS_NAME}>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className="font-medium">{t('aiRoutingPage.form.enabled')}</div>
                  <div className="text-sm text-muted-foreground">
                    {t('aiRoutingPage.form.enabledHint')}
                  </div>
                </div>
                <Checkbox
                  checked={profileForm.enabled}
                  onCheckedChange={(checked) =>
                    setProfileForm((prev) => ({ ...prev, enabled: checked === true }))
                  }
                />
              </div>
            </label>
            <div className="space-y-2 md:col-span-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.planTypes')}</label>
              <Input
                value={profileForm.planTypes}
                onChange={(event) =>
                  setProfileForm((prev) => ({ ...prev, planTypes: event.target.value }))
                }
                placeholder={t('aiRoutingPage.form.planTypesPlaceholder')}
              />
            </div>
            <div className="space-y-2">
              <div className="text-sm font-medium">{t('aiRoutingPage.form.modes')}</div>
              <div className="space-y-2 rounded-lg border p-3">
                {modeOptions.map((mode) => (
                  <label key={mode} className="flex items-center justify-between gap-3 text-sm">
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
              </div>
            </div>
            <div className="space-y-2">
              <div className="text-sm font-medium">{t('aiRoutingPage.form.authProviders')}</div>
              <div className="space-y-2 rounded-lg border p-3">
                {authProviderOptions.map((provider) => (
                  <label key={provider} className="flex items-center justify-between gap-3 text-sm">
                    <span>{authProviderLabel(provider, t)}</span>
                    <Checkbox
                      checked={profileForm.authProviders.includes(provider)}
                      onCheckedChange={() =>
                        setProfileForm((prev) => ({
                          ...prev,
                          authProviders: toggleItem(prev.authProviders, provider),
                        }))
                      }
                    />
                  </label>
                ))}
              </div>
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.includeAccounts')}</label>
              <Textarea
                value={profileForm.includeAccountIds}
                onChange={(event) =>
                  setProfileForm((prev) => ({
                    ...prev,
                    includeAccountIds: event.target.value,
                  }))
                }
                rows={4}
                placeholder={t('aiRoutingPage.form.includeAccountsPlaceholder')}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.excludeAccounts')}</label>
              <Textarea
                value={profileForm.excludeAccountIds}
                onChange={(event) =>
                  setProfileForm((prev) => ({
                    ...prev,
                    excludeAccountIds: event.target.value,
                  }))
                }
                rows={4}
                placeholder={t('aiRoutingPage.form.excludeAccountsPlaceholder')}
              />
            </div>
          </div>
          <div className="flex items-center justify-between gap-3">
            {profileForm.id ? (
              <Button
                variant="outline"
                onClick={() => deleteProfileMutation.mutate(profileForm.id!)}
                disabled={deleteProfileMutation.isPending}
              >
                <Trash2 className="mr-2 h-4 w-4" />
                {t('aiRoutingPage.actions.deleteProfile')}
              </Button>
            ) : (
              <div />
            )}
            <Button onClick={() => upsertProfileMutation.mutate()} disabled={upsertProfileMutation.isPending}>
              <Save className="mr-2 h-4 w-4" />
              {t('aiRoutingPage.actions.saveProfile')}
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      <Dialog open={policyDialogOpen} onOpenChange={setPolicyDialogOpen}>
        <DialogContent className="max-w-3xl">
          <DialogHeader>
            <DialogTitle>
              {policyForm.id
                ? t('aiRoutingPage.dialogs.editPolicy')
                : t('aiRoutingPage.dialogs.createPolicy')}
            </DialogTitle>
            <DialogDescription>{t('aiRoutingPage.dialogs.policyDescription')}</DialogDescription>
          </DialogHeader>
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.name')}</label>
              <Input
                value={policyForm.name}
                onChange={(event) =>
                  setPolicyForm((prev) => ({ ...prev, name: event.target.value }))
                }
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.family')}</label>
              <Input
                value={policyForm.family}
                onChange={(event) =>
                  setPolicyForm((prev) => ({ ...prev, family: event.target.value }))
                }
                placeholder={t('aiRoutingPage.form.familyPlaceholder')}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.priority')}</label>
              <Input
                type="number"
                value={policyForm.priority}
                onChange={(event) =>
                  setPolicyForm((prev) => ({ ...prev, priority: event.target.value }))
                }
              />
            </div>
            <label className={POOL_SECTION_CLASS_NAME}>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <div className="font-medium">{t('aiRoutingPage.form.enabled')}</div>
                  <div className="text-sm text-muted-foreground">
                    {t('aiRoutingPage.form.policyEnabledHint')}
                  </div>
                </div>
                <Checkbox
                  checked={policyForm.enabled}
                  onCheckedChange={(checked) =>
                    setPolicyForm((prev) => ({ ...prev, enabled: checked === true }))
                  }
                />
              </div>
            </label>
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.exactModels')}</label>
              <Textarea
                value={policyForm.exactModels}
                onChange={(event) =>
                  setPolicyForm((prev) => ({ ...prev, exactModels: event.target.value }))
                }
                rows={4}
                placeholder={t('aiRoutingPage.form.exactModelsPlaceholder')}
              />
            </div>
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('aiRoutingPage.form.modelPrefixes')}</label>
              <Textarea
                value={policyForm.modelPrefixes}
                onChange={(event) =>
                  setPolicyForm((prev) => ({ ...prev, modelPrefixes: event.target.value }))
                }
                rows={4}
                placeholder={t('aiRoutingPage.form.modelPrefixesPlaceholder')}
              />
            </div>
            <div className="space-y-2 md:col-span-2">
              <div className="text-sm font-medium">{t('aiRoutingPage.form.fallbackProfiles')}</div>
              <div className="space-y-2 rounded-lg border p-3">
                {profiles.length === 0 ? (
                  <div className="text-sm text-muted-foreground">
                    {t('aiRoutingPage.form.noProfilesAvailable')}
                  </div>
                ) : (
                  profiles.map((profile) => (
                    <label key={profile.id} className="flex items-center justify-between gap-3 text-sm">
                      <span>{profile.name}</span>
                      <Checkbox
                        checked={policyForm.fallbackProfileIds.includes(profile.id)}
                        onCheckedChange={() =>
                          setPolicyForm((prev) => ({
                            ...prev,
                            fallbackProfileIds: toggleItem(prev.fallbackProfileIds, profile.id),
                          }))
                        }
                      />
                    </label>
                  ))
                )}
              </div>
            </div>
          </div>
          <div className="flex items-center justify-between gap-3">
            {policyForm.id ? (
              <Button
                variant="outline"
                onClick={() => deletePolicyMutation.mutate(policyForm.id!)}
                disabled={deletePolicyMutation.isPending}
              >
                <Trash2 className="mr-2 h-4 w-4" />
                {t('aiRoutingPage.actions.deletePolicy')}
              </Button>
            ) : (
              <div />
            )}
            <Button onClick={() => upsertPolicyMutation.mutate()} disabled={upsertPolicyMutation.isPending}>
              <Save className="mr-2 h-4 w-4" />
              {t('aiRoutingPage.actions.savePolicy')}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  )
}
