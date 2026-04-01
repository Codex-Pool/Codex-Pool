import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import {
  Button,
  Chip,
  Divider,
  Input,
  Modal,
  ModalBody,
  ModalContent,
  ModalHeader,
  Pagination,
  Select,
  SelectItem,
  Spinner,
  Table,
  TableBody,
  TableCell,
  TableColumn,
  TableHeader,
  TableRow,
  type Selection,
} from '@heroui/react'
import type { TFunction } from 'i18next'
import {
  Activity,
  GitBranch,
  RefreshCcw,
  Search,
  ShieldAlert,
  Unplug,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'

import {
  eventStreamApi,
  type SystemEventCategory,
  type SystemEventItem,
  type SystemEventSeverity,
} from '@/api/eventStream'
import {
  DockedPageIntro,
  PageContent,
} from '@/components/layout/page-archetypes'
import { Card, CardBody, CardHeader } from '@/components/ui/card'
import { formatDurationMs } from '@/lib/duration-format'
import { cn } from '@/lib/utils'

type RangePreset = '1' | '7' | '30'
type CategoryFilter = 'all' | SystemEventCategory
type SeverityFilter = 'all' | SystemEventSeverity

const TABLE_PAGE_SIZE_OPTIONS = [10, 20, 50]

function normalizeSelection(selection: Selection) {
  if (selection === 'all') {
    return ''
  }

  const [first] = Array.from(selection)
  return first === undefined ? '' : String(first)
}

function currentRange(days: number) {
  const endTs = Math.floor(Date.now() / 1000)
  const startTs = endTs - days * 24 * 60 * 60
  return { start_ts: startTs, end_ts: endTs }
}

function formatDateTime(value?: string) {
  if (!value) {
    return '-'
  }
  const parsed = new Date(value)
  return Number.isNaN(parsed.getTime()) ? '-' : parsed.toLocaleString()
}

function normalizeEventValue(value?: string | null) {
  return (value ?? '').trim().toLowerCase()
}

function getCategoryLabel(
  category: SystemEventCategory,
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (category) {
    case 'request':
      return t('logs.events.categories.request')
    case 'account_pool':
      return t('logs.events.categories.accountPool')
    case 'patrol':
      return t('logs.events.categories.patrol')
    case 'import':
      return t('logs.events.categories.import')
    case 'infra':
      return t('logs.events.categories.infra')
    case 'admin_action':
      return t('logs.events.categories.adminAction')
    default:
      return t('logs.events.categories.unknown')
  }
}

function getCategoryColor(category: SystemEventCategory) {
  switch (category) {
    case 'request':
      return 'primary' as const
    case 'account_pool':
      return 'warning' as const
    case 'patrol':
      return 'secondary' as const
    case 'import':
      return 'success' as const
    case 'infra':
      return 'danger' as const
    case 'admin_action':
    default:
      return 'default' as const
  }
}

function getSeverityLabel(
  severity: SystemEventSeverity,
  t: ReturnType<typeof useTranslation>['t'],
) {
  switch (severity) {
    case 'debug':
      return t('logs.events.severities.debug')
    case 'info':
      return t('logs.events.severities.info')
    case 'warn':
      return t('logs.events.severities.warn')
    case 'error':
      return t('logs.events.severities.error')
    default:
      return t('logs.events.severities.unknown')
  }
}

function getSeverityColor(severity: SystemEventSeverity) {
  switch (severity) {
    case 'error':
      return 'danger' as const
    case 'warn':
      return 'warning' as const
    case 'info':
      return 'primary' as const
    case 'debug':
    default:
      return 'default' as const
  }
}

function localizeEventType(eventType: string | undefined, t: TFunction) {
  const normalized = normalizeEventValue(eventType)
  if (!normalized) {
    return t('logs.events.eventTypes.unknown')
  }
  if (normalized.startsWith('upstream_event:')) {
    return t('logs.events.eventTypes.upstreamEvent')
  }

  switch (normalized) {
    case 'request_received':
      return t('logs.events.eventTypes.requestReceived')
    case 'request_completed':
      return t('logs.events.eventTypes.requestCompleted')
    case 'request_failed':
      return t('logs.events.eventTypes.requestFailed')
    case 'routing_candidate_selected':
      return t('logs.events.eventTypes.routingCandidateSelected')
    case 'same_account_retry':
      return t('logs.events.eventTypes.sameAccountRetry')
    case 'cross_account_failover':
      return t('logs.events.eventTypes.crossAccountFailover')
    case 'continuation_cursor_saved':
      return t('logs.events.eventTypes.continuationCursorSaved')
    case 'continuation_cursor_restored':
      return t('logs.events.eventTypes.continuationCursorRestored')
    case 'ws_http_fallback':
      return t('logs.events.eventTypes.wsHttpFallback')
    case 'proxy_selection_failed':
      return t('logs.events.eventTypes.proxySelectionFailed')
    case 'probe_succeeded':
      return t('logs.events.eventTypes.probeSucceeded')
    case 'active_patrol_batch_completed':
      return t('logs.events.eventTypes.activePatrolBatchCompleted')
    case 'rate_limit_refresh_batch_completed':
      return t('logs.events.eventTypes.rateLimitRefreshBatchCompleted')
    case 'pending_delete_batch_completed':
      return t('logs.events.eventTypes.pendingDeleteBatchCompleted')
    case 'account_pool_state_transition':
      return t('logs.events.eventTypes.accountPoolStateTransition')
    case 'account_deleted':
      return t('logs.events.eventTypes.accountDeleted')
    case 'import_job_created':
      return t('logs.events.eventTypes.importJobCreated')
    case 'import_job_completed':
      return t('logs.events.eventTypes.importJobCompleted')
    case 'import_job_failed':
      return t('logs.events.eventTypes.importJobFailed')
    default:
      return t('logs.events.eventTypes.unknown')
  }
}

function localizeReasonCode(reasonCode: string | undefined, t: TFunction) {
  const normalized = normalizeEventValue(reasonCode)
  if (!normalized) {
    return '-'
  }

  switch (normalized) {
    case 'request_received':
      return t('logs.events.reasonCodes.requestReceived')
    case 'routing_candidate_selected':
      return t('logs.events.reasonCodes.routingCandidateSelected')
    case 'rate_limited':
      return t('logs.events.reasonCodes.rateLimited')
    case 'transport_error':
      return t('logs.events.reasonCodes.transportError')
    case 'proxy_unavailable':
      return t('logs.events.reasonCodes.proxyUnavailable')
    case 'continuation_cursor_restored':
      return t('logs.events.reasonCodes.continuationCursorRestored')
    case 'rate_limit_refresh_batch_completed':
      return t('logs.events.reasonCodes.rateLimitRefreshBatchCompleted')
    case 'pending_delete_batch_completed':
      return t('logs.events.reasonCodes.pendingDeleteBatchCompleted')
    case 'account_deactivated':
      return t('logs.events.reasonCodes.accountDeactivated')
    case 'previous_response_not_found':
      return t('logs.events.reasonCodes.previousResponseNotFound')
    case 'upstream_request_failed':
      return t('logs.events.reasonCodes.upstreamRequestFailed')
    case 'invalid_refresh_token':
      return t('logs.events.reasonCodes.invalidRefreshToken')
    case 'refresh_token_reused':
      return t('logs.events.reasonCodes.refreshTokenReused')
    case 'upstream_unavailable':
      return t('logs.events.reasonCodes.upstreamUnavailable')
    default:
      return t('logs.events.reasonCodes.unknown')
  }
}

function localizeReasonClass(reasonClass: string | undefined, t: TFunction) {
  switch (normalizeEventValue(reasonClass)) {
    case 'healthy':
      return t('logs.events.reasonClasses.healthy')
    case 'quota':
      return t('logs.events.reasonClasses.quota')
    case 'fatal':
      return t('logs.events.reasonClasses.fatal')
    case 'transient':
      return t('logs.events.reasonClasses.transient')
    case 'admin':
      return t('logs.events.reasonClasses.admin')
    case '':
      return '-'
    default:
      return t('logs.events.reasonClasses.unknown')
  }
}

function localizeRoutingDecision(routingDecision: string | undefined, t: TFunction) {
  switch (normalizeEventValue(routingDecision)) {
    case 'recent_success':
      return t('logs.events.routingDecisions.recentSuccess')
    case 'fresh_probe':
      return t('logs.events.routingDecisions.freshProbe')
    case 'round_robin':
    case 'router_round_robin':
      return t('logs.events.routingDecisions.roundRobin')
    case 'same_account_retry':
      return t('logs.events.routingDecisions.sameAccountRetry')
    case 'cross_account_failover':
      return t('logs.events.routingDecisions.crossAccountFailover')
    case 'request_received':
      return t('logs.events.routingDecisions.requestReceived')
    case '':
      return '-'
    default:
      return t('logs.events.routingDecisions.unknown')
  }
}

function localizeAuthProvider(authProvider: string | undefined, t: TFunction) {
  switch (normalizeEventValue(authProvider)) {
    case 'oauth_refresh_token':
      return t('logs.events.authProviders.oauthRefreshToken')
    case 'legacy_bearer':
      return t('logs.events.authProviders.legacyBearer')
    case 'codex_oauth':
      return t('logs.events.authProviders.codexOauth')
    case '':
      return '-'
    default:
      return t('logs.events.authProviders.unknown')
  }
}

function buildEventHeadline(item: SystemEventItem, t: TFunction) {
  return item.message?.trim() || item.preview_text?.trim() || localizeEventType(item.event_type, t)
}

function matchEventSearch(item: SystemEventItem, keyword: string) {
  const haystack = [
    item.event_type,
    item.message,
    item.preview_text,
    item.request_id,
    item.account_label,
    item.reason_code,
    item.path,
    item.model,
  ]
    .filter(Boolean)
    .join(' ')
    .toLowerCase()

  return haystack.includes(keyword)
}

export default function Logs() {
  const { t } = useTranslation()
  const [rangePreset, setRangePreset] = useState<RangePreset>('7')
  const [categoryFilter, setCategoryFilter] = useState<CategoryFilter>('all')
  const [severityFilter, setSeverityFilter] = useState<SeverityFilter>('all')
  const [searchValue, setSearchValue] = useState('')
  const [rowsPerPage, setRowsPerPage] = useState(20)
  const [currentPage, setCurrentPage] = useState(1)
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null)

  const range = useMemo(() => currentRange(Number(rangePreset)), [rangePreset])
  const queryParams = useMemo(
    () => ({
      ...range,
      limit: 200,
      category: categoryFilter === 'all' ? undefined : categoryFilter,
      severity: severityFilter === 'all' ? undefined : severityFilter,
    }),
    [categoryFilter, range, severityFilter],
  )

  const { data: eventsResponse, isLoading, refetch, isFetching } = useQuery({
    queryKey: ['systemEvents', queryParams],
    queryFn: () => eventStreamApi.adminList(queryParams),
    refetchInterval: 30_000,
  })

  const { data: summary } = useQuery({
    queryKey: ['systemEventSummary', queryParams],
    queryFn: () => eventStreamApi.adminSummary(queryParams),
    refetchInterval: 30_000,
  })

  const { data: eventDetail, isFetching: isFetchingDetail } = useQuery({
    queryKey: ['systemEventDetail', selectedEventId],
    queryFn: () => eventStreamApi.adminDetail(selectedEventId!),
    enabled: Boolean(selectedEventId),
  })

  const selectedRequestId = eventDetail?.item.request_id

  const { data: correlation, isFetching: isFetchingCorrelation } = useQuery({
    queryKey: ['systemEventCorrelation', selectedRequestId],
    queryFn: () => eventStreamApi.adminCorrelation(selectedRequestId!),
    enabled: Boolean(selectedRequestId),
  })

  const events = useMemo(() => eventsResponse?.items ?? [], [eventsResponse?.items])
  const filteredEvents = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase()

    if (!keyword) {
      return events
    }

    return events.filter((item) => matchEventSearch(item, keyword))
  }, [events, searchValue])

  const totalPages = Math.max(1, Math.ceil(filteredEvents.length / rowsPerPage))
  const resolvedPage = Math.min(currentPage, totalPages)
  const paginatedEvents = useMemo(() => {
    const start = (resolvedPage - 1) * rowsPerPage
    return filteredEvents.slice(start, start + rowsPerPage)
  }, [filteredEvents, resolvedPage, rowsPerPage])
  const visibleRangeStart = filteredEvents.length === 0 ? 0 : (resolvedPage - 1) * rowsPerPage + 1
  const visibleRangeEnd =
    filteredEvents.length === 0 ? 0 : Math.min(filteredEvents.length, resolvedPage * rowsPerPage)

  const selectedEvent = eventDetail?.item
  const topReasonCodes = summary?.by_reason_code.slice(0, 4) ?? []
  const topEventTypes = summary?.by_event_type.slice(0, 4) ?? []
  const metricCards = [
    {
      title: t('logs.events.metrics.total'),
      value: String(summary?.total ?? events.length),
      description: t('logs.events.metrics.totalDesc'),
      icon: <Activity className="h-4 w-4" />,
      tone: 'primary' as const,
    },
    {
      title: t('logs.events.metrics.error'),
      value: String(summary?.by_severity.find((item) => item.severity === 'error')?.count ?? 0),
      description: t('logs.events.metrics.errorDesc'),
      icon: <ShieldAlert className="h-4 w-4" />,
      tone: 'danger' as const,
    },
    {
      title: t('logs.events.metrics.accountPool'),
      value: String(summary?.by_category.find((item) => item.category === 'account_pool')?.count ?? 0),
      description: t('logs.events.metrics.accountPoolDesc'),
      icon: <GitBranch className="h-4 w-4" />,
      tone: 'warning' as const,
    },
    {
      title: t('logs.events.metrics.request'),
      value: String(summary?.by_category.find((item) => item.category === 'request')?.count ?? 0),
      description: t('logs.events.metrics.requestDesc'),
      icon: <Activity className="h-4 w-4" />,
      tone: 'secondary' as const,
    },
  ]

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t('nav.logs')}
        description={t('logs.events.description')}
      />

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,0.95fr)] xl:items-start">
        <Card className="xl:self-start">
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('logs.events.summaryTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('logs.events.summaryDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="grid gap-3 px-5 pb-5 pt-1 sm:grid-cols-2 xl:grid-cols-2">
            {metricCards.map((metric) => (
              <div
                key={metric.title}
                className="rounded-large border border-default-200 bg-content2/55 px-4 py-4"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className={cn(
                    'flex h-10 w-10 items-center justify-center rounded-large',
                    metric.tone === 'danger' && 'bg-danger/10 text-danger',
                    metric.tone === 'warning' && 'bg-warning/10 text-warning',
                    metric.tone === 'secondary' && 'bg-secondary/10 text-secondary',
                    metric.tone === 'primary' && 'bg-primary/10 text-primary',
                  )}
                  >
                    {metric.icon}
                  </div>
                </div>
                <div className="mt-5 text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {metric.title}
                </div>
                <div className="mt-2 text-3xl font-semibold tracking-[-0.04em] text-foreground">
                  {metric.value}
                </div>
                <p className="mt-2 text-xs leading-5 text-default-500">
                  {metric.description}
                </p>
              </div>
            ))}
          </CardBody>
        </Card>

        <Card className="border-small border-default-200 bg-[linear-gradient(135deg,hsl(var(--heroui-primary)/0.08),transparent_55%),linear-gradient(180deg,hsl(0_0%_100%/0.98),hsl(0_0%_100%/0.9))] shadow-small dark:bg-[linear-gradient(135deg,hsl(var(--heroui-primary)/0.12),transparent_55%),linear-gradient(180deg,hsl(220_13%_10%/0.92),hsl(222_14%_8%/0.9))]">
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('logs.events.insightsTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('logs.events.insightsDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="gap-5 px-5 pb-5 pt-1">
            <div className="space-y-3">
              <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                {t('logs.events.topEventTypes')}
              </div>
              {topEventTypes.length > 0 ? (
                topEventTypes.map((entry) => (
                  <div
                    key={entry.event_type}
                    className="flex items-center justify-between gap-3 rounded-large border border-default-200 bg-content1/80 px-3 py-3"
                  >
                    <div className="min-w-0">
                      <div className="truncate text-sm font-medium text-foreground">
                        {localizeEventType(entry.event_type, t)}
                      </div>
                    </div>
                    <Chip size="sm" variant="flat">
                      {entry.count}
                    </Chip>
                  </div>
                ))
              ) : (
                <div className="rounded-large border border-dashed border-default-200 px-4 py-5 text-sm text-default-600">
                  {t('logs.events.noInsights')}
                </div>
              )}
            </div>

            <div className="space-y-3">
              <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                {t('logs.events.topReasons')}
              </div>
              {topReasonCodes.length > 0 ? (
                topReasonCodes.map((entry) => (
                  <div
                    key={entry.reason_code}
                    className="flex items-center justify-between gap-3 rounded-large border border-default-200 bg-content1/80 px-3 py-3"
                  >
                    <div className="min-w-0">
                      <div className="truncate text-sm font-medium text-foreground">
                        {localizeReasonCode(entry.reason_code, t)}
                      </div>
                    </div>
                    <Chip size="sm" variant="flat">
                      {entry.count}
                    </Chip>
                  </div>
                ))
              ) : (
                <div className="rounded-large border border-dashed border-default-200 px-4 py-5 text-sm text-default-600">
                  {t('logs.events.noInsights')}
                </div>
              )}
            </div>
          </CardBody>
        </Card>
      </div>

      <Card>
        <CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5 xl:flex-row xl:justify-between">
          <div className="space-y-1">
            <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
              {t('logs.events.tableTitle')}
            </h2>
            <p className="text-sm leading-6 text-default-600">
              {t('logs.events.tableDescription')}
            </p>
            <p className="text-sm leading-6 text-default-600">
              {t('logs.events.meta')}
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Chip size="sm" variant="flat">
              {filteredEvents.length}
            </Chip>
            <Button
              isLoading={isFetching}
              size="sm"
              startContent={<RefreshCcw className="h-4 w-4" />}
              variant="flat"
              onPress={() => void refetch()}
            >
              {t('common.refresh')}
            </Button>
          </div>
        </CardHeader>

        <CardBody className="gap-4 px-5 pb-5 pt-1">
          <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
            <div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:flex-wrap">
              <Input
                aria-label={t('common.table.searchLabel')}
                className="w-full sm:w-[300px]"
                isClearable
                placeholder={t('logs.events.searchPlaceholder')}
                startContent={<Search className="h-4 w-4 text-default-400" />}
                value={searchValue}
                onClear={() => {
                  setCurrentPage(1)
                  setSearchValue('')
                }}
                onValueChange={(value) => {
                  setCurrentPage(1)
                  setSearchValue(value)
                }}
              />
              <Select
                aria-label={t('logs.events.filters.range')}
                className="w-full sm:w-[170px]"
                selectedKeys={[rangePreset]}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (nextValue) {
                    setCurrentPage(1)
                    setRangePreset(nextValue as RangePreset)
                  }
                }}
              >
                <SelectItem key="1">{t('logs.range.last24Hours')}</SelectItem>
                <SelectItem key="7">{t('logs.range.last7Days')}</SelectItem>
                <SelectItem key="30">{t('logs.range.last30Days')}</SelectItem>
              </Select>
              <Select
                aria-label={t('logs.events.filters.category')}
                className="w-full sm:w-[180px]"
                selectedKeys={[categoryFilter]}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (nextValue) {
                    setCurrentPage(1)
                    setCategoryFilter(nextValue as CategoryFilter)
                  }
                }}
              >
                <SelectItem key="all">{t('logs.events.filters.allCategories')}</SelectItem>
                <SelectItem key="request">{getCategoryLabel('request', t)}</SelectItem>
                <SelectItem key="account_pool">{getCategoryLabel('account_pool', t)}</SelectItem>
                <SelectItem key="patrol">{getCategoryLabel('patrol', t)}</SelectItem>
                <SelectItem key="import">{getCategoryLabel('import', t)}</SelectItem>
                <SelectItem key="infra">{getCategoryLabel('infra', t)}</SelectItem>
                <SelectItem key="admin_action">{getCategoryLabel('admin_action', t)}</SelectItem>
              </Select>
              <Select
                aria-label={t('logs.events.filters.severity')}
                className="w-full sm:w-[170px]"
                selectedKeys={[severityFilter]}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (nextValue) {
                    setCurrentPage(1)
                    setSeverityFilter(nextValue as SeverityFilter)
                  }
                }}
              >
                <SelectItem key="all">{t('logs.events.filters.allSeverities')}</SelectItem>
                <SelectItem key="debug">{getSeverityLabel('debug', t)}</SelectItem>
                <SelectItem key="info">{getSeverityLabel('info', t)}</SelectItem>
                <SelectItem key="warn">{getSeverityLabel('warn', t)}</SelectItem>
                <SelectItem key="error">{getSeverityLabel('error', t)}</SelectItem>
              </Select>
            </div>

            <div className="flex items-center gap-2 text-xs text-default-500">
              <span>{t('common.table.rowsPerPage')}</span>
              <Select
                aria-label={t('common.table.rowsPerPage')}
                className="w-[106px]"
                selectedKeys={[String(rowsPerPage)]}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection)
                  if (!nextValue) {
                    return
                  }
                  setCurrentPage(1)
                  setRowsPerPage(Number(nextValue))
                }}
              >
                {TABLE_PAGE_SIZE_OPTIONS.map((size) => (
                  <SelectItem key={String(size)}>{size}</SelectItem>
                ))}
              </Select>
            </div>
          </div>

          <Table
            isHeaderSticky
            aria-label={t('logs.events.tableTitle')}
            classNames={{
              base: 'min-h-[26rem]',
              wrapper: 'bg-transparent px-0 py-0 shadow-none',
              th: 'bg-default-100/60 text-xs font-semibold uppercase tracking-[0.12em] text-default-500',
              td: 'align-top py-4 text-sm text-foreground',
              tr: 'data-[hover=true]:bg-content2/35 transition-colors',
              emptyWrapper: 'h-56',
            }}
          >
            <TableHeader>
              <TableColumn>{t('logs.events.columns.time')}</TableColumn>
              <TableColumn>{t('logs.events.columns.category')}</TableColumn>
              <TableColumn>{t('logs.events.columns.severity')}</TableColumn>
              <TableColumn>{t('logs.events.columns.event')}</TableColumn>
              <TableColumn>{t('logs.events.columns.context')}</TableColumn>
              <TableColumn>{t('common.actions')}</TableColumn>
            </TableHeader>
            <TableBody
              emptyContent={(
                <div className="flex flex-col items-center gap-3 py-12 text-default-500">
                  <Unplug className="h-10 w-10 opacity-35" />
                  <div className="text-sm font-medium">{t('logs.events.empty')}</div>
                </div>
              )}
              isLoading={isLoading}
              items={paginatedEvents}
              loadingContent={<Spinner label={t('common.loading')} />}
            >
              {(item) => (
                <TableRow key={item.id}>
                  <TableCell>
                    <div className="min-w-[150px] font-mono text-xs text-default-500">
                      {formatDateTime(item.ts)}
                    </div>
                  </TableCell>
                  <TableCell>
                    <Chip color={getCategoryColor(item.category)} size="sm" variant="flat">
                      {getCategoryLabel(item.category, t)}
                    </Chip>
                  </TableCell>
                  <TableCell>
                    <Chip color={getSeverityColor(item.severity)} size="sm" variant="flat">
                      {getSeverityLabel(item.severity, t)}
                    </Chip>
                  </TableCell>
                  <TableCell>
                    <div className="min-w-[280px] space-y-1">
                      <div className="font-medium text-foreground">{localizeEventType(item.event_type, t)}</div>
                      <div className="text-xs leading-5 text-default-500">{buildEventHeadline(item, t)}</div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="min-w-[240px] space-y-1 text-xs leading-5 text-default-500">
                      <div>{t('logs.events.fields.requestId')}: {item.request_id ?? '-'}</div>
                      <div>{t('logs.events.fields.account')}: {item.account_label ?? item.account_id ?? '-'}</div>
                      <div>{t('logs.events.fields.reasonCode')}: {localizeReasonCode(item.reason_code, t)}</div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <Button
                      size="sm"
                      variant="flat"
                      onPress={() => setSelectedEventId(item.id)}
                    >
                      {t('logs.events.actions.inspect')}
                    </Button>
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>

          <div className="flex flex-col gap-3 border-t border-default-200 pt-3 text-xs text-default-500 sm:flex-row sm:items-center sm:justify-between">
            <div className="tabular-nums">
              {t('common.table.range', {
                start: visibleRangeStart,
                end: visibleRangeEnd,
                total: filteredEvents.length,
              })}
            </div>
            <Pagination
              color="primary"
              isCompact
              page={resolvedPage}
              total={totalPages}
              onChange={setCurrentPage}
            />
          </div>
        </CardBody>
      </Card>

      <Modal
        backdrop="blur"
        classNames={{
          base: 'border-small border-default-200 bg-content1 shadow-large',
          body: 'pt-0',
          backdrop: 'bg-black/52 backdrop-blur-[2px]',
          wrapper: 'px-2 py-2 sm:px-6 sm:py-6',
        }}
        isOpen={Boolean(selectedEventId)}
        placement="center"
        scrollBehavior="inside"
        size="5xl"
        onOpenChange={(open) => {
          if (!open) {
            setSelectedEventId(null)
          }
        }}
      >
        <ModalContent>
          {() => (
            <>
              <ModalHeader className="flex flex-col gap-1">
                <div className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {selectedEvent ? localizeEventType(selectedEvent.event_type, t) : t('logs.events.detailTitle')}
                </div>
                <div className="text-sm leading-6 text-default-600">
                  {selectedEvent?.request_id
                    ? t('logs.events.detailDescriptionWithRequest', { requestId: selectedEvent.request_id })
                    : t('logs.events.detailDescription')}
                </div>
              </ModalHeader>
              <ModalBody className="pb-5">
                {selectedEvent ? (
                  <div className="space-y-5">
                    <div className="grid gap-4 lg:grid-cols-2">
                      <Card className="border-small border-default-200 bg-content2/55 shadow-none">
                        <CardBody className="gap-4 p-4">
                          <div className="flex flex-wrap gap-2">
                            <Chip color={getCategoryColor(selectedEvent.category)} size="sm" variant="flat">
                              {getCategoryLabel(selectedEvent.category, t)}
                            </Chip>
                            <Chip color={getSeverityColor(selectedEvent.severity)} size="sm" variant="flat">
                              {getSeverityLabel(selectedEvent.severity, t)}
                            </Chip>
                          </div>
                          <div className="grid gap-2 text-sm text-default-600">
                            <div>{t('logs.events.fields.time')}: {formatDateTime(selectedEvent.ts)}</div>
                            <div>{t('logs.events.fields.path')}: {selectedEvent.path ?? '-'}</div>
                            <div>{t('logs.events.fields.method')}: {selectedEvent.method ?? '-'}</div>
                            <div>{t('logs.events.fields.requestId')}: {selectedEvent.request_id ?? '-'}</div>
                            <div>{t('logs.events.fields.statusCode')}: {selectedEvent.status_code ?? '-'}</div>
                            <div>{t('logs.events.fields.upstreamStatusCode')}: {selectedEvent.upstream_status_code ?? '-'}</div>
                            <div>{t('logs.events.fields.latency')}: {formatDurationMs(selectedEvent.latency_ms)}</div>
                            <div>{t('logs.events.fields.jobId')}: {selectedEvent.job_id ?? '-'}</div>
                          </div>
                        </CardBody>
                      </Card>

                      <Card className="border-small border-default-200 bg-content2/55 shadow-none">
                        <CardBody className="gap-4 p-4">
                          <div className="text-sm font-medium text-foreground">
                            {t('logs.events.previewTitle')}
                          </div>
                          <div className="text-sm leading-6 text-default-600">
                            {buildEventHeadline(selectedEvent, t)}
                          </div>
                          <Divider />
                          <div className="grid gap-2 text-sm text-default-600">
                            <div>{t('logs.events.fields.account')}: {selectedEvent.account_label ?? selectedEvent.account_id ?? '-'}</div>
                            <div>{t('logs.events.fields.tenant')}: {selectedEvent.tenant_id ?? '-'}</div>
                            <div>{t('logs.events.fields.model')}: {selectedEvent.model ?? '-'}</div>
                            <div>{t('logs.events.fields.authProvider')}: {localizeAuthProvider(selectedEvent.auth_provider, t)}</div>
                            <div>{t('logs.events.fields.reasonClass')}: {localizeReasonClass(selectedEvent.reason_class, t)}</div>
                            <div>{t('logs.events.fields.routeDecision')}: {localizeRoutingDecision(selectedEvent.routing_decision, t)}</div>
                            <div>{t('logs.events.fields.nextActionAt')}: {formatDateTime(selectedEvent.next_action_at)}</div>
                          </div>
                        </CardBody>
                      </Card>
                    </div>

                    <Card className="border-small border-default-200 bg-content1 shadow-none">
                      <CardHeader className="px-4 pb-3 pt-4">
                        <div className="space-y-1">
                          <h3 className="text-base font-semibold tracking-[-0.02em] text-foreground">
                            {t('logs.events.payloadTitle')}
                          </h3>
                          <p className="text-sm leading-6 text-default-600">
                            {t('logs.events.payloadDescription')}
                          </p>
                        </div>
                      </CardHeader>
                      <CardBody className="px-4 pb-4 pt-0">
                        <pre className="overflow-x-auto rounded-large border border-default-200 bg-content2/65 p-4 text-xs leading-6 text-default-700 dark:text-default-300">
                          {JSON.stringify(selectedEvent.payload_json ?? selectedEvent, null, 2)}
                        </pre>
                      </CardBody>
                    </Card>

                    <Card className="border-small border-default-200 bg-content1 shadow-none">
                      <CardHeader className="px-4 pb-3 pt-4">
                        <div className="space-y-1">
                          <h3 className="text-base font-semibold tracking-[-0.02em] text-foreground">
                            {t('logs.events.correlationTitle')}
                          </h3>
                          <p className="text-sm leading-6 text-default-600">
                            {t('logs.events.correlationDescription')}
                          </p>
                        </div>
                      </CardHeader>
                      <CardBody className="gap-3 px-4 pb-4 pt-0">
                        {isFetchingDetail || isFetchingCorrelation ? (
                          <div className="flex items-center gap-2 py-8 text-sm text-default-600">
                            <Spinner size="sm" />
                            {t('common.loading')}
                          </div>
                        ) : correlation?.items?.length ? (
                          correlation.items.map((item) => (
                            <div
                              key={item.id}
                              className="rounded-large border border-default-200 bg-content2/55 px-4 py-3"
                            >
                              <div className="flex flex-wrap items-center justify-between gap-3">
                                <div className="flex flex-wrap items-center gap-2">
                                  <Chip color={getCategoryColor(item.category)} size="sm" variant="flat">
                                    {getCategoryLabel(item.category, t)}
                                  </Chip>
                                  <Chip color={getSeverityColor(item.severity)} size="sm" variant="flat">
                                    {getSeverityLabel(item.severity, t)}
                                  </Chip>
                                  <span className="font-mono text-xs text-default-500">
                                    {formatDateTime(item.ts)}
                                  </span>
                                </div>
                                {item.id !== selectedEvent.id ? (
                                  <Button size="sm" variant="light" onPress={() => setSelectedEventId(item.id)}>
                                    {t('logs.events.actions.inspect')}
                                  </Button>
                                ) : null}
                              </div>
                              <div className="mt-3 text-sm font-medium text-foreground">{localizeEventType(item.event_type, t)}</div>
                              <div className="mt-1 text-sm leading-6 text-default-600">
                                {buildEventHeadline(item, t)}
                              </div>
                            </div>
                          ))
                        ) : (
                          <div className="rounded-large border border-dashed border-default-200 px-4 py-5 text-sm text-default-600">
                            {t('logs.events.noInsights')}
                          </div>
                        )}
                      </CardBody>
                    </Card>
                  </div>
                ) : (
                  <div className="flex items-center gap-2 py-12 text-sm text-default-600">
                    <Spinner size="sm" />
                    {t('common.loading')}
                  </div>
                )}
              </ModalBody>
            </>
          )}
        </ModalContent>
      </Modal>
    </PageContent>
  )
}
