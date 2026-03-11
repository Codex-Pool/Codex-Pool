import { useState } from 'react'
import { motion, useReducedMotion } from 'framer-motion'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  AlertCircle,
  Download,
  ExternalLink,
  FileJson,
  Loader2,
  RefreshCcw,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'

import {
  oauthProbeApi,
  type CodexOAuthProbeSessionResult,
} from '@/api/oauthProbe'
import type { CodexOAuthLoginSessionStatus } from '@/api/oauthImport'
import { localizeApiErrorDisplay } from '@/api/errorI18n'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { notify } from '@/lib/notification'
import { cn } from '@/lib/utils'

const DEFAULT_BASE_URL = 'https://chatgpt.com/backend-api/codex'

function isTerminalStatus(status?: CodexOAuthLoginSessionStatus): boolean {
  return status === 'completed' || status === 'failed' || status === 'expired'
}

function statusBadgeVariant(status?: CodexOAuthLoginSessionStatus) {
  if (status === 'completed') {
    return 'success'
  }
  if (status === 'failed' || status === 'expired') {
    return 'destructive'
  }
  if (status === 'exchanging' || status === 'importing') {
    return 'warning'
  }
  return 'secondary'
}

function downloadProbeResult(result: CodexOAuthProbeSessionResult, sessionId: string) {
  const blob = new Blob([JSON.stringify(result, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = `codex-oauth-probe-${sessionId}.json`
  anchor.click()
  URL.revokeObjectURL(url)
}

export default function OAuthProbe() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const prefersReducedMotion = useReducedMotion()

  const [baseUrl, setBaseUrl] = useState(DEFAULT_BASE_URL)
  const [sessionId, setSessionId] = useState<string | null>(null)
  const [manualRedirectUrl, setManualRedirectUrl] = useState('')

  const sessionQuery = useQuery({
    queryKey: ['codexOauthProbeSession', sessionId],
    queryFn: () => oauthProbeApi.getCodexProbeSession(sessionId!),
    enabled: Boolean(sessionId),
    refetchInterval: (query) => {
      const data = query.state.data
      if (!data) {
        return 2000
      }
      return isTerminalStatus(data.status) ? false : 2000
    },
  })

  const session = sessionQuery.data

  function openAuthorizeTab(authorizeUrl: string) {
    const tab = window.open(authorizeUrl, '_blank', 'noopener,noreferrer')
    if (!tab) {
      notify({
        variant: 'warning',
        title: t('oauthProbe.notifications.popupBlockedTitle'),
        description: t('oauthProbe.notifications.popupBlockedDescription'),
      })
    }
  }

  const createSessionMutation = useMutation({
    mutationFn: async () =>
      oauthProbeApi.createCodexProbeSession({
        base_url: baseUrl.trim() || undefined,
      }),
    onSuccess: (created) => {
      setSessionId(created.session_id)
      setManualRedirectUrl('')
      queryClient.setQueryData(['codexOauthProbeSession', created.session_id], created)
      openAuthorizeTab(created.authorize_url)
      notify({
        variant: 'info',
        title: t('oauthProbe.notifications.sessionCreatedTitle'),
        description: t('oauthProbe.notifications.sessionCreatedDescription'),
      })
    },
    onError: (error: unknown) => {
      notify({
        variant: 'error',
        title: t('oauthProbe.notifications.sessionCreateFailedTitle'),
        description: localizeApiErrorDisplay(t, error, t('oauthProbe.notifications.unknownError')).label,
      })
    },
  })

  const submitManualCallbackMutation = useMutation({
    mutationFn: async () => {
      if (!sessionId) {
        throw new Error('session id is missing')
      }
      return oauthProbeApi.submitCodexProbeCallback(sessionId, manualRedirectUrl.trim())
    },
    onSuccess: (updated) => {
      queryClient.setQueryData(['codexOauthProbeSession', updated.session_id], updated)
      notify({
        variant: updated.status === 'completed' ? 'success' : 'info',
        title: t('oauthProbe.notifications.manualSubmitTitle'),
        description:
          updated.status === 'completed'
            ? t('oauthProbe.notifications.manualSubmitSuccess')
            : t('oauthProbe.notifications.manualSubmitAccepted'),
      })
    },
    onError: (error: unknown) => {
      notify({
        variant: 'error',
        title: t('oauthProbe.notifications.manualSubmitFailedTitle'),
        description: localizeApiErrorDisplay(t, error, t('oauthProbe.notifications.unknownError')).label,
      })
    },
  })

  const showResult = Boolean(session?.result && session.status === 'completed')
  const showError = Boolean(session?.error && (session.status === 'failed' || session.status === 'expired'))
  const resultJson = session?.result ? JSON.stringify(session.result, null, 2) : ''

  const statusLabel = (() => {
    if (!session?.status) {
      return t('oauthProbe.status.idle')
    }
    return t(`oauthProbe.status.${session.status}`)
  })()

  const container = prefersReducedMotion
    ? undefined
    : {
        hidden: { opacity: 0 },
        show: { opacity: 1, transition: { staggerChildren: 0.08 } },
      }

  const item = prefersReducedMotion
    ? undefined
    : {
        hidden: { opacity: 0, y: 10 },
        show: {
          opacity: 1,
          y: 0,
          transition: { type: 'spring' as const, stiffness: 260, damping: 24 },
        },
      }

  return (
    <motion.div
      variants={container}
      initial={prefersReducedMotion ? undefined : 'hidden'}
      animate={prefersReducedMotion ? undefined : 'show'}
      className="flex-1 overflow-y-auto px-4 py-6 md:px-8 md:py-8 space-y-6"
    >
      <motion.div variants={item} className="space-y-2">
        <h2 className="text-3xl font-bold tracking-tight">{t('oauthProbe.title')}</h2>
        <p className="text-muted-foreground">{t('oauthProbe.subtitle')}</p>
      </motion.div>

      <motion.div variants={item}>
        <Card className="border-border/60 shadow-sm">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <FileJson className="h-5 w-5 text-primary" />
              {t('oauthProbe.start.title')}
            </CardTitle>
            <CardDescription>{t('oauthProbe.start.description')}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">{t('oauthProbe.form.baseUrl')}</label>
              <Input
                value={baseUrl}
                onChange={(event) => setBaseUrl(event.target.value)}
                placeholder={DEFAULT_BASE_URL}
              />
            </div>

            <div className="flex flex-wrap items-center gap-2">
              <Button
                type="button"
                disabled={createSessionMutation.isPending}
                onClick={() => createSessionMutation.mutate()}
              >
                {createSessionMutation.isPending ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <ExternalLink className="h-4 w-4" />
                )}
                {t('oauthProbe.actions.startProbe')}
              </Button>

              <Button
                type="button"
                variant="outline"
                disabled={!session?.authorize_url}
                onClick={() => {
                  if (session?.authorize_url) {
                    openAuthorizeTab(session.authorize_url)
                  }
                }}
              >
                <RefreshCcw className="h-4 w-4" />
                {t('oauthProbe.actions.reopenAuth')}
              </Button>

              <Button
                type="button"
                variant="outline"
                disabled={!session?.result}
                onClick={() => {
                  if (session?.result && session?.session_id) {
                    downloadProbeResult(session.result, session.session_id)
                  }
                }}
              >
                <Download className="h-4 w-4" />
                {t('oauthProbe.actions.downloadJson')}
              </Button>
            </div>

            <div className="rounded-lg border border-border/60 bg-muted/20 px-4 py-3 space-y-2">
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-sm text-muted-foreground">{t('oauthProbe.status.label')}</span>
                <Badge variant={statusBadgeVariant(session?.status)}>
                  {sessionQuery.isFetching && !isTerminalStatus(session?.status) ? (
                    <span className="inline-flex items-center gap-1">
                      <Loader2 className="h-3 w-3 animate-spin" />
                      {statusLabel}
                    </span>
                  ) : (
                    statusLabel
                  )}
                </Badge>
                {session?.session_id ? (
                  <span className="font-mono text-xs text-muted-foreground">
                    {t('oauthProbe.status.sessionId', { id: session.session_id })}
                  </span>
                ) : null}
              </div>
              {session?.callback_url ? (
                <div className="text-xs text-muted-foreground break-all">
                  {t('oauthProbe.status.callbackUrl', { url: session.callback_url })}
                </div>
              ) : null}
              {session?.expires_at ? (
                <div className="text-xs text-muted-foreground">
                  {t('oauthProbe.status.expiresAt', {
                    time: new Date(session.expires_at).toLocaleString(),
                  })}
                </div>
              ) : null}
              <div className="text-xs text-muted-foreground">
                {t('oauthProbe.status.memoryOnly')}
              </div>
            </div>

            {showError ? (
              <div
                className={cn(
                  'rounded-md border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive flex items-start gap-2',
                )}
              >
                <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
                <div>
                  <div>{t('oauthProbe.error.failed')}</div>
                  <div className="mt-1 text-xs">
                    {session?.error?.code}: {session?.error?.message}
                  </div>
                </div>
              </div>
            ) : null}

            {showResult ? (
              <div className="rounded-md border border-success/30 bg-success-muted px-4 py-3 text-sm text-success-foreground space-y-2">
                <div>{t('oauthProbe.result.success')}</div>
                <div className="grid gap-2 md:grid-cols-2">
                  <div className="text-xs text-success-foreground/80">
                    {t('oauthProbe.result.email', { email: session?.result?.email ?? '-' })}
                  </div>
                  <div className="text-xs text-success-foreground/80">
                    {t('oauthProbe.result.accountId', {
                      id: session?.result?.chatgpt_account_id ?? '-',
                    })}
                  </div>
                  <div className="text-xs text-success-foreground/80">
                    {t('oauthProbe.result.plan', {
                      plan: session?.result?.chatgpt_plan_type ?? '-',
                    })}
                  </div>
                  <div className="text-xs text-success-foreground/80">
                    {t('oauthProbe.result.expiresAt', {
                      time: session?.result?.expires_at
                        ? new Date(session.result.expires_at).toLocaleString()
                        : '-',
                    })}
                  </div>
                  <div className="text-xs text-success-foreground/80 break-all">
                    {t('oauthProbe.result.accessTokenPreview', {
                      value: session?.result?.access_token_preview ?? '-',
                    })}
                  </div>
                  <div className="text-xs text-success-foreground/80 break-all">
                    {t('oauthProbe.result.refreshTokenPreview', {
                      value: session?.result?.refresh_token_preview ?? '-',
                    })}
                  </div>
                </div>
              </div>
            ) : null}
          </CardContent>
        </Card>
      </motion.div>

      <motion.div variants={item}>
        <Card className="border-border/60 shadow-sm">
          <CardHeader>
            <CardTitle>{t('oauthProbe.payload.title')}</CardTitle>
            <CardDescription>{t('oauthProbe.payload.description')}</CardDescription>
          </CardHeader>
          <CardContent>
            <pre className="max-h-[420px] overflow-auto rounded-lg border border-border/60 bg-muted/20 p-4 text-xs leading-5 text-foreground whitespace-pre-wrap break-all">
              {resultJson || t('oauthProbe.payload.empty')}
            </pre>
          </CardContent>
        </Card>
      </motion.div>

      <motion.div variants={item}>
        <Card className="border-border/60 shadow-sm">
          <CardHeader>
            <CardTitle>{t('oauthProbe.manual.title')}</CardTitle>
            <CardDescription>{t('oauthProbe.manual.description')}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <Textarea
              value={manualRedirectUrl}
              onChange={(event) => setManualRedirectUrl(event.target.value)}
              placeholder={t('oauthProbe.manual.placeholder')}
              rows={4}
            />
            <div className="flex flex-wrap items-center gap-2">
              <Button
                type="button"
                variant="outline"
                disabled={!sessionId || submitManualCallbackMutation.isPending || !manualRedirectUrl.trim()}
                onClick={() => submitManualCallbackMutation.mutate()}
              >
                {submitManualCallbackMutation.isPending ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : null}
                {t('oauthProbe.actions.submitCallback')}
              </Button>
              <span className="text-xs text-muted-foreground">{t('oauthProbe.manual.hint')}</span>
            </div>
          </CardContent>
        </Card>
      </motion.div>
    </motion.div>
  )
}
