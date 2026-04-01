import {
  Button,
  Chip,
  Input,
  Select,
  SelectItem,
  Spinner,
  Switch,
} from '@heroui/react'
import type { Selection } from '@heroui/react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Globe, Network, Pencil, PlayCircle, Plus, Save, Trash2, X,
} from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { localizeApiErrorDisplay } from '@/api/errorI18n'
import { proxiesApi } from '@/api/proxies'
import type { ProxyFailMode, UpdateAdminProxyPoolSettingsRequest } from '@/api/types'
import {
  DockedPageIntro,
  PageContent,
  PagePanel,
  SectionHeader,
} from '@/components/layout/page-archetypes'
import { SurfaceInset } from '@/components/ui/surface'
import { createProxySettingsDraft, mapProxyNodesToCards, type ProxyHealth } from '@/features/proxies/contracts'
import { formatDurationMs } from '@/lib/duration-format'
import { notify } from '@/lib/notification'

const statusColorMap: Record<ProxyHealth, 'success' | 'warning' | 'danger' | 'default'> = {
  healthy: 'success',
  degraded: 'warning',
  offline: 'danger',
  disabled: 'default',
}

interface ProxyEditorDraft {
  id?: string
  label: string
  proxy_url: string
  enabled: boolean
  weight: string
}

const EMPTY_PROXY_NODES: NonNullable<Awaited<ReturnType<typeof proxiesApi.listProxies>>['nodes']> = []

function createEmptyDraft(): ProxyEditorDraft {
  return {
    label: '',
    proxy_url: '',
    enabled: true,
    weight: '1',
  }
}

