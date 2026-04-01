import { type ChangeEvent, useEffect, useMemo, useRef, useState } from "react";
import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import {
  Button,
  Chip,
  Divider,
  Input,
  Pagination,
  Progress,
  Select,
  SelectItem,
  Spinner,
  Table,
  TableBody,
  TableCell,
  TableColumn,
  TableHeader,
  TableRow,
  type Selection,
} from "@heroui/react";
import {
  AlertTriangle,
  Clock3,
  FileClock,
  Pause,
  Play,
  RefreshCcw,
  Search,
  Upload,
  XCircle,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  importJobsApi,
  type OAuthImportAdmissionStatus,
  type OAuthImportCredentialMode,
  type OAuthImportItemStatus,
  type OAuthImportJobItem,
  type OAuthImportJobSummary,
} from "@/api/importJobs";
import { localizeApiErrorDisplay } from "@/api/errorI18n";
import {
  DockedPageIntro,
  PageContent,
} from "@/components/layout/page-archetypes";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import {
  loadRecentJobIds,
  MAX_RECENT_IMPORT_JOB_IDS,
  mergeRecentJobIds,
  RECENT_IMPORT_JOB_IDS_STORAGE_KEY,
  sortJobSummaries,
} from "@/features/import-jobs/contracts";
import {
  calcProgress,
  getEtaLabel,
  getImportStatusFilterOptions,
  getImportStatusLabel,
} from "@/features/import-jobs/utils";
import { notify } from "@/lib/notification";
import { cn } from "@/lib/utils";

const JOB_ITEMS_PAGE_SIZE = 200;
const TABLE_PAGE_SIZE_OPTIONS = [10, 20, 50];

type AdmissionFilter = "all" | OAuthImportAdmissionStatus;
type ItemStatusFilter = "all" | OAuthImportItemStatus;

function normalizeSelection(selection: Selection) {
  if (selection === "all") {
    return "";
  }

  const [first] = Array.from(selection);
  return first === undefined ? "" : String(first);
}

function formatDateTime(value?: string) {
  if (!value) {
    return "-";
  }
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? "-" : parsed.toLocaleString();
}

function getJobStatusColor(status: OAuthImportJobSummary["status"]) {
  switch (status) {
    case "completed":
      return "success" as const;
    case "running":
      return "primary" as const;
    case "paused":
      return "warning" as const;
    case "queued":
      return "secondary" as const;
    case "failed":
    case "cancelled":
      return "danger" as const;
    default:
      return "default" as const;
  }
}

function getItemStatusColor(status: OAuthImportJobItem["status"]) {
  switch (status) {
    case "created":
    case "updated":
      return "success" as const;
    case "processing":
    case "pending":
      return "warning" as const;
    case "failed":
      return "danger" as const;
    case "cancelled":
      return "default" as const;
    case "skipped":
    default:
      return "secondary" as const;
  }
}

function getAdmissionColor(status?: OAuthImportAdmissionStatus) {
  switch (status) {
    case "ready":
      return "success" as const;
    case "needs_refresh":
    case "queued":
      return "warning" as const;
    case "no_quota":
    case "failed":
      return "danger" as const;
    default:
      return "default" as const;
  }
}

function getAdmissionLabel(
  status: OAuthImportAdmissionStatus | undefined,
  t: ReturnType<typeof useTranslation>["t"],
) {
  switch (status) {
    case "ready":
      return t("importJobs.admission.status.ready");
    case "needs_refresh":
      return t("importJobs.admission.status.needsRefresh");
    case "no_quota":
      return t("importJobs.admission.status.noQuota");
    case "failed":
      return t("importJobs.admission.status.failed");
    case "queued":
      return t("importJobs.admission.status.queued");
    default:
      return t("importJobs.admission.status.unknown");
  }
}

function getFailureStageLabel(
  stage: OAuthImportJobItem["failure_stage"],
  t: ReturnType<typeof useTranslation>["t"],
) {
  switch (stage) {
    case "admission_probe":
      return t("importJobs.admission.failureStage.admissionProbe");
    case "activation_refresh":
      return t("importJobs.admission.failureStage.activationRefresh");
    case "activation_rate_limits":
      return t("importJobs.admission.failureStage.activationRateLimits");
    case "runtime_refresh":
      return t("importJobs.admission.failureStage.runtimeRefresh");
    default:
      return "-";
  }
}

function normalizeImportErrorCode(code: string | undefined | null) {
  return (code ?? "").trim().toLowerCase();
}

