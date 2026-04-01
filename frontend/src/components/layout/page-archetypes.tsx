import {
  type HTMLAttributes,
  type ReactNode,
  useMemo,
} from 'react'
import { Area, AreaChart, ResponsiveContainer } from 'recharts'
import { Chip } from '@heroui/react'
import { ArrowUpRight, ArrowRight, ArrowDownRight } from 'lucide-react'

import {
  describeDashboardOverviewLayout,
  describeDashboardShellLayout,
  describePageRegions,
  describeReportShellLayout,
  resolvePageArchetype,
  type PageArchetype,
} from '@/lib/page-archetypes'
import { resolvePanelShellClassName } from '@/lib/panel-shell'
import { cn } from '@/lib/utils'
import { usePageHeader, usePageHeaderDocking } from '@/components/layout/page-header-context'

export interface PageIntroProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  archetype?: PageArchetype | string
  eyebrow?: ReactNode
  title: ReactNode
  description?: ReactNode
  actions?: ReactNode
}

export function PageIntro({
  archetype = 'detail',
  eyebrow,
  title,
  description,
  actions,
  className,
  ...props
}: PageIntroProps) {
  const config = resolvePageArchetype(archetype)
  const regions = describePageRegions(archetype)

  return (
    <div
      className={cn(
        'flex flex-col gap-3 md:gap-4',
        regions.introAlignment === 'between' && 'lg:flex-row lg:items-end lg:justify-between lg:gap-8',
        className,
      )}
      {...props}
    >
      <div className={cn('min-w-0 space-y-3', config.introStyle === 'stage' && 'max-w-[46rem] md:space-y-4')}>
        {eyebrow ? (
          <div className="inline-flex w-fit items-center gap-3 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
            <span className="h-px w-9 bg-current/70" />
            {eyebrow}
          </div>
        ) : null}
        <div className="space-y-2">
          <h1
            className={cn(
              'max-w-[22ch] text-balance text-2xl font-bold tracking-tight text-foreground',
              config.introStyle === 'stage'
                ? 'text-[clamp(2.1rem,5.6vw,3.8rem)] leading-[0.96]'
                : 'leading-tight sm:text-[1.75rem]',
            )}
          >
            {title}
          </h1>
          {description ? (
            <p
              className={cn(
                'max-w-[72ch] text-sm leading-6 text-default-500',
                config.introStyle === 'stage' && 'max-w-[64ch] text-[15px] leading-8 sm:text-[17px]',
              )}
            >
              {description}
            </p>
          ) : null}
        </div>
      </div>
      {actions ? <div className="flex shrink-0 flex-wrap items-center gap-2 lg:max-w-[24rem] lg:justify-end">{actions}</div> : null}
    </div>
  )
}

export function DockedPageIntro(props: PageIntroProps) {
  const {
    title,
    description,
    actions,
  } = props
  const introRef = usePageHeaderDocking()
  const pageHeader = useMemo(
    () => ({
      mode: 'dock-on-scroll' as const,
      title,
      description,
      actions,
    }),
    [actions, description, title],
  )

  usePageHeader(pageHeader)

  return (
    <div ref={introRef}>
      <PageIntro {...props} />
    </div>
  )
}

interface BrandStageProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  badge?: ReactNode
  title: ReactNode
  subtitle?: ReactNode
  points?: ReactNode[]
  footer?: ReactNode
}

