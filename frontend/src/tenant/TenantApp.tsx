import { Suspense, lazy, useCallback, useEffect, useMemo, useState } from 'react'
import type { FormEvent } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Button } from '@heroui/react'
import { isAxiosError } from 'axios'
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { AnimatePresence, motion } from 'framer-motion'
import {
  Activity,
  KeyRound,
  LayoutDashboard,
  ReceiptText,
  TerminalSquare,
} from 'lucide-react'

import AppLayout, { type AppLayoutMenuGroup } from '@/components/layout/AppLayout'
import { BrandStage, PagePanel } from '@/components/layout/page-archetypes'
import { LanguageToggle } from '@/components/LanguageToggle'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import { SurfaceInset } from '@/components/ui/surface'
import { tenantAuthApi } from '@/api/tenantAuth'
import type { SystemCapabilitiesResponse } from '@/api/types'
import { localizeApiErrorDisplay } from '@/api/errorI18n'
import {
  TENANT_AUTH_REQUIRED_EVENT,
  TENANT_LOGIN_FAILED_EVENT,
} from '@/api/tenantClient'
import { notify } from '@/lib/notification'
import { clearTenantAccessToken, setTenantAccessToken } from '@/lib/tenant-session'

const TenantDashboardPage = lazy(() =>
  import('@/tenant/pages/DashboardPage').then((module) => ({
    default: module.TenantDashboardPage,
  })),
)
const TenantUsagePage = lazy(() =>
  import('@/tenant/pages/UsagePage').then((module) => ({
    default: module.TenantUsagePage,
  })),
)
const TenantBillingPage = lazy(() =>
  import('@/tenant/pages/BillingPage').then((module) => ({
    default: module.TenantBillingPage,
  })),
)
const TenantLogsPage = lazy(() =>
  import('@/tenant/pages/LogsPage').then((module) => ({
    default: module.TenantLogsPage,
  })),
)
const TenantApiKeysPage = lazy(() =>
  import('@/tenant/pages/ApiKeysPage').then((module) => ({
    default: module.TenantApiKeysPage,
  })),
)

type AuthMode = 'login' | 'register'
type AuthScreen = 'auth' | 'verify' | 'forgot'
type ForgotStep = 'request' | 'reset'

const LABEL_CLASS_NAME = 'text-xs font-medium text-muted-foreground'
const CARD_CLASS_NAME = 'w-full space-y-6'
const INPUT_CLASS_NAME = 'sm:min-h-11'
const AUTH_PANEL_CLASS_NAME =
  'mx-auto w-full max-w-2xl p-5 md:p-6'

interface TenantAppProps {
  capabilities: SystemCapabilitiesResponse
}

