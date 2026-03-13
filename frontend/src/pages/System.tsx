import { useMemo } from 'react'
import { type ColumnDef } from '@tanstack/react-table'
import { useQuery } from '@tanstack/react-query'
import { motion } from 'framer-motion'
import { AlertTriangle, Cpu, Database, type LucideIcon, Server } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { adminApi } from '@/api/settings'
import { systemApi, DEFAULT_SYSTEM_CAPABILITIES } from '@/api/system'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { formatRelativeTime } from '@/lib/time'

type ComponentStatus = 'healthy' | 'degraded' | 'checking'

interface SystemComponentRow {
    id: string
    name: string
    icon: LucideIcon
    status: ComponentStatus
    version: string
    details: string
    uptime: string
}

interface ObservabilityMetricCard {
    id: string
    label: string
    value: string
    hint: string
}

interface DerivedObservabilityState {
    failoverEnabled: boolean
    sharedCacheEnabled: boolean
    stickyPreferNonConflicting: boolean
    sameAccountQuickRetryMax: number
    requestFailoverWaitMs: number
    retryPollIntervalMs: number
    failoverAttemptTotal: number
    failoverSuccessTotal: number
    failoverExhaustedTotal: number
    sameAccountRetryTotal: number
    billingReconcileScannedTotal: number
    billingReconcileAdjustTotal: number
    billingReconcileFailedTotal: number
    billingReconcileReleasedTotal: number
    billingPreauthErrorRatioAvg?: number
    billingPreauthErrorRatioP50?: number
    billingPreauthErrorRatioP95?: number
    billingPreauthErrorRatioCountTotal: number
    billingPreauthCaptureMissingTotal: number
    billingSettleCompleteTotal: number
    billingReleaseWithoutCaptureTotal: number
    billingSettleCompleteRatio?: number
    billingReleaseWithoutCaptureRatio?: number
    billingPreauthTopModelName?: string
    billingPreauthTopModelP95Ratio?: number
    stickyHitRatio?: number
    cacheHitRatio?: number
    failoverSuccessRatio?: number
}

function normalizedCount(value: number | undefined): number {
    return typeof value === 'number' && Number.isFinite(value) ? value : 0
}

function normalizedRatio(value: number | undefined): number | undefined {
    if (typeof value !== 'number' || !Number.isFinite(value)) {
        return undefined
    }
    if (value < 0) return 0
    if (value > 1) return 1
    return value
}

function calcRatio(numerator: number, denominator: number): number | undefined {
    if (denominator <= 0) {
        return undefined
    }
    return numerator / denominator
}

