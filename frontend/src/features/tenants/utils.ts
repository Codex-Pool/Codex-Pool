import { formatNumber, resolveLocale } from '@/lib/i18n-format'
import { notify } from '@/lib/notification'

export const LABEL_CLASS_NAME = 'text-xs font-medium text-muted-foreground'
export const USAGE_API_KEY_FILTER_ALL = '__all__'

export type TenantProfileTab = 'profile' | 'keys' | 'usage'

export function formatMicrocredits(value: number, locale?: string) {
  return formatNumber(value / 1_000_000, {
    locale,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
    useGrouping: false,
  })
}

export function toIsoDatetime(value: string): string | undefined {
  const trimmed = value.trim()
  if (!trimmed) {
    return undefined
  }
  const ts = Date.parse(trimmed)
  if (Number.isNaN(ts)) {
    return undefined
  }
  return new Date(ts).toISOString()
}

export function toLocalDatetimeInput(value?: string | null): string {
  if (!value) {
    return ''
  }
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return ''
  }
  const localDate = new Date(date.getTime() - date.getTimezoneOffset() * 60000)
  return localDate.toISOString().slice(0, 16)
}

export function maskToken(token: string): string {
  if (token.length <= 12) {
    return '******'
  }
  return `${token.slice(0, 6)}...${token.slice(-4)}`
}

export interface CopyTextNotifications {
  successTitle: string
  successDescription: string
  errorTitle: string
  errorDescription: string
}

async function writeTextToClipboard(value: string) {
  try {
    await navigator.clipboard.writeText(value)
    return true
  } catch {
    if (typeof document === 'undefined' || !document.body) {
      return false
    }

    const textarea = document.createElement('textarea')
    textarea.value = value
    textarea.style.position = 'fixed'
    textarea.style.opacity = '0'
    document.body.appendChild(textarea)

    try {
      textarea.focus()
      textarea.select()
      return document.execCommand('copy')
    } catch {
      return false
    } finally {
      document.body.removeChild(textarea)
    }
  }
}

export async function copyText(value: string, notifications: CopyTextNotifications) {
  const didCopy = await writeTextToClipboard(value)

  if (!didCopy) {
    notify({
      variant: 'error',
      title: notifications.errorTitle,
      description: notifications.errorDescription,
    })
    return false
  }

  notify({
    variant: 'success',
    title: notifications.successTitle,
    description: notifications.successDescription,
  })
  return true
}

export function createDateTimeFormatter(locale?: string) {
  return new Intl.DateTimeFormat(resolveLocale(locale), {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  })
}

export function formatDateTimeValue(
  formatter: Intl.DateTimeFormat,
  value?: string | null,
) {
  if (!value) {
    return '-'
  }
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return '-'
  }
  return formatter.format(date)
}
