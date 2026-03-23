import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { motion, useReducedMotion } from 'framer-motion'
import { useMutation, useQueries, useQuery, useQueryClient } from '@tanstack/react-query'
import { type ColumnDef } from '@tanstack/react-table'
import {
  AlertCircle,
  CheckCircle2,
  Clock3,
  HardDriveDownload,
  Loader2,
  Pause,
  Play,
  UploadCloud,
  XCircle,
} from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { accountsApi } from '@/api/accounts'
import { localizeApiErrorDisplay, localizeOAuthErrorCodeDisplay } from '@/api/errorI18n'
import {
  importJobsApi,
  type OAuthImportAdmissionStatus,
  type OAuthImportCredentialMode,
  type OAuthImportJobItem,
  type OAuthImportJobSummary,
} from '@/api/importJobs'
import AnimatedContent from '@/components/AnimatedContent'
import {
  PageIntro,
  PagePanel,
  SectionHeader,
  WorkspaceShell,
} from '@/components/layout/page-archetypes'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { useConfirmDialog } from '@/components/ui/confirm-dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { StandardDataTable } from '@/components/ui/standard-data-table'
import { cn } from '@/lib/utils'
import {
  MAX_RECENT_JOBS,
  RECENT_JOBS_STORAGE_KEY,
  type StagedImportFile,
} from '@/features/import-jobs/types'
import {
  buildFileId,
  formatBytes,
  getImportStatusLabel,
  getStagedStatusBadgeVariant,
  getStagedStatusLabel,
  inspectStagedFile,
  loadRecentJobIds,
} from '@/features/import-jobs/utils'

const JOB_ITEMS_PAGE_SIZE = 500
const JOB_ITEMS_MAX_PAGES = 200
type JobAdmissionFilter = 'all' | OAuthImportAdmissionStatus

function isRunningStatus(status: OAuthImportJobSummary['status'] | undefined) {
  return status === 'queued' || status === 'running'
}

function toDisplayStatus(status: OAuthImportJobSummary['status'] | undefined) {
  return isRunningStatus(status) ? 'running' : 'completed'
}

async function loadAllJobItems(jobId: string) {
  const all: OAuthImportJobItem[] = []
  let cursor: number | undefined
  let pages = 0

  while (pages < JOB_ITEMS_MAX_PAGES) {
    const page = await importJobsApi.getJobItems(jobId, {
      cursor,
      limit: JOB_ITEMS_PAGE_SIZE,
    })
    all.push(...page.items)
    cursor = page.next_cursor
    pages += 1
    if (!cursor) {
      break
    }
  }

  return all
}

function formatTopValues(values: string[]) {
  if (values.length === 0) {
    return '-'
  }
  return values.join(' · ')
}

function getAdmissionStatusLabel(
  t: ReturnType<typeof useTranslation>['t'],
  status: OAuthImportAdmissionStatus | undefined,
) {
  if (status === 'ready') {
    return t('importJobs.admission.status.ready', { defaultValue: 'Ready' })
  }
  if (status === 'needs_refresh') {
    return t('importJobs.admission.status.needsRefresh', { defaultValue: 'Needs refresh' })
  }
  if (status === 'no_quota') {
    return t('importJobs.admission.status.noQuota', { defaultValue: 'No quota' })
  }
  if (status === 'failed') {
    return t('importJobs.admission.status.failed', { defaultValue: 'Failed' })
  }
  if (status === 'queued') {
    return t('importJobs.admission.status.queued', { defaultValue: 'Queued' })
  }
  return t('importJobs.admission.status.unknown', { defaultValue: 'Unknown' })
}

function getAdmissionStatusVariant(
  status: OAuthImportAdmissionStatus | undefined,
): 'success' | 'warning' | 'destructive' | 'secondary' | 'info' {
  if (status === 'ready') return 'success'
  if (status === 'needs_refresh') return 'warning'
  if (status === 'no_quota') return 'info'
  if (status === 'failed') return 'destructive'
  return 'secondary'
}

