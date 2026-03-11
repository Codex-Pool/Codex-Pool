export default {
    accounts: {
        actions: {
            add: "新增帳號",
            apiKeyNoGroupAction: "API 金鑰帳號不支援同組操作",
            batchDelete: "批量刪除",
            batchDeleteConfirm: "確認刪除已選的 {{count}} 個帳號嗎？",
            batchDisable: "批量停用",
            batchEnable: "批量啟用",
            batchMenu: "批量操作",
            batchPauseFamily: "批量暫停同族群（{{count}}）",
            batchRefreshLogin: "批量刷新登入（{{count}}）",
            batchResumeFamily: "批量恢復同族群（{{count}}）",
            comfortableMode: "舒適模式",
            compactMode: "緊湊模式",
            delete: "刪除帳號",
            deleteConfirm: "確認刪除帳號 {{label}}？",
            disableAccount: "停用帳號",
            enableAccount: "啟用帳號",
            export: "匯出 CSV",
            filter: "篩選列表",
            oneTimeNoGroupAction: "一次性會話帳號不支援同組操作",
            pauseGroup: "暫停同組帳號",
            refreshAccounts: "刷新",
            refreshLogin: "刷新登入",
            refreshingAccounts: "刷新中",
            resumeGroup: "恢復同組帳號",
            selectAll: "全選目前篩選結果",
            selectOne: "選擇帳號 {{label}}",
            selectedCount: "已選 {{count}} 筆",
            viewDetails: "查看詳情",
            edit: "編輯屬性",
            refresh: "強制刷新",
            suspend: "掛起暫停",
            exportSuccess: "匯出成功",
            refreshTriggered: "已觸發帳號刷新"
        },
        columns: {
            actions: "操作",
            account: "帳號",
            added: "加入時間",
            credentialType: "憑證類型",
            health: "健康狀態",
            id: "帳號 ID",
            loginStatus: "登入狀態",
            nextRefresh: "下次刷新",
            plan: "方案",
            provider: "帳號類型",
            rateLimit: "Rate Limit 使用",
            binding: "綁定帳號 ID",
            unbound: "未綁定"
        },
        details: {
            description: "查看帳號資料、OAuth 狀態、限額與原始資料。",
            officialDescription: "這裡展示 OpenAI 官方模型資訊，唯讀。下方可以編輯手動價格覆蓋。",
            limitsTitle: "限額詳情",
            noOauthStatus: "暫無 OAuth 狀態資料",
            oauthNotApplicable: "此帳號類型不支援 OAuth 詳情",
            oauthTitle: "OAuth 狀態",
            profileTitle: "帳號資料",
            rawTitle: "原始資料",
            tabAria: "帳號詳情分頁",
            tabs: {
                limits: "限額",
                oauth: "OAuth",
                profile: "資料",
                raw: "原始"
            },
            sections: {
                cache: "限額快取",
                connection: "連線資訊",
                credentials: "憑證",
                identity: "身份資訊",
                refresh: "刷新狀態",
                subscription: "訂閱資訊"
            },
            fields: {
                email: "電子郵件",
                label: "標籤",
                mode: "帳號類型",
                accountId: "帳號 ID",
                enabled: "啟用狀態",
                baseUrl: "基礎 URL",
                chatgptAccountId: "ChatGPT 帳號 ID",
                priority: "優先級",
                createdAt: "建立時間",
                bearerToken: "Bearer 權杖",
                authProvider: "驗證提供方",
                credentialKind: "憑證類型",
                lastRefreshStatus: "最近刷新狀態",
                effectiveEnabled: "生效狀態",
                chatgptPlanType: "ChatGPT 方案",
                sourceType: "來源類型",
                tokenFamilyId: "Token 家族 ID",
                tokenVersion: "Token 版本",
                tokenExpiresAt: "Token 到期時間",
                nextRefreshAt: "下次刷新時間",
                lastRefreshAt: "最近刷新時間",
                refreshReusedDetected: "偵測到 Refresh 重用",
                lastRefreshErrorCode: "最近刷新錯誤碼",
                lastRefreshError: "最近刷新錯誤",
                rateLimitsFetchedAt: "限額擷取時間",
                rateLimitsExpiresAt: "限額到期時間",
                rateLimitsLastErrorCode: "限額最近錯誤碼",
                rateLimitsLastError: "限額最近錯誤",
                rawAccount: "帳號原始資料",
                rawOauthStatus: "OAuth 狀態原始資料"
            }
        },
        filters: {
            active: "正常",
            all: "全部",
            credential: "憑證類型",
            credentialAll: "全部憑證",
            credentialAt: "AT",
            credentialRt: "RT",
            credentialUnknown: "未知",
            disabled: "已停用",
            mode: "帳號類型",
            modeAll: "全部類型",
            modeApiKey: "API 金鑰",
            modeOAuth: "OAuth 會話",
            plan: "方案篩選",
            planAll: "全部方案",
            planUnknown: "未上報",
            total: "匹配 {{count}} 筆",
            suspended: "已掛起"
        },
        messages: {
            batchAllFailed: "{{action}}失敗",
            batchAllSuccess: "{{action}}完成",
            batchPartialFailed: "有 {{failed}} 個操作失敗{{error}}",
            batchPartialFailedTitle: "{{action}}部分失敗",
            batchSuccessCount: "成功 {{count}} 筆",
            batchUnknownError: "批量操作失敗",
            deleteFailed: "刪除帳號失敗",
            deleteSuccess: "帳號已刪除",
            disableFailed: "停用帳號失敗",
            disableSuccess: "帳號已停用",
            enableFailed: "啟用帳號失敗",
            enableSuccess: "帳號已啟用",
            exportSuccess: "匯出成功",
            pauseFamilyFailed: "暫停同族群帳號失敗",
            pauseFamilySuccess: "同族群帳號已暫停",
            rateLimitPollingTimeout: "輪詢帳號刷新任務逾時。",
            rateLimitRefreshFailedStatus: "帳號刷新任務失敗，狀態={{status}}",
            rateLimitRefreshFailedSummary: "帳號刷新任務失敗：{{summary}}",
            batchRefreshStarted: "已開始為 {{count}} 個帳號刷新登入",
            refreshFailed: "登入刷新失敗",
            refreshFailedStatus: "登入刷新失敗，狀態={{status}}",
            refreshFailedSummary: "登入刷新失敗：{{summary}}",
            refreshPollingTimeout: "輪詢登入刷新任務逾時。",
            refreshJobId: "任務 ID：{{jobId}}",
            refreshJobSummary: "任務 ID：{{jobId}} · {{processed}}/{{total}}",
            refreshListFailed: "刷新用量失敗",
            refreshListSuccess: "用量已刷新",
            refreshSuccess: "登入刷新完成",
            requestFailed: "請求失敗，請稍後重試",
            resumeFamilyFailed: "恢復同族群帳號失敗",
            resumeFamilySuccess: "同族群帳號已恢復",
            toggleUnsupported: "目前後端版本不支援帳號啟用/停用介面，請升級 control-plane。",
            refreshTriggered: "已開始刷新登入"
        },
        rateLimitRefreshJobStatus: {
            queued: "排隊中",
            running: "執行中",
            completed: "已完成",
            failed: "失敗",
            cancelled: "已取消",
            unknown: "未知"
        },
        mode: {
            apiKey: "API 金鑰",
            chatgptSession: "ChatGPT OAuth",
            codexOauth: "Codex OAuth",
            unknown: "其他"
        },
        nextRefresh: {
            none: "未排程"
        },
        oauth: {
            kindShort: {
                oneTime: "AT",
                refreshRotatable: "RT",
                unknown: "未知"
            },
            loading: "載入中",
            notApplicable: "-",
            provider: {
                legacyBearer: "舊版 Bearer 權杖",
                refreshToken: "Refresh Token"
            },
            sourceType: {
                codex: "Codex",
                unknown: "未知來源"
            },
            status: {
                failed: "失敗",
                never: "未刷新",
                ok: "正常"
            },
            unknownError: "未知錯誤",
            versionPrefix: "版本 ",
            planPrefix: "方案：",
            kind: {
                refreshRotatable: "可輪轉 Refresh Token 帳號",
                oneTime: "一次性 Access Token 帳號",
                unknown: "未知憑證類型"
            }
        },
        rateLimits: {
            labels: {
                fiveHours: "5小時限制",
                github: "GitHub",
                oneWeek: "週限制"
            },
            shortLabels: {
                fiveHours: "5h",
                oneWeek: "7d"
            },
            moreDetails: "查看更多（+{{count}}）",
            noReset: "暫無刷新時間",
            remainingPrefix: "剩餘",
            resetAt: "{{absolute}}（{{relative}}）重置",
            unavailable: "暫無額度資料",
            usedPrefix: "已用"
        },
        searchPlaceholder: "依電子郵件、標籤、URL 搜尋…",
        status: {
            active: "正常",
            disabled: "已停用"
        },
        subtitle: "在這裡查看帳號是否可用，並管理登入狀態",
        syncing: "正在同步帳號狀態…",
        title: "帳號池"
    },
    billing: {
        columns: {
            balanceAfter: "變動後餘額",
            billingDetail: "帳單詳情",
            deductedCredits: "扣除的積分",
            deductionEvents: "扣減事件",
            delta: "積分變動",
            eventType: "事件",
            model: "模型",
            periodDay: "日期",
            periodMonth: "月",
            requestType: "請求類型",
            source: "來源",
            timestamp: "時間"
        },
        exportCsv: "匯出 CSV",
        filters: {
            granularityAriaLabel: "計費粒度",
            tenantAriaLabel: "租戶篩選",
            tenantPlaceholder: "選擇租戶"
        },
        granularity: {
            day: "按日",
            month: "按月"
        },
        ledger: {
            codeLabels: {
                accountDeactivated: "帳號已停用",
                billingUsageMissing: "缺少使用結算字段",
                failoverExhausted: "重試/故障轉移已耗盡",
                noUpstreamAccount: "沒有可用的上游帳號",
                streamPreludeError: "流前奏錯誤",
                tokenInvalidated: "令牌失效",
                transportError: "上行網路錯誤",
                upstreamRequestFailed: "上游請求失敗",
                unknown: "未知"
            },
            details: {
                accrued: "應計：{{value}} 積分",
                adjustment: "調整：{{value}} 積分",
                extraCharge: "額外扣費：{{value}} 積分",
                failoverAction: "處理動作：{{action}}",
                failure: "失敗：{{summary}}",
                failurePrefix: "失敗：",
                source: "來源：{{source}}",
                upstreamStatus: "上游 {{status}}",
                tokenSettlement: "代幣結算：輸入{{input}} + 快取{{cached}} + 輸出{{output}}",
                unitPrice: "單價：{{prices}} 積分/1M 代幣"
            },
            empty: "暫無帳本流水。",
            failoverActions: {
                crossAccountFailover: "跨帳號故障轉移",
                retrySameAccount: "重試同一帳號",
                returnFailure: "回傳失敗",
                unknown: "未知"
            },
            releaseReasons: {
                billingSettleFailed: "帳單結算失敗",
                failoverExhausted: "重試/故障轉移已耗盡",
                invalidUpstreamUrl: "上游 URL 配置無效",
                noUpstreamAccount: "沒有可用的上游帳號",
                streamPreludeError: "流前奏錯誤",
                streamUsageMissing: "流使用缺失",
                transportError: "上行網路錯誤",
                upstreamRequestFailed: "上游請求失敗",
                unknown: "未知"
            },
            showRaw: "顯示原始條目",
            subtitle: "按當前租戶過濾。",
            title: "帳本流水",
            requestTypes: {
                nonStream: "非流式",
                stream: "流式",
                unknown: "-"
            },
            tokenSegments: {
                cached: "快取",
                input: "輸入",
                output: "輸出"
            }
        },
        messages: {
            rechargeFailedTitle: "儲值失敗",
            rechargeSuccessDetail: "+{{amount}}，餘額 {{balance}}",
            rechargeSuccessTitle: "儲值成功",
            retryLater: "請稍後重試"
        },
        recharge: {
            amountAriaLabel: "儲值金額",
            amountPlaceholder: "儲值積分（微積分）",
            reasonAriaLabel: "儲值原因",
            reasonPlaceholder: "儲值原因",
            submit: "執行儲值",
            subtitle: "為目前選擇的租戶儲值。",
            title: "管理員儲值"
        },
        snapshot: {
            empty: "還沒結算快照。",
            subtitle: "按 {{granularity}} 匯總扣除事件，用於月末結算和對帳。",
            title: "結算快照"
        },
        subtitle: "主要視圖：信用分類帳（實際費用），具有租戶層級管理過濾。",
        summary: {
            currentBalance: "當前餘額",
            deductionHint: "僅計算負帳本扣除事件。",
            monthConsumed: "本月消費",
            todayConsumed: "今日消費",
            unitCredits: "單位：積分"
        },
        title: "計費中心",
        trend: {
            noData: "尚無趨勢數據。",
            seriesConsumed: "消耗的積分",
            subtitle: "顯示以 {{granularity}} 匯總的分類帳扣除額。",
            title: "消費趨勢"
        }
    },
    common: {
        cancel: "取消",
        close: "關閉",
        collapseSidebar: "收合側邊欄",
        confirm: "確認",
        delete: "刪除",
        edit: "編輯",
        expand: "展開",
        collapse: "收起",
        expandSidebar: "展開側邊欄",
        loading: "讀取中…",
        logout: "登出",
        no: "否",
        noData: "目前沒有資料",
        openMenu: "打開選單",
        refresh: "刷新",
        skipToMainContent: "跳至主要內容",
        table: {
            firstPage: "第一頁",
            go: "跳轉",
            jumpToPage: "跳轉頁碼",
            lastPage: "最後一頁",
            nextPage: "下一頁",
            pageOf: "第 {{page}} / {{total}} 頁",
            previousPage: "上一頁",
            range: "{{start}}-{{end}} / 共 {{total}} 筆",
            rowsPerPage: "每頁筆數",
            searchLabel: "搜尋表格",
            searchPlaceholder: "搜尋目前列表…"
        },
        toggleLanguage: "切換語言",
        toggleTheme: "切換主題",
        yes: "是",
        save: "儲存",
        search: "搜尋…",
        routeLoading: "頁面載入中…",
        status: {
            healthy: "健康",
            degraded: "降級",
            offline: "離線",
            disabled: "停用",
            available: "可用"
        }
    },
    config: {
        controlPlane: {
            authValidateUrl: "驗證校驗位址",
            dataPlaneUrl: "轉發服務位址",
            desc: "設定控制面與轉發服務之間的連線位址",
            listen: "控制面監聽位址（唯讀）",
            title: "控制面連線設定"
        },
        refreshSettings: {
            desc: "控制登入憑證自動刷新的開關與頻率",
            enableDesc: "關閉後將不再自動更新即將過期的存取憑證。",
            enableLabel: "啟用自動刷新",
            intervalSec: "刷新間隔（秒）",
            notes: "備註",
            title: "自動刷新設定"
        },
        runtimeHint: {
            desc: "修改會立即生效。服務重啟後，仍以環境變數和 config.toml 為準。",
            title: "執行中設定提醒"
        },
        save: "儲存變更",
        subtitle: "運行時的系統設定與全域變數",
        success: "設定儲存成功。",
        title: "全域設定",
        reload: {
            title: "支援熱重載 (Runtime Reload)",
            desc: "對快取和超時等參數的修改將被全域同步，並立即生效，無須重啟服務。"
        },
        system: {
            title: "系統策略",
            desc: "配置全域操作上下文。",
            maintenance: "維護模式",
            maintenanceDesc: "全域拒絕所有新請求並回傳 503 狀態碼。",
            logRetention: "日誌保留 (天)",
            logRetentionDesc: "ClickHouse 追蹤儲存的滾動窗口期。"
        },
        network: {
            title: "網路畫像控制",
            desc: "定義全域速率限制以防止上游 API 封禁。",
            tpm: "全域每分鐘 Token 數 (TPM)",
            tpmDesc: "允許發送到 AI 端點的最大並發請求壓力。",
            scaling: "雲端資源自動擴縮",
            scalingDesc: "允許動態分配和回收底層節點資源。"
        }
    },
    dashboard: {
        actions: {
            viewBilling: "檢視帳單",
            viewLogs: "查看請求日誌"
        },
        alerts: {
            checkRoutes: "檢查路由",
            columns: {
                action: "建議操作",
                message: "告警內容",
                severity: "等級",
                source: "來源",
                status: "狀態",
                time: "時間"
            },
            empty: "系統運作穩定",
            resolve: "標記已解決",
            searchPlaceholder: "搜尋告警內容…",
            subtitle: "需要介入處理的系統隱患",
            title: "待辦告警",
            usageRepoUnavailable: "用量統計儲存目前不可用",
            dataPlaneDisconnected: "資料平面連線中斷",
            now: "剛剛",
            severity: {
                critical: "嚴重",
                warning: "警告",
                info: "資訊"
            },
            source: {
                data_plane: "資料平面",
                usage_repo: "用量倉庫"
            },
            status: {
                open: "待處理",
                resolved: "已解決"
            }
        },
        currentScope: "目前：{{scope}}",
        filters: {
            apiKeyAriaLabel: "API 金鑰篩選",
            apiKeyPlaceholder: "選擇 API 金鑰",
            range: {
                last24Hours: "過去 24 小時",
                last30Days: "過去 30 天",
                last7Days: "過去 7 天"
            },
            rangeAriaLabel: "時間範圍",
            scopeAriaLabel: "範圍篩選",
            tenantAriaLabel: "租戶篩選",
            tenantPlaceholder: "選擇租戶"
        },
        kpi: {
            activeApiKeysInRange: "活動 API 金鑰（選定範圍）",
            accounts: "帳號數",
            accountsDesc: "僅管理員可見的營運指標",
            apiKeys: "API 金鑰數",
            apiKeysDesc: "系統已配置金鑰數",
            avgFirstTokenSpeed: "平均首字速度",
            avgFirstTokenSpeedDesc: "TTFT（流式精確 / 非流式近似）",
            globalScope: "全球範圍",
            rpm: "RPM",
            rpmDesc: "每分鐘請求數",
            requests: {
                apiKey: "目前 API 金鑰請求（選定範圍）",
                global: "帳號請求總數（選定範圍）",
                tenant: "目前租用戶 API 金鑰請求（選定範圍）"
            },
            tenants: "租戶數",
            tenantsDesc: "僅管理員可見的營運指標",
            totalRequests: "總請求數",
            totalTokens: "Token 消耗總量",
            totalTokensDesc: "輸入 + 快取 + 輸出 + 推理",
            tpm: "TPM",
            tpmDesc: "每分鐘 Token 數",
            running: "運行中",
            totalConfigured: "已配置總數",
            uptime: "99.99% 上線率",
            reqs: "總請求量",
            reqsDesc: "環比上個月增長 12.5%",
            failed: "異常攔截",
            failedDesc: "今日攔截 180 次重試",
            avgLatency: "平均延遲",
            avgLatencyDesc: "比上週降低 5ms",
            activeTokens: "活躍權杖",
            activeTokensDesc: "新增 24 個可用模型",
            oauthLabel: "OAuth"
        },
        scope: {
            apiKey: "API 金鑰視圖",
            global: "全球視野",
            tenant: "租戶視圖"
        },
        subtitle: "閘道全域代理指標視角",
        table: {
            apiKey: "API 金鑰",
            requests: "請求數"
        },
        modelDistribution: {
            description: "依請求數或 Token 用量查看模型 Top 分布。",
            empty: "暫無模型分布資料",
            modeRequests: "依請求數",
            modeTokens: "依 Token",
            other: "其他",
            title: "模型請求分布"
        },
        tokenComponents: {
            cached: "快取輸入",
            input: "輸入",
            output: "輸出",
            reasoning: "推理"
        },
        tokenTrend: {
            description: "按小時展示 Token 元件趨勢，可透過開關聚焦消耗來源。",
            empty: "暫無 Token 趨勢資料",
            title: "Token 使用趨勢"
        },
        title: "服務總覽",
        topApiKeys: {
            empty: "暫無排名數據",
            scopeDescription: "範圍：{{scope}} /選定的時間窗口",
            title: "熱門 API 金鑰"
        },
        trafficChart: {
            scope: {
                apiKey: "範圍：目前 API 金鑰請求",
                global: "範圍：全域帳號請求 + 全域租用戶 API 金鑰請求",
                tenant: "範圍：目前租戶 API 金鑰請求"
            },
            series: {
                accountRequests: "帳號請求",
                tenantApiKeyRequests: "租戶 API 金鑰請求",
                tenantApiKeyRequestsSingle: "租戶 API 金鑰請求"
            },
            title: "全天流量趨勢",
            subtitle: "過去 24 小時各時段閘道傳輸量",
            success: "成功解析",
            blocked: "流控攔截"
        }
    },
    importJobs: {
        actions: {
            cancel: "取消",
            cancelJob: "取消任務",
            confirmCancelJob: "確定要取消這個匯入任務嗎？",
            exportFailed: "匯出失敗項",
            refreshItems: "刷新項目",
            remove: "移除",
            retryFailed: "重試失敗項",
            removeFromList: "從列表移除"
        },
        detail: {
            columns: {
                error: "錯誤訊息",
                label: "標籤",
                line: "行號",
                status: "狀態"
            },
            filterLabel: "狀態篩選",
            itemsEmpty: "沒有符合條件的條目。",
            itemsLoading: "正在載入任務條目…",
            jobIdLabel: "任務 ID：{{jobId}}",
            loadMore: "載入更多",
            loadedCount: "已載入 {{count}} 筆記錄",
            loadingMore: "載入中",
            retryQuery: "重試查詢",
            searchPlaceholderModern: "依 label / email / error 搜尋…",
            selectHint: "請選擇一個任務以查看詳情。",
            summaryLoading: "正在載入任務摘要…",
            title: "任務詳情",
            unreadable: "目前無法讀取此任務（可能已過期或 ID 無效）。",
            searchPlaceholder: "依 label / email / error 搜尋"
        },
        dropzone: {
            acceptsNew: "支援單次上傳多個 .json/.jsonl 檔案，後端會自動彙整為同一個任務。",
            creatingTitle: "正在建立匯入任務",
            selectFiles: "選擇檔案",
            titleNew: "拖曳匯入檔案到這裡",
            uploading: "正在上傳…",
            wait: "請稍候，檔案正在安全傳輸中。",
            title: "點擊上傳或拖曳檔案至此",
            accepts: "支援 <code>.csv</code> 或換行分隔的 <code>.txt</code> 格式。單一檔案最大 50,000 行。",
            browse: "瀏覽檔案",
            creatingDesc: "請稍候，任務建立成功後會自動加入右側佇列"
        },
        error: "上傳失敗",
        manual: {
            add: "新增",
            placeholderModern: "貼上 job_id…",
            title: "手動追蹤任務",
            placeholder: "貼上 job_id"
        },
        messages: {
            jobNotFound: "任務不存在或無法存取",
            queryFailed: "查詢失敗",
            unknownError: "未知錯誤",
            uploadFailedTitle: "匯入失敗"
        },
        errors: {
            importFailed: "匯入失敗",
            invalidRecord: "資料記錄無效",
            invalidRefreshToken: "refresh_token 無效",
            missingCredentials: "缺少憑證欄位",
            oauthProviderNotConfigured: "OAuth 提供者未設定",
            rateLimited: "請求頻率受限",
            refreshTokenReused: "refresh_token 已被使用",
            unknown: "未知匯入錯誤",
            upstreamNetworkError: "上游網路錯誤",
            upstreamUnavailable: "上游服務不可用"
        },
        metrics: {
            created: "新建",
            failed: "失敗",
            processed: "已處理",
            status: "狀態",
            throughput: "吞吐",
            total: "總數",
            updated: "已存在／已更新"
        },
        precheck: {
            createdNotice: "匯入任務已建立：{{id}}",
            defaultReady: "檔案格式與大小檢查通過。",
            duplicateBatch: "這些檔案已在待匯入清單中。",
            duplicateName: "偵測到同名檔案，建議確認來源後再匯入。",
            emptyPreview: "預檢未發現有效資料列，可能是空檔案。",
            firstLineInvalid: "JSONL 首行不是合法 JSON，匯入時可能失敗。",
            firstLineObject: "首行不是 JSON 物件，匯入時可能失敗。",
            firstLineValid: "JSONL 首行結構檢查通過。",
            jsonEmpty: "JSON 檔案內容為空。",
            jsonInvalid: "JSON 檔案不是合法 JSON，匯入時可能失敗。",
            jsonValid: "JSON 結構檢查通過。",
            noneImportable: "目前沒有可匯入檔案，請先修復阻塞問題。",
            skipLargeJson: "檔案較大，已略過本地 JSON 解析，匯入時由後端驗證。",
            status: {
                invalid: "阻塞",
                ready: "可匯入",
                warning: "需確認"
            }
        },
        progress: {
            done: "已完成",
            etaLabel: "預計剩餘：",
            etaMinutes: "約 {{count}} 分鐘",
            jobIdLabel: "任務 ID：{{jobId}}",
            lessThanMinute: "不到 1 分鐘",
            noJobSelected: "建立或選擇一個匯入任務後，這裡會顯示即時進度。",
            title: "即時匯入進度",
            topErrors: "主要錯誤分佈"
        },
        queue: {
            columns: {
                jobId: "任務 ID"
            },
            descRecent: "佇列會自動輪詢任務狀態，點擊任一任務可查看明細與錯誤。",
            emptyRecent: "暫無可追蹤任務，請先上傳檔案或手動輸入 job_id。",
            titleRecent: "最近匯入任務",
            tracked: "本地追蹤中",
            title: "最近追蹤的任務",
            empty: "當前工作階段尚未上傳任何任務。",
            card: {
                processed: "已處理",
                new: "新增",
                errors: "錯誤"
            }
        },
        status: {
            all: "全部",
            cancelled: "已取消",
            completed: "已完成",
            created: "新建",
            failed: "失敗",
            pending: "待處理",
            processing: "處理中",
            queued: "排隊中",
            running: "處理中",
            skipped: "略過",
            updated: "已存在／已更新"
        },
        subtitleModern: "先批量加入檔案，預檢後再一鍵匯入，並即時查看每個任務的進度與錯誤明細。",
        template: {
            downloadJsonl: "下載範本",
            title: "匯入範本",
            desc: "下載我們推薦的範本，以確保欄位嚴格符合系統要求。",
            download: "下載 CSV 範本",
            titleNew: "匯入範本",
            descNew: "下載 JSONL 範本並填入 refresh_token 後即可批量匯入。"
        },
        title: "批次匯入任務",
        validation: {
            fileTooLarge: "檔案 {{name}} 超過 20MB，請拆分後再匯入",
            unsupportedFormat: "檔案 {{name}} 格式不支援，僅支援 .json / .jsonl",
            format: "只允許上傳 .csv 或 .txt 格式的檔案。",
            size: "檔案過大，最大限制為 10MB。"
        },
        workspace: {
            clearQueue: "清空清單",
            columns: {
                action: "操作",
                check: "預檢結果",
                file: "檔案",
                size: "大小",
                status: "狀態"
            },
            confirmClear: "確定清空目前待匯入清單嗎？",
            desc: "支援拖曳或批量選擇檔案，先做預檢，再點擊「開始匯入」。",
            empty: "尚無待匯入檔案，先將檔案拖曳到上方區域。",
            invalidFiles: "阻塞 {{count}}",
            invalidHint: "有 {{count}} 個檔案存在阻塞問題，點擊匯入時會自動排除。",
            moreChecks: "條檢查訊息",
            readyFiles: "可匯入 {{count}}",
            readyHint: "預檢完成，可以直接開始匯入。",
            startImportWithCount: "開始匯入（{{count}} 個檔案）",
            stepCheck: "預檢",
            stepCheckDesc: "檢查格式、大小與重複名稱",
            stepImport: "開始匯入",
            stepImportDesc: "建立任務並查看即時進度",
            stepSelect: "新增檔案",
            title: "檔案匯入工作台",
            totalFiles: "檔案 {{count}} 個",
            totalSize: "總大小 {{size}}",
            warningFiles: "需確認 {{count}}"
        },
        subtitle: "透過嚴格格式的 CSV/TXT 檔案安全地上傳帳號憑證。"
    },
    oauthImport: {
        title: "OAuth 登入匯入",
        subtitle: "透過 Codex OAuth 登入，並將登入帳號直接匯入帳號池。",
        start: {
            title: "開始 Codex OAuth 登入",
            description: "先建立登入會話，完成 OAuth 授權後自動匯入帳號。"
        },
        form: {
            label: "帳號標籤（可選）",
            labelPlaceholder: "留空將根據電子郵件或帳號 ID 自動產生",
            baseUrl: "基礎 URL",
            priority: "優先級",
            enabled: "匯入後立即啟用帳號"
        },
        actions: {
            startLogin: "開始 OAuth 登入",
            reopenAuth: "重新開啟 OAuth 視窗",
            goAccounts: "前往帳號池",
            submitCallback: "提交回呼 URL"
        },
        status: {
            label: "會話狀態",
            idle: "尚未開始",
            waiting_callback: "等待回呼",
            exchanging: "正在交換權杖",
            importing: "正在匯入帳號",
            completed: "已完成",
            failed: "失敗",
            expired: "已過期",
            sessionId: "會話 ID：{{id}}",
            callbackUrl: "回呼位址：{{url}}",
            expiresAt: "過期時間：{{time}}"
        },
        error: {
            failed: "OAuth 匯入失敗。"
        },
        result: {
            success: "帳號匯入成功。",
            accountId: "帳號 ID：{{id}}",
            accountLabel: "標籤：{{label}}",
            email: "電子郵件：{{email}}",
            created: "新增",
            updated: "已存在"
        },
        manual: {
            title: "手動回呼備援",
            description: "當自動回呼無法到達時，可將完整回呼 URL 貼上並提交。",
            placeholder: "貼上包含 code/state 的完整回呼 URL…",
            hint: "僅在自動回呼失敗時使用。"
        },
        notifications: {
            popupBlockedTitle: "彈窗被封鎖",
            popupBlockedDescription: "請允許彈窗後重新開啟 OAuth 視窗。",
            sessionCreatedTitle: "會話已建立",
            sessionCreatedDescription: "OAuth 視窗已開啟，請完成登入授權。",
            sessionCreateFailedTitle: "建立會話失敗",
            manualSubmitTitle: "回呼已提交",
            manualSubmitSuccess: "手動回呼已完成並成功匯入帳號。",
            manualSubmitAccepted: "手動回呼已接收，正在處理中。",
            manualSubmitFailedTitle: "手動回呼失敗",
            unknownError: "發生未知錯誤，請稍後再試。"
        }
    },
    oauthProbe: {
        title: "OAuth 載荷探測",
        subtitle: "走一條獨立的 Codex OAuth 登入鏈路，只擷取回傳載荷，不把帳號匯入帳號池。",
        start: {
            title: "開始探測會話",
            description: "建立臨時探測會話，完成 OAuth 授權後，直接檢視並下載擷取到的 JSON。"
        },
        form: {
            baseUrl: "基礎 URL"
        },
        actions: {
            startProbe: "開始 OAuth 探測",
            reopenAuth: "重新開啟 OAuth 視窗",
            downloadJson: "下載 JSON",
            submitCallback: "提交回呼 URL"
        },
        status: {
            label: "探測狀態",
            idle: "尚未開始",
            waiting_callback: "等待回呼",
            exchanging: "正在交換權杖",
            importing: "正在處理載荷",
            completed: "已完成",
            failed: "失敗",
            expired: "已過期",
            sessionId: "會話 ID：{{id}}",
            callbackUrl: "回呼位址：{{url}}",
            expiresAt: "過期時間：{{time}}",
            memoryOnly: "探測結果只保存在記憶體中，會話過期或服務重啟後會消失。"
        },
        error: {
            failed: "OAuth 探測失敗。"
        },
        result: {
            success: "已成功擷取探測結果。",
            email: "電子郵件：{{email}}",
            accountId: "ChatGPT Account ID：{{id}}",
            plan: "方案：{{plan}}",
            expiresAt: "權杖過期時間：{{time}}",
            accessTokenPreview: "Access Token 預覽：{{value}}",
            refreshTokenPreview: "Refresh Token 預覽：{{value}}"
        },
        payload: {
            title: "擷取到的 JSON",
            description: "這裡顯示的是 OAuth code exchange 完成後保存在記憶體中的探測結果。",
            empty: "尚未擷取到探測載荷。"
        },
        manual: {
            title: "手動回呼備援",
            description: "當自動回呼無法到達時，可將完整回呼 URL 貼上並提交。",
            placeholder: "貼上包含 code/state 的完整回呼 URL…",
            hint: "僅在自動回呼失敗時使用。"
        },
        notifications: {
            popupBlockedTitle: "彈窗被封鎖",
            popupBlockedDescription: "請允許彈窗後重新開啟 OAuth 視窗。",
            sessionCreatedTitle: "探測會話已建立",
            sessionCreatedDescription: "OAuth 視窗已開啟，請完成登入以擷取回傳載荷。",
            sessionCreateFailedTitle: "建立探測會話失敗",
            manualSubmitTitle: "回呼已提交",
            manualSubmitSuccess: "手動回呼已完成，探測載荷已擷取。",
            manualSubmitAccepted: "手動回呼已接收，正在處理中。",
            manualSubmitFailedTitle: "手動回呼失敗",
            unknownError: "發生未知錯誤，請稍後再試。"
        }
    },
    login: {
        brand: {
            badge: "控制平面入口",
            points: {
                audit: "所有高風險操作都可透過 request id 全鏈路追蹤。",
                resilience: "高可用路由保障管理操作穩定可達。",
                security: "租戶隔離與憑證治理預設啟用。"
            },
            subtitle: "為系統管理員提供加固認證入口。",
            title: "以可控且可信的方式管理 Codex Pool"
        },
        messages: {
            failed: "登入失敗，請檢查帳號密碼",
            invalidCredentials: "帳號或密碼不正確，請重新輸入。",
            sessionExpired: "登入狀態已過期，請重新登入。"
        },
        password: "密碼",
        passwordPlaceholder: "請輸入管理員密碼",
        securityHint: "安全提示：連續失敗會在審計日誌中關聯記錄。",
        submit: "登入",
        subtitle: "使用管理員帳號登入控制台",
        title: "Codex-Pool 管理台",
        username: "使用者名稱",
        usernamePlaceholder: "請輸入管理員使用者名稱"
    },
    logs: {
        audit: {
            actionValues: {
                adminOperation: "管理操作",
                authOperation: "驗證操作",
                requestOperation: "請求操作",
                tenantOperation: "租戶操作",
                unknown: "未知操作"
            },
            actorTypes: {
                adminUser: "管理員使用者",
                apiKey: "API 金鑰",
                system: "系統",
                tenantUser: "租戶使用者",
                unknown: "未知操作者"
            },
            columns: {
                action: "操作",
                actor: "操作者",
                createdAt: "時間",
                reason: "原因",
                result: "結果",
                target: "目標",
                tenant: "租戶"
            },
            description: "範圍：控制平面審計事件（角色/操作/結果/目標/有效負荷）。",
            empty: "沒有可用的審計日誌數據",
            filters: {
                actionPlaceholder: "操作",
                actorIdPlaceholder: "操作者 ID",
                actorTypePlaceholder: "操作者類型",
                keywordPlaceholder: "關鍵字（原因/有效負載）",
                resultStatusPlaceholder: "結果狀態",
                actionAriaLabel: "操作篩選",
                actorIdAriaLabel: "操作者 ID 篩選",
                actorTypeAriaLabel: "操作者類型篩選",
                keywordAriaLabel: "關鍵字篩選",
                rangeAriaLabel: "時間範圍",
                resultStatusAriaLabel: "結果狀態篩選",
                tenantAriaLabel: "租戶篩選"
            },
            resultStatuses: {
                denied: "已拒絕",
                failed: "失敗",
                ok: "成功",
                unknown: "未知結果"
            },
            title: "審計日誌"
        },
        columns: {
            level: "級別",
            message: "日誌訊息",
            service: "服務節點",
            timestamp: "時間戳記"
        },
        export: "匯出日誌",
        filters: {
            allTenants: "所有租戶"
        },
        focus: "聚焦級別:",
        levels: {
            all: "所有級別",
            error: "錯誤",
            info: "資訊",
            warn: "警告"
        },
        messages: {
            configUpdated: "已更新執行時設定快照（記憶體）",
            empty: "無日誌詳情",
            keyCreated: "已建立 API 金鑰 {{keyId}}",
            keyPatched: "已設定 API 金鑰 {{keyId}} 啟用={{enabled}}",
            modelsLoaded: "已從上游帳號 {{label}} 載入模型列表",
            modelsProbed: "模型探測（{{trigger}}）：透過 {{label}} 測試 {{tested}} 個模型（可用 {{available}}，不可用 {{unavailable}}）",
            proxiesTested: "已檢測 {{count}} 個代理節點",
            systemState: "已查詢系統狀態：{{count}} 個帳號",
            unmappedAction: "{{action}} · {{message}}"
        },
        range: {
            last24Hours: "過去 24 小時",
            last30Days: "過去 30 天",
            last7Days: "過去 7 天"
        },
        request: {
            columns: {
                apiKey: "API 金鑰",
                createdAt: "時間",
                errorCode: "錯誤",
                latency: "延遲（毫秒）",
                path: "路徑",
                requestId: "請求 ID",
                status: "狀態",
                tenant: "租戶"
            },
            description: "範圍：原始資料平面請求事件（狀態/延遲/路徑/租戶/API 金鑰/請求 ID）。",
            empty: "沒有可用的請求日誌數據",
            filters: {
                apiKeyIdPlaceholder: "API 金鑰 ID",
                keywordPlaceholder: "關鍵字（路徑/錯誤/模型）",
                requestIdPlaceholder: "請求 ID",
                statusCodePlaceholder: "狀態代碼（例如 500）",
                apiKeyAriaLabel: "API 金鑰篩選",
                keywordAriaLabel: "關鍵字篩選",
                rangeAriaLabel: "時間範圍",
                requestIdAriaLabel: "請求 ID 篩選",
                statusCodeAriaLabel: "狀態碼篩選",
                tenantAriaLabel: "租戶篩選"
            },
            title: "請求日誌"
        },
        search: "搜尋負載或服務名…",
        subtitle: "即時的審計追蹤與運行時上下文。",
        tabs: {
            audit: "審計日誌",
            request: "請求日誌",
            system: "系統日誌"
        },
        title: "系統日誌",
        waiting: "等待接收新的日誌流…",
        actions: {
            systemState: "系統狀態查詢",
            configUpdate: "設定更新",
            proxiesTest: "節點健康檢查",
            keyCreate: "建立 API 金鑰",
            keyPatch: "更新 API 金鑰",
            modelsList: "拉取模型列表",
            modelsProbe: "模型探測",
            unknown: "未知操作（{{action}}）"
        }
    },
    models: {
        actions: {
            copyModelId: "複製模型名",
            createModel: "創建模型",
            syncOpenAiCatalog: "同步 OpenAI 目錄",
            probeAvailability: "探測可用性",
            openDetails: "詳情",
            deleteModel: "刪除模型",
            deletePricing: "刪除定價",
            deleteBillingRule: "刪除規則",
            editBillingRule: "編輯規則",
            editModel: "編輯模型",
            probeNow: "手動測試",
            saveModelProfile: "儲存模型設定檔",
            savePricing: "儲存價格",
            saveBillingRule: "儲存規則",
            newBillingRule: "新增規則",
            search: "搜尋模型 ID…",
            sync: "狀態同步"
        },
        availability: {
            available: "可用",
            issueHint: "查看不可用原因",
            neverChecked: "從未探測",
            noErrorDetail: "無錯誤詳情",
            unavailable: "不可用",
            unknown: "未探測",
            viewIssue: "查看原因"
        },
        cache: {
            fresh: "快取新鮮",
            stale: "快取過期"
        },
        catalog: {
            customOnly: "自訂模型",
            hidden: "目錄隱藏",
            listed: "目錄可見",
            unlisted: "未收錄"
        },
        columns: {
            actions: "詳情",
            availability: "可用性",
            cachedInputPrice: "緩存輸入價格",
            context: "上下文 / 最大輸出",
            modalities: "模態",
            syncedAt: "同步時間",
            catalog: "目錄狀態",
            checkedAt: "最近探測",
            id: "模型名稱",
            inputPrice: "輸入價格",
            outputPrice: "產出價格",
            pricingStatus: "定價狀態",
            provider: "提供商 / Hub"
        },
        description: "在這裡查看模型可用性，並管理模型資料與定價。",
        dialog: {
            description: "在此對話方塊中編輯設定檔和定價。儲存的定價將立即寫回模型池清單。",
            officialDescription: "這裡展示 OpenAI 官方模型資訊，唯讀。下方可以編輯手動價格覆蓋。",
            tabListAriaLabel: "模型資料分頁",
            titleWithId: "模型資料 · {{modelId}}"
        },
        empty: "目前未暴露或映射任何模型。",
        emptySyncRequired: "目前還沒有官方目錄，請先同步 OpenAI 官方目錄。",
        emptyActions: {
            goAccounts: "前往帳號池",
            importAccount: "導入帳號"
        },
        errors: {
            deleteModelEntityFailed: "刪除模型實體失敗。",
            deleteModelPricingFailed: "無法刪除模型定價。",
            openAiCatalogSyncFailed: "同步 OpenAI 官方目錄失敗。",
            deleteBillingRuleFailed: "刪除分段計價規則失敗。",
            modelIdRequired: "模型 ID 不能為空。",
            probeFailed: "模型探測失敗。",
            saveBillingRuleFailed: "儲存分段計價規則失敗。",
            saveModelPricingFailed: "無法儲存模型定價。",
            saveModelProfileFailed: "無法儲存模型設定檔。"
        },
        filters: {
            allProviders: "全部提供商",
            providerLabel: "提供商篩選"
        },
        form: {
            modelId: "模型 ID",
            modelIdLockedHint: "目前模型來自上游同步清單，請在上游來源中改名。",
            modelIdPlaceholder: "範例：gpt-5.3-codex",
            provider: "提供商",
            providerPlaceholder: "範例：openai / 自訂",
            source: "來源",
            sourceValues: {
                entityOnly: "僅實體",
                pricingOnly: "僅定價",
                upstream: "上游"
            },
            visibility: "可見性",
            visibilityPlaceholder: "範例：list / hide"
        },
        hints: {
            cannotDeleteMissingPricing: "目前模型沒有本地定價資料，請先補上再刪除。",
            cannotDeleteNonLocalEntity: "目前模型不是本機實體模型，因此無法刪除其實體。"
        },
        loadingHint: "正在檢測目錄與可用性狀態，完成後會自動顯示最新模型清單。",
        notice: {
            modelEntityDeleted: "模型實體已刪除。",
            modelPricingDeleted: "模型定價記錄已刪除。",
            billingRuleDeleted: "分段計價規則已刪除。",
            modelPricingSaved: "已儲存模型定價：{{model}}",
            openAiCatalogSynced: "OpenAI 官方目錄同步完成：更新 {{count}} 個模型。",
            billingRuleSaved: "分段計價規則已儲存：{{model}}",
            modelProfileSaved: "模型資料已儲存：{{model}}",
            probeCompleted: "模型探測完成。最新模型池已同步。"
        },
        pricing: {
            cachedInputPrice: "緩存輸入價格",
            creditsPerMillionTokens: "積分 / 1M 代幣",
            disabled: "停用",
            enablePricing: "啟用定價",
            enabled: "啟用",
            inputPrice: "輸入價格",
            notConfigured: "未配置",
            outputPrice: "產出價格",
            perMillionTokensMicrocredits: "每 100 萬 token，單位微積分",
            sectionTitle: "模型定價",
            effectiveSectionTitle: "實際計費價格",
            manualOverride: "手動覆蓋",
            officialBase: "官方基礎價",
            overrideSectionTitle: "手動價格覆蓋",
            sourceLabels: {
                officialSync: "OpenAI 官方",
                manualOverride: "手動覆蓋",
                unknown: "未知"
            },
        },
        rules: {
            cachedInputMultiplierPpm: "快取輸入倍率（ppm）",
            empty: "目前模型尚未設定分段計價規則。",
            enableRule: "啟用規則",
            inputMultiplierPpm: "輸入倍率（ppm）",
            noThreshold: "無",
            outputMultiplierPpm: "輸出倍率（ppm）",
            priorityLabel: "優先級",
            requestKind: "請求類型",
            requestKinds: {
                any: "任意",
                chat: "Chat",
                compact: "Compact",
                response: "Responses",
                unknown: "未知"
            },
            ruleSummary: "閾值 {{threshold}} · 輸入 ×{{inputMultiplier}} · 快取 ×{{cachedMultiplier}} · 輸出 ×{{outputMultiplier}}",
            scope: "作用範圍",
            scopes: {
                request: "單次請求",
                session: "會話",
                unknown: "未知"
            },
            sectionDescription: "為長上下文或特殊計費 band 設定按請求/會話生效的倍率規則。",
            sectionTitle: "分段計價規則",
            thresholdInputTokens: "輸入 Token 閾值"
        },
        probeSourceUnknown: "未知帳號",
        probeSummary: "探測快取：{{stale}}，最近 {{checkedAt}}，快取時長 {{ttlHours}} 小時，來源 {{source}}",
        syncHint: {
            notSynced: "OpenAI 官方目錄尚未同步。",
            syncedAt: "目錄已於 {{time}} 同步"
        },
        profile: {
            sectionTitle: "模型資料"
        },
        syncing: "正在映射端點可用性…",
        tabs: {
            pricing: "定價",
            profile: "資料"
        },
        title: "模型池",
        subtitle: "這裡顯示目前帳號可用的模型清單",
        detail: {
            title: "模型詳情",
            notFound: "找不到該模型，可能已被移除或目前篩選不可見。",
            httpStatus: "HTTP 狀態",
            error: "錯誤詳情",
            noError: "無錯誤詳情",
            officialTitle: "官方中繼資料",
            officialDescription: "這裡展示 OpenAI 官方模型資訊，唯讀。下方可以編輯手動價格覆蓋。",
            contextWindow: "上下文視窗",
            maxOutputTokens: "最大輸出 Token",
            knowledgeCutoff: "知識截止日期",
            reasoningTokenSupport: "推理 Token 支援",
            sourceUrl: "來源連結",
            openOfficialPage: "打開官方頁面",
            capabilitiesTitle: "能力資訊",
            inputModalities: "輸入模態",
            outputModalities: "輸出模態",
            endpoints: "支援端點",
            rawText: "官方文字快照",
        }
    },
    nav: {
        accounts: "帳號池",
        aiRouting: "AI 調度",
        apiKeyGroups: "分組管理",
        apiKeys: "API 金鑰",
        billing: "計費",
        config: "全域設定",
        dashboard: "服務總覽",
        groups: {
            analytics: "數據分析",
            assets: "資產池",
            operations: "營運操作",
            system: "系統管理"
        },
        importJobs: "批次上傳",
        oauthImport: "登入匯入",
        oauthProbe: "載荷探測",
        logs: "系統日誌",
        mainNavigation: "主導覽",
        models: "模型池",
        online: "上線",
        openNavigation: "開啟導覽",
        proxies: "代理池",
        system: "節點健康",
        tenants: "租戶池",
        usage: "用量帳單",
        cleanup: "憑證治理",
        closeNavigation: "關閉導覽"
    },
    notifications: {
        dismiss: "關閉通知",
        loginFailed: {
            title: "登入失敗"
        },
        sessionExpired: {
            title: "登入狀態已過期",
            description: "請重新登入後再繼續操作。"
        }
    },
    errors: {
        common: {
            failed: "失敗",
            network: "網路錯誤，請檢查網路連線。",
            timeout: "請求逾時，請稍後再試。"
        },
        api: {
            unauthorized: "未授權，請重新登入。",
            invalidRequest: "請求參數無效。",
            notFound: "資源不存在。",
            serviceUnavailable: "服務暫不可用。",
            internalError: "伺服器內部錯誤。",
            oauthProviderNotConfigured: "OAuth 服務未設定。",
            oauthCallbackListenerUnavailable: "OAuth 回呼監聽不可用。",
            invalidRefreshToken: "Refresh token 無效或已過期。",
            refreshTokenReused: "Refresh token 已重複使用，請重新取得最新 refresh token。",
            refreshTokenRevoked: "Refresh token 已被撤銷。",
            oauthMissingClientId: "OAuth 服務設定不完整（缺少 client_id）。",
            oauthUnauthorizedClient: "OAuth 用戶端未授權。",
            upstreamUnavailable: "上游服務不可用。",
            upstreamNetworkError: "上游網路錯誤。",
            oauthExchangeFailed: "OAuth 交換失敗。"
        },
        http: {
            badRequest: "請求錯誤",
            unauthorized: "未授權",
            forbidden: "無權限",
            notFound: "未找到",
            conflict: "衝突",
            payloadTooLarge: "請求內容過大",
            rateLimited: "請求過於頻繁",
            internalServerError: "伺服器錯誤",
            badGateway: "閘道錯誤",
            serviceUnavailable: "服務不可用",
            gatewayTimeout: "閘道逾時"
        }
    },
    proxies: {
        check: "立即檢查",
        columns: {
            actions: "操作",
            health: "健康狀況",
            lastPing: "最近檢查時間",
            latency: "平均回應時間",
            url: "代理節點 URL",
            weight: "路由權重"
        },
        empty: "尚未配置任何後端代理節點。",
        filters: {
            all: "全部節點",
            degraded: "降級",
            disabled: "已停用",
            healthy: "健康",
            label: "健康篩選",
            offline: "離線"
        },
        health: {
            degraded: "降級",
            disabled: "已停用",
            healthy: "健康",
            offline: "離線"
        },
        loading: "掃描網路拓撲中…",
        manage: "查看",
        pending: "尚未檢查",
        retry: "重試",
        searchPlaceholder: "搜尋節點 URL 或標籤…",
        subtitle: "在這裡查看代理是否可用，並管理代理節點。",
        title: "代理池"
    },
    system: {
        columns: {
            component: "元件",
            details: "詳情",
            status: "狀態",
            uptime: "運行時間",
            version: "版本"
        },
        components: {
            controlPlane: "控制面",
            dataPlane: "資料面路由",
            usageRepo: "用量倉庫"
        },
        details: {
            analyticsUnavailable: "統計服務暫時不可用",
            apiActive: "服務運作中",
            checkingAPI: "正在檢查服務…",
            dbConnected: "統計儲存已連接",
            endpointsResponding: "請求轉發正常"
        },
        labels: {
            local: "本服務",
            remote: "轉發服務",
            storage: "統計儲存",
            version: "版本：",
            uptime: "運行時間"
        },
        observability: {
            badges: {
                failoverOff: "故障切換：關閉",
                failoverOn: "故障切換：開啟",
                failoverWait: "切號等待 {{value}} ms",
                quickRetry: "同帳號快速重試 ≤ {{value}}",
                retryPoll: "輪詢間隔 {{value}} ms",
                sharedCacheOff: "共享快取：關閉",
                sharedCacheOn: "共享快取：開啟",
                stickyConflictAvoidOff: "黏性衝突回避：關閉",
                stickyConflictAvoidOn: "黏性衝突回避：開啟"
            },
            hints: {
                billingPreauthCaptureMissingTotal: "計費預身份驗證擷取缺失總數",
                billingPreauthErrorRatioAvg: "計費預驗證錯誤率平均值",
                billingPreauthErrorRatioP95: "計費預驗證錯誤率 p95",
                billingPreauthTopModelP95: "樣本最多模型預授權誤差 P95",
                billingReconcileAdjust: "對帳任務自動寫入的餘額修正次數。",
                billingReconcileFailed: "對帳執行失敗、需要排查的次數。",
                billingReconcileReleased: "對帳任務自動關閉授權的次數。",
                billingReconcileScanned: "對帳任務從 request_log 與帳本掃描到的事實數。",
                billingReleaseWithoutCaptureRatio: "無捕獲率計費發布",
                billingSettleCompleteRatio: "計費結算完成率",
                cacheHitRate: "本地 + 共享 sticky 快取查詢的綜合命中比例。",
                failoverAttempts: "請求在帳號間進行 failover 的總嘗試次數。",
                failoverExhausted: "重試與切號預算耗盡後仍失敗的請求次數。",
                failoverSuccess: "切換帳號後最終恢復成功的請求次數。",
                failoverSuccessRate: "故障切換嘗試中的成功占比。",
                sameAccountRetry: "切號前在同一帳號做快速重試的總次數。",
                stickyHitRate: "會話黏性映射命中的比例。"
            },
            metrics: {
                billingPreauthCaptureMissingTotal: "計費預身份驗證擷取缺失總數",
                billingPreauthErrorRatioAvg: "計費預驗證錯誤率平均值",
                billingPreauthErrorRatioP95: "計費預驗證錯誤率 p95",
                billingPreauthTopModelP95: "樣本最多模型預授權誤差 P95",
                billingReconcileAdjust: "計費對帳調帳數",
                billingReconcileFailed: "計費對帳失敗數",
                billingReconcileReleased: "計費對帳釋放數",
                billingReconcileScanned: "計費對帳掃描數",
                billingReleaseWithoutCaptureRatio: "無捕獲率計費發布",
                billingSettleCompleteRatio: "計費結算完成率",
                cacheHitRate: "路由快取命中率",
                failoverAttempts: "故障切換嘗試次數",
                failoverExhausted: "故障切換耗盡次數",
                failoverSuccess: "故障切換成功次數",
                failoverSuccessRate: "故障切換成功率",
                sameAccountRetry: "同帳號快速重試次數",
                stickyHitRate: "會話黏性命中率"
            },
            na: "暫無",
            subtitle: "觀測資料面自動切號、會話黏性與快取命中效果。",
            title: "故障切換與快取觀測",
            unavailableDesc: "請檢查 control-plane 對 /internal/v1/debug/state 的存取與權限設定。",
            unavailableLoading: "正在等待最新除錯快照…",
            unavailableTitle: "暫未取得資料面除錯快照"
        },
        searchPlaceholder: "搜尋元件、詳情或版本…",
        status: {
            checking: "檢查中",
            degraded: "降級",
            healthy: "健康",
            offline: "離線",
            unknown: "未知"
        },
        subtitle: "查看各個核心服務是否正常運作。",
        title: "系統狀態"
    },
    tenantApiKeys: {
        actions: {
            changeGroup: "變更分組",
            disable: "停用",
            enable: "啟用"
        },
        columns: {
            actions: "操作",
            group: "分組",
            ipAllowlist: "IP 白名單",
            modelAllowlist: "模型白名單",
            name: "名稱",
            prefix: "前綴",
            status: "狀態"
        },
        create: {
            description: "為目前租戶建立 API 金鑰並設定存取限制。",
            groupLabel: "API Key 分組",
            ipAllowlistAriaLabel: "IP 白名單",
            ipAllowlistPlaceholder: "選填：以逗號分隔 IP 白名單",
            modelAllowlistAriaLabel: "模型白名單",
            modelAllowlistPlaceholder: "選填：以逗號分隔模型白名單",
            nameAriaLabel: "金鑰名稱",
            namePlaceholder: "請輸入金鑰名稱",
            submit: "建立金鑰",
            title: "建立 API 金鑰"
        },
        list: {
            description: "管理目前租戶的 API 金鑰。",
            empty: "暫無 API 金鑰。",
            searchPlaceholder: "依名稱或前綴搜尋 API 金鑰",
            title: "API 金鑰列表"
        },
        messages: {
            createFailed: "建立 API 金鑰失敗",
            createSuccess: "API 金鑰建立成功",
            plaintextShownOnce: "明文金鑰僅顯示一次，請立即儲存。",
            retryLater: "稍後重試",
            updateGroupFailed: "更新 API Key 分組失敗"
        },
        group: {
            allowAllModels: "允許全部目錄模型",
            invalidHint: "此分組已刪除，請在發出請求前重新選擇分組。",
            modelCount: "已設定 {{count}} 個模型"
        },
        preview: {
            allowAllModels: "此分組可使用全部目錄模型。",
            columns: {
                finalPrice: "最終價格",
                formulaPrice: "公式價格",
                model: "模型"
            },
            description: "目前分組：{{name}} · 輸入 {{input}} · 快取 {{cached}} · 輸出 {{output}}",
            empty: "目前沒有可用分組。",
            modelCount: "此分組已設定 {{count}} 個模型。",
            title: "目前分組預覽"
        },
        status: {
            disabled: "停用",
            enabled: "啟用",
            groupInvalid: "分組失效"
        },
        subtitle: "管理目前租戶的 API 金鑰與存取策略。"
    },
    tenantApp: {
        appName: "Codex 池租戶",
        auth: {
            actions: {
                backToLogin: "返回登入",
                login: "登入",
                openForgot: "忘記密碼？",
                register: "登記",
                resetPassword: "重設密碼",
                sendResetCode: "發送重置代碼",
                switchToLogin: "已有帳號？前往登入",
                switchToRegister: "還沒有帳號？立即註冊",
                verifyEmail: "驗證信箱"
            },
            brand: {
                badge: "租戶工作區入口",
                points: {
                    audit: "策略與計費決策全程可觀測、可審計。",
                    resilience: "具備故障切換感知的路由保持請求可用。",
                    security: "憑證與會話按租戶隔離。"
                },
                subtitle: "完成認證後，在同一安全工作區管理用量、計費與金鑰。",
                title: "面向企業 AI 營運的穩定存取入口"
            },
            error: {
                invalidCredentialsOrUnverified: "登入失敗：電子郵件或密碼不正確，或電子郵件尚未驗證。",
                loginFailed: "登入失敗。",
                passwordMismatch: "兩次輸入的密碼不一致。",
                passwordResetFailed: "密碼重置失敗。",
                registerFailed: "註冊失敗。",
                sendResetCodeFailed: "發送重置代碼失敗。",
                verificationFailed: "驗證失敗。"
            },
            fields: {
                confirmPassword: "確認密碼",
                email: "電子郵件",
                newPassword: "新密碼",
                password: "密碼",
                passwordMin8: "密碼（至少8個字元）",
                resetCode: "重置程式碼",
                tenantName: "租戶名稱",
                verificationCode: "驗證碼"
            },
            forgot: {
                drawerHint: "送出驗證碼後，會自底部上滑抽屜顯示「重置碼 + 新密碼」輸入。",
                stepResetPassword: "設定新密碼",
                stepSendCode: "發送驗證碼"
            },
            notice: {
                emailVerified: "郵箱驗證成功。請使用此帳號登入。",
                loginSuccess: "登入成功。",
                passwordResetSuccess: "密碼重置成功。請重新登入。",
                registerDebugCode: "註冊成功，驗證碼（偵錯）：{{code}}",
                registerSuccess: "註冊成功。輸入電子郵件驗證碼以啟動您的帳號。",
                resetCodeDebug: "密碼重設代碼（偵錯）：{{code}}",
                resetCodeSentIfExists: "如果電子郵件存在，將發送重置代碼。",
                sessionExpired: "租戶會話已過期。請重新登入。",
                verifyCodeHint: "尚未收到驗證碼？請等待 60 秒後重新發送。"
            },
            placeholders: {
                confirmPassword: "請再次輸入密碼",
                email: "name@company.com",
                newPassword: "請輸入新密碼",
                password: "請輸入密碼",
                resetCode: "請輸入重置代碼",
                tenantName: "請輸入租戶名稱",
                verificationCode: "請輸入驗證碼"
            },
            sections: {
                authSubtitle: "在同一卡片中切換登入與註冊。",
                forgotPasswordTitle: "重設密碼",
                forgotPasswordSubtitle: "抽屜式兩步流程：先發送驗證碼，再設定新密碼。",
                loginTitle: "租戶登入",
                registerTitle: "租戶登記",
                verifyEmailSubtitle: "輸入郵件中的驗證碼以啟用帳號。",
                verifyEmailTitle: "電子郵件驗證"
            },
            social: {
                comingSoon: "第三方登入（即將支援）",
                github: "GitHub",
                google: "Google"
            },
            tabs: {
                login: "登入",
                register: "註冊"
            }
        },
        loadingPortal: "正在載入租用戶入口網站…",
        menu: {
            analytics: "分析",
            apiKeys: "API 金鑰",
            assets: "資產",
            billing: "計費中心",
            dashboard: "儀表板",
            logs: "日誌",
            usage: "用量"
        }
    },
    tenantBilling: {
        actions: {
            dailyCheckin: "每日簽到",
            exportCsv: "匯出 CSV"
        },
        failoverAction: {
            crossAccountFailover: "跨帳號故障轉移",
            retrySameAccount: "重試同一帳號",
            returnFailure: "回傳失敗",
            unknown: "未知"
        },
        failureReason: {
            accountDeactivated: "帳號已停用",
            billingUsageMissing: "帳單使用情況缺失",
            failoverExhausted: "故障轉移已耗盡",
            noUpstreamAccount: "無上游帳號",
            streamPreludeError: "流前奏錯誤",
            tokenInvalidated: "令牌失效",
            transportError: "傳輸錯誤",
            upstreamRequestFailed: "上游請求失敗",
            unknown: "未知"
        },
        filters: {
            day: "按日",
            dayShort: "日",
            granularityAriaLabel: "計費粒度",
            month: "按月",
            monthShort: "月"
        },
        groupPricing: {
            allKeys: "全部 API Key",
            apiKeyAriaLabel: "API Key 選擇器",
            columns: {
                apiKey: "API Key",
                finalPrice: "最終價格",
                formulaPrice: "公式價格",
                group: "分組",
                model: "模型",
                state: "狀態"
            },
            description: "查看每個 API Key 目前使用的計價分組，並依單一 API Key 檢查有效模型價格。",
            groupSummary: "已設定模型：{{count}} · 全量開放：{{allowAll}}",
            invalidGroup: "此 API Key 綁定到了已刪除分組，在你變更分組前請求都會失敗。",
            state: {
                active: "有效",
                invalid: "失效（分組已刪除）"
            },
            title: "API Key 分組定價"
        },
        ledger: {
            columns: {
                balanceAfter: "變動後餘額",
                delta: "積分變動",
                detail: "計費明細",
                event: "事件",
                model: "模型",
                requestType: "請求類型",
                time: "時間"
            },
            description: "依目前租戶篩選的帳本流水。",
            detail: {
                charged: "實扣",
                extraCharge: "額外扣費",
                failoverAction: "失敗處理動作",
                failure: "失敗原因",
                failureKeyword: "失敗關鍵字",
                failureSummary: "{{failure}}（{{reason}}）",
                reconcileAdjust: "對帳調整",
                source: "來源",
                tokenSettle: "Token 結算",
                unitPrice: {
                    cached: "快取",
                    input: "輸入",
                    output: "輸出",
                    summary: "單價彙總"
                },
                upstreamStatus: "上游 {{status}}"
            },
            empty: "暫無帳本流水",
            requestTypes: {
                nonStream: "非流式",
                stream: "流式",
                unknown: "-"
            },
            showRaw: "顯示原始帳本",
            title: "帳本流水"
        },
        messages: {
            checkinFailed: "簽到失敗",
            checkinReward: "簽到獎勵",
            checkinSuccess: "簽到成功",
            retryLater: "稍後重試"
        },
        releaseReason: {
            billingSettleFailed: "計費結算失敗",
            failoverExhausted: "故障轉移已耗盡",
            invalidUpstreamUrl: "無效的上游網址",
            noUpstreamAccount: "無上游帳號",
            streamPreludeError: "流前奏錯誤",
            streamUsageMissing: "流使用缺失",
            transportError: "傳輸錯誤",
            upstreamRequestFailed: "上游請求失敗",
            unknown: "未知"
        },
        snapshot: {
            columns: {
                consumed: "消耗積分",
                date: "日期",
                eventCount: "扣減事件數",
                month: "月"
            },
            description: "按週期彙總扣減事件，用於結算與對帳。",
            empty: "暫無結算快照",
            title: "結算快照"
        },
        subtitle: "查看餘額、消耗趨勢與帳本明細。",
        summary: {
            balance: "目前餘額",
            monthConsumed: "本月消耗",
            negativeOnly: "僅統計負向扣減",
            todayConsumed: "今日消耗",
            unitCredits: "單位：積分"
        },
        title: "帳務中心",
        tokenSegment: {
            cached: "快取",
            input: "輸入",
            output: "輸出"
        },
        trend: {
            description: "按時間粒度顯示租戶積分消耗趨勢。",
            empty: "暫無趨勢資料",
            series: {
                consumed: "消耗積分"
            },
            title: "消耗趨勢"
        }
    },
    groupsPage: {
        actions: {
            create: "新增分組",
            deleteGroup: "刪除分組",
            deletePolicy: "刪除策略",
            saveGroup: "儲存分組",
            savePolicy: "儲存模型策略"
        },
        columns: {
            actions: "操作",
            apiKeysCount: "API Key {{count}} 個",
            modelsCount: "模型 {{count}} 個",
            multipliers: "倍率",
            name: "分組",
            status: "狀態",
            usage: "使用情況"
        },
        editor: {
            createTitle: "新增分組",
            description: "設定分組倍率與模型級價格覆寫。",
            editTitle: "編輯分組"
        },
        empty: "暫無分組",
        form: {
            allowAllModels: "允許全部目錄模型",
            cachedInputMultiplier: "快取輸入倍率（ppm）",
            default: "預設分組",
            description: "描述",
            enabled: "啟用",
            inputMultiplier: "輸入倍率（ppm）",
            name: "分組名稱",
            outputMultiplier: "輸出倍率（ppm）"
        },
        messages: {
            groupDeleted: "分組已刪除。",
            groupDeleteFailed: "刪除分組失敗。",
            groupSaved: "分組已儲存：{{name}}",
            groupSaveFailed: "儲存分組失敗。",
            policyDeleted: "模型策略已刪除。",
            policyDeleteFailed: "刪除模型策略失敗。",
            policySaved: "模型策略已儲存。",
            policySaveFailed: "儲存模型策略失敗。"
        },
        policy: {
            cachedInputAbsolutePrice: "快取輸入絕對價格",
            cachedInputMultiplier: "快取輸入倍率（ppm）",
            description: "從統一模型目錄中選擇模型，然後設定倍率或絕對價格。",
            enabled: "啟用策略",
            inputAbsolutePrice: "輸入絕對價格",
            inputMultiplier: "輸入倍率（ppm）",
            model: "模型",
            outputAbsolutePrice: "輸出絕對價格",
            outputMultiplier: "輸出倍率（ppm）",
            title: "模型策略"
        },
        preview: {
            columns: {
                finalPrice: "最終價格",
                formulaPrice: "公式價格",
                mode: "模式",
                model: "模型"
            },
            description: "展示目前分組下對租戶可見的最終價格。",
            moreHidden: "還有 {{count}} 個模型已折疊",
            mode: {
                absolute: "絕對價覆寫",
                formula: "倍率公式"
            },
            title: "有效模型預覽"
        },
        searchPlaceholder: "依名稱、描述或狀態搜尋分組",
        status: {
            default: "預設",
            deleted: "已刪除",
            disabled: "已停用",
            enabled: "已啟用"
        },
        subtitle: "管理 API Key 分組、模型白名單、倍率與分組級絕對定價。",
        title: "分組管理"
    },
    aiRoutingPage: {
        title: "AI 調度",
        subtitle: "設定路由畫像、模型感知回退鏈，以及為 AI 智慧調度提供能力的規劃模型鏈。",
        actions: {
            refresh: "重新整理",
            createProfile: "新增畫像",
            createPolicy: "新增策略",
            edit: "編輯",
            delete: "刪除",
            deleteProfile: "刪除畫像",
            deletePolicy: "刪除策略",
            saveSettings: "儲存設定",
            saveProfile: "儲存畫像",
            savePolicy: "儲存策略"
        },
        settings: {
            title: "規劃器設定",
            description: "控制 AI 調度是否啟用、計畫如何發布，以及哪些小模型可以作為 AI 規劃鏈。",
            enabled: "啟用 AI 調度",
            enabledHint: "關閉後，編譯路由計畫只會保留人工設定路徑。",
            autoPublish: "自動發布",
            autoPublishHint: "每次重算完成後自動發布最新路由計畫。",
            killSwitch: "全域熔斷",
            killSwitchHint: "立即停止 AI 驅動的變更，但保留已儲存的設定。",
            triggerMode: "觸發模式",
            plannerModelChain: "規劃模型鏈",
            plannerModelChainPlaceholder: "gpt-5.2-codex, gpt-4.1-mini",
            plannerModelChainHint: "使用逗號或換行排列模型，順序由高優先級到低優先級。",
            updatedAt: "更新時間 {{value}}"
        },
        versions: {
            title: "已發布版本",
            description: "最近編譯完成、可供 data-plane 使用的路由計畫。",
            empty: "尚未發布任何路由計畫版本。",
            noReason: "未記錄發布原因。",
            defaultSegments: "預設分段 {{count}}",
            policyCount: "策略 {{count}} 條"
        },
        profiles: {
            title: "路由畫像",
            description: "可重用的帳號選擇器，用來描述哪些 plan、模式與驗證方式可以承接請求。",
            empty: "尚無路由畫像。",
            summary: "方案 {{plans}} 項 · 模式 {{modes}} 項 · 驗證 {{authProviders}} 項 · 強制包含 {{include}} 項 · 強制排除 {{exclude}} 項",
            anyMode: "全部帳號模式"
        },
        policies: {
            title: "模型策略",
            description: "將模型族或精確模型 ID 對應到一條明確的畫像回退鏈。",
            empty: "尚無模型路由策略。",
            summary: "精確模型 {{exact}} 個 · 前綴 {{prefixes}} 個 · 回退畫像 {{fallbacks}} 個",
            fallbackChain: "回退鏈：{{value}}",
            moreExactModels: "還有 {{count}} 個"
        },
        dialogs: {
            createProfile: "新增路由畫像",
            editProfile: "編輯路由畫像",
            profileDescription: "為一類上游帳號組合可重用的篩選條件。",
            createPolicy: "新增模型策略",
            editPolicy: "編輯模型策略",
            policyDescription: "定義模型族如何映射到有序的路由畫像鏈。"
        },
        form: {
            name: "名稱",
            description: "描述",
            priority: "優先級",
            enabled: "啟用",
            enabledHint: "只有啟用的畫像才會進入編譯後的路由計畫。",
            policyEnabledHint: "停用後會保留策略，但不會參與實際調度。",
            planTypes: "方案類型",
            planTypesPlaceholder: "free, plus, team",
            modes: "帳號模式",
            authProviders: "驗證方式",
            includeAccounts: "強制包含帳號 ID",
            includeAccountsPlaceholder: "uuid-1, uuid-2",
            excludeAccounts: "強制排除帳號 ID",
            excludeAccountsPlaceholder: "uuid-3, uuid-4",
            family: "模型族",
            familyPlaceholder: "gpt-5",
            exactModels: "精確模型",
            exactModelsPlaceholder: "gpt-5.4, gpt-5.2-codex",
            modelPrefixes: "模型前綴",
            modelPrefixesPlaceholder: "gpt-5, o3",
            fallbackProfiles: "回退畫像",
            noProfilesAvailable: "請先建立路由畫像，再為策略設定回退鏈。"
        },
        messages: {
            settingsSaved: "AI 調度設定已儲存。",
            settingsSaveFailed: "儲存 AI 調度設定失敗。",
            profileSaved: "路由畫像已儲存：{{name}}",
            profileSaveFailed: "儲存路由畫像失敗。",
            profileDeleted: "路由畫像已刪除。",
            profileDeleteFailed: "刪除路由畫像失敗。",
            policySaved: "模型路由策略已儲存：{{name}}",
            policySaveFailed: "儲存模型路由策略失敗。",
            policyDeleted: "模型路由策略已刪除。",
            policyDeleteFailed: "刪除模型路由策略失敗。"
        },
        status: {
            enabled: "已啟用",
            disabled: "已停用",
            killSwitchOn: "熔斷已開啟",
            autoPublishOn: "自動發布已開啟",
            autoPublishOff: "自動發布已關閉"
        },
        triggerModes: {
            hybrid: "混合觸發",
            scheduledOnly: "僅排程",
            eventOnly: "僅事件"
        },
        modes: {
            apiKey: "API Key",
            chatGptSession: "ChatGPT 工作階段",
            codexOauth: "Codex OAuth"
        },
        authProviders: {
            legacyBearer: "傳統 Bearer",
            oauthRefreshToken: "OAuth Refresh Token"
        },
        common: {
            none: "無",
            deletedProfile: "已刪除畫像",
            priority: "優先級 {{value}}"
        }
    },
    tenantDashboard: {
        actions: {
            manageApiKeys: "管理 API 金鑰",
            refresh: "刷新",
            viewBilling: "檢視帳單",
            viewRequestLogs: "查看請求日誌"
        },
        kpi: {
            avgFirstTokenSpeed: "平均首字速度",
            avgFirstTokenSpeedDesc: "TTFT（流式精確 / 非流式近似）",
            rpm: "RPM",
            rpmDesc: "每分鐘請求數",
            totalRequests: "總請求數",
            totalRequestsDesc: "所選時間範圍",
            totalTokens: "Token 消耗總量",
            totalTokensDesc: "輸入 + 快取 + 輸出 + 推理",
            tpm: "TPM",
            tpmDesc: "每分鐘 Token 數"
        },
        cards: {
            activeKeys: {
                description: "注意：僅計算有請求的鍵",
                title: "活躍 API 金鑰計數（選定期間）"
            },
            availableKeys: {
                description: "基於租戶密鑰管理設置",
                title: "目前可用的 API 金鑰"
            },
            keyEnableRate: {
                description: "啟用比例：{{rate}}%（{{enabled}} / {{total}} 個金鑰）",
                title: "金鑰啟用比例"
            },
            peakHour: {
                description: "請求量最高的時段",
                empty: "暫無資料",
                title: "尖峰時段"
            },
            requestVelocity: {
                description: "所選時間範圍內每小時平均請求數",
                title: "請求速率（每小時）"
            },
            totalRequests: {
                scopeAllKeys: " / 全部金鑰",
                scopePrefix: "範圍：目前租戶",
                scopeSingleKey: " / 單一金鑰",
                title: "租用戶 API 金鑰請求總數（所選期間）"
            }
        },
        filters: {
            apiKeyAll: "所有 API 金鑰",
            apiKeyAriaLabel: "API 金鑰篩選",
            apiKeyHint: "提示：使用 API 金鑰篩選可快速定位熱點。",
            range: {
                last24Hours: "過去 24 小時",
                last30Days: "過去 30 天",
                last7Days: "過去 7 天"
            },
            rangeAriaLabel: "時間範圍"
        },
        groupOverview: {
            allDescription: "查看目前 API Key 在各個計價分組中的分布情況。",
            empty: "暫無可展示的 API Key 分組。",
            invalid: "失效",
            keysBound: "綁定了 {{count}} 個 API Key",
            singleDescription: "查看目前 API Key 的分組綁定與有效性狀態。",
            title: "API Key 分組概覽",
            valid: "有效"
        },
        hero: {
            badge: "租戶工作區總覽",
            summaryAllApiKeys: "（所有 API 金鑰）",
            summaryPrefix: "範圍：目前租戶",
            summarySingleApiKey: "（單一 API 金鑰）"
        },
        subtitle: {
            allApiKeys: "（所有 API 金鑰）",
            metricsFocus: "關注指標：TPM、RPM、Token 消耗總量、總請求數與首字速度。",
            scopePrefix: "範圍：目前租戶",
            singleApiKey: "（單一 API 金鑰）",
            timeWindow: "，時間窗口："
        },
        modelDistribution: {
            description: "依請求數或 Token 用量查看模型 Top 分布。",
            empty: "暫無模型分布資料",
            modeRequests: "依請求數",
            modeTokens: "依 Token",
            other: "其他",
            title: "模型請求分布"
        },
        tokenComponents: {
            cached: "快取輸入",
            input: "輸入",
            output: "輸出",
            reasoning: "推理"
        },
        tokenSummary: {
            title: "Token 元件彙總"
        },
        tokenTrend: {
            description: "按小時展示 Token 元件趨勢，可透過開關聚焦消耗來源。",
            empty: "暫無 Token 趨勢資料",
            title: "Token 使用趨勢"
        },
        topKeys: {
            description: "依所選期間的請求量排序",
            empty: "目前沒有 API 金鑰使用排行",
            requests: "{{count}} 次請求",
            share: "占比 {{percent}}%",
            title: "Top API 金鑰",
            unknownKey: "未命名金鑰"
        },
        title: "租戶儀表板",
        trend: {
            description: "範圍：租戶 API 金鑰請求量（每小時粒度）",
            empty: "還沒有請求數據",
            series: {
                requests: "請求數"
            },
            title: "請求趨勢"
        }
    },
    tenantLogs: {
        audit: {
            actionValues: {
                adminOperation: "管理操作",
                authOperation: "驗證操作",
                requestOperation: "請求操作",
                tenantOperation: "租戶操作",
                unknown: "未知操作"
            },
            actorTypes: {
                adminUser: "管理員使用者",
                apiKey: "API 金鑰",
                system: "系統",
                tenantUser: "租戶使用者",
                unknown: "未知操作者"
            },
            columns: {
                action: "操作",
                actor: "操作者",
                reason: "細節",
                result: "結果",
                target: "目標",
                time: "時間"
            },
            description: "定義：控制平面審計事件（僅限目前租戶）",
            empty: "無審計日誌數據",
            filters: {
                actionPlaceholder: "操作",
                actorIdPlaceholder: "操作者 ID",
                actorTypePlaceholder: "操作者類型",
                keywordPlaceholder: "關鍵字（原因/有效負載）",
                resultStatusPlaceholder: "結果狀態",
                actionAriaLabel: "操作篩選",
                actorIdAriaLabel: "操作者 ID 篩選",
                actorTypeAriaLabel: "操作者類型篩選",
                keywordAriaLabel: "關鍵字篩選",
                rangeAriaLabel: "時間範圍",
                resultStatusAriaLabel: "結果狀態篩選"
            },
            resultStatuses: {
                denied: "已拒絕",
                failed: "失敗",
                ok: "成功",
                unknown: "未知結果"
            },
            title: "審計日誌"
        },
        filters: {
            range: {
                last24Hours: "過去 24 小時",
                last30Days: "過去 30 天",
                last7Days: "過去 7 天"
            }
        },
        request: {
            columns: {
                apiKey: "API 金鑰",
                error: "錯誤",
                latency: "延遲（毫秒）",
                path: "路徑",
                requestId: "請求 ID",
                status: "狀態",
                time: "時間"
            },
            description: "定義：資料平面原始請求事件（僅限目前租戶）",
            empty: "無日誌數據",
            filters: {
                apiKeyIdPlaceholder: "API 金鑰 ID",
                keywordPlaceholder: "關鍵字（路徑/錯誤/模型）",
                requestIdPlaceholder: "請求 ID",
                statusCodePlaceholder: "狀態代碼（例如 429）",
                apiKeyAriaLabel: "API 金鑰篩選",
                keywordAriaLabel: "關鍵字篩選",
                rangeAriaLabel: "時間範圍",
                requestIdAriaLabel: "請求 ID 篩選",
                statusCodeAriaLabel: "狀態碼篩選"
            },
            title: "請求日誌"
        },
        scope: "範圍：僅限目前租戶",
        tabs: {
            audit: "審計日誌",
            request: "請求日誌"
        },
        title: "日誌"
    },
    tenantUsage: {
        columns: {
            apiKey: "API 金鑰",
            requests: "請求數",
            tenantLabel: "租戶：{{tenantId}}",
            time: "時間"
        },
        filters: {
            apiKeyAll: "全部 API 金鑰",
            apiKeyAriaLabel: "API 金鑰篩選",
            range: {
                last24Hours: "過去 24 小時",
                last30Days: "過去 30 天",
                last7Days: "最後 7 天"
            },
            rangeAriaLabel: "時間範圍"
        },
        hourly: {
            description: "按小時查看請求量變化。",
            empty: "暫無資料",
            title: "每小時請求量"
        },
        leaderboard: {
            description: "依請求量統計目前租戶 API 金鑰。",
            empty: "暫無排行資料",
            title: "API 金鑰排行"
        },
        subtitle: "查看請求趨勢與 API 金鑰排行。",
        title: "用量分析",
        trend: {
            description: "按所選時間範圍彙總請求量。",
            empty: "暫無趨勢資料",
            title: "請求趨勢"
        }
    },
    tenants: {
        create: {
            fields: {
                expiresAt: "到期時間",
                name: "租戶名稱",
                plan: "計劃（積分）",
                status: "狀態（活動/非活動）"
            },
            submit: "建立租戶",
            title: "建立租戶"
        },
        impersonation: {
            copyToken: "複製令牌",
            create: "建立模擬",
            fields: {
                reason: "原因（必填）"
            },
            revoke: "撤銷會話",
            sessionIdLabel: "工作階段 ID：",
            tokenLabel: "權杖：",
            title: "管理員模擬"
        },
        keys: {
            create: {
                fields: {
                    name: "金鑰名稱",
                    namePlaceholder: "例如管理主鍵"
                },
                submit: "建立密鑰",
                title: "建立 API 金鑰"
            },
            created: {
                copyPlaintext: "複製明文金鑰",
                notice: "明文金鑰僅顯示一次。立即儲存。"
            },
            list: {
                caption: "租戶 API 金鑰列表",
                columns: {
                    actions: "操作",
                    createdAt: "創建於",
                    name: "名稱",
                    prefix: "前綴",
                    status: "狀態"
                },
                copyPrefix: "複製鍵前綴",
                disable: "停用",
                empty: "該租戶沒有 API 金鑰",
                enable: "啟用",
                status: {
                    active: "生效",
                    revoked: "已撤銷"
                },
                title: "API 金鑰列表"
            }
        },
        list: {
            caption: "租戶池列表",
            columns: {
                actions: "操作",
                apiKeys: "API 金鑰",
                expiresAt: "到期時間",
                plan: "計劃",
                status: "狀態",
                tenant: "租戶",
                tenantId: "租戶 ID",
                updatedAt: "更新於"
            },
            planValues: {
                credit: "積分方案",
                unknown: "自訂（{{value}}）"
            },
            statusValues: {
                active: "啟用",
                inactive: "停用",
                unknown: "未知（{{value}}）"
            },
            defaultBadge: "預設",
            empty: "無租戶數據",
            openProfile: "開啟租戶資料",
            searchPlaceholder: "依租戶名稱或 ID 搜尋",
            title: "租戶池"
        },
        messages: {
            apiKeyCreateFailed: "無法建立 API 金鑰",
            apiKeyCreateSuccess: "已為租用戶 {{tenantName}} 建立 API 金鑰：{{keyName}}",
            apiKeyNameRequired: "請輸入 API 金鑰名稱",
            apiKeyToggleFailed: "無法更新 API 金鑰狀態",
            createFailed: "建立租戶失敗",
            createSuccess: "建立租戶：{{name}} ({{id}})",
            impersonationCreateFailed: "建立模擬失敗",
            impersonationCreated: "建立模擬會話（返回令牌）",
            impersonationRevokeFailed: "撤銷冒充失敗",
            impersonationRevoked: "模擬會話已撤銷",
            rechargeFailed: "租戶儲值失敗",
            rechargeSuccess: "儲值成功：+{{amount}}，目前餘額{{balance}}",
            updateFailed: "更新租戶失敗",
            updateSuccess: "租戶更新：{{name}}"
        },
        profile: {
            dialogDescription: "在同一個彈窗分頁中管理租戶資料、API 金鑰與用量。",
            dialogTitle: "租戶簡介",
            dialogTitleWithName: "租戶資料 · {{name}}",
            fields: {
                expiresAt: "到期時間",
                plan: "計劃",
                status: "狀態"
            },
            meta: {
                createdAt: "創建於",
                tenantId: "租戶 ID",
                updatedAt: "更新於"
            },
            save: "儲存個人資料",
            section: {
                title: "租戶簡介"
            },
            tabs: {
                ariaLabel: "租戶資料選項卡",
                keys: "API 金鑰",
                profile: "資料",
                usage: "用量"
            }
        },
        recharge: {
            fields: {
                amount: "微積分（整數）",
                reason: "原因"
            },
            submit: "申請儲值",
            title: "租戶儲值"
        },
        subtitle: "檢查租戶可用性並管理設定檔、API 金鑰和使用情況。",
        title: "租戶",
        usage: {
            filter: {
                allKeys: "所有 API 金鑰",
                currentView: "目前視圖",
                label: "API 金鑰過濾器",
                noKeys: "目前租戶沒有 API 金鑰",
                noMatches: "沒有匹配的 API 金鑰",
                placeholder: "搜尋名稱/前綴/key_id"
            },
            meta: {
                tenantId: "租戶 ID"
            },
            metrics: {
                accountRequests: "帳號請求",
                activeAccounts: "活躍帳號",
                activeApiKeys: "活動 API 金鑰",
                apiKeyRequests: "API 金鑰請求",
                tenantApiKeyRequests: "租戶 API 金鑰請求"
            },
            sectionTitle: "過去 24 小時內的使用情況",
            status: {
                error: "無法載入使用數據",
                loading: "正在載入使用資料…"
            }
        }
    },
    theme: {
        aurora: "極光",
        colorful: "彩色",
        dark: "深色",
        light: "淺色"
    },
    usage: {
        actions: {
            export: "匯出報表",
            filters: "進階篩選"
        },
        chart: {
            empty: "此時段內無數據記錄。",
            requests: "請求次數",
            subtitle: "跨所有上游服務商的數據彙總",
            title: "近30日 Token 消耗"
        },
        subtitle: "閘道請求消耗與基礎設施呼叫量聚合統計",
        title: "用量分析",
        topKeys: {
            columns: {
                apiKey: "金鑰 ID",
                name: "租戶 / 金鑰",
                requests: "請求量",
                share: "占比",
                tenant: "租戶"
            },
            empty: "暫無用量記錄。",
            keyFallback: "API 金鑰 {{keyId}}",
            searchPlaceholder: "搜尋 API 金鑰或租戶…",
            subtitle: "按請求呼叫量統計",
            title: "API 金鑰排行",
            reqs: "次"
        }
    },
    cleanup: {
        title: "憑證清理機制",
        subtitle: "自動化的帳號治理與生命週期策略",
        save: "儲存策略",
        policy: {
            title: "自動治理策略",
            desc: "當 refresh_token 重複使用、遭撤銷或長期失效時，可透過此策略自動隔離並降低污染擴散。",
            refreshEnabled: "啟用 OAuth 自動刷新",
            refreshEnabledDesc: "關閉後帳號將不會自動續簽 access_token。",
            intervalSec: "刷新間隔（秒）",
            notes: "策略備註"
        },
        workspace: {
            title: "OAuth 帳號治理工作台",
            desc: "依帳號檢視登入刷新狀態，支援立即刷新與暫停/恢復同組帳號。",
            searchPlaceholder: "搜尋帳號名稱 / 帳號 ID",
            onlyDisabled: "只看已停用帳號",
            loadingAccounts: "載入帳號中…",
            noAccounts: "沒有符合條件的 OAuth 帳號。",
            enabled: "已啟用",
            disabled: "已停用",
            selectHint: "請先在左側選擇帳號查看狀態。",
            loadingStatus: "載入 OAuth 狀態中…",
            noStatus: "目前帳號暫無 OAuth 狀態資料。",
            refreshNow: "立即刷新",
            disableFamily: "暫停同組帳號",
            enableFamily: "恢復同組帳號",
            status: {
                never: "未刷新",
                ok: "正常",
                failed: "失敗"
            },
            fields: {
                refreshStatus: "刷新狀態",
                reuseDetected: "偵測到重複刷新",
                groupId: "帳號組 ID",
                tokenVersion: "權杖版本",
                expiresAt: "權杖到期時間",
                errorCode: "錯誤碼",
                errorMessage: "錯誤詳情"
            }
        },
        quarantine: {
            title: "自動隔離策略 (Quarantine)",
            desc: "發生未授權錯誤時自動隔離底層帳號",
            threshold: "錯誤閾值",
            thresholdDesc: "觸發隔離前允許的連續 401/403 錯誤次數",
            action: "刷新權杖撤銷操作",
            actionDesc: "當基礎 refresh_token 被判定為失效時",
            options: {
                family: "隔離整個帳號族群",
                disable: "僅停用當前項目",
                nothing: "不作處理"
            }
        },
        purge: {
            title: "自動清除策略 (Purge)",
            desc: "永久清除死亡憑證以節省資料庫空間",
            retention: "保留期限",
            retentionDesc: "在執行清除前保留死亡帳號的天數"
        }
    },
    apiKeys: {
        title: "API 金鑰",
        subtitle: "為客戶端應用程式簽發和管理安全存取憑據。",
        create: "建立金鑰",
        search: "搜尋金鑰名稱或前綴…",
        loading: "載入憑據中…",
        empty: "未找到符合條件的 API 金鑰。",
        columns: {
            name: "應用名稱",
            tenant: "租戶 ID",
            key: "API 金鑰",
            status: "狀態",
            issued: "簽發時間",
            actions: "操作"
        },
        status: {
            active: "活躍",
            revoked: "已撤銷"
        },
        defaultTenant: "預設租戶",
        filters: {
            label: "狀態篩選",
            all: "全部金鑰",
            active: "活躍",
            revoked: "已撤銷"
        },
        actions: {
            copyPrefixTitle: "複製前綴",
            menu: "金鑰操作",
            copyPrefix: "複製 key 前綴",
            processing: "處理中…",
            disable: "停用此金鑰",
            enable: "重新啟用"
        },
        messages: {
            createFailed: "建立 API 金鑰失敗",
            missingName: "請先填寫 Key 名稱"
        },
        dialog: {
            create: {
                title: "建立 API Key",
                desc: "為租戶建立可呼叫 Data Plane 的存取金鑰。建立後只會顯示一次明文 key。",
                nameLabel: "Key 名稱",
                namePlaceholder: "例如：prod-codex-clients",
                tenantLabel: "租戶名稱（可選）",
                tenantPlaceholder: "留空則使用 default",
                confirm: "建立",
                creating: "建立中…"
            },
            created: {
                title: "新金鑰已建立",
                desc: "明文金鑰只會顯示一次，請立即複製儲存。",
                securityTip: "安全提醒：關閉此視窗後將無法再次查看明文 key。",
                nameLabel: "Key 名稱",
                plaintextLabel: "明文金鑰",
                close: "關閉",
                copyPlaintext: "複製明文金鑰"
            }
        }
    }
}
