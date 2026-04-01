import {
  Button,
  Chip,
  Divider,
  Input,
  Radio,
  RadioGroup,
  Spinner,
  Switch,
  Textarea,
} from '@heroui/react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Monitor, Moon, RefreshCcw, Save, Settings, ShieldCheck, Sun, Workflow } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { localizeApiErrorDisplay } from '@/api/errorI18n'
import type { RuntimeConfigSnapshot } from '@/api/types'
import { systemApi } from '@/api/system'
import { Card, CardBody, CardHeader } from '@/components/ui/card'
import { useTheme } from '@/components/use-theme'
import { useUiPreferences } from '@/components/use-ui-preferences'
import {
  DockedPageIntro,
  PageContent,
} from '@/components/layout/page-archetypes'
import { buildRuntimeConfigUpdateRequest } from '@/features/config/contracts'
import { notify } from '@/lib/notification'

function hasRuntimeChanges(
  config: RuntimeConfigSnapshot | null,
  remoteConfig: RuntimeConfigSnapshot | undefined,
) {
  return (
    config !== null
    && remoteConfig !== undefined
    && JSON.stringify(buildRuntimeConfigUpdateRequest(config))
      !== JSON.stringify(buildRuntimeConfigUpdateRequest(remoteConfig))
  )
}

