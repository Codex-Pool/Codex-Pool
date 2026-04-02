import i18n from '@/i18n'
import { formatCompactNumber as formatCompactNumberBase } from './compact-number'

type DateTimePreset = 'date' | 'time' | 'timeWithSeconds' | 'datetime'

const DATE_TIME_PRESET_OPTIONS: Record<DateTimePreset, Intl.DateTimeFormatOptions> = {
  date: {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  },
  time: {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  },
  timeWithSeconds: {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  },
  datetime: {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  },
}

export function resolveLocale(locale?: string): string {
  const explicitLocale = locale?.trim()
  if (explicitLocale) return explicitLocale

  const resolvedLocale = i18n.resolvedLanguage?.trim()
  if (resolvedLocale) return resolvedLocale

  const currentLocale = i18n.language?.trim()
  if (currentLocale) return currentLocale

  return 'en-US'
}

function toValidDate(value: string | number | Date): Date | undefined {
  const date = value instanceof Date ? value : new Date(value)
  if (Number.isNaN(date.getTime())) {
    return undefined
  }
  return date
}

export function formatDateTime(
  value: string | number | Date,
  options: {
    locale?: string
    preset?: DateTimePreset
    fallback?: string
    timeZone?: string
    timeZoneName?: 'short' | 'long' | 'shortOffset' | 'longOffset' | 'shortGeneric' | 'longGeneric'
  } = {},
): string {
  const date = toValidDate(value)
  if (!date) {
    return options.fallback ?? '-'
  }

  const presetOptions = DATE_TIME_PRESET_OPTIONS[options.preset ?? 'datetime']
  const formatter = new Intl.DateTimeFormat(
    resolveLocale(options.locale),
    {
      ...presetOptions,
      ...(options.timeZone ? { timeZone: options.timeZone } : {}),
      ...(options.timeZoneName ? { timeZoneName: options.timeZoneName } : {}),
    },
  )
  return formatter.format(date)
}

export function formatUtcDateTime(
  value: string | number | Date,
  options: {
    locale?: string
    preset?: DateTimePreset
    fallback?: string
    timeZoneName?: 'short' | 'long' | 'shortOffset' | 'longOffset' | 'shortGeneric' | 'longGeneric'
  } = {},
): string {
  return formatDateTime(value, {
    ...options,
    timeZone: 'UTC',
  })
}

export function getUserTimeZone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'
}

export function formatNumber(
  value: number | undefined,
  options: Intl.NumberFormatOptions & {
    locale?: string
    fallback?: string
  } = {},
): string {
  if (typeof value !== 'number' || Number.isNaN(value)) {
    return options.fallback ?? '-'
  }

  const { locale, fallback, ...numberOptions } = options
  void fallback
  return new Intl.NumberFormat(resolveLocale(locale), numberOptions).format(value)
}

export function formatCompactNumber(
  value: number | undefined,
  options: {
    locale?: string
    fallback?: string
    maximumFractionDigits?: number
  } = {},
): string {
  return formatCompactNumberBase(value, {
    ...options,
    locale: resolveLocale(options.locale),
  })
}

export function formatPercent(
  value: number | undefined,
  options: {
    locale?: string
    minimumFractionDigits?: number
    maximumFractionDigits?: number
    fallback?: string
    inputScale?: 'fraction' | 'percent'
  } = {},
): string {
  if (typeof value !== 'number' || Number.isNaN(value)) {
    return options.fallback ?? '-'
  }

  return formatNumber(
    options.inputScale === 'percent' ? value / 100 : value,
    {
      locale: options.locale,
      fallback: options.fallback,
      style: 'percent',
      minimumFractionDigits: options.minimumFractionDigits,
      maximumFractionDigits: options.maximumFractionDigits,
    },
  )
}