export function BrandStage({
  badge,
  title,
  subtitle,
  points = [],
  footer,
  className,
  ...props
}: BrandStageProps) {
  return (
    <section
      className={cn(
        'relative overflow-hidden rounded-large border-small border-default-200 bg-content1 shadow-small p-5 sm:p-6 lg:p-7',
        className,
      )}
      {...props}
    >
      <div className="relative z-10 space-y-5">
        {badge ? <div>{badge}</div> : null}
        <div className="space-y-3">
          <h1 className="max-w-3xl text-balance text-[clamp(1.75rem,4vw,3rem)] font-semibold leading-[1.01] tracking-[-0.025em] text-foreground">
            {title}
          </h1>
          {subtitle ? (
            <p className="max-w-[68ch] text-[15px] leading-7 text-muted-foreground sm:text-[16px]">
              {subtitle}
            </p>
          ) : null}
        </div>
        {points.length > 0 ? (
          <ul className="grid gap-3 border-t border-default-200/70 pt-4 text-sm text-foreground/86 sm:grid-cols-2">
            {points.map((point, index) => (
              <li key={index} className="flex items-start gap-3 pr-4">
                <span className="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-foreground/75" />
                <span className="leading-6">{point}</span>
              </li>
            ))}
          </ul>
        ) : null}
        {footer ? <div className="text-sm text-muted-foreground">{footer}</div> : null}
      </div>
    </section>
  )
}

type PageContentProps = HTMLAttributes<HTMLDivElement>

export function PageContent({
  className,
  children,
  ...props
}: PageContentProps) {
  return (
    <div
      className={cn(
        'page-stagger flex-1 min-w-0 px-[var(--app-page-padding)] py-[var(--app-page-padding)] sm:px-[var(--app-page-padding-sm)] sm:py-[var(--app-page-padding-sm)] lg:px-[var(--app-page-padding-lg)] lg:py-[var(--app-page-padding-lg)]',
        className,
      )}
      {...props}
    >
      {children}
    </div>
  )
}

interface PagePanelProps extends HTMLAttributes<HTMLElement> {
  as?: 'section' | 'div' | 'aside'
  tone?: 'primary' | 'secondary'
}

export function PagePanel({
  as: Component = 'section',
  tone = 'primary',
  className,
  children,
  ...props
}: PagePanelProps) {
  return (
    <Component
      className={cn(
        resolvePanelShellClassName(tone === 'primary' ? 'primary' : 'secondary'),
        'relative overflow-hidden rounded-large px-[var(--app-panel-padding)] py-[var(--app-panel-padding)] sm:px-[var(--app-panel-padding-sm)] sm:py-[var(--app-panel-padding-sm)] lg:px-[var(--app-panel-padding-lg)] lg:py-[var(--app-panel-padding-lg)]',
        className,
      )}
      {...props}
    >
      {children}
    </Component>
  )
}

interface WorkspaceShellProps extends HTMLAttributes<HTMLDivElement> {
  intro: ReactNode
  primary: ReactNode
  secondary?: ReactNode
}

export function WorkspaceShell({
  intro,
  primary,
  secondary,
  className,
  ...props
}: WorkspaceShellProps) {
  return (
    <section className={cn('space-y-4 md:space-y-5', className)} {...props}>
      {intro}
      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.16fr)_minmax(17rem,0.84fr)] xl:items-start">
        <div className="min-w-0 space-y-4">{primary}</div>
        {secondary ? <aside className="min-w-0 space-y-4 xl:border-l xl:border-default-200/70 xl:pl-4">{secondary}</aside> : null}
      </div>
    </section>
  )
}

interface ReportShellProps extends HTMLAttributes<HTMLDivElement> {
  intro: ReactNode
  toolbar?: ReactNode
  lead: ReactNode
  rail?: ReactNode
}

export function ReportShell({
  intro,
  toolbar,
  lead,
  rail,
  className,
  children,
  ...props
}: ReportShellProps) {
  const layout = describeReportShellLayout()

  return (
    <section className={cn('space-y-4 md:space-y-5', className)} {...props}>
      {intro}
      {toolbar ? <div>{toolbar}</div> : null}
      <div
        className={cn(
          'grid gap-4 md:gap-5',
          rail && 'xl:grid-cols-[minmax(0,1.14fr)_minmax(18rem,0.86fr)]',
          layout.desktopContentBalance === 'lead-first' && 'xl:items-start',
        )}
      >
        <div className="min-w-0 space-y-4 md:space-y-5">{lead}</div>
        {rail ? (
          <aside
            className={cn(
              'min-w-0 space-y-4 xl:border-l xl:border-default-200/70 xl:pl-4',
              layout.mobileRailPlacement === 'after-content' && 'order-2',
            )}
          >
            {rail}
          </aside>
        ) : null}
      </div>
      {children ? <div className="space-y-4 md:space-y-5">{children}</div> : null}
    </section>
  )
}

