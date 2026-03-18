import { useCallback, useEffect, useMemo, useState } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Activity, Pencil, Plus, Shield, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { localizeApiErrorDisplay } from '@/api/errorI18n'
import { proxiesApi, type ProxyNode } from '@/api/proxies'
import type {
  AdminProxyPoolResponse,
  ProxyFailMode,
  UpdateAdminProxyPoolSettingsRequest,
} from '@/api/types'
import { PageIntro } from '@/components/layout/page-archetypes'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { useConfirmDialog } from '@/components/ui/confirm-dialog'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
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
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { cn } from '@/lib/utils'
import { formatRelativeTime } from '@/lib/time'
import { notify } from '@/lib/notification'

type ProxyHealthFilter = 'all' | 'healthy' | 'degraded' | 'offline' | 'disabled'

interface ProxyEditorDraft {
  id?: string
  label: string
  proxy_url: string
  enabled: boolean
  weight: string
}

const EMPTY_PROXY_NODES: ProxyNode[] = []

function createEmptyDraft(): ProxyEditorDraft {
  return {
    label: '',
    proxy_url: '',
    enabled: true,
    weight: '1',
  }
}

function resolveProxyHealth(proxy: ProxyNode): ProxyHealthFilter {
  if (!proxy.enabled) {
    return 'disabled'
  }
  if (proxy.last_test_status === 'error') {
    return 'offline'
  }
  if (proxy.last_test_status === 'skipped') {
    return 'degraded'
  }
  return 'healthy'
}

function buildProxySearchText(proxy: ProxyNode): string {
  return [
    proxy.label,
    proxy.proxy_url_masked,
    proxy.scheme,
    proxy.last_error,
    proxy.last_test_status,
  ]
    .filter(Boolean)
    .join(' ')
    .toLowerCase()
}

function mergeTestResults(
  current: AdminProxyPoolResponse | undefined,
  results: ProxyNode[],
): AdminProxyPoolResponse | undefined {
  if (!current) {
    return current
  }
  const next = new Map(results.map((node) => [node.id, node]))
  return {
    ...current,
    nodes: current.nodes.map((node) => next.get(node.id) ?? node),
  }
}

