import { useCallback, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
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
  Textarea,
  Tooltip,
  type Selection,
} from "@heroui/react";
import {
  Copy,
  KeyRound,
  Loader2,
  Plus,
  RefreshCcw,
  Search,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { localizeApiErrorDisplay } from "@/api/errorI18n";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import {
  apiKeysApi,
  type ApiKey,
  type CreateApiKeyResponse,
} from "@/api/settings";
import {
  DockedPageIntro,
  PageContent,
} from "@/components/layout/page-archetypes";
import { notify } from "@/lib/notification";
import {
  copyText,
  createDateTimeFormatter,
  formatDateTimeValue,
} from "@/features/tenants/utils";

type StatusFilter = "all" | "active" | "revoked";

const TABLE_PAGE_SIZE_OPTIONS = [10, 20, 50];

function normalizeSelection(selection: Selection) {
  if (selection === "all") {
    return "";
  }

  const [first] = Array.from(selection);
  return first === undefined ? "" : String(first);
}

function buildAdminApiKeySearchText(key: ApiKey) {
  const status = key.enabled ? "active enabled" : "revoked disabled";
  return `${key.name} ${key.key_prefix} ${key.id} ${key.tenant_id ?? ""} ${status}`.toLowerCase();
}

export default function AdminApiKeys() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [newKeyName, setNewKeyName] = useState("");
  const [createdKey, setCreatedKey] = useState<CreateApiKeyResponse | null>(
    null,
  );
  const [pendingKeyId, setPendingKeyId] = useState<string | null>(null);
  const [searchValue, setSearchValue] = useState("");
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [rowsPerPage, setRowsPerPage] = useState(20);
  const [currentPage, setCurrentPage] = useState(1);
  const dateTimeFormatter = useMemo(() => createDateTimeFormatter(), []);

  const formatDateTime = useCallback(
    (value?: string | null) => formatDateTimeValue(dateTimeFormatter, value),
    [dateTimeFormatter],
  );

  const {
    data: keys = [],
    isLoading,
    isFetching,
    refetch,
  } = useQuery({
    queryKey: ["apiKeys"],
    queryFn: () => apiKeysApi.listKeys(),
    staleTime: 60_000,
  });

  const createMutation = useMutation({
    mutationFn: (name: string) => apiKeysApi.createKey(name),
    onSuccess: (payload) => {
      setCreatedKey(payload);
      setNewKeyName("");
      queryClient.invalidateQueries({ queryKey: ["apiKeys"] });
      notify({
        variant: "success",
        title: t("apiKeys.dialog.created.title", {
          defaultValue: "New key created",
        }),
        description: t("tenants.keys.created.notice", {
          defaultValue: "The plaintext key is shown only once. Save it now.",
        }),
      });
    },
    onError: (error) => {
      const fallback = t("apiKeys.messages.createFailed", {
        defaultValue: "Failed to create API key",
      });
      notify({
        variant: "error",
        title: fallback,
        description: localizeApiErrorDisplay(t, error, fallback).label,
      });
    },
  });

  const toggleMutation = useMutation({
    mutationFn: ({ keyId, enabled }: { keyId: string; enabled: boolean }) =>
      apiKeysApi.updateKeyEnabled(keyId, enabled),
    onMutate: ({ keyId }) => setPendingKeyId(keyId),
    onSettled: () => setPendingKeyId(null),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["apiKeys"] });
    },
    onError: (error) => {
      const fallback = t("tenants.messages.apiKeyToggleFailed", {
        defaultValue: "Failed to update API key status",
      });
      notify({
        variant: "error",
        title: fallback,
        description: localizeApiErrorDisplay(t, error, fallback).label,
      });
    },
  });

  const activeCount = useMemo(
    () => keys.filter((key) => key.enabled).length,
    [keys],
  );
  const revokedCount = keys.length - activeCount;

  const filteredKeys = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase();
    return keys.filter((key) => {
      const matchesKeyword =
        !keyword || buildAdminApiKeySearchText(key).includes(keyword);
      const matchesStatus =
        statusFilter === "all"
          ? true
          : statusFilter === "active"
            ? key.enabled
            : !key.enabled;
      return matchesKeyword && matchesStatus;
    });
  }, [keys, searchValue, statusFilter]);

  const totalPages = Math.max(1, Math.ceil(filteredKeys.length / rowsPerPage));
  const resolvedPage = Math.min(currentPage, totalPages);
  const paginatedKeys = useMemo(() => {
    const start = (resolvedPage - 1) * rowsPerPage;
    return filteredKeys.slice(start, start + rowsPerPage);
  }, [filteredKeys, resolvedPage, rowsPerPage]);
  const visibleRangeStart =
    filteredKeys.length === 0 ? 0 : (resolvedPage - 1) * rowsPerPage + 1;
  const visibleRangeEnd =
    filteredKeys.length === 0
      ? 0
      : Math.min(filteredKeys.length, resolvedPage * rowsPerPage);

  const handleCreate = () => {
    const name = newKeyName.trim();
    if (!name) {
      notify({
        variant: "error",
        title: t("apiKeys.messages.createFailed", {
          defaultValue: "Failed to create API key",
        }),
        description: t("apiKeys.messages.missingName", {
          defaultValue: "Please enter a key name",
        }),
      });
      return;
    }
    createMutation.mutate(name);
  };

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t("nav.apiKeys", { defaultValue: "Key Pool" })}
        description={t("apiKeys.subtitle", {
          defaultValue:
            "Manage the standalone workspace key pool and issue secure access credentials.",
        })}
        actions={
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
        }
      />

      <div className="grid gap-6 xl:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
        <Card
          className={createdKey ? undefined : "xl:col-span-2"}
        >
          <CardHeader className="px-5 pb-3 pt-5">
            <div className="space-y-3">
              <div className="space-y-1">
                <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {t("tenants.keys.create.title", {
                    defaultValue: "Create API Key",
                  })}
                </h2>
                <p className="text-sm leading-6 text-default-600">
                  {t("apiKeys.createPanelDescription", {
                    defaultValue:
                      "Create a Data Plane access key for this standalone workspace. The plaintext key is shown only once.",
                  })}
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <Chip color="warning" size="sm" variant="flat">
                  {t("apiKeys.dialog.created.plaintextLabel", {
                    defaultValue: "Plaintext key",
                  })}
                </Chip>
              </div>
            </div>
          </CardHeader>
          <CardBody className="gap-4 px-5 pb-5 pt-1">
            <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end">
              <Input
                id="admin-api-key-name"
                classNames={{ label: "text-xs font-medium text-default-500" }}
                label={t("tenants.keys.create.fields.name", {
                  defaultValue: "Key Name",
                })}
                labelPlacement="outside"
                placeholder={t("tenants.keys.create.fields.namePlaceholder", {
                  defaultValue: "e.g. admin-main-key",
                })}
                value={newKeyName}
                onValueChange={setNewKeyName}
              />
              <div className="flex items-center lg:justify-end">
                <Button
                  color="primary"
                  isLoading={createMutation.isPending}
                  startContent={
                    createMutation.isPending ? undefined : (
                      <Plus className="h-4 w-4" />
                    )
                  }
                  onPress={handleCreate}
                >
                  {t("tenants.keys.create.submit", {
                    defaultValue: "Create Key",
                  })}
                </Button>
              </div>
            </div>
          </CardBody>
        </Card>

        {createdKey ? (
          <Card className="border-small border-warning-200 bg-warning-50/80 shadow-none dark:bg-warning/10">
            <CardHeader className="px-5 pb-3 pt-5">
              <div className="space-y-1">
                <div className="flex flex-wrap items-center gap-2">
                  <h2 className="text-lg font-semibold tracking-[-0.02em] text-warning-700 dark:text-warning-300">
                    {t("apiKeys.dialog.created.title", {
                      defaultValue: "New key created",
                    })}
                  </h2>
                  <Chip color="warning" size="sm" variant="flat">
                    {t("apiKeys.dialog.created.plaintextLabel", {
                      defaultValue: "Plaintext key",
                    })}
                  </Chip>
                </div>
                <p className="text-sm leading-6 text-warning-700/80 dark:text-warning-200/80">
                  {t("apiKeys.dialog.created.desc", {
                    defaultValue:
                      "The plaintext key is shown only once. Please copy and store it now.",
                  })}
                </p>
              </div>
            </CardHeader>
            <CardBody className="gap-4 px-5 pb-5 pt-1">
              <div className="rounded-large border border-warning-200/80 bg-content1/85 px-4 py-4 dark:border-warning/20">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t("apiKeys.dialog.created.nameLabel", {
                    defaultValue: "Key name",
                  })}
                </div>
                <div className="mt-2 text-sm font-medium text-foreground">
                  {createdKey.record.name}
                </div>
              </div>
              <p className="text-xs leading-5 text-warning-800/80 dark:text-warning-200/80">
                {t("apiKeys.dialog.created.securityTip", {
                  defaultValue:
                    "Security notice: once this dialog is closed, the plaintext key cannot be viewed again.",
                })}
              </p>
              <Textarea
                classNames={{ label: "text-xs font-medium text-default-500" }}
                isReadOnly
                label={t("apiKeys.dialog.created.plaintextLabel", {
                  defaultValue: "Plaintext key",
                })}
                labelPlacement="outside"
                minRows={4}
                size="sm"
                value={createdKey.plaintext_key}
              />
              <div className="flex flex-wrap gap-2">
                <Button
                  startContent={<Copy className="h-4 w-4" />}
                  variant="flat"
                  onPress={() =>
                    copyText(createdKey.plaintext_key, {
                      successTitle: t("apiKeys.dialog.created.copyPlaintext", {
                        defaultValue: "Copy plaintext key",
                      }),
                      successDescription: t("apiKeys.notifications.copyPlaintextSuccess", {
                        defaultValue: "Copied plaintext key.",
                      }),
                      errorTitle: t("apiKeys.dialog.created.copyPlaintext", {
                        defaultValue: "Copy plaintext key",
                      }),
                      errorDescription: t("apiKeys.notifications.copyPlaintextFailed", {
                        defaultValue: "Failed to copy plaintext key.",
                      }),
                    })
                  }
                >
                  {t("apiKeys.dialog.created.copyPlaintext", {
                    defaultValue: "Copy plaintext key",
                  })}
                </Button>
                <Button variant="light" onPress={() => setCreatedKey(null)}>
                  {t("apiKeys.dialog.created.close", { defaultValue: "Close" })}
                </Button>
              </div>
            </CardBody>
          </Card>
        ) : null}
      </div>

      <Card>
        <CardHeader className="flex flex-col items-start gap-4 px-5 pb-4 pt-5">
          <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
            <div className="space-y-2">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                  {t("apiKeys.listTitle", {
                    defaultValue: "Key Pool",
                  })}
                </h2>
                <Chip size="sm" variant="flat">
                  {t("common.table.totalItems", { count: keys.length })}
                </Chip>
                <Chip color="success" size="sm" variant="flat">
                  {t("apiKeys.filters.active")} · {activeCount}
                </Chip>
                <Chip color="default" size="sm" variant="flat">
                  {t("apiKeys.filters.revoked")} · {revokedCount}
                </Chip>
              </div>
              <p className="text-sm leading-6 text-default-600">
                {t("apiKeys.subtitle", {
                  defaultValue:
                    "Manage the standalone workspace key pool and issue secure access credentials.",
                })}
              </p>
            </div>

            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-[minmax(18rem,1fr)_12rem_10rem]">
              <Input
                aria-label={t("common.table.searchLabel")}
                label={t("common.table.searchLabel")}
                labelPlacement="outside"
                placeholder={t("apiKeys.search", {
                  defaultValue: "Search key name or prefix…",
                })}
                startContent={<Search className="h-4 w-4 text-default-400" />}
                value={searchValue}
                onValueChange={(value) => {
                  setSearchValue(value);
                  setCurrentPage(1);
                }}
              />
              <Select
                aria-label={t("apiKeys.filters.label")}
                label={t("apiKeys.filters.label")}
                labelPlacement="outside"
                selectedKeys={new Set([statusFilter])}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection);
                  if (!nextValue) {
                    return;
                  }
                  setStatusFilter(nextValue as StatusFilter);
                  setCurrentPage(1);
                }}
              >
                <SelectItem key="all">{t("apiKeys.filters.all")}</SelectItem>
                <SelectItem key="active">
                  {t("apiKeys.filters.active")}
                </SelectItem>
                <SelectItem key="revoked">
                  {t("apiKeys.filters.revoked")}
                </SelectItem>
              </Select>
              <Select
                aria-label={t("common.table.rowsPerPage")}
                label={t("common.table.rowsPerPage")}
                labelPlacement="outside"
                selectedKeys={new Set([String(rowsPerPage)])}
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
          </div>
        </CardHeader>

        <CardBody className="gap-4 px-5 pb-5 pt-0">
          <Table
            isHeaderSticky
            aria-label={t("nav.apiKeys", { defaultValue: "Key Pool" })}
            classNames={{
              base: "min-h-[24rem]",
              wrapper: "bg-transparent px-0 py-0 shadow-none",
              th: "bg-default-100/60 text-xs font-semibold uppercase tracking-[0.12em] text-default-500",
              td: "align-top py-4 text-sm text-foreground",
              tr: "data-[hover=true]:bg-content2/35 transition-colors",
              emptyWrapper: "h-48",
            }}
          >
            <TableHeader>
              <TableColumn>{t("apiKeys.columns.name")}</TableColumn>
              <TableColumn>{t("apiKeys.columns.key")}</TableColumn>
              <TableColumn>{t("apiKeys.columns.status")}</TableColumn>
              <TableColumn>{t("apiKeys.columns.issued")}</TableColumn>
              <TableColumn>{t("apiKeys.columns.actions")}</TableColumn>
            </TableHeader>
            <TableBody
              emptyContent={
                <div className="flex flex-col items-center gap-3 py-10 text-default-500">
                  <KeyRound className="h-10 w-10 opacity-35" />
                  <div className="text-sm font-medium">
                    {t("apiKeys.empty")}
                  </div>
                </div>
              }
              isLoading={isLoading}
              items={paginatedKeys}
              loadingContent={<Spinner label={t("apiKeys.loading")} />}
            >
              {(key) => {
                const isPending =
                  pendingKeyId === key.id && toggleMutation.isPending;
                return (
                  <TableRow key={key.id}>
                    <TableCell>
                      <div className="min-w-[220px] space-y-1">
                        <div className="font-medium text-foreground">
                          {key.name}
                        </div>
                        <div className="text-xs leading-5 text-default-500">
                          {key.tenant_id ?? t("apiKeys.defaultTenant")}
                        </div>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex min-w-[240px] items-center gap-2">
                        <div className="font-mono text-xs text-default-500">
                          {key.key_prefix}****************
                        </div>
                        <Tooltip content={t("apiKeys.actions.copyPrefixTitle")}>
                          <Button
                            isIconOnly
                            size="sm"
                            variant="light"
                            aria-label={t("apiKeys.actions.copyPrefix")}
                            onPress={() =>
                              copyText(key.key_prefix, {
                                successTitle: t("apiKeys.actions.copyPrefix"),
                                successDescription: t("apiKeys.notifications.copyPrefixSuccess", {
                                  defaultValue: "Copied key prefix.",
                                }),
                                errorTitle: t("apiKeys.actions.copyPrefix"),
                                errorDescription: t("apiKeys.notifications.copyPrefixFailed", {
                                  defaultValue: "Failed to copy key prefix.",
                                }),
                              })
                            }
                          >
                            <Copy className="h-4 w-4" />
                          </Button>
                        </Tooltip>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[140px]">
                        <Chip
                          color={key.enabled ? "success" : "default"}
                          size="sm"
                          variant="flat"
                        >
                          {key.enabled
                            ? t("apiKeys.status.active")
                            : t("apiKeys.status.revoked")}
                        </Chip>
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="min-w-[180px] font-mono text-xs text-default-500">
                        {formatDateTime(key.created_at)}
                      </div>
                    </TableCell>
                    <TableCell>
                      <div className="flex min-w-[180px] flex-wrap gap-2">
                        <Button
                          isDisabled={isPending}
                          size="sm"
                          variant="light"
                          onPress={() =>
                            toggleMutation.mutate({
                              keyId: key.id,
                              enabled: !key.enabled,
                            })
                          }
                        >
                          {isPending ? (
                            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                          ) : null}
                          {isPending
                            ? t("apiKeys.actions.processing")
                            : key.enabled
                              ? t("apiKeys.actions.disable")
                              : t("apiKeys.actions.enable")}
                        </Button>
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
                start: visibleRangeStart,
                end: visibleRangeEnd,
                total: filteredKeys.length,
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
    </PageContent>
  );
}