export default function System() {
    const { t, i18n } = useTranslation()
    const { data: capabilities = DEFAULT_SYSTEM_CAPABILITIES } = useQuery({
        queryKey: ['systemCapabilities'],
        queryFn: () => systemApi.getCapabilities(),
        staleTime: 5 * 60_000,
    })
    const { data: systemState, isLoading } = useQuery({
        queryKey: ['adminSystemState'],
        queryFn: adminApi.getSystemState,
        refetchInterval: 15000 // Poll health every 15s
    })

    const uptimeStr = systemState
        ? formatRelativeTime(systemState.started_at, i18n.resolvedLanguage, false)
        : t('system.status.unknown')

    const countFormatter = useMemo(
        () => new Intl.NumberFormat(i18n.resolvedLanguage),
        [i18n.resolvedLanguage],
    )
    const ratioFormatter = useMemo(
        () =>
            new Intl.NumberFormat(i18n.resolvedLanguage, {
                style: 'percent',
                maximumFractionDigits: 1,
            }),
        [i18n.resolvedLanguage],
    )

    const observability = useMemo<DerivedObservabilityState | null>(() => {
        const debug = systemState?.data_plane_debug
        if (!debug) {
            return null
        }
        const controlDebug = systemState?.control_plane_debug

        const failoverAttemptTotal = normalizedCount(debug.failover_attempt_total)
        const failoverSuccessTotal = normalizedCount(debug.failover_success_total)
        const failoverExhaustedTotal = normalizedCount(debug.failover_exhausted_total)
        const sameAccountRetryTotal = normalizedCount(debug.same_account_retry_total)
        const billingAuthorizeTotal = normalizedCount(debug.billing_authorize_total)
        const billingReconcileScannedTotal = normalizedCount(
            controlDebug?.billing_reconcile_scanned_total,
        )
        const billingReconcileAdjustTotal = normalizedCount(
            controlDebug?.billing_reconcile_adjust_total,
        )
        const billingReconcileFailedTotal = normalizedCount(
            controlDebug?.billing_reconcile_failed_total,
        )
        const billingReconcileReleasedTotal = normalizedCount(
            controlDebug?.billing_reconcile_released_total,
        )
        const stickyHitCount = normalizedCount(debug.sticky_hit_count)
        const stickySessionTotal = normalizedCount(debug.sticky_session_total)
        const billingPreauthErrorRatioCountTotal = normalizedCount(
            debug.billing_preauth_error_ratio_count_total,
        )
        const billingPreauthCaptureMissingTotal = normalizedCount(
            debug.billing_preauth_capture_missing_total,
        )
        const billingSettleCompleteTotal = normalizedCount(debug.billing_settle_complete_total)
        const billingReleaseWithoutCaptureTotal = normalizedCount(
            debug.billing_release_without_capture_total,
        )
        const preauthModelStats = Array.isArray(debug.billing_preauth_model_error_stats)
            ? [...debug.billing_preauth_model_error_stats].sort(
                  (left, right) => (right?.sample_count ?? 0) - (left?.sample_count ?? 0),
              )
            : []
        const topModel = preauthModelStats[0]

        const localCacheHitTotal = normalizedCount(debug.routing_cache_local_sticky_hit_total)
        const localCacheMissTotal = normalizedCount(debug.routing_cache_local_sticky_miss_total)
        const sharedCacheHitTotal = normalizedCount(debug.routing_cache_shared_sticky_hit_total)
        const sharedCacheMissTotal = normalizedCount(debug.routing_cache_shared_sticky_miss_total)

        const cacheHitTotal = localCacheHitTotal + sharedCacheHitTotal
        const cacheLookupTotal =
            localCacheHitTotal +
            localCacheMissTotal +
            sharedCacheHitTotal +
            sharedCacheMissTotal

        return {
            failoverEnabled: Boolean(debug.failover_enabled),
            sharedCacheEnabled: Boolean(debug.shared_routing_cache_enabled),
            stickyPreferNonConflicting: Boolean(debug.sticky_prefer_non_conflicting),
            sameAccountQuickRetryMax: normalizedCount(debug.same_account_quick_retry_max),
            requestFailoverWaitMs: normalizedCount(debug.request_failover_wait_ms),
            retryPollIntervalMs: normalizedCount(debug.retry_poll_interval_ms),
            failoverAttemptTotal,
            failoverSuccessTotal,
            failoverExhaustedTotal,
            sameAccountRetryTotal,
            billingReconcileScannedTotal,
            billingReconcileAdjustTotal,
            billingReconcileFailedTotal,
            billingReconcileReleasedTotal,
            billingPreauthErrorRatioAvg:
                typeof debug.billing_preauth_error_ratio_avg === 'number'
                    ? debug.billing_preauth_error_ratio_avg
                    : undefined,
            billingPreauthErrorRatioP50:
                typeof debug.billing_preauth_error_ratio_p50 === 'number'
                    ? debug.billing_preauth_error_ratio_p50
                    : undefined,
            billingPreauthErrorRatioP95:
                typeof debug.billing_preauth_error_ratio_p95 === 'number'
                    ? debug.billing_preauth_error_ratio_p95
                    : undefined,
            billingPreauthErrorRatioCountTotal,
            billingPreauthCaptureMissingTotal,
            billingSettleCompleteTotal,
            billingReleaseWithoutCaptureTotal,
            billingSettleCompleteRatio:
                typeof debug.billing_settle_complete_ratio === 'number'
                    ? debug.billing_settle_complete_ratio
                    : calcRatio(billingSettleCompleteTotal, billingAuthorizeTotal),
            billingReleaseWithoutCaptureRatio:
                typeof debug.billing_release_without_capture_ratio === 'number'
                    ? debug.billing_release_without_capture_ratio
                    : calcRatio(billingReleaseWithoutCaptureTotal, billingAuthorizeTotal),
            billingPreauthTopModelName: topModel?.model,
            billingPreauthTopModelP95Ratio:
                typeof topModel?.p95_ratio === 'number' ? topModel.p95_ratio : undefined,
            stickyHitRatio:
                normalizedRatio(debug.sticky_hit_ratio) ??
                calcRatio(stickyHitCount, stickySessionTotal),
            cacheHitRatio: calcRatio(cacheHitTotal, cacheLookupTotal),
            failoverSuccessRatio: calcRatio(failoverSuccessTotal, failoverAttemptTotal),
        }
    }, [systemState?.control_plane_debug, systemState?.data_plane_debug])

    const observabilityMetrics = useMemo<ObservabilityMetricCard[]>(() => {
        if (!observability) {
            return []
        }

        const formatRatio = (value: number | undefined) =>
            typeof value === 'number' ? ratioFormatter.format(value) : t('system.observability.na')

        return [
            {
                id: 'failover_attempt_total',
                label: t('system.observability.metrics.failoverAttempts'),
                value: countFormatter.format(observability.failoverAttemptTotal),
                hint: t('system.observability.hints.failoverAttempts'),
            },
            {
                id: 'failover_success_total',
                label: t('system.observability.metrics.failoverSuccess'),
                value: countFormatter.format(observability.failoverSuccessTotal),
                hint: t('system.observability.hints.failoverSuccess'),
            },
            {
                id: 'failover_exhausted_total',
                label: t('system.observability.metrics.failoverExhausted'),
                value: countFormatter.format(observability.failoverExhaustedTotal),
                hint: t('system.observability.hints.failoverExhausted'),
            },
            {
                id: 'same_account_retry_total',
                label: t('system.observability.metrics.sameAccountRetry'),
                value: countFormatter.format(observability.sameAccountRetryTotal),
                hint: t('system.observability.hints.sameAccountRetry'),
            },
            {
                id: 'sticky_hit_ratio',
                label: t('system.observability.metrics.stickyHitRate'),
                value: formatRatio(observability.stickyHitRatio),
                hint: t('system.observability.hints.stickyHitRate'),
            },
            {
                id: 'cache_hit_ratio',
                label: t('system.observability.metrics.cacheHitRate'),
                value: formatRatio(observability.cacheHitRatio),
                hint: t('system.observability.hints.cacheHitRate'),
            },
            {
                id: 'failover_success_ratio',
                label: t('system.observability.metrics.failoverSuccessRate'),
                value: formatRatio(observability.failoverSuccessRatio),
                hint: t('system.observability.hints.failoverSuccessRate'),
            },
            {
                id: 'billing_reconcile_scanned_total',
                label: t('system.observability.metrics.billingReconcileScanned'),
                value: countFormatter.format(observability.billingReconcileScannedTotal),
                hint: t('system.observability.hints.billingReconcileScanned'),
            },
            {
                id: 'billing_reconcile_adjust_total',
                label: t('system.observability.metrics.billingReconcileAdjust'),
                value: countFormatter.format(observability.billingReconcileAdjustTotal),
                hint: t('system.observability.hints.billingReconcileAdjust'),
            },
            {
                id: 'billing_reconcile_failed_total',
                label: t('system.observability.metrics.billingReconcileFailed'),
                value: countFormatter.format(observability.billingReconcileFailedTotal),
                hint: t('system.observability.hints.billingReconcileFailed'),
            },
            {
                id: 'billing_reconcile_released_total',
                label: t('system.observability.metrics.billingReconcileReleased'),
                value: countFormatter.format(observability.billingReconcileReleasedTotal),
                hint: t('system.observability.hints.billingReconcileReleased'),
            },
            {
                id: 'billing_preauth_error_ratio_avg',
                label: t('system.observability.metrics.billingPreauthErrorRatioAvg'),
                value: formatRatio(observability.billingPreauthErrorRatioAvg),
                hint: t('system.observability.hints.billingPreauthErrorRatioAvg'),
            },
            {
                id: 'billing_preauth_error_ratio_p95',
                label: t('system.observability.metrics.billingPreauthErrorRatioP95'),
                value: formatRatio(observability.billingPreauthErrorRatioP95),
                hint: t('system.observability.hints.billingPreauthErrorRatioP95'),
            },
            {
                id: 'billing_settle_complete_ratio',
                label: t('system.observability.metrics.billingSettleCompleteRatio'),
                value: formatRatio(observability.billingSettleCompleteRatio),
                hint: t('system.observability.hints.billingSettleCompleteRatio'),
            },
            {
                id: 'billing_release_without_capture_ratio',
                label: t('system.observability.metrics.billingReleaseWithoutCaptureRatio'),
                value: formatRatio(observability.billingReleaseWithoutCaptureRatio),
                hint: t('system.observability.hints.billingReleaseWithoutCaptureRatio'),
            },
            {
                id: 'billing_preauth_capture_missing_total',
                label: t('system.observability.metrics.billingPreauthCaptureMissingTotal'),
                value: countFormatter.format(observability.billingPreauthCaptureMissingTotal),
                hint: t('system.observability.hints.billingPreauthCaptureMissingTotal'),
            },
            {
                id: 'billing_preauth_top_model_p95',
                label: t('system.observability.metrics.billingPreauthTopModelP95'),
                value:
                    typeof observability.billingPreauthTopModelP95Ratio === 'number'
                        ? `${formatRatio(observability.billingPreauthTopModelP95Ratio)}${
                              observability.billingPreauthTopModelName
                                  ? ` (${observability.billingPreauthTopModelName})`
                                  : ''
                          }`
                        : t('system.observability.na'),
                hint: t('system.observability.hints.billingPreauthTopModelP95'),
            },
        ].filter((item) => capabilities.features.credit_billing || !item.id.startsWith('billing_'))
    }, [capabilities.features.credit_billing, countFormatter, observability, ratioFormatter, t])

    const components = useMemo<SystemComponentRow[]>(() => {
        const controlPlaneStatus: ComponentStatus = isLoading
            ? 'checking'
            : systemState
                ? 'healthy'
                : 'degraded'
        const dataPlaneStatus: ComponentStatus = isLoading
            ? 'checking'
            : systemState && !systemState.data_plane_error
                ? 'healthy'
                : 'degraded'
        const usageRepoStatus: ComponentStatus = isLoading
            ? 'checking'
            : systemState?.usage_repo_available
                ? 'healthy'
                : 'degraded'

        return [
            {
                id: 'control_plane',
                name: t('system.components.controlPlane'),
                icon: Cpu,
                status: controlPlaneStatus,
                version: t('system.labels.local'),
                details: isLoading ? t('system.details.checkingAPI') : t('system.details.apiActive'),
                uptime: controlPlaneStatus === 'degraded' ? t('system.status.unknown') : uptimeStr,
            },
            {
                id: 'data_plane',
                name: t('system.components.dataPlane'),
                icon: Server,
                status: dataPlaneStatus,
                version: t('system.labels.remote'),
                details: isLoading
                    ? t('system.details.checkingAPI')
                    : systemState?.data_plane_error || t('system.details.endpointsResponding'),
                uptime:
                    dataPlaneStatus === 'healthy'
                        ? uptimeStr
                        : dataPlaneStatus === 'checking'
                            ? t('system.status.checking')
                            : t('system.status.offline'),
            },
            {
                id: 'usage_repo',
                name: t('system.components.usageRepo'),
                icon: Database,
                status: usageRepoStatus,
                version: t('system.labels.storage'),
                details:
                    usageRepoStatus === 'healthy'
                        ? t('system.details.dbConnected')
                        : usageRepoStatus === 'checking'
                            ? t('system.details.checkingAPI')
                            : t('system.details.analyticsUnavailable'),
                uptime:
                    usageRepoStatus === 'healthy'
                        ? uptimeStr
                        : usageRepoStatus === 'checking'
                            ? t('system.status.checking')
                            : t('system.status.offline'),
            },
        ]
    }, [isLoading, systemState, t, uptimeStr])

    const columns = useMemo<ColumnDef<SystemComponentRow>[]>(() => {
        return [
            {
                id: 'name',
                header: t('system.columns.component'),
                accessorFn: (row) => row.name.toLowerCase(),
                cell: ({ row }) => (
                    <div className="flex items-center gap-2 min-w-[180px]">
                        <div className="p-1.5 rounded-md bg-muted/50">
                            <row.original.icon className="h-4 w-4 text-foreground/70" />
                        </div>
                        <span className="font-medium">{row.original.name}</span>
                    </div>
                ),
            },
            {
                id: 'status',
                header: t('system.columns.status'),
                accessorFn: (row) => row.status,
                cell: ({ row }) => {
                    const status = row.original.status
                    const variant =
                        status === 'healthy'
                            ? 'success'
                            : status === 'checking'
                                ? 'secondary'
                                : 'destructive'
                    const statusLabel =
                        status === 'healthy'
                            ? t('system.status.healthy')
                            : status === 'checking'
                                ? t('system.status.checking')
                                : t('system.status.degraded')
                    return (
                        <Badge variant={variant} className="uppercase text-[10px]">
                            {statusLabel}
                        </Badge>
                    )
                },
            },
            {
                id: 'version',
                header: t('system.columns.version'),
                accessorFn: (row) => row.version,
                cell: ({ row }) => (
                    <span className="text-sm text-muted-foreground">
                        {row.original.version}
                    </span>
                ),
            },
            {
                id: 'details',
                header: t('system.columns.details'),
                accessorFn: (row) => row.details.toLowerCase(),
                cell: ({ row }) => (
                    <span className="text-sm leading-6">
                        {row.original.details}
                    </span>
                ),
            },
            {
                id: 'uptime',
                header: t('system.columns.uptime'),
                accessorFn: (row) => row.uptime.toLowerCase(),
                cell: ({ row }) => (
                    <span className="font-mono text-xs">
                        {row.original.uptime}
                    </span>
                ),
            },
        ]
    }, [t])

    const container = {
        hidden: { opacity: 0 },
        show: { opacity: 1, transition: { staggerChildren: 0.1 } },
    }

    return (
        <div className="flex-1 p-8 bg-background max-w-7xl overflow-y-auto">
            <motion.div initial={{ opacity: 0, y: -10 }} animate={{ opacity: 1, y: 0 }} className="mb-8">
                <h2 className="text-3xl font-bold tracking-tight">{t('system.title')}</h2>
                <p className="text-muted-foreground mt-1">{t('system.subtitle')}</p>
            </motion.div>

            <motion.div variants={container} initial="hidden" animate="show" className="space-y-6">
                <Card className="shadow-sm border-border/50">
                    <CardContent className="h-[460px] min-h-0 pt-6">
                        <StandardDataTable
                            columns={columns}
                            data={components}
                            className="h-full"
                            density="compact"
                            defaultPageSize={10}
                            pageSizeOptions={[10, 20, 50]}
                            searchPlaceholder={t('system.searchPlaceholder')}
                            searchFn={(row, keyword) =>
                                `${row.name} ${row.version} ${row.details} ${row.uptime}`
                                    .toLowerCase()
                                    .includes(keyword)
                            }
                        />
                    </CardContent>
                </Card>

                <Card className="shadow-sm border-border/50">
                    <CardHeader>
                        <CardTitle>{t('system.observability.title')}</CardTitle>
                        <CardDescription>{t('system.observability.subtitle')}</CardDescription>
                    </CardHeader>
                    <CardContent className="space-y-4">
                        {observability ? (
                            <>
                                <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
                                    {observabilityMetrics.map((metric) => (
                                        <div
                                            key={metric.id}
                                            className="rounded-lg border border-border/60 bg-muted/20 p-3"
                                        >
                                            <p className="text-xs text-muted-foreground">{metric.label}</p>
                                            <p className="mt-1 text-xl font-semibold tracking-tight">{metric.value}</p>
                                            <p className="mt-1 text-xs text-muted-foreground">{metric.hint}</p>
                                        </div>
                                    ))}
                                </div>
                                <div className="flex flex-wrap gap-2">
                                    <Badge variant={observability.failoverEnabled ? 'success' : 'secondary'}>
                                        {observability.failoverEnabled
                                            ? t('system.observability.badges.failoverOn')
                                            : t('system.observability.badges.failoverOff')}
                                    </Badge>
                                    <Badge
                                        variant={observability.sharedCacheEnabled ? 'success' : 'secondary'}
                                    >
                                        {observability.sharedCacheEnabled
                                            ? t('system.observability.badges.sharedCacheOn')
                                            : t('system.observability.badges.sharedCacheOff')}
                                    </Badge>
                                    <Badge
                                        variant={
                                            observability.stickyPreferNonConflicting
                                                ? 'success'
                                                : 'secondary'
                                        }
                                    >
                                        {observability.stickyPreferNonConflicting
                                            ? t('system.observability.badges.stickyConflictAvoidOn')
                                            : t('system.observability.badges.stickyConflictAvoidOff')}
                                    </Badge>
                                    <Badge variant="outline">
                                        {t('system.observability.badges.quickRetry', {
                                            value: observability.sameAccountQuickRetryMax,
                                        })}
                                    </Badge>
                                    <Badge variant="outline">
                                        {t('system.observability.badges.failoverWait', {
                                            value: observability.requestFailoverWaitMs,
                                        })}
                                    </Badge>
                                    <Badge variant="outline">
                                        {t('system.observability.badges.retryPoll', {
                                            value: observability.retryPollIntervalMs,
                                        })}
                                    </Badge>
                                </div>
                            </>
                        ) : (
                            <div className="flex items-start gap-3 rounded-lg border border-dashed p-4">
                                <AlertTriangle className="mt-0.5 h-4 w-4 text-muted-foreground" />
                                <div className="space-y-1">
                                    <p className="text-sm font-medium">
                                        {isLoading
                                            ? t('system.details.checkingAPI')
                                            : t('system.observability.unavailableTitle')}
                                    </p>
                                    <p className="text-sm text-muted-foreground">
                                        {isLoading
                                            ? t('system.observability.unavailableLoading')
                                            : systemState?.data_plane_error ||
                                              t('system.observability.unavailableDesc')}
                                    </p>
                                </div>
                            </div>
                        )}
                    </CardContent>
                </Card>
            </motion.div>
        </div>
    )
}
