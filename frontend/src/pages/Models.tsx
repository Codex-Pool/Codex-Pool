import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  Button,
  Card,
  CardBody,
  CardHeader,
  Chip,
  Divider,
  Dropdown,
  DropdownItem,
  DropdownMenu,
  DropdownTrigger,
  Input,
  Select,
  SelectItem,
  Spinner,
  type Selection,
} from "@heroui/react";
import {
  ChevronDown,
  Copy,
  Database,
  ExternalLink,
  RefreshCcw,
  Search,
  ShieldCheck,
  ShieldX,
  Sparkles,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { localizeApiErrorDisplay } from "@/api/errorI18n";
import {
  modelsApi,
  type AdminModelSectionItem,
  type ListModelsResponse,
  type ModelAvailabilityStatus,
  type ModelSchema,
} from "@/api/models";
import {
  AntigravityDialogActions,
  AntigravityDialogBody,
  AntigravityDialogMeta,
  AntigravityDialogPanel,
  AntigravityDialogShell,
} from "@/components/layout/dialog-archetypes";
import {
  DockedPageIntro,
  PageContent,
} from "@/components/layout/page-archetypes";
import { Dialog } from "@/components/ui/dialog";
import { notify } from "@/lib/notification";
import { cn } from "@/lib/utils";

type AvailabilityFilter = "all" | ModelAvailabilityStatus;

const EMPTY_MODELS: ModelSchema[] = [];

function normalizeSelection(selection: Selection) {
  if (selection === "all") {
    return "";
  }

  const [first] = Array.from(selection);
  return first === undefined ? "" : String(first);
}

function formatDateTime(value?: string | null) {
  if (!value) {
    return "-";
  }

  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? "-" : parsed.toLocaleString();
}

function formatUsdPerMillion(
  value: number | null | undefined,
  fallback: string,
) {
  if (value == null) {
    return fallback;
  }

  return `$${(value / 1_000_000).toFixed(4)}`;
}

function formatTokenCount(value: number | null | undefined, fallback: string) {
  if (value == null || value <= 0) {
    return fallback;
  }
  if (value >= 1_000_000) {
    return `${(value / 1_000_000).toFixed(1)}M`;
  }
  if (value >= 1_000) {
    return `${Math.round(value / 1_000)}K`;
  }
  return String(value);
}

function getAvailabilityColor(status: ModelAvailabilityStatus) {
  switch (status) {
    case "available":
      return "success" as const;
    case "unavailable":
      return "danger" as const;
    case "unknown":
    default:
      return "default" as const;
  }
}

function getAvailabilityLabel(
  status: ModelAvailabilityStatus,
  t: ReturnType<typeof useTranslation>["t"],
) {
  switch (status) {
    case "available":
      return t("models.availability.available");
    case "unavailable":
      return t("models.availability.unavailable");
    case "unknown":
    default:
      return t("models.availability.unknown");
  }
}

function getPricingSourceLabel(
  source: string,
  t: ReturnType<typeof useTranslation>["t"],
) {
  switch (source) {
    case "official_sync":
      return t("models.pricing.sourceLabels.officialSync");
    case "manual_override":
      return t("models.pricing.sourceLabels.manualOverride");
    case "probe_only":
      return t("models.pricing.sourceLabels.probeOnly");
    default:
      return t("models.pricing.sourceLabels.unknown");
  }
}

function resolveNoValueLabel(t: ReturnType<typeof useTranslation>["t"]) {
  return t("models.antigravity.notAvailable");
}

function resolveModelName(model: ModelSchema) {
  return (
    model.official?.display_name ||
    model.official?.title ||
    model.id
  );
}

function resolveModelDescription(
  model: ModelSchema,
  t: ReturnType<typeof useTranslation>["t"],
) {
  return (
    model.official?.tagline ||
    model.official?.description ||
    t("models.antigravity.noDescription")
  );
}

function resolveModelFamilyLabel(
  model: ModelSchema,
  t: ReturnType<typeof useTranslation>["t"],
) {
  return (
    model.official?.family_label ||
    model.official?.family ||
    t("models.antigravity.ungroupedFamily")
  );
}

function matchModelSearch(model: ModelSchema, keyword: string) {
  const haystack = [
    model.id,
    model.owned_by,
    model.official?.title,
    model.official?.display_name,
    model.official?.tagline,
    model.official?.family,
    model.official?.family_label,
    model.official?.description,
    model.official?.input_modalities.join(" "),
    model.official?.output_modalities.join(" "),
    model.official?.endpoints.join(" "),
    model.official?.supported_features.join(" "),
    model.official?.supported_tools.join(" "),
    model.official?.snapshots.join(" "),
    model.official?.modality_items
      .map((item) => `${item.label} ${item.detail ?? ""}`)
      .join(" "),
    model.official?.endpoint_items
      .map((item) => `${item.label} ${item.detail ?? ""}`)
      .join(" "),
    model.official?.feature_items
      .map((item) => `${item.label} ${item.detail ?? ""}`)
      .join(" "),
    model.official?.tool_items
      .map((item) => `${item.label} ${item.detail ?? ""}`)
      .join(" "),
    model.effective_pricing.source,
    model.availability_error,
  ]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();

  return haystack.includes(keyword);
}

function buildCatalogAttention(
  payload: ListModelsResponse["meta"] | undefined,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (!payload) {
    return null;
  }

  if (payload.catalog_sync_required) {
    return t("models.antigravity.catalogAttentionSyncRequired");
  }

  if (payload.catalog_last_error) {
    return t("models.antigravity.catalogAttentionRetry");
  }

  if (payload.probe_cache_stale) {
    return t("models.antigravity.catalogAttentionCacheStale");
  }

  return null;
}

function describeAvailabilityOutcome(
  model: ModelSchema,
  t: ReturnType<typeof useTranslation>["t"],
) {
  if (model.availability_status === "available") {
    return t("models.antigravity.availabilityOutcome.available");
  }

  if (
    model.availability_status === "unavailable" &&
    model.availability_http_status != null
  ) {
    return t("models.antigravity.availabilityOutcome.unavailableWithStatus", {
      status: model.availability_http_status,
    });
  }

  if (model.availability_status === "unavailable") {
    return t("models.antigravity.availabilityOutcome.unavailable");
  }

  return t("models.antigravity.availabilityOutcome.unknown");
}

function buildModelsSummary(
  payload: ListModelsResponse | undefined,
  t: ReturnType<typeof useTranslation>["t"],
) {
  const models = payload?.data ?? [];
  const providers = new Set(
    models.map((model) => model.owned_by).filter(Boolean),
  );
  const available = models.filter(
    (model) => model.availability_status === "available",
  ).length;
  const unavailable = models.filter(
    (model) => model.availability_status === "unavailable",
  ).length;

  return [
    {
      title: t("models.antigravity.metrics.total"),
      value: models.length,
      description: t("models.antigravity.metrics.totalDesc"),
      icon: Database,
      toneClassName: "bg-primary/10 text-primary",
    },
    {
      title: t("models.antigravity.metrics.available"),
      value: available,
      description: t("models.antigravity.metrics.availableDesc"),
      icon: ShieldCheck,
      toneClassName: "bg-success/10 text-success",
    },
    {
      title: t("models.antigravity.metrics.unavailable"),
      value: unavailable,
      description: t("models.antigravity.metrics.unavailableDesc"),
      icon: ShieldX,
      toneClassName: "bg-danger/10 text-danger",
    },
    {
      title: t("models.antigravity.metrics.providers"),
      value: providers.size,
      description: t("models.antigravity.metrics.providersDesc"),
      icon: Sparkles,
      toneClassName: "bg-secondary/10 text-secondary",
    },
  ];
}

function localizeCapabilityDetail(
  detail: string | null | undefined,
  t: ReturnType<typeof useTranslation>["t"],
) {
  switch (detail) {
    case "Supported":
      return t("models.detail.capabilityDetails.supported");
    case "Not supported":
      return t("models.detail.capabilityDetails.notSupported");
    case "Input and output":
      return t("models.detail.capabilityDetails.inputAndOutput");
    case "Input only":
      return t("models.detail.capabilityDetails.inputOnly");
    case "Output only":
      return t("models.detail.capabilityDetails.outputOnly");
    default:
      return detail;
  }
}

function sanitizeSvg(svg?: string | null) {
  if (!svg) {
    return null;
  }
  const trimmed = svg.trim();
  if (!trimmed.startsWith("<svg")) {
    return null;
  }
  return trimmed
    .replace(/<script[\s\S]*?<\/script>/gi, "")
    .replace(/\son[a-z-]+="[^"]*"/gi, "")
    .replace(/\son[a-z-]+='[^']*'/gi, "");
}

function SummaryMetric({
  title,
  value,
  description,
  icon: Icon,
  toneClassName,
}: {
  title: string;
  value: number;
  description: string;
  icon: typeof Database;
  toneClassName: string;
}) {
  return (
    <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-4">
      <div className="flex items-start justify-between gap-3">
        <div
          className={cn(
            "flex h-10 w-10 items-center justify-center rounded-large",
            toneClassName,
          )}
        >
          <Icon className="h-4 w-4" />
        </div>
      </div>
      <div className="mt-5 text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
        {title}
      </div>
      <div className="mt-2 text-3xl font-semibold tracking-[-0.04em] text-foreground">
        {value}
      </div>
      <p className="mt-2 text-xs leading-5 text-default-500">{description}</p>
    </div>
  );
}

function OfficialSvgIcon({
  svg,
  className,
}: {
  svg?: string | null;
  className?: string;
}) {
  const sanitized = sanitizeSvg(svg);

  if (!sanitized) {
    return (
      <div
        className={cn(
          "h-2.5 w-2.5 rounded-full bg-default-400/70",
          className,
        )}
      />
    );
  }

  return (
    <span
      aria-hidden="true"
      className={cn(
        "flex h-5 w-5 items-center justify-center text-default-700 [&_svg]:h-5 [&_svg]:w-5",
        className,
      )}
      dangerouslySetInnerHTML={{ __html: sanitized }}
    />
  );
}

function ModelAvatar({
  model,
  className,
}: {
  model: ModelSchema;
  className?: string;
}) {
  const name = resolveModelName(model);
  const initial = name.trim().charAt(0).toUpperCase() || "M";

  if (model.official?.avatar_url) {
    return (
      <div
        className={cn(
          "flex h-12 w-12 shrink-0 overflow-hidden rounded-2xl border border-default-200 bg-content2",
          className,
        )}
      >
        <img
          alt={name}
          className="h-full w-full object-cover"
          src={model.official.avatar_url}
        />
      </div>
    );
  }

  return (
    <div
      className={cn(
        "flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl border border-default-200 bg-content2 text-lg font-semibold text-default-600",
        className,
      )}
    >
      {initial}
    </div>
  );
}

function ModelPriceTile({
  label,
  value,
  toneClassName,
}: {
  label: string;
  value: string;
  toneClassName?: string;
}) {
  return (
    <div className="rounded-2xl border border-default-200 bg-content2/70 px-3 py-3">
      <div className="text-[11px] font-semibold uppercase tracking-[0.12em] text-default-500">
        {label}
      </div>
      <div
        className={cn(
          "mt-2 text-sm font-semibold text-foreground",
          toneClassName,
        )}
      >
        {value}
      </div>
    </div>
  );
}

function CapabilityItemCard({
  item,
  t,
}: {
  item: AdminModelSectionItem;
  t: ReturnType<typeof useTranslation>["t"];
}) {
  return (
    <div className="flex gap-3 rounded-2xl border border-default-200 bg-content2/55 px-3 py-3">
      <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-content1 shadow-sm">
        <OfficialSvgIcon svg={item.icon_svg} />
      </div>
      <div className="min-w-0">
        <div className="text-sm font-semibold text-foreground">{item.label}</div>
        <div className="mt-1 text-xs leading-5 text-default-500">
          {localizeCapabilityDetail(item.detail, t) ||
            t("models.antigravity.notAvailable")}
        </div>
      </div>
    </div>
  );
}

function CapabilitySection({
  title,
  items,
  t,
}: {
  title: string;
  items: AdminModelSectionItem[];
  t: ReturnType<typeof useTranslation>["t"];
}) {
  return (
    <AntigravityDialogPanel tone="primary">
      <div className="flex items-center justify-between gap-3">
        <h3 className="text-base font-semibold tracking-[-0.02em] text-foreground">
          {title}
        </h3>
        <Chip size="sm" variant="flat">
          {items.length}
        </Chip>
      </div>
      {items.length > 0 ? (
        <div className="grid gap-3 md:grid-cols-2">
          {items.map((item) => (
            <CapabilityItemCard key={`${item.key}-${item.detail ?? "empty"}`} item={item} t={t} />
          ))}
        </div>
      ) : (
        <div className="text-sm leading-6 text-default-500">
          {t("models.antigravity.noSectionItems")}
        </div>
      )}
    </AntigravityDialogPanel>
  );
}

function SnapshotSection({
  model,
  t,
}: {
  model: ModelSchema;
  t: ReturnType<typeof useTranslation>["t"];
}) {
  const snapshotItems = model.official?.snapshot_items ?? [];

  return (
    <AntigravityDialogPanel tone="secondary">
      <div className="flex items-center justify-between gap-3">
        <h3 className="text-base font-semibold tracking-[-0.02em] text-foreground">
          {t("models.antigravity.sections.snapshots")}
        </h3>
        <Chip size="sm" variant="flat">
          {snapshotItems.length}
        </Chip>
      </div>

      {snapshotItems.length > 0 ? (
        <div className="space-y-3">
          {snapshotItems.map((item) => (
            <div
              key={item.alias}
              className="rounded-2xl border border-default-200 bg-content2/55 px-4 py-4"
            >
              <div className="flex items-start justify-between gap-4">
                <div>
                  <div className="text-sm font-semibold text-foreground">
                    {item.label}
                  </div>
                  <div className="mt-1 font-mono text-xs text-default-500">
                    {item.alias}
                  </div>
                </div>
                <Chip color="primary" size="sm" variant="flat">
                  {item.latest_snapshot ||
                    t("models.antigravity.notAvailable")}
                </Chip>
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {item.versions.length > 0 ? (
                  item.versions.map((version) => (
                    <Chip
                      key={version}
                      className="font-mono"
                      size="sm"
                      variant="bordered"
                    >
                      {version}
                    </Chip>
                  ))
                ) : (
                  <div className="text-sm text-default-500">
                    {t("models.antigravity.noSectionItems")}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-sm leading-6 text-default-500">
          {t("models.antigravity.noSectionItems")}
        </div>
      )}
    </AntigravityDialogPanel>
  );
}

function ModelDirectoryCard({
  model,
  noValueLabel,
  onOpen,
  onCopy,
  t,
}: {
  model: ModelSchema;
  noValueLabel: string;
  onOpen: (modelId: string) => void;
  onCopy: (modelId: string) => void;
  t: ReturnType<typeof useTranslation>["t"];
}) {
  const summaryItems = [
    model.official?.modality_items?.[0]?.label,
    model.official?.endpoint_items?.[0]?.label,
    model.official?.feature_items?.[0]?.label,
    model.official?.tool_items?.[0]?.label,
  ].filter(Boolean) as string[];

  return (
    <div
      className="group flex h-full flex-col rounded-[28px] border border-default-200 bg-content1/90 p-5 shadow-[0_18px_48px_-34px_rgba(15,23,42,0.45)] transition duration-200 hover:border-default-300 hover:bg-content1 hover:shadow-[0_24px_56px_-36px_rgba(15,23,42,0.52)]"
      role="button"
      tabIndex={0}
      onClick={() => onOpen(model.id)}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onOpen(model.id);
        }
      }}
    >
      <div className="flex items-start justify-between gap-4">
        <div className="flex min-w-0 items-start gap-3">
          <ModelAvatar model={model} />
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <div className="truncate text-base font-semibold tracking-[-0.03em] text-foreground">
                {resolveModelName(model)}
              </div>
              {model.official?.deprecated ? (
                <Chip color="warning" size="sm" variant="flat">
                  {t("models.antigravity.deprecated")}
                </Chip>
              ) : null}
            </div>
            <div className="mt-1 font-mono text-xs text-default-500">
              {model.id}
            </div>
          </div>
        </div>
        <Chip
          color={getAvailabilityColor(model.availability_status)}
          size="sm"
          variant="flat"
        >
          {getAvailabilityLabel(model.availability_status, t)}
        </Chip>
      </div>

      <p className="mt-4 min-h-[3rem] text-sm leading-6 text-default-600">
        {resolveModelDescription(model, t)}
      </p>

      <div className="mt-4 grid gap-2 md:grid-cols-3">
        <ModelPriceTile
          label={t("models.columns.inputPrice")}
          value={formatUsdPerMillion(
            model.effective_pricing.input_price_microcredits,
            noValueLabel,
          )}
        />
        <ModelPriceTile
          label={t("models.columns.cachedInputPrice")}
          value={formatUsdPerMillion(
            model.effective_pricing.cached_input_price_microcredits,
            noValueLabel,
          )}
        />
        <ModelPriceTile
          label={t("models.columns.outputPrice")}
          value={formatUsdPerMillion(
            model.effective_pricing.output_price_microcredits,
            noValueLabel,
          )}
        />
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        <Chip size="sm" variant="flat">
          {model.owned_by}
        </Chip>
        <Chip size="sm" variant="flat">
          {getPricingSourceLabel(model.effective_pricing.source, t)}
        </Chip>
        {model.official?.context_window_tokens ? (
          <Chip size="sm" variant="bordered">
            {t("models.detail.contextWindow")}:{" "}
            {formatTokenCount(model.official.context_window_tokens, noValueLabel)}
          </Chip>
        ) : null}
        {summaryItems.map((item) => (
          <Chip key={item} size="sm" variant="bordered">
            {item}
          </Chip>
        ))}
      </div>

      <div className="mt-auto pt-4">
        <div className="rounded-2xl bg-content2/60 px-3 py-3 text-xs leading-5 text-default-500">
          {describeAvailabilityOutcome(model, t)}
        </div>
        <div className="mt-4 flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="flat"
            onClick={(event) => event.stopPropagation()}
            onPress={() => onOpen(model.id)}
          >
            {t("models.actions.openDetails")}
          </Button>
          <Button
            size="sm"
            startContent={<Copy className="h-4 w-4" />}
            variant="light"
            onClick={(event) => event.stopPropagation()}
            onPress={() => onCopy(model.id)}
          >
            {t("models.actions.copyModelId")}
          </Button>
        </div>
      </div>
    </div>
  );
}

export default function Models() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [searchValue, setSearchValue] = useState("");
  const [providerFilter, setProviderFilter] = useState("all");
  const [availabilityFilter, setAvailabilityFilter] =
    useState<AvailabilityFilter>("all");
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);

  const {
    data: modelsPayload,
    isLoading,
    isFetching,
    refetch,
  } = useQuery({
    queryKey: ["models"],
    queryFn: modelsApi.listModels,
    refetchInterval: 60_000,
  });

  const syncMutation = useMutation({
    mutationFn: modelsApi.syncOpenAiCatalog,
    onSuccess: async (result) => {
      await queryClient.invalidateQueries({ queryKey: ["models"] });
      notify({
        variant: "success",
        title: t("models.actions.syncOpenAiCatalog"),
        description: t("models.notice.openAiCatalogSynced", {
          count: result.created_or_updated,
        }),
      });
    },
    onError: async (error) => {
      await queryClient.invalidateQueries({ queryKey: ["models"] });
      const fallback = t("models.errors.openAiCatalogSyncFailed");
      notify({
        variant: "error",
        title: t("models.actions.syncOpenAiCatalog"),
        description: localizeApiErrorDisplay(t, error, fallback).label,
      });
    },
  });

  const probeMutation = useMutation({
    mutationFn: () => modelsApi.probeModels({ force: true }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["models"] });
      notify({
        variant: "success",
        title: t("models.actions.probeAvailability"),
        description: t("models.notice.probeCompleted"),
      });
    },
    onError: (error) => {
      const fallback = t("models.errors.probeFailed");
      notify({
        variant: "error",
        title: t("models.actions.probeAvailability"),
        description: localizeApiErrorDisplay(t, error, fallback).label,
      });
    },
  });

  const models = useMemo(
    () => modelsPayload?.data ?? EMPTY_MODELS,
    [modelsPayload?.data],
  );
  const meta = modelsPayload?.meta;
  const noValueLabel = resolveNoValueLabel(t);
  const summaryCards = useMemo(
    () => buildModelsSummary(modelsPayload, t),
    [modelsPayload, t],
  );
  const catalogAttention = useMemo(
    () => buildCatalogAttention(meta, t),
    [meta, t],
  );

  const providerOptions = useMemo(
    () =>
      [...new Set(models.map((model) => model.owned_by).filter(Boolean))].sort(
        (left, right) => left.localeCompare(right),
      ),
    [models],
  );

  const filteredModels = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase();

    return models.filter((model) => {
      if (providerFilter !== "all" && model.owned_by !== providerFilter) {
        return false;
      }
      if (
        availabilityFilter !== "all" &&
        model.availability_status !== availabilityFilter
      ) {
        return false;
      }
      if (keyword && !matchModelSearch(model, keyword)) {
        return false;
      }
      return true;
    });
  }, [availabilityFilter, models, providerFilter, searchValue]);

  const groupedModels = useMemo(() => {
    const groups = new Map<string, { label: string; items: ModelSchema[] }>();

    for (const model of filteredModels) {
      const key = model.official?.family || "ungrouped";
      const label = resolveModelFamilyLabel(model, t);
      const existing = groups.get(key);
      if (existing) {
        existing.items.push(model);
        continue;
      }
      groups.set(key, { label, items: [model] });
    }

    return [...groups.entries()]
      .map(([key, group]) => ({
        key,
        label: group.label,
        items: group.items.sort((left, right) =>
          resolveModelName(left).localeCompare(resolveModelName(right)),
        ),
      }))
      .sort((left, right) => left.label.localeCompare(right.label));
  }, [filteredModels, t]);

  const selectedModel = useMemo(
    () => models.find((model) => model.id === selectedModelId) ?? null,
    [models, selectedModelId],
  );

  const pricingNoteItems = useMemo(() => {
    if (!selectedModel) {
      return [];
    }
    if (selectedModel.official?.pricing_note_items?.length) {
      return selectedModel.official.pricing_note_items;
    }
    if (selectedModel.official?.pricing_notes) {
      return [selectedModel.official.pricing_notes];
    }
    return [];
  }, [selectedModel]);

  async function copyModelId(modelId: string) {
    try {
      await navigator.clipboard.writeText(modelId);
      notify({
        variant: "success",
        title: t("models.actions.copyModelId"),
        description: t("models.antigravity.copyModelIdSuccess", {
          modelId,
        }),
      });
    } catch (error) {
      const fallback = t("models.antigravity.copyModelIdFailed");
      notify({
        variant: "error",
        title: t("models.actions.copyModelId"),
        description: localizeApiErrorDisplay(t, error, fallback).label,
      });
    }
  }

  return (
    <PageContent className="space-y-6">
      <DockedPageIntro
        archetype="workspace"
        title={t("models.title")}
        description={t("models.subtitle")}
        actions={
          <div className="flex flex-wrap gap-2">
            <Dropdown>
              <DropdownTrigger>
                <Button
                  color="primary"
                  endContent={
                    probeMutation.isPending ||
                    syncMutation.isPending ? undefined : (
                      <ChevronDown className="h-4 w-4" />
                    )
                  }
                  isDisabled={probeMutation.isPending || syncMutation.isPending}
                  isLoading={probeMutation.isPending || syncMutation.isPending}
                  startContent={<Sparkles className="h-4 w-4" />}
                  variant="flat"
                >
                  {t("models.antigravity.maintenance")}
                </Button>
              </DropdownTrigger>
              <DropdownMenu
                aria-label={t("models.antigravity.maintenance")}
                disabledKeys={
                  probeMutation.isPending || syncMutation.isPending
                    ? ["probe", "sync"]
                    : []
                }
                onAction={(key) => {
                  if (String(key) === "probe") {
                    probeMutation.mutate();
                    return;
                  }
                  if (String(key) === "sync") {
                    syncMutation.mutate();
                  }
                }}
              >
                <DropdownItem
                  key="probe"
                  description={t(
                    "models.antigravity.maintenanceProbeDescription",
                  )}
                  startContent={<Sparkles className="h-4 w-4" />}
                >
                  {t("models.actions.probeAvailability")}
                </DropdownItem>
                <DropdownItem
                  key="sync"
                  description={t(
                    "models.antigravity.maintenanceSyncDescription",
                  )}
                  startContent={<RefreshCcw className="h-4 w-4" />}
                >
                  {t("models.actions.syncOpenAiCatalog")}
                </DropdownItem>
              </DropdownMenu>
            </Dropdown>
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

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,0.95fr)]">
        <Card className="border-small border-default-200 bg-content1 shadow-small">
          <CardHeader className="px-5 pb-3 pt-5">
            <div>
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t("models.antigravity.summaryTitle")}
              </h2>
            </div>
          </CardHeader>
          <CardBody className="grid gap-3 px-5 pb-5 pt-1 sm:grid-cols-2">
            {summaryCards.map((card) => (
              <SummaryMetric key={card.title} {...card} />
            ))}
          </CardBody>
        </Card>

        <Card className="border-small border-default-200 bg-content1 shadow-small">
          <CardHeader className="px-5 pb-3 pt-5">
            <div>
              <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
                {t("models.antigravity.catalogTitle")}
              </h2>
            </div>
          </CardHeader>
          <CardBody className="gap-4 px-5 pb-5 pt-1">
            <div className="flex flex-wrap gap-2">
              <Chip
                color={meta?.probe_cache_stale ? "warning" : "success"}
                size="sm"
                variant="flat"
              >
                {meta?.probe_cache_stale
                  ? t("models.antigravity.cacheStale")
                  : t("models.antigravity.cacheFresh")}
              </Chip>
              <Chip
                color={meta?.catalog_sync_required ? "warning" : "primary"}
                size="sm"
                variant="flat"
              >
                {meta?.catalog_sync_required
                  ? t("models.antigravity.catalogNeedsSync")
                  : t("models.antigravity.catalogReady")}
              </Chip>
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-3">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t("models.antigravity.cacheUpdatedAt")}
                </div>
                <div className="mt-2 text-sm font-semibold text-foreground">
                  {formatDateTime(meta?.probe_cache_updated_at)}
                </div>
              </div>
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-3">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t("models.antigravity.probeSource")}
                </div>
                <div className="mt-2 text-sm font-semibold text-foreground">
                  {meta?.probe_source_account_label ??
                    t("models.probeSourceUnknown")}
                </div>
              </div>
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-3">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t("models.antigravity.catalogSyncedAt")}
                </div>
                <div className="mt-2 text-sm font-semibold text-foreground">
                  {formatDateTime(meta?.catalog_synced_at)}
                </div>
              </div>
              <div className="rounded-large border border-default-200 bg-content2/55 px-4 py-3">
                <div className="text-xs font-semibold uppercase tracking-[0.14em] text-default-500">
                  {t("models.antigravity.cacheTtl")}
                </div>
                <div className="mt-2 text-sm font-semibold text-foreground">
                  {t("models.antigravity.cacheTtlHours", {
                    hours: Math.round((meta?.probe_cache_ttl_sec ?? 0) / 3600),
                  })}
                </div>
              </div>
            </div>

            {catalogAttention ? (
              <div className="rounded-large border border-warning-200 bg-warning-50/80 px-4 py-3 text-sm leading-6 text-warning-700 dark:bg-warning/10 dark:text-warning-300">
                <div className="font-semibold">
                  {t("models.antigravity.catalogAttentionTitle")}
                </div>
                <div className="mt-1">{catalogAttention}</div>
              </div>
            ) : null}
          </CardBody>
        </Card>
      </div>

      <Card className="border-small border-default-200 bg-content1 shadow-small">
        <CardHeader className="flex flex-col items-start gap-4 px-5 pb-3 pt-5">
          <div>
            <h2 className="text-lg font-semibold tracking-[-0.02em] text-foreground">
              {t("models.antigravity.directoryTitle")}
            </h2>
          </div>

          <div className="flex w-full flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
            <div className="grid flex-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
              <Input
                aria-label={t("models.actions.search")}
                placeholder={t("models.actions.search")}
                size="sm"
                startContent={<Search className="h-4 w-4 text-default-400" />}
                value={searchValue}
                onValueChange={setSearchValue}
              />

              <Select
                aria-label={t("models.filters.providerLabel")}
                items={[
                  { key: "all", label: t("models.filters.allProviders") },
                  ...providerOptions.map((provider) => ({
                    key: provider,
                    label: provider,
                  })),
                ]}
                selectedKeys={[providerFilter]}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection);
                  if (!nextValue) {
                    return;
                  }
                  setProviderFilter(nextValue);
                }}
              >
                {(item) => <SelectItem key={item.key}>{item.label}</SelectItem>}
              </Select>

              <Select
                aria-label={t("models.filters.availabilityLabel")}
                selectedKeys={[availabilityFilter]}
                size="sm"
                onSelectionChange={(selection) => {
                  const nextValue = normalizeSelection(selection);
                  if (!nextValue) {
                    return;
                  }
                  setAvailabilityFilter(nextValue as AvailabilityFilter);
                }}
              >
                <SelectItem key="all">
                  {t("models.filters.allAvailability")}
                </SelectItem>
                <SelectItem key="available">
                  {t("models.availability.available")}
                </SelectItem>
                <SelectItem key="unavailable">
                  {t("models.availability.unavailable")}
                </SelectItem>
                <SelectItem key="unknown">
                  {t("models.availability.unknown")}
                </SelectItem>
              </Select>
            </div>

            <div className="flex items-center gap-2 text-xs text-default-500">
              <Chip size="sm" variant="flat">
                {filteredModels.length}
              </Chip>
            </div>
          </div>
        </CardHeader>

        <CardBody className="gap-6 px-5 pb-5 pt-0">
          {isLoading ? (
            <div className="flex min-h-[24rem] items-center justify-center">
              <Spinner label={t("common.loading")} />
            </div>
          ) : groupedModels.length > 0 ? (
            groupedModels.map((group) => (
              <section key={group.key} className="space-y-4">
                <div className="flex items-center justify-between gap-4 border-b border-default-200 pb-3">
                  <div>
                    <h3 className="text-base font-semibold tracking-[-0.02em] text-foreground">
                      {group.label}
                    </h3>
                    <p className="mt-1 text-sm text-default-500">
                      {group.items.length} {t("models.title")}
                    </p>
                  </div>
                  <Chip size="sm" variant="flat">
                    {group.items.length}
                  </Chip>
                </div>

                <div className="grid gap-4 md:grid-cols-2 2xl:grid-cols-3">
                  {group.items.map((model) => (
                    <ModelDirectoryCard
                      key={model.id}
                      model={model}
                      noValueLabel={noValueLabel}
                      onCopy={(modelId) => {
                        void copyModelId(modelId);
                      }}
                      onOpen={setSelectedModelId}
                      t={t}
                    />
                  ))}
                </div>
              </section>
            ))
          ) : (
            <div className="flex min-h-[22rem] flex-col items-center justify-center gap-3 rounded-[28px] border border-dashed border-default-300 bg-content2/40 px-6 py-10 text-center">
              <Database className="h-10 w-10 text-default-300" />
              <div className="text-base font-semibold text-foreground">
                {t("models.antigravity.emptyFilteredTitle")}
              </div>
              <div className="max-w-xl text-sm leading-6 text-default-500">
                {t("models.antigravity.emptyFilteredDescription")}
              </div>
            </div>
          )}
        </CardBody>
      </Card>

      <Dialog
        open={Boolean(selectedModel)}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedModelId(null);
          }
        }}
      >
        {selectedModel ? (
          <AntigravityDialogShell
            bodyClassName="space-y-5"
            description={resolveModelDescription(selectedModel, t)}
            meta={
              <AntigravityDialogMeta>
                <Chip size="sm" variant="flat">
                  {selectedModel.owned_by}
                </Chip>
                <Chip
                  color={getAvailabilityColor(selectedModel.availability_status)}
                  size="sm"
                  variant="flat"
                >
                  {getAvailabilityLabel(selectedModel.availability_status, t)}
                </Chip>
                <Chip size="sm" variant="bordered">
                  {resolveModelFamilyLabel(selectedModel, t)}
                </Chip>
              </AntigravityDialogMeta>
            }
            size="xl"
            title={
              <div className="flex items-start gap-4">
                <ModelAvatar className="h-14 w-14 rounded-[20px]" model={selectedModel} />
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <span>{resolveModelName(selectedModel)}</span>
                    {selectedModel.official?.deprecated ? (
                      <Chip color="warning" size="sm" variant="flat">
                        {t("models.antigravity.deprecated")}
                      </Chip>
                    ) : null}
                  </div>
                  <div className="mt-1 font-mono text-xs text-default-500">
                    {selectedModel.id}
                  </div>
                </div>
              </div>
            }
            footer={
              <AntigravityDialogActions>
                <Button
                  startContent={<Copy className="h-4 w-4" />}
                  variant="flat"
                  onPress={() => {
                    void copyModelId(selectedModel.id);
                  }}
                >
                  {t("models.actions.copyModelId")}
                </Button>
                <Button
                  as="a"
                  color="primary"
                  endContent={<ExternalLink className="h-4 w-4" />}
                  href={selectedModel.official?.source_url || "#"}
                  isDisabled={!selectedModel.official?.source_url}
                  rel="noreferrer"
                  target="_blank"
                >
                  {t("models.detail.openOfficialPage")}
                </Button>
              </AntigravityDialogActions>
            }
          >
            <AntigravityDialogBody>
              <div className="grid gap-4 xl:grid-cols-3">
                <AntigravityDialogPanel tone="primary">
                  <h3 className="text-base font-semibold tracking-[-0.02em] text-foreground">
                    {t("models.antigravity.sections.identity")}
                  </h3>
                  <div className="space-y-2 text-sm leading-6 text-default-600">
                    <div>
                      {t("models.columns.provider")}: {selectedModel.owned_by}
                    </div>
                    <div>
                      {t("models.antigravity.effectivePricingSource")}:{" "}
                      {getPricingSourceLabel(
                        selectedModel.effective_pricing.source,
                        t,
                      )}
                    </div>
                    <div>
                      {t("models.antigravity.catalogSyncedAt")}:{" "}
                      {formatDateTime(selectedModel.official?.synced_at)}
                    </div>
                    <div>
                      {t("models.columns.checkedAt")}:{" "}
                      {formatDateTime(selectedModel.availability_checked_at)}
                    </div>
                    <div>
                      {t("models.detail.httpStatus")}:{" "}
                      {selectedModel.availability_http_status ?? noValueLabel}
                    </div>
                    <div>
                      {t("models.antigravity.officialPageStatus")}:{" "}
                      {selectedModel.official?.source_url
                        ? t("models.antigravity.officialPageReady")
                        : t("models.antigravity.officialPageMissing")}
                    </div>
                  </div>
                  <Divider />
                  <div className="text-sm leading-6 text-default-500">
                    {describeAvailabilityOutcome(selectedModel, t)}
                  </div>
                </AntigravityDialogPanel>

                <AntigravityDialogPanel tone="secondary">
                  <h3 className="text-base font-semibold tracking-[-0.02em] text-foreground">
                    {t("models.antigravity.sections.pricing")}
                  </h3>
                  <div className="grid gap-3">
                    <ModelPriceTile
                      label={t("models.columns.inputPrice")}
                      value={formatUsdPerMillion(
                        selectedModel.effective_pricing.input_price_microcredits,
                        noValueLabel,
                      )}
                    />
                    <ModelPriceTile
                      label={t("models.columns.cachedInputPrice")}
                      value={formatUsdPerMillion(
                        selectedModel.effective_pricing
                          .cached_input_price_microcredits,
                        noValueLabel,
                      )}
                    />
                    <ModelPriceTile
                      label={t("models.columns.outputPrice")}
                      value={formatUsdPerMillion(
                        selectedModel.effective_pricing
                          .output_price_microcredits,
                        noValueLabel,
                      )}
                    />
                  </div>
                  <Divider />
                  <div className="space-y-2 text-sm leading-6 text-default-600">
                    <div>
                      {t("models.pricing.officialBase")}:{" "}
                      {formatUsdPerMillion(
                        selectedModel.official?.input_price_microcredits,
                        noValueLabel,
                      )}
                    </div>
                    <div>
                      {t("models.pricing.manualOverride")}:{" "}
                      {selectedModel.override_pricing
                        ? t("common.yes")
                        : t("common.no")}
                    </div>
                  </div>
                </AntigravityDialogPanel>

                <AntigravityDialogPanel tone="primary">
                  <h3 className="text-base font-semibold tracking-[-0.02em] text-foreground">
                    {t("models.antigravity.sections.limits")}
                  </h3>
                  <div className="space-y-2 text-sm leading-6 text-default-600">
                    <div>
                      {t("models.detail.contextWindow")}:{" "}
                      {formatTokenCount(
                        selectedModel.official?.context_window_tokens,
                        noValueLabel,
                      )}
                    </div>
                    <div>
                      {t("models.detail.maxInputTokens")}:{" "}
                      {formatTokenCount(
                        selectedModel.official?.max_input_tokens,
                        noValueLabel,
                      )}
                    </div>
                    <div>
                      {t("models.detail.maxOutputTokens")}:{" "}
                      {formatTokenCount(
                        selectedModel.official?.max_output_tokens,
                        noValueLabel,
                      )}
                    </div>
                    <div>
                      {t("models.detail.knowledgeCutoff")}:{" "}
                      {selectedModel.official?.knowledge_cutoff || noValueLabel}
                    </div>
                    <div>
                      {t("models.detail.reasoningTokenSupport")}:{" "}
                      {selectedModel.official?.reasoning_token_support
                        ? t("common.yes")
                        : t("common.no")}
                    </div>
                  </div>
                </AntigravityDialogPanel>
              </div>

              {pricingNoteItems.length > 0 ? (
                <AntigravityDialogPanel tone="secondary">
                  <h3 className="text-base font-semibold tracking-[-0.02em] text-foreground">
                    {t("models.antigravity.sections.pricingNotes")}
                  </h3>
                  <div className="space-y-3">
                    {pricingNoteItems.map((note, index) => (
                      <div
                        key={`${note}-${index}`}
                        className="rounded-2xl border border-default-200 bg-content2/55 px-4 py-3 text-sm leading-6 text-default-600"
                      >
                        {note}
                      </div>
                    ))}
                  </div>
                </AntigravityDialogPanel>
              ) : null}

              <div className="grid gap-4 xl:grid-cols-2">
                <CapabilitySection
                  items={selectedModel.official?.modality_items ?? []}
                  t={t}
                  title={t("models.antigravity.sections.modalities")}
                />
                <CapabilitySection
                  items={selectedModel.official?.endpoint_items ?? []}
                  t={t}
                  title={t("models.antigravity.sections.endpoints")}
                />
                <CapabilitySection
                  items={selectedModel.official?.feature_items ?? []}
                  t={t}
                  title={t("models.antigravity.sections.features")}
                />
                <CapabilitySection
                  items={selectedModel.official?.tool_items ?? []}
                  t={t}
                  title={t("models.antigravity.sections.tools")}
                />
              </div>

              <SnapshotSection model={selectedModel} t={t} />
            </AntigravityDialogBody>
          </AntigravityDialogShell>
        ) : null}
      </Dialog>
    </PageContent>
  );
}