export default function ImportJobs() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const prefersReducedMotion = useReducedMotion()
  const { confirm, confirmDialog } = useConfirmDialog()

  const [isDragging, setIsDragging] = useState(false)
  const [isInspectingFiles, setIsInspectingFiles] = useState(false)
  const [uploadError, setUploadError] = useState<string | null>(null)
  const [uploadNotice, setUploadNotice] = useState<string | null>(null)
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null)
  const [recentJobIds, setRecentJobIds] = useState<string[]>(() => loadRecentJobIds())
  const [stagedFiles, setStagedFiles] = useState<StagedImportFile[]>([])
  const [pausedTrackingJobIds, setPausedTrackingJobIds] = useState<string[]>([])
  const [credentialMode, setCredentialMode] = useState<OAuthImportCredentialMode>('refresh_token')
  const [jobAdmissionFilter, setJobAdmissionFilter] = useState<JobAdmissionFilter>('all')
  const fileInputRef = useRef<HTMLInputElement>(null)

  const pausedTrackingJobIdSet = useMemo(
    () => new Set(pausedTrackingJobIds),
    [pausedTrackingJobIds],
  )

  const uploadMutation = useMutation({
    mutationFn: (files: File[]) => importJobsApi.createJob(files, { credential_mode: credentialMode }),
    onSuccess: (summary) => {
      setUploadError(null)
      setUploadNotice(
        t('importJobs.precheck.createdNotice', {
          id: summary.job_id,
        }),
      )
      setStagedFiles([])
      setSelectedJobId(summary.job_id)
      setPausedTrackingJobIds((prev) => prev.filter((id) => id !== summary.job_id))
      setRecentJobIds((prev) => {
        const next = [summary.job_id, ...prev.filter((id) => id !== summary.job_id)]
        return next.slice(0, MAX_RECENT_JOBS)
      })
    },
    onError: (error: unknown) => {
      setUploadError(localizeApiErrorDisplay(t, error, t('importJobs.error')).label)
      setUploadNotice(null)
    },
  })

  const cancelMutation = useMutation({
    mutationFn: (jobId: string) => importJobsApi.cancelJob(jobId),
    onSuccess: (_, jobId) => {
      queryClient.invalidateQueries({ queryKey: ['jobSummary', jobId] })
    },
  })

  useEffect(() => {
    localStorage.setItem(RECENT_JOBS_STORAGE_KEY, JSON.stringify(recentJobIds))
  }, [recentJobIds])

  const effectiveSelectedJobId = useMemo(() => {
    if (selectedJobId && recentJobIds.includes(selectedJobId)) {
      return selectedJobId
    }
    return recentJobIds[0] ?? null
  }, [recentJobIds, selectedJobId])

  const reviewStats = useMemo(() => {
    let ready = 0
    let warning = 0
    let invalid = 0
    let totalBytes = 0
    let estimatedRecords = 0
    let refreshTokenRecords = 0
    let accessTokenRecords = 0
    let chatgptAccountIdRecords = 0
    let emailRecords = 0

    const baseUrlHints = new Set<string>()
    const sourceTypeHints = new Set<string>()
    const planTypeHints = new Set<string>()

    stagedFiles.forEach((item) => {
      totalBytes += item.file.size
      estimatedRecords += item.metadata.estimatedRecords
      refreshTokenRecords += item.metadata.refreshTokenRecords
      accessTokenRecords += item.metadata.accessTokenRecords
      chatgptAccountIdRecords += item.metadata.chatgptAccountIdRecords
      emailRecords += item.metadata.emailRecords

      item.metadata.baseUrlTop.forEach((value) => {
        if (baseUrlHints.size < 3) {
          baseUrlHints.add(value)
        }
      })
      item.metadata.sourceTypeTop.forEach((value) => {
        if (sourceTypeHints.size < 3) {
          sourceTypeHints.add(value)
        }
      })
      item.metadata.planTypeTop.forEach((value) => {
        if (planTypeHints.size < 3) {
          planTypeHints.add(value)
        }
      })

      if (item.status === 'ready') {
        ready += 1
      } else if (item.status === 'warning') {
        warning += 1
      } else {
        invalid += 1
      }
    })

    return {
      ready,
      warning,
      invalid,
      total: stagedFiles.length,
      totalBytes,
      estimatedRecords,
      refreshTokenRecords,
      accessTokenRecords,
      chatgptAccountIdRecords,
      emailRecords,
      baseUrlHints: [...baseUrlHints],
      sourceTypeHints: [...sourceTypeHints],
      planTypeHints: [...planTypeHints],
    }
  }, [stagedFiles])

  const importableFiles = useMemo(
    () => stagedFiles.filter((item) => item.status !== 'invalid').map((item) => item.file),
    [stagedFiles],
  )

  const stagedColumns = useMemo<ColumnDef<StagedImportFile>[]>(
    () => [
      {
        id: 'file',
        accessorFn: (row) => row.file.name.toLowerCase(),
        header: t('importJobs.workspace.columns.file'),
        cell: ({ row }) => (
          <div className="min-w-[220px]">
            <div className="break-all font-medium">{row.original.file.name}</div>
            <div className="mt-1 text-[11px] text-muted-foreground">
              .{row.original.extension === 'unknown' ? '-' : row.original.extension}
            </div>
          </div>
        ),
      },
      {
        id: 'size',
        accessorFn: (row) => row.file.size,
        header: t('importJobs.workspace.columns.size'),
        cell: ({ row }) => (
          <span className="text-muted-foreground">{formatBytes(row.original.file.size)}</span>
        ),
      },
      {
        id: 'metadata',
        accessorFn: (row) => row.metadata.estimatedRecords,
        header: t('importJobs.metrics.total'),
        cell: ({ row }) => {
          const metadata = row.original.metadata
          return (
            <div className="min-w-[280px] space-y-1 text-[11px] text-muted-foreground">
              <div className="font-medium text-foreground">
                {t('importJobs.metrics.total')} {metadata.estimatedRecords}
              </div>
              <div>refresh_token: {metadata.refreshTokenRecords}</div>
              <div>access_token: {metadata.accessTokenRecords}</div>
              <div>chatgpt_account_id: {metadata.chatgptAccountIdRecords}</div>
              <div>email: {metadata.emailRecords}</div>
            </div>
          )
        },
      },
      {
        id: 'check',
        accessorFn: (row) => row.checks.join(' ').toLowerCase(),
        header: t('importJobs.workspace.columns.check'),
        cell: ({ row }) => (
          <div className="min-w-[280px] text-muted-foreground">
            <ul className="space-y-1">
              {row.original.checks.slice(0, 2).map((check, index) => (
                <li key={`${row.original.id}-${index}`}>{check}</li>
              ))}
            </ul>
            {row.original.checks.length > 2 ? (
              <div className="mt-1 text-[11px] text-muted-foreground">
                +{row.original.checks.length - 2} {t('importJobs.workspace.moreChecks')}
              </div>
            ) : null}
          </div>
        ),
      },
      {
        id: 'status',
        accessorFn: (row) => row.status,
        header: t('importJobs.workspace.columns.status'),
        cell: ({ row }) => (
          <Badge variant={getStagedStatusBadgeVariant(row.original.status)}>
            {getStagedStatusLabel(t, row.original.status)}
          </Badge>
        ),
      },
      {
        id: 'actions',
        enableSorting: false,
        header: t('importJobs.workspace.columns.action'),
        cell: ({ row }) => (
          <Button
            type="button"
            variant="ghost"
            size="xs"
            className="cursor-pointer"
            onClick={() => {
              setStagedFiles((prev) => prev.filter((item) => item.id !== row.original.id))
            }}
          >
            {t('importJobs.actions.remove')}
          </Button>
        ),
      },
    ],
    [t],
  )

  const queueFilesForReview = useCallback(
    async (incomingFiles: File[]) => {
      if (incomingFiles.length === 0) {
        return
      }

      setUploadError(null)
      setUploadNotice(null)

      const existingIds = new Set(stagedFiles.map((item) => item.id))
      const deduped = incomingFiles.filter((file) => !existingIds.has(buildFileId(file)))

      if (deduped.length === 0) {
        setUploadError(t('importJobs.precheck.duplicateBatch'))
        return
      }

      const nameCounts = new Map<string, number>()
      ;[...stagedFiles.map((item) => item.file), ...deduped].forEach((file) => {
        const key = file.name.trim().toLowerCase()
        nameCounts.set(key, (nameCounts.get(key) ?? 0) + 1)
      })

      setIsInspectingFiles(true)
      try {
        const inspected = await Promise.all(
          deduped.map((file) =>
            inspectStagedFile(file, (nameCounts.get(file.name.trim().toLowerCase()) ?? 0) > 1, t),
          ),
        )
        setStagedFiles((prev) => [...prev, ...inspected])
      } finally {
        setIsInspectingFiles(false)
      }
    },
    [stagedFiles, t],
  )

  useEffect(() => {
    const handlePaste = (event: ClipboardEvent) => {
      const files = Array.from(event.clipboardData?.files ?? [])
      if (files.length === 0) {
        return
      }
      event.preventDefault()
      void queueFilesForReview(files)
    }

    window.addEventListener('paste', handlePaste)
    return () => window.removeEventListener('paste', handlePaste)
  }, [queueFilesForReview])

  const handleDropZoneDrag = useCallback(
    (event: React.DragEvent<HTMLDivElement>) => {
      event.preventDefault()
      event.stopPropagation()

      if (event.type === 'dragenter' || event.type === 'dragover') {
        setIsDragging(true)
        return
      }

      if (event.type === 'dragleave') {
        setIsDragging(false)
        return
      }

      if (event.type === 'drop') {
        setIsDragging(false)
        const files = Array.from(event.dataTransfer.files || [])
        if (files.length > 0) {
          void queueFilesForReview(files)
        }
      }
    },
    [queueFilesForReview],
  )

  const handleStartImport = useCallback(() => {
    if (importableFiles.length === 0) {
      setUploadError(t('importJobs.precheck.noneImportable'))
      return
    }
    uploadMutation.mutate(importableFiles)
  }, [importableFiles, t, uploadMutation])

  const handleDownloadTemplate = useCallback(() => {
    const example =
      credentialMode === 'access_token'
        ? {
            email: 'demo@example.com',
            account_id: '00000000-0000-0000-0000-000000000000',
            access_token: 'ak_xxx',
            exp: 1893456000,
            base_url: 'https://chatgpt.com/backend-api/codex',
            enabled: true,
            priority: 100,
          }
        : {
            email: 'demo@example.com',
            account_id: '00000000-0000-0000-0000-000000000000',
            refresh_token: 'rt_xxx',
            base_url: 'https://chatgpt.com/backend-api/codex',
            enabled: true,
            priority: 100,
          }
    const blob = new Blob([`${JSON.stringify(example)}\n`], { type: 'application/jsonl' })
    const url = URL.createObjectURL(blob)
    const anchor = document.createElement('a')
    anchor.href = url
    anchor.download = 'oauth-import-template.jsonl'
    anchor.click()
    URL.revokeObjectURL(url)
  }, [credentialMode])

  const recentJobQueries = useQueries({
    queries: recentJobIds.map((jobId) => ({
      queryKey: ['jobSummary', jobId],
      queryFn: () => importJobsApi.getJobSummary(jobId),
      retry: false,
      staleTime: 3000,
      refetchInterval: pausedTrackingJobIdSet.has(jobId) ? false : 3000,
    })),
  })

  const recentJobs = useMemo(() => {
    return recentJobIds.map((jobId, index) => {
      const query = recentJobQueries[index]
      return {
        jobId,
        summary: query?.data,
        isLoading: query?.isLoading ?? false,
        errorMessage: query?.error
          ? localizeApiErrorDisplay(
              t,
              query.error,
              t('importJobs.messages.unknownError', { defaultValue: 'Unknown error' }),
            ).label
          : null,
      }
    })
  }, [recentJobIds, recentJobQueries, t])

  const selectedJob = useMemo(
    () => recentJobs.find((item) => item.jobId === effectiveSelectedJobId),
    [effectiveSelectedJobId, recentJobs],
  )

  const selectedSummary = selectedJob?.summary
  const selectedDisplayStatus = toDisplayStatus(selectedSummary?.status)
  const selectedIsRunning = selectedDisplayStatus === 'running'
  const selectedTrackingPaused = effectiveSelectedJobId
    ? pausedTrackingJobIdSet.has(effectiveSelectedJobId)
    : false

  const shouldTrackSelectedJob =
    !!effectiveSelectedJobId && selectedIsRunning && !selectedTrackingPaused

  const selectedJobItemsQuery = useQuery({
    queryKey: ['jobItemsAll', effectiveSelectedJobId],
    enabled: !!effectiveSelectedJobId,
    queryFn: () => loadAllJobItems(effectiveSelectedJobId!),
    staleTime: shouldTrackSelectedJob ? 0 : 20000,
    refetchInterval: shouldTrackSelectedJob ? 3000 : false,
  })

  const accountsInPoolQuery = useQuery({
    queryKey: ['upstreamAccountsForImportJobsPoolProgress'],
    enabled: !!effectiveSelectedJobId,
    queryFn: accountsApi.listAccounts,
    staleTime: shouldTrackSelectedJob ? 0 : 20000,
    refetchInterval: shouldTrackSelectedJob ? 4000 : false,
  })

  const poolProgress = useMemo(() => {
    const summary = selectedSummary
    if (!summary) {
      return {
        inPool: 0,
        total: 0,
        percent: 0,
      }
    }

    const targetItems = (selectedJobItemsQuery.data ?? []).filter(
      (item) => !['failed', 'cancelled', 'skipped'].includes(item.status),
    )

    const targetKeys = new Set<string>()
    targetItems.forEach((item) => {
      if (item.account_id) {
        targetKeys.add(`id:${item.account_id}`)
        return
      }
      if (item.chatgpt_account_id) {
        targetKeys.add(`chatgpt:${item.chatgpt_account_id}`)
      }
    })

    const fallbackTotal = Math.max(0, summary.total - summary.failed_count - summary.skipped_count)
    const total = targetKeys.size > 0 ? targetKeys.size : fallbackTotal

    if (total <= 0) {
      return {
        inPool: 0,
        total: 0,
        percent: 0,
      }
    }

    if (targetKeys.size === 0) {
      const fallbackInPool = Math.min(summary.created_count + summary.updated_count, total)
      return {
        inPool: fallbackInPool,
        total,
        percent: (fallbackInPool / total) * 100,
      }
    }

    const accounts = accountsInPoolQuery.data ?? []
    const accountIdSet = new Set(accounts.map((item) => item.id))
    const chatgptAccountIdSet = new Set(
      accounts
        .map((item) => item.chatgpt_account_id?.trim())
        .filter((value): value is string => !!value),
    )

    let inPool = 0
    targetKeys.forEach((key) => {
      if (key.startsWith('id:')) {
        if (accountIdSet.has(key.slice(3))) {
          inPool += 1
        }
        return
      }
      if (key.startsWith('chatgpt:') && chatgptAccountIdSet.has(key.slice(8))) {
        inPool += 1
      }
    })

    return {
      inPool,
      total,
      percent: (inPool / total) * 100,
    }
  }, [accountsInPoolQuery.data, selectedJobItemsQuery.data, selectedSummary])

  const filteredJobItems = useMemo(() => {
    const items = selectedJobItemsQuery.data ?? []
    if (jobAdmissionFilter === 'all') {
      return items
    }
    return items.filter((item) => item.admission_status === jobAdmissionFilter)
  }, [jobAdmissionFilter, selectedJobItemsQuery.data])

  const selectedAdmissionCounts = selectedSummary?.admission_counts ?? {
    ready: 0,
    needs_refresh: 0,
    no_quota: 0,
    failed: 0,
  }

  const jobItemColumns = useMemo<ColumnDef<OAuthImportJobItem>[]>(
    () => [
      {
        id: 'label',
        accessorFn: (row) => `${row.email ?? ''} ${row.label ?? ''}`.toLowerCase(),
        header: t('importJobs.detail.columns.label'),
        cell: ({ row }) => (
          <div className="min-w-[220px] space-y-1">
            <div className="font-medium text-foreground">{row.original.email ?? row.original.label}</div>
            <div className="text-xs text-muted-foreground">{row.original.label}</div>
            {row.original.chatgpt_account_id ? (
              <div className="font-mono text-[11px] text-muted-foreground">
                {row.original.chatgpt_account_id}
              </div>
            ) : null}
          </div>
        ),
      },
      {
        id: 'status',
        accessorFn: (row) => row.status,
        header: t('importJobs.detail.columns.status'),
        cell: ({ row }) => (
          <Badge variant={row.original.status === 'failed' ? 'destructive' : row.original.status === 'created' || row.original.status === 'updated' ? 'success' : 'secondary'}>
            {getImportStatusLabel(t, row.original.status)}
          </Badge>
        ),
      },
      {
        id: 'admission',
        accessorFn: (row) => row.admission_status ?? '',
        header: t('importJobs.detail.columns.admission', { defaultValue: 'Admission' }),
        cell: ({ row }) => (
          <div className="min-w-[148px] space-y-1">
            <Badge variant={getAdmissionStatusVariant(row.original.admission_status)}>
              {getAdmissionStatusLabel(t, row.original.admission_status)}
            </Badge>
            {row.original.admission_source ? (
              <div className="text-xs text-muted-foreground">
                {row.original.admission_source}
              </div>
            ) : null}
          </div>
        ),
      },
      {
        id: 'quota',
        accessorFn: (row) => row.admission_status ?? '',
        header: t('importJobs.detail.columns.quota', { defaultValue: 'Quota' }),
        cell: ({ row }) => {
          if (row.original.admission_status === 'no_quota') {
            return (
              <div className="min-w-[140px] text-xs text-sky-700 dark:text-sky-300">
                {t('importJobs.admission.quotaExhausted', {
                  defaultValue: 'Quota exhausted, waiting for reprobe.',
                })}
              </div>
            )
          }
          if (row.original.admission_status === 'ready') {
            return (
              <div className="min-w-[140px] text-xs text-emerald-700 dark:text-emerald-300">
                {t('importJobs.admission.quotaReady', {
                  defaultValue: 'Probe succeeded and quota is available.',
                })}
              </div>
            )
          }
          return (
            <div className="min-w-[140px] text-xs text-muted-foreground">
              {t('importJobs.admission.quotaNotApplicable', {
                defaultValue: 'Quota summary unavailable.',
              })}
            </div>
          )
        },
      },
      {
        id: 'reason',
        accessorFn: (row) => `${row.error_code ?? ''} ${row.admission_reason ?? ''}`.toLowerCase(),
        header: t('importJobs.detail.columns.reason', { defaultValue: 'Reason' }),
        cell: ({ row }) => {
          const admissionDisplay = localizeOAuthErrorCodeDisplay(t, row.original.admission_reason)
          const importDisplay = localizeOAuthErrorCodeDisplay(t, row.original.error_code)
          const label = row.original.admission_reason
            ? admissionDisplay.label
            : row.original.error_code
              ? importDisplay.label
              : '-'
          const tooltip = row.original.admission_reason
            ? admissionDisplay.tooltip
            : row.original.error_code
              ? importDisplay.tooltip
              : undefined
          return (
            <div className="min-w-[220px]">
              <div className="truncate text-sm text-foreground" title={tooltip}>
                {label}
              </div>
              {row.original.error_message ? (
                <div className="mt-1 truncate text-xs text-muted-foreground" title={row.original.error_message}>
                  {row.original.error_message}
                </div>
              ) : null}
            </div>
          )
        },
      },
      {
        id: 'line',
        accessorFn: (row) => row.line_no,
        header: t('importJobs.detail.columns.line'),
        cell: ({ row }) => (
          <div className="text-xs text-muted-foreground">
            {row.original.source_file}:{row.original.line_no}
          </div>
        ),
      },
    ],
    [t],
  )

  const toggleTrackingPaused = useCallback(() => {
    if (!effectiveSelectedJobId) {
      return
    }
    setPausedTrackingJobIds((prev) => {
      if (prev.includes(effectiveSelectedJobId)) {
        return prev.filter((id) => id !== effectiveSelectedJobId)
      }
      return [...prev, effectiveSelectedJobId]
    })
  }, [effectiveSelectedJobId])

  const handleCancelSelectedJob = useCallback(async () => {
    if (!effectiveSelectedJobId) {
      return
    }
    const confirmed = await confirm({
      title: t('importJobs.actions.cancelJob'),
      description: t('importJobs.actions.confirmCancelJob'),
      cancelText: t('common.cancel', { defaultValue: 'Cancel' }),
      confirmText: t('common.confirm', { defaultValue: 'Confirm' }),
      variant: 'destructive',
    })
    if (!confirmed) {
      return
    }
    cancelMutation.mutate(effectiveSelectedJobId)
  }, [cancelMutation, confirm, effectiveSelectedJobId, t])

  return (
    <motion.div
      initial={prefersReducedMotion ? undefined : { opacity: 0, y: 8 }}
      animate={prefersReducedMotion ? undefined : { opacity: 1, y: 0 }}
      transition={{ duration: 0.35 }}
      className="flex-1 space-y-5 overflow-y-auto px-4 py-6 md:px-8 md:py-8"
    >
      <WorkspaceShell
        intro={(
          <PageIntro
            archetype="workspace"
            eyebrow={t('nav.importJobs')}
            title={t('importJobs.title')}
            description={t('importJobs.subtitleModern')}
            meta={(
              <div className="flex flex-wrap gap-x-3 gap-y-1">
                <span>{t('importJobs.dropzone.acceptsNew')}</span>
                <span>
                  {t('importJobs.workspace.totalFiles', { count: reviewStats.total })} ·{' '}
                  {t('importJobs.workspace.totalSize', { size: formatBytes(reviewStats.totalBytes) })}
                </span>
              </div>
            )}
          />
        )}
        primary={(
          <AnimatedContent>
            <PagePanel className="space-y-0 overflow-hidden p-0">
              <div className="space-y-5 px-5 py-5 sm:px-6">
                <SectionHeader
                  title={t('importJobs.workspace.title')}
                  description={t('importJobs.workspace.desc')}
                />

                <div className="rounded-[0.95rem] border border-border/70 bg-muted/14 px-4 py-4">
                  <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_14rem] lg:items-start">
                    <div className="space-y-1">
                      <div className="text-sm font-medium">
                        {t('importJobs.credentialMode.title')}
                      </div>
                      <p className="text-sm text-muted-foreground">
                        {t('importJobs.credentialMode.description')}
                      </p>
                      <p className="text-xs text-muted-foreground">
                        {credentialMode === 'access_token'
                          ? t('importJobs.credentialMode.accessTokenHint')
                          : t('importJobs.credentialMode.refreshTokenHint')}
                      </p>
                    </div>
                    <Select
                      value={credentialMode}
                      onValueChange={(value) => setCredentialMode(value as OAuthImportCredentialMode)}
                    >
                      <SelectTrigger aria-label={t('importJobs.credentialMode.title')}>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="refresh_token">
                          {t('importJobs.credentialMode.refreshToken')}
                        </SelectItem>
                        <SelectItem value="access_token">
                          {t('importJobs.credentialMode.accessToken')}
                        </SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>

                <div
                  className={cn(
                    'rounded-[0.95rem] border border-dashed p-5 transition-colors sm:p-6',
                    isDragging ? 'border-primary bg-primary/5' : 'border-border/70 bg-card/55',
                    (uploadMutation.isPending || isInspectingFiles) && 'pointer-events-none opacity-80',
                  )}
                  onDragEnter={handleDropZoneDrag}
                  onDragOver={handleDropZoneDrag}
                  onDragLeave={handleDropZoneDrag}
                  onDrop={handleDropZoneDrag}
                >
                <input
                  ref={fileInputRef}
                  type="file"
                  name="import_files"
                  className="hidden"
                  aria-label={t('importJobs.dropzone.selectFiles')}
                  accept=".json,.jsonl,application/json"
                  multiple
                  onChange={(event) => {
                    const files = Array.from(event.target.files || [])
                    if (files.length > 0) {
                      void queueFilesForReview(files)
                    }
                    event.currentTarget.value = ''
                  }}
                />

                  <div className="flex flex-col items-start gap-5 text-left">
                    <div className="flex items-center gap-4">
                      <div className="flex h-12 w-12 items-center justify-center rounded-[0.95rem] border border-border/70 bg-background/80">
                        {isInspectingFiles ? (
                          <Loader2 className="h-6 w-6 animate-spin text-primary" />
                        ) : (
                          <UploadCloud className="h-6 w-6 text-primary" />
                        )}
                      </div>
                      <div className="space-y-1">
                        <h3 className="text-base font-semibold text-foreground">
                          {t('importJobs.dropzone.titleNew')}
                        </h3>
                        <p className="text-sm text-muted-foreground">
                          {t('importJobs.dropzone.acceptsNew')}
                        </p>
                      </div>
                    </div>

                    <div className="flex w-full flex-wrap gap-2">
                      <Button type="button" onClick={() => fileInputRef.current?.click()}>
                        {t('importJobs.dropzone.selectFiles')}
                      </Button>
                      <Button type="button" variant="outline" onClick={handleDownloadTemplate}>
                        <HardDriveDownload className="h-4 w-4" />
                        {t('importJobs.template.downloadJsonl')}
                      </Button>
                    </div>
                  </div>
                </div>

                {uploadError ? (
                  <div className="flex items-start gap-2 rounded-[0.95rem] border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                    <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />
                    <div>{uploadError}</div>
                  </div>
                ) : null}

                {uploadNotice ? (
                  <div className="flex items-start gap-2 rounded-[0.95rem] border border-success/30 bg-success/10 px-4 py-3 text-sm text-success">
                    <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0" />
                    <div>{uploadNotice}</div>
                  </div>
                ) : null}

                <div className="space-y-4 border-t border-border/70 pt-4">
                  <div className="grid gap-px overflow-hidden rounded-[0.9rem] border border-border/70 bg-border/70 sm:grid-cols-3">
                    <StatChip
                      label={t('importJobs.workspace.readyFiles', { count: reviewStats.ready })}
                      value={reviewStats.ready}
                      tone="success"
                    />
                    <StatChip
                      label={t('importJobs.workspace.warningFiles', { count: reviewStats.warning })}
                      value={reviewStats.warning}
                      tone="warning"
                    />
                    <StatChip
                      label={t('importJobs.workspace.invalidFiles', { count: reviewStats.invalid })}
                      value={reviewStats.invalid}
                      tone="destructive"
                    />
                  </div>

                  <div className="grid gap-px overflow-hidden rounded-[0.9rem] border border-border/70 bg-border/70 sm:grid-cols-2 xl:grid-cols-4">
                    <MiniMetric title={t('importJobs.metrics.total')} value={reviewStats.estimatedRecords} />
                    <MiniMetric title="refresh_token" value={reviewStats.refreshTokenRecords} />
                    <MiniMetric title="access_token" value={reviewStats.accessTokenRecords} />
                    <MiniMetric title="chatgpt_account_id" value={reviewStats.chatgptAccountIdRecords} />
                  </div>

                  <details className="rounded-[0.9rem] border border-border/60 bg-muted/12 px-4 py-3">
                    <summary className="cursor-pointer list-none text-sm font-medium text-slate-700 marker:hidden dark:text-slate-200 [&::-webkit-details-marker]:hidden">
                      {t('importJobs.workspace.totalFiles', { count: reviewStats.total })} ·{' '}
                      {t('importJobs.workspace.totalSize', { size: formatBytes(reviewStats.totalBytes) })}
                    </summary>
                    <div className="mt-3 space-y-1 text-xs leading-5 text-muted-foreground">
                      <div>email: {reviewStats.emailRecords}</div>
                      <div>base_url: {formatTopValues(reviewStats.baseUrlHints)}</div>
                      <div>source_type: {formatTopValues(reviewStats.sourceTypeHints)}</div>
                      <div>plan_type: {formatTopValues(reviewStats.planTypeHints)}</div>
                    </div>
                  </details>
                </div>
              </div>

              <div className="space-y-3 border-t border-border/70 px-5 py-5 sm:px-6">
                <SectionHeader
                  title={t('importJobs.detail.title')}
                  description={t('importJobs.detail.searchPlaceholderModern')}
                />

                <div className="h-[360px]">
                  <StandardDataTable
                    columns={stagedColumns}
                    data={stagedFiles}
                    density="compact"
                    defaultPageSize={10}
                    pageSizeOptions={[10, 20, 50]}
                    searchPlaceholder={t('importJobs.detail.searchPlaceholderModern')}
                    searchFn={(row, keyword) => {
                      const haystack = [
                        row.file.name,
                        row.extension,
                        row.status,
                        row.checks.join(' '),
                        row.metadata.baseUrlTop.join(' '),
                        row.metadata.sourceTypeTop.join(' '),
                        row.metadata.planTypeTop.join(' '),
                      ]
                        .join(' ')
                        .toLowerCase()
                      return haystack.includes(keyword)
                    }}
                    emptyText={t('importJobs.workspace.empty')}
                  />
                </div>
              </div>

              <div className="border-t border-border/70 bg-muted/16 px-5 py-4 sm:px-6">
                <Button
                  type="button"
                  className="w-full"
                  disabled={uploadMutation.isPending || isInspectingFiles}
                  onClick={handleStartImport}
                >
                  {uploadMutation.isPending ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      {t('importJobs.dropzone.uploading')}
                    </>
                  ) : (
                    t('importJobs.workspace.startImportWithCount', {
                      count: importableFiles.length,
                    })
                  )}
                </Button>
              </div>
            </PagePanel>
          </AnimatedContent>
        )}
        secondary={(
          <AnimatedContent className="space-y-5">
            <PagePanel tone="secondary" className="space-y-0 rounded-[0.95rem] bg-transparent shadow-none">
              <div className="space-y-4">
                <SectionHeader
                  title={t('importJobs.progress.title')}
                  description={
                    effectiveSelectedJobId
                      ? t('importJobs.progress.jobIdLabel', { jobId: effectiveSelectedJobId })
                      : t('importJobs.progress.noJobSelected')
                  }
                />

                {!effectiveSelectedJobId ? (
                  <div className="rounded-[0.9rem] border border-border/60 bg-muted/20 px-4 py-10 text-center text-sm text-muted-foreground">
                    {t('importJobs.progress.noJobSelected')}
                  </div>
                ) : (
                  <>
                    <div className="flex items-center justify-between">
                      <Badge
                        variant={selectedIsRunning ? 'warning' : 'success'}
                        className="uppercase text-[10px]"
                      >
                        {getImportStatusLabel(t, selectedDisplayStatus)}
                      </Badge>
                      <Badge variant={selectedTrackingPaused ? 'secondary' : 'info'}>
                        {selectedTrackingPaused
                          ? t('accounts.actions.pauseGroup', { defaultValue: 'Paused' })
                          : t('importJobs.queue.tracked')}
                      </Badge>
                    </div>

                    <div className="space-y-2">
                      <div className="flex items-center justify-between text-xs text-muted-foreground tabular-nums">
                        <span>
                          {t('accounts.title', { defaultValue: 'Accounts' })} {poolProgress.inPool}/
                          {poolProgress.total}
                        </span>
                        <span>{poolProgress.percent.toFixed(1)}%</span>
                      </div>
                      <div className="h-2.5 overflow-hidden rounded-full bg-muted">
                        <div
                          className="h-full bg-primary transition-[width] duration-300"
                          style={{ width: `${Math.min(100, poolProgress.percent)}%` }}
                        />
                      </div>
                    </div>

                    <div className="grid gap-px overflow-hidden rounded-[0.9rem] border border-border/70 bg-border/70 sm:grid-cols-2">
                      <MiniMetric
                        title={t('accounts.title', { defaultValue: 'Accounts' })}
                        value={`${poolProgress.inPool}/${poolProgress.total}`}
                      />
                      <MiniMetric
                        title={t('importJobs.metrics.status')}
                        value={getImportStatusLabel(t, selectedDisplayStatus)}
                      />
                    </div>

                    <div className="text-xs text-muted-foreground">
                      {selectedJobItemsQuery.isFetching || accountsInPoolQuery.isFetching ? (
                        <span className="inline-flex items-center gap-1">
                          <Loader2 className="h-3.5 w-3.5 animate-spin" />
                          {t('common.loading')}
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1">
                          <Clock3 className="h-3.5 w-3.5" />
                          {t('importJobs.queue.tracked')}
                        </span>
                      )}
                    </div>

                    <div className="flex flex-wrap gap-2">
                      <Button type="button" variant="outline" size="sm" onClick={toggleTrackingPaused}>
                        {selectedTrackingPaused ? (
                          <>
                            <Play className="mr-1 h-3.5 w-3.5" />
                            {t('accounts.actions.resumeGroup', { defaultValue: 'Resume' })}
                          </>
                        ) : (
                          <>
                            <Pause className="mr-1 h-3.5 w-3.5" />
                            {t('accounts.actions.pauseGroup', { defaultValue: 'Pause' })}
                          </>
                        )}
                      </Button>
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        disabled={!selectedIsRunning || cancelMutation.isPending}
                        onClick={() => {
                          void handleCancelSelectedJob()
                        }}
                      >
                        {cancelMutation.isPending ? (
                          <>
                            <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />
                            {t('importJobs.actions.cancel')}
                          </>
                        ) : (
                          <>
                            <XCircle className="mr-1 h-3.5 w-3.5" />
                            {t('importJobs.actions.cancelJob')}
                          </>
                        )}
                      </Button>
                    </div>
                  </>
                )}
              </div>

              <div className="mt-5 space-y-4 border-t border-border/70 pt-5">
                <SectionHeader
                  title={t('importJobs.queue.titleRecent')}
                  description={t('importJobs.queue.descRecent')}
                />

                {recentJobs.length === 0 ? (
                  <div className="rounded-[0.9rem] border border-border/60 bg-muted/20 px-3 py-6 text-center text-sm text-muted-foreground">
                    {t('importJobs.queue.emptyRecent')}
                  </div>
                ) : (
                  <div className="overflow-hidden rounded-[0.9rem] border border-border/70">
                    {recentJobs.map((item, index) => {
                      const status = toDisplayStatus(item.summary?.status)
                      const selected = item.jobId === effectiveSelectedJobId
                      return (
                        <button
                          type="button"
                          key={item.jobId}
                          className={cn(
                            'w-full border-b border-border/70 px-3 py-3 text-left transition-colors last:border-b-0',
                            selected
                              ? 'bg-primary/6'
                              : 'bg-background/68 hover:bg-primary/5',
                            index === 0 && 'rounded-t-[0.9rem]',
                            index === recentJobs.length - 1 && 'rounded-b-[0.9rem]',
                          )}
                          onClick={() => setSelectedJobId(item.jobId)}
                        >
                          <div className="flex items-start justify-between gap-2">
                            <div className="min-w-0">
                              <div className="truncate font-mono text-xs">{item.jobId}</div>
                              <div className="mt-1 text-[11px] text-muted-foreground">
                                {item.summary
                                  ? `${item.summary.processed}/${item.summary.total}`
                                  : item.isLoading
                                    ? t('common.loading')
                                    : item.errorMessage ?? t('importJobs.messages.queryFailed')}
                              </div>
                            </div>
                            <Badge
                              variant={status === 'running' ? 'warning' : 'success'}
                              className="uppercase text-[10px]"
                            >
                              {getImportStatusLabel(t, status)}
                            </Badge>
                          </div>
                        </button>
                      )
                    })}
                  </div>
                )}
              </div>
            </PagePanel>
          </AnimatedContent>
        )}
      />

      <PagePanel className="space-y-4">
        <SectionHeader
          eyebrow={t('importJobs.admission.eyebrow', { defaultValue: 'Admission outcome' })}
          title={t('importJobs.detail.title')}
          description={
            effectiveSelectedJobId
              ? t('importJobs.detail.jobIdLabel', { jobId: effectiveSelectedJobId })
              : t('importJobs.detail.selectHint')
          }
          actions={(
            <Select
              value={jobAdmissionFilter}
              onValueChange={(value) => setJobAdmissionFilter(value as JobAdmissionFilter)}
            >
              <SelectTrigger
                className="min-w-[11rem]"
                aria-label={t('importJobs.detail.admissionFilterLabel', {
                  defaultValue: 'Admission filter',
                })}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">
                  {t('importJobs.detail.admissionFilterAll', { defaultValue: 'All outcomes' })}
                </SelectItem>
                <SelectItem value="ready">{getAdmissionStatusLabel(t, 'ready')}</SelectItem>
                <SelectItem value="needs_refresh">{getAdmissionStatusLabel(t, 'needs_refresh')}</SelectItem>
                <SelectItem value="no_quota">{getAdmissionStatusLabel(t, 'no_quota')}</SelectItem>
                <SelectItem value="failed">{getAdmissionStatusLabel(t, 'failed')}</SelectItem>
              </SelectContent>
            </Select>
          )}
        />

        <div className="grid gap-px overflow-hidden rounded-[0.9rem] border border-border/70 bg-border/70 sm:grid-cols-2 xl:grid-cols-4">
          <MiniMetric
            title={getAdmissionStatusLabel(t, 'ready')}
            value={selectedAdmissionCounts.ready}
          />
          <MiniMetric
            title={getAdmissionStatusLabel(t, 'needs_refresh')}
            value={selectedAdmissionCounts.needs_refresh}
          />
          <MiniMetric
            title={getAdmissionStatusLabel(t, 'no_quota')}
            value={selectedAdmissionCounts.no_quota}
          />
          <MiniMetric
            title={getAdmissionStatusLabel(t, 'failed')}
            value={selectedAdmissionCounts.failed}
          />
        </div>

        <StandardDataTable
          columns={jobItemColumns}
          data={filteredJobItems}
          density="comfortable"
          defaultPageSize={20}
          pageSizeOptions={[20, 50, 100]}
          className="min-h-[28rem] border border-border/60 bg-background/[0.5] shadow-none backdrop-blur-[2px]"
          searchPlaceholder={t('importJobs.detail.searchPlaceholderModern')}
          emptyText={
            effectiveSelectedJobId
              ? selectedJobItemsQuery.isLoading
                ? t('importJobs.detail.itemsLoading')
                : t('importJobs.detail.itemsEmpty')
              : t('importJobs.detail.selectHint')
          }
        />
      </PagePanel>

      {confirmDialog}
    </motion.div>
  )
}

function MiniMetric({ title, value }: { title: string; value: string | number }) {
  return (
    <div className="bg-background/88 px-3 py-3 dark:bg-card/86">
      <div className="text-[11px] text-muted-foreground">{title}</div>
      <div className="mt-1 text-sm font-semibold tabular-nums">{value}</div>
    </div>
  )
}

function StatChip({
  label,
  value,
  tone,
}: {
  label: string
  value: number
  tone: 'success' | 'warning' | 'destructive'
}) {
  return (
    <div
      className={cn(
        'bg-background/88 px-3 py-3 text-xs dark:bg-card/86',
        tone === 'success' && 'text-success',
        tone === 'warning' && 'text-warning',
        tone === 'destructive' && 'text-destructive',
      )}
    >
      <div className="font-medium">{label}</div>
      <div className="mt-0.5 text-[11px]">{value}</div>
    </div>
  )
}
