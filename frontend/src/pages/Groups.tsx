import { useCallback, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Button,
  Checkbox,
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
  Textarea,
  type Selection,
} from "@heroui/react";
import {
  Boxes,
  ChevronDown,
  ChevronUp,
  FolderKanban,
  RefreshCcw,
  RotateCw,
  Search,
  ShieldCheck,
  SquarePen,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import {
  AntigravityDialogActions,
  AntigravityDialogShell,
} from "@/components/layout/dialog-archetypes";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import {
  groupsApi,
  type ApiKeyGroupAdminListResponse,
  type ApiKeyGroupCatalogItem,
  type ApiKeyGroupItem,
} from "@/api/groups";
import { localizeApiErrorDisplay } from "@/api/errorI18n";
import {
  DockedPageIntro,
  PageContent,
} from "@/components/layout/page-archetypes";
import { Dialog } from "@/components/ui/dialog";
import { notify } from "@/lib/notification";
import {
  SurfaceCard,
  SurfaceCardBody,
  SurfaceSection,
} from "@/components/ui/surface";

type GroupStatusFilter = "all" | "enabled" | "disabled" | "deleted";

const TABLE_PAGE_SIZE_OPTIONS = [10, 20, 50];

function normalizeSelection(selection: Selection) {
  if (selection === "all") {
    return "";
  }

  const [first] = Array.from(selection);
  return first === undefined ? "" : String(first);
}

function formatMultiplier(ppm?: number | null) {
  if (typeof ppm !== "number") return "-";
  return `×${(ppm / 1_000_000).toFixed(2)}`;
}

function formatMicrocredits(value?: number | null) {
  if (typeof value !== "number") return "-";
  return (value / 1_000_000).toFixed(4);
}

function formatDateTime(value?: string | null) {
  if (!value) return "-";
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? "-" : parsed.toLocaleString();
}

function buildGroupSearchText(group: ApiKeyGroupItem) {
  const status = group.deleted_at
    ? "deleted"
    : group.enabled
      ? "enabled"
      : "disabled";
  const scope = group.allow_all_models ? "catalog all" : "scoped";
  return `${group.name} ${group.description ?? ""} ${status} ${scope} ${group.api_key_count} ${group.model_count}`.toLowerCase();
}

function getGroupStatusColor(group: ApiKeyGroupItem) {
  if (group.deleted_at) {
    return "danger" as const;
  }
  if (!group.enabled) {
    return "default" as const;
  }
  return "success" as const;
}

function pricingLineForModel(
  model: ApiKeyGroupItem["models"][number],
  t: ReturnType<typeof useTranslation>["t"],
) {
  const inputLabel = t("common.tokenSegments.input");
  const cachedLabel = t("common.tokenSegments.cached");
  const outputLabel = t("common.tokenSegments.output");
  const formula = `${inputLabel} ${formatMicrocredits(model.formula_input_price_microcredits)} · ${cachedLabel} ${formatMicrocredits(model.formula_cached_input_price_microcredits)} · ${outputLabel} ${formatMicrocredits(model.formula_output_price_microcredits)}`;
  const finalPricing = `${inputLabel} ${formatMicrocredits(model.final_input_price_microcredits)} · ${cachedLabel} ${formatMicrocredits(model.final_cached_input_price_microcredits)} · ${outputLabel} ${formatMicrocredits(model.final_output_price_microcredits)}`;
  return { formula, finalPricing };
}

export default function Groups() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingGroupId, setEditingGroupId] = useState<string | null>(null);
  const [selectedModel, setSelectedModel] = useState<string>("");
  const [mobilePreviewExpanded, setMobilePreviewExpanded] = useState(false);
  const [searchValue, setSearchValue] = useState("");
  const [statusFilter, setStatusFilter] = useState<GroupStatusFilter>("all");
  const [rowsPerPage, setRowsPerPage] = useState(10);
  const [currentPage, setCurrentPage] = useState(1);
  const [groupForm, setGroupForm] = useState({
    id: "",
    name: "",
    description: "",
    enabled: true,
    is_default: false,
    allow_all_models: false,
    input_multiplier_ppm: "1000000",
    cached_input_multiplier_ppm: "1000000",
    output_multiplier_ppm: "1000000",
  });
  const [policyForm, setPolicyForm] = useState({
    enabled: true,
    input_multiplier_ppm: "1000000",
    cached_input_multiplier_ppm: "1000000",
    output_multiplier_ppm: "1000000",
    input_price_microcredits: "",
    cached_input_price_microcredits: "",
    output_price_microcredits: "",
  });

  const { data, isLoading, isFetching, refetch } =
    useQuery<ApiKeyGroupAdminListResponse>({
      queryKey: ["adminApiKeyGroups"],
      queryFn: groupsApi.adminList,
      staleTime: 30_000,
    });

  const groups = useMemo(
    () =>
      [...(data?.groups ?? [])].sort((left, right) => {
        if (left.is_default !== right.is_default) {
          return left.is_default ? -1 : 1;
        }
        return left.name.localeCompare(right.name);
      }),
    [data?.groups],
  );
  const catalog = useMemo(() => data?.catalog ?? [], [data]);
  const currentGroup = useMemo(
    () => groups.find((item) => item.id === editingGroupId) ?? null,
    [editingGroupId, groups],
  );

  const selectedPolicy = useMemo(() => {
    if (!currentGroup || !selectedModel) return null;
    return (
      currentGroup.policies.find((item) => item.model === selectedModel) ?? null
    );
  }, [currentGroup, selectedModel]);

  const selectedCatalogModel = useMemo(() => {
    if (!selectedModel) return null;
    return catalog.find((item) => item.model === selectedModel) ?? null;
  }, [catalog, selectedModel]);

  const resolveError = useCallback(
    (err: unknown, fallback: string) =>
      localizeApiErrorDisplay(t, err, fallback).label,
    [t],
  );

  const notifyError = useCallback(
    (err: unknown, fallback: string) => {
      const description = resolveError(err, fallback);
      notify({
        variant: "error",
        title: fallback,
        description: description !== fallback ? description : undefined,
      });
    },
    [resolveError],
  );

  const notifySuccess = useCallback((title: string) => {
    notify({
      variant: "success",
      title,
    });
  }, []);

  const openEditor = useCallback(
    (group?: ApiKeyGroupItem | null) => {
      const target = group ?? null;
      setEditingGroupId(target?.id ?? null);
      setGroupForm({
        id: target?.id ?? "",
        name: target?.name ?? "",
        description: target?.description ?? "",
        enabled: target?.enabled ?? true,
        is_default: target?.is_default ?? false,
        allow_all_models: target?.allow_all_models ?? false,
        input_multiplier_ppm: String(target?.input_multiplier_ppm ?? 1_000_000),
        cached_input_multiplier_ppm: String(
          target?.cached_input_multiplier_ppm ?? 1_000_000,
        ),
        output_multiplier_ppm: String(
          target?.output_multiplier_ppm ?? 1_000_000,
        ),
      });
      const firstModel =
        target?.policies[0]?.model ??
        target?.models[0]?.model ??
        catalog[0]?.model ??
        "";
      setSelectedModel(firstModel);
      setMobilePreviewExpanded(false);
      const firstPolicy =
        target?.policies.find((item) => item.model === firstModel) ?? null;
      setPolicyForm({
        enabled: firstPolicy?.enabled ?? true,
        input_multiplier_ppm: String(
          firstPolicy?.input_multiplier_ppm ?? 1_000_000,
        ),
        cached_input_multiplier_ppm: String(
          firstPolicy?.cached_input_multiplier_ppm ?? 1_000_000,
        ),
        output_multiplier_ppm: String(
          firstPolicy?.output_multiplier_ppm ?? 1_000_000,
        ),
        input_price_microcredits:
          firstPolicy?.input_price_microcredits != null
            ? String(firstPolicy.input_price_microcredits)
            : "",
        cached_input_price_microcredits:
          firstPolicy?.cached_input_price_microcredits != null
            ? String(firstPolicy.cached_input_price_microcredits)
            : "",
        output_price_microcredits:
          firstPolicy?.output_price_microcredits != null
            ? String(firstPolicy.output_price_microcredits)
            : "",
      });
      setEditorOpen(true);
    },
    [catalog],
  );

  const upsertGroupMutation = useMutation({
    mutationFn: async () =>
      groupsApi.adminUpsert({
        id: groupForm.id || undefined,
        name: groupForm.name,
        description: groupForm.description.trim() || undefined,
        enabled: groupForm.enabled,
        is_default: groupForm.is_default,
        allow_all_models: groupForm.allow_all_models,
        input_multiplier_ppm: Number(groupForm.input_multiplier_ppm),
        cached_input_multiplier_ppm: Number(
          groupForm.cached_input_multiplier_ppm,
        ),
        output_multiplier_ppm: Number(groupForm.output_multiplier_ppm),
      }),
    onSuccess: (response) => {
      notifySuccess(t("groupsPage.messages.groupSaved", { name: response.name }));
      setEditingGroupId(response.id);
      setGroupForm((prev) => ({ ...prev, id: response.id }));
      void queryClient.invalidateQueries({ queryKey: ["adminApiKeyGroups"] });
    },
    onError: (err) => {
      notifyError(err, t("groupsPage.messages.groupSaveFailed"));
    },
  });

  const deleteGroupMutation = useMutation({
    mutationFn: (groupId: string) => groupsApi.adminDelete(groupId),
    onSuccess: () => {
      notifySuccess(t("groupsPage.messages.groupDeleted"));
      setEditorOpen(false);
      setEditingGroupId(null);
      void queryClient.invalidateQueries({ queryKey: ["adminApiKeyGroups"] });
    },
    onError: (err) => {
      notifyError(err, t("groupsPage.messages.groupDeleteFailed"));
    },
  });

  const upsertPolicyMutation = useMutation({
    mutationFn: async () => {
      const groupId = editingGroupId || groupForm.id;
      if (!groupId) {
        throw new Error("group_not_saved");
      }
      if (!selectedModel) {
        throw new Error("model_required");
      }
      return groupsApi.adminUpsertPolicy({
        group_id: groupId,
        model: selectedModel,
        enabled: policyForm.enabled,
        input_multiplier_ppm: Number(policyForm.input_multiplier_ppm),
        cached_input_multiplier_ppm: Number(
          policyForm.cached_input_multiplier_ppm,
        ),
        output_multiplier_ppm: Number(policyForm.output_multiplier_ppm),
        input_price_microcredits: policyForm.input_price_microcredits.trim()
          ? Number(policyForm.input_price_microcredits)
          : undefined,
        cached_input_price_microcredits:
          policyForm.cached_input_price_microcredits.trim()
            ? Number(policyForm.cached_input_price_microcredits)
            : undefined,
        output_price_microcredits: policyForm.output_price_microcredits.trim()
          ? Number(policyForm.output_price_microcredits)
          : undefined,
      });
    },
    onSuccess: () => {
      notifySuccess(t("groupsPage.messages.policySaved"));
      void queryClient.invalidateQueries({ queryKey: ["adminApiKeyGroups"] });
    },
    onError: (err) => {
      notifyError(err, t("groupsPage.messages.policySaveFailed"));
    },
  });

  const deletePolicyMutation = useMutation({
    mutationFn: (policyId: string) => groupsApi.adminDeletePolicy(policyId),
    onSuccess: () => {
      notifySuccess(t("groupsPage.messages.policyDeleted"));
      void queryClient.invalidateQueries({ queryKey: ["adminApiKeyGroups"] });
    },
    onError: (err) => {
      notifyError(err, t("groupsPage.messages.policyDeleteFailed"));
    },
  });

  const filteredGroups = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase();

    return groups.filter((group) => {
      if (statusFilter === "enabled" && (!group.enabled || group.deleted_at)) {
        return false;
      }
      if (statusFilter === "disabled" && (group.enabled || group.deleted_at)) {
        return false;
      }
      if (statusFilter === "deleted" && !group.deleted_at) {
        return false;
      }
      if (!keyword) {
        return true;
      }
      return buildGroupSearchText(group).includes(keyword);
    });
  }, [groups, searchValue, statusFilter]);

  const totalPages = Math.max(
    1,
    Math.ceil(filteredGroups.length / rowsPerPage),
  );
  const resolvedPage = Math.min(currentPage, totalPages);
  const paginatedGroups = useMemo(() => {
    const start = (resolvedPage - 1) * rowsPerPage;
    return filteredGroups.slice(start, start + rowsPerPage);
  }, [filteredGroups, resolvedPage, rowsPerPage]);
  const visibleRangeStart =
    filteredGroups.length === 0 ? 0 : (resolvedPage - 1) * rowsPerPage + 1;
  const visibleRangeEnd =
    filteredGroups.length === 0
      ? 0
      : Math.min(filteredGroups.length, resolvedPage * rowsPerPage);

  const mobilePreviewModels = useMemo(() => {
    const models = currentGroup?.models ?? [];
    return mobilePreviewExpanded ? models : models.slice(0, 3);
  }, [currentGroup, mobilePreviewExpanded]);

  const summaryCards = useMemo(
    () => [
      {
        key: "total",
        title: t("groupsPage.antigravity.metrics.total"),
        value: groups.length,
        description: t("groupsPage.antigravity.metrics.totalDesc"),
        icon: FolderKanban,
        toneClassName: "bg-primary/10 text-primary",
      },
      {
        key: "enabled",
        title: t("groupsPage.antigravity.metrics.enabled"),
        value: groups.filter((group) => group.enabled && !group.deleted_at)
          .length,
        description: t("groupsPage.antigravity.metrics.enabledDesc"),
        icon: ShieldCheck,
        toneClassName: "bg-success/10 text-success",
      },
      {
        key: "defaults",
        title: t("groupsPage.antigravity.metrics.defaults"),
        value: groups.filter((group) => group.is_default && !group.deleted_at)
          .length,
        description: t("groupsPage.antigravity.metrics.defaultsDesc"),
        icon: Boxes,
        toneClassName: "bg-secondary/10 text-secondary",
      },
      {
        key: "catalog",
        title: t("groupsPage.antigravity.metrics.catalog"),
        value: catalog.length,
        description: t("groupsPage.antigravity.metrics.catalogDesc"),
        icon: Boxes,
        toneClassName: "bg-warning/10 text-warning",
      },
    ],
    [catalog.length, groups, t],
  );

  const handleSelectedModelChange = (selection: Selection) => {
    const model = normalizeSelection(selection);
    setSelectedModel(model);
    const policy =
      currentGroup?.policies.find((item) => item.model === model) ?? null;
    setPolicyForm({
      enabled: policy?.enabled ?? true,
      input_multiplier_ppm: String(policy?.input_multiplier_ppm ?? 1_000_000),
      cached_input_multiplier_ppm: String(
        policy?.cached_input_multiplier_ppm ?? 1_000_000,
      ),
      output_multiplier_ppm: String(policy?.output_multiplier_ppm ?? 1_000_000),
      input_price_microcredits:
        policy?.input_price_microcredits != null
          ? String(policy.input_price_microcredits)
          : "",
      cached_input_price_microcredits:
        policy?.cached_input_price_microcredits != null
          ? String(policy.cached_input_price_microcredits)
          : "",
      output_price_microcredits:
        policy?.output_price_microcredits != null
          ? String(policy.output_price_microcredits)
          : "",
    });
  };

  return (
    <PageContent className="space-y-6 lg:space-y-7">
      <DockedPageIntro
        archetype="workspace"
        title={t("nav.apiKeyGroups")}
        description={t("groupsPage.subtitle")}
        actions={
          <div className="flex flex-wrap gap-2">
            <Button color="primary" onPress={() => openEditor(null)}>
              {t("groupsPage.actions.create")}
            </Button>
            <Button
              isLoading={isFetching}
              startContent={
                isFetching ? undefined : <RefreshCcw className="h-4 w-4" />
              }
              variant="light"
              onPress={() => {
                void refetch();
              }}
            >
              {t("common.refresh")}
            </Button>
          </div>
        }
      />

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {summaryCards.map((card) => {
          const Icon = card.icon;
          return (
            <Card
              key={card.key}
             
            >
              <CardBody className="px-5 py-5">
                <div className="flex items-start justify-between gap-3">
                  <div
                    className={`flex h-11 w-11 items-center justify-center rounded-large ${card.toneClassName}`}
                  >
                    <Icon className="h-5 w-5" />
                  </div>
                </div>
                <div className="mt-5 text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {card.title}
                </div>
                <div className="mt-2 text-3xl font-semibold tracking-[-0.04em] text-foreground">
                  {card.value}
                </div>
                <p className="mt-2 text-xs leading-5 text-default-500">
                  {card.description}
                </p>
              </CardBody>
            </Card>
          );
        })}
      </div>

      <Card>
        <CardHeader className="flex flex-col items-start gap-4 px-5 pb-4 pt-5">
          <div className="space-y-1">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t("groupsPage.antigravity.listTitle")}
              </h2>
              <Chip size="sm" variant="flat">
                {t("common.table.totalItems", { count: filteredGroups.length })}
              </Chip>
              {isFetching && !isLoading ? (
                <Chip color="primary" size="sm" variant="flat">
                  {t("common.loading")}
                </Chip>
              ) : null}
            </div>
            <p className="text-sm leading-6 text-default-600">
              {t("groupsPage.antigravity.listDescription")}
            </p>
          </div>

          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-[minmax(18rem,1fr)_12rem_10rem]">
            <Input
              aria-label={t("common.table.searchLabel")}
              placeholder={t("groupsPage.searchPlaceholder")}
              startContent={<Search className="h-4 w-4 text-default-400" />}
              value={searchValue}
              onValueChange={(value) => {
                setSearchValue(value);
                setCurrentPage(1);
              }}
            />

            <Select
              aria-label={t("groupsPage.filters.statusLabel")}
              selectedKeys={[statusFilter]}
              size="sm"
              onSelectionChange={(selection) => {
                const nextValue = normalizeSelection(selection);
                if (!nextValue) {
                  return;
                }
                setStatusFilter(nextValue as GroupStatusFilter);
                setCurrentPage(1);
              }}
            >
              <SelectItem key="all">{t("groupsPage.filters.all")}</SelectItem>
              <SelectItem key="enabled">
                {t("groupsPage.filters.enabled")}
              </SelectItem>
              <SelectItem key="disabled">
                {t("groupsPage.filters.disabled")}
              </SelectItem>
              <SelectItem key="deleted">
                {t("groupsPage.filters.deleted")}
              </SelectItem>
            </Select>

            <Select
              aria-label={t("common.table.rowsPerPage")}
              selectedKeys={[String(rowsPerPage)]}
              size="sm"
              onSelectionChange={(selection) => {
                const nextValue = normalizeSelection(selection);
                if (!nextValue) {
                  return;
                }
                setRowsPerPage(Number(nextValue));
                setCurrentPage(1);
              }}
            >
              {TABLE_PAGE_SIZE_OPTIONS.map((size) => (
                <SelectItem key={String(size)}>{size}</SelectItem>
              ))}
            </Select>
          </div>
        </CardHeader>

        <CardBody className="gap-4 px-5 pb-5 pt-0">
          <Table
            isHeaderSticky
            aria-label={t("groupsPage.title")}
            classNames={{
              base: "min-h-[28rem]",
              wrapper: "bg-transparent px-0 py-0 shadow-none",
              th: "bg-default-100/60 text-xs font-semibold uppercase tracking-[0.12em] text-default-500",
              td: "align-top py-4 text-sm text-foreground",
              tr: "data-[hover=true]:bg-content2/35 transition-colors",
              emptyWrapper: "h-56",
            }}
          >
            <TableHeader>
              <TableColumn>{t("groupsPage.columns.name")}</TableColumn>
              <TableColumn>{t("groupsPage.columns.status")}</TableColumn>
              <TableColumn>{t("groupsPage.columns.multipliers")}</TableColumn>
              <TableColumn>{t("groupsPage.columns.usage")}</TableColumn>
              <TableColumn>{t("groupsPage.columns.updated")}</TableColumn>
              <TableColumn>{t("groupsPage.columns.actions")}</TableColumn>
            </TableHeader>
            <TableBody
              emptyContent={
                <div className="flex flex-col items-center gap-3 py-12 text-default-500">
                  <FolderKanban className="h-10 w-10 opacity-35" />
                  <div className="text-sm font-medium">
                    {t("groupsPage.empty")}
                  </div>
                </div>
              }
              isLoading={isLoading}
              items={paginatedGroups}
              loadingContent={<Spinner label={t("common.loading")} />}
            >
              {(group) => (
                <TableRow key={group.id}>
                  <TableCell>
                    <div className="min-w-[240px] space-y-2">
                      <div className="font-medium text-foreground">
                        {group.name}
                      </div>
                      <div className="text-xs leading-5 text-default-500">
                        {group.description || "-"}
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="min-w-[200px] space-y-2">
                      <Chip
                        color={getGroupStatusColor(group)}
                        size="sm"
                        variant="flat"
                      >
                        {group.deleted_at
                          ? t("groupsPage.status.deleted")
                          : group.enabled
                            ? t("groupsPage.status.enabled")
                            : t("groupsPage.status.disabled")}
                      </Chip>
                      <div className="flex flex-wrap gap-2">
                        {group.is_default ? (
                          <Chip color="primary" size="sm" variant="flat">
                            {t("groupsPage.status.default")}
                          </Chip>
                        ) : null}
                        <Chip
                          color={
                            group.allow_all_models ? "secondary" : "default"
                          }
                          size="sm"
                          variant="flat"
                        >
                          {group.allow_all_models
                            ? t("groupsPage.antigravity.allowAllModels")
                            : t("groupsPage.antigravity.scopedPolicy")}
                        </Chip>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="min-w-[220px] space-y-1 text-xs leading-5 text-default-500">
                      <div>
                        {t("groupsPage.form.inputMultiplier")}:{" "}
                        {formatMultiplier(group.input_multiplier_ppm)}
                      </div>
                      <div>
                        {t("groupsPage.form.cachedInputMultiplier")}:{" "}
                        {formatMultiplier(group.cached_input_multiplier_ppm)}
                      </div>
                      <div>
                        {t("groupsPage.form.outputMultiplier")}:{" "}
                        {formatMultiplier(group.output_multiplier_ppm)}
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="min-w-[180px] space-y-1 text-xs leading-5 text-default-500">
                      <div>
                        {t("groupsPage.columns.apiKeysCount", {
                          count: group.api_key_count,
                        })}
                      </div>
                      <div>
                        {t("groupsPage.columns.modelsCount", {
                          count: group.model_count,
                        })}
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="min-w-[180px] text-xs leading-5 text-default-500">
                      {formatDateTime(group.updated_at)}
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex min-w-[180px] flex-wrap gap-2">
                      <Button
                        size="sm"
                        variant="flat"
                        onPress={() => openEditor(group)}
                      >
                        <SquarePen className="h-4 w-4" />
                        {t("common.edit")}
                      </Button>
                      <Button
                        color="danger"
                        isDisabled={
                          deleteGroupMutation.isPending || group.is_default
                        }
                        size="sm"
                        variant="light"
                        onPress={() => deleteGroupMutation.mutate(group.id)}
                      >
                        <Trash2 className="h-4 w-4" />
                        {t("common.delete")}
                      </Button>
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
                total: filteredGroups.length,
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

      <Dialog open={editorOpen} onOpenChange={setEditorOpen}>
        <AntigravityDialogShell
          size="xl"
          title={
            groupForm.id
              ? t("groupsPage.editor.editTitle")
              : t("groupsPage.editor.createTitle")
          }
          description={t("groupsPage.editor.description")}
          bodyClassName="p-0"
          footer={
            <AntigravityDialogActions>
              <Button variant="light" onPress={() => setEditorOpen(false)}>
                {t("common.close")}
              </Button>
            </AntigravityDialogActions>
          }
        >
          <div className="flex h-full flex-col">
            <div className="flex-1 overflow-y-auto px-4 pb-4 pt-4 sm:px-6 sm:pb-6">
              <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.32fr)] lg:gap-6">
                <SurfaceSection
                  title={t("groupsPage.editor.groupSettingsTitle")}
                  description={t("groupsPage.editor.groupSettingsDescription")}
                  tone="muted"
                >
                  <div className="space-y-4">
                    <Input
                      label={t("groupsPage.form.name")}
                      labelPlacement="outside"
                      value={groupForm.name}
                      onValueChange={(value) =>
                        setGroupForm((prev) => ({ ...prev, name: value }))
                      }
                    />
                    <Textarea
                      label={t("groupsPage.form.description")}
                      labelPlacement="outside"
                      minRows={3}
                      value={groupForm.description}
                      onValueChange={(value) =>
                        setGroupForm((prev) => ({
                          ...prev,
                          description: value,
                        }))
                      }
                    />

                    <div className="grid gap-3 md:grid-cols-3">
                      <Input
                        label={t("groupsPage.form.inputMultiplier")}
                        labelPlacement="outside"
                        type="number"
                        value={groupForm.input_multiplier_ppm}
                        onValueChange={(value) =>
                          setGroupForm((prev) => ({
                            ...prev,
                            input_multiplier_ppm: value,
                          }))
                        }
                      />
                      <Input
                        label={t("groupsPage.form.cachedInputMultiplier")}
                        labelPlacement="outside"
                        type="number"
                        value={groupForm.cached_input_multiplier_ppm}
                        onValueChange={(value) =>
                          setGroupForm((prev) => ({
                            ...prev,
                            cached_input_multiplier_ppm: value,
                          }))
                        }
                      />
                      <Input
                        label={t("groupsPage.form.outputMultiplier")}
                        labelPlacement="outside"
                        type="number"
                        value={groupForm.output_multiplier_ppm}
                        onValueChange={(value) =>
                          setGroupForm((prev) => ({
                            ...prev,
                            output_multiplier_ppm: value,
                          }))
                        }
                      />
                    </div>

                    <div className="grid gap-3 sm:grid-cols-2">
                      <Checkbox
                        isSelected={groupForm.enabled}
                        onValueChange={(checked) =>
                          setGroupForm((prev) => ({
                            ...prev,
                            enabled: checked,
                          }))
                        }
                      >
                        {t("groupsPage.form.enabled")}
                      </Checkbox>
                      <Checkbox
                        isSelected={groupForm.is_default}
                        onValueChange={(checked) =>
                          setGroupForm((prev) => ({
                            ...prev,
                            is_default: checked,
                          }))
                        }
                      >
                        {t("groupsPage.form.default")}
                      </Checkbox>
                      <Checkbox
                        isSelected={groupForm.allow_all_models}
                        onValueChange={(checked) =>
                          setGroupForm((prev) => ({
                            ...prev,
                            allow_all_models: checked,
                          }))
                        }
                      >
                        {t("groupsPage.form.allowAllModels")}
                      </Checkbox>
                    </div>

                    <div className="flex flex-wrap gap-2">
                      <Button
                        color="primary"
                        isLoading={upsertGroupMutation.isPending}
                        startContent={
                          upsertGroupMutation.isPending ? undefined : (
                            <RotateCw className="h-4 w-4" />
                          )
                        }
                        onPress={() => upsertGroupMutation.mutate()}
                      >
                        {t("groupsPage.actions.saveGroup")}
                      </Button>
                      {groupForm.id ? (
                        <Button
                          color="danger"
                          isDisabled={
                            deleteGroupMutation.isPending ||
                            groupForm.is_default
                          }
                          startContent={<Trash2 className="h-4 w-4" />}
                          variant="light"
                          onPress={() =>
                            deleteGroupMutation.mutate(groupForm.id)
                          }
                        >
                          {t("groupsPage.actions.deleteGroup")}
                        </Button>
                      ) : null}
                    </div>
                  </div>
                </SurfaceSection>

                <div className="min-w-0 space-y-4">
                  <SurfaceSection
                    title={t("groupsPage.policy.title")}
                    description={t("groupsPage.policy.description")}
                    tone="muted"
                  >
                    <div className="space-y-4">
                      <Select
                        aria-label={t("groupsPage.policy.model")}
                        label={t("groupsPage.policy.model")}
                        labelPlacement="outside"
                        placeholder={t("groupsPage.policy.model")}
                        selectedKeys={selectedModel ? [selectedModel] : []}
                        onSelectionChange={handleSelectedModelChange}
                      >
                        {catalog.map((item: ApiKeyGroupCatalogItem) => (
                          <SelectItem key={item.model}>{item.model}</SelectItem>
                        ))}
                      </Select>

                      <SurfaceCard tone="default" shadow="none">
                        <SurfaceCardBody className="p-3 text-xs leading-5 text-default-500">
                          {selectedCatalogModel
                            ? t("groupsPage.preview.basePricingSummary", {
                                provider: selectedCatalogModel.provider,
                                title: selectedCatalogModel.title || "-",
                                input: formatMicrocredits(
                                  selectedCatalogModel.base_input_price_microcredits,
                                ),
                                cached: formatMicrocredits(
                                  selectedCatalogModel.base_cached_input_price_microcredits,
                                ),
                                output: formatMicrocredits(
                                  selectedCatalogModel.base_output_price_microcredits,
                                ),
                              })
                            : "-"}
                        </SurfaceCardBody>
                      </SurfaceCard>

                      <div className="grid gap-3 md:grid-cols-3">
                        <Input
                          label={t("groupsPage.policy.inputMultiplier")}
                          labelPlacement="outside"
                          type="number"
                          value={policyForm.input_multiplier_ppm}
                          onValueChange={(value) =>
                            setPolicyForm((prev) => ({
                              ...prev,
                              input_multiplier_ppm: value,
                            }))
                          }
                        />
                        <Input
                          label={t("groupsPage.policy.cachedInputMultiplier")}
                          labelPlacement="outside"
                          type="number"
                          value={policyForm.cached_input_multiplier_ppm}
                          onValueChange={(value) =>
                            setPolicyForm((prev) => ({
                              ...prev,
                              cached_input_multiplier_ppm: value,
                            }))
                          }
                        />
                        <Input
                          label={t("groupsPage.policy.outputMultiplier")}
                          labelPlacement="outside"
                          type="number"
                          value={policyForm.output_multiplier_ppm}
                          onValueChange={(value) =>
                            setPolicyForm((prev) => ({
                              ...prev,
                              output_multiplier_ppm: value,
                            }))
                          }
                        />
                      </div>

                      <div className="grid gap-3 md:grid-cols-3">
                        <Input
                          label={t("groupsPage.policy.inputAbsolutePrice")}
                          labelPlacement="outside"
                          type="number"
                          value={policyForm.input_price_microcredits}
                          onValueChange={(value) =>
                            setPolicyForm((prev) => ({
                              ...prev,
                              input_price_microcredits: value,
                            }))
                          }
                        />
                        <Input
                          label={t(
                            "groupsPage.policy.cachedInputAbsolutePrice",
                          )}
                          labelPlacement="outside"
                          type="number"
                          value={policyForm.cached_input_price_microcredits}
                          onValueChange={(value) =>
                            setPolicyForm((prev) => ({
                              ...prev,
                              cached_input_price_microcredits: value,
                            }))
                          }
                        />
                        <Input
                          label={t("groupsPage.policy.outputAbsolutePrice")}
                          labelPlacement="outside"
                          type="number"
                          value={policyForm.output_price_microcredits}
                          onValueChange={(value) =>
                            setPolicyForm((prev) => ({
                              ...prev,
                              output_price_microcredits: value,
                            }))
                          }
                        />
                      </div>

                      <Checkbox
                        isSelected={policyForm.enabled}
                        onValueChange={(checked) =>
                          setPolicyForm((prev) => ({
                            ...prev,
                            enabled: checked,
                          }))
                        }
                      >
                        {t("groupsPage.policy.enabled")}
                      </Checkbox>

                      <div className="flex flex-wrap gap-2">
                        <Button
                          color="primary"
                          isDisabled={
                            upsertPolicyMutation.isPending ||
                            !groupForm.id ||
                            !selectedModel
                          }
                          isLoading={upsertPolicyMutation.isPending}
                          startContent={
                            upsertPolicyMutation.isPending ? undefined : (
                              <RotateCw className="h-4 w-4" />
                            )
                          }
                          onPress={() => upsertPolicyMutation.mutate()}
                        >
                          {t("groupsPage.actions.savePolicy")}
                        </Button>
                        {selectedPolicy ? (
                          <Button
                            color="danger"
                            isDisabled={deletePolicyMutation.isPending}
                            startContent={<Trash2 className="h-4 w-4" />}
                            variant="light"
                            onPress={() =>
                              deletePolicyMutation.mutate(selectedPolicy.id)
                            }
                          >
                            {t("groupsPage.actions.deletePolicy")}
                          </Button>
                        ) : null}
                      </div>
                    </div>
                  </SurfaceSection>

                  <SurfaceSection
                    title={t("groupsPage.preview.title")}
                    description={t("groupsPage.preview.description")}
                    tone="muted"
                  >
                    <div className="space-y-4">
                      <div className="flex items-start justify-between gap-3">
                        <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                          <Chip size="sm" variant="flat">
                            {t("groupsPage.columns.modelsCount", {
                              count: currentGroup?.models.length ?? 0,
                            })}
                          </Chip>
                          <Chip size="sm" variant="flat">
                            {currentGroup?.allow_all_models
                              ? t("groupsPage.antigravity.allowAllModels")
                              : t("groupsPage.antigravity.scopedPolicy")}
                          </Chip>
                        </div>
                        {(currentGroup?.models?.length ?? 0) > 3 ? (
                          <Button
                            className="shrink-0 md:hidden"
                            size="sm"
                            variant="light"
                            onPress={() =>
                              setMobilePreviewExpanded((prev) => !prev)
                            }
                          >
                            {mobilePreviewExpanded ? (
                              <ChevronUp className="h-4 w-4" />
                            ) : (
                              <ChevronDown className="h-4 w-4" />
                            )}
                            {mobilePreviewExpanded
                              ? t("common.collapse", {
                                  defaultValue: "Collapse",
                                })
                              : t("common.expand", { defaultValue: "Expand" })}
                          </Button>
                        ) : null}
                      </div>

                      <div className="space-y-2 md:hidden">
                        {mobilePreviewModels.map((item) => {
                          const pricingLine = pricingLineForModel(item, t);
                          return (
                            <SurfaceCard
                              key={item.model}
                              tone="default"
                              shadow="none"
                            >
                              <SurfaceCardBody className="p-3 text-sm">
                                <div className="flex items-start justify-between gap-3">
                                  <div className="font-mono text-xs break-all">
                                    {item.model}
                                  </div>
                                  <Chip
                                    color={
                                      item.uses_absolute_pricing
                                        ? "success"
                                        : "default"
                                    }
                                    size="sm"
                                    variant="flat"
                                  >
                                    {item.uses_absolute_pricing
                                      ? t("groupsPage.preview.mode.absolute")
                                      : t("groupsPage.preview.mode.formula")}
                                  </Chip>
                                </div>
                                <div className="mt-3 space-y-2 text-xs leading-5 text-default-500">
                                  <div>
                                    <div>
                                      {t(
                                        "groupsPage.preview.columns.finalPrice",
                                      )}
                                    </div>
                                    <div className="text-foreground">
                                      {pricingLine.finalPricing}
                                    </div>
                                  </div>
                                  <div>
                                    <div>
                                      {t(
                                        "groupsPage.preview.columns.formulaPrice",
                                      )}
                                    </div>
                                    <div
                                      className={
                                        item.uses_absolute_pricing
                                          ? "line-through"
                                          : ""
                                      }
                                    >
                                      {pricingLine.formula}
                                    </div>
                                  </div>
                                </div>
                              </SurfaceCardBody>
                            </SurfaceCard>
                          );
                        })}
                        {!mobilePreviewExpanded &&
                        (currentGroup?.models?.length ?? 0) > 3 ? (
                          <div className="text-center text-xs text-muted-foreground">
                            {t("groupsPage.preview.moreHidden", {
                              count:
                                (currentGroup?.models?.length ?? 0) -
                                mobilePreviewModels.length,
                            })}
                          </div>
                        ) : null}
                      </div>

                      <Table
                        aria-label={t("groupsPage.preview.title")}
                        className="hidden md:block"
                        classNames={{
                          wrapper: "bg-transparent px-0 py-0 shadow-none",
                          th: "bg-default-100/60 text-xs font-semibold uppercase tracking-[0.12em] text-default-500",
                          td: "align-top py-3 text-sm text-foreground",
                          emptyWrapper: "h-44",
                        }}
                      >
                        <TableHeader>
                          <TableColumn>
                            {t("groupsPage.preview.columns.model")}
                          </TableColumn>
                          <TableColumn>
                            {t("groupsPage.preview.columns.finalPrice")}
                          </TableColumn>
                          <TableColumn>
                            {t("groupsPage.preview.columns.formulaPrice")}
                          </TableColumn>
                          <TableColumn>
                            {t("groupsPage.preview.columns.mode")}
                          </TableColumn>
                        </TableHeader>
                        <TableBody
                          emptyContent={
                            <div className="flex flex-col items-center gap-3 py-10 text-default-500">
                              <Boxes className="h-10 w-10 opacity-35" />
                              <div className="text-sm font-medium">
                                {t("groupsPage.preview.empty")}
                              </div>
                            </div>
                          }
                          items={currentGroup?.models ?? []}
                        >
                          {(item) => {
                            const pricingLine = pricingLineForModel(item, t);
                            return (
                              <TableRow key={item.model}>
                                <TableCell>
                                  <div className="font-mono text-xs text-default-500">
                                    {item.model}
                                  </div>
                                </TableCell>
                                <TableCell>
                                  {pricingLine.finalPricing}
                                </TableCell>
                                <TableCell>
                                  <span
                                    className={
                                      item.uses_absolute_pricing
                                        ? "line-through text-default-400"
                                        : "text-default-500"
                                    }
                                  >
                                    {pricingLine.formula}
                                  </span>
                                </TableCell>
                                <TableCell>
                                  <Chip
                                    color={
                                      item.uses_absolute_pricing
                                        ? "success"
                                        : "default"
                                    }
                                    size="sm"
                                    variant="flat"
                                  >
                                    {item.uses_absolute_pricing
                                      ? t("groupsPage.preview.mode.absolute")
                                      : t("groupsPage.preview.mode.formula")}
                                  </Chip>
                                </TableCell>
                              </TableRow>
                            );
                          }}
                        </TableBody>
                      </Table>
                    </div>
                  </SurfaceSection>
                </div>
              </div>
            </div>
          </div>
        </AntigravityDialogShell>
      </Dialog>
    </PageContent>
  );
}
