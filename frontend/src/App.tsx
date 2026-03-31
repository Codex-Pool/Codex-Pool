import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { HeroUIProvider } from '@heroui/react'
import { lazy, Suspense, useEffect, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'

import {
  AUTH_REQUIRED_EVENT,
  LOGIN_FAILED_EVENT,
  SESSION_EXPIRED_REASON,
} from '@/api/client'
import { adminAuthApi } from '@/api/adminAuth'
import { adminTenantsApi } from '@/api/adminTenants'
import { systemApi, DEFAULT_SYSTEM_CAPABILITIES } from '@/api/system'
import { ThemeProvider } from '@/components/theme-provider'
import { UiPreferencesProvider } from '@/components/ui-preferences-provider'
import { LoadingScreen } from '@/components/ui/loading-overlay'
import { NotificationCenter } from '@/components/ui/notification-center'
import { resolveAppShellTarget } from '@/lib/edition-shell-routing'
import { notify } from '@/lib/notification'
import { clearAdminAccessToken, setAdminAccessToken } from '@/lib/admin-session'
import {
  LEGACY_STANDALONE_ADMIN_API_KEYS_PATH,
  resolveAdminCapabilityRedirect,
  STANDALONE_ADMIN_API_KEYS_PATH,
} from '@/features/api-keys/admin-capabilities'

const AppLayout = lazy(() => import('@/components/layout/AppLayout'))
const Login = lazy(() => import('@/pages/Login'))
const Dashboard = lazy(() => import('@/pages/Dashboard'))
const Accounts = lazy(() => import('@/pages/Accounts'))
const Inventory = lazy(() => import('@/pages/Inventory'))
const Models = lazy(() => import('@/pages/Models'))
const Logs = lazy(() => import('@/pages/Logs'))
const System = lazy(() => import('@/pages/System'))
const Billing = lazy(() => import('@/pages/Billing'))
const Groups = lazy(() => import('@/pages/Groups'))
const ImportJobs = lazy(() => import('@/pages/ImportJobs'))
const OAuthImport = lazy(() => import('@/pages/OAuthImport'))
const ModelRouting = lazy(() => import('@/pages/ModelRouting'))
const Proxies = lazy(() => import('@/pages/Proxies'))
const Config = lazy(() => import('@/pages/Config'))
const Tenants = lazy(() => import('@/pages/Tenants'))
const AdminApiKeys = lazy(() => import('@/pages/AdminApiKeys'))
const Usage = lazy(() => import('@/pages/Usage'))
const TenantApp = lazy(() =>
  import('@/tenant/TenantApp').then((module) => ({
    default: module.TenantApp,
  })),
)

function LoadingFallback() {
  const { t } = useTranslation()

  return (
    <LoadingScreen
      title={t('common.routeLoading', { defaultValue: 'Loading page...' })}
      description={t('common.loading', { defaultValue: 'Loading...' })}
      className="min-h-screen"
    />
  )
}

interface AdminAppProps {
  capabilities: typeof DEFAULT_SYSTEM_CAPABILITIES
}

function AdminApp({ capabilities }: AdminAppProps) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const [authChecked, setAuthChecked] = useState(false)
  const [authenticated, setAuthenticated] = useState(false)
  const [adminUsername, setAdminUsername] = useState<string | undefined>()

  useEffect(() => {
    let cancelled = false
    adminAuthApi
      .me()
      .then((me) => {
        if (!cancelled) {
          setAuthenticated(true)
          setAdminUsername(me.username)
        }
      })
      .catch(() => {
        if (!cancelled) {
          setAuthenticated(false)
        }
      })
      .finally(() => {
        if (!cancelled) {
          setAuthChecked(true)
        }
      })

    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    const onAuthRequired = (event: Event) => {
      const reason = (event as CustomEvent<{ reason?: string }>).detail?.reason
      if (reason === SESSION_EXPIRED_REASON) {
        notify({
          variant: 'warning',
          title: t('notifications.sessionExpired.title'),
          description: t('notifications.sessionExpired.description'),
        })
      }

      clearAdminAccessToken()
      queryClient.clear()
      setAuthenticated(false)
      setAuthChecked(true)
    }

    const onLoginFailed = () => {
      notify({
        variant: 'error',
        title: t('notifications.loginFailed.title'),
        description: t('login.messages.invalidCredentials'),
      })
    }

    window.addEventListener(AUTH_REQUIRED_EVENT, onAuthRequired)
    window.addEventListener(LOGIN_FAILED_EVENT, onLoginFailed)

    return () => {
      window.removeEventListener(AUTH_REQUIRED_EVENT, onAuthRequired)
      window.removeEventListener(LOGIN_FAILED_EVENT, onLoginFailed)
    }
  }, [queryClient, t])

  useEffect(() => {
    if (!authenticated || !capabilities.features.multi_tenant) {
      return
    }

    let cancelled = false
    adminTenantsApi
      .ensureDefaultTenant()
      .then(() => {
        if (!cancelled) {
          queryClient.invalidateQueries({ queryKey: ['adminTenants'] })
        }
      })
      .catch(() => {
        // best-effort warmup: avoid interrupting admin login flow
      })

    return () => {
      cancelled = true
    }
  }, [authenticated, capabilities.features.multi_tenant, queryClient])

  const handleLogin = async (username: string, password: string) => {
    const response = await adminAuthApi.login(username, password)
    setAdminAccessToken(response.access_token)
    setAuthenticated(true)
    setAuthChecked(true)
  }

  const handleLogout = async () => {
    try {
      await adminAuthApi.logout()
    } finally {
      clearAdminAccessToken()
      queryClient.clear()
      setAuthenticated(false)
      setAuthChecked(true)
    }
  }

  if (!authChecked) {
    return <LoadingFallback />
  }

  if (!authenticated) {
    return (
      <Suspense fallback={<LoadingFallback />}>
        <Login onLogin={handleLogin} />
      </Suspense>
    )
  }

  const tenantsRedirect = resolveAdminCapabilityRedirect('/tenants', capabilities)
  const adminApiKeysRedirect = resolveAdminCapabilityRedirect(
    STANDALONE_ADMIN_API_KEYS_PATH,
    capabilities,
  )

  return (
    <BrowserRouter>
      <Suspense fallback={<LoadingFallback />}>
        <Routes>
          <Route element={<AppLayout onLogout={handleLogout} capabilities={capabilities} username={adminUsername} />}>
            <Route path="/" element={<Navigate to="/dashboard" replace />} />
            <Route path="/dashboard" element={<Dashboard />} />
            <Route path="/accounts" element={<Accounts />} />
            <Route path="/inventory" element={<Inventory />} />
            <Route path="/models" element={<Models />} />
            <Route path="/logs" element={<Logs />} />
            <Route path="/system" element={<System />} />
            <Route path="/billing" element={<Billing />} />
            <Route path="/groups" element={<Groups />} />
            <Route path="/imports" element={<ImportJobs />} />
            <Route path="/oauth-import" element={<OAuthImport />} />
            <Route path="/model-routing" element={<ModelRouting />} />
            <Route path="/proxies" element={<Proxies />} />
            <Route path="/config" element={<Config />} />
            <Route
              path="/tenants"
              element={tenantsRedirect ? <Navigate to={tenantsRedirect} replace /> : <Tenants />}
            />
            <Route
              path={LEGACY_STANDALONE_ADMIN_API_KEYS_PATH}
              element={
                <Navigate
                  to={adminApiKeysRedirect ?? STANDALONE_ADMIN_API_KEYS_PATH}
                  replace
                />
              }
            />
            <Route
              path={STANDALONE_ADMIN_API_KEYS_PATH}
              element={
                adminApiKeysRedirect ? (
                  <Navigate to={adminApiKeysRedirect} replace />
                ) : (
                  <AdminApiKeys />
                )
              }
            />
            <Route path="/usage" element={<Usage />} />
            <Route path="*" element={<Navigate to="/dashboard" replace />} />
          </Route>
        </Routes>
      </Suspense>
    </BrowserRouter>
  )
}

function AppShell() {
  const pathname = typeof window !== 'undefined' ? window.location.pathname : '/'
  const capabilitiesQuery = useQuery({
    queryKey: ['systemCapabilities'],
    queryFn: systemApi.getCapabilities,
    staleTime: 60_000,
  })
  const capabilities = capabilitiesQuery.data ?? DEFAULT_SYSTEM_CAPABILITIES
  const shellTarget = resolveAppShellTarget(pathname, capabilitiesQuery.data)

  if (shellTarget === 'loading' || capabilitiesQuery.isLoading) {
    return <LoadingFallback />
  }

  if (shellTarget === 'tenant') {
    return (
      <Suspense fallback={<LoadingFallback />}>
        <TenantApp capabilities={capabilities} />
      </Suspense>
    )
  }

  return <AdminApp capabilities={capabilities} />
}

export default function App() {
  return (
    <ThemeProvider defaultTheme="system" storageKey="codex-ui-theme">
      <UiPreferencesProvider>
        <HeroUIProvider>
          <main className="bg-background text-foreground min-h-screen">
            <AppShell />
            <NotificationCenter />
          </main>
        </HeroUIProvider>
      </UiPreferencesProvider>
    </ThemeProvider>
  )
}
