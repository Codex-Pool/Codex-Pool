import { formatNumber, resolveLocale } from './i18n-format'

export function microusdToUsd(value: number | undefined): number | undefined {
  if (typeof value !== 'number' || Number.isNaN(value)) {
    return undefined
  }
  return value / 1_000_000
}

export function formatMicrousd(
  value: number | undefined,
  options: {
    locale?: string
    minimumFractionDigits?: number
    maximumFractionDigits?: number
    fallback?: string
  } = {},
): string {
  const usd = microusdToUsd(value)
  if (typeof usd !== 'number') {
    return options.fallback ?? '-'
  }

  return new Intl.NumberFormat(resolveLocale(options.locale), {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: options.minimumFractionDigits ?? 2,
    maximumFractionDigits: options.maximumFractionDigits ?? 4,
  }).format(usd)
}

export function formatMicrousdNumber(
  value: number | undefined,
  options: {
    locale?: string
    minimumFractionDigits?: number
    maximumFractionDigits?: number
    fallback?: string
  } = {},
): string {
  const usd = microusdToUsd(value)
  return formatNumber(usd, {
    locale: options.locale,
    minimumFractionDigits: options.minimumFractionDigits ?? 2,
    maximumFractionDigits: options.maximumFractionDigits ?? 4,
    fallback: options.fallback,
  })
}
