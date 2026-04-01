import { useMemo, useState } from "react";
import {
  Button,
  Checkbox,
  Chip,
  Form,
  Input,
  Progress,
  Textarea,
} from "@heroui/react";
import { motion, useReducedMotion } from "framer-motion";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import {
  AlertCircle,
  CheckCircle2,
  ExternalLink,
  Loader2,
  RefreshCcw,
  ShieldCheck,
} from "lucide-react";

import {
  oauthImportApi,
  type CodexOAuthLoginSession,
  type CodexOAuthLoginSessionStatus,
} from "@/api/oauthImport";
import {
  localizeApiErrorDisplay,
  localizeOAuthErrorCodeDisplay,
} from "@/api/errorI18n";
import {
  DockedPageIntro,
  PageContent,
} from "@/components/layout/page-archetypes";
import { Card, CardBody, CardHeader } from "@/components/ui/card";
import { getPlanLabel } from "@/features/accounts/utils";
import { notify } from "@/lib/notification";
import { cn } from "@/lib/utils";

const DEFAULT_BASE_URL = "https://chatgpt.com/backend-api/codex";
const DEFAULT_PRIORITY = 100;

type WizardStepKey = "setup" | "authorize" | "monitor" | "result";

function isTerminalStatus(status?: CodexOAuthLoginSessionStatus): boolean {
  return status === "completed" || status === "failed" || status === "expired";
}

function statusChipColor(status?: CodexOAuthLoginSessionStatus) {
  if (status === "completed") {
    return "success" as const;
  }
  if (status === "failed" || status === "expired") {
    return "danger" as const;
  }
  if (status === "exchanging" || status === "importing") {
    return "warning" as const;
  }
  if (status === "waiting_callback") {
    return "primary" as const;
  }
  return "default" as const;
}

function formatDateTime(value?: string) {
  if (!value) {
    return "-";
  }
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? "-" : parsed.toLocaleString();
}

function resolveWizardStep(
  sessionId: string | null,
  status?: CodexOAuthLoginSessionStatus,
): WizardStepKey {
  if (!sessionId) {
    return "setup";
  }
  if (!status || status === "waiting_callback") {
    return "authorize";
  }
  if (status === "exchanging" || status === "importing") {
    return "monitor";
  }
  return "result";
}

