import { NavLink, Outlet, useLocation } from "react-router-dom";
import {
  LayoutDashboard,
  KeyRound,
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
  LogOut,
  ChevronLeft,
  ChevronRight,
  Menu,
  X,
  type LucideIcon,
} from "lucide-react";
import {
  Button,
  Tooltip,
} from "@heroui/react";
import { cn } from "@/lib/utils";
import { useCallback, useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { useTranslation } from "react-i18next";
import type { SystemCapabilitiesResponse } from "@/api/types";
import { LanguageToggle } from "@/components/LanguageToggle";
import { useUiPreferences } from "@/components/use-ui-preferences";
import { ThemeToggleButton } from "@/components/ui/theme-toggle-button";
import type { AppPageHeader } from "@/components/layout/page-header-context";
import {
  filterAdminMenuGroupsByCapabilities,
  STANDALONE_ADMIN_API_KEYS_PATH,
} from "@/features/api-keys/admin-capabilities";

interface AppLayoutProps {
  onLogout: () => void;
  capabilities?: SystemCapabilitiesResponse;
  menuGroups?: AppLayoutMenuGroup[];
  username?: string;
}

export interface AppLayoutMenuItem {
  path: string;
  icon: LucideIcon;
  label: string;
}

export interface AppLayoutMenuGroup {
  label: string;
  items: AppLayoutMenuItem[];
}

interface ActiveNavigationContext {
  groupLabel: string;
  itemLabel: string;
}



export default function AppLayout({
  onLogout,
  capabilities,
  menuGroups,
  username,
}: AppLayoutProps) {
  const { t } = useTranslation();
  const location = useLocation();
  const { drawerPlacement } = useUiPreferences();
  const [collapsed, setCollapsed] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [pageHeader, setPageHeaderState] = useState<AppPageHeader | null>(null);
  const [pageHeaderBodyVisible, setPageHeaderBodyVisibleState] = useState(true);
  const setPageHeader = useCallback((header: AppPageHeader | null) => {
    setPageHeaderState(header);
  }, []);
  const setPageHeaderBodyVisible = useCallback((visible: boolean) => {
    setPageHeaderBodyVisibleState(visible);
  }, []);
  const navGroups = useMemo<AppLayoutMenuGroup[]>(
    () =>
      menuGroups ??
      filterAdminMenuGroupsByCapabilities(
        [
          {
            label: t("nav.groups.analytics", { defaultValue: "Analytics" }),
            items: [
              {
                path: "/dashboard",
                icon: LayoutDashboard,
                label: t("nav.dashboard", { defaultValue: "Dashboard" }),
              },
              {
                path: "/usage",
                icon: Activity,
                label: t("nav.usage", { defaultValue: "Usage" }),
              },
              {
                path: "/billing",
                icon: ReceiptText,
                label: t("nav.billing", { defaultValue: "Billing" }),
              },
            ],
          },
          {
            label: t("nav.groups.assets", { defaultValue: "Pool Assets" }),
            items: [
              {
                path: "/accounts",
                icon: UserCog,
                label: t("nav.accounts", { defaultValue: "Accounts Pool" }),
              },
              {
                path: "/models",
                icon: Box,
                label: t("nav.models", { defaultValue: "Models" }),
              },
              {
                path: STANDALONE_ADMIN_API_KEYS_PATH,
                icon: KeyRound,
                label: t("nav.apiKeys", { defaultValue: "Key Pool" }),
              },
              {
                path: "/tenants",
                icon: Users,
                label: t("nav.tenants", { defaultValue: "Tenants" }),
              },
              {
                path: "/proxies",
                icon: Network,
                label: t("nav.proxies", { defaultValue: "Proxies" }),
              },
            ],
          },
          {
            label: t("nav.groups.operations", { defaultValue: "Operations" }),
            items: [
              {
                path: "/groups",
                icon: Layers3,
                label: t("nav.apiKeyGroups", {
                  defaultValue: "Group Management",
                }),
              },
              {
                path: "/model-routing",
                icon: Route,
                label: t("nav.modelRouting", { defaultValue: "Model Routing" }),
              },
              {
                path: "/imports",
                icon: HardDriveDownload,
                label: t("nav.importJobs", { defaultValue: "Import Jobs" }),
              },
              {
                path: "/oauth-import",
                icon: ShieldCheck,
                label: t("nav.oauthImport", {
                  defaultValue: "OAuth Login Import",
                }),
              },
            ],
          },
          {
            label: t("nav.groups.system", { defaultValue: "System" }),
            items: [
              {
                path: "/config",
                icon: Settings,
                label: t("nav.config", { defaultValue: "Configuration" }),
              },
              {
                path: "/logs",
                icon: TerminalSquare,
                label: t("nav.logs", { defaultValue: "Logs" }),
              },
              {
                path: "/system",
                icon: Cpu,
                label: t("nav.system", { defaultValue: "System" }),
              },
            ],
          },
        ],
        capabilities,
      ),
    [capabilities, menuGroups, t],
  );
  const activeNavigationContext: ActiveNavigationContext | null = (() => {
    for (const group of navGroups) {
      for (const item of group.items) {
        if (
          location.pathname === item.path ||
          (item.path !== "/" && location.pathname.startsWith(`${item.path}/`))
        ) {
          return {
            groupLabel: group.label,
            itemLabel: item.label,
          };
        }
      }
    }

    return null;
  })();
  const compactHeaderTitle =
    pageHeader?.title ?? activeNavigationContext?.itemLabel ?? null;
  const showDockedPageActions =
    pageHeader?.mode === "dock-on-scroll" && !pageHeaderBodyVisible;
  const mobileDrawerPlacementClassName =
    drawerPlacement === "right"
      ? "right-0 left-auto translate-x-full"
      : drawerPlacement === "top"
        ? "inset-x-0 top-0 h-auto -translate-y-full"
        : drawerPlacement === "left"
          ? "left-0 -translate-x-full"
          : "inset-x-0 bottom-0 top-auto h-auto translate-y-full";
  const mobileDrawerOpenClassName =
    drawerPlacement === "right"
      ? "translate-x-0"
      : drawerPlacement === "top"
        ? "translate-y-0"
        : drawerPlacement === "left"
          ? "translate-x-0"
          : "translate-y-0";

  return (
    <div className="flex h-screen overflow-hidden bg-background text-foreground">
      {/* Mobile overlay */}
      <AnimatePresence>
        {mobileOpen && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.18 }}
            className="fixed inset-0 z-40 bg-black/50 md:hidden"
            onClick={() => setMobileOpen(false)}
          />
        )}
      </AnimatePresence>

      {/* Sidebar */}
      <aside
        className={cn(
          "fixed z-50 flex flex-col border-r border-default-100 bg-content1/70 backdrop-blur-xl transition-all duration-200 md:static md:inset-auto md:z-auto",
          collapsed ? "w-[68px]" : "w-60",
          mobileOpen ? mobileDrawerOpenClassName : mobileDrawerPlacementClassName,
          drawerPlacement === "top" || drawerPlacement === "bottom"
            ? "max-h-[min(80vh,42rem)] w-full border-b border-r-0 md:inset-y-0 md:left-0 md:max-h-none md:w-[68px] md:border-b-0 md:border-r"
            : "inset-y-0",
          drawerPlacement === "right" ? "md:left-0" : "",
          !collapsed && (drawerPlacement === "top" || drawerPlacement === "bottom") ? "md:w-60" : "",
          "md:translate-x-0 md:translate-y-0",
        )}
      >
        {/* Logo区域 */}
        <div
          className={cn(
            "flex h-16 shrink-0 items-center border-b border-default-100",
            collapsed ? "justify-center px-2" : "px-5",
          )}
        >
          <AnimatePresence initial={false}>
            {!collapsed && (
              <motion.div
                key="logo-text"
                initial={{ opacity: 0 }}
                animate={{ opacity: 1, transition: { duration: 0.12, delay: 0.08 } }}
                exit={{ opacity: 0, transition: { duration: 0.06 } }}
                className="flex items-center gap-3"
              >
                <img src="/favicon.svg" alt="Codex-Pool" className="h-8 w-8 rounded-lg" />
                <div>
                  <p className="text-xs font-semibold uppercase tracking-widest text-default-400">
                    Codex
                  </p>
                  <h1 className="text-sm font-semibold text-foreground">Pool</h1>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
          {collapsed && (
            <img src="/favicon.svg" alt="Codex-Pool" className="h-8 w-8 rounded-lg" />
          )}
          {/* Mobile close */}
          <Button
            isIconOnly
            variant="light"
            size="sm"
            aria-label={t("common.closeMenu", { defaultValue: "Close menu" })}
            className="absolute right-2 top-4 md:hidden"
            onPress={() => setMobileOpen(false)}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        {/* Navigation */}
        <nav className="flex-1 overflow-y-auto scrollbar-none py-4">
          {navGroups.map((group) => (
            <div key={group.label} className="mb-4">
              <AnimatePresence initial={false}>
                {!collapsed && (
                  <motion.p
                    key="group-label"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1, transition: { duration: 0.1, delay: 0.07 } }}
                    exit={{ opacity: 0, transition: { duration: 0.05 } }}
                    className="mb-2 px-5 text-xs font-semibold uppercase tracking-widest text-default-400"
                  >
                    {group.label}
                  </motion.p>
                )}
              </AnimatePresence>
              <ul className="space-y-0.5 px-3">
                {group.items.map((item) => {
                  const isActive = location.pathname === item.path;
                  return (
                    <li key={item.path}>
                      <Tooltip
                        content={item.label}
                        placement="right"
                        isDisabled={!collapsed}
                      >
                        <NavLink
                          to={item.path}
                          viewTransition
                          onClick={() => setMobileOpen(false)}
                          className={cn(
                            "flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-colors duration-150 relative",
                            collapsed && "justify-center px-0",
                            isActive
                              ? "text-foreground"
                              : "text-default-500 hover:bg-default-100 hover:text-foreground",
                          )}
                        >
                          {/* 弹簧背景高亮 — layoutId 使 Framer Motion 在导航项间 FLIP 动画 */}
                          {isActive && (
                            <motion.div
                              layoutId="nav-bg"
                              className="absolute inset-0 rounded-xl bg-primary/8"
                              initial={false}
                              transition={{ type: "spring", stiffness: 380, damping: 32 }}
                            />
                          )}
                          {/* 弹簧左侧 teal 竖条 */}
                          {isActive && (
                            <motion.span
                              layoutId="nav-indicator"
                              className="absolute left-0 top-1/2 h-1/2 w-[3px] -translate-y-1/2 rounded-r-full bg-primary"
                              initial={false}
                              transition={{ type: "spring", stiffness: 380, damping: 32 }}
                            />
                          )}
                          <item.icon
                            className={cn(
                              "relative z-10 h-[18px] w-[18px] shrink-0",
                              isActive && "text-primary",
                            )}
                          />
                          <AnimatePresence initial={false}>
                            {!collapsed && (
                              <motion.span
                                key="nav-label"
                                initial={{ opacity: 0 }}
                                animate={{ opacity: 1, transition: { duration: 0.1, delay: 0.06 } }}
                                exit={{ opacity: 0, transition: { duration: 0.04 } }}
                                className="relative z-10 truncate"
                              >
                                {item.label}
                              </motion.span>
                            )}
                          </AnimatePresence>
                        </NavLink>
                      </Tooltip>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </nav>

        {/* Sidebar footer — user info + collapse toggle */}
        <div
          className={cn(
            "flex shrink-0 flex-col gap-2 border-t border-default-100 p-3",
          )}
        >
          {/* User info row */}
          <Tooltip content={username ?? 'admin'} placement="right" isDisabled={!collapsed}>
            <div className={cn(
              "flex items-center gap-2.5 rounded-xl px-2 py-1.5",
              collapsed && "justify-center px-0",
            )}>
              <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary/15 text-[11px] font-semibold text-primary">
                {(username ?? 'A').charAt(0).toUpperCase()}
              </div>
              <AnimatePresence initial={false}>
                {!collapsed && (
                  <motion.div
                    key="footer-user"
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1, transition: { duration: 0.1, delay: 0.08 } }}
                    exit={{ opacity: 0, transition: { duration: 0.05 } }}
                    className="min-w-0 flex-1"
                  >
                    <p className="truncate text-xs font-medium text-foreground">{username ?? 'admin'}</p>
                    <div className="flex items-center gap-1.5">
                      <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-success" />
                      <span className="text-[10px] text-default-400">
                        {t("nav.online", { defaultValue: "Online" })}
                      </span>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </Tooltip>

          {/* Collapse toggle */}
          <Tooltip
            content={
              collapsed
                ? t("common.expandSidebar", { defaultValue: "Expand sidebar" })
                : t("common.collapseSidebar", { defaultValue: "Collapse sidebar" })
            }
            placement="right"
          >
            <Button
              isIconOnly
              variant="flat"
              className="hidden w-full md:flex bg-default-100 hover:bg-default-200"
              size="sm"
              aria-label={collapsed
                ? t("common.expandSidebar", { defaultValue: "Expand sidebar" })
                : t("common.collapseSidebar", { defaultValue: "Collapse sidebar" })}
              onPress={() => setCollapsed(!collapsed)}
            >
              {collapsed ? (
                <ChevronRight className="h-4 w-4 text-default-600" />
              ) : (
                <ChevronLeft className="h-4 w-4 text-default-600" />
              )}
            </Button>
          </Tooltip>
        </div>
      </aside>

      {/* Main content area */}
      <main className="flex flex-1 flex-col overflow-hidden relative">
        {/* Top bar */}
        <header className="sticky top-0 z-40 shrink-0 border-b border-default-200/70 bg-content1/82 backdrop-blur-xl">
          <div className="flex h-16 items-center justify-between px-4 sm:px-6">
            <div className="flex min-w-0 items-center gap-3">
              <div className="md:hidden">
                <Button
                  isIconOnly
                  variant="light"
                  size="sm"
                  aria-label={t("common.openMenu", { defaultValue: "Open menu" })}
                  onPress={() => {
                    setCollapsed(false);
                    setMobileOpen(true);
                  }}
                >
                  <Menu className="h-5 w-5" />
                </Button>
              </div>

              <div className="min-w-0">
                {activeNavigationContext?.groupLabel ? (
                  <p className="truncate text-xs font-semibold uppercase tracking-[0.18em] text-default-400">
                    {activeNavigationContext.groupLabel}
                  </p>
                ) : null}
                <AnimatePresence initial={false} mode="popLayout">
                  <motion.div
                    key={
                      showDockedPageActions ? "header-docked" : "header-inline"
                    }
                    initial={{ opacity: 0, y: -6 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -6 }}
                    transition={{ duration: 0.16, ease: [0.16, 1, 0.3, 1] }}
                    className="min-w-0"
                  >
                    <p
                      className={cn(
                        "truncate font-semibold tracking-[-0.025em] text-foreground",
                        showDockedPageActions
                          ? "text-base sm:text-lg"
                          : "text-sm sm:text-[15px]",
                      )}
                    >
                      {compactHeaderTitle}
                    </p>
                  </motion.div>
                </AnimatePresence>
              </div>
            </div>

            <div className="ml-auto flex items-center gap-2">
              {showDockedPageActions && pageHeader?.actions ? (
                <div className="hidden items-center gap-2 lg:flex">
                  {pageHeader.actions}
                </div>
              ) : null}
              <LanguageToggle />
              <ThemeToggleButton />
              <Button
                variant="flat"
                size="sm"
                color="danger"
                startContent={<LogOut className="h-4 w-4" />}
                onPress={onLogout}
              >
                {t("common.logout", { defaultValue: "Sign Out" })}
              </Button>
            </div>
          </div>

          {pageHeader && pageHeader?.mode !== "dock-on-scroll" ? (
            <div className="border-t border-default-200/70 px-4 py-3 sm:px-6">
              <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
                <div className="min-w-0 space-y-1">
                  <h1 className="min-w-0 text-pretty text-lg font-semibold tracking-[-0.02em] text-foreground sm:text-xl lg:text-2xl">
                    {pageHeader?.title}
                  </h1>
                  {pageHeader?.description ? (
                    <p className="max-w-[72ch] text-sm leading-6 text-default-600 line-clamp-2 sm:line-clamp-none">
                      {pageHeader?.description}
                    </p>
                  ) : null}
                </div>
                {pageHeader?.actions ? (
                  <div className="flex shrink-0 flex-wrap items-center gap-2 lg:justify-end">
                    {pageHeader?.actions}
                  </div>
                ) : null}
              </div>
            </div>
          ) : null}
        </header>

        {/* Page content */}
        <div className="flex-1 overflow-auto" data-app-scroll-root style={{ viewTransitionName: 'main-content' }}>
          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={location.pathname}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.18, ease: [0.25, 0.46, 0.45, 0.94] }}
              className="h-full"
            >
              <Outlet context={{ setPageHeader, setPageHeaderBodyVisible }} />
            </motion.div>
          </AnimatePresence>
        </div>
      </main>
    </div>
  );
}
