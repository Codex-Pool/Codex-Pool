export type CompactRateLimitBucket = 'five_hours' | 'one_week'

export type CompactRateLimitInput = {
  bucket: string
  remainingPercent: number
  resetsAt?: string
}

export type CompactRateLimitRow = {
  bucket: CompactRateLimitBucket
  bucketText: string
  remainingText: string
  resetText: string
  progressPercent: number
  span: 'full' | 'half'
}

type CompactRateLimitOptions = {
  locale: string
  fiveHoursLabel: string
  oneWeekLabel: string
  noResetText: string
  timeZone?: string
}

function clampPercent(value: number | undefined) {
  if (typeof value !== 'number' || Number.isNaN(value)) {
    return 0
  }
  return Math.min(100, Math.max(0, value))
}

function isCompactRateLimitBucket(value: string): value is CompactRateLimitBucket {
  return value === 'five_hours' || value === 'one_week'
}

function formatParts(date: Date, locale: string, timeZone?: string) {
  const formatter = new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
    timeZone,
  })

  return Object.fromEntries(
    formatter
      .formatToParts(date)
      .filter((part) => part.type !== 'literal')
      .map((part) => [part.type, part.value]),
  ) as Record<string, string>
}

export function formatCompactRemainingPercent(value: number | undefined) {
  const rounded = Math.round(clampPercent(value) * 10) / 10
  const numberText = Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1)
  return `${numberText}%`
}

export function getCompactRateLimitBarColor(value: number | undefined) {
  const percent = clampPercent(value)
  const hue = Math.round((percent / 100) * 120)
  return `hsl(${hue} 72% 44%)`
}

export function formatCompactRateLimitReset(
  bucket: CompactRateLimitBucket,
  resetsAt: string | undefined,
  options: Pick<CompactRateLimitOptions, 'locale' | 'noResetText' | 'timeZone'>,
) {
  if (!resetsAt) {
    return options.noResetText
  }

  const date = new Date(resetsAt)
  if (Number.isNaN(date.getTime())) {
    return options.noResetText
  }

  const parts = formatParts(date, options.locale, options.timeZone)
  if (bucket === 'five_hours') {
    return `${parts.hour}:${parts.minute}`
  }
  return `${parts.year}-${parts.month}-${parts.day} ${parts.hour}:${parts.minute}`
}

export function buildCompactRateLimitRows(
  displays: CompactRateLimitInput[],
  options: CompactRateLimitOptions,
): CompactRateLimitRow[] {
  const visibleByBucket = new Map<CompactRateLimitBucket, CompactRateLimitInput & { bucket: CompactRateLimitBucket }>()

  for (const item of displays) {
    if (isCompactRateLimitBucket(item.bucket)) {
      const visibleItem: CompactRateLimitInput & { bucket: CompactRateLimitBucket } = {
        ...item,
        bucket: item.bucket,
      }
      visibleByBucket.set(visibleItem.bucket, visibleItem)
    }
  }

  const visible = (['five_hours', 'one_week'] as const)
    .map((bucket) => visibleByBucket.get(bucket))
    .filter((item): item is CompactRateLimitInput & { bucket: CompactRateLimitBucket } => Boolean(item))

  const span = visible.length <= 1 ? 'full' : 'half'

  return visible.map((item) => ({
    bucket: item.bucket,
    bucketText: item.bucket === 'five_hours' ? options.fiveHoursLabel : options.oneWeekLabel,
    remainingText: formatCompactRemainingPercent(item.remainingPercent),
    resetText: formatCompactRateLimitReset(item.bucket, item.resetsAt, options),
    progressPercent: clampPercent(item.remainingPercent),
    span,
  }))
}