export default function Config() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const { theme, resolvedTheme, setTheme } = useTheme()
  const {
    themeDensity,
    setThemeDensity,
    themeRadius,
    setThemeRadius,
  } = useUiPreferences()
  const [formDraft, setFormDraft] = useState<RuntimeConfigSnapshot | null>(null)

  const { data: remoteConfig, isLoading, isFetching, refetch } = useQuery({
    queryKey: ['systemConfig'],
    queryFn: systemApi.getConfig,
  })

  const config = formDraft ?? remoteConfig ?? null
  const hasChanges = hasRuntimeChanges(config, remoteConfig)

  const updateDraft = (updater: (current: RuntimeConfigSnapshot) => RuntimeConfigSnapshot) => {
    setFormDraft((previous) => {
      const base = previous ?? remoteConfig
      if (!base) return previous
      return updater(base)
    })
  }

  const resetDraft = () => {
    setFormDraft(null)
  }

  const mutation = useMutation({
    mutationFn: systemApi.updateConfig,
    onSuccess: (nextConfig) => {
      queryClient.setQueryData(['systemConfig'], nextConfig)
      setFormDraft(nextConfig)
      notify({
        variant: 'success',
        title: t('config.success'),
      })
    },
    onError: (error) => {
      const fallback = t('config.antigravity.saveFailed')
      const description = localizeApiErrorDisplay(t, error, fallback).label
      notify({
        variant: 'error',
        title: fallback,
        description: description !== fallback ? description : undefined,
      })
    },
  })

  const handleSave = () => {
    if (!config) {
      return
    }
    mutation.mutate(buildRuntimeConfigUpdateRequest(config))
  }

  const resetThemeLab = () => {
    setTheme('system')
    setThemeDensity('comfortable')
    setThemeRadius('default')
  }

  const overviewCards = useMemo(
    () => [
      {
        title: t('config.antigravity.metrics.controlPlane'),
        value: config?.control_plane_listen ?? '-',
        description: t('config.controlPlane.listen'),
        icon: <Settings className="h-4 w-4" />,
        toneClassName: 'bg-primary/10 text-primary',
      },
      {
        title: t('config.antigravity.metrics.dataPlane'),
        value: config?.data_plane_base_url ?? '-',
        description: t('config.controlPlane.dataPlaneUrl'),
        icon: <Workflow className="h-4 w-4" />,
        toneClassName: 'bg-secondary/10 text-secondary',
      },
      {
        title: t('config.antigravity.metrics.authValidation'),
        value: config?.auth_validate_url ?? '-',
        description: t('config.controlPlane.authValidateUrl'),
        icon: <ShieldCheck className="h-4 w-4" />,
        toneClassName: 'bg-success/10 text-success',
      },
      {
        title: t('config.antigravity.metrics.refreshStatus'),
        value: config?.oauth_refresh_enabled
          ? t('config.antigravity.refreshEnabled')
          : t('config.antigravity.refreshDisabled'),
        description: t('config.refreshSettings.intervalSec', {
        }) + `: ${t('common.units.secondsShort', { value: config?.oauth_refresh_interval_sec ?? 0 })}`,
        icon: <RefreshCcw className="h-4 w-4" />,
        toneClassName: 'bg-warning/10 text-warning',
      },
    ],
    [config, t],
  )
  const themeModeLabel =
    theme === 'system'
      ? t('theme.system')
      : theme === 'dark'
        ? t('theme.dark')
        : t('theme.light')
  const resolvedThemeLabel = resolvedTheme === 'dark' ? t('theme.dark') : t('theme.light')

  if (isLoading) {
    return (
      <div className="flex h-[calc(100vh-100px)] w-full items-center justify-center">
        <Spinner
          color="primary"
          label={t('config.antigravity.loading')}
          size="lg"
        />
      </div>
    )
  }

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="settings"
        title={t('nav.config')}
        description={t('config.subtitle')}
        actions={(
          <div className="flex flex-wrap gap-2">
            <Button
              isDisabled={!hasChanges || mutation.isPending}
              startContent={<RefreshCcw className="h-4 w-4" />}
              variant="light"
              onPress={resetDraft}
            >
              {t('config.antigravity.reset')}
            </Button>
            <Button
              isLoading={isFetching}
              startContent={isFetching ? undefined : <RefreshCcw className="h-4 w-4" />}
              variant="light"
              onPress={() => {
                void refetch()
              }}
            >
              {t('common.refresh')}
            </Button>
            <Button
              color="primary"
              isDisabled={!config || !hasChanges}
              isLoading={mutation.isPending}
              startContent={mutation.isPending ? undefined : <Save className="h-4 w-4" />}
              variant="flat"
              onPress={handleSave}
            >
              {t('config.save')}
            </Button>
          </div>
        )}
      />

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {overviewCards.map((card) => (
          <Card key={card.title}>
            <CardBody className="space-y-5 p-4">
              <div className={card.toneClassName + ' flex h-10 w-10 items-center justify-center rounded-large'}>
                {card.icon}
              </div>
              <div className="space-y-2">
                <p className="text-xs font-semibold uppercase tracking-[0.16em] text-default-500">
                  {card.title}
                </p>
                <p className="truncate text-sm font-semibold leading-6 text-foreground">
                  {card.value}
                </p>
                <p className="text-sm leading-6 text-default-600">
                  {card.description}
                </p>
              </div>
            </CardBody>
          </Card>
        ))}
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.25fr)_minmax(0,0.95fr)]">
        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('config.controlPlane.title')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('config.controlPlane.desc')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="gap-4 px-5 pb-5 pt-1">
            <Input
              classNames={{ label: 'text-xs font-medium text-default-500' }}
              isReadOnly
              label={t('config.controlPlane.listen')}
              labelPlacement="outside"
              size="sm"
              value={config?.control_plane_listen ?? ''}
            />
            <Input
              classNames={{ label: 'text-xs font-medium text-default-500' }}
              label={t('config.controlPlane.dataPlaneUrl')}
              labelPlacement="outside"
              size="sm"
              value={config?.data_plane_base_url ?? ''}
              onValueChange={(value) =>
                updateDraft((current) => ({ ...current, data_plane_base_url: value }))
              }
            />
            <Input
              classNames={{ label: 'text-xs font-medium text-default-500' }}
              label={t('config.controlPlane.authValidateUrl')}
              labelPlacement="outside"
              size="sm"
              value={config?.auth_validate_url ?? ''}
              onValueChange={(value) =>
                updateDraft((current) => ({ ...current, auth_validate_url: value }))
              }
            />
          </CardBody>
        </Card>

        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('config.antigravity.runtimePanelTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('config.runtimeHint.desc')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="gap-4 px-5 pb-5 pt-1">
            <div className="flex flex-wrap gap-2">
              <Chip color="primary" size="sm" variant="flat">
                {t('config.reload.title')}
              </Chip>
              <Chip color={hasChanges ? 'warning' : 'success'} size="sm" variant="flat">
                {hasChanges
                  ? t('config.antigravity.unsavedChanges')
                  : t('config.antigravity.synced')}
              </Chip>
            </div>
            <Divider />
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t('config.antigravity.notesStatus')}
                </div>
                <div className="mt-2 text-sm font-semibold text-foreground">
                  {config?.notes?.trim()
                    ? t('config.antigravity.notesPresent')
                    : t('config.antigravity.notesEmpty')}
                </div>
              </div>
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t('config.antigravity.refreshIntervalLabel')}
                </div>
                <div className="mt-2 text-sm font-semibold text-foreground">
                  {config?.oauth_refresh_interval_sec ?? 0}s
                </div>
              </div>
            </div>
          </CardBody>
        </Card>
      </div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.18fr)_minmax(0,0.94fr)]">
        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('config.themeLab.title')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('config.themeLab.description')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="gap-5 px-5 pb-5 pt-1">
            <div className="flex flex-wrap gap-2">
              <Chip color="secondary" size="sm" variant="flat">
                {t('config.themeLab.localOnly')}
              </Chip>
              <Chip size="sm" variant="flat">
                {t('config.themeLab.currentMode', { value: themeModeLabel })}
              </Chip>
              <Chip color="primary" size="sm" variant="flat">
                {t('config.themeLab.resolvedMode', { value: resolvedThemeLabel })}
              </Chip>
            </div>

            <Divider />

            <div className="space-y-3">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold text-foreground">
                  {t('config.themeLab.modeTitle')}
                </h3>
                <p className="text-sm leading-6 text-default-600">
                  {t('config.themeLab.modeDescription')}
                </p>
              </div>
              <RadioGroup
                aria-label={t('config.themeLab.modeTitle')}
                classNames={{ wrapper: 'flex gap-3 md:flex-row' }}
                orientation="horizontal"
                value={theme}
                onValueChange={(value) => setTheme(value as 'light' | 'dark' | 'system')}
              >
                <Radio description={t('config.themeLab.modeLightHint')} value="light">
                  <div className="inline-flex items-center gap-2">
                    <Sun className="h-4 w-4" />
                    {t('theme.light')}
                  </div>
                </Radio>
                <Radio description={t('config.themeLab.modeDarkHint')} value="dark">
                  <div className="inline-flex items-center gap-2">
                    <Moon className="h-4 w-4" />
                    {t('theme.dark')}
                  </div>
                </Radio>
                <Radio description={t('config.themeLab.modeSystemHint')} value="system">
                  <div className="inline-flex items-center gap-2">
                    <Monitor className="h-4 w-4" />
                    {t('theme.system')}
                  </div>
                </Radio>
              </RadioGroup>
            </div>

            <Divider />

            <div className="grid gap-5 md:grid-cols-2">
              <div className="space-y-3">
                <div className="space-y-1">
                  <h3 className="text-sm font-semibold text-foreground">
                    {t('config.themeLab.radiusTitle')}
                  </h3>
                  <p className="text-sm leading-6 text-default-600">
                    {t('config.themeLab.radiusDescription')}
                  </p>
                </div>
                <RadioGroup
                  aria-label={t('config.themeLab.radiusTitle')}
                  value={themeRadius}
                  onValueChange={(value) => setThemeRadius(value as 'compact' | 'default' | 'relaxed')}
                >
                  <Radio value="compact">{t('config.themeLab.radius.compact')}</Radio>
                  <Radio value="default">{t('config.themeLab.radius.default')}</Radio>
                  <Radio value="relaxed">{t('config.themeLab.radius.relaxed')}</Radio>
                </RadioGroup>
              </div>

              <div className="space-y-3">
                <div className="space-y-1">
                  <h3 className="text-sm font-semibold text-foreground">
                    {t('config.themeLab.densityTitle')}
                  </h3>
                  <p className="text-sm leading-6 text-default-600">
                    {t('config.themeLab.densityDescription')}
                  </p>
                </div>
                <RadioGroup
                  aria-label={t('config.themeLab.densityTitle')}
                  value={themeDensity}
                  onValueChange={(value) => setThemeDensity(value as 'compact' | 'comfortable')}
                >
                  <Radio value="compact">{t('config.themeLab.density.compact')}</Radio>
                  <Radio value="comfortable">{t('config.themeLab.density.comfortable')}</Radio>
                </RadioGroup>
              </div>
            </div>

            <Divider />

            <div className="flex flex-wrap items-center justify-between gap-3">
              <p className="max-w-2xl text-sm leading-6 text-default-600">
                {t('config.themeLab.footer')}
              </p>
              <Button variant="light" onPress={resetThemeLab}>
                {t('config.themeLab.reset')}
              </Button>
            </div>
          </CardBody>
        </Card>

        <Card>
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t('config.themeLab.previewTitle')}
              </h2>
              <p className="text-sm leading-6 text-default-600">
                {t('config.themeLab.previewDescription')}
              </p>
            </div>
          </CardHeader>
          <CardBody className="gap-5 px-5 pb-5 pt-1">
            <div className="rounded-large border-small border-default-200 bg-content1 p-4 shadow-small">
              <div className="flex items-start justify-between gap-3">
                <div className="space-y-1">
                  <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                    {t('config.themeLab.previewMetric')}
                  </div>
                  <div className="text-2xl font-semibold tracking-[-0.04em] text-foreground">
                    128
                  </div>
                  <div className="text-sm leading-6 text-default-600">
                    {t('config.themeLab.previewMetricHint')}
                  </div>
                </div>
                <Chip color="success" size="sm" variant="flat">
                  {t('config.themeLab.previewResolvedChip', { value: resolvedThemeLabel })}
                </Chip>
              </div>
            </div>

            <Input
              classNames={{ label: 'text-xs font-medium text-default-500' }}
              label={t('config.themeLab.previewInputLabel')}
              labelPlacement="outside"
              size="sm"
              value={t('config.themeLab.previewInputValue')}
            />

            <div className="flex flex-wrap gap-3">
              <Button color="primary" variant="solid">
                {t('config.themeLab.previewPrimaryAction')}
              </Button>
              <Button variant="flat">
                {t('config.themeLab.previewSecondaryAction')}
              </Button>
            </div>

            <div className="flex flex-wrap gap-2">
              <Chip color="success" size="sm" variant="flat">
                {t('config.themeLab.previewChipHealthy')}
              </Chip>
              <Chip color="warning" size="sm" variant="flat">
                {t('config.themeLab.previewChipAttention')}
              </Chip>
              <Chip size="sm" variant="bordered">
                {t('config.themeLab.previewChipRadius', { value: t(`config.themeLab.radius.${themeRadius}`) })}
              </Chip>
              <Chip size="sm" variant="bordered">
                {t('config.themeLab.previewChipDensity', { value: t(`config.themeLab.density.${themeDensity}`) })}
              </Chip>
            </div>
          </CardBody>
        </Card>
      </div>

      <Card>
        <CardHeader className="px-5 pb-3 pt-5">
          <div className="space-y-1">
            <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
              {t('config.refreshSettings.title')}
            </h2>
            <p className="text-sm leading-6 text-default-600">
              {t('config.refreshSettings.desc')}
            </p>
          </div>
        </CardHeader>
        <CardBody className="gap-4 px-5 pb-5 pt-1">
          <div className="flex items-center justify-between gap-4 rounded-large border border-default-200 bg-content2/55 px-4 py-4">
            <div className="space-y-1">
              <p className="text-sm font-medium text-foreground">
                {t('config.refreshSettings.enableLabel')}
              </p>
              <p className="text-xs leading-5 text-default-500">
                {t('config.refreshSettings.enableDesc')}
              </p>
            </div>
            <Switch
              color="primary"
              isSelected={config?.oauth_refresh_enabled ?? false}
              onValueChange={(value) =>
                updateDraft((current) => ({ ...current, oauth_refresh_enabled: value }))
              }
            />
          </div>

          <Input
            classNames={{ label: 'text-xs font-medium text-default-500' }}
            label={t('config.refreshSettings.intervalSec')}
            labelPlacement="outside"
            size="sm"
            type="number"
            value={String(config?.oauth_refresh_interval_sec ?? 3600)}
            onValueChange={(value) =>
              updateDraft((current) => ({
                ...current,
                oauth_refresh_interval_sec: Math.max(1, Number(value) || 1),
              }))
            }
          />

          <Textarea
            classNames={{ label: 'text-xs font-medium text-default-500' }}
            label={t('config.refreshSettings.notes')}
            labelPlacement="outside"
            minRows={5}
            size="sm"
            value={config?.notes ?? ''}
            onValueChange={(value) => updateDraft((current) => ({ ...current, notes: value }))}
          />
        </CardBody>
      </Card>
    </PageContent>
  )
}
