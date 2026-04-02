import i18n from 'i18next'
import type { ResourceLanguage } from 'i18next'
import { initReactI18next } from 'react-i18next'
import LanguageDetector from 'i18next-browser-languagedetector'

import en from './locales/en.ts'
import zhCN from './locales/zh-CN.ts'

export const supportedLanguages = ['en', 'zh-CN'] as const
export const fallbackLanguage = 'en' as const

type SupportedLanguage = (typeof supportedLanguages)[number]
const bundledResources: Record<SupportedLanguage, ResourceLanguage> = {
  en,
  'zh-CN': zhCN,
}

export function normalizeLanguage(rawLanguage?: string | null): SupportedLanguage {
  if (!rawLanguage) {
    return fallbackLanguage
  }

  const normalized = rawLanguage.trim().replace('_', '-').toLowerCase()
  if (!normalized || normalized === 'cimode') {
    return fallbackLanguage
  }

  if (normalized === 'en' || normalized.startsWith('en-')) {
    return 'en'
  }

  if (
    normalized === 'zh'
    || normalized === 'zh-cn'
    || normalized.startsWith('zh-cn')
    || normalized.startsWith('zh-hans')
    || normalized.startsWith('zh-hant')
    || normalized.startsWith('zh-tw')
  ) {
    return 'zh-CN'
  }

  return fallbackLanguage
}

async function ensureSupportedLanguage(language?: string | null) {
  const nextLanguage = normalizeLanguage(language)
  if (typeof window !== 'undefined') {
    try {
      window.localStorage.setItem('codex-ui-language', nextLanguage)
    } catch {
      // ignore storage errors (private mode / quota)
    }
    const url = new URL(window.location.href)
    if (url.searchParams.has('lng')) {
      url.searchParams.delete('lng')
      window.history.replaceState({}, '', `${url.pathname}${url.search}${url.hash}`)
    }
  }

  if (!i18n.hasResourceBundle(nextLanguage, 'translation')) {
    i18n.addResourceBundle(
      nextLanguage,
      'translation',
      bundledResources[nextLanguage],
      true,
      true,
    )
  }
  if (i18n.language !== nextLanguage) {
    await i18n.changeLanguage(nextLanguage)
  }
}

export async function setAppLanguage(language?: string | null) {
  await ensureSupportedLanguage(language)
}

const initPromise = i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: en },
      'zh-CN': { translation: zhCN },
    },
    supportedLngs: [...supportedLanguages],
    fallbackLng: fallbackLanguage,
    detection: {
      order: ['localStorage', 'navigator', 'htmlTag'],
      lookupLocalStorage: 'codex-ui-language',
      caches: ['localStorage'],
      excludeCacheFor: ['cimode'],
    },
    interpolation: {
      escapeValue: false,
    },
    nonExplicitSupportedLngs: false,
  })

i18n.on('languageChanged', (language) => {
  void ensureSupportedLanguage(language)
})

void initPromise.then(() => {
  void ensureSupportedLanguage(i18n.resolvedLanguage ?? i18n.language)
})

export default i18n
