import type { TFunction } from 'i18next'

const APP_NAME = 'Codex-Pool'
const RTL_LANGUAGE_PREFIXES = ['ar', 'fa', 'he', 'ps', 'ur']

type RouteSeoConfig = {
    pattern: RegExp
    titleKey: string
    titleDefault: string
    descriptionKey: string
    descriptionDefault: string
}

const routeSeoConfigs: RouteSeoConfig[] = [
    {
        pattern: /^\/(?:dashboard)?\/?$/,
        titleKey: 'dashboard.title',
        titleDefault: 'Overview',
        descriptionKey: 'dashboard.subtitle',
        descriptionDefault: 'Global gateway proxy metrics.',
    },
    {
        pattern: /^\/accounts\/?$/,
        titleKey: 'accounts.title',
        titleDefault: 'Accounts Pool',
        descriptionKey: 'accounts.subtitle',
        descriptionDefault: 'Manage API credentials and billing health.',
    },
    {
        pattern: /^\/imports\/?$/,
        titleKey: 'importJobs.title',
        titleDefault: 'Batch Import Jobs',
        descriptionKey: 'importJobs.subtitle',
        descriptionDefault: 'Upload account secrets securely in strictly formatted CSV/TXT files.',
    },
    {
        pattern: /^\/oauth-import\/?$/,
        titleKey: 'oauthImport.title',
        titleDefault: 'OAuth Login Import',
        descriptionKey: 'oauthImport.subtitle',
        descriptionDefault: 'Sign in with Codex OAuth and import the account directly into the pool.',
    },
    {
        pattern: /^\/groups\/?$/,
        titleKey: 'groupsPage.title',
        titleDefault: 'Group Management',
        descriptionKey: 'groupsPage.subtitle',
        descriptionDefault: 'Manage API key groups, model allowlists, multipliers, and group-level absolute prices.',
    },
    {
        pattern: /^\/ai-routing\/?$/,
        titleKey: 'aiRoutingPage.title',
        titleDefault: 'AI Routing',
        descriptionKey: 'aiRoutingPage.subtitle',
        descriptionDefault: 'Manage routing profiles, model-aware policies, and AI planning settings.',
    },
    {
        pattern: /^\/models\/?$/,
        titleKey: 'models.title',
        titleDefault: 'Models',
        descriptionKey: 'models.subtitle',
        descriptionDefault: 'Available endpoints mapped from the accounts pool.',
    },
    {
        pattern: /^\/usage\/?$/,
        titleKey: 'usage.title',
        titleDefault: 'Usage Analysis',
        descriptionKey: 'usage.subtitle',
        descriptionDefault: 'Request consumption and infrastructure profiling.',
    },
    {
        pattern: /^\/billing\/?$/,
        titleKey: 'nav.usage',
        titleDefault: 'Billing',
        descriptionKey: 'usage.subtitle',
        descriptionDefault: 'Request consumption and infrastructure profiling.',
    },
    {
        pattern: /^\/proxies\/?$/,
        titleKey: 'proxies.title',
        titleDefault: 'Proxy Nodes',
        descriptionKey: 'proxies.subtitle',
        descriptionDefault: 'Manage reverse proxy nodes and traffic routing topology.',
    },
    {
        pattern: /^\/tenants\/?$/,
        titleKey: 'nav.accounts',
        titleDefault: 'Tenants',
        descriptionKey: 'accounts.subtitle',
        descriptionDefault: 'Manage API credentials and billing health.',
    },
    {
        pattern: /^\/config\/?$/,
        titleKey: 'config.title',
        titleDefault: 'Configuration',
        descriptionKey: 'config.subtitle',
        descriptionDefault: 'Runtime settings and global variables',
    },
    {
        pattern: /^\/logs\/?$/,
        titleKey: 'logs.title',
        titleDefault: 'System Logs',
        descriptionKey: 'logs.subtitle',
        descriptionDefault: 'Real-time audit trails and operational context.',
    },
    {
        pattern: /^\/system\/?$/,
        titleKey: 'system.title',
        titleDefault: 'System Status',
        descriptionKey: 'system.subtitle',
        descriptionDefault: 'Infrastructure dependencies and health self-check.',
    },
    {
        pattern: /^\/tenant(?:\/.*)?$/,
        titleKey: 'login.title',
        titleDefault: 'Codex-Pool Console',
        descriptionKey: 'login.subtitle',
        descriptionDefault: 'Sign in with your admin account',
    },
    {
        pattern: /^\/login\/?$/,
        titleKey: 'login.title',
        titleDefault: 'Codex-Pool Console',
        descriptionKey: 'login.subtitle',
        descriptionDefault: 'Sign in with your admin account',
    },
]

const normalizeLanguageTag = (language?: string): string => {
    if (!language) {
        return 'zh-CN'
    }
    return language.replace(/_/g, '-')
}

const getDirectionForLanguage = (language: string): 'ltr' | 'rtl' => {
    const baseLanguage = normalizeLanguageTag(language).toLowerCase().split('-')[0]
    return RTL_LANGUAGE_PREFIXES.includes(baseLanguage) ? 'rtl' : 'ltr'
}

const getTranslation = (t: TFunction, key: string, defaultValue: string): string => {
    const translated = t(key, { defaultValue })
    return typeof translated === 'string' ? translated : defaultValue
}

const withAppName = (title: string): string => {
    const normalizedTitle = title.trim()
    if (!normalizedTitle) {
        return APP_NAME
    }
    return normalizedTitle.toLowerCase().includes(APP_NAME.toLowerCase())
        ? normalizedTitle
        : `${normalizedTitle} | ${APP_NAME}`
}

const getRouteSeoConfig = (pathname: string): RouteSeoConfig => {
    return (
        routeSeoConfigs.find((config) => config.pattern.test(pathname)) ?? routeSeoConfigs[0]
    )
}

const getDescriptionMeta = (): HTMLMetaElement => {
    const current =
        document.querySelector<HTMLMetaElement>('meta[name="description"]')
    if (current) {
        return current
    }
    const created = document.createElement('meta')
    created.name = 'description'
    document.head.appendChild(created)
    return created
}

export const syncDocumentLanguage = (language?: string): void => {
    if (typeof document === 'undefined') {
        return
    }
    const normalizedLanguage = normalizeLanguageTag(language)
    document.documentElement.lang = normalizedLanguage
    document.documentElement.dir = getDirectionForLanguage(normalizedLanguage)
}

export const applyRouteSeo = (pathname: string, t: TFunction): void => {
    if (typeof document === 'undefined') {
        return
    }
    const config = getRouteSeoConfig(pathname)
    const title = getTranslation(t, config.titleKey, config.titleDefault)
    const description = getTranslation(
        t,
        config.descriptionKey,
        config.descriptionDefault,
    )

    document.title = withAppName(title)
    getDescriptionMeta().content = description
}
