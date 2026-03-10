export default {
    accounts: {
        actions: {
            add: "Add Account",
            apiKeyNoGroupAction: "API-key account: no linked-account action",
            batchDelete: "Batch Delete",
            batchDeleteConfirm: "Delete {{count}} selected accounts?",
            batchDisable: "Batch Disable",
            batchEnable: "Batch Enable",
            batchMenu: "Batch Actions",
            batchPauseFamily: "Batch Pause Family ({{count}})",
            batchRefreshLogin: "Batch Refresh Login ({{count}})",
            batchResumeFamily: "Batch Resume Family ({{count}})",
            comfortableMode: "Comfortable Mode",
            compactMode: "Compact Mode",
            delete: "Delete Account",
            deleteConfirm: "Delete account {{label}}?",
            disableAccount: "Disable Account",
            enableAccount: "Enable Account",
            export: "Export CSV",
            filter: "Filter List",
            oneTimeNoGroupAction: "One-time session account: no linked-account action",
            pauseGroup: "Pause Linked Accounts",
            refreshAccounts: "Refresh Accounts",
            refreshLogin: "Refresh Login",
            refreshingAccounts: "Refresh Accounts",
            resumeGroup: "Resume Linked Accounts",
            selectAll: "Select all filtered results",
            selectOne: "Select account {{label}}",
            selectedCount: "{{count}} selected",
            viewDetails: "View Details",
            edit: "Edit Properties",
            refresh: "Force Refresh",
            suspend: "Suspend",
            exportSuccess: "Export successful",
            refreshTriggered: "Account refresh triggered"
        },
        columns: {
            actions: "Actions",
            added: "Added",
            credentialType: "Credential Type",
            health: "Health Status",
            id: "Account ID",
            loginStatus: "Login Status",
            nextRefresh: "Next Refresh",
            plan: "Plan",
            provider: "Provider / Mode",
            rateLimit: "Rate Limit Usage",
            binding: "Binding",
            unbound: "unbound"
        },
        details: {
            description: "View account profile, OAuth status, limits, and raw payloads.",
            officialDescription: "Official OpenAI model metadata is read-only here. Manual override pricing can be edited below.",
            limitsTitle: "Rate Limits",
            noOauthStatus: "No OAuth status data yet.",
            oauthNotApplicable: "OAuth details are not available for this account type.",
            oauthTitle: "OAuth Status",
            profileTitle: "Account Profile",
            rawTitle: "Raw Payload",
            tabAria: "Account detail tabs",
            tabs: {
                limits: "Limits",
                oauth: "OAuth",
                profile: "Profile",
                raw: "Raw"
            },
            fields: {
                label: "Label",
                mode: "Mode",
                accountId: "Account ID",
                enabled: "Enabled",
                baseUrl: "Base URL",
                chatgptAccountId: "ChatGPT Account ID",
                priority: "Priority",
                createdAt: "Created At",
                bearerToken: "Bearer Token",
                authProvider: "Auth Provider",
                credentialKind: "Credential Kind",
                lastRefreshStatus: "Last Refresh Status",
                effectiveEnabled: "Effective Enabled",
                chatgptPlanType: "ChatGPT Plan Type",
                sourceType: "Source Type",
                tokenFamilyId: "Token Family ID",
                tokenVersion: "Token Version",
                tokenExpiresAt: "Token Expires At",
                nextRefreshAt: "Next Refresh At",
                lastRefreshAt: "Last Refresh At",
                refreshReusedDetected: "Refresh Reused Detected",
                lastRefreshErrorCode: "Last Refresh Error Code",
                lastRefreshError: "Last Refresh Error",
                rateLimitsFetchedAt: "Rate Limits Fetched At",
                rateLimitsExpiresAt: "Rate Limits Expires At",
                rateLimitsLastErrorCode: "Rate Limits Last Error Code",
                rateLimitsLastError: "Rate Limits Last Error",
                rawAccount: "Account Payload",
                rawOauthStatus: "OAuth Status Payload"
            }
        },
        filters: {
            active: "Active",
            all: "All",
            credential: "Credential Type",
            credentialAll: "All Credentials",
            credentialAt: "AT",
            credentialRt: "RT",
            credentialUnknown: "Unknown",
            disabled: "Disabled",
            mode: "Mode",
            modeAll: "All Modes",
            modeApiKey: "API Key",
            modeOAuth: "OAuth Session",
            plan: "Plan Filter",
            planAll: "All Plans",
            planUnknown: "Not Reported",
            total: "Matched {{count}}",
            suspended: "Suspended"
        },
        messages: {
            batchAllFailed: "{{action}} failed",
            batchAllSuccess: "{{action}} completed",
            batchPartialFailed: "{{failed}} operations failed{{error}}",
            batchPartialFailedTitle: "{{action}} partially failed",
            batchSuccessCount: "{{count}} succeeded",
            batchUnknownError: "Batch operation failed",
            deleteFailed: "Failed to delete account",
            deleteSuccess: "Account deleted",
            disableFailed: "Failed to disable account",
            disableSuccess: "Account disabled",
            enableFailed: "Failed to enable account",
            enableSuccess: "Account enabled",
            exportSuccess: "Export successful",
            pauseFamilyFailed: "Failed to pause linked accounts",
            pauseFamilySuccess: "Linked accounts paused",
            rateLimitPollingTimeout: "Rate-limit refresh job polling timed out.",
            rateLimitRefreshFailedStatus: "Rate-limit refresh job failed, status={{status}}",
            rateLimitRefreshFailedSummary: "Rate-limit refresh job failed: {{summary}}",
            refreshFailed: "Login refresh failed",
            refreshJobId: "Job ID: {{jobId}}",
            refreshJobSummary: "Job ID: {{jobId}} · {{processed}}/{{total}}",
            refreshListFailed: "Refresh List Failed",
            refreshListSuccess: "Refresh List Success",
            refreshSuccess: "Login refreshed successfully",
            requestFailed: "Request failed. Please try again later.",
            resumeFamilyFailed: "Failed to resume linked accounts",
            resumeFamilySuccess: "Linked accounts resumed",
            toggleUnsupported: "Current backend does not support account enable/disable. Please upgrade control-plane.",
            refreshTriggered: "Account refresh triggered"
        },
        rateLimitRefreshJobStatus: {
            queued: "Queued",
            running: "Running",
            completed: "Completed",
            failed: "Failed",
            cancelled: "Cancelled",
            unknown: "Unknown"
        },
        mode: {
            apiKey: "API Key",
            chatgptSession: "ChatGPT OAuth",
            codexOauth: "Codex OAuth",
            unknown: "Other"
        },
        nextRefresh: {
            none: "Not scheduled"
        },
        oauth: {
            kindShort: {
                oneTime: "AT",
                refreshRotatable: "RT",
                unknown: "Unknown"
            },
            loading: "Loading…",
            notApplicable: "-",
            status: {
                failed: "Failed",
                never: "Not refreshed",
                ok: "Healthy"
            },
            unknownError: "unknown",
            versionPrefix: "v",
            planPrefix: "Plan: ",
            kind: {
                refreshRotatable: "Refresh-token account",
                oneTime: "One-time access-token account",
                unknown: "Unknown credential type"
            }
        },
        rateLimits: {
            labels: {
                fiveHours: "5-Hour Limit",
                github: "GitHub",
                oneWeek: "Weekly Limit"
            },
            shortLabels: {
                fiveHours: "5h",
                oneWeek: "7d"
            },
            moreDetails: "More details (+{{count}})",
            noReset: "Reset time unavailable",
            remainingPrefix: "Remaining",
            resetAt: "{{absolute}} ({{relative}}) reset",
            unavailable: "No rate limit data",
            usedPrefix: "Used"
        },
        searchPlaceholder: "Search by label, account ID, URL…",
        status: {
            active: "Active",
            disabled: "Disabled"
        },
        subtitle: "Manage API credentials and billing health.",
        syncing: "Syncing account status…",
        title: "Accounts Pool"
    },
    billing: {
        columns: {
            balanceAfter: "Balance After Change",
            billingDetail: "Billing Details",
            deductedCredits: "Deducted Credits",
            deductionEvents: "Deduction Events",
            delta: "Delta Credits",
            eventType: "Event",
            model: "Model",
            periodDay: "Date",
            periodMonth: "Month",
            requestType: "Request Type",
            source: "Source",
            timestamp: "Time"
        },
        exportCsv: "Export CSV",
        filters: {
            granularityAriaLabel: "Billing granularity",
            tenantAriaLabel: "Tenant filter",
            tenantPlaceholder: "Select tenant"
        },
        granularity: {
            day: "Daily",
            month: "Monthly"
        },
        ledger: {
            codeLabels: {
                accountDeactivated: "Account deactivated",
                billingUsageMissing: "Missing usage settlement fields",
                failoverExhausted: "Retry/failover exhausted",
                noUpstreamAccount: "No upstream account available",
                streamPreludeError: "Stream prelude error",
                tokenInvalidated: "Token invalidated",
                transportError: "Upstream network error",
                upstreamRequestFailed: "Upstream request failed",
                unknown: "Unknown"
            },
            details: {
                accrued: "Accrued: {{value}} credits",
                adjustment: "Adjustment: {{value}}",
                extraCharge: "Extra charge: {{value}} credits",
                failoverAction: "Action: {{action}}",
                failure: "Failure: {{summary}}",
                failurePrefix: "Failure:",
                source: "Source: {{source}}",
                upstreamStatus: "Upstream {{status}}",
                tokenSettlement: "Token settlement: input {{input}} + cached {{cached}} + output {{output}}",
                unitPrice: "Unit price: {{prices}} credits/1M tokens"
            },
            empty: "No ledger entries yet.",
            failoverActions: {
                crossAccountFailover: "Cross-account failover",
                retrySameAccount: "Retry same account",
                returnFailure: "Return failure",
                unknown: "Unknown"
            },
            releaseReasons: {
                billingSettleFailed: "Billing settlement failed",
                failoverExhausted: "Retry/failover exhausted",
                invalidUpstreamUrl: "Invalid upstream URL configuration",
                noUpstreamAccount: "No upstream account available",
                streamPreludeError: "Stream prelude error",
                streamUsageMissing: "Stream usage missing",
                transportError: "Upstream network error",
                upstreamRequestFailed: "Upstream request failed",
                unknown: "Unknown"
            },
            showRaw: "Show raw entries",
            subtitle: "Filtered by current tenant.",
            title: "Ledger Entries",
            requestTypes: {
                nonStream: "Non-stream",
                stream: "Stream",
                unknown: "-"
            },
            tokenSegments: {
                cached: "Cached",
                input: "Input",
                output: "Output"
            }
        },
        messages: {
            rechargeFailedTitle: "Recharge failed",
            rechargeSuccessDetail: "+{{amount}}, balance {{balance}}",
            rechargeSuccessTitle: "Recharge successful",
            retryLater: "Please try again later"
        },
        recharge: {
            amountAriaLabel: "Recharge amount",
            amountPlaceholder: "Recharge credits (microcredits)",
            reasonAriaLabel: "Recharge reason",
            reasonPlaceholder: "Recharge reason",
            submit: "Execute Recharge",
            subtitle: "Recharge the currently selected tenant.",
            title: "Admin Recharge"
        },
        snapshot: {
            empty: "No settlement snapshots yet.",
            subtitle: "Aggregate deduction events by {{granularity}} for month-end settlement and reconciliation.",
            title: "Settlement Snapshot"
        },
        subtitle: "Primary view: credit ledger (actual charges), with tenant-level admin filtering.",
        summary: {
            currentBalance: "Current Balance",
            deductionHint: "Only negative ledger deduction events are counted.",
            monthConsumed: "This Month Consumption",
            todayConsumed: "Today's Consumption",
            unitCredits: "Unit: credits"
        },
        title: "Billing Center",
        trend: {
            noData: "No trend data yet.",
            seriesConsumed: "Consumed Credits",
            subtitle: "Show ledger deductions aggregated by {{granularity}}.",
            title: "Consumption Trend"
        }
    },
    common: {
        cancel: "Cancel",
        close: "Close",
        collapseSidebar: "Collapse sidebar",
        confirm: "Confirm",
        delete: "Delete",
        edit: "Edit",
        expand: "Expand",
        collapse: "Collapse",
        expandSidebar: "Expand sidebar",
        loading: "Loading…",
        logout: "Sign Out",
        no: "No",
        noData: "No data yet.",
        openMenu: "Open menu",
        refresh: "Refresh",
        skipToMainContent: "Skip to main content",
        table: {
            firstPage: "First page",
            go: "Go",
            jumpToPage: "Jump to page",
            lastPage: "Last page",
            nextPage: "Next page",
            pageOf: "Page {{page}} / {{total}}",
            previousPage: "Previous page",
            range: "{{start}}-{{end}} / {{total}}",
            rowsPerPage: "Rows per page",
            searchLabel: "Search table",
            searchPlaceholder: "Search in current list…"
        },
        toggleLanguage: "Toggle language",
        toggleTheme: "Toggle theme",
        yes: "Yes",
        save: "Save",
        search: "Search…",
        routeLoading: "Loading page…",
        status: {
            healthy: "Healthy",
            degraded: "Degraded",
            offline: "Offline",
            disabled: "Disabled",
            available: "Available"
        }
    },
    config: {
        controlPlane: {
            authValidateUrl: "Auth Validation URL",
            dataPlaneUrl: "Forwarding Service URL",
            desc: "Configure how control plane talks to forwarding services",
            listen: "Control Plane Listen Address (read-only)",
            title: "Control Plane Connection"
        },
        refreshSettings: {
            desc: "Control the switch and interval for automatic login credential refresh",
            enableDesc: "If disabled, expiring access credentials will no longer refresh automatically.",
            enableLabel: "Enable auto refresh",
            intervalSec: "Refresh interval (seconds)",
            notes: "Notes",
            title: "Auto Refresh Settings"
        },
        runtimeHint: {
            desc: "Changes take effect immediately. After restart, environment variables and config.toml still take priority.",
            title: "Runtime Config Notice"
        },
        save: "Save Changes",
        subtitle: "Runtime settings and global variables",
        success: "Configuration saved successfully.",
        title: "Configuration",
        reload: {
            title: "Runtime Reload Enabled",
            desc: "Changes to caching and timeouts will be synced globally and take effect immediately without requiring service restarts."
        },
        system: {
            title: "System Strategy",
            desc: "Configure global operations context.",
            maintenance: "Maintenance Mode",
            maintenanceDesc: "Reject all new requests globally holding 503.",
            logRetention: "Log Retention (Days)",
            logRetentionDesc: "Rolling window for clickhouse trace storage."
        },
        network: {
            title: "Network Profiling",
            desc: "Define global ratelimits to prevent upstream banning.",
            tpm: "Global Tokens Per Min",
            tpmDesc: "Maximum simultaneous requests to AI endpoints.",
            scaling: "Cloud Resource Scaling",
            scalingDesc: "Allow dynamic node allocation bounds."
        }
    },
    dashboard: {
        actions: {
            viewBilling: "View billing",
            viewLogs: "View request logs"
        },
        alerts: {
            checkRoutes: "Check Routes",
            columns: {
                action: "Suggested Action",
                message: "Message",
                severity: "Severity",
                source: "Source",
                status: "Status",
                time: "Time"
            },
            empty: "All systems looking good.",
            resolve: "Mark Resolved",
            searchPlaceholder: "Search alerts…",
            subtitle: "System states needing review",
            title: "Requires Attention",
            usageRepoUnavailable: "Usage analytics storage is unavailable",
            dataPlaneDisconnected: "Data plane disconnected",
            now: "Now",
            severity: {
                critical: "Critical",
                warning: "Warning",
                info: "Info"
            },
            source: {
                data_plane: "Data Plane",
                usage_repo: "Usage Repo"
            },
            status: {
                open: "Open",
                resolved: "Resolved"
            }
        },
        currentScope: "Current: {{scope}}",
        filters: {
            apiKeyAriaLabel: "API key filter",
            apiKeyPlaceholder: "Select API key",
            range: {
                last24Hours: "Last 24 hours",
                last30Days: "Last 30 days",
                last7Days: "Last 7 days"
            },
            rangeAriaLabel: "Time range",
            scopeAriaLabel: "Scope filter",
            tenantAriaLabel: "Tenant filter",
            tenantPlaceholder: "Select tenant"
        },
        kpi: {
            activeApiKeysInRange: "Active API keys (selected range)",
            accounts: "Accounts",
            accountsDesc: "Admin-only operational metric",
            apiKeys: "API keys",
            apiKeysDesc: "Configured keys in system",
            avgFirstTokenSpeed: "Average first-token speed",
            avgFirstTokenSpeedDesc: "TTFT (streaming exact / non-stream approximate)",
            globalScope: "Global scope",
            rpm: "RPM",
            rpmDesc: "Requests per minute",
            requests: {
                apiKey: "Current API key requests (selected range)",
                global: "Total account requests (selected range)",
                tenant: "Current tenant API key requests (selected range)"
            },
            tenants: "Tenants",
            tenantsDesc: "Admin-only operational metric",
            totalRequests: "Total requests",
            totalTokens: "Token consumption total",
            totalTokensDesc: "Input + cached + output + reasoning",
            tpm: "TPM",
            tpmDesc: "Tokens per minute",
            running: "Running",
            totalConfigured: "Total Configured",
            uptime: "Uptime 99.99%",
            reqs: "Total Requests",
            reqsDesc: "+12.5% from last month",
            failed: "Failed Requests",
            failedDesc: "+180 today",
            avgLatency: "Avg Latency",
            avgLatencyDesc: "-5ms from last month",
            activeTokens: "Active Tokens",
            activeTokensDesc: "+24 new tokens mapped",
            oauthLabel: "OAuth"
        },
        modelDistribution: {
            description: "Top models by request count or token usage.",
            empty: "No model distribution data yet",
            modeRequests: "By requests",
            modeTokens: "By tokens",
            other: "Other",
            title: "Model request distribution"
        },
        scope: {
            apiKey: "API Key View",
            global: "Global View",
            tenant: "Tenant View"
        },
        subtitle: "Global gateway proxy metrics.",
        table: {
            apiKey: "API Key",
            requests: "Requests"
        },
        tokenComponents: {
            cached: "Cached",
            input: "Input",
            output: "Output",
            reasoning: "Reasoning"
        },
        tokenTrend: {
            description: "Hourly token trend by component. Toggle components to focus specific consumption.",
            empty: "No token trend data yet",
            title: "Token usage trend"
        },
        title: "Overview",
        topApiKeys: {
            empty: "No ranking data yet",
            scopeDescription: "Scope: {{scope}} / selected time window",
            title: "Top API Keys"
        },
        trafficChart: {
            scope: {
                apiKey: "Scope: current API key requests",
                global: "Scope: global account requests + global tenant API key requests",
                tenant: "Scope: current tenant API key requests"
            },
            series: {
                accountRequests: "Account requests",
                tenantApiKeyRequests: "Tenant API key requests",
                tenantApiKeyRequestsSingle: "Tenant API key requests"
            },
            title: "Traffic Overview",
            subtitle: "Hourly proxy volume past 24h",
            success: "Successful",
            blocked: "Blocked"
        }
    },
    importJobs: {
        actions: {
            cancel: "Cancel",
            cancelJob: "Cancel job",
            confirmCancelJob: "Cancel this import job?",
            exportFailed: "Export failed items",
            refreshItems: "Refresh items",
            remove: "Remove",
            retryFailed: "Retry failed",
            removeFromList: "Remove from list"
        },
        detail: {
            columns: {
                error: "Error",
                label: "Label",
                line: "Line",
                status: "Status"
            },
            filterLabel: "Status filter",
            itemsEmpty: "No matching job items.",
            itemsLoading: "Loading job items…",
            jobIdLabel: "Job ID: {{jobId}}",
            loadMore: "Load more",
            loadedCount: "Loaded {{count}} records",
            loadingMore: "Loading…",
            retryQuery: "Retry query",
            searchPlaceholderModern: "Search by label / email / error…",
            selectHint: "Select a job to view details.",
            summaryLoading: "Loading job summary…",
            title: "Job Details",
            unreadable: "This job cannot be read (possibly expired or invalid ID).",
            searchPlaceholder: "Search by label / email / error"
        },
        dropzone: {
            acceptsNew: "Upload multiple .json/.jsonl files in one batch. The backend will merge them into a single job.",
            creatingTitle: "Creating import job…",
            selectFiles: "Select Files",
            titleNew: "Drop import files here",
            uploading: "Uploading…",
            wait: "Please wait while the file is transmitted securely.",
            title: "Click to Upload or Drag File Here",
            accepts: "Accepts <code>.csv</code> or line-separated <code>.txt</code> format. Max 50,000 rows per file.",
            browse: "Browse Files",
            creatingDesc: "Please wait. The new job will appear in the queue automatically."
        },
        error: "Upload Failed",
        manual: {
            add: "Add",
            placeholderModern: "Paste job_id…",
            title: "Manual job tracking",
            placeholder: "Paste job_id"
        },
        messages: {
            jobNotFound: "Job not found or inaccessible",
            queryFailed: "Query Failed",
            unknownError: "unknown error",
            uploadFailedTitle: "Import failed"
        },
        errors: {
            importFailed: "Import failed",
            invalidRecord: "Invalid record",
            invalidRefreshToken: "Invalid refresh token",
            missingCredentials: "Missing credentials",
            oauthProviderNotConfigured: "OAuth provider not configured",
            rateLimited: "Rate limited",
            refreshTokenReused: "Refresh token already used",
            unknown: "Unknown import error",
            upstreamNetworkError: "Upstream network error",
            upstreamUnavailable: "Upstream service unavailable"
        },
        metrics: {
            created: "Created",
            failed: "Failed",
            processed: "Processed",
            status: "Status",
            throughput: "Throughput",
            total: "Total",
            updated: "Updated"
        },
        precheck: {
            createdNotice: "Import job created: {{id}}",
            defaultReady: "File format and size checks passed.",
            duplicateBatch: "These files are already in the staging list.",
            duplicateName: "Duplicate filename detected. Review source before importing.",
            emptyPreview: "No valid data row found in preview. File may be empty.",
            firstLineInvalid: "JSONL first line is invalid JSON. Import may fail.",
            firstLineObject: "First line is not a JSON object. Import may fail.",
            firstLineValid: "JSONL first-line structure check passed.",
            jsonEmpty: "JSON file appears empty.",
            jsonInvalid: "JSON file is invalid. Import may fail.",
            jsonValid: "JSON structure check passed.",
            noneImportable: "No importable files. Resolve blocked files first.",
            skipLargeJson: "Large file detected. Local JSON parse skipped; backend will validate.",
            status: {
                invalid: "Blocked",
                ready: "Ready",
                warning: "Needs Review"
            }
        },
        progress: {
            done: "Completed",
            etaLabel: "Estimated Remaining: ",
            etaMinutes: "About {{count}} minutes",
            jobIdLabel: "Job ID: {{jobId}}",
            lessThanMinute: "Less than 1 minute",
            noJobSelected: "Create or select an import job to see live progress here.",
            title: "Live Import Progress",
            topErrors: "Top Error Breakdown"
        },
        queue: {
            columns: {
                jobId: "Job ID"
            },
            descRecent: "The queue auto-polls job status. Click a job to inspect items and errors.",
            emptyRecent: "No tracked jobs yet. Upload files or add a job_id manually.",
            titleRecent: "Recent import jobs",
            tracked: "tracked locally",
            title: "Recent Tracking Jobs",
            empty: "No jobs uploaded in this session yet.",
            card: {
                processed: "processed",
                new: "new",
                errors: "errors"
            }
        },
        status: {
            all: "All",
            cancelled: "Cancelled",
            completed: "Completed",
            created: "Created",
            failed: "Failed",
            pending: "Pending",
            processing: "Processing",
            queued: "Queued",
            running: "Running",
            skipped: "Skipped",
            updated: "Updated"
        },
        subtitleModern: "Add files in bulk, review checks, then import in one click with live progress and error details.",
        template: {
            downloadJsonl: "Download Template",
            title: "Import Template",
            desc: "Download our recommended template to ensure columns strictly map to system requirements.",
            download: "Download CSV Template",
            titleNew: "Import Template",
            descNew: "Download the JSONL template and fill in refresh_token values for bulk import."
        },
        title: "Batch Import Jobs",
        validation: {
            fileTooLarge: "File {{name}} exceeds 20MB. Please split and retry.",
            unsupportedFormat: "File {{name}} is unsupported. Only .json / .jsonl are allowed.",
            format: "Only .csv or .txt files are allowed.",
            size: "File is too large. Maximum size is 10MB."
        },
        workspace: {
            clearQueue: "Clear List",
            columns: {
                action: "Action",
                check: "Precheck",
                file: "File",
                size: "Size",
                status: "Status"
            },
            confirmClear: "Clear the current staged file list?",
            desc: "Drag & drop or select files in bulk, run checks, then click Start Import.",
            empty: "No staged files yet. Drag files into the area above.",
            invalidFiles: "Blocked {{count}}",
            invalidHint: "{{count}} files are blocked and will be excluded when importing.",
            moreChecks: "more checks",
            readyFiles: "Ready {{count}}",
            readyHint: "Precheck complete. Ready to start importing.",
            startImportWithCount: "Start Import ({{count}} files)",
            stepCheck: "Run Checks",
            stepCheckDesc: "Validate format, size, and duplicate names",
            stepImport: "Start Import",
            stepImportDesc: "Create a job and watch live progress",
            stepSelect: "Add Files",
            title: "File Import Workspace",
            totalFiles: "{{count}} files",
            totalSize: "Total {{size}}",
            warningFiles: "Needs Review {{count}}"
        },
        subtitle: "Upload account secrets securely in strictly formatted CSV/TXT files."
    },
    oauthImport: {
        title: "OAuth Login Import",
        subtitle: "Sign in with Codex OAuth and import the logged-in account directly into the pool.",
        start: {
            title: "Start Codex OAuth Login",
            description: "Create a login session, complete OAuth authorization, then auto-import the account."
        },
        form: {
            label: "Account Label (Optional)",
            labelPlaceholder: "Leave empty to auto-generate from email/account ID",
            baseUrl: "Base URL",
            priority: "Priority",
            enabled: "Enable account after import"
        },
        actions: {
            startLogin: "Start OAuth Login",
            reopenAuth: "Open OAuth Window Again",
            goAccounts: "Go to Accounts",
            submitCallback: "Submit Callback URL"
        },
        status: {
            label: "Session Status",
            idle: "Not started",
            waiting_callback: "Waiting for callback",
            exchanging: "Exchanging token",
            importing: "Importing account",
            completed: "Completed",
            failed: "Failed",
            expired: "Expired",
            sessionId: "Session ID: {{id}}",
            callbackUrl: "Callback URL: {{url}}",
            expiresAt: "Expires at: {{time}}"
        },
        error: {
            failed: "OAuth import failed."
        },
        result: {
            success: "Account imported successfully.",
            accountId: "Account ID: {{id}}",
            accountLabel: "Label: {{label}}",
            email: "Email: {{email}}",
            created: "Created",
            updated: "Updated"
        },
        manual: {
            title: "Manual Callback Fallback",
            description: "If automatic callback is unreachable, paste the full redirect URL here.",
            placeholder: "Paste full callback URL containing code/state…",
            hint: "Use this only when automatic callback fails."
        },
        notifications: {
            popupBlockedTitle: "Popup blocked",
            popupBlockedDescription: "Allow popups and reopen the OAuth window.",
            sessionCreatedTitle: "Session created",
            sessionCreatedDescription: "OAuth window opened. Complete sign-in to continue.",
            sessionCreateFailedTitle: "Failed to create session",
            manualSubmitTitle: "Callback submitted",
            manualSubmitSuccess: "Manual callback completed and account imported.",
            manualSubmitAccepted: "Manual callback accepted. Session is processing.",
            manualSubmitFailedTitle: "Manual callback failed",
            unknownError: "Unexpected error. Please try again."
        }
    },
    login: {
        brand: {
            badge: "Control Plane Access",
            points: {
                audit: "Every high-risk operation is fully traceable by request ID.",
                resilience: "High-availability routing keeps management workflows stable.",
                security: "Tenant isolation and credential governance are enforced by default."
            },
            subtitle: "A hardened entry point for system operators.",
            title: "Operate Codex Pool with confidence"
        },
        messages: {
            failed: "Sign in failed. Please check username and password.",
            invalidCredentials: "Incorrect username or password. Please try again.",
            sessionExpired: "Your session expired. Please sign in again."
        },
        password: "Password",
        passwordPlaceholder: "Enter admin password",
        securityHint: "Security note: repeated failures will be correlated in audit logs.",
        submit: "Sign In",
        subtitle: "Sign in with your admin account",
        title: "Codex-Pool Console",
        username: "Username",
        usernamePlaceholder: "Enter admin username"
    },
    logs: {
        audit: {
            actionValues: {
                adminOperation: "Admin operation",
                authOperation: "Auth operation",
                requestOperation: "Request operation",
                tenantOperation: "Tenant operation",
                unknown: "Unknown action"
            },
            actorTypes: {
                adminUser: "Admin user",
                apiKey: "API key",
                system: "System",
                tenantUser: "Tenant user",
                unknown: "Unknown actor"
            },
            columns: {
                action: "Action",
                actor: "Actor",
                createdAt: "Time",
                reason: "Reason",
                result: "Result",
                target: "Target",
                tenant: "Tenant"
            },
            description: "Scope: Control Plane audit events (role / action / result / target / payload).",
            empty: "No audit log data available",
            filters: {
                actionPlaceholder: "Action",
                actorIdPlaceholder: "Actor ID",
                actorTypePlaceholder: "Actor type",
                keywordPlaceholder: "Keyword (reason / payload)",
                resultStatusPlaceholder: "Result status",
                actionAriaLabel: "Action filter",
                actorIdAriaLabel: "Actor ID filter",
                actorTypeAriaLabel: "Actor type filter",
                keywordAriaLabel: "Keyword filter",
                rangeAriaLabel: "Time range",
                resultStatusAriaLabel: "Result status filter",
                tenantAriaLabel: "Tenant filter"
            },
            resultStatuses: {
                denied: "Denied",
                failed: "Failed",
                ok: "Success",
                unknown: "Unknown result"
            },
            title: "Audit Logs"
        },
        columns: {
            level: "Level",
            message: "Message",
            service: "Service Node",
            timestamp: "Timestamp"
        },
        export: "Export logs",
        filters: {
            allTenants: "All tenants"
        },
        focus: "Focus:",
        levels: {
            all: "All Levels",
            error: "Error",
            info: "Info",
            warn: "Warning"
        },
        messages: {
            configUpdated: "Updated runtime config snapshot in memory",
            empty: "No message details",
            keyCreated: "Created API key {{keyId}}",
            keyPatched: "Set API key {{keyId}} enabled={{enabled}}",
            modelsLoaded: "Loaded models from upstream account {{label}}",
            modelsProbed: "Model probe ({{trigger}}): tested {{tested}} models via {{label}} (available {{available}}, unavailable {{unavailable}})",
            proxiesTested: "Tested {{count}} proxy nodes",
            systemState: "Queried system state: {{count}} accounts",
            unmappedAction: "{{action}} · {{message}}"
        },
        range: {
            last24Hours: "Last 24 hours",
            last30Days: "Last 30 days",
            last7Days: "Last 7 days"
        },
        request: {
            columns: {
                apiKey: "API Key",
                createdAt: "Time",
                errorCode: "Error",
                latency: "Latency (ms)",
                path: "Path",
                requestId: "Request ID",
                status: "Status",
                tenant: "Tenant"
            },
            description: "Scope: Raw Data Plane request events (status / latency / path / tenant / API key / request ID).",
            empty: "No request log data available",
            filters: {
                apiKeyIdPlaceholder: "API Key ID",
                keywordPlaceholder: "Keyword (path / error / model)",
                requestIdPlaceholder: "Request ID",
                statusCodePlaceholder: "Status code (e.g. 500)",
                apiKeyAriaLabel: "API key filter",
                keywordAriaLabel: "Keyword filter",
                rangeAriaLabel: "Time range",
                requestIdAriaLabel: "Request ID filter",
                statusCodeAriaLabel: "Status code filter",
                tenantAriaLabel: "Tenant filter"
            },
            title: "Request Logs"
        },
        search: "Search payload or service…",
        subtitle: "Real-time audit trails and operational context.",
        tabs: {
            audit: "Audit Logs",
            request: "Request Logs",
            system: "System Logs"
        },
        title: "System Logs",
        waiting: "Waiting for incoming streams…",
        actions: {
            systemState: "System State",
            configUpdate: "Config Update",
            proxiesTest: "Proxy Health Check",
            keyCreate: "API Key Created",
            keyPatch: "API Key Updated",
            modelsList: "Model List Fetch",
            modelsProbe: "Model Probe",
            unknown: "Unknown action ({{action}})"
        }
    },
    models: {
        actions: {
            copyModelId: "Copy model name",
            createModel: "Create model",
            syncOpenAiCatalog: "Sync OpenAI catalog",
            probeAvailability: "Probe availability",
            openDetails: "Details",
            deleteModel: "Delete model",
            deletePricing: "Delete pricing",
            deleteBillingRule: "Delete rule",
            editBillingRule: "Edit rule",
            editModel: "Edit model",
            probeNow: "Probe Now",
            saveModelProfile: "Save model profile",
            savePricing: "Save pricing",
            saveBillingRule: "Save rule",
            newBillingRule: "New rule",
            search: "Search model IDs…",
            sync: "Sync Status"
        },
        availability: {
            available: "Available",
            issueHint: "View unavailable reason",
            neverChecked: "Never checked",
            noErrorDetail: "No error detail",
            unavailable: "Unavailable",
            unknown: "Unknown",
            viewIssue: "View Issue"
        },
        cache: {
            fresh: "fresh",
            stale: "stale"
        },
        catalog: {
            customOnly: "Custom model",
            hidden: "Hidden",
            listed: "Listed",
            unlisted: "Unlisted"
        },
        columns: {
            actions: "Details",
            availability: "Availability",
            cachedInputPrice: "Cached Input Price",
            context: "Context / Max output",
            modalities: "Modalities",
            syncedAt: "Synced",
            catalog: "Catalog",
            checkedAt: "Last Checked",
            id: "Model Name",
            inputPrice: "Input Price",
            outputPrice: "Output Price",
            pricingStatus: "Pricing Status",
            provider: "Provider / Hub"
        },
        description: "View model availability and manage model profiles and pricing here.",
        dialog: {
            description: "Edit profile and pricing in this dialog. Saved pricing will be written back to the model pool list immediately.",
            officialDescription: "Official OpenAI model metadata is read-only here. Manual override pricing can be edited below.",
            tabListAriaLabel: "Model profile tabs",
            titleWithId: "Model profile · {{modelId}}"
        },
        empty: "No models mapped or exposed yet.",
        emptySyncRequired: "No official catalog yet. Sync OpenAI catalog first.",
        emptyActions: {
            goAccounts: "Go to Accounts",
            importAccount: "Import Accounts"
        },
        errors: {
            deleteModelEntityFailed: "Failed to delete model entity.",
            deleteModelPricingFailed: "Failed to delete model pricing.",
            openAiCatalogSyncFailed: "Failed to sync OpenAI catalog.",
            deleteBillingRuleFailed: "Failed to delete tiered pricing rule.",
            modelIdRequired: "Model ID cannot be empty.",
            probeFailed: "Model probing failed.",
            saveBillingRuleFailed: "Failed to save tiered pricing rule.",
            saveModelPricingFailed: "Failed to save model pricing.",
            saveModelProfileFailed: "Failed to save model profile."
        },
        filters: {
            allProviders: "All providers",
            providerLabel: "Provider filter"
        },
        form: {
            modelId: "Model ID",
            modelIdLockedHint: "Existing models cannot change the ID. Use \"Create model\" to add a new one.",
            modelIdPlaceholder: "Example: gpt-5.3-codex",
            provider: "Provider",
            providerPlaceholder: "Example: openai / custom",
            source: "Source",
            sourceValues: {
                entityOnly: "Entity only",
                pricingOnly: "Pricing only",
                upstream: "Upstream"
            },
            visibility: "Visibility",
            visibilityPlaceholder: "Example: list / hide"
        },
        hints: {
            cannotDeleteMissingPricing: "The current model has no local pricing record. Save pricing first before deleting it.",
            cannotDeleteNonLocalEntity: "The current model is not a local entity model, so its entity cannot be deleted."
        },
        loadingHint: "Checking directory and availability status. The latest model list will appear automatically once complete.",
        notice: {
            modelEntityDeleted: "Model entity deleted.",
            modelPricingDeleted: "Model pricing record deleted.",
            billingRuleDeleted: "Tiered pricing rule deleted.",
            modelPricingSaved: "Model pricing saved: {{model}}",
            openAiCatalogSynced: "OpenAI catalog synced: {{count}} models updated.",
            billingRuleSaved: "Tiered pricing rule saved: {{model}}",
            modelProfileSaved: "Model profile saved: {{model}}",
            probeCompleted: "Model probing completed. The latest model pool has been synced."
        },
        pricing: {
            cachedInputPrice: "Cached input price",
            creditsPerMillionTokens: "credits / 1M tokens",
            disabled: "Disabled",
            enablePricing: "Enable pricing",
            enabled: "Enabled",
            inputPrice: "Input price",
            notConfigured: "Not configured",
            outputPrice: "Output price",
            perMillionTokensMicrocredits: "Per 1M tokens, in microcredits",
            sectionTitle: "Model pricing",
            effectiveSectionTitle: "Effective pricing",
            manualOverride: "Manual override",
            officialBase: "Official base",
            overrideSectionTitle: "Manual price override",
            sourceLabels: {
                officialSync: "OpenAI official",
                manualOverride: "Manual override",
                unknown: "Unknown"
            },
        },
        rules: {
            cachedInputMultiplierPpm: "Cached input multiplier (ppm)",
            empty: "No tiered pricing rules are configured for this model yet.",
            enableRule: "Enable rule",
            inputMultiplierPpm: "Input multiplier (ppm)",
            noThreshold: "none",
            outputMultiplierPpm: "Output multiplier (ppm)",
            priorityLabel: "Priority",
            requestKind: "Request kind",
            requestKinds: {
                any: "Any",
                chat: "Chat",
                compact: "Compact",
                response: "Responses",
                unknown: "Unknown"
            },
            ruleSummary: "Threshold {{threshold}} · input ×{{inputMultiplier}} · cached ×{{cachedMultiplier}} · output ×{{outputMultiplier}}",
            scope: "Scope",
            scopes: {
                request: "Request",
                session: "Session",
                unknown: "Unknown"
            },
            sectionDescription: "Configure request/session-based multipliers for long-context or special billing bands.",
            sectionTitle: "Tiered pricing rules",
            thresholdInputTokens: "Threshold input tokens"
        },
        probeSourceUnknown: "unknown account",
        probeSummary: "Probe cache: {{stale}}, checked {{checkedAt}}, ttl {{ttlHours}}h, source {{source}}",
        syncHint: {
            notSynced: "OpenAI catalog has not been synced yet.",
            syncedAt: "Catalog synced {{time}}"
        },
        profile: {
            sectionTitle: "Model profile"
        },
        syncing: "Mapping endpoint capabilities…",
        tabs: {
            pricing: "Pricing",
            profile: "Profile"
        },
        title: "Models",
        subtitle: "Available endpoints mapped from the accounts pool.",
        detail: {
            title: "Model Details",
            notFound: "Model not found. It may have been removed or filtered out.",
            httpStatus: "HTTP Status",
            error: "Error",
            noError: "No error detail",
            officialTitle: "Official metadata",
            officialDescription: "Official OpenAI model metadata is read-only here. Manual override pricing can be edited below.",
            contextWindow: "Context window",
            maxOutputTokens: "Max output tokens",
            knowledgeCutoff: "Knowledge cutoff",
            reasoningTokenSupport: "Reasoning token support",
            sourceUrl: "Source URL",
            openOfficialPage: "Open official page",
            capabilitiesTitle: "Capabilities",
            inputModalities: "Input modalities",
            outputModalities: "Output modalities",
            endpoints: "Endpoints",
            rawText: "Official text snapshot",
        }
    },
    nav: {
        accounts: "Accounts Pool",
        apiKeyGroups: "Group Management",
        apiKeys: "API Keys",
        billing: "Billing",
        config: "Configuration",
        dashboard: "Dashboard",
        groups: {
            analytics: "Analytics",
            assets: "Pool Assets",
            operations: "Operations",
            system: "System"
        },
        importJobs: "Import Jobs",
        oauthImport: "OAuth Login Import",
        logs: "System Logs",
        mainNavigation: "Main navigation",
        models: "Models",
        online: "Online",
        openNavigation: "Open navigation",
        proxies: "Proxies",
        system: "System Status",
        tenants: "Tenant Pool",
        usage: "Usage",
        cleanup: "Cleanup",
        closeNavigation: "Close navigation"
    },
    notifications: {
        dismiss: "Dismiss notification",
        loginFailed: {
            title: "Sign in failed"
        },
        sessionExpired: {
            title: "Session expired",
            description: "Please sign in again to continue."
        }
    },
    errors: {
        common: {
            failed: "Failed",
            network: "Network error. Please check your connection.",
            timeout: "Request timed out. Please try again."
        },
        api: {
            unauthorized: "Unauthorized. Please sign in again.",
            invalidRequest: "Invalid request.",
            notFound: "Resource not found.",
            serviceUnavailable: "Service unavailable.",
            internalError: "Internal server error.",
            oauthProviderNotConfigured: "OAuth provider is not configured.",
            oauthCallbackListenerUnavailable: "OAuth callback listener is unavailable.",
            invalidRefreshToken: "Refresh token is invalid or expired.",
            refreshTokenReused: "Refresh token has been reused. Obtain the latest refresh token.",
            refreshTokenRevoked: "Refresh token has been revoked.",
            oauthMissingClientId: "OAuth provider is misconfigured (missing client_id).",
            oauthUnauthorizedClient: "OAuth client is unauthorized.",
            upstreamUnavailable: "Upstream service unavailable.",
            upstreamNetworkError: "Upstream network error.",
            oauthExchangeFailed: "OAuth exchange failed."
        },
        http: {
            badRequest: "Bad request",
            unauthorized: "Unauthorized",
            forbidden: "Forbidden",
            notFound: "Not found",
            conflict: "Conflict",
            payloadTooLarge: "Payload too large",
            rateLimited: "Rate limited",
            internalServerError: "Server error",
            badGateway: "Bad gateway",
            serviceUnavailable: "Service unavailable",
            gatewayTimeout: "Gateway timeout"
        }
    },
    proxies: {
        check: "Run Health Check",
        columns: {
            actions: "Actions",
            health: "Health",
            lastPing: "Last Ping",
            latency: "Avg Latency",
            url: "Proxy Node URL",
            weight: "Routing Wt"
        },
        empty: "No backend proxies configured.",
        filters: {
            all: "All nodes",
            degraded: "Degraded",
            disabled: "Disabled",
            healthy: "Healthy",
            label: "Health filter",
            offline: "Offline"
        },
        health: {
            degraded: "Degraded",
            disabled: "Disabled",
            healthy: "Healthy",
            offline: "Offline"
        },
        loading: "Scanning network topology…",
        manage: "Manage",
        pending: "Pending",
        retry: "Retry",
        searchPlaceholder: "Search node URL or label…",
        subtitle: "Manage reverse proxy nodes and traffic routing topology.",
        title: "Proxy Nodes"
    },
    system: {
        columns: {
            component: "Component",
            details: "Details",
            status: "Status",
            uptime: "Uptime",
            version: "Version"
        },
        components: {
            controlPlane: "Control Plane",
            dataPlane: "Data Plane Routes",
            usageRepo: "Usage Repository"
        },
        details: {
            analyticsUnavailable: "Analytics unavailable",
            apiActive: "API Gateway Active",
            checkingAPI: "Checking API…",
            dbConnected: "Time-series DB connected",
            endpointsResponding: "Proxy endpoints responding"
        },
        labels: {
            local: "Local",
            remote: "Remote",
            storage: "Storage",
            version: "Version:",
            uptime: "Uptime"
        },
        observability: {
            badges: {
                failoverOff: "Failover: OFF",
                failoverOn: "Failover: ON",
                failoverWait: "Failover wait {{value}} ms",
                quickRetry: "Quick retry ≤ {{value}}",
                retryPoll: "Retry poll {{value}} ms",
                sharedCacheOff: "Shared cache: OFF",
                sharedCacheOn: "Shared cache: ON",
                stickyConflictAvoidOff: "Sticky conflict avoid: OFF",
                stickyConflictAvoidOn: "Sticky conflict avoid: ON"
            },
            hints: {
                billingPreauthCaptureMissingTotal: "Billing preauth capture missing total",
                billingPreauthErrorRatioAvg: "Billing preauth error ratio avg",
                billingPreauthErrorRatioP95: "Billing preauth error ratio p95",
                billingPreauthTopModelP95: "Billing preauth top model p95",
                billingReconcileAdjust: "Automatic balance adjustments written by reconcile.",
                billingReconcileFailed: "Reconcile operations that failed and need inspection.",
                billingReconcileReleased: "Authorizations auto-closed by reconcile.",
                billingReconcileScanned: "Reconcile facts scanned from request_log and ledger.",
                billingReleaseWithoutCaptureRatio: "Billing release without capture ratio",
                billingSettleCompleteRatio: "Billing settle complete ratio",
                cacheHitRate: "Combined hit ratio of local + shared sticky cache lookups.",
                failoverAttempts: "Total number of cross-account failover attempts.",
                failoverExhausted: "Requests that still failed after retries/failover.",
                failoverSuccess: "Requests recovered successfully after switching accounts.",
                failoverSuccessRate: "Success ratio among all failover attempts.",
                sameAccountRetry: "Quick retries on the same account before switching.",
                stickyHitRate: "Sticky-session hit ratio across routed requests."
            },
            metrics: {
                billingPreauthCaptureMissingTotal: "Billing preauth capture missing total",
                billingPreauthErrorRatioAvg: "Billing preauth error ratio avg",
                billingPreauthErrorRatioP95: "Billing preauth error ratio p95",
                billingPreauthTopModelP95: "Billing preauth top model p95",
                billingReconcileAdjust: "Billing reconcile adjustments",
                billingReconcileFailed: "Billing reconcile failed",
                billingReconcileReleased: "Billing reconcile releases",
                billingReconcileScanned: "Billing reconcile scanned",
                billingReleaseWithoutCaptureRatio: "Billing release without capture ratio",
                billingSettleCompleteRatio: "Billing settle complete ratio",
                cacheHitRate: "Routing cache hit rate",
                failoverAttempts: "Failover attempts",
                failoverExhausted: "Failover exhausted",
                failoverSuccess: "Failover successes",
                failoverSuccessRate: "Failover success rate",
                sameAccountRetry: "Same-account quick retries",
                stickyHitRate: "Sticky hit rate"
            },
            na: "N/A",
            subtitle: "Data-plane automatic failover, sticky routing, and cache effectiveness.",
            title: "Failover & Cache Observability",
            unavailableDesc: "Check whether /internal/v1/debug/state is enabled and accessible from control-plane.",
            unavailableLoading: "Waiting for latest debug snapshot...",
            unavailableTitle: "Data-plane debug state unavailable"
        },
        searchPlaceholder: "Search component, details, version…",
        status: {
            checking: "Checking",
            degraded: "Degraded",
            healthy: "Healthy",
            offline: "Offline",
            unknown: "Unknown"
        },
        subtitle: "Infrastructure dependencies and health self-check.",
        title: "System Status"
    },
    tenantApiKeys: {
        actions: {
            changeGroup: "Change Group",
            disable: "Disable",
            enable: "Enable"
        },
        columns: {
            actions: "Actions",
            group: "Group",
            ipAllowlist: "IP Allowlist",
            modelAllowlist: "Model Allowlist",
            name: "Name",
            prefix: "Prefix",
            status: "Status"
        },
        create: {
            description: "Create an API key for the current tenant, configure IP allowlists, and choose a pricing group.",
            groupLabel: "API key group",
            ipAllowlistAriaLabel: "IP allowlist",
            ipAllowlistPlaceholder: "Optional: comma-separated IP allowlist",
            modelAllowlistAriaLabel: "Model allowlist",
            modelAllowlistPlaceholder: "Optional: comma-separated model allowlist",
            nameAriaLabel: "Key name",
            namePlaceholder: "Enter key name",
            submit: "Create Key",
            title: "Create API Key"
        },
        list: {
            description: "Manage API keys, update their group assignment, and review status.",
            empty: "No API keys yet.",
            searchPlaceholder: "Search by name, prefix, group or status",
            title: "API Key List"
        },
        messages: {
            createFailed: "Failed to create API key",
            createSuccess: "API key created successfully",
            plaintextShownOnce: "The plaintext key is shown only once. Save it now.",
            retryLater: "Please retry later",
            updateGroupFailed: "Failed to update API key group"
        },
        group: {
            allowAllModels: "All catalog models enabled",
            invalidHint: "This group was deleted. Choose a new group before making requests.",
            modelCount: "{{count}} configured models"
        },
        preview: {
            allowAllModels: "All catalog models are available in this group.",
            columns: {
                finalPrice: "Final price",
                formulaPrice: "Formula price",
                model: "Model"
            },
            description: "Current group: {{name}} · in {{input}} · cached {{cached}} · out {{output}}",
            empty: "No group available yet.",
            modelCount: "{{count}} models are configured in this group.",
            title: "Selected group preview"
        },
        status: {
            disabled: "Disabled",
            enabled: "Enabled",
            groupInvalid: "Group invalid"
        },
        subtitle: "Manage API keys and bind each key to a pricing and model group."
    },
    tenantApp: {
        appName: "Codex Pool Tenant",
        auth: {
            actions: {
                backToLogin: "Back to Sign In",
                login: "Sign In",
                openForgot: "Forgot password?",
                register: "Register",
                resetPassword: "Reset Password",
                sendResetCode: "Send Reset Code",
                switchToLogin: "Already have an account? Sign in",
                switchToRegister: "No account yet? Create one",
                verifyEmail: "Verify Email"
            },
            brand: {
                badge: "Tenant Workspace Access",
                points: {
                    audit: "Policy and billing decisions are observable end to end.",
                    resilience: "Failover-aware routing keeps requests available.",
                    security: "Credentials and account sessions are isolated by tenant."
                },
                subtitle: "Authenticate once, then manage usage, billing, and keys in one secure workspace.",
                title: "Stability-first access for enterprise AI operations"
            },
            error: {
                invalidCredentialsOrUnverified: "Sign-in failed: incorrect email or password, or email not verified yet.",
                loginFailed: "Sign-in failed.",
                passwordMismatch: "Password and confirm password do not match.",
                passwordResetFailed: "Password reset failed.",
                registerFailed: "Registration failed.",
                sendResetCodeFailed: "Failed to send reset code.",
                verificationFailed: "Verification failed."
            },
            fields: {
                confirmPassword: "Confirm Password",
                email: "Email",
                newPassword: "New Password",
                password: "Password",
                passwordMin8: "Password (at least 8 characters)",
                resetCode: "Reset Code",
                tenantName: "Tenant Name",
                verificationCode: "Verification Code"
            },
            forgot: {
                drawerHint: "After sending the code, a drawer appears from the bottom for reset code and new password.",
                stepResetPassword: "Set New Password",
                stepSendCode: "Send Code"
            },
            notice: {
                emailVerified: "Email verification successful. Please sign in with this account.",
                loginSuccess: "Sign-in successful.",
                passwordResetSuccess: "Password reset successful. Please sign in again.",
                registerDebugCode: "Registration successful, verification code (debug): {{code}}",
                registerSuccess: "Registration successful. Enter the email verification code to activate your account.",
                resetCodeDebug: "Password reset code (debug): {{code}}",
                resetCodeSentIfExists: "If the email exists, a reset code will be sent.",
                sessionExpired: "Tenant session expired. Please sign in again.",
                verifyCodeHint: "Didn’t receive it? Wait 60s and request a new code."
            },
            placeholders: {
                confirmPassword: "Re-enter password",
                email: "name@company.com",
                newPassword: "Enter a new password",
                password: "Enter your password",
                resetCode: "Enter reset code",
                tenantName: "Enter tenant name",
                verificationCode: "Enter verification code"
            },
            sections: {
                authSubtitle: "Switch between sign in and registration in one focused card.",
                forgotPasswordTitle: "Reset Password",
                forgotPasswordSubtitle: "Two-step drawer flow: send code first, then set a new password.",
                loginTitle: "Tenant Sign In",
                registerTitle: "Tenant Registration",
                verifyEmailSubtitle: "Use the code sent to your email to activate your account.",
                verifyEmailTitle: "Email Verification"
            },
            social: {
                comingSoon: "Third-party sign in (coming soon)",
                github: "GitHub",
                google: "Google"
            },
            tabs: {
                login: "Sign In",
                register: "Register"
            }
        },
        loadingPortal: "Loading tenant portal…",
        menu: {
            analytics: "Analytics",
            apiKeys: "API Keys",
            assets: "Assets",
            billing: "Billing Center",
            dashboard: "Dashboard",
            logs: "Logs",
            usage: "Usage"
        }
    },
    tenantBilling: {
        actions: {
            dailyCheckin: "Daily Checkin",
            exportCsv: "Export Csv"
        },
        failoverAction: {
            crossAccountFailover: "Cross Account Failover",
            retrySameAccount: "Retry Same Account",
            returnFailure: "Return Failure",
            unknown: "Unknown"
        },
        failureReason: {
            accountDeactivated: "Account Deactivated",
            billingUsageMissing: "Billing Usage Missing",
            failoverExhausted: "Failover Exhausted",
            noUpstreamAccount: "No Upstream Account",
            streamPreludeError: "Stream Prelude Error",
            tokenInvalidated: "Token Invalidated",
            transportError: "Transport Error",
            upstreamRequestFailed: "Upstream Request Failed",
            unknown: "Unknown"
        },
        filters: {
            day: "Day",
            dayShort: "D",
            granularityAriaLabel: "Billing granularity",
            month: "Month",
            monthShort: "M"
        },
        groupPricing: {
            allKeys: "All API keys",
            apiKeyAriaLabel: "API key selector",
            columns: {
                apiKey: "API key",
                finalPrice: "Final price",
                formulaPrice: "Formula price",
                group: "Group",
                model: "Model",
                state: "State"
            },
            description: "Review which pricing group each API key uses, and inspect effective model prices for a selected key.",
            groupSummary: "Configured models: {{count}} · allow-all: {{allowAll}}",
            invalidGroup: "This API key is bound to a deleted group. Requests will fail until you change the group.",
            state: {
                active: "Active",
                invalid: "Invalid (deleted group)"
            },
            title: "API key group pricing"
        },
        ledger: {
            columns: {
                balanceAfter: "Balance After",
                delta: "Delta",
                detail: "Detail",
                event: "Event",
                model: "Model",
                requestType: "Request Type",
                time: "Time"
            },
            description: "Tenant-filtered ledger entries.",
            detail: {
                charged: "Charged",
                extraCharge: "Extra Charge",
                failoverAction: "Failover Action",
                failure: "Failure",
                failureKeyword: "Failure Keyword",
                failureSummary: "{{failure}}（{{reason}}）",
                reconcileAdjust: "Reconcile Adjust",
                source: "Source",
                tokenSettle: "Token Settle",
                unitPrice: {
                    cached: "Cached",
                    input: "Input",
                    output: "Output",
                    summary: "Summary"
                },
                upstreamStatus: "Upstream {{status}}"
            },
            empty: "No ledger entries yet.",
            requestTypes: {
                nonStream: "Non-stream",
                stream: "Stream",
                unknown: "-"
            },
            showRaw: "Show Raw",
            title: "Ledger Entries"
        },
        messages: {
            checkinFailed: "Checkin Failed",
            checkinReward: "Checkin Reward",
            checkinSuccess: "Checkin Success",
            retryLater: "Retry Later"
        },
        releaseReason: {
            billingSettleFailed: "Billing Settle Failed",
            failoverExhausted: "Failover Exhausted",
            invalidUpstreamUrl: "Invalid Upstream Url",
            noUpstreamAccount: "No Upstream Account",
            streamPreludeError: "Stream Prelude Error",
            streamUsageMissing: "Stream Usage Missing",
            transportError: "Transport Error",
            upstreamRequestFailed: "Upstream Request Failed",
            unknown: "Unknown"
        },
        snapshot: {
            columns: {
                consumed: "Consumed",
                date: "Date",
                eventCount: "Event Count",
                month: "Month"
            },
            description: "Aggregate deduction events for settlement and reconciliation.",
            empty: "No settlement snapshots yet.",
            title: "Settlement Snapshot"
        },
        subtitle: "View tenant balance, consumption trend, and ledger details.",
        summary: {
            balance: "Current Balance",
            monthConsumed: "This Month Consumed",
            negativeOnly: "Negative deductions only",
            todayConsumed: "Today Consumed",
            unitCredits: "Unit: credits"
        },
        title: "Billing Center",
        tokenSegment: {
            cached: "Cached",
            input: "Input",
            output: "Output"
        },
        trend: {
            description: "Consumption trend aggregated by selected granularity.",
            empty: "No trend data yet.",
            series: {
                consumed: "Consumed"
            },
            title: "Consumption Trend"
        }
    },
    groupsPage: {
        actions: {
            create: "Create group",
            deleteGroup: "Delete group",
            deletePolicy: "Delete policy",
            saveGroup: "Save group",
            savePolicy: "Save model policy"
        },
        columns: {
            actions: "Actions",
            apiKeysCount: "API Keys {{count}}",
            modelsCount: "Models {{count}}",
            multipliers: "Multipliers",
            name: "Group",
            status: "Status",
            usage: "Usage"
        },
        editor: {
            createTitle: "Create group",
            description: "Configure group-wide multipliers and per-model pricing overrides.",
            editTitle: "Edit group"
        },
        empty: "No groups yet",
        form: {
            allowAllModels: "Allow all catalog models",
            cachedInputMultiplier: "Cached input multiplier (ppm)",
            default: "Default group",
            description: "Description",
            enabled: "Enabled",
            inputMultiplier: "Input multiplier (ppm)",
            name: "Group name",
            outputMultiplier: "Output multiplier (ppm)"
        },
        messages: {
            groupDeleted: "Group deleted.",
            groupDeleteFailed: "Failed to delete group.",
            groupSaved: "Group saved: {{name}}",
            groupSaveFailed: "Failed to save group.",
            policyDeleted: "Model policy deleted.",
            policyDeleteFailed: "Failed to delete model policy.",
            policySaved: "Model policy saved.",
            policySaveFailed: "Failed to save model policy."
        },
        policy: {
            cachedInputAbsolutePrice: "Cached input absolute price",
            cachedInputMultiplier: "Cached input multiplier (ppm)",
            description: "Select a model from the unified catalog, then configure multipliers or absolute pricing.",
            enabled: "Policy enabled",
            inputAbsolutePrice: "Input absolute price",
            inputMultiplier: "Input multiplier (ppm)",
            model: "Model",
            outputAbsolutePrice: "Output absolute price",
            outputMultiplier: "Output multiplier (ppm)",
            title: "Model policy"
        },
        preview: {
            columns: {
                finalPrice: "Final price",
                formulaPrice: "Formula price",
                mode: "Mode",
                model: "Model"
            },
            description: "Shows the final displayed price for the selected group.",
            moreHidden: "{{count}} more models are collapsed",
            mode: {
                absolute: "Absolute override",
                formula: "Multiplier formula"
            },
            title: "Effective model preview"
        },
        searchPlaceholder: "Search groups by name, description or status",
        status: {
            default: "Default",
            deleted: "Deleted",
            disabled: "Disabled",
            enabled: "Enabled"
        },
        subtitle: "Manage API key groups, model allowlists, multipliers, and group-level absolute prices.",
        title: "Group Management"
    },
    tenantDashboard: {
        actions: {
            manageApiKeys: "Manage API keys",
            refresh: "Refresh",
            viewBilling: "View billing",
            viewRequestLogs: "View request logs"
        },
        kpi: {
            avgFirstTokenSpeed: "Average first-token speed",
            avgFirstTokenSpeedDesc: "TTFT (streaming exact / non-stream approximate)",
            rpm: "RPM",
            rpmDesc: "Requests per minute",
            totalRequests: "Total requests",
            totalRequestsDesc: "Selected time range",
            totalTokens: "Token consumption total",
            totalTokensDesc: "Input + cached + output + reasoning",
            tpm: "TPM",
            tpmDesc: "Tokens per minute"
        },
        cards: {
            activeKeys: {
                description: "Note: only keys with requests are counted",
                title: "Active API key count (selected period)"
            },
            availableKeys: {
                description: "Based on tenant key management settings",
                title: "Currently available API keys"
            },
            keyEnableRate: {
                description: "Enabled ratio: {{rate}}% ({{enabled}} / {{total}} keys)",
                title: "Enabled key ratio"
            },
            peakHour: {
                description: "Highest request volume window",
                empty: "No data",
                title: "Peak hour"
            },
            requestVelocity: {
                description: "Average requests per hour in selected range",
                title: "Request velocity (per hour)"
            },
            totalRequests: {
                scopeAllKeys: " / all keys",
                scopePrefix: "Scope: current tenant",
                scopeSingleKey: " / single key",
                title: "Total tenant API key requests (selected period)"
            }
        },
        filters: {
            apiKeyAll: "All API keys",
            apiKeyAriaLabel: "API key filter",
            apiKeyHint: "Tip: use API key filter to isolate hotspots quickly.",
            range: {
                last24Hours: "Last 24 hours",
                last30Days: "Last 30 days",
                last7Days: "Last 7 days"
            },
            rangeAriaLabel: "Time range"
        },
        groupOverview: {
            allDescription: "How your current API keys are distributed across pricing groups.",
            empty: "No API key groups to show yet.",
            invalid: "Invalid",
            keysBound: "{{count}} API keys bound",
            singleDescription: "Current API key group binding and validity state.",
            title: "API key group overview",
            valid: "Valid"
        },
        hero: {
            badge: "Tenant Workspace Overview",
            summaryAllApiKeys: "(all API keys)",
            summaryPrefix: "Scope: current tenant ",
            summarySingleApiKey: "(single API key)"
        },
        subtitle: {
            allApiKeys: "(all API keys)",
            metricsFocus: "Focus metrics: TPM, RPM, total token consumption, total requests, and first-token speed.",
            scopePrefix: "Scope: current tenant ",
            singleApiKey: "(single API key)",
            timeWindow: ", time window: "
        },
        modelDistribution: {
            description: "Top models by request count or token usage.",
            empty: "No model distribution data yet",
            modeRequests: "By requests",
            modeTokens: "By tokens",
            other: "Other",
            title: "Model request distribution"
        },
        tokenComponents: {
            cached: "Cached",
            input: "Input",
            output: "Output",
            reasoning: "Reasoning"
        },
        tokenSummary: {
            title: "Token component summary"
        },
        tokenTrend: {
            description: "Hourly token trend by component. Toggle components to focus specific consumption.",
            empty: "No token trend data yet",
            title: "Token usage trend"
        },
        topKeys: {
            description: "Based on request volume in selected period",
            empty: "No API key usage rankings yet",
            requests: "{{count}} requests",
            share: "Share {{percent}}%",
            title: "Top API keys",
            unknownKey: "Unnamed key"
        },
        title: "Tenant Dashboard",
        trend: {
            description: "Scope: tenant API key request volume (hourly granularity)",
            empty: "No request data yet",
            series: {
                requests: "Requests"
            },
            title: "Request trend"
        }
    },
    tenantLogs: {
        audit: {
            actionValues: {
                adminOperation: "Admin operation",
                authOperation: "Auth operation",
                requestOperation: "Request operation",
                tenantOperation: "Tenant operation",
                unknown: "Unknown action"
            },
            actorTypes: {
                adminUser: "Admin user",
                apiKey: "API key",
                system: "System",
                tenantUser: "Tenant user",
                unknown: "Unknown actor"
            },
            columns: {
                action: "Action",
                actor: "Actor",
                reason: "Details",
                result: "Result",
                target: "Target",
                time: "Time"
            },
            description: "Definition: Control Plane audit events (current tenant only)",
            empty: "No audit log data",
            filters: {
                actionPlaceholder: "Action",
                actorIdPlaceholder: "Actor ID",
                actorTypePlaceholder: "Actor type",
                keywordPlaceholder: "Keyword (reason/payload)",
                resultStatusPlaceholder: "Result status",
                actionAriaLabel: "Action filter",
                actorIdAriaLabel: "Actor ID filter",
                actorTypeAriaLabel: "Actor type filter",
                keywordAriaLabel: "Keyword filter",
                rangeAriaLabel: "Time range",
                resultStatusAriaLabel: "Result status filter"
            },
            resultStatuses: {
                denied: "Denied",
                failed: "Failed",
                ok: "Success",
                unknown: "Unknown result"
            },
            title: "Audit Logs"
        },
        filters: {
            range: {
                last24Hours: "Last 24 hours",
                last30Days: "Last 30 days",
                last7Days: "Last 7 days"
            }
        },
        request: {
            columns: {
                apiKey: "API Key",
                error: "Error",
                latency: "Latency (ms)",
                path: "Path",
                requestId: "Request ID",
                status: "Status",
                time: "Time"
            },
            description: "Definition: Data Plane raw request events (current tenant only)",
            empty: "No log data",
            filters: {
                apiKeyIdPlaceholder: "API Key ID",
                keywordPlaceholder: "Keyword (path/error/model)",
                requestIdPlaceholder: "Request ID",
                statusCodePlaceholder: "Status code (e.g. 429)",
                apiKeyAriaLabel: "API key filter",
                keywordAriaLabel: "Keyword filter",
                rangeAriaLabel: "Time range",
                requestIdAriaLabel: "Request ID filter",
                statusCodeAriaLabel: "Status code filter"
            },
            title: "Request Logs"
        },
        scope: "Scope: current tenant only",
        tabs: {
            audit: "Audit Logs",
            request: "Request Logs"
        },
        title: "Logs"
    },
    tenantUsage: {
        columns: {
            apiKey: "API Key",
            requests: "Requests",
            tenantLabel: "Tenant: {{tenantId}}",
            time: "Time"
        },
        filters: {
            apiKeyAll: "All API Keys",
            apiKeyAriaLabel: "API key filter",
            range: {
                last24Hours: "Last 24 Hours",
                last30Days: "Last 30 Days",
                last7Days: "Last 7 Days"
            },
            rangeAriaLabel: "Time range"
        },
        hourly: {
            description: "Hourly request volume.",
            empty: "No data yet",
            title: "Hourly Requests"
        },
        leaderboard: {
            description: "API keys ranked by request volume.",
            empty: "No ranking data yet",
            title: "API Key Leaderboard"
        },
        subtitle: "View request trends and API key rankings.",
        title: "Usage Analysis",
        trend: {
            description: "Request trend over selected time range.",
            empty: "No trend data yet",
            title: "Request Trend"
        }
    },
    tenants: {
        create: {
            fields: {
                expiresAt: "Expires At",
                name: "Tenant Name",
                plan: "Plan (credit)",
                status: "Status (active/inactive)"
            },
            submit: "Create Tenant",
            title: "Create Tenant"
        },
        impersonation: {
            copyToken: "Copy Token",
            create: "Create Impersonation",
            fields: {
                reason: "Reason (required)"
            },
            revoke: "Revoke Session",
            sessionIdLabel: "Session ID:",
            tokenLabel: "Token:",
            title: "Admin Impersonation"
        },
        keys: {
            create: {
                fields: {
                    name: "Key Name",
                    namePlaceholder: "e.g. admin-main-key"
                },
                submit: "Create Key",
                title: "Create API Key"
            },
            created: {
                copyPlaintext: "Copy Plaintext Key",
                notice: "The plaintext key is shown only once. Save it now."
            },
            list: {
                caption: "Tenant API key list",
                columns: {
                    actions: "Actions",
                    createdAt: "Created At",
                    name: "Name",
                    prefix: "Prefix",
                    status: "Status"
                },
                copyPrefix: "Copy key prefix",
                disable: "Disable",
                empty: "No API keys for this tenant",
                enable: "Enable",
                status: {
                    active: "Active",
                    revoked: "Revoked"
                },
                title: "API Key List"
            }
        },
        list: {
            caption: "Tenant pool list",
            columns: {
                actions: "Actions",
                apiKeys: "API Keys",
                expiresAt: "Expires At",
                plan: "Plan",
                status: "Status",
                tenant: "Tenant",
                tenantId: "Tenant ID",
                updatedAt: "Updated At"
            },
            planValues: {
                credit: "Credit",
                unknown: "Custom ({{value}})"
            },
            statusValues: {
                active: "Active",
                inactive: "Inactive",
                unknown: "Unknown ({{value}})"
            },
            defaultBadge: "Default",
            empty: "No tenant data",
            openProfile: "Open Tenant Profile",
            searchPlaceholder: "Search tenants by name or ID",
            title: "Tenant Pool"
        },
        messages: {
            apiKeyCreateFailed: "Failed to create API key",
            apiKeyCreateSuccess: "Created API key for tenant {{tenantName}}: {{keyName}}",
            apiKeyNameRequired: "Please enter an API key name",
            apiKeyToggleFailed: "Failed to update API key status",
            createFailed: "Failed to create tenant",
            createSuccess: "Tenant created: {{name}} ({{id}})",
            impersonationCreateFailed: "Failed to create impersonation",
            impersonationCreated: "Impersonation session created (token returned)",
            impersonationRevokeFailed: "Failed to revoke impersonation",
            impersonationRevoked: "Impersonation session revoked",
            rechargeFailed: "Failed to recharge tenant",
            rechargeSuccess: "Recharge successful: +{{amount}}, current balance {{balance}}",
            updateFailed: "Failed to update tenant",
            updateSuccess: "Tenant updated: {{name}}"
        },
        profile: {
            dialogDescription: "Manage profile, API keys, and usage in one dialog with tabs.",
            dialogTitle: "Tenant Profile",
            dialogTitleWithName: "Tenant Profile · {{name}}",
            fields: {
                expiresAt: "Expires At",
                plan: "Plan",
                status: "Status"
            },
            meta: {
                createdAt: "Created At",
                tenantId: "Tenant ID",
                updatedAt: "Updated At"
            },
            save: "Save Profile",
            section: {
                title: "Tenant Profile"
            },
            tabs: {
                ariaLabel: "Tenant profile tabs",
                keys: "API Keys",
                profile: "Profile",
                usage: "Usage"
            }
        },
        recharge: {
            fields: {
                amount: "Microcredits (integer)",
                reason: "Reason"
            },
            submit: "Apply Recharge",
            title: "Tenant Recharge"
        },
        subtitle: "Check tenant availability and manage profiles, API keys, and usage.",
        title: "Tenants",
        usage: {
            filter: {
                allKeys: "All API keys",
                currentView: "Current view",
                label: "API key filter",
                noKeys: "No API keys for current tenant",
                noMatches: "No matching API keys",
                placeholder: "Search name / prefix / key_id"
            },
            meta: {
                tenantId: "Tenant ID"
            },
            metrics: {
                accountRequests: "Account requests",
                activeAccounts: "Active accounts",
                activeApiKeys: "Active API keys",
                apiKeyRequests: "API key requests",
                tenantApiKeyRequests: "Tenant API key requests"
            },
            sectionTitle: "Usage in the last 24 hours",
            status: {
                error: "Failed to load usage data",
                loading: "Loading usage data…"
            }
        }
    },
    theme: {
        aurora: "Aurora",
        colorful: "Colorful",
        dark: "Dark",
        light: "Light"
    },
    usage: {
        actions: {
            export: "Export CSV",
            filters: "Filters"
        },
        chart: {
            empty: "No data recorded in this period.",
            requests: "Requests",
            subtitle: "Aggregated usage across all providers",
            title: "Token Consumption (30 Days)"
        },
        subtitle: "Request consumption and infrastructure profiling.",
        title: "Usage Analysis",
        topKeys: {
            columns: {
                apiKey: "API Key",
                name: "Tenant / Key",
                requests: "Requests",
                share: "Share",
                tenant: "Tenant"
            },
            empty: "No usage recorded yet.",
            keyFallback: "API Key {{keyId}}",
            searchPlaceholder: "Search API keys…",
            subtitle: "By request volume",
            title: "Top API Keys",
            reqs: "reqs"
        }
    },
    cleanup: {
        title: "Credential Cleanup",
        subtitle: "Automated governance and lifecycle policies",
        save: "Save Policies",
        policy: {
            title: "Automatic governance policy",
            desc: "When refresh_token is reused, revoked, or continuously invalid, isolate accounts automatically to reduce blast radius.",
            refreshEnabled: "Enable OAuth auto refresh",
            refreshEnabledDesc: "If disabled, accounts will no longer auto-renew access_token.",
            intervalSec: "Refresh interval (seconds)",
            notes: "Policy notes"
        },
        workspace: {
            title: "OAuth account governance workspace",
            desc: "Check account login refresh status, then refresh now or pause/resume linked accounts.",
            searchPlaceholder: "Search by label / account id",
            onlyDisabled: "Only disabled accounts",
            loadingAccounts: "Loading accounts…",
            noAccounts: "No matching OAuth accounts.",
            enabled: "Enabled",
            disabled: "Disabled",
            selectHint: "Select an account on the left to view status.",
            loadingStatus: "Loading OAuth status…",
            noStatus: "No OAuth status available for this account yet.",
            refreshNow: "Refresh now",
            disableFamily: "Pause linked accounts",
            enableFamily: "Resume linked accounts",
            status: {
                never: "Not refreshed",
                ok: "Healthy",
                failed: "Failed"
            },
            fields: {
                refreshStatus: "Refresh status",
                reuseDetected: "Refresh token reuse detected",
                groupId: "Group ID",
                tokenVersion: "Token version",
                expiresAt: "Token expiry",
                errorCode: "Error code",
                errorMessage: "Error details"
            }
        },
        quarantine: {
            title: "Auto-Quarantine Policy",
            desc: "Automatically isolate accounts that fail authorization",
            threshold: "Failure Threshold",
            thresholdDesc: "Consecutive 401/403 errors before isolation",
            action: "Refresh Revoked Action",
            actionDesc: "When generic refresh_token is revoked",
            options: {
                family: "Quarantine Family",
                disable: "Disable Account Only",
                nothing: "Do Nothing"
            }
        },
        purge: {
            title: "Auto-Purge Policy",
            desc: "Permanently remove dead credentials to save space",
            retention: "Retention Period",
            retentionDesc: "Days to keep disabled accounts before purging"
        }
    },
    apiKeys: {
        title: "API Keys",
        subtitle: "Issue and manage secure access credentials for client applications.",
        create: "Create Secret Key",
        search: "Search key name or prefix…",
        loading: "Loading credentials…",
        empty: "No valid API keys found matching your criteria.",
        columns: {
            name: "Application Name",
            tenant: "Tenant ID",
            key: "API Key",
            status: "Status",
            issued: "Issued At",
            actions: "Actions"
        },
        status: {
            active: "Active",
            revoked: "Revoked"
        },
        defaultTenant: "Default",
        filters: {
            label: "Status filter",
            all: "All keys",
            active: "Active",
            revoked: "Revoked"
        },
        actions: {
            copyPrefixTitle: "Copy prefix",
            menu: "Key actions",
            copyPrefix: "Copy key prefix",
            processing: "Processing…",
            disable: "Disable key",
            enable: "Re-enable key"
        },
        messages: {
            createFailed: "Failed to create API key",
            missingName: "Please enter a key name"
        },
        dialog: {
            create: {
                title: "Create API Key",
                desc: "Create a Data Plane access key for a tenant. The plaintext key is shown only once.",
                nameLabel: "Key name",
                namePlaceholder: "e.g. prod-codex-clients",
                tenantLabel: "Tenant name (optional)",
                tenantPlaceholder: "Leave empty to use default",
                confirm: "Create",
                creating: "Creating…"
            },
            created: {
                title: "New key created",
                desc: "The plaintext key is shown only once. Please copy and store it now.",
                securityTip: "Security notice: once this dialog is closed, the plaintext key cannot be viewed again.",
                nameLabel: "Key name",
                plaintextLabel: "Plaintext key",
                close: "Close",
                copyPlaintext: "Copy plaintext key"
            }
        }
    }
}