export function TenantApp({ capabilities }: TenantAppProps) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const allowTenantSelfService = capabilities.features.tenant_self_service
  const [authChecked, setAuthChecked] = useState(false)
  const [authenticated, setAuthenticated] = useState(false)
  const [authMode, setAuthMode] = useState<AuthMode>('login')
  const [authScreen, setAuthScreen] = useState<AuthScreen>('auth')
  const [forgotStep, setForgotStep] = useState<ForgotStep>('request')

  const [registerForm, setRegisterForm] = useState({
    tenant_name: '',
    email: '',
    password: '',
  })
  const [registerConfirmPassword, setRegisterConfirmPassword] = useState('')
  const [verifyForm, setVerifyForm] = useState({ email: '', code: '' })
  const [loginForm, setLoginForm] = useState({ email: '', password: '' })
  const [forgotForm, setForgotForm] = useState({ email: '' })
  const [resetForm, setResetForm] = useState({
    email: '',
    code: '',
    new_password: '',
  })

  const tenantBrandPoints = useMemo(
    () => [
      t('tenantApp.auth.brand.points.audit'),
      t('tenantApp.auth.brand.points.security'),
      t('tenantApp.auth.brand.points.resilience'),
    ],
    [t],
  )

  const notifySuccess = useCallback((title: string) => {
    notify({
      variant: 'success',
      title,
    })
  }, [])

  const notifyError = useCallback((title: string, description?: string) => {
    notify({
      variant: 'error',
      title,
      description,
    })
  }, [])

  const openAuthScreen = (mode: AuthMode = 'login') => {
    setAuthScreen('auth')
    setAuthMode(allowTenantSelfService ? mode : 'login')
    setForgotStep('request')
  }

  useEffect(() => {
    let cancelled = false
    tenantAuthApi
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
    const onAuthRequired = () => {
      clearTenantAccessToken()
      queryClient.clear()
      setAuthenticated(false)
      setAuthChecked(true)
      setAuthScreen('auth')
      setAuthMode('login')
      setForgotStep('request')
      notify({
        variant: 'warning',
        title: t('tenantApp.auth.notice.sessionExpired'),
      })
    }
    const onLoginFailed = () => {
      notifyError(t('tenantApp.auth.error.invalidCredentialsOrUnverified'))
    }
    window.addEventListener(TENANT_AUTH_REQUIRED_EVENT, onAuthRequired)
    window.addEventListener(TENANT_LOGIN_FAILED_EVENT, onLoginFailed)
    return () => {
      window.removeEventListener(TENANT_AUTH_REQUIRED_EVENT, onAuthRequired)
      window.removeEventListener(TENANT_LOGIN_FAILED_EVENT, onLoginFailed)
    }
  }, [notifyError, queryClient, t])

  const loginMutation = useMutation({
    mutationFn: async () => tenantAuthApi.login(loginForm.email, loginForm.password),
    onSuccess: (response) => {
      setTenantAccessToken(response.access_token)
      setAuthenticated(true)
      setAuthChecked(true)
      notifySuccess(t('tenantApp.auth.notice.loginSuccess'))
    },
    onError: (err) => {
      if (isAxiosError(err) && err.response?.status === 401) {
        return
      }
      const fallback = t('tenantApp.auth.error.loginFailed')
      const description = localizeApiErrorDisplay(t, err, t('tenantApp.auth.error.loginFailed')).label
      notifyError(fallback, description !== fallback ? description : undefined)
    },
  })

  const registerMutation = useMutation({
    mutationFn: async () => tenantAuthApi.register(registerForm),
    onSuccess: () => {
      setVerifyForm((prev) => ({ ...prev, email: registerForm.email }))
      setAuthScreen('verify')
      setAuthMode('login')
      setRegisterConfirmPassword('')
      notifySuccess(t('tenantApp.auth.notice.registerSuccess'))
    },
    onError: (err) => {
      const fallback = t('tenantApp.auth.error.registerFailed')
      const description = localizeApiErrorDisplay(t, err, t('tenantApp.auth.error.registerFailed')).label
      notifyError(fallback, description !== fallback ? description : undefined)
    },
  })

  const verifyMutation = useMutation({
    mutationFn: async () => tenantAuthApi.verifyEmail(verifyForm.email, verifyForm.code),
    onSuccess: () => {
      setAuthScreen('auth')
      setAuthMode('login')
      notifySuccess(t('tenantApp.auth.notice.emailVerified'))
    },
    onError: (err) => {
      const fallback = t('tenantApp.auth.error.verificationFailed')
      const description = localizeApiErrorDisplay(t, err, t('tenantApp.auth.error.verificationFailed')).label
      notifyError(fallback, description !== fallback ? description : undefined)
    },
  })

  const forgotMutation = useMutation({
    mutationFn: async () => tenantAuthApi.forgotPassword(forgotForm.email),
    onSuccess: () => {
      setResetForm((prev) => ({ ...prev, email: forgotForm.email }))
      setForgotStep('reset')
      notifySuccess(t('tenantApp.auth.notice.resetCodeSentIfExists'))
    },
    onError: (err) => {
      const fallback = t('tenantApp.auth.error.sendResetCodeFailed')
      const description = localizeApiErrorDisplay(t, err, t('tenantApp.auth.error.sendResetCodeFailed')).label
      notifyError(fallback, description !== fallback ? description : undefined)
    },
  })

  const resetMutation = useMutation({
    mutationFn: async () =>
      tenantAuthApi.resetPassword(resetForm.email, resetForm.code, resetForm.new_password),
    onSuccess: () => {
      setForgotStep('request')
      setAuthScreen('auth')
      setAuthMode('login')
      notifySuccess(t('tenantApp.auth.notice.passwordResetSuccess'))
    },
    onError: (err) => {
      const fallback = t('tenantApp.auth.error.passwordResetFailed')
      const description = localizeApiErrorDisplay(t, err, t('tenantApp.auth.error.passwordResetFailed')).label
      notifyError(fallback, description !== fallback ? description : undefined)
    },
  })

  const logoutMutation = useMutation({
    mutationFn: async () => tenantAuthApi.logout(),
    onSettled: () => {
      clearTenantAccessToken()
      queryClient.clear()
      setAuthenticated(false)
      setAuthChecked(true)
      setAuthScreen('auth')
      setAuthMode('login')
      setForgotStep('request')
    },
  })

  const handleLogout = async () => {
    await logoutMutation.mutateAsync()
  }

  const tenantMenuGroups = useMemo<AppLayoutMenuGroup[]>(
    () => [
      {
        label: t('tenantApp.menu.analytics'),
        items: [
          {
            path: '/dashboard',
            icon: LayoutDashboard,
            label: t('tenantApp.menu.dashboard'),
          },
          {
            path: '/usage',
            icon: Activity,
            label: t('tenantApp.menu.usage'),
          },
          {
            path: '/billing',
            icon: ReceiptText,
            label: t('tenantApp.menu.billing'),
          },
          {
            path: '/logs',
            icon: TerminalSquare,
            label: t('tenantApp.menu.logs'),
          },
        ],
      },
      {
        label: t('tenantApp.menu.assets'),
        items: [
          {
            path: '/api-keys',
            icon: KeyRound,
            label: t('tenantApp.menu.apiKeys'),
          },
        ],
      },
    ],
    [t],
  )

  const handleLoginSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    loginMutation.mutate()
  }

  const handleRegisterSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (registerForm.password !== registerConfirmPassword) {
      notifyError(t('tenantApp.auth.error.passwordMismatch'))
      return
    }
    registerMutation.mutate()
  }

  const handleVerifySubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    verifyMutation.mutate()
  }

  const handleForgotRequestSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    forgotMutation.mutate()
  }

  const handleResetSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    resetMutation.mutate()
  }

  const openForgotPassword = () => {
    setForgotForm({ email: loginForm.email })
    setResetForm((prev) => ({ ...prev, email: loginForm.email }))
    setForgotStep('request')
    setAuthScreen('forgot')
  }

  const authCard = (
    <div className={CARD_CLASS_NAME}>
      <div className="space-y-2">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
          {t('tenantApp.auth.brand.badge')}
        </p>
        <h2 className="text-xl font-semibold text-foreground sm:text-3xl">
          {authMode === 'login' || !allowTenantSelfService
            ? t('tenantApp.auth.sections.loginTitle')
            : t('tenantApp.auth.sections.registerTitle')}
        </h2>
        <p className="text-sm text-default-600">
          {t('tenantApp.auth.sections.authSubtitle')}
        </p>
      </div>

      {allowTenantSelfService ? (
        <div className="mt-4 grid grid-cols-2 gap-2 rounded-large bg-content2 p-1 sm:mt-6">
          <Button
            type="button"
            color={authMode === 'login' ? 'primary' : 'default'}
            variant={authMode === 'login' ? 'solid' : 'light'}
            className="h-11"
            onClick={() => {
              setAuthMode('login')
            }}
          >
            {t('tenantApp.auth.tabs.login')}
          </Button>
          <Button
            type="button"
            color={authMode === 'register' ? 'primary' : 'default'}
            variant={authMode === 'register' ? 'solid' : 'light'}
            className="h-11"
            onClick={() => {
              setAuthMode('register')
            }}
          >
            {t('tenantApp.auth.tabs.register')}
          </Button>
        </div>
      ) : null}

      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={allowTenantSelfService ? authMode : 'login'}
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          transition={{ duration: 0.2, ease: 'easeOut' }}
          className="mt-4 sm:mt-6"
        >
          {authMode === 'login' || !allowTenantSelfService ? (
            <form className="space-y-3.5 sm:space-y-4" onSubmit={handleLoginSubmit}>
              <div className="space-y-2">
                <label htmlFor="tenant-login-email" className={LABEL_CLASS_NAME}>
                  {t('tenantApp.auth.fields.email')}
                </label>
                <Input
                  id="tenant-login-email"
                  name="email"
                  type="email"
                  inputMode="email"
                  value={loginForm.email}
                  autoComplete="email"
                  spellCheck={false}
                  onChange={(e) => setLoginForm((prev) => ({ ...prev, email: e.target.value }))}
                  placeholder={t('tenantApp.auth.placeholders.email')}
                  className={INPUT_CLASS_NAME}
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="tenant-login-password" className={LABEL_CLASS_NAME}>
                  {t('tenantApp.auth.fields.password')}
                </label>
                <Input
                  id="tenant-login-password"
                  name="password"
                  type="password"
                  value={loginForm.password}
                  autoComplete="current-password"
                  onChange={(e) => setLoginForm((prev) => ({ ...prev, password: e.target.value }))}
                  placeholder={t('tenantApp.auth.placeholders.password')}
                  className={INPUT_CLASS_NAME}
                />
              </div>

              {allowTenantSelfService ? (
                <Button
                  type="button"
                  variant="light"
                  className="h-11 px-1"
                  onClick={openForgotPassword}
                >
                  {t('tenantApp.auth.actions.openForgot')}
                </Button>
              ) : null}

              <Button
                color="primary"
                type="submit"
                disabled={loginMutation.isPending}
                className="h-11 w-full"
              >
                {t('tenantApp.auth.actions.login')}
              </Button>
            </form>
          ) : (
            <form className="space-y-3.5 sm:space-y-4" onSubmit={handleRegisterSubmit}>
              <div className="space-y-2">
                <label htmlFor="tenant-register-name" className={LABEL_CLASS_NAME}>
                  {t('tenantApp.auth.fields.tenantName')}
                </label>
                <Input
                  id="tenant-register-name"
                  name="tenant_name"
                  value={registerForm.tenant_name}
                  autoComplete="organization"
                  onChange={(e) =>
                    setRegisterForm((prev) => ({ ...prev, tenant_name: e.target.value }))
                  }
                  placeholder={t('tenantApp.auth.placeholders.tenantName')}
                  className={INPUT_CLASS_NAME}
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="tenant-register-email" className={LABEL_CLASS_NAME}>
                  {t('tenantApp.auth.fields.email')}
                </label>
                <Input
                  id="tenant-register-email"
                  name="email"
                  type="email"
                  inputMode="email"
                  value={registerForm.email}
                  autoComplete="email"
                  spellCheck={false}
                  onChange={(e) => setRegisterForm((prev) => ({ ...prev, email: e.target.value }))}
                  placeholder={t('tenantApp.auth.placeholders.email')}
                  className={INPUT_CLASS_NAME}
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="tenant-register-password" className={LABEL_CLASS_NAME}>
                  {t('tenantApp.auth.fields.passwordMin8')}
                </label>
                <Input
                  id="tenant-register-password"
                  name="password"
                  type="password"
                  value={registerForm.password}
                  autoComplete="new-password"
                  onChange={(e) =>
                    setRegisterForm((prev) => ({ ...prev, password: e.target.value }))
                  }
                  placeholder={t('tenantApp.auth.placeholders.password')}
                  className={INPUT_CLASS_NAME}
                />
              </div>
              <div className="space-y-2">
                <label htmlFor="tenant-register-password-confirm" className={LABEL_CLASS_NAME}>
                  {t('tenantApp.auth.fields.confirmPassword')}
                </label>
                <Input
                  id="tenant-register-password-confirm"
                  name="confirm_password"
                  type="password"
                  value={registerConfirmPassword}
                  autoComplete="new-password"
                  onChange={(e) => setRegisterConfirmPassword(e.target.value)}
                  placeholder={t('tenantApp.auth.placeholders.confirmPassword')}
                  className={INPUT_CLASS_NAME}
                />
              </div>

              <Button
                color="primary"
                type="submit"
                disabled={registerMutation.isPending}
                className="h-11 w-full"
              >
                {t('tenantApp.auth.actions.register')}
              </Button>
            </form>
          )}
        </motion.div>
      </AnimatePresence>

      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.08, duration: 0.22 }}
        className="mt-4 hidden sm:block sm:mt-6"
      >
        <div className="space-y-3">
          <p className="text-center text-xs text-default-500">
            {t('tenantApp.auth.social.comingSoon')}
          </p>
          <div className="grid grid-cols-2 gap-2">
            <Button
              type="button"
              disabled
              className="h-10"
            >
              {t('tenantApp.auth.social.google')}
            </Button>
            <Button
              type="button"
              disabled
              className="h-10"
            >
              {t('tenantApp.auth.social.github')}
            </Button>
          </div>
        </div>
      </motion.div>

      {allowTenantSelfService ? (
        <div className="mt-4 flex justify-center sm:mt-5">
          {authMode === 'login' ? (
            <Button
              type="button"
              variant="light"
              className="h-11 px-2"
              onClick={() => {
                setAuthMode('register')
              }}
            >
              {t('tenantApp.auth.actions.switchToRegister')}
            </Button>
          ) : (
            <Button
              type="button"
              variant="light"
              className="h-11 px-2"
              onClick={() => {
                setAuthMode('login')
              }}
            >
              {t('tenantApp.auth.actions.switchToLogin')}
            </Button>
          )}
        </div>
      ) : null}
    </div>
  )

  const verifyCard = (
    <div className={CARD_CLASS_NAME}>
      <div className="space-y-2">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
          {t('tenantApp.auth.brand.badge')}
        </p>
        <h2 className="text-xl font-semibold text-foreground sm:text-3xl">
          {t('tenantApp.auth.sections.verifyEmailTitle')}
        </h2>
        <p className="text-sm text-default-600">
          {t('tenantApp.auth.sections.verifyEmailSubtitle')}
        </p>
      </div>

      <form className="mt-4 space-y-3.5 sm:mt-6 sm:space-y-4" onSubmit={handleVerifySubmit}>
        <div className="space-y-2">
          <label htmlFor="tenant-verify-email" className={LABEL_CLASS_NAME}>
            {t('tenantApp.auth.fields.email')}
          </label>
          <Input
            id="tenant-verify-email"
            name="email"
            type="email"
            inputMode="email"
            value={verifyForm.email}
            autoComplete="email"
            spellCheck={false}
            onChange={(e) => setVerifyForm((prev) => ({ ...prev, email: e.target.value }))}
            placeholder={t('tenantApp.auth.placeholders.email')}
            className={INPUT_CLASS_NAME}
          />
        </div>
        <div className="space-y-2">
          <label htmlFor="tenant-verify-code" className={LABEL_CLASS_NAME}>
            {t('tenantApp.auth.fields.verificationCode')}
          </label>
          <Input
            id="tenant-verify-code"
            name="code"
            value={verifyForm.code}
            autoComplete="one-time-code"
            spellCheck={false}
            onChange={(e) => setVerifyForm((prev) => ({ ...prev, code: e.target.value }))}
            placeholder={t('tenantApp.auth.placeholders.verificationCode')}
            className={INPUT_CLASS_NAME}
          />
        </div>
        <Button
          color="primary"
          type="submit"
          disabled={verifyMutation.isPending}
          className="h-11 w-full"
        >
          {t('tenantApp.auth.actions.verifyEmail')}
        </Button>
      </form>

      <div className="mt-4 flex flex-col gap-3 sm:mt-5 sm:flex-row sm:items-center sm:justify-between">
        <span className="text-xs text-default-500">
          {t('tenantApp.auth.notice.verifyCodeHint')}
        </span>
        <Button
          type="button"
          variant="light"
          className="h-11 px-2"
          onClick={() => openAuthScreen('login')}
        >
          {t('tenantApp.auth.actions.backToLogin')}
        </Button>
      </div>
    </div>
  )

  const forgotCard = (
    <div className={CARD_CLASS_NAME}>
      <div className="space-y-2">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
          {t('tenantApp.auth.brand.badge')}
        </p>
        <h2 className="text-xl font-semibold text-foreground sm:text-3xl">
          {t('tenantApp.auth.sections.forgotPasswordTitle')}
        </h2>
        <p className="text-sm text-default-600">
          {t('tenantApp.auth.sections.forgotPasswordSubtitle')}
        </p>
      </div>

      <div className="mt-4 flex flex-wrap gap-2 sm:mt-6">
        <Badge variant={forgotStep === 'request' ? 'default' : 'secondary'}>
          {t('tenantApp.auth.forgot.stepSendCode')}
        </Badge>
        <Badge variant={forgotStep === 'reset' ? 'default' : 'secondary'}>
          {t('tenantApp.auth.forgot.stepResetPassword')}
        </Badge>
      </div>

      <div className="mt-4 space-y-3.5 sm:mt-6 sm:space-y-4">
        <form className="space-y-3.5 sm:space-y-4" onSubmit={handleForgotRequestSubmit}>
          <div className="space-y-2">
            <label htmlFor="tenant-forgot-email" className={LABEL_CLASS_NAME}>
              {t('tenantApp.auth.fields.email')}
            </label>
            <Input
              id="tenant-forgot-email"
              name="email"
              type="email"
              inputMode="email"
              value={forgotForm.email}
              autoComplete="email"
              spellCheck={false}
              onChange={(e) => setForgotForm({ email: e.target.value })}
              placeholder={t('tenantApp.auth.placeholders.email')}
              className={INPUT_CLASS_NAME}
            />
          </div>
          <Button
            color="primary"
            type="submit"
            disabled={forgotMutation.isPending}
            className="h-11 w-full"
          >
            {t('tenantApp.auth.actions.sendResetCode')}
          </Button>
        </form>

        {forgotStep === 'reset' ? (
          <motion.div
            key="forgot-reset-step"
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.2, ease: 'easeOut' }}
          >
            <SurfaceInset className="space-y-3.5 sm:space-y-4">
              <form className="space-y-3.5 sm:space-y-4" onSubmit={handleResetSubmit}>
                <div className="space-y-2">
                  <label htmlFor="tenant-reset-code" className={LABEL_CLASS_NAME}>
                    {t('tenantApp.auth.fields.resetCode')}
                  </label>
                  <Input
                    id="tenant-reset-code"
                    name="code"
                    value={resetForm.code}
                    autoComplete="one-time-code"
                    spellCheck={false}
                    onChange={(e) => setResetForm((prev) => ({ ...prev, code: e.target.value }))}
                    placeholder={t('tenantApp.auth.placeholders.resetCode')}
                    className={INPUT_CLASS_NAME}
                  />
                </div>
                <div className="space-y-2">
                  <label htmlFor="tenant-reset-password" className={LABEL_CLASS_NAME}>
                    {t('tenantApp.auth.fields.newPassword')}
                  </label>
                  <Input
                    id="tenant-reset-password"
                    name="new_password"
                    type="password"
                    value={resetForm.new_password}
                    autoComplete="new-password"
                    onChange={(e) =>
                      setResetForm((prev) => ({
                        ...prev,
                        new_password: e.target.value,
                      }))
                    }
                    placeholder={t('tenantApp.auth.placeholders.newPassword')}
                    className={INPUT_CLASS_NAME}
                  />
                </div>
                <Button
                  color="primary"
                  type="submit"
                  disabled={resetMutation.isPending}
                  className="h-11 w-full"
                >
                  {t('tenantApp.auth.actions.resetPassword')}
                </Button>
              </form>
            </SurfaceInset>
          </motion.div>
        ) : (
          <SurfaceInset className="px-4 py-3 text-xs text-default-500">
            {t('tenantApp.auth.forgot.drawerHint')}
          </SurfaceInset>
        )}
      </div>

      <div className="mt-4 flex justify-end sm:mt-5">
        <Button
          type="button"
          variant="light"
          className="h-11 px-2"
          onClick={() => openAuthScreen('login')}
        >
          {t('tenantApp.auth.actions.backToLogin')}
        </Button>
      </div>
    </div>
  )

  if (!authChecked) {
    return (
      <div className="min-h-screen p-8 text-sm text-default-600">
        {t('tenantApp.loadingPortal')}
      </div>
    )
  }

  if (!authenticated) {
    return (
      <div className="relative min-h-screen overflow-hidden bg-background px-4 py-8 sm:px-6 lg:px-8">
        <div className="absolute right-4 top-4 z-20">
          <LanguageToggle />
        </div>
        <div className="relative z-10 mx-auto flex min-h-[calc(100vh-4rem)] w-full max-w-6xl items-center">
          <div className="grid w-full gap-5 lg:grid-cols-[minmax(0,1.05fr)_minmax(0,0.95fr)] lg:gap-8">
            <BrandStage
              badge={t('tenantApp.auth.brand.badge')}
              title={t('tenantApp.auth.brand.title')}
              subtitle={t('tenantApp.auth.brand.subtitle')}
              points={tenantBrandPoints}
              className="hidden lg:block"
            />
            <div className="space-y-4">
              <div className="lg:hidden">
                <BrandStage
                  badge={t('tenantApp.auth.brand.badge')}
                  title={t('tenantApp.auth.brand.title')}
                  subtitle={t('tenantApp.auth.brand.subtitle')}
                  points={tenantBrandPoints}
                  className="p-5"
                />
              </div>
              <PagePanel tone="primary" className={AUTH_PANEL_CLASS_NAME}>
                <AnimatePresence mode="wait" initial={false}>
                  <motion.div
                    key={`${authScreen}:${authMode}:${forgotStep}`}
                    initial={{ opacity: 0, y: 16 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -10 }}
                    transition={{ duration: 0.24, ease: 'easeOut' }}
                  >
                    {authScreen === 'auth'
                      ? authCard
                      : authScreen === 'verify'
                        ? verifyCard
                        : forgotCard}
                  </motion.div>
                </AnimatePresence>
              </PagePanel>
            </div>
          </div>
        </div>
      </div>
    )
  }

  const routeFallback = (
    <div className="p-8 text-sm text-default-600">
      {t('common.loading')}
    </div>
  )

  return (
    <BrowserRouter basename="/tenant">
      <Routes>
        <Route
          element={
            <AppLayout
              onLogout={handleLogout}
              capabilities={capabilities}
              menuGroups={tenantMenuGroups}
            />
          }
        >
          <Route path="/" element={<Navigate to="/dashboard" replace />} />
          <Route
            path="/dashboard"
            element={(
              <Suspense fallback={routeFallback}>
                <TenantDashboardPage />
              </Suspense>
            )}
          />
          <Route
            path="/usage"
            element={(
              <Suspense fallback={routeFallback}>
                <TenantUsagePage />
              </Suspense>
            )}
          />
          <Route
            path="/billing"
            element={(
              <Suspense fallback={routeFallback}>
                <TenantBillingPage />
              </Suspense>
            )}
          />
          <Route
            path="/logs"
            element={(
              <Suspense fallback={routeFallback}>
                <TenantLogsPage />
              </Suspense>
            )}
          />
          <Route
            path="/api-keys"
            element={(
              <Suspense fallback={routeFallback}>
                <TenantApiKeysPage />
              </Suspense>
            )}
          />
          <Route path="*" element={<Navigate to="/dashboard" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}