function localizeImportErrorCode(
  errorCode: string | undefined | null,
  t: ReturnType<typeof useTranslation>["t"],
) {
  switch (normalizeImportErrorCode(errorCode)) {
    case "invalid_record":
      return t("importJobs.errors.invalidRecord");
    case "missing_access_token":
      return t("importJobs.errors.missingAccessToken");
    case "missing_refresh_token":
      return t("importJobs.errors.missingRefreshToken");
    case "missing_credentials":
      return t("importJobs.errors.missingCredentials");
    case "refresh_token_reused":
      return t("importJobs.errors.refreshTokenReused");
    case "invalid_refresh_token":
      return t("importJobs.errors.invalidRefreshToken");
    case "oauth_provider_not_configured":
      return t("importJobs.errors.oauthProviderNotConfigured");
    case "rate_limited":
      return t("importJobs.errors.rateLimited");
    case "upstream_network_error":
      return t("importJobs.errors.upstreamNetworkError");
    case "upstream_unavailable":
      return t("importJobs.errors.upstreamUnavailable");
    case "import_failed":
      return t("importJobs.errors.importFailed");
    default:
      return t("importJobs.errors.unknown");
  }
}

function resolveImportIssueLabel(
  item: OAuthImportJobItem,
  t: ReturnType<typeof useTranslation>["t"],
) {
  for (const candidate of [item.error_code, item.terminal_reason, item.admission_reason]) {
    if (!normalizeImportErrorCode(candidate)) {
      continue
    }

    const localized = localizeImportErrorCode(candidate, t)
    if (localized !== t("importJobs.errors.unknown")) {
      return localized
    }
  }

  return t("importJobs.errors.unknown")
}

async function fetchAllJobItems(jobId: string) {
  const items: OAuthImportJobItem[] = [];
  let cursor: number | undefined;
  let pages = 0;

  do {
    const response = await importJobsApi.getJobItems(jobId, {
      cursor,
      limit: JOB_ITEMS_PAGE_SIZE,
    });
    items.push(...response.items);
    cursor = response.next_cursor;
    pages += 1;
  } while (cursor !== undefined && pages < 20);

  return items;
}

