import { useMemo, useState } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Activity, Server } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { proxiesApi, type ProxyNode } from '@/api/proxies'
import {
  PageIntro,
  PagePanel,
} from '@/components/layout/page-archetypes'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { LoadingOverlay } from '@/components/ui/loading-overlay'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { describeProxiesWorkspaceLayout } from '@/lib/page-archetypes'
import { cn } from '@/lib/utils'
import { formatRelativeTime } from '@/lib/time'

type ProxyFilter = 'all' | 'healthy' | 'degraded' | 'offline' | 'disabled'

function resolveProxyHealth(proxy: ProxyNode): ProxyFilter {
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

function matchesProxySearch(proxy: ProxyNode, keyword: string) {
  return [proxy.label, proxy.base_url, proxy.last_error].some((item) =>
    item?.toLowerCase().includes(keyword),
  )
}

export default function Proxies() {
  const { t, i18n } = useTranslation()
  const queryClient = useQueryClient()
  const [healthFilter, setHealthFilter] = useState<ProxyFilter>('all')
  const [pendingProxyId, setPendingProxyId] = useState<string | null>(null)
  const [searchKeyword, setSearchKeyword] = useState('')
  const [compactMode, setCompactMode] = useState<boolean>(() => {
    const raw = localStorage.getItem('cp.proxies.compact')
    return raw === '1'
  })

  const { data: proxies = [], isLoading, isFetching } = useQuery({
    queryKey: ['proxyNodes'],
    queryFn: proxiesApi.listProxies,
    refetchInterval: 15000,
  })

  const healthCheckMutation = useMutation({
    mutationFn: proxiesApi.testAll,
    onSuccess: (payload) => {
      queryClient.setQueryData(['proxyNodes'], payload.results)
    },
  })

  const filteredData = useMemo(() => {
    const normalizedKeyword = searchKeyword.trim().toLowerCase()
    return proxies.filter((proxy) => {
      if (healthFilter !== 'all' && resolveProxyHealth(proxy) !== healthFilter) {
        return false
      }
      if (!normalizedKeyword) {
        return true
      }
      return matchesProxySearch(proxy, normalizedKeyword)
    })
  }, [healthFilter, proxies, searchKeyword])
  const proxiesLayout = describeProxiesWorkspaceLayout()
  const tableSurfaceClassName = 'border-0 bg-transparent shadow-none'

  const columns = useMemo<ColumnDef<ProxyNode>[]>(
    () => [
      {
        accessorKey: 'base_url',
        header: t('proxies.columns.url'),
        cell: ({ row }) => {
          const isOnline = row.original.enabled && row.original.last_test_status !== 'error'
          return (
            <div className="flex min-w-[220px] items-center gap-2">
              <Server className={cn('h-4 w-4', isOnline ? 'text-primary' : 'text-muted-foreground')} />
              <span className="max-w-[300px] truncate font-mono text-sm font-medium" title={row.original.base_url}>
                {row.original.base_url.replace(/^https?:\/\//, '')}
              </span>
            </div>
          )
        },
      },
      {
        id: 'health',
        accessorFn: (row) => resolveProxyHealth(row),
        header: t('proxies.columns.health'),
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
        accessorKey: 'last_latency_ms',
        header: t('proxies.columns.latency'),
        cell: ({ row }) => (
          <span className="font-mono text-sm tabular-nums">
            {typeof row.original.last_latency_ms === 'number' ? `${row.original.last_latency_ms}ms` : '-'}
          </span>
        ),
      },
      {
        accessorKey: 'updated_at',
        header: t('proxies.columns.lastPing'),
        cell: ({ row }) => (
          <span className="text-sm text-muted-foreground">
            {row.original.updated_at
              ? formatRelativeTime(row.original.updated_at, i18n.resolvedLanguage, true)
              : t('proxies.pending')}
          </span>
        ),
      },
      {
        id: 'actions',
        enableSorting: false,
        header: t('proxies.columns.actions'),
        cell: ({ row }) => (
          <Button
            variant="ghost"
            size="sm"
            onClick={async () => {
              setPendingProxyId(row.original.id)
              try {
                const payload = await proxiesApi.testProxy(row.original.id)
                queryClient.setQueryData(['proxyNodes'], payload.results)
              } finally {
                setPendingProxyId(null)
              }
            }}
            disabled={pendingProxyId === row.original.id}
          >
            {row.original.last_test_status === 'error' ? t('proxies.retry') : t('proxies.manage')}
          </Button>
        ),
      },
    ],
    [i18n.resolvedLanguage, pendingProxyId, queryClient, t],
  )

  return (
    <div className="flex-1 p-4 sm:p-6 lg:p-8">
      <div className="space-y-6 md:space-y-8">
        <PageIntro
          archetype="workspace"
          title={t('proxies.title')}
          description={t('proxies.subtitle')}
          actions={
            <Button
              onClick={() => healthCheckMutation.mutate()}
              disabled={isFetching || healthCheckMutation.isPending}
            >
              <Activity className={cn('mr-2 h-4 w-4', (isFetching || healthCheckMutation.isPending) && 'animate-spin')} />
              {t('proxies.check')}
            </Button>
          }
        />

        {proxiesLayout.mobileControlsPlacement === 'after-intro' ? (
          <PagePanel tone="secondary" className="space-y-4">
            <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
              <div className="grid gap-3 md:grid-cols-[minmax(0,14rem)_minmax(0,1fr)]">
                {proxiesLayout.filterPlacement === 'within-controls-panel' ? (
                  <div className="space-y-1.5">
                    <label className="text-xs font-medium text-muted-foreground">
                      {t('proxies.filters.label')}
                    </label>
                    <Select value={healthFilter} onValueChange={(value) => setHealthFilter(value as ProxyFilter)}>
                      <SelectTrigger className="w-full" aria-label={t('proxies.filters.label')}>
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
                ) : null}

                <div className="space-y-1.5">
                  <label htmlFor="proxies-search" className="text-xs font-medium text-muted-foreground">
                    {t('common.table.searchLabel')}
                  </label>
                  <Input
                    id="proxies-search"
                    name="proxies_search"
                    value={searchKeyword}
                    onChange={(event) => setSearchKeyword(event.target.value)}
                    placeholder={t('proxies.searchPlaceholder')}
                    autoComplete="off"
                  />
                </div>
              </div>

              {proxiesLayout.densityPlacement === 'within-controls-panel' ? (
                <div className="flex flex-wrap items-center gap-2 lg:justify-end">
                  <Button
                    variant="outline"
                    onClick={() => {
                      const next = !compactMode
                      setCompactMode(next)
                      localStorage.setItem('cp.proxies.compact', next ? '1' : '0')
                    }}
                  >
                    {compactMode ? t('accounts.actions.comfortableMode') : t('accounts.actions.compactMode')}
                  </Button>
                </div>
              ) : null}
            </div>
          </PagePanel>
        ) : null}

        <PagePanel className="relative overflow-hidden p-0">
          <LoadingOverlay
            show={isLoading}
            title={t('proxies.loading')}
            description={t('common.loading')}
          />

          <StandardDataTable
            columns={columns}
            data={filteredData}
            className={tableSurfaceClassName}
            density={compactMode ? 'compact' : 'comfortable'}
            enableSearch={false}
            emptyText={t('proxies.empty')}
          />
        </PagePanel>
      </div>
    </div>
  )
}
