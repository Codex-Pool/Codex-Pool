import { NavLink, Outlet, useLocation } from 'react-router-dom'
import {
    LayoutDashboard,
    KeyRound,
    Users,
    UserCog,
    Archive,
    HardDriveDownload,
    ShieldCheck,
    Layers3,
    Route,
    Box,
    Activity,
    ReceiptText,
    Settings,
    Cpu,
    Network,
    TerminalSquare,
    LogOut,
    ChevronsLeft,
    ChevronsRight,
    Menu,
    X,
    type LucideIcon,
} from 'lucide-react'
import { motion, AnimatePresence, useReducedMotion } from 'framer-motion'
import { cn } from '@/lib/utils'
import { canAccessByRole, type AppRole } from '@/lib/permissions'
import { resolvePageEnterMotion } from '@/lib/motion-presets'
import { useTranslation } from 'react-i18next'
import { ThemeToggle } from '@/components/ThemeToggle'
import { LanguageToggle } from '@/components/LanguageToggle'
import { ParallaxBackground } from '@/components/ui/parallax-background'
import { Button } from '@/components/ui/button'
import type { SystemCapabilitiesResponse } from '@/api/types'
import { useEffect, useMemo, useRef, useState } from 'react'
import codexPoolLogo from '@/assets/codex-pool-logo.png'
import {
    filterAdminMenuGroupsByCapabilities,
    STANDALONE_ADMIN_API_KEYS_PATH,
} from '@/features/api-keys/admin-capabilities'

const MOBILE_FOCUSABLE_SELECTOR = [
    'a[href]',
    'button:not([disabled])',
    'input:not([disabled])',
    'select:not([disabled])',
    'textarea:not([disabled])',
    '[tabindex]:not([tabindex="-1"])',
].join(',')

const MOBILE_ICON_BUTTON_CLASS_NAME = 'h-11 w-11 md:h-8 md:w-8'
const MOBILE_INLINE_BUTTON_CLASS_NAME = 'min-h-11 px-4 md:min-h-0'

function getFocusableElements(container: HTMLElement) {
    return Array.from(container.querySelectorAll<HTMLElement>(MOBILE_FOCUSABLE_SELECTOR))
        .filter((element) =>
            !element.hasAttribute('disabled') &&
            element.getAttribute('aria-hidden') !== 'true'
        )
}

interface AppLayoutProps {
    onLogout: () => Promise<void>
    appName?: string
    menuGroups?: AppLayoutMenuGroup[]
    capabilities?: SystemCapabilitiesResponse
    role?: AppRole
}

export interface AppLayoutMenuItem {
    path: string
    icon: LucideIcon
    label: string
    roles?: AppRole[]
}

export interface AppLayoutMenuGroup {
    label: string
    items: AppLayoutMenuItem[]
}

