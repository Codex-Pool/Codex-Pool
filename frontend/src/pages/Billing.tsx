import { useMemo, useState } from "react";
import {
  Button,
  Chip,
  Input,
  Pagination,
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
import { useQuery } from "@tanstack/react-query";
import {
  Building2,
  CreditCard,
  RefreshCcw,
  Search,
  Wallet,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate, useSearchParams } from "react-router-dom";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { billingApi } from "@/api/billing";
import { DEFAULT_SYSTEM_CAPABILITIES, systemApi } from "@/api/system";
import { tenantsApi } from "@/api/tenants";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import {
  DockedPageIntro,
  PageContent,
} from "@/components/layout/page-archetypes";
import { useChartTheme } from "@/lib/chart-theme";
import { AdminCostReportPage } from "@/features/billing/admin-cost-report";
import {
  buildBillingTrendPoints,
  microcreditsToCredits,
  resolveActiveTenantId,
  shouldUseCostReportBilling,
} from "@/features/billing/contracts";

const TABLE_PAGE_SIZE_OPTIONS = [10, 20, 50];

function normalizeSelection(selection: Selection) {
  if (selection === "all") {
    return "";
  }

  const [first] = Array.from(selection);
  return first === undefined ? "" : String(first);
}

function formatCredits(microcredits: number | undefined): string {
  return microcreditsToCredits(microcredits).toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 4,
  });
}

function formatDateTime(value?: string | null) {
  if (!value) {
    return "-";
  }

  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? "-" : parsed.toLocaleString();
}

function mapLedgerEventType(
  t: ReturnType<typeof useTranslation>["t"],
  eventType: string,
): string {
  switch (eventType) {
    case "usage":
      return t("billing.antigravity.eventType.usage");
    case "admin_recharge":
      return t("billing.antigravity.eventType.adminRecharge");
    default:
      return t("billing.antigravity.eventType.unknown");
  }
}

function matchLedgerSearch(
  keyword: string,
  row: {
    event_type: string;
    request_id?: string;
    api_key_id?: string;
    model?: string;
  },
) {
  return [row.event_type, row.request_id, row.api_key_id, row.model]
    .filter(Boolean)
    .join(" ")
    .toLowerCase()
    .includes(keyword);
}

export default function Billing() {
  const { data: capabilities = DEFAULT_SYSTEM_CAPABILITIES } = useQuery({
    queryKey: ["systemCapabilities"],
    queryFn: () => systemApi.getCapabilities(),
    staleTime: 5 * 60_000,
  });

  if (shouldUseCostReportBilling(capabilities)) {
    return <AdminCostReportPage capabilities={capabilities} />;
  }

  return <CreditBillingPage />;
}