export default function Proxies() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [settingsDraft, setSettingsDraft] = useState<UpdateAdminProxyPoolSettingsRequest | null>(null)
  const [editorDraft, setEditorDraft] = useState<ProxyEditorDraft | null>(null)
  const [pendingProxyId, setPendingProxyId] = useState<string | null>(null)

  const { data: proxyPool, isLoading, isFetching } = useQuery({
    queryKey: ['proxies'],
    queryFn: proxiesApi.listProxies,
    refetchInterval: 15_000,
  })

  const effectiveSettings = settingsDraft ?? createProxySettingsDraft(proxyPool)
  const nodes = proxyPool?.nodes ?? EMPTY_PROXY_NODES
  const proxyCards = useMemo(() => mapProxyNodesToCards(nodes), [nodes])
  const hasSettingsChanges =
    proxyPool !== undefined
    && JSON.stringify(effectiveSettings) !== JSON.stringify(createProxySettingsDraft(proxyPool))

  const invalidate = async () => {
    await queryClient.invalidateQueries({ queryKey: ['proxies'] })
  }

  const notifyProxyError = (error: unknown, fallbackTitle: string) => {
    const description = localizeApiErrorDisplay(t, error, fallbackTitle).label
    notify({
      variant: 'error',
      title: fallbackTitle,
      description: description !== fallbackTitle ? description : undefined,
    })
  }

  const settingsMutation = useMutation({
    mutationFn: proxiesApi.updateSettings,
    onSuccess: async () => {
      setSettingsDraft(null)
      await invalidate()
      notify({
        variant: 'success',
        title: t('proxies.notifications.settingsSavedDescription', {
          defaultValue: 'The global outbound proxy pool settings have been saved.',
        }),
      })
    },
    onError: (error) => {
      notifyProxyError(
        error,
        t('proxies.notifications.settingsFailedTitle', {
          defaultValue: 'Failed to save proxy settings',
        }),
      )
    },
  })

  const createMutation = useMutation({
    mutationFn: proxiesApi.createProxy,
    onSuccess: async () => {
      setEditorDraft(null)
      await invalidate()
      notify({
        variant: 'success',
        title: t('proxies.notifications.nodeCreatedDescription', {
          defaultValue: 'The proxy node has been added to the global pool.',
        }),
      })
    },
    onError: (error) => {
      notifyProxyError(
        error,
        t('proxies.notifications.nodeCreateFailedTitle', {
          defaultValue: 'Failed to create proxy',
        }),
      )
    },
  })

  const updateMutation = useMutation({
    mutationFn: ({ proxyId, payload }: { proxyId: string; payload: ProxyEditorDraft }) =>
      proxiesApi.updateProxy(proxyId, {
        label: payload.label,
        proxy_url: payload.proxy_url.trim() || undefined,
        enabled: payload.enabled,
        weight: Number(payload.weight) || 1,
      }),
    onSuccess: async () => {
      setEditorDraft(null)
      await invalidate()
      notify({
        variant: 'success',
        title: t('proxies.notifications.nodeUpdatedDescription', {
          defaultValue: 'The proxy node has been updated.',
        }),
      })
    },
    onError: (error) => {
      notifyProxyError(
        error,
        t('proxies.notifications.nodeUpdateFailedTitle', {
          defaultValue: 'Failed to update proxy',
        }),
      )
    },
  })

  const deleteMutation = useMutation({
    mutationFn: proxiesApi.deleteProxy,
    onMutate: (proxyId) => {
      setPendingProxyId(proxyId)
    },
    onSuccess: async () => {
      await invalidate()
      notify({
        variant: 'success',
        title: t('proxies.notifications.nodeDeletedDescription', {
          defaultValue: 'The proxy node has been removed from the global pool.',
        }),
      })
    },
    onError: (error) => {
      notifyProxyError(
        error,
        t('proxies.notifications.nodeDeleteFailedTitle', {
          defaultValue: 'Failed to delete proxy',
        }),
      )
    },
    onSettled: () => {
      setPendingProxyId(null)
    },
  })

  const testAllMutation = useMutation({
    mutationFn: proxiesApi.testAll,
    onSuccess: async () => {
      await invalidate()
      notify({
        variant: 'success',
        title: t('proxies.notifications.testCompletedDescription', {
          count: nodes.length,
          defaultValue: 'Finished testing {{count}} proxy nodes.',
        }),
      })
    },
    onError: (error) => {
      notifyProxyError(
        error,
        t('proxies.notifications.testFailedTitle', {
          defaultValue: 'Proxy test failed',
        }),
      )
    },
  })

  const testOneMutation = useMutation({
    mutationFn: async (proxyId: string) => {
      setPendingProxyId(proxyId)
      return proxiesApi.testProxy(proxyId)
    },
    onSuccess: async () => {
      await invalidate()
      notify({
        variant: 'success',
        title: t('proxies.notifications.singleTestCompletedDescription', {
          defaultValue: 'The proxy test has finished.',
        }),
      })
    },
    onError: (error) => {
      notifyProxyError(
        error,
        t('proxies.notifications.testFailedTitle', {
          defaultValue: 'Proxy test failed',
        }),
      )
    },
    onSettled: () => {
      setPendingProxyId(null)
    },
  })

  const beginCreate = () => {
    setEditorDraft(createEmptyDraft())
  }

  const beginEdit = (proxyId: string) => {
    const node = nodes.find((item) => item.id === proxyId)
    if (!node) {
      return
    }
    setEditorDraft({
      id: node.id,
      label: node.label,
      proxy_url: '',
      enabled: node.enabled,
      weight: String(node.weight),
    })
  }

  const handleSaveSettings = () => {
    settingsMutation.mutate(effectiveSettings)
  }

  const handleSaveNode = () => {
    if (!editorDraft) {
      return
    }
    if (editorDraft.id) {
      updateMutation.mutate({ proxyId: editorDraft.id, payload: editorDraft })
      return
    }
    createMutation.mutate({
      label: editorDraft.label,
      proxy_url: editorDraft.proxy_url.trim(),
      enabled: editorDraft.enabled,
      weight: Number(editorDraft.weight) || 1,
    })
  }

  const handleDeleteNode = (proxyId: string) => {
    deleteMutation.mutate(proxyId)
  }

  if (isLoading) {
    return (
      <div className="flex h-[calc(100vh-100px)] w-full items-center justify-center">
        <Spinner
          size="lg"
          color="primary"
          label={t('proxies.loading', { defaultValue: 'Loading outbound proxy pool…' })}
        />
      </div>
    )
  }

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t('nav.proxies')}
        description={t('proxies.subtitle', {
          defaultValue: 'Configure a global outbound proxy pool for all upstream traffic. This replaces the old node-health placeholder page.',
        })}
        actions={(
          <>
            <Button
              variant="flat"
              startContent={<PlayCircle className="h-4 w-4" />}
              onPress={() => testAllMutation.mutate()}
              isLoading={testAllMutation.isPending}
            >
              {t('proxies.actions.testAll', { defaultValue: 'Test All' })}
            </Button>
            <Button color="primary" startContent={<Plus className="h-4 w-4" />} onPress={beginCreate}>
              {t('proxies.actions.add', { defaultValue: 'Add Proxy' })}
            </Button>
          </>
        )}
      />

      <PagePanel>
        <SectionHeader
          title={t('proxies.settings.title', { defaultValue: 'Global Proxy Pool' })}
          description={t('proxies.settings.description', {
            defaultValue: 'These settings apply to every outbound HTTP and WebSocket request that goes through the platform.',
          })}
          actions={(
            <Button
              color="primary"
              variant="flat"
              startContent={<Save className="h-4 w-4" />}
              onPress={handleSaveSettings}
              isLoading={settingsMutation.isPending}
              isDisabled={!hasSettingsChanges}
            >
              {t('proxies.settings.save', { defaultValue: 'Save Settings' })}
            </Button>
          )}
        />
        <div className="grid gap-5 pt-4 lg:grid-cols-[minmax(0,0.45fr)_minmax(0,0.55fr)]">
          <SurfaceInset tone="muted">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-sm font-medium text-foreground">
                  {t('proxies.settings.enabled', { defaultValue: 'Enable outbound proxy pool' })}
                </p>
                <p className="text-xs text-default-500">
                  {t('proxies.settings.enabledHint', {
                    defaultValue: 'When disabled, all outbound traffic stays direct. When enabled, traffic is selected from the weighted proxy pool below.',
                  })}
                </p>
              </div>
              <Switch
                isSelected={effectiveSettings.enabled}
                onValueChange={(value) =>
                  setSettingsDraft((previous) => ({
                    ...(previous ?? createProxySettingsDraft(proxyPool)),
                    enabled: value,
                  }))
                }
                color="primary"
              />
            </div>
          </SurfaceInset>

          <Select
            aria-label={t('proxies.settings.failMode', { defaultValue: 'Failure mode' })}
            label={t('proxies.settings.failMode', { defaultValue: 'Failure mode' })}
            labelPlacement="outside"
            selectedKeys={new Set([effectiveSettings.fail_mode])}
            disallowEmptySelection
            onSelectionChange={(keys: Selection) => {
              if (keys === 'all') {
                return
              }
              const next = Array.from(keys)[0]
              if (next === 'strict_proxy' || next === 'allow_direct_fallback') {
                setSettingsDraft((previous) => ({
                  ...(previous ?? createProxySettingsDraft(proxyPool)),
                  fail_mode: next as ProxyFailMode,
                }))
              }
            }}
          >
            <SelectItem key="strict_proxy">
              {t('proxies.failModes.strictProxy', { defaultValue: 'Strict proxy only' })}
            </SelectItem>
            <SelectItem key="allow_direct_fallback">
              {t('proxies.failModes.allowDirectFallback', { defaultValue: 'Allow direct fallback' })}
            </SelectItem>
          </Select>
        </div>
      </PagePanel>

      {editorDraft ? (
        <PagePanel>
          <SectionHeader
            title={editorDraft.id
              ? t('proxies.editor.editTitle', { defaultValue: 'Edit Outbound Proxy' })
              : t('proxies.editor.createTitle', { defaultValue: 'Create Outbound Proxy' })}
            description={t('proxies.editor.description', {
              defaultValue: 'Configure a global outbound proxy node. Leave the URL blank during edit to keep the current secret unchanged.',
            })}
            actions={(
              <Button variant="light" isIconOnly aria-label={t('common.close')} onPress={() => setEditorDraft(null)}>
                <X className="h-4 w-4" />
              </Button>
            )}
          />
          <div className="grid gap-4 pt-4 md:grid-cols-2">
            <Input
              label={t('proxies.editor.fields.label', { defaultValue: 'Label' })}
              labelPlacement="outside"
              value={editorDraft.label}
              onValueChange={(value) => setEditorDraft((current) => (current ? { ...current, label: value } : current))}
            />
            <Input
              label={t('proxies.editor.fields.weight', { defaultValue: 'Weight' })}
              labelPlacement="outside"
              type="number"
              value={editorDraft.weight}
              onValueChange={(value) => setEditorDraft((current) => (current ? { ...current, weight: value } : current))}
            />
            <Input
              className="md:col-span-2"
              label={t('proxies.editor.fields.proxyUrl', { defaultValue: 'Proxy URL' })}
              labelPlacement="outside"
              value={editorDraft.proxy_url}
              placeholder={t('proxies.editor.proxyUrlPlaceholder', {
                defaultValue: 'http://user:password@127.0.0.1:6152',
              })}
              onValueChange={(value) => setEditorDraft((current) => (current ? { ...current, proxy_url: value } : current))}
            />
            <div className="md:col-span-2">
              <SurfaceInset tone="muted">
                <div className="flex items-center justify-between gap-4">
                  <div>
                    <p className="text-sm font-medium text-foreground">
                      {t('proxies.editor.fields.enabled', { defaultValue: 'Enabled' })}
                    </p>
                    <p className="text-xs text-default-500">
                      {t('proxies.editor.enabledHint', {
                        defaultValue: 'Disabled nodes stay in the list but will not be selected or tested automatically.',
                      })}
                    </p>
                  </div>
                  <Switch
                    isSelected={editorDraft.enabled}
                    onValueChange={(value) => setEditorDraft((current) => (current ? { ...current, enabled: value } : current))}
                    color="primary"
                  />
                </div>
              </SurfaceInset>
            </div>
            <div className="md:col-span-2 flex justify-end gap-3">
              <Button variant="flat" onPress={() => setEditorDraft(null)}>
                {t('common.cancel', { defaultValue: 'Cancel' })}
              </Button>
              <Button
                color="primary"
                startContent={<Save className="h-4 w-4" />}
                onPress={handleSaveNode}
                isLoading={createMutation.isPending || updateMutation.isPending}
                isDisabled={!editorDraft.label.trim() || !editorDraft.proxy_url.trim()}
              >
                {t('proxies.editor.save', { defaultValue: 'Save Changes' })}
              </Button>
            </div>
          </div>
        </PagePanel>
      ) : null}

      <div className="space-y-4">
        <SectionHeader
          title={t('proxies.list.title', { defaultValue: 'Proxy Nodes' })}
          description={t('proxies.list.description', {
            defaultValue: 'Add, edit, delete, and test weighted proxy nodes. The admin API stores secrets but only returns masked URLs.',
          })}
        />

        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {proxyCards.map((proxy) => (
            <PagePanel key={proxy.id}>
              <div className="flex items-start gap-3">
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-primary/10">
                  <Globe className="h-5 w-5 text-primary" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="truncate text-sm font-semibold text-foreground">{proxy.name}</h3>
                    <Chip size="sm" variant="dot" color={statusColorMap[proxy.status]}>
                      {t(`proxies.health.${proxy.status}`, { defaultValue: proxy.status })}
                    </Chip>
                  </div>
                  <p className="truncate text-xs text-default-400">{proxy.endpoint}</p>
                </div>
              </div>
              <div className="space-y-4 pt-4">
                <div className="grid gap-3 text-sm text-default-600">
                  <div className="flex items-center justify-between gap-3">
                    <span className="inline-flex items-center gap-2">
                      <Network className="h-4 w-4 text-default-400" />
                      {t('proxies.antigravity.latency', { defaultValue: 'Latency' })}
                    </span>
                    <span className={`font-mono font-medium ${proxy.latencyMs > 200 ? 'text-warning' : 'text-success'}`}>
                      {formatDurationMs(proxy.latencyMs)}
                    </span>
                  </div>
                  <div className="flex items-center justify-between gap-3">
                    <span>{t('proxies.antigravity.scheme', { defaultValue: 'Scheme' })}</span>
                    <span className="font-mono text-foreground">{proxy.scheme}</span>
                  </div>
                  <div className="flex items-center justify-between gap-3">
                    <span>{t('proxies.columns.weight', { defaultValue: 'Weight' })}</span>
                    <span className="font-mono text-foreground">{proxy.weight}</span>
                  </div>
                  <div className="flex items-center justify-between gap-3">
                    <span>{t('proxies.antigravity.auth', { defaultValue: 'Auth' })}</span>
                    <span className="text-foreground">
                      {proxy.hasAuth
                        ? t('proxies.antigravity.authConfigured', { defaultValue: 'Configured' })
                        : t('proxies.antigravity.authNone', { defaultValue: 'None' })}
                    </span>
                  </div>
                  {proxy.lastError ? (
                    <div className="rounded-xl border border-warning-200 bg-warning-50 px-3 py-2 text-xs text-warning-700">
                      {t('proxies.antigravity.lastErrorSummary', {
                        defaultValue: 'The latest probe failed. Continue the investigation in the unified event stream.',
                      })}
                    </div>
                  ) : null}
                </div>

                <div className="flex flex-wrap gap-2">
                  <Button
                    size="sm"
                    variant="flat"
                    startContent={<PlayCircle className="h-4 w-4" />}
                    onPress={() => testOneMutation.mutate(proxy.id)}
                    isLoading={pendingProxyId === proxy.id}
                  >
                    {t('proxies.actions.test', { defaultValue: 'Test' })}
                  </Button>
                  <Button
                    size="sm"
                    variant="flat"
                    startContent={<Pencil className="h-4 w-4" />}
                    onPress={() => beginEdit(proxy.id)}
                  >
                    {t('proxies.actions.edit', { defaultValue: 'Edit' })}
                  </Button>
                  <Button
                    size="sm"
                    variant="flat"
                    color="danger"
                    startContent={<Trash2 className="h-4 w-4" />}
                    onPress={() => handleDeleteNode(proxy.id)}
                    isLoading={deleteMutation.isPending && pendingProxyId === proxy.id}
                  >
                    {t('proxies.actions.delete', { defaultValue: 'Delete' })}
                  </Button>
                </div>
              </div>
            </PagePanel>
          ))}
        </div>
      </div>

      {proxyCards.length === 0 && !isFetching ? (
        <PagePanel tone="secondary" className="border-dashed py-10">
          <div className="py-6 text-center">
            <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-2xl bg-default-100">
              <Globe className="h-6 w-6 text-default-500" />
            </div>
            <h3 className="mt-4 text-lg font-semibold text-foreground">
              {t('proxies.empty', { defaultValue: 'No outbound proxies configured yet.' })}
            </h3>
            <p className="mt-2 text-sm text-default-600">
              {t('proxies.antigravity.emptyDescription', {
                defaultValue: 'Add the first proxy node to start routing traffic through the admin-managed pool.',
              })}
            </p>
          </div>
        </PagePanel>
      ) : null}
    </PageContent>
  )
}