export default function Proxies() {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const { confirm, confirmDialog } = useConfirmDialog()

  const [healthFilter, setHealthFilter] = useState<ProxyHealthFilter>('all')
  const [searchKeyword, setSearchKeyword] = useState('')
  const [editorOpen, setEditorOpen] = useState(false)
  const [editorDraft, setEditorDraft] = useState<ProxyEditorDraft>(createEmptyDraft)
  const [settingsDraft, setSettingsDraft] = useState<UpdateAdminProxyPoolSettingsRequest>({
    enabled: false,
    fail_mode: 'strict_proxy',
  })
  const [pendingRowActionId, setPendingRowActionId] = useState<string | null>(null)

  const { data, isLoading } = useQuery({
    queryKey: ['proxyPool'],
    queryFn: proxiesApi.listProxies,
    refetchInterval: 15_000,
  })

  const proxies = data?.nodes ?? EMPTY_PROXY_NODES
  const totalCount = proxies.length
  const enabledCount = proxies.filter((proxy) => proxy.enabled).length
  const healthyCount = proxies.filter((proxy) => resolveProxyHealth(proxy) === 'healthy').length
  const hasSettingsChanges =
    settingsDraft.enabled !== (data?.settings.enabled ?? false) ||
    settingsDraft.fail_mode !== (data?.settings.fail_mode ?? 'strict_proxy')

  useEffect(() => {
    if (!data || hasSettingsChanges) {
      return
    }
    setSettingsDraft({
      enabled: data.settings.enabled,
      fail_mode: data.settings.fail_mode,
    })
  }, [data, hasSettingsChanges])

  const updatePoolNodes = (updater: (current: AdminProxyPoolResponse | undefined) => AdminProxyPoolResponse | undefined) => {
    queryClient.setQueryData<AdminProxyPoolResponse>(['proxyPool'], updater)
  }

  const showMutationError = (fallback: string, error: unknown) => {
    notify({
      variant: 'error',
      title: fallback,
      description: localizeApiErrorDisplay(t, error, fallback).label,
    })
  }

  const settingsMutation = useMutation({
    mutationFn: proxiesApi.updateSettings,
    onSuccess: (payload) => {
      updatePoolNodes((current) =>
        current
          ? {
              ...current,
              settings: payload.settings,
            }
          : current,
      )
      notify({
        variant: 'success',
        title: t('proxies.notifications.settingsSavedTitle'),
        description: t('proxies.notifications.settingsSavedDescription'),
      })
    },
    onError: (error) => {
      showMutationError(t('proxies.notifications.settingsFailedTitle'), error)
    },
  })

  const createMutation = useMutation({
    mutationFn: proxiesApi.createProxy,
    onSuccess: (payload) => {
      setEditorOpen(false)
      setEditorDraft(createEmptyDraft())
      updatePoolNodes((current) =>
        current
          ? {
              ...current,
              nodes: [...current.nodes, payload.node],
            }
          : current,
      )
      notify({
        variant: 'success',
        title: t('proxies.notifications.nodeCreatedTitle'),
        description: t('proxies.notifications.nodeCreatedDescription'),
      })
    },
    onError: (error) => {
      showMutationError(t('proxies.notifications.nodeCreateFailedTitle'), error)
    },
  })

  const updateMutation = useMutation({
    mutationFn: ({ proxyId, payload }: { proxyId: string; payload: ProxyEditorDraft }) =>
      proxiesApi.updateProxy(proxyId, {
        label: payload.label,
        proxy_url: payload.proxy_url.trim() || undefined,
        enabled: payload.enabled,
        weight: Number(payload.weight),
      }),
    onSuccess: (payload) => {
      setEditorOpen(false)
      setEditorDraft(createEmptyDraft())
      updatePoolNodes((current) =>
        current
          ? {
              ...current,
              nodes: current.nodes.map((node) => (node.id === payload.node.id ? payload.node : node)),
            }
          : current,
      )
      notify({
        variant: 'success',
        title: t('proxies.notifications.nodeUpdatedTitle'),
        description: t('proxies.notifications.nodeUpdatedDescription'),
      })
    },
    onError: (error) => {
      showMutationError(t('proxies.notifications.nodeUpdateFailedTitle'), error)
    },
  })

  const deleteMutation = useMutation({
    mutationFn: proxiesApi.deleteProxy,
    onSuccess: (_payload, proxyId) => {
      updatePoolNodes((current) =>
        current
          ? {
              ...current,
              nodes: current.nodes.filter((node) => node.id !== proxyId),
            }
          : current,
      )
      notify({
        variant: 'success',
        title: t('proxies.notifications.nodeDeletedTitle'),
        description: t('proxies.notifications.nodeDeletedDescription'),
      })
    },
    onError: (error) => {
      showMutationError(t('proxies.notifications.nodeDeleteFailedTitle'), error)
    },
  })

  const testAllMutation = useMutation({
    mutationFn: proxiesApi.testAll,
    onSuccess: (payload) => {
      updatePoolNodes((current) => mergeTestResults(current, payload.results))
      notify({
        variant: 'success',
        title: t('proxies.notifications.testCompletedTitle'),
        description: t('proxies.notifications.testCompletedDescription', {
          count: payload.tested,
        }),
      })
    },
    onError: (error) => {
      showMutationError(t('proxies.notifications.testFailedTitle'), error)
    },
  })

  const testOneMutation = useMutation({
    mutationFn: proxiesApi.testProxy,
    onMutate: (proxyId) => setPendingRowActionId(`test:${proxyId}`),
    onSettled: () => setPendingRowActionId(null),
    onSuccess: (payload) => {
      updatePoolNodes((current) => mergeTestResults(current, payload.results))
      notify({
        variant: 'success',
        title: t('proxies.notifications.testCompletedTitle'),
        description: t('proxies.notifications.singleTestCompletedDescription'),
      })
    },
    onError: (error) => {
      showMutationError(t('proxies.notifications.testFailedTitle'), error)
    },
  })

  const filteredData = useMemo(() => {
    const keyword = searchKeyword.trim().toLowerCase()
    return proxies.filter((proxy) => {
      if (healthFilter !== 'all' && resolveProxyHealth(proxy) !== healthFilter) {
        return false
      }
      if (!keyword) {
        return true
      }
      return buildProxySearchText(proxy).includes(keyword)
    })
  }, [healthFilter, proxies, searchKeyword])

  const openCreateDialog = () => {
    setEditorDraft(createEmptyDraft())
    setEditorOpen(true)
  }

  const openEditDialog = (proxy: ProxyNode) => {
    setEditorDraft({
      id: proxy.id,
      label: proxy.label,
      proxy_url: '',
      enabled: proxy.enabled,
      weight: String(proxy.weight),
    })
    setEditorOpen(true)
  }

  const submitEditor = () => {
    const label = editorDraft.label.trim()
    const proxyUrl = editorDraft.proxy_url.trim()
    const weight = Number(editorDraft.weight)
    if (!label) {
      notify({
        variant: 'error',
        title: t('proxies.notifications.validationFailedTitle'),
        description: t('proxies.editor.errors.labelRequired'),
      })
      return
    }
    if (!editorDraft.id && !proxyUrl) {
      notify({
        variant: 'error',
        title: t('proxies.notifications.validationFailedTitle'),
        description: t('proxies.editor.errors.proxyUrlRequired'),
      })
      return
    }
    if (!Number.isFinite(weight) || weight <= 0) {
      notify({
        variant: 'error',
        title: t('proxies.notifications.validationFailedTitle'),
        description: t('proxies.editor.errors.weightInvalid'),
      })
      return
    }

    if (editorDraft.id) {
      updateMutation.mutate({
        proxyId: editorDraft.id,
        payload: {
          ...editorDraft,
          label,
          proxy_url: proxyUrl,
          weight: String(weight),
        },
      })
      return
    }

    createMutation.mutate({
      label,
      proxy_url: proxyUrl,
      enabled: editorDraft.enabled,
      weight,
    })
  }

  const confirmDelete = useCallback(
    async (proxy: ProxyNode) => {
      const approved = await confirm({
        title: t('proxies.deleteDialog.title'),
        description: t('proxies.deleteDialog.description', { label: proxy.label }),
        confirmText: t('proxies.deleteDialog.confirm'),
        cancelText: t('common.cancel'),
        variant: 'destructive',
      })
      if (!approved) {
        return
      }
      setPendingRowActionId(`delete:${proxy.id}`)
      try {
        await deleteMutation.mutateAsync(proxy.id)
      } finally {
        setPendingRowActionId(null)
      }
    },
    [confirm, deleteMutation, t],
  )

  const columns = useMemo<ColumnDef<ProxyNode>[]>(
    () => [
      {
        id: 'proxy',
        header: t('proxies.columns.proxy'),
        accessorFn: (row) => `${row.label} ${row.proxy_url_masked}`.toLowerCase(),
        cell: ({ row }) => (
          <div className="min-w-[16rem] space-y-1">
            <div className="flex items-center gap-2">
              <span className="font-medium">{row.original.label}</span>
              <Badge variant="secondary" className="font-mono uppercase">
                {row.original.scheme}
              </Badge>
              {row.original.has_auth ? (
                <Badge variant="warning">{t('proxies.badges.auth')}</Badge>
              ) : null}
            </div>
            <div className="max-w-[28rem] truncate font-mono text-xs text-muted-foreground" title={row.original.proxy_url_masked}>
              {row.original.proxy_url_masked}
            </div>
          </div>
        ),
      },
      {
        id: 'status',
        header: t('proxies.columns.status'),
        accessorFn: (row) => resolveProxyHealth(row),
        cell: ({ row }) => {
          const health = resolveProxyHealth(row.original)
          if (health === 'disabled') {
            return <Badge variant="secondary">{t('proxies.health.disabled')}</Badge>
          }
          if (health === 'offline') {
            return <Badge variant="destructive">{t('proxies.health.offline')}</Badge>
          }
          if (health === 'degraded') {
            return <Badge variant="warning">{t('proxies.health.degraded')}</Badge>
          }
          return <Badge variant="success">{t('proxies.health.healthy')}</Badge>
        },
      },
      {
        accessorKey: 'weight',
        header: t('proxies.columns.weight'),
        cell: ({ row }) => <span className="font-mono text-sm">{row.original.weight}</span>,
      },
      {
        accessorKey: 'last_latency_ms',
        header: t('proxies.columns.latency'),
        cell: ({ row }) => (
          <span className="font-mono text-sm tabular-nums">
            {typeof row.original.last_latency_ms === 'number'
              ? `${row.original.last_latency_ms}ms`
              : '-'}
          </span>
        ),
      },
      {
        id: 'lastTest',
        header: t('proxies.columns.lastTest'),
        accessorFn: (row) => row.last_tested_at ?? row.updated_at,
        cell: ({ row }) => (
          <div className="space-y-1 text-sm">
            <div className="text-muted-foreground">
              {row.original.last_tested_at
                ? formatRelativeTime(row.original.last_tested_at, i18n.resolvedLanguage, true)
                : t('proxies.pending')}
            </div>
            {row.original.last_error ? (
              <div className="max-w-[16rem] truncate text-xs text-destructive" title={row.original.last_error}>
                {row.original.last_error}
              </div>
            ) : null}
          </div>
        ),
      },
      {
        id: 'actions',
        enableSorting: false,
        header: t('proxies.columns.actions'),
        cell: ({ row }) => {
          const proxy = row.original
          const testing = pendingRowActionId === `test:${proxy.id}` && testOneMutation.isPending
          const deleting = pendingRowActionId === `delete:${proxy.id}` && deleteMutation.isPending
          return (
            <div className="flex flex-wrap items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => testOneMutation.mutate(proxy.id)}
                disabled={testing || deleting}
              >
                {testing ? <Activity className="mr-2 h-4 w-4 animate-spin" /> : <Shield className="mr-2 h-4 w-4" />}
                {t('proxies.actions.test')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => openEditDialog(proxy)}
                disabled={testing || deleting}
              >
                <Pencil className="mr-2 h-4 w-4" />
                {t('proxies.actions.edit')}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  void confirmDelete(proxy)
                }}
                disabled={testing || deleting}
              >
                <Trash2 className="mr-2 h-4 w-4" />
                {t('proxies.actions.delete')}
              </Button>
            </div>
          )
        },
      },
    ],
    [confirmDelete, deleteMutation.isPending, i18n.resolvedLanguage, pendingRowActionId, t, testOneMutation],
  )

  return (
    <div className="flex-1 p-4 sm:p-6 lg:p-8">
      <div className="space-y-6 md:space-y-7">
        <PageIntro
          archetype="workspace"
          title={t('proxies.title')}
          description={t('proxies.subtitle')}
          meta={(
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
              <span>{t('proxies.meta.total', { count: totalCount })}</span>
              <span>{t('proxies.meta.enabled', { count: enabledCount })}</span>
              <span>{t('proxies.meta.healthy', { count: healthyCount })}</span>
            </div>
          )}
          actions={(
            <div className="flex flex-wrap items-center gap-2">
              <Button variant="outline" onClick={() => testAllMutation.mutate()} disabled={testAllMutation.isPending}>
                <Activity className={cn('mr-2 h-4 w-4', testAllMutation.isPending && 'animate-spin')} />
                {t('proxies.actions.testAll')}
              </Button>
              <Button onClick={openCreateDialog}>
                <Plus className="mr-2 h-4 w-4" />
                {t('proxies.actions.add')}
              </Button>
            </div>
          )}
        />

        <Card className="border-border/60">
          <CardHeader>
            <CardTitle>{t('proxies.settings.title')}</CardTitle>
            <CardDescription>{t('proxies.settings.description')}</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,20rem)]">
            <div className="grid gap-4 sm:grid-cols-3">
              <div className="rounded-xl border border-border/70 bg-muted/[0.16] p-4">
                <div className="text-xs text-muted-foreground">{t('proxies.stats.total')}</div>
                <div className="mt-2 text-2xl font-semibold">{totalCount}</div>
              </div>
              <div className="rounded-xl border border-border/70 bg-muted/[0.16] p-4">
                <div className="text-xs text-muted-foreground">{t('proxies.stats.enabled')}</div>
                <div className="mt-2 text-2xl font-semibold">{enabledCount}</div>
              </div>
              <div className="rounded-xl border border-border/70 bg-muted/[0.16] p-4">
                <div className="text-xs text-muted-foreground">{t('proxies.stats.healthy')}</div>
                <div className="mt-2 text-2xl font-semibold">{healthyCount}</div>
              </div>
            </div>

            <div className="space-y-4 rounded-xl border border-border/70 bg-muted/[0.16] p-4">
              <label className="flex items-start gap-3">
                <Checkbox
                  checked={settingsDraft.enabled}
                  onCheckedChange={(checked) =>
                    setSettingsDraft((current) => ({ ...current, enabled: checked === true }))
                  }
                />
                <span className="space-y-1">
                  <span className="block text-sm font-medium">{t('proxies.settings.enabled')}</span>
                  <span className="block text-xs text-muted-foreground">
                    {t('proxies.settings.enabledHint')}
                  </span>
                </span>
              </label>

              <div className="space-y-1.5">
                <label className="text-sm font-medium">{t('proxies.settings.failMode')}</label>
                <Select
                  value={settingsDraft.fail_mode}
                  onValueChange={(value) =>
                    setSettingsDraft((current) => ({
                      ...current,
                      fail_mode: value as ProxyFailMode,
                    }))
                  }
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="strict_proxy">{t('proxies.failModes.strictProxy')}</SelectItem>
                    <SelectItem value="allow_direct_fallback">
                      {t('proxies.failModes.allowDirectFallback')}
                    </SelectItem>
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  {settingsDraft.fail_mode === 'strict_proxy'
                    ? t('proxies.failModeDescriptions.strictProxy')
                    : t('proxies.failModeDescriptions.allowDirectFallback')}
                </p>
              </div>

              <Button
                className="w-full"
                onClick={() => settingsMutation.mutate(settingsDraft)}
                disabled={!hasSettingsChanges || settingsMutation.isPending}
              >
                {settingsMutation.isPending ? (
                  <Activity className="mr-2 h-4 w-4 animate-spin" />
                ) : null}
                {t('proxies.settings.save')}
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card className="border-border/60">
          <CardHeader className="gap-4 sm:flex-row sm:items-end sm:justify-between">
            <div className="space-y-1">
              <CardTitle>{t('proxies.list.title')}</CardTitle>
              <CardDescription>{t('proxies.list.description')}</CardDescription>
            </div>
            <div className="grid gap-3 sm:grid-cols-[12rem_16rem]">
              <div className="space-y-1.5">
                <label className="text-xs font-medium text-muted-foreground">
                  {t('proxies.filters.label')}
                </label>
                <Select value={healthFilter} onValueChange={(value) => setHealthFilter(value as ProxyHealthFilter)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">{t('proxies.filters.all')}</SelectItem>
                    <SelectItem value="healthy">{t('proxies.filters.healthy')}</SelectItem>
                    <SelectItem value="degraded">{t('proxies.filters.degraded')}</SelectItem>
                    <SelectItem value="offline">{t('proxies.filters.offline')}</SelectItem>
                    <SelectItem value="disabled">{t('proxies.filters.disabled')}</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <label htmlFor="proxy-search" className="text-xs font-medium text-muted-foreground">
                  {t('common.table.searchLabel')}
                </label>
                <Input
                  id="proxy-search"
                  value={searchKeyword}
                  onChange={(event) => setSearchKeyword(event.target.value)}
                  placeholder={t('proxies.searchPlaceholder')}
                  autoComplete="off"
                />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div className="relative">
              <LoadingOverlay
                show={isLoading}
                title={t('proxies.loading')}
                description={t('common.loading')}
              />
              <StandardDataTable
                columns={columns}
                data={filteredData}
                className="min-h-[24rem] border-0 bg-transparent shadow-none"
                enableSearch={false}
                emptyText={t('proxies.empty')}
              />
            </div>
          </CardContent>
        </Card>
      </div>

      <Dialog open={editorOpen} onOpenChange={setEditorOpen}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>
              {editorDraft.id ? t('proxies.editor.editTitle') : t('proxies.editor.createTitle')}
            </DialogTitle>
            <DialogDescription>{t('proxies.editor.description')}</DialogDescription>
          </DialogHeader>

          <div className="grid gap-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <div className="space-y-1.5">
                <label className="text-sm font-medium">{t('proxies.editor.fields.label')}</label>
                <Input
                  value={editorDraft.label}
                  onChange={(event) =>
                    setEditorDraft((current) => ({ ...current, label: event.target.value }))
                  }
                />
              </div>
              <div className="space-y-1.5">
                <label className="text-sm font-medium">{t('proxies.editor.fields.weight')}</label>
                <Input
                  type="number"
                  min="1"
                  step="1"
                  value={editorDraft.weight}
                  onChange={(event) =>
                    setEditorDraft((current) => ({ ...current, weight: event.target.value }))
                  }
                />
              </div>
            </div>

            <div className="space-y-1.5">
              <label className="text-sm font-medium">{t('proxies.editor.fields.proxyUrl')}</label>
              <Input
                value={editorDraft.proxy_url}
                onChange={(event) =>
                  setEditorDraft((current) => ({ ...current, proxy_url: event.target.value }))
                }
                placeholder={t('proxies.editor.proxyUrlPlaceholder')}
              />
              <p className="text-xs text-muted-foreground">{t('proxies.editor.proxyUrlHint')}</p>
            </div>

            <label className="flex items-start gap-3 rounded-xl border border-border/70 bg-muted/[0.16] p-3">
              <Checkbox
                checked={editorDraft.enabled}
                onCheckedChange={(checked) =>
                  setEditorDraft((current) => ({ ...current, enabled: checked === true }))
                }
              />
              <span className="space-y-1">
                <span className="block text-sm font-medium">{t('proxies.editor.fields.enabled')}</span>
                <span className="block text-xs text-muted-foreground">
                  {t('proxies.editor.enabledHint')}
                </span>
              </span>
            </label>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setEditorOpen(false)}
              disabled={createMutation.isPending || updateMutation.isPending}
            >
              {t('common.cancel')}
            </Button>
            <Button
              type="button"
              onClick={submitEditor}
              disabled={createMutation.isPending || updateMutation.isPending}
            >
              {createMutation.isPending || updateMutation.isPending ? (
                <Activity className="mr-2 h-4 w-4 animate-spin" />
              ) : null}
              {editorDraft.id ? t('proxies.editor.save') : t('proxies.editor.create')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {confirmDialog}

    </div>
  )
}