export function AppLayout({
    onLogout,
    appName = 'Codex Pool',
    menuGroups,
    capabilities,
    role = 'admin',
}: AppLayoutProps) {
    const { t } = useTranslation()
    const location = useLocation()
    const prefersReducedMotion = useReducedMotion()
    const shellEnterMotion = resolvePageEnterMotion(prefersReducedMotion)
    const [sidebarCollapsed, setSidebarCollapsed] = useState(false)
    const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false)
    const mobileDrawerRef = useRef<HTMLElement | null>(null)
    const mobileOpenButtonRef = useRef<HTMLButtonElement | null>(null)
    const previousFocusedElementRef = useRef<HTMLElement | null>(null)

    useEffect(() => {
        if (!mobileSidebarOpen) {
            return
        }

        const drawer = mobileDrawerRef.current
        if (!drawer) {
            return
        }

        const previousOverflow = document.body.style.overflow
        const openButton = mobileOpenButtonRef.current
        previousFocusedElementRef.current = document.activeElement instanceof HTMLElement
            ? document.activeElement
            : null

        const focusFirstElement = () => {
            const focusableElements = getFocusableElements(drawer)
            const target = focusableElements[0] ?? drawer
            target.focus()
        }

        const handleKeyDown = (event: KeyboardEvent) => {
            if (event.key === 'Escape') {
                event.preventDefault()
                setMobileSidebarOpen(false)
                return
            }

            if (event.key !== 'Tab') {
                return
            }

            const focusableElements = getFocusableElements(drawer)
            if (focusableElements.length === 0) {
                event.preventDefault()
                drawer.focus()
                return
            }

            const first = focusableElements[0]
            const last = focusableElements[focusableElements.length - 1]
            const active = document.activeElement as HTMLElement | null

            if (event.shiftKey) {
                if (active === first || !active || !drawer.contains(active)) {
                    event.preventDefault()
                    last.focus()
                }
                return
            }

            if (active === last) {
                event.preventDefault()
                first.focus()
            }
        }

        document.body.style.overflow = 'hidden'
        const focusRaf = window.requestAnimationFrame(focusFirstElement)
        window.addEventListener('keydown', handleKeyDown)

        return () => {
            window.cancelAnimationFrame(focusRaf)
            document.body.style.overflow = previousOverflow
            window.removeEventListener('keydown', handleKeyDown)
            const restoreTarget = previousFocusedElementRef.current ?? openButton
            restoreTarget?.focus()
        }
    }, [mobileSidebarOpen])

    const defaultAdminMenuGroups: AppLayoutMenuGroup[] = [
        {
            label: t('nav.groups.analytics'),
            items: [
                { path: '/dashboard', icon: LayoutDashboard, label: t('nav.dashboard') },
                { path: '/usage', icon: Activity, label: t('nav.usage') },
                { path: '/billing', icon: ReceiptText, label: t('nav.billing') },
            ]
        },
        {
            label: t('nav.groups.assets'),
            items: [
                { path: '/accounts', icon: UserCog, label: t('nav.accounts') },
                { path: '/inventory', icon: Archive, label: t('nav.inventory') },
                { path: '/models', icon: Box, label: t('nav.models') },
                { path: STANDALONE_ADMIN_API_KEYS_PATH, icon: KeyRound, label: t('nav.apiKeys') },
                { path: '/tenants', icon: Users, label: t('nav.tenants') },
                { path: '/proxies', icon: Network, label: t('nav.proxies') },
            ]
        },
        {
            label: t('nav.groups.operations'),
            items: [
                { path: '/groups', icon: Layers3, label: t('nav.apiKeyGroups') },
                { path: '/model-routing', icon: Route, label: t('nav.modelRouting') },
                { path: '/oauth-import', icon: ShieldCheck, label: t('nav.oauthImport') },
                { path: '/imports', icon: HardDriveDownload, label: t('nav.importJobs') },
            ]
        },
        {
            label: t('nav.groups.system'),
            items: [
                { path: '/config', icon: Settings, label: t('nav.config') },
                { path: '/logs', icon: TerminalSquare, label: t('nav.logs') },
                { path: '/system', icon: Cpu, label: t('nav.system') },
            ]
        }
    ]
    const resolvedMenuGroups = menuGroups ?? defaultAdminMenuGroups
    const capabilityScopedMenuGroups = useMemo(
        () => filterAdminMenuGroupsByCapabilities(resolvedMenuGroups, capabilities),
        [capabilities, resolvedMenuGroups],
    )
    const visibleMenuGroups = useMemo(() => {
        return capabilityScopedMenuGroups
            .map((group) => ({
                ...group,
                items: group.items.filter((item) => canAccessByRole(item.roles, role)),
            }))
            .filter((group) => group.items.length > 0)
    }, [capabilityScopedMenuGroups, role])

    return (
        <>
            <a
                href="#main-content"
                className="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-[200] focus:rounded-md focus:bg-primary focus:px-3 focus:py-2 focus:text-primary-foreground"
            >
                {t('common.skipToMainContent', { defaultValue: 'Skip to main content' })}
            </a>
            <ParallaxBackground />
            <div className="flex h-screen text-foreground overflow-hidden selection:bg-primary/20 relative z-10 w-full">
                {mobileSidebarOpen ? (
                    <button
                        type="button"
                        className="fixed inset-0 z-30 bg-slate-950/30 md:hidden"
                        onClick={() => setMobileSidebarOpen(false)}
                        aria-label={t('nav.closeNavigation', { defaultValue: 'Close navigation menu' })}
                    />
                ) : null}
                {/* Sidebar */}
                <motion.aside
                    ref={mobileDrawerRef}
                    initial={prefersReducedMotion ? false : { x: -shellEnterMotion.initial.y, opacity: shellEnterMotion.initial.opacity }}
                    animate={prefersReducedMotion ? undefined : { x: 0, opacity: shellEnterMotion.animate.opacity }}
                    transition={prefersReducedMotion ? undefined : shellEnterMotion.transition}
                    tabIndex={-1}
                    role={mobileSidebarOpen ? 'dialog' : undefined}
                    aria-modal={mobileSidebarOpen ? true : undefined}
                    aria-label={t('nav.mainNavigation', { defaultValue: 'Main navigation' })}
                    className={cn(
                        "app-shell-sidebar relative flex shrink-0 flex-col overflow-hidden border-r border-sidebar-border z-40",
                        "fixed inset-y-0 left-0 w-56 transform transition-transform md:static md:translate-x-0",
                        sidebarCollapsed ? "md:w-20" : "md:w-56",
                        mobileSidebarOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0",
                    )}
                >
                    <div
                        className={cn(
                            "relative z-[1] flex shrink-0 items-center justify-start border-b border-sidebar-border/85",
                            sidebarCollapsed ? "h-[3.75rem] px-3 pr-10" : "h-[4.5rem] px-4 pr-12",
                        )}
                    >
                        <div className="relative z-[1] flex min-w-0 flex-1 items-center justify-start gap-3.5 pr-2">
                            <div
                                className={cn(
                                    "relative flex shrink-0 items-center justify-start rounded-[0.8rem] border border-sidebar-border/80 bg-background/72",
                                    sidebarCollapsed ? "h-10 w-10" : "h-10 w-10",
                                )}
                            >
                                <img
                                    src={codexPoolLogo}
                                    alt=""
                                    aria-hidden="true"
                                    className={cn(
                                        "relative z-[1] object-contain",
                                        sidebarCollapsed ? "h-8 w-8" : "h-9 w-9",
                                    )}
                                />
                            </div>
                            {!sidebarCollapsed ? (
                                <div className="min-w-0 flex-1 space-y-0.5">
                                    <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-sidebar-foreground/42">
                                        Codex
                                    </p>
                                    <h1 className="max-h-[2.5rem] overflow-hidden break-words text-[13px] font-semibold leading-5 tracking-[0.02em] text-sidebar-foreground">
                                        {appName}
                                    </h1>
                                </div>
                            ) : null}
                        </div>
                        <Button
                            variant="ghost"
                            size="icon"
                            className={cn(
                                'absolute right-1 top-1/2 -translate-y-1/2 rounded-md text-muted-foreground hover:text-foreground md:hidden',
                                MOBILE_ICON_BUTTON_CLASS_NAME,
                            )}
                            onClick={() => setMobileSidebarOpen(false)}
                            aria-label={t('nav.closeNavigation', { defaultValue: 'Close navigation menu' })}
                        >
                            <X className="h-4 w-4" />
                        </Button>
                    </div>

                    <nav
                        aria-label={t('nav.mainNavigation', { defaultValue: 'Main navigation' })}
                        className={cn("flex-1 overflow-y-auto py-5 scrollbar-none", sidebarCollapsed ? "space-y-4" : "space-y-6")}
                    >
                        {visibleMenuGroups.map((group, i) => (
                            <div key={i} className={cn(sidebarCollapsed ? "px-2" : "px-4")}>
                                {!sidebarCollapsed ? (
                                    <div className="mb-2 flex items-center gap-2 px-2">
                                        <span className="h-px flex-1 bg-sidebar-border/90" />
                                        <h2 className="text-[10px] font-semibold uppercase tracking-[0.16em] text-muted-foreground/78">
                                            {group.label}
                                        </h2>
                                    </div>
                                ) : null}
                                <ul className="space-y-1">
                                    {group.items.map(item => (
                                        <li key={item.path}>
                                            <NavLink
                                                to={item.path}
                                                title={sidebarCollapsed ? item.label : undefined}
                                                onClick={() => setMobileSidebarOpen(false)}
                                                className={({ isActive }) =>
                                                    cn(
                                                        "group relative flex items-center overflow-hidden rounded-[0.85rem] border text-sm font-medium transition-[background-color,border-color,color,box-shadow,transform]",
                                                        sidebarCollapsed ? "justify-center px-2 py-2.5" : "px-3 py-2.5",
                                                        isActive
                                                            ? "border-sidebar-border/90 bg-background/92 text-foreground shadow-[0_10px_22px_rgba(67,79,97,0.08),inset_0_1px_0_rgba(255,255,255,0.3)]"
                                                            : "border-transparent text-muted-foreground hover:border-sidebar-border/70 hover:bg-background/62 hover:text-foreground"
                                                    )
                                                }
                                            >
                                                {({ isActive }) => (
                                                    <>
                                                        {isActive && (
                                                            <motion.div
                                                                layoutId="activeNavIndicator"
                                                                className="absolute inset-y-[6px] left-[7px] w-[3px] rounded-full bg-primary"
                                                                initial={false}
                                                                transition={{ duration: 0.18 }}
                                                            />
                                                        )}
                                                        <item.icon className={cn("relative z-10 h-4 w-4", !sidebarCollapsed && "mr-2.5", isActive && "text-primary")} />
                                                        {!sidebarCollapsed ? (
                                                            <span className="relative z-10 truncate tracking-[0.01em]">{item.label}</span>
                                                        ) : null}
                                                    </>
                                                )}
                                            </NavLink>
                                        </li>
                                    ))}
                                </ul>
                            </div>
                        ))}
                    </nav>
                    <div className={cn("flex items-center border-t border-sidebar-border/80 p-4 text-xs text-muted-foreground/72", sidebarCollapsed ? "flex-col justify-center gap-3" : "justify-between gap-3")}>
                        {!sidebarCollapsed ? <span>v1.0.0</span> : null}
                        <div className={cn("flex items-center", sidebarCollapsed ? "flex-col gap-3" : "ml-auto gap-3")}>
                            <span className="flex items-center gap-1.5" title={t('nav.online')}>
                                <span className="h-2 w-2 rounded-full bg-success motion-reduce:animate-none animate-pulse" />
                                {!sidebarCollapsed ? t('nav.online') : <span className="sr-only">{t('nav.online')}</span>}
                            </span>
                            <Button
                                variant="ghost"
                                size="icon"
                                className="hidden h-7 w-7 rounded-[0.7rem] border border-sidebar-border/80 bg-background/50 text-muted-foreground hover:bg-background/72 hover:text-foreground md:inline-flex"
                                onClick={() => setSidebarCollapsed((prev) => !prev)}
                                aria-label={sidebarCollapsed ? t('common.expandSidebar') : t('common.collapseSidebar')}
                                title={sidebarCollapsed ? t('common.expandSidebar') : t('common.collapseSidebar')}
                            >
                                {sidebarCollapsed ? <ChevronsRight className="h-4 w-4" /> : <ChevronsLeft className="h-4 w-4" />}
                            </Button>
                        </div>
                    </div>
                </motion.aside>

                {/* Main Content */}
                <main id="main-content" tabIndex={-1} className="relative z-20 flex flex-1 flex-col overflow-hidden bg-transparent">
                    {/* Top Action Header */}
                    <header className="app-shell-header flex h-14 items-center justify-between gap-3 border-b border-border/60 px-4 sm:px-6 lg:px-8">
                        <div className="md:hidden">
                            <Button
                                ref={mobileOpenButtonRef}
                                variant="outline"
                                size="icon"
                                className={MOBILE_ICON_BUTTON_CLASS_NAME}
                                onClick={() => {
                                    setSidebarCollapsed(false)
                                    setMobileSidebarOpen(true)
                                }}
                                aria-label={t('nav.openNavigation', { defaultValue: 'Open navigation menu' })}
                            >
                                <Menu className="h-4 w-4" />
                            </Button>
                        </div>
                        <div className="ml-auto flex items-center gap-2.5">
                            <LanguageToggle />
                            <ThemeToggle />
                            <Button
                                variant="outline"
                                size="sm"
                                className={MOBILE_INLINE_BUTTON_CLASS_NAME}
                                onClick={async () => {
                                    try {
                                        await onLogout()
                                    } catch {
                                        window.location.reload()
                                    }
                                }}
                            >
                                <LogOut className="h-4 w-4 mr-2" />
                                {t('common.logout')}
                            </Button>
                        </div>
                    </header>

                    <div className="flex-1 relative overflow-auto">
                        <AnimatePresence mode="wait" initial={false}>
                            <motion.div
                                key={location.pathname}
                                initial={shellEnterMotion.initial}
                                animate={shellEnterMotion.animate}
                                exit={shellEnterMotion.exit}
                                transition={shellEnterMotion.transition}
                                className="h-full"
                            >
                                <Outlet />
                            </motion.div>
                        </AnimatePresence>
                    </div>
                </main>
            </div>
        </>
    )
}
