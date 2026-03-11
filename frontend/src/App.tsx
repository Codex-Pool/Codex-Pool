import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom'
import { HeroUIProvider } from '@heroui/react'
import { AppLayout } from '@/components/layout/AppLayout'
import { ThemeProvider } from '@/components/theme-provider'
import { lazy, Suspense, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import {
  AUTH_REQUIRED_EVENT,
  LOGIN_FAILED_EVENT,
  SESSION_EXPIRED_REASON,
} from '@/api/client'
import { adminTenantsApi } from '@/api/adminTenants'
import { notify } from '@/lib/notification'
import { clearAdminAccessToken, setAdminAccessToken } from '@/lib/admin-session'
import { LoadingScreen } from '@/components/ui/loading-overlay'
import { NotificationCenter } from '@/components/ui/notification-center'
import { applyRouteSeo } from '@/lib/seo'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
})

import { adminAuthApi } from '@/api/adminAuth'

const Dashboard = lazy(() => import('@/pages/Dashboard'))
const Accounts = lazy(() => import('@/pages/Accounts'))
const ImportJobs = lazy(() => import('@/pages/ImportJobs'))
const OAuthImport = lazy(() => import('@/pages/OAuthImport'))
const OAuthProbe = lazy(() => import('@/pages/OAuthProbe'))
const Groups = lazy(() => import('@/pages/Groups'))
const ModelRouting = lazy(() => import('@/pages/ModelRouting'))
const Models = lazy(() => import('@/pages/Models'))
const Usage = lazy(() => import('@/pages/Usage'))
const Billing = lazy(() => import('@/pages/Billing'))
const Proxies = lazy(() => import('@/pages/Proxies'))
const Tenants = lazy(() => import('@/pages/Tenants'))
const Config = lazy(() => import('@/pages/Config'))
const Logs = lazy(() => import('@/pages/Logs'))
const System = lazy(() => import('@/pages/System'))
const Login = lazy(() => import('@/pages/Login'))
const TenantApp = lazy(() =>
  import('@/tenant/TenantApp').then((module) => ({ default: module.TenantApp })),
)

const RouteSkeleton = () => {
  const { t } = useTranslation()
  return (
    <LoadingScreen
      title={t('common.routeLoading')}
      description={t('common.loading')}
      className="flex-1 min-h-0"
    />
  )
}

const RouteSeoSync = () => {
  const location = useLocation()
  const { t, i18n } = useTranslation()

  useEffect(() => {
    applyRouteSeo(location.pathname, t)
  }, [location.pathname, t, i18n.resolvedLanguage])

  return null
}

function AdminApp() {
  const { t, i18n } = useTranslation()
  const [authChecked, setAuthChecked] = useState(false)
  const [authenticated, setAuthenticated] = useState(false)

  useEffect(() => {
    let cancelled = false
    adminAuthApi
      .me()
      .then(() => {
        if (!cancelled) {
          setAuthenticated(true)
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
  }, [t])

  useEffect(() => {
    if (!authenticated) {
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
  }, [authenticated])

  useEffect(() => {
    if (!authenticated) {
      applyRouteSeo('/login', t)
    }
  }, [authenticated, t, i18n.resolvedLanguage])

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
    return (
      <LoadingScreen
        title={t('common.routeLoading')}
        description={t('common.loading')}
        className="min-h-screen"
      />
    )
  }

  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider defaultTheme="system" storageKey="codex-ui-theme">
        <HeroUIProvider>
          <NotificationCenter />
          {!authenticated ? (
            <Suspense fallback={<RouteSkeleton />}>
              <Login onLogin={handleLogin} />
            </Suspense>
          ) : (
            <BrowserRouter>
              <RouteSeoSync />
              <Routes>
                <Route element={<AppLayout onLogout={handleLogout} role="admin" />}>
                  <Route path="/" element={<Navigate to="/dashboard" replace />} />
                  <Route path="/dashboard" element={<Suspense fallback={<RouteSkeleton />}><Dashboard /></Suspense>} />
                  <Route path="/accounts" element={<Suspense fallback={<RouteSkeleton />}><Accounts /></Suspense>} />
                  <Route path="/imports" element={<Suspense fallback={<RouteSkeleton />}><ImportJobs /></Suspense>} />
                  <Route path="/oauth-import" element={<Suspense fallback={<RouteSkeleton />}><OAuthImport /></Suspense>} />
                  <Route path="/oauth-probe" element={<Suspense fallback={<RouteSkeleton />}><OAuthProbe /></Suspense>} />
                  <Route path="/groups" element={<Suspense fallback={<RouteSkeleton />}><Groups /></Suspense>} />
                  <Route path="/model-routing" element={<Suspense fallback={<RouteSkeleton />}><ModelRouting /></Suspense>} />
                  <Route path="/ai-routing" element={<Navigate to="/model-routing" replace />} />
                  <Route path="/models" element={<Suspense fallback={<RouteSkeleton />}><Models /></Suspense>} />
                  <Route path="/usage" element={<Suspense fallback={<RouteSkeleton />}><Usage /></Suspense>} />
                  <Route path="/billing" element={<Suspense fallback={<RouteSkeleton />}><Billing /></Suspense>} />
                  <Route path="/proxies" element={<Suspense fallback={<RouteSkeleton />}><Proxies /></Suspense>} />
                  <Route path="/tenants" element={<Suspense fallback={<RouteSkeleton />}><Tenants /></Suspense>} />
                  <Route path="/config" element={<Suspense fallback={<RouteSkeleton />}><Config /></Suspense>} />
                  <Route path="/logs" element={<Suspense fallback={<RouteSkeleton />}><Logs /></Suspense>} />
                  <Route path="/system" element={<Suspense fallback={<RouteSkeleton />}><System /></Suspense>} />
                  <Route path="*" element={<Navigate to="/dashboard" replace />} />
                </Route>
              </Routes>
            </BrowserRouter>
          )}
        </HeroUIProvider>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

function App() {
  const { t, i18n } = useTranslation()
  const isTenantPortal =
    typeof window !== 'undefined' && window.location.pathname.startsWith('/tenant')

  useEffect(() => {
    if (isTenantPortal && typeof window !== 'undefined') {
      applyRouteSeo(window.location.pathname, t)
    }
  }, [isTenantPortal, t, i18n.resolvedLanguage])

  if (isTenantPortal) {
    return (
      <QueryClientProvider client={queryClient}>
        <ThemeProvider defaultTheme="system" storageKey="codex-ui-theme">
          <HeroUIProvider>
            <NotificationCenter />
            <Suspense fallback={<RouteSkeleton />}>
              <TenantApp />
            </Suspense>
          </HeroUIProvider>
        </ThemeProvider>
      </QueryClientProvider>
    )
  }

  return <AdminApp />
}

export default App
