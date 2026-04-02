import { useMemo } from 'react'
import {
  Button,
  Chip,
  Divider,
  Progress,
  Spinner,
} from '@heroui/react'
import { useQuery } from '@tanstack/react-query'
import { Activity, Database, RefreshCcw, Server, ShieldCheck, Wifi } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { adminApi } from '@/api/settings'
import { DEFAULT_SYSTEM_CAPABILITIES, systemApi } from '@/api/system'
import { Card, CardBody, CardHeader } from '@/components/ui/card'
import {
  DockedPageIntro,
  PageContent,
} from '@/components/layout/page-archetypes'
import { formatDurationFromSeconds, resolveSystemComponentRows } from '@/features/system/contracts'
import { formatDurationMs } from '@/lib/duration-format'
import { cn } from '@/lib/utils'

export default function System() {
  const { t } = useTranslation()
  const { data: capabilities = DEFAULT_SYSTEM_CAPABILITIES } = useQuery({
    queryKey: ['systemCapabilities'],
    queryFn: systemApi.getCapabilities,
    staleTime: 5 * 60_000,
  })

  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ['adminSystemState'],
    queryFn: adminApi.getSystemState,
    refetchInterval: 15_000,
  })

  const componentRows = useMemo(() => resolveSystemComponentRows(data, t), [data, t])
  const dataPlaneActiveRatio = useMemo(() => {
    const total = data?.data_plane_debug?.account_total ?? 0
    const active = data?.data_plane_debug?.active_account_total ?? 0
    if (total <= 0) {
      return 0
    }
    return Math.round((active / total) * 100)
  }, [data?.data_plane_debug?.account_total, data?.data_plane_debug?.active_account_total])

  const editionLabel =
    capabilities.edition === 'personal'
      ? t('system.antigravity.edition.personal')
      : capabilities.edition === 'team'
        ? t('system.antigravity.edition.team')
        : capabilities.edition === 'business'
          ? t('system.antigravity.edition.business')
          : t('system.status.unknown')

  const billingModeLabel =
    capabilities.billing_mode === 'cost_report_only'
      ? t('system.antigravity.billingMode.costReportOnly')
      : capabilities.billing_mode === 'credit_enforced'
        ? t('system.antigravity.billingMode.creditEnforced')
        : t('system.status.unknown')

  const summaryCards = [
    {
      label: t('system.antigravity.summary.edition'),
      value: editionLabel,
      hint: t('system.antigravity.summary.editionHint'),
      icon: ShieldCheck,
      toneClassName: 'border-primary-200 bg-primary-50 text-primary-700 dark:bg-primary/10 dark:text-primary-300',
    },
    {
      label: t('system.antigravity.summary.billingMode'),
      value: billingModeLabel,
      hint: t('system.antigravity.summary.billingModeHint'),
      icon: Activity,
      toneClassName: 'border-secondary-200 bg-secondary-50 text-secondary-700 dark:bg-secondary/10 dark:text-secondary-300',
    },
    {
      label: t('system.columns.uptime'),
      value: formatDurationFromSeconds(data?.uptime_sec, t('system.status.unknown')),
      hint: t('system.antigravity.summary.uptimeHint'),
      icon: Server,
      toneClassName: 'border-success-200 bg-success-50 text-success-700 dark:bg-success/10 dark:text-success-300',
    },
    {
      label: t('system.antigravity.summary.generatedAt'),
      value: data?.generated_at
        ? new Date(data.generated_at).toLocaleString()
        : t('system.status.unknown'),
      hint: t('system.antigravity.summary.generatedAtHint'),
      icon: Database,
      toneClassName: 'border-warning-200 bg-warning-50 text-warning-700 dark:bg-warning/10 dark:text-warning-300',
    },
  ]

  const capabilityItems = [
    {
      label: t('system.antigravity.features.multiTenant'),
      description: t('system.antigravity.features.multiTenantHint'),
      enabled: capabilities.features.multi_tenant,
    },
    {
      label: t('system.antigravity.features.tenantPortal'),
      description: t('system.antigravity.features.tenantPortalHint'),
      enabled: capabilities.features.tenant_portal,
    },
    {
      label: t('system.antigravity.features.tenantSelfService'),
      description: t('system.antigravity.features.tenantSelfServiceHint'),
      enabled: capabilities.features.tenant_self_service,
    },
    {
      label: t('system.antigravity.features.tenantRecharge'),
      description: t('system.antigravity.features.tenantRechargeHint'),
      enabled: capabilities.features.tenant_recharge,
    },
    {
      label: t('system.antigravity.features.creditBilling'),
      description: t('system.antigravity.features.creditBillingHint'),
      enabled: capabilities.features.credit_billing,
    },
    {
      label: t('system.antigravity.features.costReports'),
      description: t('system.antigravity.features.costReportsHint'),
      enabled: capabilities.features.cost_reports,
    },
  ]

  const runtimeCountCards = [
    {
      label: t('system.antigravity.counts.totalAccounts'),
      value: String(data?.counts.total_accounts ?? 0),
    },
    {
      label: t('system.antigravity.counts.enabledAccounts'),
      value: String(data?.counts.enabled_accounts ?? 0),
    },
    {
      label: t('system.antigravity.counts.oauthAccounts'),
      value: String(data?.counts.oauth_accounts ?? 0),
    },
    {
      label: t('system.antigravity.counts.apiKeys'),
      value: String(data?.counts.api_keys ?? 0),
    },
    {
      label: t('system.antigravity.counts.tenants'),
      value: String(data?.counts.tenants ?? 0),
    },
  ]

  const runtimeConfigCards = [
    {
      label: t('system.antigravity.config.controlPlaneListen'),
      value: data?.config.control_plane_listen ?? t('system.status.unknown'),
    },
    {
      label: t('system.antigravity.config.dataPlaneUrl'),
      value: data?.config.data_plane_base_url ?? t('system.status.unknown'),
    },
    {
      label: t('system.antigravity.config.authValidateUrl'),
      value: data?.config.auth_validate_url ?? t('system.status.unknown'),
    },
    {
      label: t('system.antigravity.config.oauthRefresh'),
      value: data?.config.oauth_refresh_enabled
        ? t('system.antigravity.enabled')
        : t('system.antigravity.disabled'),
    },
    {
      label: t('system.antigravity.config.refreshInterval'),
      value: t('system.antigravity.seconds', {
        value: data?.config.oauth_refresh_interval_sec ?? 0,
      }),
    },
  ]

  const debugCards = [
    {
      label: t('system.antigravity.debug.failoverEnabled'),
      value: data?.data_plane_debug?.failover_enabled ? t('common.yes') : t('common.no'),
      toneClassName: data?.data_plane_debug?.failover_enabled ? 'bg-success/10 text-success' : 'bg-default-100 text-default-700',
    },
    {
      label: t('system.antigravity.debug.authValidatorEnabled'),
      value: data?.data_plane_debug?.auth_validator_enabled ? t('common.yes') : t('common.no'),
      toneClassName: data?.data_plane_debug?.auth_validator_enabled ? 'bg-success/10 text-success' : 'bg-default-100 text-default-700',
    },
    {
      label: t('system.antigravity.debug.sharedRoutingCache'),
      value: data?.data_plane_debug?.shared_routing_cache_enabled ? t('common.yes') : t('common.no'),
      toneClassName: data?.data_plane_debug?.shared_routing_cache_enabled ? 'bg-primary/10 text-primary' : 'bg-default-100 text-default-700',
    },
    {
      label: t('system.antigravity.debug.quickRetryMax'),
      value: String(data?.data_plane_debug?.same_account_quick_retry_max ?? 0),
      toneClassName: 'bg-content2 text-foreground',
    },
    {
      label: t('system.antigravity.debug.requestFailoverWait'),
      value: formatDurationMs(data?.data_plane_debug?.request_failover_wait_ms ?? 0),
      toneClassName: 'bg-content2 text-foreground',
    },
    {
      label: t('system.antigravity.debug.retryPollInterval'),
      value: formatDurationMs(data?.data_plane_debug?.retry_poll_interval_ms ?? 0),
      toneClassName: 'bg-content2 text-foreground',
    },
    {
      label: t('system.antigravity.debug.snapshotRevision'),
      value: String(data?.data_plane_debug?.snapshot_revision ?? t('system.status.unknown')),
      toneClassName: 'bg-content2 text-foreground',
    },
    {
      label: t('system.antigravity.debug.billingReconcileScanned'),
      value: String(data?.control_plane_debug?.billing_reconcile_scanned_total ?? 0),
      toneClassName: 'bg-secondary/10 text-secondary',
    },
    {
      label: t('system.antigravity.debug.billingReconcileAdjust'),
      value: String(data?.control_plane_debug?.billing_reconcile_adjust_total ?? 0),
      toneClassName: 'bg-secondary/10 text-secondary',
    },
    {
      label: t('system.antigravity.debug.billingReconcileReleased'),
      value: String(data?.control_plane_debug?.billing_reconcile_released_total ?? 0),
      toneClassName: 'bg-secondary/10 text-secondary',
    },
    {
      label: t('system.antigravity.debug.billingReconcileFailed'),
      value: String(data?.control_plane_debug?.billing_reconcile_failed_total ?? 0),
      toneClassName: 'bg-danger/10 text-danger',
    },
  ]

  if (isLoading) {
    return (
      <div className="flex h-[calc(100vh-100px)] w-full items-center justify-center">
        <Spinner
          color="primary"
          label={t('system.antigravity.loading')}
          size="lg"
        />
      </div>
    )
  }

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t('nav.system')}
        description={t('system.subtitle')}
        actions={(
          <Button
            color="primary"
            isLoading={isFetching}
            startContent={isFetching ? undefined : <RefreshCcw className="h-4 w-4" />}
            variant="flat"
            onPress={() => {
              void refetch()
            }}
          >
            {t('common.refresh')}
          </Button>
        )}
      />

      <div className="grid w-full grid-cols-1 gap-5 sm:grid-cols-2 xl:grid-cols-4">
        {summaryCards.map((card) => {
          const Icon = card.icon
          return (
            <Card
              key={card.label}
             
            >
              <CardBody className="space-y-5 p-4">
                <div className={cn(
                  'flex h-11 w-11 items-center justify-center rounded-large border-small',
                  card.toneClassName,
                )}
                >
                  <Icon className="h-5 w-5" />
                </div>
                <div className="space-y-2">
                  <p className="text-xs font-semibold uppercase tracking-[0.16em] text-default-500">
                    {card.label}
                  </p>
                  <p className="text-[clamp(1.55rem,3vw,2.15rem)] font-semibold leading-none tracking-[-0.045em] text-foreground">
                    {card.value}
                  </p>
                  <p className="text-sm leading-6 text-default-600">
                    {card.hint}
                  </p>
                </div>
              </CardBody>
            </Card>
          )
        })}
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,0.95fr)]">
        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('system.antigravity.componentsTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('system.antigravity.componentsDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="grid gap-4 px-5 pb-5 pt-1 xl:grid-cols-3">
            {componentRows.map((row) => {
              const Icon =
                row.id === 'control-plane' ? Server : row.id === 'data-plane' ? Wifi : Database

              const color =
                row.status === 'healthy'
                  ? 'success'
                  : row.status === 'degraded'
                    ? 'danger'
                    : 'warning'

              const localizedName =
                row.id === 'control-plane'
                  ? t('system.components.controlPlane')
                  : row.id === 'data-plane'
                    ? t('system.components.dataPlane')
                    : t('system.components.usageRepo')

              const localizedDescription =
                row.id === 'control-plane'
                  ? t('system.details.apiActive')
                  : row.id === 'data-plane'
                    ? row.status === 'healthy'
                      ? t('system.details.endpointsResponding')
                      : row.status === 'checking'
                        ? t('system.details.checkingAPI')
                        : t('system.antigravity.dataPlaneIssue')
                    : row.status === 'healthy'
                      ? t('system.details.dbConnected')
                      : t('system.details.analyticsUnavailable')

              const surfaceToneClassName =
                row.status === 'healthy'
                  ? 'border-success-200 bg-success-50 text-success-700 dark:bg-success/10 dark:text-success-300'
                  : row.status === 'degraded'
                    ? 'border-danger-200 bg-danger-50 text-danger-700 dark:bg-danger/10 dark:text-danger-300'
                    : 'border-warning-200 bg-warning-50 text-warning-700 dark:bg-warning/10 dark:text-warning-300'

              return (
                <div
                  key={row.id}
                  className="rounded-large border border-default-200 bg-content2/55 px-4 py-4"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="space-y-3">
                      <div className={cn(
                        'flex h-11 w-11 items-center justify-center rounded-large border-small',
                        surfaceToneClassName,
                      )}
                      >
                        <Icon className="h-5 w-5" />
                      </div>
                      <div>
                        <p className="text-base font-semibold text-foreground">{localizedName}</p>
                        <p className="mt-1 text-sm leading-6 text-default-600">{localizedDescription}</p>
                      </div>
                    </div>
                    <Chip color={color} size="sm" variant="flat">
                      {t(`system.status.${row.status}`)}
                    </Chip>
                  </div>

                  <div className="mt-4 rounded-large border border-default-200 bg-content1/70 p-4">
                    {row.id === 'data-plane' && data?.data_plane_debug ? (
                      <div className="space-y-4">
                        <div className="space-y-2">
                          <div className="flex items-center justify-between gap-3 text-xs font-medium text-default-500">
                            <span>{t('system.antigravity.dataPlane.active')}</span>
                            <span>{dataPlaneActiveRatio}%</span>
                          </div>
                          <Progress
                            aria-label={t('system.antigravity.dataPlane.active')}
                            color={color}
                            size="sm"
                            value={dataPlaneActiveRatio}
                          />
                        </div>
                        <div className="grid grid-cols-2 gap-3">
                          <div>
                            <p className="text-xs uppercase tracking-[0.14em] text-default-400">
                              {t('system.antigravity.dataPlane.accounts')}
                            </p>
                            <p className="mt-2 text-sm font-semibold text-foreground">
                              {data.data_plane_debug.account_total ?? 0}
                            </p>
                          </div>
                          <div>
                            <p className="text-xs uppercase tracking-[0.14em] text-default-400">
                              {t('system.antigravity.dataPlane.active')}
                            </p>
                            <p className="mt-2 text-sm font-semibold text-foreground">
                              {data.data_plane_debug.active_account_total ?? 0}
                            </p>
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div className="text-sm leading-6 text-default-600">
                        {row.description}
                      </div>
                    )}
                  </div>
                </div>
              )
            })}
          </CardBody>
        </Card>

        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('system.antigravity.capabilitiesTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('system.antigravity.capabilitiesDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="gap-4 px-5 pb-5 pt-1">
            <div className="flex flex-wrap gap-2">
              <Chip color="primary" size="sm" variant="flat">
                {editionLabel}
              </Chip>
              <Chip color="secondary" size="sm" variant="flat">
                {billingModeLabel}
              </Chip>
            </div>
            <Divider />
            <div className="grid gap-3 sm:grid-cols-2">
              {capabilityItems.map((item) => (
                <div
                  key={item.label}
                  className="rounded-large border border-default-200 bg-content2/55 px-4 py-3"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="text-sm font-medium text-foreground">{item.label}</div>
                    <Chip color={item.enabled ? 'success' : 'default'} size="sm" variant="flat">
                      {item.enabled ? t('common.yes') : t('common.no')}
                    </Chip>
                  </div>
                  <p className="mt-2 text-xs leading-5 text-default-500">
                    {item.description}
                  </p>
                </div>
              ))}
            </div>
          </CardBody>
        </Card>
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)]">
        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('system.antigravity.runtimeCounts')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('system.antigravity.runtimeCountsDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="grid gap-3 px-5 pb-5 pt-1 sm:grid-cols-2 xl:grid-cols-3">
            {runtimeCountCards.map((card) => (
              <div
                key={card.label}
                className="rounded-large border border-default-200 bg-content2/55 px-4 py-4"
              >
                <p className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {card.label}
                </p>
                <p className="mt-3 text-2xl font-semibold tracking-[-0.04em] text-foreground">
                  {card.value}
                </p>
              </div>
            ))}
          </CardBody>
        </Card>

        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('system.antigravity.runtimeConfig')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('system.antigravity.runtimeConfigDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="grid gap-3 px-5 pb-5 pt-1 sm:grid-cols-2">
            {runtimeConfigCards.map((card) => (
              <div
                key={card.label}
                className="rounded-large border border-default-200 bg-content2/55 px-4 py-4"
              >
                <p className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {card.label}
                </p>
                <p className="mt-3 break-all text-sm font-semibold leading-6 text-foreground">
                  {card.value}
                </p>
              </div>
            ))}
          </CardBody>
        </Card>
      </div>

      <Card>
        <CardHeader className="px-5 pb-3 pt-5">
          <div className="space-y-1">
            <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
              {t('system.antigravity.debugSignals')}
            </h2>
            <p className="text-sm leading-6 text-default-600">
              {t('system.antigravity.debugSignalsDescription')}
            </p>
          </div>
        </CardHeader>
        <CardBody className="grid gap-3 px-5 pb-5 pt-1 sm:grid-cols-2 xl:grid-cols-4">
          {debugCards.map((card) => (
            <div
              key={card.label}
              className="rounded-large border border-default-200 bg-content2/55 px-4 py-4"
            >
              <div className={cn(
                'inline-flex rounded-full px-2.5 py-1 text-xs font-semibold uppercase tracking-[0.12em]',
                card.toneClassName,
              )}
              >
                {card.label}
              </div>
              <div className="mt-4 text-sm font-semibold leading-6 text-foreground">
                {card.value}
              </div>
            </div>
          ))}
        </CardBody>
      </Card>
    </PageContent>
  )
}