export default function OAuthImport() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const prefersReducedMotion = useReducedMotion();

  const [label, setLabel] = useState("");
  const [baseUrl, setBaseUrl] = useState(DEFAULT_BASE_URL);
  const [enabled, setEnabled] = useState(true);
  const [priorityInput, setPriorityInput] = useState(String(DEFAULT_PRIORITY));
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [manualRedirectUrl, setManualRedirectUrl] = useState("");

  const sessionQuery = useQuery({
    queryKey: ["codexOauthLoginSession", sessionId],
    queryFn: () => oauthImportApi.getCodexLoginSession(sessionId!),
    enabled: Boolean(sessionId),
    refetchInterval: (query) => {
      const data = query.state.data;
      if (!data) {
        return 2_000;
      }
      return isTerminalStatus(data.status) ? false : 2_000;
    },
  });

  const session = sessionQuery.data;
  const isBusy = createSessionMutationIsPending(sessionId, session);

  function openAuthorizeTab(authorizeUrl: string) {
    const tab = window.open(authorizeUrl, "_blank", "noopener,noreferrer");
    if (!tab) {
      notify({
        variant: "warning",
        title: t("oauthImport.notifications.popupBlockedTitle"),
        description: t("oauthImport.notifications.popupBlockedDescription"),
      });
    }
  }

  const createSessionMutation = useMutation({
    mutationFn: async () => {
      const normalizedPriority = Number.parseInt(priorityInput, 10);
      return oauthImportApi.createCodexLoginSession({
        label: label.trim() || undefined,
        base_url: baseUrl.trim() || undefined,
        enabled,
        priority: Number.isFinite(normalizedPriority)
          ? normalizedPriority
          : DEFAULT_PRIORITY,
      });
    },
    onSuccess: (created) => {
      setSessionId(created.session_id);
      setManualRedirectUrl("");
      queryClient.setQueryData(
        ["codexOauthLoginSession", created.session_id],
        created,
      );
      openAuthorizeTab(created.authorize_url);
      notify({
        variant: "info",
        title: t("oauthImport.notifications.sessionCreatedTitle"),
        description: t("oauthImport.notifications.sessionCreatedDescription"),
      });
    },
    onError: (error: unknown) => {
      notify({
        variant: "error",
        title: t("oauthImport.notifications.sessionCreateFailedTitle"),
        description: localizeApiErrorDisplay(
          t,
          error,
          t("oauthImport.notifications.unknownError"),
        ).label,
      });
    },
  });

  const submitManualCallbackMutation = useMutation({
    mutationFn: async () => {
      if (!sessionId) {
        throw new Error("session id is missing");
      }
      return oauthImportApi.submitCodexLoginCallback(
        sessionId,
        manualRedirectUrl.trim(),
      );
    },
    onSuccess: (updated) => {
      queryClient.setQueryData(
        ["codexOauthLoginSession", updated.session_id],
        updated,
      );
      notify({
        variant: updated.status === "completed" ? "success" : "info",
        title: t("oauthImport.notifications.manualSubmitTitle"),
        description:
          updated.status === "completed"
            ? t("oauthImport.notifications.manualSubmitSuccess")
            : t("oauthImport.notifications.manualSubmitAccepted"),
      });
    },
    onError: (error: unknown) => {
      notify({
        variant: "error",
        title: t("oauthImport.notifications.manualSubmitFailedTitle"),
        description: localizeApiErrorDisplay(
          t,
          error,
          t("oauthImport.notifications.unknownError"),
        ).label,
      });
    },
  });

  const currentStep = useMemo(
    () => resolveWizardStep(sessionId, session?.status),
    [session?.status, sessionId],
  );

  const steps = useMemo(
    () => [
      {
        key: "setup" as const,
        title: t("oauthImport.start.title"),
        description: t("oauthImport.start.description"),
      },
      {
        key: "authorize" as const,
        title: t("oauthImport.authorize.title", {
          defaultValue: "Complete OAuth authorization",
        }),
        description: t("oauthImport.authorize.description", {
          defaultValue:
            "Open the authorization window, finish sign-in, and wait for the callback.",
        }),
      },
      {
        key: "monitor" as const,
        title: t("oauthImport.monitor.title", {
          defaultValue: "Watch token exchange and import",
        }),
        description: t("oauthImport.monitor.description", {
          defaultValue:
            "The backend is exchanging credentials and importing the account into the pool.",
        }),
      },
      {
        key: "result" as const,
        title: t("oauthImport.result.reviewTitle", {
          defaultValue: "Review import result",
        }),
        description: t("oauthImport.result.reviewDescription", {
          defaultValue:
            "Confirm whether the account was created, updated, or requires a retry.",
        }),
      },
    ],
    [t],
  );

  const currentStepIndex = Math.max(
    steps.findIndex((step) => step.key === currentStep),
    0,
  );
  const progressValue = Math.round(
    ((currentStepIndex + 1) / steps.length) * 100,
  );
  const showManualFallback =
    Boolean(sessionId) && !isTerminalStatus(session?.status);
  const showResult = Boolean(session?.result && session.status === "completed");
  const showError = Boolean(
    session?.error &&
    (session.status === "failed" || session.status === "expired"),
  );
  const statusLabel = useMemo(() => {
    if (!session?.status) {
      return t("oauthImport.status.idle");
    }
    return t(`oauthImport.status.${session.status}`);
  }, [session?.status, t]);
  const localizedError = session?.error?.code
    ? localizeOAuthErrorCodeDisplay(t, session.error.code)
    : { label: t("oauthImport.error.failed") };

  const container = prefersReducedMotion
    ? undefined
    : {
        hidden: { opacity: 0 },
        show: { opacity: 1, transition: { staggerChildren: 0.08 } },
      };

  const item = prefersReducedMotion
    ? undefined
    : {
        hidden: { opacity: 0, y: 10 },
        show: {
          opacity: 1,
          y: 0,
          transition: { duration: 0.22, ease: [0.16, 1, 0.3, 1] as [number, number, number, number] },
        },
      };

  return (
    <PageContent className="overflow-y-auto py-6 md:py-8">
      <motion.div
        variants={container}
        initial={prefersReducedMotion ? undefined : "hidden"}
        animate={prefersReducedMotion ? undefined : "show"}
        className="space-y-5"
      >
        <motion.div variants={item}>
          <DockedPageIntro
            archetype="workspace"
            title={t("nav.oauthImport")}
            description={t("oauthImport.subtitle")}
          />
        </motion.div>

        <div className="grid gap-5 xl:grid-cols-[15rem_minmax(0,1fr)] xl:items-start">
          <motion.aside variants={item} className="space-y-5">
            <Card>
              <CardHeader className="px-5 pb-3 pt-5">
                <div className="space-y-1">
                  <p className="text-xs font-semibold uppercase tracking-[0.18em] text-default-500">
                    {t("oauthImport.wizard.label", {
                      defaultValue: "Import wizard",
                    })}
                  </p>
                  <p className="text-sm leading-6 text-default-600">
                    {t("oauthImport.wizard.description", {
                      defaultValue:
                        "Move through the OAuth flow step by step, keep callback fallback nearby, and finish with a pool-ready account result.",
                    })}
                  </p>
                </div>
              </CardHeader>
              <CardBody className="space-y-5 px-5 pb-5 pt-0">
                <div className="space-y-2">
                  <div className="flex items-center justify-between gap-3 text-xs font-medium text-default-500">
                    <span>
                      {t("oauthImport.wizard.progress", {
                        defaultValue: "Progress",
                      })}
                    </span>
                    <span>{progressValue}%</span>
                  </div>
                  <Progress
                    aria-label={t("oauthImport.wizard.progress", {
                      defaultValue: "Progress",
                    })}
                    color="primary"
                    size="sm"
                    value={progressValue}
                  />
                </div>

                <div className="space-y-4">
                  {steps.map((step, index) => {
                    const isCurrent = step.key === currentStep;
                    const isComplete = index < currentStepIndex;

                    return (
                      <div key={step.key} className="flex gap-3">
                        <div className="flex flex-col items-center">
                          <div
                            className={cn(
                              "flex h-8 w-8 items-center justify-center rounded-full border-small text-xs font-semibold",
                              isCurrent &&
                                "border-primary-200 bg-primary-50 text-primary-700 dark:bg-primary/10 dark:text-primary-300",
                              !isCurrent &&
                                isComplete &&
                                "border-success-200 bg-success-50 text-success-700 dark:bg-success/10 dark:text-success-300",
                              !isCurrent &&
                                !isComplete &&
                                "border-default-200 bg-content2 text-default-500",
                            )}
                          >
                            {isComplete ? (
                              <CheckCircle2 className="h-4 w-4" />
                            ) : (
                              index + 1
                            )}
                          </div>
                          {index < steps.length - 1 ? (
                            <div
                              className={cn(
                                "mt-2 h-full w-px",
                                isCurrent || isComplete
                                  ? "bg-primary-200 dark:bg-primary/40"
                                  : "bg-default-200",
                              )}
                            />
                          ) : null}
                        </div>
                        <div className="min-w-0 space-y-1 pb-2">
                          <div className="flex flex-wrap items-center gap-2">
                            <p
                              className={cn(
                                "text-sm font-semibold",
                                isCurrent
                                  ? "text-foreground"
                                  : "text-foreground/86",
                              )}
                            >
                              {step.title}
                            </p>
                            {isCurrent ? (
                              <Chip size="sm" color="primary" variant="flat">
                                {t("oauthImport.wizard.current", {
                                  defaultValue: "Current",
                                })}
                              </Chip>
                            ) : null}
                            {!isCurrent && isComplete ? (
                              <Chip size="sm" color="success" variant="flat">
                                {t("oauthImport.wizard.completed", {
                                  defaultValue: "Done",
                                })}
                              </Chip>
                            ) : null}
                          </div>
                          {isCurrent ? (
                            <p className="text-xs leading-5 text-default-500">
                              {step.description}
                            </p>
                          ) : null}
                        </div>
                      </div>
                    );
                  })}
                </div>

                <div className="rounded-large border-small border-default-200 bg-content2 p-3">
                  {sessionId ? (
                    <div className="space-y-2.5">
                      <div className="flex items-center justify-between gap-3">
                        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-default-500">
                          {t("oauthImport.status.label")}
                        </p>
                        <Chip
                          size="sm"
                          color={statusChipColor(session?.status)}
                          variant="flat"
                        >
                          {sessionQuery.isFetching &&
                          !isTerminalStatus(session?.status) ? (
                            <span className="inline-flex items-center gap-1">
                              <Loader2 className="h-3 w-3 animate-spin" />
                              {statusLabel}
                            </span>
                          ) : (
                            statusLabel
                          )}
                        </Chip>
                      </div>
                      <p className="break-all font-mono text-xs text-default-500">
                        {t("oauthImport.status.sessionId", {
                          id: session?.session_id ?? sessionId,
                        })}
                      </p>
                      <p className="text-xs text-default-500">
                        {t("oauthImport.status.expiresAt", {
                          time: formatDateTime(session?.expires_at),
                        })}
                      </p>
                    </div>
                  ) : (
                    <p className="text-sm leading-6 text-default-600">
                      {t("oauthImport.wizard.noSession", {
                        defaultValue:
                          "No session yet. Start from step one to open the OAuth flow.",
                      })}
                    </p>
                  )}
                </div>
              </CardBody>
            </Card>
          </motion.aside>

          <div className="space-y-5">
            <motion.div variants={item}>
              {currentStep === "setup" ? (
                <Card>
                  <CardHeader className="px-5 pb-3 pt-5">
                    <div className="space-y-2">
                      <div className="inline-flex items-center gap-2 rounded-full border-small border-default-200 bg-content2 px-3 py-1 text-xs font-semibold uppercase tracking-[0.16em] text-default-600">
                        <ShieldCheck className="h-3.5 w-3.5 text-primary" />
                        <span>{t("oauthImport.start.title")}</span>
                      </div>
                      <p className="max-w-[56rem] text-sm leading-6 text-default-600">
                        {t("oauthImport.start.description")}
                      </p>
                    </div>
                  </CardHeader>
                  <CardBody className="space-y-5 px-5 pb-5 pt-1">
                    <Form
                      className="grid gap-4 lg:grid-cols-2"
                      validationBehavior="native"
                      onSubmit={(event) => {
                        event.preventDefault();
                        createSessionMutation.mutate();
                      }}
                    >
                      <Input
                        label={t("oauthImport.form.label")}
                        labelPlacement="outside"
                        placeholder={t("oauthImport.form.labelPlaceholder")}
                        value={label}
                        onValueChange={setLabel}
                        classNames={{ inputWrapper: "bg-content2/70" }}
                      />
                      <Input
                        label={t("oauthImport.form.baseUrl")}
                        labelPlacement="outside"
                        placeholder={DEFAULT_BASE_URL}
                        value={baseUrl}
                        onValueChange={setBaseUrl}
                        classNames={{ inputWrapper: "bg-content2/70" }}
                      />
                      <Input
                        label={t("oauthImport.form.priority")}
                        labelPlacement="outside"
                        placeholder={String(DEFAULT_PRIORITY)}
                        value={priorityInput}
                        inputMode="numeric"
                        onValueChange={setPriorityInput}
                        classNames={{ inputWrapper: "bg-content2/70" }}
                      />
                      <div className="rounded-large border-small border-default-200 bg-content2 p-4">
                        <Checkbox
                          isSelected={enabled}
                          onValueChange={setEnabled}
                        >
                          {t("oauthImport.form.enabled")}
                        </Checkbox>
                      </div>

                      <div className="lg:col-span-2 flex flex-wrap items-center gap-3">
                        <Button
                          color="primary"
                          isLoading={createSessionMutation.isPending}
                          startContent={
                            createSessionMutation.isPending ? null : (
                              <ExternalLink className="h-4 w-4" />
                            )
                          }
                          type="submit"
                        >
                          {t("oauthImport.actions.startLogin")}
                        </Button>
                        <p className="text-sm leading-6 text-default-600">
                          {t("oauthImport.wizard.setupHint", {
                            defaultValue:
                              "Creating a session will immediately open the OAuth window and start polling for callback delivery.",
                          })}
                        </p>
                      </div>
                    </Form>
                  </CardBody>
                </Card>
              ) : null}

              {currentStep === "authorize" ? (
                <Card>
                  <CardHeader className="px-5 pb-3 pt-5">
                    <div className="space-y-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <Chip color="primary" variant="flat">
                          {t("oauthImport.authorize.title", {
                            defaultValue: "Complete OAuth authorization",
                          })}
                        </Chip>
                        <Chip
                          color={statusChipColor(session?.status)}
                          variant="flat"
                        >
                          {statusLabel}
                        </Chip>
                      </div>
                      <p className="max-w-[56rem] text-sm leading-6 text-default-600">
                        {t("oauthImport.authorize.description", {
                          defaultValue:
                            "Open the authorization window, finish sign-in, and wait for the callback.",
                        })}
                      </p>
                    </div>
                  </CardHeader>
                  <CardBody className="space-y-5 px-5 pb-5 pt-1">
                    <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(16rem,0.9fr)]">
                      <div className="rounded-large border-small border-default-200 bg-content2 p-4">
                        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-default-500">
                          {t("oauthImport.authorize.callbackLabel", {
                            defaultValue: "Callback URL",
                          })}
                        </p>
                        <p className="mt-2 break-all text-sm leading-6 text-default-600">
                          {session?.callback_url ?? "-"}
                        </p>
                      </div>
                      <div className="rounded-large border-small border-default-200 bg-content2 p-4">
                        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-default-500">
                          {t("oauthImport.authorize.helperTitle", {
                            defaultValue: "What to watch",
                          })}
                        </p>
                        <p className="mt-2 text-sm leading-6 text-default-600">
                          {t("oauthImport.authorize.helperDescription", {
                            defaultValue:
                              "If the popup is blocked or the callback never reaches the backend, keep the manual fallback ready in the next panel.",
                          })}
                        </p>
                      </div>
                    </div>

                    <div className="flex flex-wrap items-center gap-3">
                      <Button
                        color="primary"
                        isDisabled={!session?.authorize_url}
                        startContent={<ExternalLink className="h-4 w-4" />}
                        onPress={() => {
                          if (session?.authorize_url) {
                            openAuthorizeTab(session.authorize_url);
                          }
                        }}
                      >
                        {t("oauthImport.actions.reopenAuth")}
                      </Button>
                      <Button
                        startContent={<RefreshCcw className="h-4 w-4" />}
                        onPress={() =>
                          sessionId &&
                          queryClient.invalidateQueries({
                            queryKey: ["codexOauthLoginSession", sessionId],
                          })
                        }
                      >
                        {t("common.refresh", { defaultValue: "Refresh" })}
                      </Button>
                    </div>
                  </CardBody>
                </Card>
              ) : null}

              {currentStep === "monitor" ? (
                <Card>
                  <CardHeader className="px-5 pb-3 pt-5">
                    <div className="space-y-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <Chip color="warning" variant="flat">
                          {t("oauthImport.monitor.title", {
                            defaultValue: "Watch token exchange and import",
                          })}
                        </Chip>
                        <Chip
                          color={statusChipColor(session?.status)}
                          variant="flat"
                        >
                          {statusLabel}
                        </Chip>
                      </div>
                      <p className="max-w-[56rem] text-sm leading-6 text-default-600">
                        {t("oauthImport.monitor.description", {
                          defaultValue:
                            "The backend is exchanging credentials and importing the account into the pool.",
                        })}
                      </p>
                    </div>
                  </CardHeader>
                  <CardBody className="space-y-5 px-5 pb-5 pt-1">
                    <div className="rounded-large border-small border-default-200 bg-content2 p-4">
                      <div className="flex items-center gap-2 text-sm font-medium text-foreground">
                        <Loader2 className="h-4 w-4 animate-spin text-warning" />
                        {t("oauthImport.monitor.activeLabel", {
                          defaultValue: "Import is still running",
                        })}
                      </div>
                      <p className="mt-2 text-sm leading-6 text-default-600">
                        {t("oauthImport.monitor.activeHint", {
                          defaultValue:
                            "Stay on this page or jump back to accounts later. The status card on the left refreshes automatically every few seconds.",
                        })}
                      </p>
                      <Progress
                        aria-label={t("oauthImport.monitor.activeLabel", {
                          defaultValue: "Import is still running",
                        })}
                        className="mt-4"
                        color="warning"
                        isIndeterminate
                        size="sm"
                      />
                    </div>
                  </CardBody>
                </Card>
              ) : null}

              {currentStep === "result" ? (
                <Card>
                  <CardHeader className="px-5 pb-3 pt-5">
                    <div className="space-y-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <Chip
                          color={
                            showResult
                              ? "success"
                              : showError
                                ? "danger"
                                : "default"
                          }
                          variant="flat"
                        >
                          {t("oauthImport.result.reviewTitle", {
                            defaultValue: "Review import result",
                          })}
                        </Chip>
                        <Chip
                          color={statusChipColor(session?.status)}
                          variant="flat"
                        >
                          {statusLabel}
                        </Chip>
                      </div>
                      <p className="max-w-[56rem] text-sm leading-6 text-default-600">
                        {t("oauthImport.result.reviewDescription", {
                          defaultValue:
                            "Confirm whether the account was created, updated, or requires a retry.",
                        })}
                      </p>
                    </div>
                  </CardHeader>
                  <CardBody className="space-y-5 px-5 pb-5 pt-1">
                    {showResult ? (
                      <div className="rounded-large border-small border-success-200 bg-success-50 p-4 text-success-700 dark:bg-success/10 dark:text-success-300">
                        <div className="flex items-start gap-3">
                          <CheckCircle2 className="mt-0.5 h-5 w-5 shrink-0" />
                          <div className="min-w-0 space-y-2">
                            <p className="font-medium">
                              {t("oauthImport.result.success")}
                            </p>
                            <p className="break-all text-sm leading-6">
                              {t("oauthImport.result.accountId", {
                                id: session?.result?.account.id ?? "-",
                              })}
                            </p>
                            <p className="break-all text-sm leading-6">
                              {t("oauthImport.result.accountLabel", {
                                label: session?.result?.account.label ?? "-",
                              })}
                            </p>
                            {session?.result?.email ? (
                              <p className="text-sm leading-6">
                                {t("oauthImport.result.email", {
                                  email: session.result.email,
                                })}
                              </p>
                            ) : null}
                            {session?.result?.chatgpt_account_id ? (
                              <p className="text-sm leading-6">
                                {t("oauthImport.result.chatgptAccountId", {
                                  defaultValue: "ChatGPT account ID: {{id}}",
                                  id: session.result.chatgpt_account_id,
                                })}
                              </p>
                            ) : null}
                            {session?.result?.chatgpt_plan_type ? (
                              <p className="text-sm leading-6">
                                {t("oauthImport.result.chatgptPlanType", {
                                  defaultValue: "Plan type: {{plan}}",
                                  plan: getPlanLabel(
                                    session.result.chatgpt_plan_type,
                                    t,
                                  ),
                                })}
                              </p>
                            ) : null}
                            <div className="flex flex-wrap gap-2 pt-1">
                              <Chip
                                color={
                                  session?.result?.created
                                    ? "success"
                                    : "primary"
                                }
                                variant="flat"
                              >
                                {session?.result?.created
                                  ? t("oauthImport.result.created")
                                  : t("oauthImport.result.updated")}
                              </Chip>
                              <Button
                                color="success"
                                variant="flat"
                                onPress={() => navigate("/accounts")}
                              >
                                {t("oauthImport.actions.goAccounts")}
                              </Button>
                            </div>
                          </div>
                        </div>
                      </div>
                    ) : null}

                    {showError ? (
                      <div className="rounded-large border-small border-danger-200 bg-danger-50 p-4 text-danger-700 dark:bg-danger/10 dark:text-danger-300">
                        <div className="flex items-start gap-3">
                          <AlertCircle className="mt-0.5 h-5 w-5 shrink-0" />
                          <div className="min-w-0 space-y-2">
                            <p className="font-medium">
                              {t("oauthImport.error.failed")}
                            </p>
                            <p className="text-sm leading-6">
                              {localizedError.label}
                            </p>
                          </div>
                        </div>
                      </div>
                    ) : null}

                    {!showResult && !showError ? (
                      <div className="rounded-large border-small border-default-200 bg-content2 p-4 text-sm leading-6 text-default-600">
                        {t("oauthImport.result.pending", {
                          defaultValue:
                            "The session reached the final step, but the backend has not published a result payload yet.",
                        })}
                      </div>
                    ) : null}

                    <div className="flex flex-wrap items-center gap-3">
                      <Button
                        color="primary"
                        isLoading={createSessionMutation.isPending}
                        startContent={
                          createSessionMutation.isPending ? null : (
                            <ExternalLink className="h-4 w-4" />
                          )
                        }
                        onPress={() => createSessionMutation.mutate()}
                      >
                        {t("oauthImport.actions.startLogin")}
                      </Button>
                      <Button
                        startContent={<RefreshCcw className="h-4 w-4" />}
                        onPress={() =>
                          sessionId &&
                          queryClient.invalidateQueries({
                            queryKey: ["codexOauthLoginSession", sessionId],
                          })
                        }
                      >
                        {t("common.refresh", { defaultValue: "Refresh" })}
                      </Button>
                    </div>
                  </CardBody>
                </Card>
              ) : null}
            </motion.div>

            {showManualFallback ? (
              <motion.div variants={item}>
                <Card>
                  <CardBody className="space-y-5 p-5">
                    <div className="space-y-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <Chip color="secondary" variant="flat">
                          {t("oauthImport.manual.title")}
                        </Chip>
                        <Chip
                          color={statusChipColor(session?.status)}
                          variant="flat"
                        >
                          {statusLabel}
                        </Chip>
                      </div>
                      <p className="max-w-[56rem] text-sm leading-6 text-default-600">
                        {t("oauthImport.manual.description")}
                      </p>
                    </div>

                    <Textarea
                      minRows={5}
                      placeholder={t("oauthImport.manual.placeholder")}
                      value={manualRedirectUrl}
                      onValueChange={setManualRedirectUrl}
                      classNames={{ inputWrapper: "bg-content2/70" }}
                    />

                    <div className="flex flex-wrap items-center gap-3">
                      <Button
                        color="primary"
                        isDisabled={!sessionId || !manualRedirectUrl.trim()}
                        isLoading={submitManualCallbackMutation.isPending}
                        onPress={() => submitManualCallbackMutation.mutate()}
                      >
                        {t("oauthImport.actions.submitCallback")}
                      </Button>
                      <p className="text-sm leading-6 text-default-600">
                        {t("oauthImport.manual.hint")}
                      </p>
                    </div>
                  </CardBody>
                </Card>
              </motion.div>
            ) : null}
          </div>
        </div>

        {isBusy ? <div className="sr-only">{t("common.loading")}</div> : null}
      </motion.div>
    </PageContent>
  );
}

function createSessionMutationIsPending(
  sessionId: string | null,
  session?: CodexOAuthLoginSession,
): boolean {
  if (!sessionId) {
    return false;
  }
  return !isTerminalStatus(session?.status);
}
