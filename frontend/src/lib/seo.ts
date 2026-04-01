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
        titleKey: 'nav.dashboard',
        titleDefault: 'Dashboard',
        descriptionKey: 'dashboard.subtitle',
        descriptionDefault: 'Global gateway proxy metrics.',
    },
    {
        pattern: /^\/accounts\/?$/,
        titleKey: 'nav.accounts',
        titleDefault: 'Accounts Pool',
        descriptionKey: 'accounts.subtitle',
        descriptionDefault: 'Manage API credentials and billing health.',
    },
    {
        pattern: /^\/imports\/?$/,
        titleKey: 'nav.importJobs',
        titleDefault: 'Import Jobs',
        descriptionKey: 'importJobs.subtitle',
        descriptionDefault: 'Upload account secrets securely in strictly formatted CSV/TXT files.',
    },
    {
        pattern: /^\/oauth-import\/?$/,
        titleKey: 'nav.oauthImport',
        titleDefault: 'OAuth Login Import',
        descriptionKey: 'oauthImport.subtitle',
        descriptionDefault: 'Sign in with Codex OAuth and import the account directly into the pool.',
    },
    {
        pattern: /^\/groups\/?$/,
        titleKey: 'nav.apiKeyGroups',
        titleDefault: 'Group Management',
        descriptionKey: 'groupsPage.subtitle',
        descriptionDefault: 'Manage API key groups, model allowlists, multipliers, and group-level absolute prices.',
    },
    {
        pattern: /^\/model-routing\/?$/,
        titleKey: 'nav.modelRouting',
        titleDefault: 'Model Routing',
        descriptionKey: 'modelRoutingPage.subtitle',
        descriptionDefault: 'Manage routing profiles, model-aware policies, and model dispatch planning settings.',
    },
    {
        pattern: /^\/models\/?$/,
        titleKey: 'nav.models',
        titleDefault: 'Models',
        descriptionKey: 'models.subtitle',
        descriptionDefault: 'Available endpoints mapped from the accounts pool.',
    },
    {
        pattern: /^\/usage\/?$/,
        titleKey: 'nav.usage',
        titleDefault: 'Usage',
        descriptionKey: 'usage.subtitle',
        descriptionDefault: 'Request consumption and infrastructure profiling.',
    },
    {
        pattern: /^\/billing\/?$/,
        titleKey: 'nav.billing',
        titleDefault: 'Billing',
        descriptionKey: 'billing.subtitle',
        descriptionDefault: 'Review tenant billing, balances, and ledger activity.',
    },
    {
        pattern: /^\/(?:admin-api-keys|access-keys)\/?$/,
        titleKey: 'nav.apiKeys',
        titleDefault: 'Key Pool',
        descriptionKey: 'apiKeys.subtitle',
        descriptionDefault: 'Manage the standalone workspace key pool and issue secure access credentials.',
    },
    {
        pattern: /^\/proxies\/?$/,
        titleKey: 'nav.proxies',
        titleDefault: 'Proxy Nodes',
        descriptionKey: 'proxies.subtitle',
        descriptionDefault: 'Manage reverse proxy nodes and traffic routing topology.',
    },
    {
        pattern: /^\/tenants\/?$/,
        titleKey: 'nav.tenants',
        titleDefault: 'Tenants',
        descriptionKey: 'tenants.subtitle',
        descriptionDefault: 'Check tenant availability and manage profiles, API keys, and usage.',
    },
    {
        pattern: /^\/config\/?$/,
        titleKey: 'nav.config',
        titleDefault: 'Configuration',
        descriptionKey: 'config.subtitle',
        descriptionDefault: 'Runtime settings and global variables',
    },
    {
        pattern: /^\/logs\/?$/,
        titleKey: 'nav.logs',
        titleDefault: 'System Logs',
        descriptionKey: 'logs.subtitle',
        descriptionDefault: 'Real-time audit trails and operational context.',
    },
    {
        pattern: /^\/system\/?$/,
        titleKey: 'nav.system',
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