export default function ImportJobs() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [credentialMode, setCredentialMode] =
    useState<OAuthImportCredentialMode>("refresh_token");
  const [recentJobIds, setRecentJobIds] = useState<string[]>(() =>
    loadRecentJobIds(localStorage.getItem(RECENT_IMPORT_JOB_IDS_STORAGE_KEY)),
  );
  const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
  const [admissionFilter, setAdmissionFilter] =
    useState<AdmissionFilter>("all");
  const [itemStatusFilter, setItemStatusFilter] =
    useState<ItemStatusFilter>("all");
  const [searchValue, setSearchValue] = useState("");
  const [currentPage, setCurrentPage] = useState(1);
  const [rowsPerPage, setRowsPerPage] = useState(10);

  const uploadMutation = useMutation({
    mutationFn: (files: File[]) =>
      importJobsApi.createJob(files, { credential_mode: credentialMode }),
    onSuccess: (summary) => {
      setRecentJobIds((current) =>
        mergeRecentJobIds(current, summary.job_id, MAX_RECENT_IMPORT_JOB_IDS),
      );
      setCurrentPage(1);
      setSelectedJobId(summary.job_id);
    },
    onError: (error) => {
      const fallback = t("importJobs.messages.uploadFailedTitle");
      const description = localizeApiErrorDisplay(t, error, fallback).label;
      notify({
        variant: "error",
        title: fallback,
        description: description !== fallback ? description : undefined,
      });
    },
  });

  const pauseMutation = useMutation({
    mutationFn: (jobId: string) => importJobsApi.pauseJob(jobId),
    onSuccess: (_, jobId) => {
      queryClient.invalidateQueries({ queryKey: ["importJobSummary", jobId] });
    },
  });

  const resumeMutation = useMutation({
    mutationFn: (jobId: string) => importJobsApi.resumeJob(jobId),
    onSuccess: (_, jobId) => {
      queryClient.invalidateQueries({ queryKey: ["importJobSummary", jobId] });
    },
  });

  const retryMutation = useMutation({
    mutationFn: (jobId: string) => importJobsApi.retryFailed(jobId),
    onSuccess: (_, jobId) => {
      queryClient.invalidateQueries({ queryKey: ["importJobSummary", jobId] });
      queryClient.invalidateQueries({ queryKey: ["importJobItems", jobId] });
    },
  });

  const cancelMutation = useMutation({
    mutationFn: (jobId: string) => importJobsApi.cancelJob(jobId),
    onSuccess: (_, jobId) => {
      queryClient.invalidateQueries({ queryKey: ["importJobSummary", jobId] });
    },
  });

  useEffect(() => {
    localStorage.setItem(
      RECENT_IMPORT_JOB_IDS_STORAGE_KEY,
      JSON.stringify(recentJobIds),
    );
  }, [recentJobIds]);

  const jobQueries = useQueries({
    queries: recentJobIds.map((jobId) => ({
      queryKey: ["importJobSummary", jobId],
      queryFn: () => importJobsApi.getJobSummary(jobId),
      refetchInterval: 4_000,
    })),
  });

  const jobs = useMemo(
    () => sortJobSummaries(jobQueries.map((query) => query.data ?? null)),
    [jobQueries],
  );

  const effectiveSelectedJobId = selectedJobId ?? jobs[0]?.job_id ?? null;

  const selectedSummary = useMemo(
    () => jobs.find((job) => job.job_id === effectiveSelectedJobId) ?? null,
    [effectiveSelectedJobId, jobs],
  );
  const errorSummary = selectedSummary?.error_summary ?? [];
  const admissionCounts = selectedSummary?.admission_counts ?? {
    ready: 0,
    needs_refresh: 0,
    no_quota: 0,
    failed: 0,
  };

  const selectedItemsQuery = useQuery({
    queryKey: ["importJobItems", effectiveSelectedJobId],
    queryFn: () => fetchAllJobItems(effectiveSelectedJobId!),
    enabled: Boolean(effectiveSelectedJobId),
    refetchInterval:
      selectedSummary?.status === "running" ||
      selectedSummary?.status === "queued"
        ? 5_000
        : false,
  });
  const isFetching =
    jobQueries.some((query) => query.isFetching) ||
    selectedItemsQuery.isFetching;

  const importStatusOptions = useMemo(
    () => getImportStatusFilterOptions(t),
    [t],
  );
  const itemStatusSelectOptions = useMemo(
    () => [
      {
        value: "all" as ItemStatusFilter,
        label: t("importJobs.detail.filters.allStatuses"),
      },
      ...importStatusOptions
        .filter((option) => option.value !== "all")
        .map((option) => ({
          value: option.value as ItemStatusFilter,
          label: option.label,
        })),
    ],
    [importStatusOptions, t],
  );

  const filteredItems = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase();

    return (selectedItemsQuery.data ?? []).filter((item) => {
      if (
        admissionFilter !== "all" &&
        item.admission_status !== admissionFilter
      ) {
        return false;
      }

      if (itemStatusFilter !== "all" && item.status !== itemStatusFilter) {
        return false;
      }

      if (!keyword) {
        return true;
      }

      return `${item.label} ${item.email ?? ""} ${item.chatgpt_account_id ?? ""} ${item.account_id ?? ""} ${item.error_code ?? ""} ${item.error_message ?? ""}`
        .toLowerCase()
        .includes(keyword);
    });
  }, [admissionFilter, itemStatusFilter, searchValue, selectedItemsQuery.data]);

  const totalPages = Math.max(1, Math.ceil(filteredItems.length / rowsPerPage));
  const resolvedPage = Math.min(currentPage, totalPages);
  const paginatedItems = useMemo(() => {
    const start = (resolvedPage - 1) * rowsPerPage;
    return filteredItems.slice(start, start + rowsPerPage);
  }, [filteredItems, resolvedPage, rowsPerPage]);
  const visibleRangeStart =
    filteredItems.length === 0 ? 0 : (resolvedPage - 1) * rowsPerPage + 1;
  const visibleRangeEnd =
    filteredItems.length === 0
      ? 0
      : Math.min(filteredItems.length, resolvedPage * rowsPerPage);

  const selectedProgress = calcProgress(selectedSummary ?? undefined);
  const itemsError = selectedItemsQuery.error
    ? localizeApiErrorDisplay(
        t,
        selectedItemsQuery.error,
        t("importJobs.messages.queryFailed"),
      ).label
    : null;

  const handleBrowseClick = () => fileInputRef.current?.click();
  const handleRefreshJobs = async () => {
    await queryClient.invalidateQueries({ queryKey: ["importJobSummary"] });
    if (effectiveSelectedJobId) {
      await queryClient.invalidateQueries({
        queryKey: ["importJobItems", effectiveSelectedJobId],
      });
    }
  };

  const handleFileSelection = (event: ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(event.target.files ?? []);
    if (files.length === 0) {
      return;
    }
    uploadMutation.mutate(files);
    event.target.value = "";
  };

  const handleRefreshSelectedJob = async () => {
    if (!effectiveSelectedJobId) {
      return;
    }

    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: ["importJobSummary", effectiveSelectedJobId],
      }),
      queryClient.invalidateQueries({
        queryKey: ["importJobItems", effectiveSelectedJobId],
      }),
    ]);
  };

  const summaryTiles = selectedSummary
    ? [
        {
          key: "created",
          title: t("importJobs.metrics.created"),
          value: String(selectedSummary.created_count),
          description: t("importJobs.metrics.createdDesc"),
        },
        {
          key: "updated",
          title: t("importJobs.metrics.updated"),
          value: String(selectedSummary.updated_count),
          description: t("importJobs.metrics.updatedDesc"),
        },
        {
          key: "failed",
          title: t("importJobs.metrics.failed"),
          value: String(selectedSummary.failed_count),
          description: t("importJobs.metrics.failedDesc"),
        },
        {
          key: "throughput",
          title: t("importJobs.metrics.throughput"),
          value: String(Math.round(selectedSummary.throughput_per_min ?? 0)),
          description: t("importJobs.metrics.throughputDesc"),
        },
      ]
    : [];

  const admissionTiles = selectedSummary
    ? [
        {
          key: "ready",
          title: t("importJobs.admission.ready"),
          value: String(admissionCounts.ready),
          description: t("importJobs.admission.readyDesc"),
          color: "success" as const,
        },
        {
          key: "needs-refresh",
          title: t("importJobs.admission.needsRefresh"),
          value: String(admissionCounts.needs_refresh),
          description: t("importJobs.admission.needsRefreshDesc"),
          color: "warning" as const,
        },
        {
          key: "no-quota",
          title: t("importJobs.admission.noQuota"),
          value: String(admissionCounts.no_quota),
          description: t("importJobs.admission.noQuotaDesc"),
          color: "danger" as const,
        },
        {
          key: "failed",
          title: t("importJobs.admission.failed"),
          value: String(admissionCounts.failed),
          description: t("importJobs.admission.failedDesc"),
          color: "secondary" as const,
        },
      ]
    : [];

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t("nav.importJobs")}
        description={t("importJobs.description")}
        actions={
          <Button
            isLoading={isFetching}
            startContent={
              isFetching ? undefined : <RefreshCcw className="h-4 w-4" />
            }
            variant="light"
            onPress={() => {
              void handleRefreshJobs();
            }}
          >
            {t("common.refresh")}
          </Button>
        }
      />

      <input
        ref={fileInputRef}
        type="file"
        accept=".json,.jsonl"
        multiple
        className="hidden"
        onChange={handleFileSelection}
      />

      <div className="grid gap-6 xl:grid-cols-[minmax(0,22rem)_minmax(0,1fr)]">
        <div className="space-y-6">
          <Card>
            <CardHeader className="px-5 pb-3 pt-5">
              <div className="space-y-1">
                <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {t("importJobs.antigravity.intakeTitle")}
                </h2>
                <p className="text-sm leading-6 text-default-600">
                  {t("importJobs.antigravity.intakeDescription")}
                </p>
              </div>
            </CardHeader>
            <CardBody className="gap-4 px-5 pb-5 pt-1">
              <div className="flex flex-wrap items-center gap-2">
                <Chip color="primary" variant="flat">
                  {credentialMode === "refresh_token"
                    ? t("importJobs.credentialMode.refreshToken")
                    : t("importJobs.credentialMode.accessToken")}
                </Chip>
                <Chip variant="flat">{t("importJobs.queue.tracked")}</Chip>
              </div>

              <Select
                aria-label={t("importJobs.credentialMode.title")}
                label={t("importJobs.credentialMode.title")}
                labelPlacement="outside"
                selectedKeys={[credentialMode]}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection);
                  if (nextValue) {
                    setCredentialMode(nextValue as OAuthImportCredentialMode);
                  }
                }}
              >
                <SelectItem
                  key="refresh_token"
                  textValue={t("importJobs.credentialMode.refreshToken")}
                >
                  {t("importJobs.credentialMode.refreshToken")}
                </SelectItem>
                <SelectItem
                  key="access_token"
                  textValue={t("importJobs.credentialMode.accessToken")}
                >
                  {t("importJobs.credentialMode.accessToken")}
                </SelectItem>
              </Select>

              <p className="text-sm leading-6 text-default-600">
                {t("importJobs.antigravity.intakeHint")}
              </p>

              <div className="flex flex-wrap items-center gap-3">
                <Button
                  color="primary"
                  isLoading={uploadMutation.isPending}
                  startContent={<Upload className="h-4 w-4" />}
                  onPress={handleBrowseClick}
                >
                  {t("importJobs.dropzone.selectFiles")}
                </Button>
                <span className="text-xs text-default-500">
                  {t("importJobs.dropzone.acceptsNew")}
                </span>
              </div>

            </CardBody>
          </Card>

          <Card className="overflow-hidden">
            <CardHeader className="flex items-start justify-between gap-3 px-5 pb-3 pt-5">
              <div className="space-y-1">
                <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {t("importJobs.queue.title")}
                </h2>
                <p className="text-sm leading-6 text-default-600">
                  {t("importJobs.queue.description")}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <Chip size="sm" variant="flat">
                  {jobs.length}
                </Chip>
                <Button
                  aria-label={t("common.refresh")}
                  isIconOnly
                  size="sm"
                  variant="light"
                  onPress={() => {
                    void handleRefreshJobs();
                  }}
                >
                  <RefreshCcw className="h-4 w-4" />
                </Button>
              </div>
            </CardHeader>
            <Divider />
            <CardBody className="p-0">
              {jobs.length === 0 ? (
                <div className="flex flex-col items-center gap-3 px-5 py-12 text-center">
                  <FileClock className="h-10 w-10 text-default-300" />
                  <div className="space-y-1">
                    <div className="text-sm font-medium text-foreground">
                      {t("importJobs.queue.empty")}
                    </div>
                    <div className="text-xs leading-5 text-default-500">
                      {t("importJobs.queue.description")}
                    </div>
                  </div>
                </div>
              ) : (
                <div className="divide-y divide-default-200">
                  {jobs.map((job) => {
                    const progress = calcProgress(job);
                    const selected = job.job_id === effectiveSelectedJobId;

                    return (
                      <button
                        key={job.job_id}
                        type="button"
                        className={cn(
                          "w-full space-y-3 px-5 py-4 text-left transition-colors",
                          selected ? "bg-primary/8" : "hover:bg-content2/60",
                        )}
                        onClick={() => {
                          setCurrentPage(1);
                          setSelectedJobId(job.job_id);
                        }}
                      >
                        <div className="flex items-start justify-between gap-3">
                          <div className="min-w-0">
                            <div className="truncate font-mono text-xs text-foreground">
                              {job.job_id}
                            </div>
                            <div className="mt-1 text-xs text-default-500">
                              {formatDateTime(job.created_at)}
                            </div>
                          </div>
                          <Chip
                            color={getJobStatusColor(job.status)}
                            size="sm"
                            variant="flat"
                          >
                            {getImportStatusLabel(t, job.status)}
                          </Chip>
                        </div>

                        <div className="space-y-2">
                          <div className="flex items-center justify-between gap-3 text-xs text-default-500">
                            <span>
                              {job.processed}/{job.total}
                            </span>
                            <span>{Math.round(progress)}%</span>
                          </div>
                          <Progress
                            aria-label={job.job_id}
                            color={
                              getJobStatusColor(job.status) === "secondary"
                                ? "primary"
                                : getJobStatusColor(job.status)
                            }
                            size="sm"
                            value={progress}
                          />
                        </div>
                      </button>
                    );
                  })}
                </div>
              )}
            </CardBody>
          </Card>
        </div>

        <div className="space-y-6">
          <Card>
            <CardHeader className="flex flex-col items-start justify-between gap-4 px-5 pb-3 pt-5 lg:flex-row">
              <div className="space-y-1">
                <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {selectedSummary?.job_id ?? t("importJobs.detail.title")}
                </h2>
                <p className="text-sm leading-6 text-default-600">
                  {selectedSummary
                    ? t("importJobs.detail.description")
                    : t("importJobs.detail.selectHint")}
                </p>
              </div>

              {selectedSummary ? (
                <div className="flex flex-wrap gap-2">
                  <Button
                    size="sm"
                    startContent={<RefreshCcw className="h-4 w-4" />}
                    variant="light"
                    onPress={handleRefreshSelectedJob}
                  >
                    {t("importJobs.actions.refreshItems")}
                  </Button>
                  {selectedSummary.status === "failed" ? (
                    <Button
                      color="warning"
                      isLoading={retryMutation.isPending}
                      size="sm"
                      startContent={<RefreshCcw className="h-4 w-4" />}
                      variant="flat"
                      onPress={() =>
                        retryMutation.mutate(selectedSummary.job_id)
                      }
                    >
                      {t("importJobs.actions.retryFailed")}
                    </Button>
                  ) : null}
                  {selectedSummary.status === "paused" ? (
                    <Button
                      color="primary"
                      isLoading={resumeMutation.isPending}
                      size="sm"
                      startContent={<Play className="h-4 w-4" />}
                      variant="flat"
                      onPress={() =>
                        resumeMutation.mutate(selectedSummary.job_id)
                      }
                    >
                      {t("importJobs.antigravity.resume")}
                    </Button>
                  ) : null}
                  {selectedSummary.status === "queued" ||
                  selectedSummary.status === "running" ? (
                    <Button
                      color="warning"
                      isLoading={pauseMutation.isPending}
                      size="sm"
                      startContent={<Pause className="h-4 w-4" />}
                      variant="flat"
                      onPress={() =>
                        pauseMutation.mutate(selectedSummary.job_id)
                      }
                    >
                      {t("importJobs.antigravity.pause")}
                    </Button>
                  ) : null}
                  {selectedSummary.status === "queued" ||
                  selectedSummary.status === "running" ||
                  selectedSummary.status === "paused" ? (
                    <Button
                      color="danger"
                      isLoading={cancelMutation.isPending}
                      size="sm"
                      startContent={<XCircle className="h-4 w-4" />}
                      variant="flat"
                      onPress={() =>
                        cancelMutation.mutate(selectedSummary.job_id)
                      }
                    >
                      {t("importJobs.actions.cancel")}
                    </Button>
                  ) : null}
                </div>
              ) : null}
            </CardHeader>

            <CardBody className="gap-5 px-5 pb-5 pt-1">
              {selectedSummary ? (
                <>
                  <div className="flex flex-wrap items-center gap-2">
                    <Chip size="sm" variant="flat">
                      {formatDateTime(selectedSummary.created_at)}
                    </Chip>
                    <Chip
                      color={getJobStatusColor(selectedSummary.status)}
                      size="sm"
                      variant="flat"
                    >
                      {getImportStatusLabel(t, selectedSummary.status)}
                    </Chip>
                    <Chip
                      size="sm"
                      startContent={<Clock3 className="h-3.5 w-3.5" />}
                      variant="flat"
                    >
                      {getEtaLabel(selectedSummary, t)}
                    </Chip>
                  </div>

                  <div className="space-y-2">
                    <div className="flex items-center justify-between gap-3 text-xs text-default-500">
                      <span>
                        {selectedSummary.processed}/{selectedSummary.total}
                      </span>
                      <span>{Math.round(selectedProgress)}%</span>
                    </div>
                    <Progress
                      aria-label={selectedSummary.job_id}
                      color={
                        selectedSummary.status === "failed"
                          ? "danger"
                          : "primary"
                      }
                      size="sm"
                      value={selectedProgress}
                    />
                  </div>

                  <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
                    {summaryTiles.map((tile) => (
                      <div
                        key={tile.key}
                        className="rounded-large border border-default-200 bg-content2/55 px-4 py-4"
                      >
                        <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                          {tile.title}
                        </div>
                        <div className="mt-2 text-3xl font-semibold tracking-[-0.04em] text-foreground">
                          {tile.value}
                        </div>
                        <p className="mt-2 text-xs leading-5 text-default-500">
                          {tile.description}
                        </p>
                      </div>
                    ))}
                  </div>

                  <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
                    {admissionTiles.map((tile) => (
                      <div
                        key={tile.key}
                        className="rounded-large border border-default-200 bg-content2/55 px-4 py-4"
                      >
                        <div className="flex items-center justify-between gap-3">
                          <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                            {tile.title}
                          </div>
                          <Chip color={tile.color} size="sm" variant="flat">
                            {tile.value}
                          </Chip>
                        </div>
                        <p className="mt-3 text-xs leading-5 text-default-500">
                          {tile.description}
                        </p>
                      </div>
                    ))}
                  </div>

                  {errorSummary.length > 0 ? (
                    <div className="rounded-large border border-warning/20 bg-warning/5 px-4 py-4">
                      <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                        <AlertTriangle className="h-4 w-4 text-warning" />
                        {t("importJobs.progress.topErrors")}
                      </div>
                      <div className="mt-3 grid gap-2 md:grid-cols-2">
                        {errorSummary
                          .slice(0, 4)
                          .map((entry) => (
                            <div
                              key={`${entry.error_code}-${entry.count}`}
                              className="flex items-center justify-between gap-3 rounded-medium border border-default-200 bg-content1/80 px-3 py-2 text-sm"
                            >
                              <span className="truncate text-default-700 dark:text-default-300">
                                {localizeImportErrorCode(entry.error_code, t)}
                              </span>
                              <Chip size="sm" variant="flat">
                                {entry.count}
                              </Chip>
                            </div>
                          ))}
                      </div>
                    </div>
                  ) : null}
                </>
              ) : (
                <div className="flex min-h-[260px] flex-col items-center justify-center gap-3 rounded-large border border-dashed border-default-200 bg-content2/35 px-6 py-10 text-center">
                  <FileClock className="h-10 w-10 text-default-300" />
                  <div className="space-y-1">
                    <div className="text-sm font-medium text-foreground">
                      {t("importJobs.detail.title")}
                    </div>
                    <div className="text-sm leading-6 text-default-600">
                      {t("importJobs.detail.selectHint")}
                    </div>
                  </div>
                </div>
              )}
            </CardBody>
          </Card>

          <Card>
            <CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5 lg:flex-row lg:justify-between">
              <div className="space-y-1">
                <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {t("importJobs.detail.itemsTitle")}
                </h2>
                <p className="text-sm leading-6 text-default-600">
                  {selectedSummary
                    ? t("importJobs.detail.itemsDescription")
                    : t("importJobs.detail.selectHint")}
                </p>
              </div>

              <div className="flex flex-wrap gap-2">
                <Chip size="sm" variant="flat">
                  {filteredItems.length}
                </Chip>
                {selectedSummary ? (
                  <Chip
                    color={getJobStatusColor(selectedSummary.status)}
                    size="sm"
                    variant="flat"
                  >
                    {getImportStatusLabel(t, selectedSummary.status)}
                  </Chip>
                ) : null}
              </div>
            </CardHeader>

            <CardBody className="gap-4 px-5 pb-5 pt-1">
              <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
                <div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:flex-wrap">
                  <Input
                    aria-label={t("common.table.searchLabel")}
                    className="w-full sm:w-[280px]"
                    isClearable
                    placeholder={t("importJobs.detail.searchPlaceholderModern")}
                    startContent={
                      <Search className="h-4 w-4 text-default-400" />
                    }
                    value={searchValue}
                    onClear={() => {
                      setCurrentPage(1);
                      setSearchValue("");
                    }}
                    onValueChange={(value) => {
                      setCurrentPage(1);
                      setSearchValue(value);
                    }}
                  />
                  <Select
                    aria-label={t("importJobs.detail.filters.status")}
                    className="w-full sm:w-[180px]"
                    selectedKeys={[itemStatusFilter]}
                    size="sm"
                    onSelectionChange={(selection) => {
                      const nextValue = normalizeSelection(selection);
                      if (nextValue) {
                        setCurrentPage(1);
                        setItemStatusFilter(nextValue as ItemStatusFilter);
                      }
                    }}
                  >
                    {itemStatusSelectOptions.map((option) => (
                      <SelectItem key={option.value} textValue={option.label}>
                        {option.label}
                      </SelectItem>
                    ))}
                  </Select>
                  <Select
                    aria-label={t("importJobs.detail.filters.admission")}
                    className="w-full sm:w-[190px]"
                    selectedKeys={[admissionFilter]}
                    size="sm"
                    onSelectionChange={(selection) => {
                      const nextValue = normalizeSelection(selection);
                      if (nextValue) {
                        setCurrentPage(1);
                        setAdmissionFilter(nextValue as AdmissionFilter);
                      }
                    }}
                  >
                    <SelectItem
                      key="all"
                      textValue={t("importJobs.detail.filters.allAdmissions")}
                    >
                      {t("importJobs.detail.filters.allAdmissions")}
                    </SelectItem>
                    <SelectItem
                      key="queued"
                      textValue={getAdmissionLabel("queued", t)}
                    >
                      {getAdmissionLabel("queued", t)}
                    </SelectItem>
                    <SelectItem
                      key="ready"
                      textValue={getAdmissionLabel("ready", t)}
                    >
                      {getAdmissionLabel("ready", t)}
                    </SelectItem>
                    <SelectItem
                      key="needs_refresh"
                      textValue={getAdmissionLabel("needs_refresh", t)}
                    >
                      {getAdmissionLabel("needs_refresh", t)}
                    </SelectItem>
                    <SelectItem
                      key="no_quota"
                      textValue={getAdmissionLabel("no_quota", t)}
                    >
                      {getAdmissionLabel("no_quota", t)}
                    </SelectItem>
                    <SelectItem
                      key="failed"
                      textValue={getAdmissionLabel("failed", t)}
                    >
                      {getAdmissionLabel("failed", t)}
                    </SelectItem>
                  </Select>
                </div>

                <div className="flex items-center gap-2 text-xs text-default-500">
                  <span>{t("common.table.rowsPerPage")}</span>
                  <Select
                    aria-label={t("common.table.rowsPerPage")}
                    className="w-[106px]"
                    selectedKeys={[String(rowsPerPage)]}
                    size="sm"
                    onSelectionChange={(selection) => {
                      const nextValue = normalizeSelection(selection);
                      if (!nextValue) {
                        return;
                      }
                      setCurrentPage(1);
                      setRowsPerPage(Number(nextValue));
                    }}
                  >
                    {TABLE_PAGE_SIZE_OPTIONS.map((size) => (
                      <SelectItem key={String(size)}>{size}</SelectItem>
                    ))}
                  </Select>
                </div>
              </div>

              {itemsError ? (
                <div className="rounded-large border border-danger/20 bg-danger/8 px-4 py-3 text-sm text-danger-700 dark:text-danger-300">
                  {itemsError}
                </div>
              ) : null}

              <Table
                isHeaderSticky
                aria-label={t("importJobs.detail.itemsTitle")}
                classNames={{
                  base: "min-h-[24rem]",
                  wrapper: "bg-transparent px-0 py-0 shadow-none",
                  th: "bg-default-100/60 text-xs font-semibold uppercase tracking-[0.12em] text-default-500",
                  td: "align-top py-4 text-sm text-foreground",
                  tr: "data-[hover=true]:bg-content2/35 transition-colors",
                  emptyWrapper: "h-56",
                }}
              >
                <TableHeader>
                  <TableColumn>
                    {t("importJobs.detail.columns.source")}
                  </TableColumn>
                  <TableColumn>
                    {t("importJobs.detail.columns.status")}
                  </TableColumn>
                  <TableColumn>
                    {t("importJobs.detail.columns.admission")}
                  </TableColumn>
                  <TableColumn>
                    {t("importJobs.detail.columns.failure")}
                  </TableColumn>
                  <TableColumn>
                    {t("importJobs.detail.columns.retry")}
                  </TableColumn>
                  <TableColumn>
                    {t("importJobs.detail.columns.message")}
                  </TableColumn>
                </TableHeader>
                <TableBody
                  emptyContent={
                    <div className="flex flex-col items-center gap-2 py-12 text-default-500">
                      <FileClock className="h-10 w-10 opacity-35" />
                      <p className="text-sm font-medium">
                        {effectiveSelectedJobId
                          ? t("importJobs.detail.itemsEmpty")
                          : t("importJobs.detail.selectHint")}
                      </p>
                    </div>
                  }
                  isLoading={selectedItemsQuery.isLoading}
                  items={paginatedItems}
                  loadingContent={
                    <Spinner label={t("importJobs.detail.itemsLoading")} />
                  }
                >
                  {(item) => {
                    const messageTitle = resolveImportIssueLabel(item, t);

                    return (
                      <TableRow key={item.item_id}>
                        <TableCell>
                          <div className="min-w-[220px] space-y-1">
                            <div className="font-medium text-foreground">
                              {item.label}
                            </div>
                            <div className="text-xs text-default-500">
                              {item.source_file}:{item.line_no}
                            </div>
                            {item.email ? (
                              <div className="text-xs text-default-500">
                                {item.email}
                              </div>
                            ) : null}
                            {item.chatgpt_account_id ? (
                              <div className="text-xs text-default-500">
                                {item.chatgpt_account_id}
                              </div>
                            ) : null}
                          </div>
                        </TableCell>
                        <TableCell>
                          <Chip
                            color={getItemStatusColor(item.status)}
                            size="sm"
                            variant="flat"
                          >
                            {getImportStatusLabel(t, item.status)}
                          </Chip>
                        </TableCell>
                        <TableCell>
                          <div className="min-w-[180px]">
                            <Chip
                              color={getAdmissionColor(item.admission_status)}
                              size="sm"
                              variant="flat"
                            >
                              {getAdmissionLabel(item.admission_status, t)}
                            </Chip>
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="min-w-[160px] text-xs leading-5 text-default-500">
                            {getFailureStageLabel(item.failure_stage, t)}
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="min-w-[150px] space-y-1 text-xs leading-5 text-default-500">
                            <div>
                              {item.retryable
                                ? t("common.yes")
                                : t("common.no")}
                            </div>
                            <div>{formatDateTime(item.next_retry_at)}</div>
                          </div>
                        </TableCell>
                        <TableCell>
                          <div className="min-w-[220px] text-xs leading-5 text-default-500">
                            <div className="text-sm text-foreground">
                              {messageTitle}
                            </div>
                          </div>
                        </TableCell>
                      </TableRow>
                    );
                  }}
                </TableBody>
              </Table>

              <div className="flex flex-col gap-3 border-t border-default-200 pt-3 text-xs text-default-500 sm:flex-row sm:items-center sm:justify-between">
                <div className="tabular-nums">
                  {t("common.table.range", {
                    end: visibleRangeEnd,
                    start: visibleRangeStart,
                    total: filteredItems.length,
                  })}
                </div>
                <Pagination
                  color="primary"
                  isCompact
                  page={resolvedPage}
                  total={totalPages}
                  onChange={setCurrentPage}
                />
              </div>
            </CardBody>
          </Card>
        </div>
      </div>
    </PageContent>
  );
}