function CreditBillingPage() {
  const { t } = useTranslation();
  const { textColor: chartTextColor, gridColor: chartGridColor, tooltipStyle: chartTooltipStyle } = useChartTheme();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [searchValue, setSearchValue] = useState("");
  const [rowsPerPage, setRowsPerPage] = useState(10);
  const [currentPage, setCurrentPage] = useState(1);

  const {
    data: tenants = [],
    isLoading: isTenantsLoading,
    isFetching: isFetchingTenants,
    refetch: refetchTenants,
  } = useQuery({
    queryKey: ["billing-tenants"],
    queryFn: tenantsApi.list,
  });

  const activeTenantId = useMemo(
    () => resolveActiveTenantId(tenants, searchParams.get("tenant")),
    [tenants, searchParams],
  );

  const {
    data: summary,
    isLoading: isSummaryLoading,
    isFetching: isFetchingSummary,
    refetch: refetchSummary,
  } = useQuery({
    queryKey: ["billing-summary", activeTenantId],
    queryFn: () => billingApi.getTenantSummary(activeTenantId!),
    enabled: activeTenantId !== null,
  });

  const {
    data: ledgerData,
    isLoading: isLedgerLoading,
    isFetching: isFetchingLedger,
    refetch: refetchLedger,
  } = useQuery({
    queryKey: ["billing-ledger", activeTenantId],
    queryFn: () => billingApi.getTenantLedger(activeTenantId!),
    enabled: activeTenantId !== null,
  });

  const tenantOptions = useMemo(
    () => tenants.map((tenant) => ({ key: tenant.id, label: tenant.name })),
    [tenants],
  );

  const ledger = useMemo(() => ledgerData?.items ?? [], [ledgerData?.items]);
  const trendData = useMemo(
    () =>
      buildBillingTrendPoints(ledger).map((point) => ({
        ...point,
        time: new Date(point.created_at).toLocaleTimeString([], {
          hour: "2-digit",
          minute: "2-digit",
        }),
        consumed: microcreditsToCredits(point.consumed_microcredits),
      })),
    [ledger],
  );

  const filteredLedger = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase();
    if (!keyword) {
      return ledger;
    }
    return ledger.filter((entry) => matchLedgerSearch(keyword, entry));
  }, [ledger, searchValue]);

  const totalPages = Math.max(
    1,
    Math.ceil(filteredLedger.length / rowsPerPage),
  );
  const resolvedPage = Math.min(currentPage, totalPages);
  const paginatedLedger = useMemo(() => {
    const start = (resolvedPage - 1) * rowsPerPage;
    return filteredLedger.slice(start, start + rowsPerPage);
  }, [filteredLedger, resolvedPage, rowsPerPage]);
  const visibleRangeStart =
    filteredLedger.length === 0 ? 0 : (resolvedPage - 1) * rowsPerPage + 1;
  const visibleRangeEnd =
    filteredLedger.length === 0
      ? 0
      : Math.min(filteredLedger.length, resolvedPage * rowsPerPage);

  const positiveEntries = ledger.filter(
    (entry) => entry.delta_microcredits > 0,
  ).length;
  const negativeEntries = ledger.filter(
    (entry) => entry.delta_microcredits < 0,
  ).length;
  const isLoading =
    isTenantsLoading ||
    (activeTenantId !== null && (isSummaryLoading || isLedgerLoading));
  const isRefreshing =
    isFetchingTenants || isFetchingSummary || isFetchingLedger;
  const selectedTenantName = activeTenantId
    ? (tenants.find((tenant) => tenant.id === activeTenantId)?.name ??
      activeTenantId)
    : null;
  const hasLedgerActivity = ledger.length > 0;

  if (isLoading) {
    return (
      <div className="flex h-[calc(100vh-100px)] w-full items-center justify-center">
        <Spinner
          color="primary"
          label={t("billing.antigravity.loading")}
          size="lg"
        />
      </div>
    );
  }

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t("nav.billing")}
        description={t("billing.subtitle")}
        actions={
          <Button
            color="primary"
            isLoading={isRefreshing}
            startContent={
              isRefreshing ? undefined : <RefreshCcw className="h-4 w-4" />
            }
            variant="flat"
            onPress={() => {
              void refetchTenants();
              if (activeTenantId !== null) {
                void refetchSummary();
                void refetchLedger();
              }
            }}
          >
            {t("common.refresh")}
          </Button>
        }
      />

      {activeTenantId === null ? (
        <Card className="border-small border-dashed border-default-300 bg-content1 shadow-small">
          <CardBody className="py-16 text-center">
            <div className="mx-auto flex h-14 w-14 items-center justify-center rounded-2xl bg-default-100">
              <Building2 className="h-6 w-6 text-default-500" />
            </div>
            <h3 className="mt-4 text-lg font-semibold text-foreground">
              {t("billing.antigravity.noTenant")}
            </h3>
            <p className="mx-auto mt-2 max-w-xl text-sm leading-6 text-default-600">
              {t("billing.antigravity.noTenantDescription")}
            </p>
            <div className="mt-6 flex flex-wrap items-center justify-center gap-3">
              <Button color="primary" onPress={() => navigate("/tenants")}>
                {t("nav.tenants", { defaultValue: "Tenants" })}
              </Button>
              <Button
                startContent={<RefreshCcw className="h-4 w-4" />}
                variant="light"
                onPress={() => {
                  void refetchTenants();
                }}
              >
                {t("common.refresh")}
              </Button>
            </div>
          </CardBody>
        </Card>
      ) : (
        <>
          <div className="grid gap-6 xl:grid-cols-[minmax(0,1.25fr)_minmax(0,0.95fr)]">
            <Card>
              <CardHeader className="px-5 pb-3 pt-5">
                <div className="space-y-1">
                  <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                    {t("billing.antigravity.scopePanelTitle")}
                  </h2>
                  <p className="text-sm leading-6 text-default-600">
                    {t("billing.antigravity.contractHint")}
                  </p>
                </div>
              </CardHeader>
              <CardBody className="gap-4 px-5 pb-5 pt-1">
                <Select
                  aria-label={t("billing.filters.tenantAriaLabel")}
                  items={tenantOptions}
                  selectedKeys={activeTenantId ? [activeTenantId] : []}
                  disallowEmptySelection={tenantOptions.length > 0}
                  isDisabled={tenantOptions.length === 0}
                  placeholder={t("billing.filters.tenantPlaceholder")}
                  size="sm"
                  onSelectionChange={(selection) => {
                    const nextValue = normalizeSelection(selection);
                    if (!nextValue || nextValue === activeTenantId) {
                      return;
                    }
                    setCurrentPage(1);
                    setSearchParams({ tenant: nextValue });
                  }}
                >
                  {(item) => (
                    <SelectItem key={item.key}>{item.label}</SelectItem>
                  )}
                </Select>

                <div className="grid gap-3 sm:grid-cols-2">
                  <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                    <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                      {t("billing.antigravity.activeTenant")}
                    </div>
                    <div className="mt-2 text-sm font-semibold text-foreground">
                      {selectedTenantName ?? t("billing.antigravity.noTenant")}
                    </div>
                  </div>
                  <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                    <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                      {t("billing.antigravity.lastUpdatedLabel")}
                    </div>
                    <div className="mt-2 text-sm font-semibold text-foreground">
                      {formatDateTime(summary?.updated_at)}
                    </div>
                  </div>
                </div>
              </CardBody>
            </Card>

            <Card>
              <CardHeader className="px-5 pb-3 pt-5">
                <div className="space-y-1">
                  <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                    {t("billing.antigravity.ledgerSignalsTitle")}
                  </h2>
                  <p className="text-sm leading-6 text-default-600">
                    {t("billing.antigravity.ledgerSignalsDescription")}
                  </p>
                </div>
              </CardHeader>
              <CardBody className="grid gap-3 px-5 pb-5 pt-1 sm:grid-cols-2">
                <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                  <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                    {t("billing.antigravity.rechargeEvents")}
                  </div>
                  <div className="mt-2 text-2xl font-semibold tracking-[-0.04em] text-foreground">
                    {positiveEntries}
                  </div>
                </div>
                <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                  <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                    {t("billing.antigravity.usageEvents")}
                  </div>
                  <div className="mt-2 text-2xl font-semibold tracking-[-0.04em] text-foreground">
                    {negativeEntries}
                  </div>
                </div>
                <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                  <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                    {t("billing.antigravity.ledgerRows")}
                  </div>
                  <div className="mt-2 text-2xl font-semibold tracking-[-0.04em] text-foreground">
                    {ledger.length}
                  </div>
                </div>
                <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
                  <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                    {t("billing.antigravity.currentBalanceChip")}
                  </div>
                  <div className="mt-2 text-2xl font-semibold tracking-[-0.04em] text-foreground">
                    {formatCredits(summary?.balance_microcredits)}
                  </div>
                </div>
              </CardBody>
            </Card>
          </div>

          {!hasLedgerActivity ? (
            <Card>
              <CardBody className="flex flex-col gap-4 px-6 py-8 lg:flex-row lg:items-center lg:justify-between">
                <div className="flex items-start gap-4">
                  <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-large bg-warning/10 text-warning">
                    <Wallet className="h-5 w-5" />
                  </div>
                  <div className="space-y-2">
                    <h3 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                      {t("billing.antigravity.noActivityTitle")}
                    </h3>
                    <p className="max-w-2xl text-sm leading-6 text-default-600">
                      {t("billing.antigravity.noActivityDescription", {
                        tenant:
                          selectedTenantName ??
                          t("billing.antigravity.noTenant"),
                      })}
                    </p>
                  </div>
                </div>
                <div className="flex shrink-0 flex-wrap gap-3">
                  <Button color="primary" onPress={() => navigate("/accounts")}>
                    {t("nav.accounts", { defaultValue: "Accounts Pool" })}
                  </Button>
                  <Button
                    startContent={<RefreshCcw className="h-4 w-4" />}
                    variant="light"
                    onPress={() => {
                      void refetchSummary();
                      void refetchLedger();
                    }}
                  >
                    {t("common.refresh")}
                  </Button>
                </div>
              </CardBody>
            </Card>
          ) : null}

          <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            {[
              {
                title: t("billing.summary.currentBalance"),
                value: formatCredits(summary?.balance_microcredits),
                description: t("billing.antigravity.balanceHint"),
                icon: <Wallet className="h-4 w-4" />,
                toneClassName: "bg-success/10 text-success",
              },
              {
                title: t("billing.summary.todayConsumed"),
                value: formatCredits(summary?.today_consumed_microcredits),
                description: t("billing.antigravity.todayConsumedHint"),
                icon: <CreditCard className="h-4 w-4" />,
                toneClassName: "bg-primary/10 text-primary",
              },
              {
                title: t("billing.summary.monthConsumed"),
                value: formatCredits(summary?.month_consumed_microcredits),
                description: t("billing.antigravity.monthConsumedHint"),
                icon: <CreditCard className="h-4 w-4" />,
                toneClassName: "bg-warning/10 text-warning",
              },
              {
                title: t("billing.antigravity.averageDeduction"),
                value: formatCredits(
                  negativeEntries > 0
                    ? Math.round(
                        Math.abs(
                          ledger
                            .filter((entry) => entry.delta_microcredits < 0)
                            .reduce(
                              (sum, entry) => sum + entry.delta_microcredits,
                              0,
                            ),
                        ) / negativeEntries,
                      )
                    : 0,
                ),
                description: t("billing.antigravity.averageDeductionHint"),
                icon: <CreditCard className="h-4 w-4" />,
                toneClassName: "bg-secondary/10 text-secondary",
              },
            ].map((metric) => (
              <Card
                key={metric.title}
               
              >
                <CardBody className="space-y-5 p-4">
                  <div
                    className={
                      metric.toneClassName +
                      " flex h-10 w-10 items-center justify-center rounded-large"
                    }
                  >
                    {metric.icon}
                  </div>
                  <div className="space-y-2">
                    <p className="text-xs font-semibold uppercase tracking-[0.16em] text-default-500">
                      {metric.title}
                    </p>
                    <p className="text-[clamp(1.55rem,3vw,2.15rem)] font-semibold leading-none tracking-[-0.045em] text-foreground">
                      {metric.value}
                    </p>
                    <p className="text-sm leading-6 text-default-600">
                      {metric.description}
                    </p>
                  </div>
                </CardBody>
              </Card>
            ))}
          </div>

          <Card>
            <CardHeader className="px-5 pb-3 pt-5">
              <div className="space-y-1">
                <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {t("billing.trend.title")}
                </h2>
                <p className="text-sm leading-6 text-default-600">
                  {t("billing.trend.subtitle", {
                    granularity: t("billing.granularity.day"),
                  })}
                </p>
              </div>
            </CardHeader>
            <CardBody className="px-5 pb-5 pt-1">
              {trendData.length ? (
                <ResponsiveContainer height={260} width="100%">
                  <AreaChart data={trendData}>
                    <defs>
                      <linearGradient
                        id="billingConsumedGradient"
                        x1="0"
                        x2="0"
                        y1="0"
                        y2="1"
                      >
                        <stop
                          offset="0%"
                          stopColor="hsl(var(--heroui-warning))"
                          stopOpacity={0.28}
                        />
                        <stop
                          offset="100%"
                          stopColor="hsl(var(--heroui-warning))"
                          stopOpacity={0}
                        />
                      </linearGradient>
                    </defs>
                    <CartesianGrid
                      stroke={chartGridColor}
                      strokeDasharray="3 3"
                    />
                    <XAxis
                      axisLine={false}
                      dataKey="time"
                      tick={{ fill: chartTextColor, fontSize: 11 }}
                      tickLine={false}
                    />
                    <YAxis
                      axisLine={false}
                      tick={{ fill: chartTextColor, fontSize: 11 }}
                      tickLine={false}
                    />
                    <Tooltip contentStyle={chartTooltipStyle} />
                    <Area
                      dataKey="consumed"
                      fill="url(#billingConsumedGradient)"
                      stroke="hsl(var(--heroui-warning))"
                      strokeWidth={2}
                      type="monotone"
                    />
                  </AreaChart>
                </ResponsiveContainer>
              ) : (
                <div className="flex h-[260px] flex-col items-center justify-center gap-3 rounded-large border border-dashed border-default-200 px-6 text-center">
                  <Wallet className="h-8 w-8 text-default-300" />
                  <div className="space-y-1">
                    <div className="text-sm font-medium text-foreground">
                      {t("billing.antigravity.noActivityTitle")}
                    </div>
                    <div className="text-sm leading-6 text-default-600">
                      {t("billing.trend.noData")}
                    </div>
                  </div>
                </div>
              )}
            </CardBody>
          </Card>

          <Card>
            <CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5">
              <div className="space-y-1">
                <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {t("billing.ledger.title")}
                </h2>
                <p className="text-sm leading-6 text-default-600">
                  {t("billing.antigravity.lastUpdated", {
                    time: summary?.updated_at
                      ? new Date(summary.updated_at).toLocaleString()
                      : "-",
                  })}
                </p>
              </div>

              <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
                <Input
                  aria-label={t("billing.antigravity.ledgerSearch")}
                  className="sm:max-w-sm"
                  placeholder={t("billing.antigravity.ledgerSearch")}
                  size="sm"
                  startContent={<Search className="h-4 w-4 text-default-400" />}
                  value={searchValue}
                  onValueChange={(value) => {
                    setCurrentPage(1);
                    setSearchValue(value);
                  }}
                />

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
            </CardHeader>

            <CardBody className="gap-4 px-5 pb-5 pt-0">
              <Table
                isHeaderSticky
                aria-label={t("billing.ledger.title")}
                classNames={{
                  base: "min-h-[26rem]",
                  wrapper: "bg-transparent px-0 py-0 shadow-none",
                  th: "bg-default-100/60 text-xs font-semibold uppercase tracking-[0.12em] text-default-500",
                  td: "align-top py-4 text-sm text-foreground",
                  tr: "data-[hover=true]:bg-content2/35 transition-colors",
                  emptyWrapper: "h-48",
                }}
              >
                <TableHeader>
                  <TableColumn>{t("billing.columns.timestamp")}</TableColumn>
                  <TableColumn>{t("billing.columns.eventType")}</TableColumn>
                  <TableColumn>{t("billing.columns.model")}</TableColumn>
                  <TableColumn>{t("billing.columns.delta")}</TableColumn>
                  <TableColumn>{t("billing.columns.balanceAfter")}</TableColumn>
                </TableHeader>
                <TableBody
                  emptyContent={
                    <div className="flex flex-col items-center gap-3 py-10 text-default-500">
                      <CreditCard className="h-10 w-10 opacity-35" />
                      <div className="text-sm font-medium">
                        {t("billing.ledger.empty")}
                      </div>
                    </div>
                  }
                  items={paginatedLedger}
                >
                  {(entry) => (
                    <TableRow key={entry.id}>
                      <TableCell>
                        <div className="min-w-[180px] font-mono text-xs text-default-500">
                          {formatDateTime(entry.created_at)}
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="min-w-[180px] space-y-2">
                          <Chip
                            color={
                              entry.delta_microcredits >= 0
                                ? "success"
                                : "warning"
                            }
                            size="sm"
                            variant="flat"
                          >
                            {mapLedgerEventType(t, entry.event_type)}
                          </Chip>
                          <div className="text-xs leading-5 text-default-500">
                            {entry.request_id ??
                              entry.api_key_id ??
                              t("billing.antigravity.tenantCreditEvent")}
                          </div>
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="min-w-[180px] text-sm text-default-600">
                          {entry.model ?? "-"}
                        </div>
                      </TableCell>
                      <TableCell>
                        <div
                          className={`min-w-[140px] font-mono text-sm font-semibold ${
                            entry.delta_microcredits >= 0
                              ? "text-success"
                              : "text-danger"
                          }`}
                        >
                          {entry.delta_microcredits >= 0 ? "+" : "-"}
                          {formatCredits(Math.abs(entry.delta_microcredits))}
                        </div>
                      </TableCell>
                      <TableCell>
                        <div className="min-w-[140px] font-mono text-xs text-default-500">
                          {formatCredits(entry.balance_after_microcredits)}
                        </div>
                      </TableCell>
                    </TableRow>
                  )}
                </TableBody>
              </Table>

              <div className="flex flex-col gap-3 border-t border-default-200 pt-3 text-xs text-default-500 sm:flex-row sm:items-center sm:justify-between">
                <div className="tabular-nums">
                  {t("common.table.range", {
                    start: visibleRangeStart,
                    end: visibleRangeEnd,
                    total: filteredLedger.length,
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
        </>
      )}
    </PageContent>
  );
}
