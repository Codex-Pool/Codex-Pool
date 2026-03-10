import { useTranslation } from 'react-i18next'

import type { OAuthAccountStatusResponse } from '@/api/accounts'
import { Skeleton } from '@/components/ui/skeleton'
import { cn } from '@/lib/utils'

import { extractRateLimitDisplays, sortRateLimitDisplays } from './utils'
import { buildCompactRateLimitRows, getCompactRateLimitBarColor } from './rate-limit-layout'

type RateLimitCellProps = {
  status?: OAuthAccountStatusResponse
  locale: string
  refreshing: boolean
}

export function RateLimitCell({ status, locale, refreshing }: RateLimitCellProps) {
  const { t } = useTranslation()
  const wrapperClass = 'flex w-[360px] min-w-0 min-h-[38px] flex-col justify-center gap-1'
  const displays = sortRateLimitDisplays(extractRateLimitDisplays(status))
  const compactRows = buildCompactRateLimitRows(displays, {
    locale,
    fiveHoursLabel: t('accounts.rateLimits.shortLabels.fiveHours', { defaultValue: '5h' }),
    oneWeekLabel: t('accounts.rateLimits.shortLabels.oneWeek', { defaultValue: '7d' }),
    noResetText: t('accounts.rateLimits.noReset'),
  })

  if (compactRows.length === 0 && !refreshing) {
    return (
      <div className={wrapperClass}>
        <span className="text-xs text-muted-foreground">{t('accounts.rateLimits.unavailable')}</span>
      </div>
    )
  }

  if (refreshing) {
    return (
      <div className={wrapperClass}>
        <div className="flex items-center gap-2">
          <Skeleton className="h-4 flex-1 rounded-sm" />
          <Skeleton className="h-6 w-10 rounded-md" />
        </div>
        <div className="flex items-center gap-2">
          <Skeleton className="h-3 w-20" />
          <Skeleton className="h-1.5 flex-1 rounded-full" />
        </div>
      </div>
    )
  }

  return (
    <div className={wrapperClass}>
      <div className={cn('grid gap-2', compactRows.length > 1 ? 'grid-cols-2' : 'grid-cols-1')}>
        {compactRows.map((row) => (
          <div key={row.bucket} className="rounded-md border border-border/50 bg-muted/15 px-2 py-1.5">
            <div className="flex items-center justify-between gap-2 text-[10px] leading-none text-muted-foreground">
              <span className="tabular-nums font-semibold text-foreground">{row.remainingText}</span>
              <span className="tabular-nums">{row.resetText}</span>
              <span className="shrink-0 text-[9px] font-medium uppercase tracking-[0.08em] text-muted-foreground/80">
                {row.bucketText}
              </span>
            </div>
            <div className="mt-1.5 h-0.5 overflow-hidden rounded-full bg-muted-foreground/15">
              <div
                className={cn('h-full transition-[width,background-color] duration-300')}
                style={{
                  width: `${row.progressPercent}%`,
                  backgroundColor: getCompactRateLimitBarColor(row.progressPercent),
                }}
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