type ReportMetricGridProps = HTMLAttributes<HTMLDivElement>

export function ReportMetricGrid({
  className,
  children,
  ...props
}: ReportMetricGridProps) {
  return (
    <div
      className={cn(
        'grid gap-px overflow-hidden rounded-large border-small border-default-200 bg-default-100 sm:grid-cols-2 xl:grid-cols-3',
        className,
      )}
      {...props}
    >
      {children}
    </div>
  )
}

interface ReportMetricCardProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  title: ReactNode
  value: ReactNode
  description?: ReactNode
  valueTitle?: string
  loading?: boolean
}

export function ReportMetricCard({
  title,
  value,
  description,
  valueTitle,
  loading = false,
  className,
  ...props
}: ReportMetricCardProps) {
  return (
    <div
      className={cn(
        'h-full space-y-3 bg-content1/84 px-4 py-4',
        className,
      )}
      {...props}
    >
      <div className="space-y-1.5">
        <p className="text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground">{title}</p>
        {loading ? (
          <div className="h-8 w-28 animate-pulse rounded-lg bg-default-200/75" />
        ) : (
          <p
            title={valueTitle}
            className="text-[clamp(1.45rem,2.4vw,2rem)] font-semibold leading-none tracking-[-0.03em] text-foreground"
          >
            {value}
          </p>
        )}
      </div>
      {description ? (
        loading ? (
          <div className="h-3.5 w-36 animate-pulse rounded bg-default-200/70" />
        ) : (
          <p className="text-[12px] leading-6 text-muted-foreground">{description}</p>
        )
        ) : null}
    </div>
  )
}

interface SectionHeaderProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  title: ReactNode
  description?: ReactNode
  actions?: ReactNode
  eyebrow?: ReactNode
}

export function SectionHeader({
  title,
  description,
  actions,
  eyebrow,
  className,
  ...props
}: SectionHeaderProps) {
  return (
    <div
      className={cn(
        'flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between sm:gap-4',
        className,
      )}
      {...props}
    >
      <div className="min-w-0 space-y-1.5">
        {eyebrow ? (
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
            {eyebrow}
          </p>
        ) : null}
        <h2 className="text-[1.12rem] font-semibold tracking-[-0.024em] text-foreground">
          {title}
        </h2>
        {description ? (
          <p className="max-w-[62ch] text-sm leading-6 text-muted-foreground">{description}</p>
        ) : null}
      </div>
      {actions ? <div className="flex shrink-0 flex-wrap items-center gap-2">{actions}</div> : null}
    </div>
  )
}

interface DashboardShellProps extends HTMLAttributes<HTMLDivElement> {
  intro: ReactNode
  rail?: ReactNode
}

