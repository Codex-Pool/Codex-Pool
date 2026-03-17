import { useCallback, useMemo, useState } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronDown, ChevronUp, RotateCw, SquarePen, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import {
  groupsApi,
  type ApiKeyGroupAdminListResponse,
  type ApiKeyGroupCatalogItem,
  type ApiKeyGroupItem,
} from '@/api/groups'
import { localizeApiErrorDisplay } from '@/api/errorI18n'
import { PageIntro } from '@/components/layout/page-archetypes'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
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
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { Textarea } from '@/components/ui/textarea'

function formatMultiplier(ppm?: number | null) {
  if (typeof ppm !== 'number') return '-'
  return `×${(ppm / 1_000_000).toFixed(2)}`
}

function formatMicrocredits(value?: number | null) {
  if (typeof value !== 'number') return '-'
  return (value / 1_000_000).toFixed(4)
}

function groupStatusVariant(group: ApiKeyGroupItem): 'success' | 'warning' | 'secondary' | 'destructive' {
  if (group.deleted_at) return 'destructive'
  if (!group.enabled) return 'secondary'
  if (group.is_default) return 'success'
  return 'warning'
}

function pricingLineForModel(model: ApiKeyGroupItem['models'][number]) {
  const formula = `in ${formatMicrocredits(model.formula_input_price_microcredits)} · cached ${formatMicrocredits(model.formula_cached_input_price_microcredits)} · out ${formatMicrocredits(model.formula_output_price_microcredits)}`
  const finalPricing = `in ${formatMicrocredits(model.final_input_price_microcredits)} · cached ${formatMicrocredits(model.final_cached_input_price_microcredits)} · out ${formatMicrocredits(model.final_output_price_microcredits)}`
  return { formula, finalPricing }
}

const tableSurfaceClassName =
  'border border-border/60 bg-background/[0.5] shadow-none backdrop-blur-[2px]'
const editorSectionClassName =
  'space-y-4 rounded-[1rem] border border-border/60 bg-background/[0.55] p-4'

