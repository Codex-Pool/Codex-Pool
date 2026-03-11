import { NavLink, Outlet } from 'react-router-dom'
import {
    LayoutDashboard,
    Users,
    UserCog,
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
    FlaskConical,
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
import { useTranslation } from 'react-i18next'
import { ThemeToggle } from '@/components/ThemeToggle'
import { LanguageToggle } from '@/components/LanguageToggle'
import { ParallaxBackground } from '@/components/ui/parallax-background'
import { Button } from '@/components/ui/button'
import { useEffect, useMemo, useRef, useState } from 'react'
import codexPoolLogo from '@/assets/codex-pool-logo.png'

const MOBILE_FOCUSABLE_SELECTOR = [
    'a[href]',
    'button:not([disabled])',
    'input:not([disabled])',
    'select:not([disabled])',
    'textarea:not([disabled])',
    '[tabindex]:not([tabindex="-1"])',
].join(',')

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
    role = 'admin',
}: AppLayoutProps) {
    const { t } = useTranslation()
    const prefersReducedMotion = useReducedMotion()
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
                { path: '/models', icon: Box, label: t('nav.models') },
                { path: '/tenants', icon: Users, label: t('nav.tenants') },
                { path: '/proxies', icon: Network, label: t('nav.proxies') },
            ]
        },
        {
            label: t('nav.groups.operations'),
            items: [
                { path: '/groups', icon: Layers3, label: t('nav.apiKeyGroups') },
                { path: '/ai-routing', icon: Route, label: t('nav.aiRouting') },
                { path: '/oauth-import', icon: ShieldCheck, label: t('nav.oauthImport') },
                { path: '/oauth-probe', icon: FlaskConical, label: t('nav.oauthProbe') },
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
    const visibleMenuGroups = useMemo(() => {
        return resolvedMenuGroups
            .map((group) => ({
                ...group,
                items: group.items.filter((item) => canAccessByRole(item.roles, role)),
            }))
            .filter((group) => group.items.length > 0)
    }, [resolvedMenuGroups, role])

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
                        className="fixed inset-0 z-30 bg-black/40 backdrop-blur-[1px] md:hidden"
                        onClick={() => setMobileSidebarOpen(false)}
                        aria-label={t('nav.closeNavigation', { defaultValue: 'Close navigation menu' })}
                    />
                ) : null}
                {/* Sidebar */}
                <motion.aside
                    ref={mobileDrawerRef}
                    initial={prefersReducedMotion ? false : { x: -20, opacity: 0 }}
                    animate={prefersReducedMotion ? undefined : { x: 0, opacity: 1 }}
                    transition={prefersReducedMotion ? undefined : { duration: 0.35, ease: "easeOut" }}
                    tabIndex={-1}
                    role={mobileSidebarOpen ? 'dialog' : undefined}
                    aria-modal={mobileSidebarOpen ? true : undefined}
                    aria-label={t('nav.mainNavigation', { defaultValue: 'Main navigation' })}
                    className={cn(
                        "relative overflow-hidden border-r border-border/40 bg-card/40 backdrop-blur-2xl flex flex-col shrink-0 z-40",
                        "fixed inset-y-0 left-0 w-56 transform transition-transform md:static md:translate-x-0",
                        sidebarCollapsed ? "md:w-20" : "md:w-56",
                        mobileSidebarOpen ? "translate-x-0" : "-translate-x-full md:translate-x-0",
                    )}
                >
                    <div
                        aria-hidden="true"
                        className="pointer-events-none absolute inset-x-0 top-0 z-0 h-[300px] bg-[linear-gradient(180deg,rgba(248,250,252,0.96)_0%,rgba(226,232,240,0.82)_22%,rgba(203,213,225,0.42)_48%,rgba(148,163,184,0.12)_72%,transparent_92%)] dark:bg-[linear-gradient(180deg,rgba(2,6,23,0.98)_0%,rgba(15,23,42,0.88)_24%,rgba(51,65,85,0.46)_50%,rgba(148,163,184,0.12)_74%,transparent_92%)]"
                    />
                    <div
                        aria-hidden="true"
                        className="pointer-events-none absolute inset-x-0 top-0 z-0 h-[220px] bg-[radial-gradient(circle_at_14%_10%,rgba(255,255,255,0.62)_0%,rgba(255,255,255,0.20)_18%,rgba(255,255,255,0.06)_34%,transparent_58%)] dark:bg-[radial-gradient(circle_at_14%_10%,rgba(226,232,240,0.18)_0%,rgba(148,163,184,0.09)_22%,rgba(125,211,252,0.04)_36%,transparent_58%)]"
                    />
                    <div
                        className={cn(
                            "relative z-[1] overflow-hidden flex items-center justify-start border-b border-border/30 shrink-0 bg-white/18 shadow-[inset_0_1px_0_rgba(255,255,255,0.24)] backdrop-blur-[2px] dark:border-white/5 dark:bg-white/[0.03] dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]",
                            sidebarCollapsed ? "h-16 px-3 pr-10" : "h-20 px-4 pr-12",
                        )}
                    >
                        <div
                            aria-hidden="true"
                            className="pointer-events-none absolute inset-x-0 top-0 h-px bg-[linear-gradient(90deg,rgba(255,255,255,0.54)_0%,rgba(255,255,255,0.10)_36%,rgba(255,255,255,0.28)_100%)] dark:bg-[linear-gradient(90deg,rgba(255,255,255,0.08)_0%,rgba(255,255,255,0.02)_36%,rgba(226,232,240,0.12)_100%)]"
                        />
                        <div
                            aria-hidden="true"
                            className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_12%_24%,rgba(255,255,255,0.22)_0%,rgba(255,255,255,0.08)_18%,transparent_44%)] dark:bg-[radial-gradient(circle_at_12%_24%,rgba(226,232,240,0.08)_0%,rgba(148,163,184,0.04)_24%,transparent_48%)]"
                        />
                        <div className="relative z-[1] flex min-w-0 flex-1 items-center justify-start gap-3 pr-2">
                            <div
                                className={cn(
                                    "relative isolate shrink-0 flex items-center justify-start",
                                    sidebarCollapsed ? "h-10 w-10" : "h-14 w-14",
                                )}
                            >
                                <div
                                    aria-hidden="true"
                                    className={cn(
                                        "pointer-events-none absolute inset-0 -z-10 rounded-full blur-xl bg-[radial-gradient(circle,rgba(255,255,255,0.26)_0%,rgba(203,213,225,0.16)_30%,rgba(148,163,184,0.08)_50%,transparent_78%)] dark:bg-[radial-gradient(circle,rgba(241,245,249,0.18)_0%,rgba(148,163,184,0.12)_34%,rgba(125,211,252,0.05)_56%,transparent_80%)]",
                                        sidebarCollapsed ? "scale-90 opacity-55" : "scale-110 opacity-70",
                                    )}
                                />
                                <img
                                    src={codexPoolLogo}
                                    alt=""
                                    aria-hidden="true"
                                    className={cn(
                                        "relative z-[1] object-contain [filter:drop-shadow(0_10px_18px_rgba(15,23,42,0.14))] dark:[filter:drop-shadow(0_0_12px_rgba(226,232,240,0.10))_drop-shadow(0_12px_26px_rgba(2,6,23,0.50))]",
                                        sidebarCollapsed ? "h-10 w-10" : "h-14 w-14",
                                    )}
                                />
                            </div>
                            {!sidebarCollapsed ? (
                                <h1 className="min-w-0 flex-1 truncate text-[13.5px] font-semibold tracking-[0.01em] text-slate-700 [text-shadow:0_1px_0_rgba(255,255,255,0.20)] dark:text-slate-200 dark:[text-shadow:0_1px_0_rgba(255,255,255,0.08)]">
                                    {appName}
                                </h1>
                            ) : null}
                        </div>
                        <Button
                            variant="ghost"
                            size="icon"
                            className="absolute right-1 top-1/2 -translate-y-1/2 h-8 w-8 rounded-md text-muted-foreground hover:text-foreground md:hidden"
                            onClick={() => setMobileSidebarOpen(false)}
                            aria-label={t('nav.closeNavigation', { defaultValue: 'Close navigation menu' })}
                        >
                            <X className="h-4 w-4" />
                        </Button>
                    </div>

                    <nav
                        aria-label={t('nav.mainNavigation', { defaultValue: 'Main navigation' })}
                        className={cn("flex-1 overflow-y-auto py-6 scrollbar-none", sidebarCollapsed ? "space-y-4" : "space-y-8")}
                    >
                        {visibleMenuGroups.map((group, i) => (
                            <div key={i} className={cn(sidebarCollapsed ? "px-2" : "px-4")}>
                                {!sidebarCollapsed ? (
                                    <h2 className="mb-2 px-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/70">
                                        {group.label}
                                    </h2>
                                ) : null}
                                <ul className="space-y-0.5">
                                    {group.items.map(item => (
                                        <li key={item.path}>
                                            <NavLink
                                                to={item.path}
                                                title={sidebarCollapsed ? item.label : undefined}
                                                onClick={() => setMobileSidebarOpen(false)}
                                                className={({ isActive }) =>
                                                    cn(
                                                        "relative flex items-center rounded-md text-sm font-medium transition-colors group",
                                                        sidebarCollapsed ? "justify-center px-2 py-2" : "px-2 py-1.5",
                                                        isActive
                                                            ? "text-primary"
                                                            : "text-muted-foreground hover:text-foreground hover:bg-muted/50"
                                                    )
                                                }
                                            >
                                                {({ isActive }) => (
                                                    <>
                                                        {isActive && (
                                                            <motion.div
                                                                layoutId="activeNavIndicator"
                                                                className="absolute inset-0 bg-primary/10 rounded-md"
                                                                initial={false}
                                                                transition={{ type: "spring", stiffness: 350, damping: 30 }}
                                                            />
                                                        )}
                                                        <item.icon className={cn("h-4 w-4 relative z-10 transition-transform group-hover:scale-110", !sidebarCollapsed && "mr-2.5", isActive && "text-primary")} />
                                                        {!sidebarCollapsed ? (
                                                            <span className="relative z-10">{item.label}</span>
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
                    <div className={cn("p-4 border-t border-border/40 text-xs text-muted-foreground/60 flex items-center", sidebarCollapsed ? "flex-col justify-center gap-3" : "justify-between gap-3")}>
                        {!sidebarCollapsed ? <span>v1.0.0</span> : null}
                        <div className={cn("flex items-center", sidebarCollapsed ? "flex-col gap-3" : "ml-auto gap-3")}>
                            <span className="flex items-center gap-1.5" title={t('nav.online')}>
                                <span className="h-2 w-2 rounded-full bg-success motion-reduce:animate-none animate-pulse" />
                                {!sidebarCollapsed ? t('nav.online') : <span className="sr-only">{t('nav.online')}</span>}
                            </span>
                            <Button
                                variant="ghost"
                                size="icon"
                                className="hidden h-7 w-7 rounded-full border border-white/35 bg-white/20 text-muted-foreground/75 shadow-sm backdrop-blur-sm hover:bg-white/30 hover:text-foreground dark:border-white/10 dark:bg-white/5 dark:hover:bg-white/10 md:inline-flex"
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
                <main id="main-content" tabIndex={-1} className="flex-1 bg-transparent relative overflow-hidden flex flex-col z-20">
                    {/* Top Action Header */}
                    <header className="h-14 flex items-center justify-between px-4 sm:px-6 lg:px-8 shrink-0 z-30 gap-3 border-b border-border/10 bg-background/10 backdrop-blur-xl">
                        <div className="md:hidden">
                            <Button
                                ref={mobileOpenButtonRef}
                                variant="outline"
                                size="icon"
                                className="h-8 w-8"
                                onClick={() => {
                                    setSidebarCollapsed(false)
                                    setMobileSidebarOpen(true)
                                }}
                                aria-label={t('nav.openNavigation', { defaultValue: 'Open navigation menu' })}
                            >
                                <Menu className="h-4 w-4" />
                            </Button>
                        </div>
                        <div className="ml-auto flex items-center gap-3">
                            <LanguageToggle />
                            <ThemeToggle />
                            <Button
                                variant="outline"
                                size="sm"
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
                        <AnimatePresence mode="popLayout">
                            <Outlet />
                        </AnimatePresence>
                    </div>
                </main>
            </div>
        </>
    )
}
