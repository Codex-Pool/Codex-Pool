export function formatCompactNumber(
  value: number | undefined,
  options: {
    locale?: string
    fallback?: string
    maximumFractionDigits?: number
  } = {},
): string {
  if (typeof value !== 'number' || Number.isNaN(value)) {
    return options.fallback ?? '-'
  }

  const absValue = Math.abs(value)
  const formatter = new Intl.NumberFormat(options.locale, {
    minimumFractionDigits: 0,
    maximumFractionDigits: options.maximumFractionDigits ?? 1,
  })

  const units = [
    { threshold: 1_000_000_000_000, suffix: 'T' },
    { threshold: 1_000_000_000, suffix: 'B' },
    { threshold: 1_000_000, suffix: 'M' },
    { threshold: 1_000, suffix: 'K' },
  ] as const

  const matched = units.find((unit) => absValue >= unit.threshold)
  if (!matched) {
    return new Intl.NumberFormat(options.locale, {
      maximumFractionDigits: 0,
    }).format(value)
  }

  return `${formatter.format(value / matched.threshold)}${matched.suffix}`
}