export function DashboardShell({
  intro,
  rail,
  className,
  children,
  ...props
}: DashboardShellProps) {
  const config = resolvePageArchetype('dashboard')
  const layout = describeDashboardShellLayout()

  return (
    <section
      className={cn(
        'grid gap-4 md:gap-5',
        rail && 'xl:grid-cols-[minmax(0,1.12fr)_minmax(17.5rem,0.88fr)] 2xl:grid-cols-[minmax(0,1.18fr)_minmax(18.5rem,0.82fr)]',
        layout.desktopAlignment === 'start' && 'xl:items-start',
        className,
      )}
      {...props}
    >
      <div
        className={cn(
          'relative order-1 min-w-0 border-b border-default-200/70 pb-4 md:pb-5',
          rail && 'xl:col-start-1 xl:row-start-1',
          config.headerSurface === 'section' && 'rounded-large border-small border-default-200 bg-content1 shadow-small p-4 sm:p-5',
        )}
      >
        <div className="relative">{intro}</div>
      </div>
      <div className={cn('order-2 min-w-0 space-y-4 md:space-y-5', rail && 'xl:col-start-1 xl:row-start-2')}>
        {children}
      </div>
      {rail ? (
        <div
          className={cn(
            'min-w-0 space-y-4',
            layout.mobileRailPlacement === 'after-content' ? 'order-3' : 'order-2',
            'xl:order-2 xl:col-start-2 xl:row-span-2 xl:row-start-1',
            layout.railTone === 'attached' && 'xl:border-l xl:border-default-200/70 xl:pl-5',
          )}
        >
          {rail}
        </div>
      ) : null}
    </section>
  )
}

interface DashboardMetricGridProps extends HTMLAttributes<HTMLDivElement> {
  variant?: 'primary' | 'secondary'
}

export function DashboardMetricGrid({
  className,
  children,
  variant = 'primary',
  ...props
}: DashboardMetricGridProps) {
  const overview = describeDashboardOverviewLayout()

  return (
    <div
      className={cn(
        overview.metricPresentation === 'strip' &&
          'grid gap-px overflow-hidden rounded-large border-small border-default-200 bg-default-100 sm:grid-cols-2 2xl:grid-cols-4',
        overview.metricPresentation !== 'strip' && 'grid gap-3 sm:grid-cols-2 2xl:grid-cols-4',
        variant === 'secondary' && 'xl:grid-cols-4',
        className,
      )}
      {...props}
    >
      {children}
    </div>
  )
}

/* ── Sparkline mini-chart for KPI cards ─────────────────── */