export default function Groups() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [editorOpen, setEditorOpen] = useState(false)
  const [editingGroupId, setEditingGroupId] = useState<string | null>(null)
  const [selectedModel, setSelectedModel] = useState<string>('')
  const [mobilePreviewExpanded, setMobilePreviewExpanded] = useState(false)
  const [groupForm, setGroupForm] = useState({
    id: '',
    name: '',
    description: '',
    enabled: true,
    is_default: false,
    allow_all_models: false,
    input_multiplier_ppm: '1000000',
    cached_input_multiplier_ppm: '1000000',
    output_multiplier_ppm: '1000000',
  })
  const [policyForm, setPolicyForm] = useState({
    enabled: true,
    input_multiplier_ppm: '1000000',
    cached_input_multiplier_ppm: '1000000',
    output_multiplier_ppm: '1000000',
    input_price_microcredits: '',
    cached_input_price_microcredits: '',
    output_price_microcredits: '',
  })

  const { data, isLoading, isFetching } = useQuery<ApiKeyGroupAdminListResponse>({
    queryKey: ['adminApiKeyGroups'],
    queryFn: groupsApi.adminList,
    staleTime: 30_000,
  })

  const groups = useMemo(() => data?.groups ?? [], [data])
  const catalog = useMemo(() => data?.catalog ?? [], [data])
  const currentGroup = useMemo(
    () => groups.find((item) => item.id === editingGroupId) ?? null,
    [editingGroupId, groups],
  )

  const selectedPolicy = useMemo(() => {
    if (!currentGroup || !selectedModel) return null
    return currentGroup.policies.find((item) => item.model === selectedModel) ?? null
  }, [currentGroup, selectedModel])

  const selectedCatalogModel = useMemo(() => {
    if (!selectedModel) return null
    return catalog.find((item) => item.model === selectedModel) ?? null
  }, [catalog, selectedModel])

  const resolveError = useCallback(
    (err: unknown, fallback: string) => localizeApiErrorDisplay(t, err, fallback).label,
    [t],
  )

  const openEditor = useCallback(
    (group?: ApiKeyGroupItem | null) => {
      const target = group ?? null
      setEditingGroupId(target?.id ?? null)
      setGroupForm({
        id: target?.id ?? '',
        name: target?.name ?? '',
        description: target?.description ?? '',
        enabled: target?.enabled ?? true,
        is_default: target?.is_default ?? false,
        allow_all_models: target?.allow_all_models ?? false,
        input_multiplier_ppm: String(target?.input_multiplier_ppm ?? 1_000_000),
        cached_input_multiplier_ppm: String(target?.cached_input_multiplier_ppm ?? 1_000_000),
        output_multiplier_ppm: String(target?.output_multiplier_ppm ?? 1_000_000),
      })
      const firstModel = target?.policies[0]?.model ?? catalog[0]?.model ?? ''
      setSelectedModel(firstModel)
      setMobilePreviewExpanded(false)
      const firstPolicy = target?.policies.find((item) => item.model === firstModel) ?? null
      setPolicyForm({
        enabled: firstPolicy?.enabled ?? true,
        input_multiplier_ppm: String(firstPolicy?.input_multiplier_ppm ?? 1_000_000),
        cached_input_multiplier_ppm: String(firstPolicy?.cached_input_multiplier_ppm ?? 1_000_000),
        output_multiplier_ppm: String(firstPolicy?.output_multiplier_ppm ?? 1_000_000),
        input_price_microcredits: firstPolicy?.input_price_microcredits != null ? String(firstPolicy.input_price_microcredits) : '',
        cached_input_price_microcredits: firstPolicy?.cached_input_price_microcredits != null ? String(firstPolicy.cached_input_price_microcredits) : '',
        output_price_microcredits: firstPolicy?.output_price_microcredits != null ? String(firstPolicy.output_price_microcredits) : '',
      })
      setError(null)
      setNotice(null)
      setEditorOpen(true)
    },
    [catalog],
  )

  const upsertGroupMutation = useMutation({
    mutationFn: async () =>
      groupsApi.adminUpsert({
        id: groupForm.id || undefined,
        name: groupForm.name,
        description: groupForm.description.trim() || undefined,
        enabled: groupForm.enabled,
        is_default: groupForm.is_default,
        allow_all_models: groupForm.allow_all_models,
        input_multiplier_ppm: Number(groupForm.input_multiplier_ppm),
        cached_input_multiplier_ppm: Number(groupForm.cached_input_multiplier_ppm),
        output_multiplier_ppm: Number(groupForm.output_multiplier_ppm),
      }),
    onSuccess: (response) => {
      setError(null)
      setNotice(t('groupsPage.messages.groupSaved', { defaultValue: 'Group saved: {{name}}', name: response.name }))
      setEditingGroupId(response.id)
      setGroupForm((prev) => ({ ...prev, id: response.id }))
      queryClient.invalidateQueries({ queryKey: ['adminApiKeyGroups'] })
    },
    onError: (err) => {
      setError(resolveError(err, t('groupsPage.messages.groupSaveFailed', { defaultValue: 'Failed to save group.' })))
    },
  })

  const deleteGroupMutation = useMutation({
    mutationFn: (groupId: string) => groupsApi.adminDelete(groupId),
    onSuccess: () => {
      setError(null)
      setNotice(t('groupsPage.messages.groupDeleted', { defaultValue: 'Group deleted.' }))
      setEditorOpen(false)
      setEditingGroupId(null)
      queryClient.invalidateQueries({ queryKey: ['adminApiKeyGroups'] })
    },
    onError: (err) => {
      setError(resolveError(err, t('groupsPage.messages.groupDeleteFailed', { defaultValue: 'Failed to delete group.' })))
    },
  })

  const upsertPolicyMutation = useMutation({
    mutationFn: async () => {
      const groupId = editingGroupId || groupForm.id
      if (!groupId) {
        throw new Error('group_not_saved')
      }
      if (!selectedModel) {
        throw new Error('model_required')
      }
      return groupsApi.adminUpsertPolicy({
        group_id: groupId,
        model: selectedModel,
        enabled: policyForm.enabled,
        input_multiplier_ppm: Number(policyForm.input_multiplier_ppm),
        cached_input_multiplier_ppm: Number(policyForm.cached_input_multiplier_ppm),
        output_multiplier_ppm: Number(policyForm.output_multiplier_ppm),
        input_price_microcredits: policyForm.input_price_microcredits.trim() ? Number(policyForm.input_price_microcredits) : undefined,
        cached_input_price_microcredits: policyForm.cached_input_price_microcredits.trim() ? Number(policyForm.cached_input_price_microcredits) : undefined,
        output_price_microcredits: policyForm.output_price_microcredits.trim() ? Number(policyForm.output_price_microcredits) : undefined,
      })
    },
    onSuccess: () => {
      setError(null)
      setNotice(t('groupsPage.messages.policySaved', { defaultValue: 'Model policy saved.' }))
      queryClient.invalidateQueries({ queryKey: ['adminApiKeyGroups'] })
    },
    onError: (err) => {
      setError(resolveError(err, t('groupsPage.messages.policySaveFailed', { defaultValue: 'Failed to save model policy.' })))
    },
  })

  const deletePolicyMutation = useMutation({
    mutationFn: (policyId: string) => groupsApi.adminDeletePolicy(policyId),
    onSuccess: () => {
      setError(null)
      setNotice(t('groupsPage.messages.policyDeleted', { defaultValue: 'Model policy deleted.' }))
      queryClient.invalidateQueries({ queryKey: ['adminApiKeyGroups'] })
    },
    onError: (err) => {
      setError(resolveError(err, t('groupsPage.messages.policyDeleteFailed', { defaultValue: 'Failed to delete model policy.' })))
    },
  })

  const columns = useMemo<ColumnDef<ApiKeyGroupItem>[]>(
    () => [
      {
        id: 'name',
        header: t('groupsPage.columns.name', { defaultValue: 'Group' }),
        accessorFn: (row) => row.name.toLowerCase(),
        cell: ({ row }) => (
          <div className="space-y-1">
            <div className="font-medium">{row.original.name}</div>
            <div className="text-xs text-muted-foreground">{row.original.description || '-'}</div>
          </div>
        ),
      },
      {
        id: 'status',
        header: t('groupsPage.columns.status', { defaultValue: 'Status' }),
        accessorFn: (row) => `${row.deleted_at ? 'deleted' : row.enabled ? 'enabled' : 'disabled'} ${row.is_default ? 'default' : ''}`,
        cell: ({ row }) => (
          <div className="flex flex-wrap gap-2">
            <Badge variant={groupStatusVariant(row.original)}>
              {row.original.deleted_at
                ? t('groupsPage.status.deleted', { defaultValue: 'Deleted' })
                : row.original.enabled
                  ? t('groupsPage.status.enabled', { defaultValue: 'Enabled' })
                  : t('groupsPage.status.disabled', { defaultValue: 'Disabled' })}
            </Badge>
            {row.original.is_default ? (
              <Badge variant="info">{t('groupsPage.status.default', { defaultValue: 'Default' })}</Badge>
            ) : null}
          </div>
        ),
      },
      {
        id: 'multipliers',
        header: t('groupsPage.columns.multipliers', { defaultValue: 'Multipliers' }),
        accessorFn: (row) => `${row.input_multiplier_ppm}-${row.cached_input_multiplier_ppm}-${row.output_multiplier_ppm}`,
        cell: ({ row }) => (
          <div className="text-xs text-muted-foreground">
            in {formatMultiplier(row.original.input_multiplier_ppm)} · cached {formatMultiplier(row.original.cached_input_multiplier_ppm)} · out {formatMultiplier(row.original.output_multiplier_ppm)}
          </div>
        ),
      },
      {
        id: 'usage',
        header: t('groupsPage.columns.usage', { defaultValue: 'Usage' }),
        accessorFn: (row) => `${row.api_key_count}-${row.model_count}`,
        cell: ({ row }) => (
          <div className="text-xs text-muted-foreground">
            {t('groupsPage.columns.apiKeysCount', { defaultValue: 'API Keys {{count}}', count: row.original.api_key_count })}
            {' · '}
            {t('groupsPage.columns.modelsCount', { defaultValue: 'Models {{count}}', count: row.original.model_count })}
          </div>
        ),
      },
      {
        id: 'actions',
        header: t('groupsPage.columns.actions', { defaultValue: 'Actions' }),
        cell: ({ row }) => (
          <div className="flex gap-2">
            <Button type="button" size="sm" variant="outline" onClick={() => openEditor(row.original)}>
              <SquarePen className="mr-2 h-4 w-4" />
              {t('common.edit')}
            </Button>
            <Button
              type="button"
              size="sm"
              variant="destructive"
              onClick={() => deleteGroupMutation.mutate(row.original.id)}
              disabled={deleteGroupMutation.isPending || row.original.is_default}
            >
              <Trash2 className="mr-2 h-4 w-4" />
              {t('common.delete')}
            </Button>
          </div>
        ),
      },
    ],
    [deleteGroupMutation, openEditor, t],
  )

  const handleSelectedModelChange = (model: string) => {
    setSelectedModel(model)
    const policy = currentGroup?.policies.find((item) => item.model === model) ?? null
    setPolicyForm({
      enabled: policy?.enabled ?? true,
      input_multiplier_ppm: String(policy?.input_multiplier_ppm ?? 1_000_000),
      cached_input_multiplier_ppm: String(policy?.cached_input_multiplier_ppm ?? 1_000_000),
      output_multiplier_ppm: String(policy?.output_multiplier_ppm ?? 1_000_000),
      input_price_microcredits: policy?.input_price_microcredits != null ? String(policy.input_price_microcredits) : '',
      cached_input_price_microcredits: policy?.cached_input_price_microcredits != null ? String(policy.cached_input_price_microcredits) : '',
      output_price_microcredits: policy?.output_price_microcredits != null ? String(policy.output_price_microcredits) : '',
    })
  }

  const mobilePreviewModels = useMemo(() => {
    const models = currentGroup?.models ?? []
    return mobilePreviewExpanded ? models : models.slice(0, 3)
  }, [currentGroup, mobilePreviewExpanded])

  return (
    <div className="flex-1 space-y-6 p-4 sm:p-6 lg:space-y-7 lg:p-8">
      <PageIntro
        archetype="workspace"
        title={t('groupsPage.title', { defaultValue: 'Group Management' })}
        description={t('groupsPage.subtitle', {
          defaultValue: 'Manage API key groups, model allowlists, multipliers, and group-level absolute prices.',
        })}
        meta={(
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
            <span>{t('accounts.filters.total', { count: groups.length, defaultValue: '{{count}} matched' })}</span>
            <span className="text-border">/</span>
            <span>{t('groupsPage.actions.create', { defaultValue: 'Create group' })}</span>
          </div>
        )}
        actions={(
          <Button type="button" className="w-full sm:w-auto" onClick={() => openEditor(null)}>
            {t('groupsPage.actions.create', { defaultValue: 'Create group' })}
          </Button>
        )}
      />

      {error ? (
        <div className="rounded-[0.95rem] border border-destructive/35 bg-destructive/5 px-3.5 py-2.5 text-sm text-destructive">
          {error}
        </div>
      ) : null}
      {notice ? (
        <div className="rounded-[0.95rem] border border-primary/25 bg-primary/5 px-3.5 py-2.5 text-sm text-primary">
          {notice}
        </div>
      ) : null}

      <section className="space-y-4 border-t border-border/70 pt-4">
        <div className="flex flex-wrap items-center justify-between gap-3 text-xs text-muted-foreground">
          <span>{t('accounts.filters.total', { count: groups.length, defaultValue: '{{count}} matched' })}</span>
          {isFetching && !isLoading ? <span>{t('common.loading')}</span> : null}
        </div>

        <div className="relative">
          <LoadingOverlay
            show={isFetching && !isLoading}
            title={t('common.loading')}
            size="compact"
          />
          {isLoading ? (
            <p className="text-sm text-muted-foreground">{t('common.loading')}</p>
          ) : (
            <StandardDataTable
              columns={columns}
              data={groups}
              defaultPageSize={20}
              pageSizeOptions={[20, 50, 100]}
              density="compact"
              className={tableSurfaceClassName}
              searchPlaceholder={t('groupsPage.searchPlaceholder', { defaultValue: 'Search groups by name, description or status' })}
              emptyText={t('groupsPage.empty', { defaultValue: 'No groups yet' })}
            />
          )}
        </div>
      </section>

      <Dialog open={editorOpen} onOpenChange={setEditorOpen}>
        <DialogContent className="h-[calc(100dvh-1rem)] max-h-[calc(100dvh-1rem)] w-[calc(100vw-1rem)] max-w-[calc(100vw-1rem)] overflow-hidden border border-border/70 bg-background/98 p-0 sm:h-auto sm:max-h-[92vh] sm:max-w-[min(96vw,1400px)]">
          <div className="flex h-full flex-col">
            <DialogHeader className="shrink-0 border-b border-border/70 bg-muted/[0.16] px-4 py-4 text-left sm:px-6">
              <DialogTitle>
                {groupForm.id
                  ? t('groupsPage.editor.editTitle', { defaultValue: 'Edit group' })
                  : t('groupsPage.editor.createTitle', { defaultValue: 'Create group' })}
              </DialogTitle>
              <DialogDescription>
                {t('groupsPage.editor.description', { defaultValue: 'Configure group-wide multipliers and per-model pricing overrides.' })}
              </DialogDescription>
            </DialogHeader>

            <div className="flex-1 overflow-y-auto px-4 pb-4 pt-4 sm:px-6 sm:pb-6">
              <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)] lg:gap-6">
                <div className={editorSectionClassName}>
              <div className="space-y-2">
                <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.form.name', { defaultValue: 'Group name' })}</label>
                <Input value={groupForm.name} onChange={(event) => setGroupForm((prev) => ({ ...prev, name: event.target.value }))} />
              </div>
              <div className="space-y-2">
                <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.form.description', { defaultValue: 'Description' })}</label>
                <Textarea value={groupForm.description} onChange={(event) => setGroupForm((prev) => ({ ...prev, description: event.target.value }))} />
              </div>
              <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.form.inputMultiplier', { defaultValue: 'Input multiplier (ppm)' })}</label>
                  <Input type="number" value={groupForm.input_multiplier_ppm} onChange={(event) => setGroupForm((prev) => ({ ...prev, input_multiplier_ppm: event.target.value }))} />
                </div>
                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.form.cachedInputMultiplier', { defaultValue: 'Cached input multiplier (ppm)' })}</label>
                  <Input type="number" value={groupForm.cached_input_multiplier_ppm} onChange={(event) => setGroupForm((prev) => ({ ...prev, cached_input_multiplier_ppm: event.target.value }))} />
                </div>
                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.form.outputMultiplier', { defaultValue: 'Output multiplier (ppm)' })}</label>
                  <Input type="number" value={groupForm.output_multiplier_ppm} onChange={(event) => setGroupForm((prev) => ({ ...prev, output_multiplier_ppm: event.target.value }))} />
                </div>
              </div>
              <div className="flex flex-wrap gap-4 text-sm text-muted-foreground">
                <label className="inline-flex items-center gap-2">
                  <Checkbox checked={groupForm.enabled} onCheckedChange={(checked) => setGroupForm((prev) => ({ ...prev, enabled: Boolean(checked) }))} />
                  {t('groupsPage.form.enabled', { defaultValue: 'Enabled' })}
                </label>
                <label className="inline-flex items-center gap-2">
                  <Checkbox checked={groupForm.is_default} onCheckedChange={(checked) => setGroupForm((prev) => ({ ...prev, is_default: Boolean(checked) }))} />
                  {t('groupsPage.form.default', { defaultValue: 'Default group' })}
                </label>
                <label className="inline-flex items-center gap-2">
                  <Checkbox checked={groupForm.allow_all_models} onCheckedChange={(checked) => setGroupForm((prev) => ({ ...prev, allow_all_models: Boolean(checked) }))} />
                  {t('groupsPage.form.allowAllModels', { defaultValue: 'Allow all catalog models' })}
                </label>
              </div>
              <div className="hidden flex-col gap-2 sm:flex sm:flex-row">
                <Button type="button" onClick={() => upsertGroupMutation.mutate()} disabled={upsertGroupMutation.isPending}>
                  {upsertGroupMutation.isPending ? <RotateCw className="mr-2 h-4 w-4 animate-spin" /> : null}
                  {t('groupsPage.actions.saveGroup', { defaultValue: 'Save group' })}
                </Button>
                {groupForm.id ? (
                  <Button type="button" variant="destructive" onClick={() => deleteGroupMutation.mutate(groupForm.id)} disabled={deleteGroupMutation.isPending || groupForm.is_default}>
                    <Trash2 className="mr-2 h-4 w-4" />
                    {t('groupsPage.actions.deleteGroup', { defaultValue: 'Delete group' })}
                  </Button>
                ) : null}
              </div>
            </div>

                <div className="min-w-0 space-y-4">
              <div className={editorSectionClassName}>
                <div>
                  <div className="font-medium">{t('groupsPage.policy.title', { defaultValue: 'Model policy' })}</div>
                  <div className="text-xs text-muted-foreground">{t('groupsPage.policy.description', { defaultValue: 'Select a model from the unified catalog, then configure multipliers or absolute pricing.' })}</div>
                </div>

                <div className="space-y-2">
                  <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.policy.model', { defaultValue: 'Model' })}</label>
                  <select
                    className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                    value={selectedModel}
                    onChange={(event) => handleSelectedModelChange(event.target.value)}
                  >
                    {catalog.map((item: ApiKeyGroupCatalogItem) => (
                      <option key={item.model} value={item.model}>
                        {item.model}
                      </option>
                    ))}
                  </select>
                </div>

                <div className="text-xs text-muted-foreground">
                  {selectedCatalogModel
                    ? `${selectedCatalogModel.provider} · ${selectedCatalogModel.title || '-'} · base in ${formatMicrocredits(selectedCatalogModel.base_input_price_microcredits)} · cached ${formatMicrocredits(selectedCatalogModel.base_cached_input_price_microcredits)} · out ${formatMicrocredits(selectedCatalogModel.base_output_price_microcredits)}`
                    : '-'}
                </div>

                <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
                  <div className="space-y-2">
                    <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.policy.inputMultiplier', { defaultValue: 'Input multiplier (ppm)' })}</label>
                    <Input type="number" value={policyForm.input_multiplier_ppm} onChange={(event) => setPolicyForm((prev) => ({ ...prev, input_multiplier_ppm: event.target.value }))} />
                  </div>
                  <div className="space-y-2">
                    <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.policy.cachedInputMultiplier', { defaultValue: 'Cached input multiplier (ppm)' })}</label>
                    <Input type="number" value={policyForm.cached_input_multiplier_ppm} onChange={(event) => setPolicyForm((prev) => ({ ...prev, cached_input_multiplier_ppm: event.target.value }))} />
                  </div>
                  <div className="space-y-2">
                    <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.policy.outputMultiplier', { defaultValue: 'Output multiplier (ppm)' })}</label>
                    <Input type="number" value={policyForm.output_multiplier_ppm} onChange={(event) => setPolicyForm((prev) => ({ ...prev, output_multiplier_ppm: event.target.value }))} />
                  </div>
                </div>

                <div className="grid grid-cols-1 gap-3 md:grid-cols-3">
                  <div className="space-y-2">
                    <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.policy.inputAbsolutePrice', { defaultValue: 'Input absolute price' })}</label>
                    <Input type="number" value={policyForm.input_price_microcredits} onChange={(event) => setPolicyForm((prev) => ({ ...prev, input_price_microcredits: event.target.value }))} />
                  </div>
                  <div className="space-y-2">
                    <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.policy.cachedInputAbsolutePrice', { defaultValue: 'Cached input absolute price' })}</label>
                    <Input type="number" value={policyForm.cached_input_price_microcredits} onChange={(event) => setPolicyForm((prev) => ({ ...prev, cached_input_price_microcredits: event.target.value }))} />
                  </div>
                  <div className="space-y-2">
                    <label className="text-xs font-medium text-muted-foreground">{t('groupsPage.policy.outputAbsolutePrice', { defaultValue: 'Output absolute price' })}</label>
                    <Input type="number" value={policyForm.output_price_microcredits} onChange={(event) => setPolicyForm((prev) => ({ ...prev, output_price_microcredits: event.target.value }))} />
                  </div>
                </div>

                <label className="inline-flex items-center gap-2 text-sm text-muted-foreground">
                  <Checkbox checked={policyForm.enabled} onCheckedChange={(checked) => setPolicyForm((prev) => ({ ...prev, enabled: Boolean(checked) }))} />
                  {t('groupsPage.policy.enabled', { defaultValue: 'Policy enabled' })}
                </label>

                <div className="hidden flex-col gap-2 sm:flex sm:flex-row">
                  <Button type="button" onClick={() => upsertPolicyMutation.mutate()} disabled={upsertPolicyMutation.isPending || !groupForm.id}>
                    {upsertPolicyMutation.isPending ? <RotateCw className="mr-2 h-4 w-4 animate-spin" /> : null}
                    {t('groupsPage.actions.savePolicy', { defaultValue: 'Save model policy' })}
                  </Button>
                  {selectedPolicy ? (
                    <Button type="button" variant="destructive" onClick={() => deletePolicyMutation.mutate(selectedPolicy.id)} disabled={deletePolicyMutation.isPending}>
                      <Trash2 className="mr-2 h-4 w-4" />
                      {t('groupsPage.actions.deletePolicy', { defaultValue: 'Delete policy' })}
                    </Button>
                  ) : null}
                </div>
              </div>

              <div className={editorSectionClassName}>
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <div className="font-medium">{t('groupsPage.preview.title', { defaultValue: 'Effective model preview' })}</div>
                    <div className="text-xs text-muted-foreground">{t('groupsPage.preview.description', { defaultValue: 'Shows the final displayed price for the selected group.' })}</div>
                  </div>
                  {(currentGroup?.models?.length ?? 0) > 3 ? (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className="shrink-0 md:hidden"
                      onClick={() => setMobilePreviewExpanded((prev) => !prev)}
                    >
                      {mobilePreviewExpanded ? (
                        <ChevronUp className="mr-2 h-4 w-4" />
                      ) : (
                        <ChevronDown className="mr-2 h-4 w-4" />
                      )}
                      {mobilePreviewExpanded
                        ? t('common.collapse', { defaultValue: 'Collapse' })
                        : t('common.expand', { defaultValue: 'Expand' })}
                    </Button>
                  ) : null}
                </div>
                <div className="space-y-2 md:hidden">
                  {mobilePreviewModels.map((item) => {
                    const pricingLine = pricingLineForModel(item)
                    return (
                      <div key={item.model} className="rounded-[0.9rem] border border-border/60 bg-background/[0.5] p-3 text-sm">
                        <div className="flex items-start justify-between gap-3">
                          <div className="font-mono text-xs break-all">{item.model}</div>
                          <Badge variant={item.uses_absolute_pricing ? 'success' : 'secondary'}>
                            {item.uses_absolute_pricing
                              ? t('groupsPage.preview.mode.absolute', { defaultValue: 'Absolute override' })
                              : t('groupsPage.preview.mode.formula', { defaultValue: 'Multiplier formula' })}
                          </Badge>
                        </div>
                        <div className="mt-3 space-y-2 text-xs">
                          <div>
                            <div className="text-muted-foreground">{t('groupsPage.preview.columns.finalPrice', { defaultValue: 'Final price' })}</div>
                            <div>{pricingLine.finalPricing}</div>
                          </div>
                          <div>
                            <div className="text-muted-foreground">{t('groupsPage.preview.columns.formulaPrice', { defaultValue: 'Formula price' })}</div>
                            <div className={item.uses_absolute_pricing ? 'line-through text-muted-foreground' : 'text-muted-foreground'}>{pricingLine.formula}</div>
                          </div>
                        </div>
                      </div>
                    )
                  })}
                  {!mobilePreviewExpanded && (currentGroup?.models?.length ?? 0) > 3 ? (
                    <div className="text-center text-xs text-muted-foreground">
                      {t('groupsPage.preview.moreHidden', {
                        defaultValue: '还有 {{count}} 个模型已折叠',
                        count: (currentGroup?.models?.length ?? 0) - mobilePreviewModels.length,
                      })}
                    </div>
                  ) : null}
                </div>
                <div className="hidden max-h-[360px] overflow-auto rounded-[0.95rem] border border-border/60 bg-background/[0.5] md:block">
                  <table className="min-w-[720px] w-full text-sm">
                    <thead className="bg-muted/40 text-left text-xs text-muted-foreground">
                      <tr>
                        <th className="px-3 py-2">{t('groupsPage.preview.columns.model', { defaultValue: 'Model' })}</th>
                        <th className="px-3 py-2">{t('groupsPage.preview.columns.finalPrice', { defaultValue: 'Final price' })}</th>
                        <th className="px-3 py-2">{t('groupsPage.preview.columns.formulaPrice', { defaultValue: 'Formula price' })}</th>
                        <th className="px-3 py-2">{t('groupsPage.preview.columns.mode', { defaultValue: 'Mode' })}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(currentGroup?.models ?? []).map((item) => {
                        const pricingLine = pricingLineForModel(item)
                        return (
                          <tr key={item.model} className="border-t align-top">
                            <td className="px-3 py-2 font-mono text-xs">{item.model}</td>
                            <td className="px-3 py-2 text-xs">{pricingLine.finalPricing}</td>
                            <td className="px-3 py-2 text-xs text-muted-foreground">
                              {item.uses_absolute_pricing ? <span className="line-through">{pricingLine.formula}</span> : pricingLine.formula}
                            </td>
                            <td className="px-3 py-2 text-xs">
                              <Badge variant={item.uses_absolute_pricing ? 'success' : 'secondary'}>
                                {item.uses_absolute_pricing
                                  ? t('groupsPage.preview.mode.absolute', { defaultValue: 'Absolute override' })
                                  : t('groupsPage.preview.mode.formula', { defaultValue: 'Multiplier formula' })}
                              </Badge>
                            </td>
                          </tr>
                        )
                      })}
                    </tbody>
                  </table>
                </div>
              </div>
                </div>
              </div>
            </div>

            <div className="shrink-0 border-t border-border/70 bg-background/95 p-3 backdrop-blur sm:hidden">
              <div className="grid gap-2">
                <Button type="button" onClick={() => upsertGroupMutation.mutate()} disabled={upsertGroupMutation.isPending}>
                  {upsertGroupMutation.isPending ? <RotateCw className="mr-2 h-4 w-4 animate-spin" /> : null}
                  {t('groupsPage.actions.saveGroup', { defaultValue: 'Save group' })}
                </Button>
                {groupForm.id ? (
                  <Button
                    type="button"
                    variant="destructive"
                    onClick={() => deleteGroupMutation.mutate(groupForm.id)}
                    disabled={deleteGroupMutation.isPending || groupForm.is_default}
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    {t('groupsPage.actions.deleteGroup', { defaultValue: 'Delete group' })}
                  </Button>
                ) : null}
                <Button type="button" variant="outline" onClick={() => upsertPolicyMutation.mutate()} disabled={upsertPolicyMutation.isPending || !groupForm.id}>
                  {upsertPolicyMutation.isPending ? <RotateCw className="mr-2 h-4 w-4 animate-spin" /> : null}
                  {t('groupsPage.actions.savePolicy', { defaultValue: 'Save model policy' })}
                </Button>
                {selectedPolicy ? (
                  <Button
                    type="button"
                    variant="destructive"
                    onClick={() => deletePolicyMutation.mutate(selectedPolicy.id)}
                    disabled={deletePolicyMutation.isPending}
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    {t('groupsPage.actions.deletePolicy', { defaultValue: 'Delete policy' })}
                  </Button>
                ) : null}
              </div>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  )
}