function KpiSparkline({
  data,
  trendType = 'neutral',
}: {
  data: number[]
  trendType?: 'up' | 'neutral' | 'down'
}) {
  const chartData = useMemo(() => data.map((v, i) => ({ i, v })), [data])

  const strokeColor =
    trendType === 'up'
      ? 'hsl(var(--heroui-success))'
      : trendType === 'down'
        ? 'hsl(var(--heroui-danger))'
        : 'hsl(var(--heroui-warning))'

  const fillId = `kpi-spark-${trendType}`

  if (chartData.length < 2) return null

  return (
    <div className="h-[52px] w-[140px] shrink-0">
      <ResponsiveContainer width="100%" height="100%">
        <AreaChart data={chartData} margin={{ top: 2, right: 2, bottom: 2, left: 2 }}>
          <defs>
            <linearGradient id={fillId} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={strokeColor} stopOpacity={0.2} />
              <stop offset="100%" stopColor={strokeColor} stopOpacity={0} />
            </linearGradient>
          </defs>
          <Area
            type="monotone"
            dataKey="v"
            stroke={strokeColor}
            strokeWidth={1.5}
            fill={`url(#${fillId})`}
            dot={false}
            isAnimationActive={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  )
}

/* ── Trend chip (HeroUI Pro KPI Stat 1 style) ──────────── */

function TrendChip({
  change,
  trendType,
  changeType,
  comparisonLabel,
}: {
  change: string
  trendType: 'up' | 'neutral' | 'down'
  changeType: 'positive' | 'neutral' | 'negative'
  comparisonLabel?: string
}) {
  const TrendIcon =
    trendType === 'up' ? ArrowUpRight : trendType === 'down' ? ArrowDownRight : ArrowRight
  const chipColor =
    changeType === 'positive' ? 'success' : changeType === 'negative' ? 'danger' : 'warning'

  return (
    <div className="flex items-center gap-1.5">
      <Chip
        color={chipColor}
        radius="sm"
        size="sm"
        variant="flat"
        classNames={{ content: 'font-medium text-[0.65rem]' }}
        startContent={<TrendIcon className="h-3 w-3" />}
      >
        {change}
      </Chip>
      {comparisonLabel ? (
        <span className="text-[11px] text-default-400">{comparisonLabel}</span>
      ) : null}
    </div>
  )
}

/* ── Dashboard Metric Card (HeroUI Pro KPI Stat 3 style) ── */

interface DashboardMetricCardProps extends Omit<HTMLAttributes<HTMLDivElement>, 'title'> {
  eyebrow?: ReactNode
  title: ReactNode
  value: ReactNode
  valueTitle?: string
  description?: ReactNode
  icon?: ReactNode
  loading?: boolean
  variant?: 'primary' | 'secondary'
  sparklineData?: number[]
  trendChange?: string
  trendType?: 'up' | 'neutral' | 'down'
  changeType?: 'positive' | 'neutral' | 'negative'
  comparisonLabel?: string
}

export function DashboardMetricCard({
  eyebrow,
  title,
  value,
  valueTitle,
  description,
  icon,
  loading = false,
  variant = 'primary',
  sparklineData,
  trendChange,
  trendType = 'neutral',
  changeType = 'neutral',
  comparisonLabel,
  className,
  ...props
}: DashboardMetricCardProps) {
  const isPrimary = variant === 'primary'
  const hasSparkline = isPrimary && sparklineData && sparklineData.length >= 2
  const hasTrend = !!trendChange

  return (
    <div
      className={cn(
        'h-full bg-content1/82',
        isPrimary ? 'flex flex-col justify-between px-4 py-4' : 'flex flex-col justify-center px-4 py-3',
        className,
      )}
      {...props}
    >
      {/* Header: title + sparkline (primary) or title + icon (secondary) */}
      <div className={cn('flex items-start justify-between gap-3', isPrimary && 'mb-1')}>
        <div className="min-w-0 space-y-1">
          {eyebrow ? (
            <p className="text-xs font-semibold uppercase tracking-[0.15em] text-muted-foreground">
              {eyebrow}
            </p>
          ) : null}
          <p className={cn(
            'font-medium',
            isPrimary
              ? 'text-small text-default-500'
              : 'text-[11px] font-semibold uppercase tracking-[0.1em] text-default-400',
          )}>
            {title}
          </p>
        </div>
        {isPrimary && hasSparkline ? (
          <KpiSparkline data={sparklineData} trendType={trendType} />
        ) : icon ? (
          <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-medium border-small border-default-200 bg-default-50 text-default-400">
            {icon}
          </div>
        ) : null}
      </div>

      {/* Value */}
      <div className={cn(isPrimary ? 'space-y-2' : 'space-y-0.5')}>
        {loading ? (
          <div className={cn(
            'animate-pulse rounded-lg bg-default-200/75',
            isPrimary ? 'h-9 w-32' : 'h-7 w-24',
          )} />
        ) : (
          <p
            title={valueTitle}
            className={cn(
              'font-semibold leading-none tracking-[-0.04em] text-foreground',
              isPrimary
                ? 'text-[clamp(1.7rem,2.7vw,2.35rem)]'
                : 'text-[clamp(1.2rem,2vw,1.5rem)]',
            )}
          >
            {value}
          </p>
        )}

        {/* Trend chip (primary) or description (secondary) */}
        {loading ? (
          <div className="h-3.5 w-40 animate-pulse rounded bg-default-200/70" />
        ) : hasTrend ? (
          <TrendChip
            change={trendChange}
            trendType={trendType}
            changeType={changeType}
            comparisonLabel={comparisonLabel}
          />
        ) : description ? (
          <p className={cn(
            'leading-6 text-muted-foreground',
            isPrimary ? 'text-[12px]' : 'text-[11px]',
          )}>
            {description}
          </p>
        ) : null}
      </div>
    </div>
  )
}
