export default {
    accounts: {
        actions: {
            add: "添加账号",
            apiKeyNoGroupAction: "API 密钥账号不支持同组操作",
            batchDelete: "批量删除",
            batchDeleteConfirm: "确认删除已选 {{count}} 个账号吗？",
            batchDisable: "批量禁用",
            batchEnable: "批量启用",
            batchMenu: "批量操作",
            batchPauseFamily: "批量暂停同家族（{{count}}）",
            batchRefreshLogin: "批量刷新登录（{{count}}）",
            batchResumeFamily: "批量恢复同家族（{{count}}）",
            comfortableMode: "舒适模式",
            compactMode: "紧凑模式",
            delete: "删除账号",
            deleteConfirm: "确认删除账号 {{label}}？",
            disableAccount: "禁用账号",
            enableAccount: "启用账号",
            export: "导出 CSV",
            filter: "筛选列表",
            oneTimeNoGroupAction: "一次性会话账号不支持同组操作",
            pauseGroup: "暂停同组账号",
            refreshAccounts: "刷新账号",
            refreshLogin: "立即刷新登录",
            refreshingAccounts: "刷新账号",
            resumeGroup: "恢复同组账号",
            selectAll: "全选当前筛选结果",
            selectOne: "选择账号 {{label}}",
            selectedCount: "已选 {{count}} 条",
            viewDetails: "查看详情",
            edit: "编辑属性",
            refresh: "强制刷新",
            suspend: "挂起暂停",
            exportSuccess: "导出成功",
            refreshTriggered: "已触发账号刷新"
        },
        columns: {
            actions: "操作",
            added: "添加时间",
            credentialType: "凭据类型",
            health: "健康状态",
            id: "账号 ID",
            loginStatus: "登录状态",
            nextRefresh: "下次刷新",
            plan: "套餐",
            provider: "账号类型",
            rateLimit: "Rate Limit 使用",
            binding: "绑定账号 ID",
            unbound: "未绑定"
        },
        details: {
            description: "查看账号资料、OAuth 状态、限额与原始数据。",
            officialDescription: "这里展示 OpenAI 官方模型信息，只读。下方可以编辑手工价格覆盖。",
            limitsTitle: "限额详情",
            noOauthStatus: "暂无 OAuth 状态数据",
            oauthNotApplicable: "该账号类型不支持 OAuth 详情",
            oauthTitle: "OAuth 状态",
            profileTitle: "账号资料",
            rawTitle: "原始数据",
            tabAria: "账号详情标签页",
            tabs: {
                limits: "限额",
                oauth: "OAuth",
                profile: "资料",
                raw: "原始"
            },
            fields: {
                label: "标签",
                mode: "账号类型",
                accountId: "账号 ID",
                enabled: "启用状态",
                baseUrl: "基础 URL",
                chatgptAccountId: "ChatGPT 账号 ID",
                priority: "优先级",
                createdAt: "创建时间",
                bearerToken: "Bearer 令牌",
                authProvider: "认证提供方",
                credentialKind: "凭据类型",
                lastRefreshStatus: "最近刷新状态",
                effectiveEnabled: "生效状态",
                chatgptPlanType: "ChatGPT 套餐",
                sourceType: "来源类型",
                tokenFamilyId: "Token 家族 ID",
                tokenVersion: "Token 版本",
                tokenExpiresAt: "Token 过期时间",
                nextRefreshAt: "下次刷新时间",
                lastRefreshAt: "最近刷新时间",
                refreshReusedDetected: "检测到 Refresh 重放",
                lastRefreshErrorCode: "最近刷新错误码",
                lastRefreshError: "最近刷新错误",
                rateLimitsFetchedAt: "限额拉取时间",
                rateLimitsExpiresAt: "限额过期时间",
                rateLimitsLastErrorCode: "限额最近错误码",
                rateLimitsLastError: "限额最近错误",
                rawAccount: "账号原始数据",
                rawOauthStatus: "OAuth 状态原始数据"
            }
        },
        filters: {
            active: "正常",
            all: "全部",
            credential: "凭据类型",
            credentialAll: "全部凭据",
            credentialAt: "AT",
            credentialRt: "RT",
            credentialUnknown: "未知",
            disabled: "已禁用",
            mode: "账号类型",
            modeAll: "全部类型",
            modeApiKey: "API 密钥",
            modeOAuth: "OAuth 会话",
            plan: "套餐筛选",
            planAll: "全部套餐",
            planUnknown: "未上报",
            total: "匹配 {{count}} 条",
            suspended: "已挂起"
        },
        messages: {
            batchAllFailed: "{{action}}失败",
            batchAllSuccess: "{{action}}完成",
            batchPartialFailed: "有 {{failed}} 个操作失败{{error}}",
            batchPartialFailedTitle: "{{action}}部分失败",
            batchSuccessCount: "成功 {{count}} 条",
            batchUnknownError: "批量操作失败",
            deleteFailed: "删除账号失败",
            deleteSuccess: "账号已删除",
            disableFailed: "禁用账号失败",
            disableSuccess: "账号已禁用",
            enableFailed: "启用账号失败",
            enableSuccess: "账号已启用",
            exportSuccess: "导出成功",
            pauseFamilyFailed: "暂停同家族账号失败",
            pauseFamilySuccess: "同家族账号已暂停",
            rateLimitPollingTimeout: "轮询账号刷新任务超时。",
            rateLimitRefreshFailedStatus: "账号刷新任务失败，状态={{status}}",
            rateLimitRefreshFailedSummary: "账号刷新任务失败：{{summary}}",
            refreshFailed: "登录刷新失败",
            refreshJobId: "任务 ID：{{jobId}}",
            refreshJobSummary: "任务 ID：{{jobId}} · {{processed}}/{{total}}",
            refreshListFailed: "账号列表刷新失败",
            refreshListSuccess: "账号列表刷新成功",
            refreshSuccess: "登录刷新成功",
            requestFailed: "请求失败，请稍后重试",
            resumeFamilyFailed: "恢复同家族账号失败",
            resumeFamilySuccess: "同家族账号已恢复",
            toggleUnsupported: "当前后端版本不支持账号启用/禁用接口，请升级 control-plane。",
            refreshTriggered: "已触发账号刷新"
        },
        rateLimitRefreshJobStatus: {
            queued: "排队中",
            running: "运行中",
            completed: "已完成",
            failed: "失败",
            cancelled: "已取消",
            unknown: "未知"
        },
        mode: {
            apiKey: "API 密钥",
            chatgptSession: "ChatGPT OAuth",
            codexOauth: "Codex OAuth",
            unknown: "其他"
        },
        nextRefresh: {
            none: "未安排"
        },
        oauth: {
            kindShort: {
                oneTime: "AT",
                refreshRotatable: "RT",
                unknown: "未知"
            },
            loading: "加载中",
            notApplicable: "-",
            status: {
                failed: "失败",
                never: "未刷新",
                ok: "正常"
            },
            unknownError: "未知错误",
            versionPrefix: "版本 ",
            planPrefix: "套餐：",
            kind: {
                refreshRotatable: "可轮转 Refresh Token 账号",
                oneTime: "一次性 Access Token 账号",
                unknown: "未知凭据类型"
            }
        },
        rateLimits: {
            labels: {
                fiveHours: "5小时限制",
                github: "GitHub",
                oneWeek: "周限制"
            },
            shortLabels: {
                fiveHours: "5h",
                oneWeek: "7d"
            },
            moreDetails: "查看更多（+{{count}}）",
            noReset: "暂无刷新时间",
            remainingPrefix: "剩余",
            resetAt: "{{absolute}}（{{relative}}）重置",
            unavailable: "暂无限额数据",
            usedPrefix: "已用"
        },
        searchPlaceholder: "按标签、账号 ID、URL 搜索…",
        status: {
            active: "正常",
            disabled: "已禁用"
        },
        subtitle: "在这里查看账号是否可用，并管理登录状态",
        syncing: "正在同步账号状态…",
        title: "账号池"
    },
    billing: {
        columns: {
            balanceAfter: "变动后余额",
            billingDetail: "账单详情",
            deductedCredits: "扣除的积分",
            deductionEvents: "扣减事件",
            delta: "积分变动",
            eventType: "事件",
            model: "模型",
            periodDay: "日期",
            periodMonth: "月",
            requestType: "请求类型",
            source: "来源",
            timestamp: "时间"
        },
        exportCsv: "导出 CSV",
        filters: {
            granularityAriaLabel: "计费粒度",
            tenantAriaLabel: "租户筛选",
            tenantPlaceholder: "选择租户"
        },
        granularity: {
            day: "按日",
            month: "按月"
        },
        ledger: {
            codeLabels: {
                accountDeactivated: "账号已停用",
                billingUsageMissing: "缺少使用结算字段",
                failoverExhausted: "重试/故障转移已耗尽",
                noUpstreamAccount: "没有可用的上游账号",
                streamPreludeError: "流前奏错误",
                tokenInvalidated: "令牌失效",
                transportError: "上行网络错误",
                upstreamRequestFailed: "上游请求失败",
                unknown: "未知"
            },
            details: {
                accrued: "应计：{{value}} 积分",
                adjustment: "调整：{{value}} 积分",
                extraCharge: "额外扣费：{{value}} 积分",
                failoverAction: "处理动作：{{action}}",
                failure: "失败：{{summary}}",
                failurePrefix: "失败：",
                source: "来源：{{source}}",
                upstreamStatus: "上游 {{status}}",
                tokenSettlement: "代币结算：输入{{input}} + 缓存{{cached}} + 输出{{output}}",
                unitPrice: "单价：{{prices}} 积分/1M 代币"
            },
            empty: "暂无账本流水。",
            failoverActions: {
                crossAccountFailover: "跨账号故障转移",
                retrySameAccount: "重试同一账号",
                returnFailure: "返回失败",
                unknown: "未知"
            },
            releaseReasons: {
                billingSettleFailed: "账单结算失败",
                failoverExhausted: "重试/故障转移已耗尽",
                invalidUpstreamUrl: "上游 URL 配置无效",
                noUpstreamAccount: "没有可用的上游账号",
                streamPreludeError: "流前奏错误",
                streamUsageMissing: "流使用缺失",
                transportError: "上行网络错误",
                upstreamRequestFailed: "上游请求失败",
                unknown: "未知"
            },
            showRaw: "显示原始条目",
            subtitle: "按当前租户过滤。",
            title: "账本流水",
            requestTypes: {
                nonStream: "非流式",
                stream: "流式",
                unknown: "-"
            },
            tokenSegments: {
                cached: "缓存",
                input: "输入",
                output: "输出"
            }
        },
        messages: {
            rechargeFailedTitle: "充值失败",
            rechargeSuccessDetail: "+{{amount}}，余额 {{balance}}",
            rechargeSuccessTitle: "充值成功",
            retryLater: "请稍后重试"
        },
        recharge: {
            amountAriaLabel: "充值金额",
            amountPlaceholder: "充值积分（微积分）",
            reasonAriaLabel: "充值原因",
            reasonPlaceholder: "充值原因",
            submit: "执行充值",
            subtitle: "为当前选择的租户充值。",
            title: "管理员充值"
        },
        snapshot: {
            empty: "还没有结算快照。",
            subtitle: "按 {{granularity}} 汇总扣除事件，用于月末结算和对账。",
            title: "结算快照"
        },
        subtitle: "主要视图：信用分类帐（实际费用），具有租户级管理过滤。",
        summary: {
            currentBalance: "当前余额",
            deductionHint: "仅计算负账本扣除事件。",
            monthConsumed: "本月消费",
            todayConsumed: "今日消费",
            unitCredits: "单位：积分"
        },
        title: "计费中心",
        trend: {
            noData: "尚无趋势数据。",
            seriesConsumed: "消耗的积分",
            subtitle: "显示按 {{granularity}} 汇总的分类帐扣除额。",
            title: "消费趋势"
        }
    },
    common: {
        cancel: "取消",
        close: "关闭",
        collapseSidebar: "收起侧边栏",
        confirm: "确认",
        delete: "删除",
        edit: "编辑",
        expand: "展开",
        collapse: "收起",
        expandSidebar: "展开侧边栏",
        loading: "加载中…",
        logout: "退出登录",
        no: "否",
        noData: "暂无数据",
        openMenu: "打开菜单",
        refresh: "刷新",
        skipToMainContent: "跳至主要内容",
        table: {
            firstPage: "第一页",
            go: "跳转",
            jumpToPage: "跳转页码",
            lastPage: "最后一页",
            nextPage: "下一页",
            pageOf: "第 {{page}} / {{total}} 页",
            previousPage: "上一页",
            range: "{{start}}-{{end}} / 共 {{total}} 条",
            rowsPerPage: "每页条数",
            searchLabel: "搜索表格",
            searchPlaceholder: "搜索当前列表…"
        },
        toggleLanguage: "切换语言",
        toggleTheme: "切换主题",
        yes: "是",
        save: "保存",
        search: "搜索…",
        routeLoading: "页面加载中…",
        status: {
            healthy: "健康",
            degraded: "降级",
            offline: "离线",
            disabled: "禁用",
            available: "可用"
        }
    },
    config: {
        controlPlane: {
            authValidateUrl: "鉴权校验地址",
            dataPlaneUrl: "转发服务地址",
            desc: "设置控制面与转发服务之间的连接地址",
            listen: "控制面监听地址（只读）",
            title: "控制面连接设置"
        },
        refreshSettings: {
            desc: "控制登录凭据自动刷新的开关与频率",
            enableDesc: "关闭后将不再自动更新即将过期的访问凭据。",
            enableLabel: "启用自动刷新",
            intervalSec: "刷新间隔（秒）",
            notes: "备注",
            title: "自动刷新设置"
        },
        runtimeHint: {
            desc: "修改会立即生效。服务重启后，仍以环境变量和 config.toml 为准。",
            title: "运行时配置提示"
        },
        save: "保存更改",
        subtitle: "运行时的系统设置与全局变量",
        success: "配置保存成功。",
        title: "全局配置",
        reload: {
            title: "支持热重载 (Runtime Reload)",
            desc: "对缓存和超时等参数的修改将被全局同步，并立即生效，无需重启服务。"
        },
        system: {
            title: "系统策略",
            desc: "配置全局操作上下文。",
            maintenance: "维护模式",
            maintenanceDesc: "全局拒绝所有新请求并返回 503 状态码。",
            logRetention: "日志保留 (天)",
            logRetentionDesc: "ClickHouse 追踪存储的滚动窗口期。"
        },
        network: {
            title: "网络画像控制",
            desc: "定义全局速率限制以防止上游 API 封禁。",
            tpm: "全局每分钟 Token 数 (TPM)",
            tpmDesc: "允许发送到 AI 端点的最大并发请求压力。",
            scaling: "云端资源自动扩缩",
            scalingDesc: "允许动态分配和回收底层节点资源。"
        }
    },
    dashboard: {
        actions: {
            viewBilling: "查看账单",
            viewLogs: "查看请求日志"
        },
        alerts: {
            checkRoutes: "检查路由",
            columns: {
                action: "建议操作",
                message: "告警内容",
                severity: "级别",
                source: "来源",
                status: "状态",
                time: "时间"
            },
            empty: "系统运行稳定",
            resolve: "标为解决",
            searchPlaceholder: "搜索告警内容…",
            subtitle: "需要干预处置的系统隐患",
            title: "待办告警",
            usageRepoUnavailable: "用量统计存储当前不可用",
            dataPlaneDisconnected: "数据平面连接中断",
            now: "刚刚",
            severity: {
                critical: "严重",
                warning: "警告",
                info: "信息"
            },
            source: {
                data_plane: "数据平面",
                usage_repo: "用量仓库"
            },
            status: {
                open: "待处理",
                resolved: "已解决"
            }
        },
        currentScope: "当前：{{scope}}",
        filters: {
            apiKeyAriaLabel: "API 密钥筛选",
            apiKeyPlaceholder: "选择 API 密钥",
            range: {
                last24Hours: "过去 24 小时",
                last30Days: "过去 30 天",
                last7Days: "过去 7 天"
            },
            rangeAriaLabel: "时间范围",
            scopeAriaLabel: "范围筛选",
            tenantAriaLabel: "租户筛选",
            tenantPlaceholder: "选择租户"
        },
        kpi: {
            activeApiKeysInRange: "活动 API 密钥（选定范围）",
            accounts: "账号数",
            accountsDesc: "仅管理员可见的运营指标",
            apiKeys: "API 密钥数",
            apiKeysDesc: "系统已配置密钥数",
            avgFirstTokenSpeed: "平均首字速度",
            avgFirstTokenSpeedDesc: "TTFT（流式精确 / 非流式近似）",
            globalScope: "全球范围",
            rpm: "RPM",
            rpmDesc: "每分钟请求数",
            requests: {
                apiKey: "当前 API 密钥请求（选定范围）",
                global: "账号请求总数（选定范围）",
                tenant: "当前租户 API 密钥请求（选定范围）"
            },
            tenants: "租户数",
            tenantsDesc: "仅管理员可见的运营指标",
            totalRequests: "总请求数",
            totalTokens: "Token 消耗总量",
            totalTokensDesc: "输入 + 缓存 + 输出 + 推理",
            tpm: "TPM",
            tpmDesc: "每分钟 Token 数",
            running: "运行中",
            totalConfigured: "已配置总数",
            uptime: "99.99% 在线率",
            reqs: "总请求量",
            reqsDesc: "环比上月增长 12.5%",
            failed: "异常拦截",
            failedDesc: "今日拦截 180 次重试",
            avgLatency: "平均延时",
            avgLatencyDesc: "比上周降低 5ms",
            activeTokens: "活跃令牌",
            activeTokensDesc: "新增 24 个可用模型",
            oauthLabel: "OAuth"
        },
        scope: {
            apiKey: "API密钥视角",
            global: "全局视角",
            tenant: "租户视角"
        },
        subtitle: "网关全局代理指标视角",
        table: {
            apiKey: "API 密钥",
            requests: "请求数"
        },
        modelDistribution: {
            description: "按请求数或 Token 用量查看模型 Top 分布。",
            empty: "暂无模型分布数据",
            modeRequests: "按请求数",
            modeTokens: "按 Token",
            other: "其他",
            title: "模型请求分布"
        },
        tokenComponents: {
            cached: "缓存输入",
            input: "输入",
            output: "输出",
            reasoning: "推理"
        },
        tokenTrend: {
            description: "按小时展示 Token 组件趋势，可按组件开关聚焦消耗来源。",
            empty: "暂无 Token 趋势数据",
            title: "Token 使用趋势"
        },
        title: "服务总览",
        topApiKeys: {
            empty: "暂无排名数据",
            scopeDescription: "范围：{{scope}} /选定的时间窗口",
            title: "热门 API 密钥"
        },
        trafficChart: {
            scope: {
                apiKey: "范围：当前 API 密钥请求",
                global: "范围：全局账号请求 + 全局租户 API 密钥请求",
                tenant: "范围：当前租户 API 密钥请求"
            },
            series: {
                accountRequests: "账号请求",
                tenantApiKeyRequests: "租户 API 密钥请求",
                tenantApiKeyRequestsSingle: "租户 API 密钥请求"
            },
            title: "全天流量趋势",
            subtitle: "过去 24 小时各时段网关出度",
            success: "成功解析",
            blocked: "流控拦截"
        }
    },
    importJobs: {
        actions: {
            cancel: "取消",
            cancelJob: "取消任务",
            confirmCancelJob: "确定要取消这个导入任务吗？",
            exportFailed: "导出失败项",
            refreshItems: "刷新项",
            remove: "移除",
            retryFailed: "重试失败",
            removeFromList: "从列表移除"
        },
        detail: {
            columns: {
                error: "错误信息",
                label: "标签",
                line: "行号",
                status: "状态"
            },
            filterLabel: "状态筛选",
            itemsEmpty: "没有匹配的任务条目。",
            itemsLoading: "正在加载任务条目…",
            jobIdLabel: "任务 ID：{{jobId}}",
            loadMore: "加载更多",
            loadedCount: "已加载 {{count}} 条记录",
            loadingMore: "加载中",
            retryQuery: "重试查询",
            searchPlaceholderModern: "按 label / email / error 搜索…",
            selectHint: "请选择一个导入任务查看详情。",
            summaryLoading: "正在加载任务概要…",
            title: "任务详情",
            unreadable: "当前任务无法读取（可能已过期或 ID 无效）。",
            searchPlaceholder: "按 label / email / error 搜索"
        },
        dropzone: {
            acceptsNew: "支持单次上传多个 .json/.jsonl 文件，后端会自动汇总为同一个导入任务。",
            creatingTitle: "正在创建导入任务",
            selectFiles: "选择文件",
            titleNew: "拖拽导入文件到这里",
            uploading: "正在上传…",
            wait: "请稍候，文件正在安全传输。",
            title: "点击上传或拖拽文件到此处",
            accepts: "支持 <code>.csv</code> 或换行分隔的 <code>.txt</code> 格式。单文件最大 50,000 行。",
            browse: "浏览文件",
            creatingDesc: "请稍候，任务创建成功后会自动加入右侧队列"
        },
        error: "上传失败",
        manual: {
            add: "添加",
            placeholderModern: "粘贴 job_id…",
            title: "手动追踪任务",
            placeholder: "粘贴 job_id"
        },
        messages: {
            jobNotFound: "任务不存在或无法访问",
            queryFailed: "查询失败",
            unknownError: "未知错误",
            uploadFailedTitle: "导入失败"
        },
        errors: {
            importFailed: "导入失败",
            invalidRecord: "数据记录无效",
            invalidRefreshToken: "refresh_token 无效",
            missingCredentials: "缺少凭据字段",
            oauthProviderNotConfigured: "OAuth 提供方未配置",
            rateLimited: "请求频率受限",
            refreshTokenReused: "refresh_token 已被使用",
            unknown: "未知导入错误",
            upstreamNetworkError: "上游网络错误",
            upstreamUnavailable: "上游服务不可用"
        },
        metrics: {
            created: "新建",
            failed: "失败",
            processed: "已处理",
            status: "状态",
            throughput: "吞吐",
            total: "总数",
            updated: "更新"
        },
        precheck: {
            createdNotice: "导入任务已创建：{{id}}",
            defaultReady: "文件格式与大小检查通过。",
            duplicateBatch: "这些文件已经在待导入列表中。",
            duplicateName: "检测到同名文件，建议确认来源后再导入。",
            emptyPreview: "预检查未发现有效数据行，可能是空文件。",
            firstLineInvalid: "JSONL 首行不是合法 JSON，导入时可能失败。",
            firstLineObject: "首行不是 JSON 对象，导入时可能失败。",
            firstLineValid: "JSONL 首行结构检查通过。",
            jsonEmpty: "JSON 文件内容为空。",
            jsonInvalid: "JSON 文件不是合法 JSON，导入时可能失败。",
            jsonValid: "JSON 结构检查通过。",
            noneImportable: "当前没有可导入文件，请先修复阻塞问题。",
            skipLargeJson: "文件较大，已跳过本地 JSON 解析，导入时由后端校验。",
            status: {
                invalid: "阻塞",
                ready: "可导入",
                warning: "需确认"
            }
        },
        progress: {
            done: "已完成",
            etaLabel: "预计剩余：",
            etaMinutes: "约 {{count}} 分钟",
            jobIdLabel: "任务 ID：{{jobId}}",
            lessThanMinute: "不到 1 分钟",
            noJobSelected: "创建或选择一个导入任务后，这里会显示实时进度。",
            title: "实时导入进度",
            topErrors: "主要错误分布"
        },
        queue: {
            columns: {
                jobId: "任务 ID"
            },
            descRecent: "队列会自动轮询任务状态，点击任意任务查看明细项与错误信息。",
            emptyRecent: "暂无可追踪任务，请先上传文件或手动输入 job_id。",
            titleRecent: "最近导入任务",
            tracked: "本地追踪中",
            title: "最近追踪的任务",
            empty: "当前会话尚未上传任何任务。",
            card: {
                processed: "已处理",
                new: "新增",
                errors: "错误"
            }
        },
        status: {
            all: "全部",
            cancelled: "已取消",
            completed: "已完成",
            created: "新建",
            failed: "失败",
            pending: "待处理",
            processing: "处理中",
            queued: "排队中",
            running: "处理中",
            skipped: "跳过",
            updated: "已更新"
        },
        subtitleModern: "先批量加入文件，预检查后再一键导入，并实时查看每个任务的进度和错误明细。",
        template: {
            downloadJsonl: "下载模板",
            title: "导入模板",
            desc: "下载我们推荐的模板，以确保列名严格符合系统要求。",
            download: "下载 CSV 模板",
            titleNew: "导入模板",
            descNew: "下载 JSONL 模板并填入 refresh_token 后即可批量导入。"
        },
        title: "批量导入任务",
        validation: {
            fileTooLarge: "文件 {{name}} 超过 20MB，请拆分后再导入",
            unsupportedFormat: "文件 {{name}} 格式不支持，仅支持 .json / .jsonl",
            format: "只允许上传 .csv 或 .txt 格式的文件。",
            size: "文件过大，最大限制为 10MB。"
        },
        workspace: {
            clearQueue: "清空列表",
            columns: {
                action: "操作",
                check: "预检查结果",
                file: "文件",
                size: "大小",
                status: "状态"
            },
            confirmClear: "确定清空当前待导入列表吗？",
            desc: "支持拖拽或批量选择文件，先做预检查，再点击“开始导入”。",
            empty: "还没有待导入文件，先拖拽文件到上方区域。",
            invalidFiles: "阻塞 {{count}}",
            invalidHint: "{{count}} 个文件存在阻塞问题，点击导入时会自动排除。",
            moreChecks: "条检查信息",
            readyFiles: "可导入 {{count}}",
            readyHint: "预检查完成，可以直接开始导入。",
            startImportWithCount: "开始导入（{{count}} 个文件）",
            stepCheck: "预检查",
            stepCheckDesc: "检查格式、大小、重复名",
            stepImport: "开始导入",
            stepImportDesc: "创建任务并查看实时进度",
            stepSelect: "添加文件",
            title: "文件导入工作台",
            totalFiles: "文件 {{count}} 个",
            totalSize: "总大小 {{size}}",
            warningFiles: "需确认 {{count}}"
        },
        subtitle: "通过严格格式的 CSV/TXT 文件安全地上传账号凭证。"
    },
    oauthImport: {
        title: "OAuth 登录导入",
        subtitle: "通过 Codex OAuth 登录，并将登录账号直接导入账号池。",
        start: {
            title: "开始 Codex OAuth 登录",
            description: "先创建登录会话，完成 OAuth 授权后自动导入账号。"
        },
        form: {
            label: "账号标签（可选）",
            labelPlaceholder: "留空将根据邮箱或账号 ID 自动生成",
            baseUrl: "基础 URL",
            priority: "优先级",
            enabled: "导入后立即启用账号"
        },
        actions: {
            startLogin: "开始 OAuth 登录",
            reopenAuth: "重新打开 OAuth 窗口",
            goAccounts: "前往账号池",
            submitCallback: "提交回调 URL"
        },
        status: {
            label: "会话状态",
            idle: "未开始",
            waiting_callback: "等待回调",
            exchanging: "正在换取令牌",
            importing: "正在导入账号",
            completed: "已完成",
            failed: "失败",
            expired: "已过期",
            sessionId: "会话 ID：{{id}}",
            callbackUrl: "回调地址：{{url}}",
            expiresAt: "过期时间：{{time}}"
        },
        error: {
            failed: "OAuth 导入失败。"
        },
        result: {
            success: "账号导入成功。",
            accountId: "账号 ID：{{id}}",
            accountLabel: "标签：{{label}}",
            email: "邮箱：{{email}}",
            created: "新建",
            updated: "更新"
        },
        manual: {
            title: "手动回调兜底",
            description: "当自动回调不可达时，可将完整回调 URL 粘贴到这里提交。",
            placeholder: "粘贴包含 code/state 的完整回调 URL…",
            hint: "仅在自动回调失败时使用。"
        },
        notifications: {
            popupBlockedTitle: "弹窗被拦截",
            popupBlockedDescription: "请允许弹窗后重新打开 OAuth 窗口。",
            sessionCreatedTitle: "会话已创建",
            sessionCreatedDescription: "OAuth 窗口已打开，请完成登录授权。",
            sessionCreateFailedTitle: "创建会话失败",
            manualSubmitTitle: "回调已提交",
            manualSubmitSuccess: "手动回调已完成并成功导入账号。",
            manualSubmitAccepted: "手动回调已接收，正在处理中。",
            manualSubmitFailedTitle: "手动回调失败",
            unknownError: "发生未知错误，请稍后重试。"
        }
    },
    login: {
        brand: {
            badge: "控制平面入口",
            points: {
                audit: "所有高风险操作都可通过 request id 全链路追踪。",
                resilience: "高可用路由保障管理操作稳定可达。",
                security: "租户隔离与凭据治理默认开启。"
            },
            subtitle: "为系统管理员提供加固认证入口。",
            title: "以可控与可信方式管理 Codex Pool"
        },
        messages: {
            failed: "登录失败，请检查账号密码",
            invalidCredentials: "账号或密码不正确，请重新输入。",
            sessionExpired: "登录状态已过期，请重新登录。"
        },
        password: "密码",
        passwordPlaceholder: "请输入管理员密码",
        securityHint: "安全提示：连续失败会在审计日志中关联记录。",
        submit: "登录",
        subtitle: "使用管理员账号登录控制台",
        title: "Codex-Pool 管理台",
        username: "用户名",
        usernamePlaceholder: "请输入管理员用户名"
    },
    logs: {
        audit: {
            actionValues: {
                adminOperation: "管理操作",
                authOperation: "认证操作",
                requestOperation: "请求操作",
                tenantOperation: "租户操作",
                unknown: "未知操作"
            },
            actorTypes: {
                adminUser: "管理员用户",
                apiKey: "API 密钥",
                system: "系统",
                tenantUser: "租户用户",
                unknown: "未知操作者"
            },
            columns: {
                action: "操作",
                actor: "操作者",
                createdAt: "时间",
                reason: "原因",
                result: "结果",
                target: "目标",
                tenant: "租户"
            },
            description: "范围：控制平面审计事件（角色/操作/结果/目标/有效负载）。",
            empty: "没有可用的审计日志数据",
            filters: {
                actionPlaceholder: "操作",
                actorIdPlaceholder: "操作者 ID",
                actorTypePlaceholder: "操作者类型",
                keywordPlaceholder: "关键字（原因/有效负载）",
                resultStatusPlaceholder: "结果状态",
                actionAriaLabel: "操作筛选",
                actorIdAriaLabel: "操作者 ID 筛选",
                actorTypeAriaLabel: "操作者类型筛选",
                keywordAriaLabel: "关键字筛选",
                rangeAriaLabel: "时间范围",
                resultStatusAriaLabel: "结果状态筛选",
                tenantAriaLabel: "租户筛选"
            },
            resultStatuses: {
                denied: "已拒绝",
                failed: "失败",
                ok: "成功",
                unknown: "未知结果"
            },
            title: "审计日志"
        },
        columns: {
            level: "级别",
            message: "日志消息",
            service: "服务节点",
            timestamp: "时间戳"
        },
        export: "导出日志",
        filters: {
            allTenants: "所有租户"
        },
        focus: "聚焦级别:",
        levels: {
            all: "所有级别",
            error: "错误",
            info: "信息",
            warn: "告警"
        },
        messages: {
            configUpdated: "已更新运行时配置快照（内存）",
            empty: "无日志详情",
            keyCreated: "已创建 API 密钥 {{keyId}}",
            keyPatched: "已设置 API 密钥 {{keyId}} 启用={{enabled}}",
            modelsLoaded: "已从上游账号 {{label}} 加载模型列表",
            modelsProbed: "模型探测（{{trigger}}）：通过 {{label}} 测试 {{tested}} 个模型（可用 {{available}}，不可用 {{unavailable}}）",
            proxiesTested: "已检测 {{count}} 个代理节点",
            systemState: "已查询系统状态：{{count}} 个账号",
            unmappedAction: "{{action}} · {{message}}"
        },
        range: {
            last24Hours: "过去 24 小时",
            last30Days: "过去 30 天",
            last7Days: "过去 7 天"
        },
        request: {
            columns: {
                apiKey: "API 密钥",
                createdAt: "时间",
                errorCode: "错误",
                latency: "延迟（毫秒）",
                path: "路径",
                requestId: "请求 ID",
                status: "状态",
                tenant: "租户"
            },
            description: "范围：原始数据平面请求事件（状态/延迟/路径/租户/API 密钥/请求 ID）。",
            empty: "没有可用的请求日志数据",
            filters: {
                apiKeyIdPlaceholder: "API 密钥 ID",
                keywordPlaceholder: "关键字（路径/错误/模型）",
                requestIdPlaceholder: "请求 ID",
                statusCodePlaceholder: "状态代码（例如 500）",
                apiKeyAriaLabel: "API 密钥筛选",
                keywordAriaLabel: "关键字筛选",
                rangeAriaLabel: "时间范围",
                requestIdAriaLabel: "请求 ID 筛选",
                statusCodeAriaLabel: "状态码筛选",
                tenantAriaLabel: "租户筛选"
            },
            title: "请求日志"
        },
        search: "搜索载荷或服务名…",
        subtitle: "实时的审计追踪与运行时上下文。",
        tabs: {
            audit: "审计日志",
            request: "请求日志",
            system: "系统日志"
        },
        title: "系统日志",
        waiting: "等待接收新的日志流…",
        actions: {
            systemState: "系统状态查询",
            configUpdate: "配置更新",
            proxiesTest: "节点健康检查",
            keyCreate: "创建 API 密钥",
            keyPatch: "更新 API 密钥",
            modelsList: "拉取模型列表",
            modelsProbe: "模型探测",
            unknown: "未知操作（{{action}}）"
        }
    },
    models: {
        actions: {
            copyModelId: "复制模型名",
            createModel: "创建模型",
            syncOpenAiCatalog: "同步 OpenAI 目录",
            probeAvailability: "探测可用性",
            openDetails: "详情",
            deleteModel: "删除模型",
            deletePricing: "删除定价",
            deleteBillingRule: "删除规则",
            editBillingRule: "编辑规则",
            editModel: "编辑模型",
            probeNow: "手动测试",
            saveModelProfile: "保存模型配置文件",
            savePricing: "保存价格",
            saveBillingRule: "保存规则",
            newBillingRule: "新建规则",
            search: "搜索模型 ID…",
            sync: "状态同步"
        },
        availability: {
            available: "可用",
            issueHint: "查看不可用原因",
            neverChecked: "从未探测",
            noErrorDetail: "无错误详情",
            unavailable: "不可用",
            unknown: "未探测",
            viewIssue: "查看原因"
        },
        cache: {
            fresh: "缓存新鲜",
            stale: "缓存过期"
        },
        catalog: {
            customOnly: "自定义模型",
            hidden: "目录隐藏",
            listed: "目录可见",
            unlisted: "未收录"
        },
        columns: {
            actions: "详情",
            availability: "可用性",
            cachedInputPrice: "缓存输入价格",
            context: "上下文 / 最大输出",
            modalities: "模态",
            syncedAt: "同步时间",
            catalog: "目录状态",
            checkedAt: "最近探测",
            id: "模型名称",
            inputPrice: "输入价格",
            outputPrice: "产出价格",
            pricingStatus: "定价状态",
            provider: "提供商 / Hub"
        },
        description: "在这里查看模型可用性，并管理模型资料与定价。",
        dialog: {
            description: "在此对话框中编辑配置文件和定价。保存的定价将立即写回模型池列表。",
            officialDescription: "这里展示 OpenAI 官方模型信息，只读。下方可以编辑手工价格覆盖。",
            tabListAriaLabel: "模型资料标签页",
            titleWithId: "模型资料 · {{modelId}}"
        },
        empty: "当前未暴露或映射任何模型。",
        emptySyncRequired: "当前还没有官方目录，请先同步 OpenAI 官方目录。",
        emptyActions: {
            goAccounts: "前往账号池",
            importAccount: "导入账号"
        },
        errors: {
            deleteModelEntityFailed: "删除模型实体失败。",
            deleteModelPricingFailed: "无法删除模型定价。",
            openAiCatalogSyncFailed: "同步 OpenAI 官方目录失败。",
            deleteBillingRuleFailed: "删除分段计价规则失败。",
            modelIdRequired: "模型 ID 不能为空。",
            probeFailed: "模型探测失败。",
            saveBillingRuleFailed: "保存分段计价规则失败。",
            saveModelPricingFailed: "无法保存模型定价。",
            saveModelProfileFailed: "无法保存模型配置文件。"
        },
        filters: {
            allProviders: "全部提供商",
            providerLabel: "提供商筛选"
        },
        form: {
            modelId: "模型 ID",
            modelIdLockedHint: "当前模型来自上游同步列表，请在上游来源中改名。",
            modelIdPlaceholder: "示例：gpt-5.3-codex",
            provider: "提供商",
            providerPlaceholder: "示例：openai / 自定义",
            source: "来源",
            sourceValues: {
                entityOnly: "仅实体",
                pricingOnly: "仅定价",
                upstream: "上游"
            },
            visibility: "可见性",
            visibilityPlaceholder: "示例：list / hide"
        },
        hints: {
            cannotDeleteMissingPricing: "当前模型没有本地定价数据，请先补充后再删除。",
            cannotDeleteNonLocalEntity: "当前模型不是本地实体模型，因此无法删除其实体。"
        },
        loadingHint: "正在检测目录与可用性状态，完成后会自动显示最新模型列表。",
        notice: {
            modelEntityDeleted: "模型实体已删除。",
            modelPricingDeleted: "模型定价记录已删除。",
            billingRuleDeleted: "分段计价规则已删除。",
            modelPricingSaved: "已保存模型定价：{{model}}",
            openAiCatalogSynced: "OpenAI 官方目录同步完成：更新 {{count}} 个模型。",
            billingRuleSaved: "分段计价规则已保存：{{model}}",
            modelProfileSaved: "模型资料已保存：{{model}}",
            probeCompleted: "模型探测完成。最新模型池已同步。"
        },
        pricing: {
            cachedInputPrice: "缓存输入价格",
            creditsPerMillionTokens: "积分 / 1M 代币",
            disabled: "禁用",
            enablePricing: "启用定价",
            enabled: "启用",
            inputPrice: "输入价格",
            notConfigured: "未配置",
            outputPrice: "产出价格",
            perMillionTokensMicrocredits: "每 100 万 token，单位微积分",
            sectionTitle: "模型定价",
            effectiveSectionTitle: "实际计费价格",
            manualOverride: "手工覆盖",
            officialBase: "官方基础价",
            overrideSectionTitle: "手工价格覆盖",
            sourceLabels: {
                officialSync: "OpenAI 官方",
                manualOverride: "手工覆盖",
                unknown: "未知"
            },
        },
        rules: {
            cachedInputMultiplierPpm: "缓存输入倍率（ppm)",
            empty: "当前模型还没有配置分段计价规则。",
            enableRule: "启用规则",
            inputMultiplierPpm: "输入倍率（ppm）",
            noThreshold: "无",
            outputMultiplierPpm: "输出倍率（ppm）",
            priorityLabel: "优先级",
            requestKind: "请求类型",
            requestKinds: {
                any: "任意",
                chat: "Chat",
                compact: "Compact",
                response: "Responses",
                unknown: "未知"
            },
            ruleSummary: "阈值 {{threshold}} · 输入 ×{{inputMultiplier}} · 缓存 ×{{cachedMultiplier}} · 输出 ×{{outputMultiplier}}",
            scope: "作用域",
            scopes: {
                request: "单次请求",
                session: "会话",
                unknown: "未知"
            },
            sectionDescription: "为长上下文或特殊计费 band 配置按请求/会话生效的倍率规则。",
            sectionTitle: "分段计价规则",
            thresholdInputTokens: "输入 Token 阈值"
        },
        probeSourceUnknown: "未知账号",
        probeSummary: "探测缓存：{{stale}}，最近 {{checkedAt}}，缓存时长 {{ttlHours}} 小时，来源 {{source}}",
        syncHint: {
            notSynced: "OpenAI 官方目录尚未同步。",
            syncedAt: "目录已于 {{time}} 同步"
        },
        profile: {
            sectionTitle: "模型资料"
        },
        syncing: "正在映射端点可用性…",
        tabs: {
            pricing: "定价",
            profile: "资料"
        },
        title: "模型池",
        subtitle: "这里展示当前账号可用的模型列表",
        detail: {
            title: "模型详情",
            notFound: "未找到该模型，可能已被移除或当前筛选不可见。",
            httpStatus: "HTTP 状态",
            error: "错误详情",
            noError: "无错误详情",
            officialTitle: "官方元数据",
            officialDescription: "这里展示 OpenAI 官方模型信息，只读。下方可以编辑手工价格覆盖。",
            contextWindow: "上下文窗口",
            maxOutputTokens: "最大输出 Token",
            knowledgeCutoff: "知识截止日期",
            reasoningTokenSupport: "推理 Token 支持",
            sourceUrl: "来源链接",
            openOfficialPage: "打开官方页面",
            capabilitiesTitle: "能力信息",
            inputModalities: "输入模态",
            outputModalities: "输出模态",
            endpoints: "支持端点",
            rawText: "官方文本快照",
        }
    },
    nav: {
        accounts: "账号池",
        apiKeyGroups: "分组管理",
        apiKeys: "API 密钥",
        billing: "计费",
        config: "全局配置",
        dashboard: "服务总览",
        groups: {
            analytics: "数据分析",
            assets: "资产池",
            operations: "运营操作",
            system: "系统管理"
        },
        importJobs: "批量上传",
        oauthImport: "登录导入",
        logs: "系统日志",
        mainNavigation: "主导航",
        models: "模型池",
        online: "在线",
        openNavigation: "打开导航",
        proxies: "代理池",
        system: "节点健康",
        tenants: "租户池",
        usage: "用量账单",
        cleanup: "凭证治理",
        closeNavigation: "关闭导航"
    },
    notifications: {
        dismiss: "关闭通知",
        loginFailed: {
            title: "登录失败"
        },
        sessionExpired: {
            title: "登录状态已过期",
            description: "请重新登录后继续操作。"
        }
    },
    errors: {
        common: {
            failed: "失败",
            network: "网络错误，请检查网络连接。",
            timeout: "请求超时，请稍后重试。"
        },
        api: {
            unauthorized: "未授权，请重新登录。",
            invalidRequest: "请求参数无效。",
            notFound: "资源不存在。",
            serviceUnavailable: "服务暂不可用。",
            internalError: "服务器内部错误。",
            oauthProviderNotConfigured: "OAuth 服务未配置。",
            oauthCallbackListenerUnavailable: "OAuth 回调监听不可用。",
            invalidRefreshToken: "Refresh token 无效或已过期。",
            refreshTokenReused: "Refresh token 已复用，请重新获取最新 refresh token。",
            refreshTokenRevoked: "Refresh token 已被吊销。",
            oauthMissingClientId: "OAuth 服务配置不完整（缺少 client_id）。",
            oauthUnauthorizedClient: "OAuth 客户端未授权。",
            upstreamUnavailable: "上游服务不可用。",
            upstreamNetworkError: "上游网络错误。",
            oauthExchangeFailed: "OAuth 交换失败。"
        },
        http: {
            badRequest: "请求错误",
            unauthorized: "未授权",
            forbidden: "无权限",
            notFound: "未找到",
            conflict: "冲突",
            payloadTooLarge: "请求体过大",
            rateLimited: "请求过于频繁",
            internalServerError: "服务器错误",
            badGateway: "网关错误",
            serviceUnavailable: "服务不可用",
            gatewayTimeout: "网关超时"
        }
    },
    proxies: {
        check: "立即检测",
        columns: {
            actions: "操作",
            health: "健康状况",
            lastPing: "最近检测时间",
            latency: "平均响应时间",
            url: "代理节点 URL",
            weight: "路由权重"
        },
        empty: "尚未配置任何后端代理节点。",
        filters: {
            all: "全部节点",
            degraded: "降级",
            disabled: "已禁用",
            healthy: "健康",
            label: "健康筛选",
            offline: "离线"
        },
        health: {
            degraded: "降级",
            disabled: "已禁用",
            healthy: "健康",
            offline: "离线"
        },
        loading: "扫描网络拓扑中…",
        manage: "查看",
        pending: "尚未检测",
        retry: "重试",
        searchPlaceholder: "搜索节点 URL 或标签…",
        subtitle: "在这里查看代理是否可用，并管理代理节点。",
        title: "代理池"
    },
    system: {
        columns: {
            component: "组件",
            details: "详情",
            status: "状态",
            uptime: "运行时间",
            version: "版本"
        },
        components: {
            controlPlane: "控制面",
            dataPlane: "数据面路由",
            usageRepo: "用量仓库"
        },
        details: {
            analyticsUnavailable: "统计服务暂不可用",
            apiActive: "服务运行中",
            checkingAPI: "正在检查服务…",
            dbConnected: "统计存储已连接",
            endpointsResponding: "请求转发正常"
        },
        labels: {
            local: "本服务",
            remote: "转发服务",
            storage: "统计存储",
            version: "版本：",
            uptime: "运行时间"
        },
        observability: {
            badges: {
                failoverOff: "故障切换：关闭",
                failoverOn: "故障切换：开启",
                failoverWait: "切号等待 {{value}} ms",
                quickRetry: "同账号快速重试 ≤ {{value}}",
                retryPoll: "轮询间隔 {{value}} ms",
                sharedCacheOff: "共享缓存：关闭",
                sharedCacheOn: "共享缓存：开启",
                stickyConflictAvoidOff: "粘性冲突回避：关闭",
                stickyConflictAvoidOn: "粘性冲突回避：开启"
            },
            hints: {
                billingPreauthCaptureMissingTotal: "计费预身份验证捕获缺失总数",
                billingPreauthErrorRatioAvg: "计费预验证错误率平均值",
                billingPreauthErrorRatioP95: "计费预验证错误率 p95",
                billingPreauthTopModelP95: "样本最多模型预授权误差 P95",
                billingReconcileAdjust: "对账任务自动写入的余额修正次数。",
                billingReconcileFailed: "对账执行失败、需要排查的次数。",
                billingReconcileReleased: "对账任务自动关闭授权的次数。",
                billingReconcileScanned: "对账任务从 request_log 与账本扫描到的事实数。",
                billingReleaseWithoutCaptureRatio: "无捕获率计费发布",
                billingSettleCompleteRatio: "计费结算完成率",
                cacheHitRate: "本地 + 共享 sticky 缓存查询的综合命中比例。",
                failoverAttempts: "请求在账号间进行 failover 的总尝试次数。",
                failoverExhausted: "重试与切号预算耗尽后仍失败的请求次数。",
                failoverSuccess: "切换账号后最终恢复成功的请求次数。",
                failoverSuccessRate: "故障切换尝试中的成功占比。",
                sameAccountRetry: "切号前在同一账号做快速重试的总次数。",
                stickyHitRate: "会话粘性映射命中的比例。"
            },
            metrics: {
                billingPreauthCaptureMissingTotal: "计费预身份验证捕获缺失总数",
                billingPreauthErrorRatioAvg: "计费预验证错误率平均值",
                billingPreauthErrorRatioP95: "计费预验证错误率 p95",
                billingPreauthTopModelP95: "样本最多模型预授权误差 P95",
                billingReconcileAdjust: "计费对账调账数",
                billingReconcileFailed: "计费对账失败数",
                billingReconcileReleased: "计费对账释放数",
                billingReconcileScanned: "计费对账扫描数",
                billingReleaseWithoutCaptureRatio: "无捕获率计费发布",
                billingSettleCompleteRatio: "计费结算完成率",
                cacheHitRate: "路由缓存命中率",
                failoverAttempts: "故障切换尝试次数",
                failoverExhausted: "故障切换耗尽次数",
                failoverSuccess: "故障切换成功次数",
                failoverSuccessRate: "故障切换成功率",
                sameAccountRetry: "同账号快速重试次数",
                stickyHitRate: "会话粘性命中率"
            },
            na: "暂无",
            subtitle: "观测数据面自动切号、会话粘性与缓存命中效果。",
            title: "故障切换与缓存观测",
            unavailableDesc: "请检查 control-plane 到 /internal/v1/debug/state 的访问与权限配置。",
            unavailableLoading: "正在等待最新调试快照…",
            unavailableTitle: "暂未获取到数据面调试快照"
        },
        searchPlaceholder: "搜索组件、详情或版本…",
        status: {
            checking: "检查中",
            degraded: "降级",
            healthy: "健康",
            offline: "离线",
            unknown: "未知"
        },
        subtitle: "查看各个核心服务是否正常运行。",
        title: "系统状态"
    },
    tenantApiKeys: {
        actions: {
            changeGroup: "更改分组",
            disable: "禁用",
            enable: "启用"
        },
        columns: {
            actions: "操作",
            group: "分组",
            ipAllowlist: "IP 白名单",
            modelAllowlist: "模型白名单",
            name: "名称",
            prefix: "前缀",
            status: "状态"
        },
        create: {
            description: "为当前租户创建 API 密钥并配置访问限制。",
            groupLabel: "API Key 分组",
            ipAllowlistAriaLabel: "IP 白名单",
            ipAllowlistPlaceholder: "可选：逗号分隔 IP 白名单",
            modelAllowlistAriaLabel: "模型白名单",
            modelAllowlistPlaceholder: "可选：逗号分隔模型白名单",
            nameAriaLabel: "密钥名称",
            namePlaceholder: "请输入密钥名称",
            submit: "创建密钥",
            title: "创建 API 密钥"
        },
        list: {
            description: "管理当前租户的 API 密钥。",
            empty: "暂无 API 密钥。",
            searchPlaceholder: "按名称或前缀搜索 API 密钥",
            title: "API 密钥列表"
        },
        messages: {
            createFailed: "创建 API 密钥失败",
            createSuccess: "API 密钥创建成功",
            plaintextShownOnce: "明文密钥仅显示一次，请立即保存。",
            retryLater: "稍后重试",
            updateGroupFailed: "更新 API Key 分组失败"
        },
        group: {
            allowAllModels: "允许全部目录模型",
            invalidHint: "该分组已删除，请在发起请求前重新选择分组。",
            modelCount: "已配置 {{count}} 个模型"
        },
        preview: {
            allowAllModels: "该分组可使用全部目录模型。",
            columns: {
                finalPrice: "最终价格",
                formulaPrice: "公式价格",
                model: "模型"
            },
            description: "当前分组：{{name}} · 输入 {{input}} · 缓存 {{cached}} · 输出 {{output}}",
            empty: "暂无可用分组。",
            modelCount: "该分组已配置 {{count}} 个模型。",
            title: "当前分组预览"
        },
        status: {
            disabled: "禁用",
            enabled: "启用",
            groupInvalid: "分组失效"
        },
        subtitle: "管理当前租户的 API 密钥及访问策略。"
    },
    tenantApp: {
        appName: "Codex 池租户",
        auth: {
            actions: {
                backToLogin: "返回登录",
                login: "登录",
                openForgot: "忘记密码？",
                register: "注册",
                resetPassword: "重置密码",
                sendResetCode: "发送重置代码",
                switchToLogin: "已有账号？去登录",
                switchToRegister: "还没有账号？立即注册",
                verifyEmail: "验证邮箱"
            },
            brand: {
                badge: "租户工作区入口",
                points: {
                    audit: "策略与计费决策全程可观测可审计。",
                    resilience: "感知故障切换的路由保障请求持续可用。",
                    security: "凭据与会话按租户隔离。"
                },
                subtitle: "完成认证后，在同一安全工作区管理用量、计费与密钥。",
                title: "面向企业 AI 运营的稳定访问入口"
            },
            error: {
                invalidCredentialsOrUnverified: "登录失败：电子邮件或密码不正确，或电子邮件尚未验证。",
                loginFailed: "登录失败。",
                passwordMismatch: "两次输入的密码不一致。",
                passwordResetFailed: "密码重置失败。",
                registerFailed: "注册失败。",
                sendResetCodeFailed: "发送重置代码失败。",
                verificationFailed: "验证失败。"
            },
            fields: {
                confirmPassword: "确认密码",
                email: "电子邮件",
                newPassword: "新密码",
                password: "密码",
                passwordMin8: "密码（至少8个字符）",
                resetCode: "重置代码",
                tenantName: "租户名称",
                verificationCode: "验证码"
            },
            forgot: {
                drawerHint: "发送验证码后，将从底部上滑抽屉展示“重置码 + 新密码”输入。",
                stepResetPassword: "设置新密码",
                stepSendCode: "发送验证码"
            },
            notice: {
                emailVerified: "邮箱验证成功。请使用此账号登录。",
                loginSuccess: "登录成功。",
                passwordResetSuccess: "密码重置成功。请重新登录。",
                registerDebugCode: "注册成功，验证码（调试）：{{code}}",
                registerSuccess: "注册成功。输入电子邮件验证码以激活您的账号。",
                resetCodeDebug: "密码重置代码（调试）：{{code}}",
                resetCodeSentIfExists: "如果电子邮件存在，将发送重置代码。",
                sessionExpired: "租户会话已过期。请重新登录。",
                verifyCodeHint: "未收到验证码？请等待 60 秒后重新发送。"
            },
            placeholders: {
                confirmPassword: "请再次输入密码",
                email: "name@company.com",
                newPassword: "请输入新密码",
                password: "请输入密码",
                resetCode: "请输入重置代码",
                tenantName: "请输入租户名称",
                verificationCode: "请输入验证码"
            },
            sections: {
                authSubtitle: "在同一卡片中切换登录与注册。",
                forgotPasswordTitle: "重置密码",
                forgotPasswordSubtitle: "抽屉式两步流程：先发送验证码，再设置新密码。",
                loginTitle: "租户登录",
                registerTitle: "租户注册",
                verifyEmailSubtitle: "输入邮件中的验证码以激活账号。",
                verifyEmailTitle: "电子邮件验证"
            },
            social: {
                comingSoon: "第三方登录（即将支持）",
                github: "GitHub",
                google: "Google"
            },
            tabs: {
                login: "登录",
                register: "注册"
            }
        },
        loadingPortal: "正在加载租户门户…",
        menu: {
            analytics: "分析",
            apiKeys: "API 密钥",
            assets: "资产",
            billing: "计费中心",
            dashboard: "仪表板",
            logs: "日志",
            usage: "用法"
        }
    },
    tenantBilling: {
        actions: {
            dailyCheckin: "每日签到",
            exportCsv: "导出 CSV"
        },
        failoverAction: {
            crossAccountFailover: "跨账号故障转移",
            retrySameAccount: "重试同一账号",
            returnFailure: "返回失败",
            unknown: "未知"
        },
        failureReason: {
            accountDeactivated: "账号已停用",
            billingUsageMissing: "账单使用情况缺失",
            failoverExhausted: "故障转移已耗尽",
            noUpstreamAccount: "无上游账号",
            streamPreludeError: "流前奏错误",
            tokenInvalidated: "令牌失效",
            transportError: "传输错误",
            upstreamRequestFailed: "上游请求失败",
            unknown: "未知"
        },
        filters: {
            day: "按日",
            dayShort: "日",
            granularityAriaLabel: "计费粒度",
            month: "按月",
            monthShort: "月"
        },
        groupPricing: {
            allKeys: "全部 API Key",
            apiKeyAriaLabel: "API Key 选择器",
            columns: {
                apiKey: "API Key",
                finalPrice: "最终价格",
                formulaPrice: "公式价格",
                group: "分组",
                model: "模型",
                state: "状态"
            },
            description: "查看每个 API Key 当前使用的计价分组，并按单个 API Key 检查有效模型价格。",
            groupSummary: "已配置模型：{{count}} · 全量放开：{{allowAll}}",
            invalidGroup: "该 API Key 绑定到了已删除分组，在你改分组前请求都会失败。",
            state: {
                active: "有效",
                invalid: "失效（分组已删除）"
            },
            title: "API Key 分组定价"
        },
        ledger: {
            columns: {
                balanceAfter: "变动后余额",
                delta: "积分变动",
                detail: "计费明细",
                event: "事件",
                model: "模型",
                requestType: "请求类型",
                time: "时间"
            },
            description: "按当前租户筛选的账本流水。",
            detail: {
                charged: "实扣",
                extraCharge: "额外扣费",
                failoverAction: "失败处理动作",
                failure: "失败原因",
                failureKeyword: "失败关键字",
                failureSummary: "{{failure}}（{{reason}}）",
                reconcileAdjust: "对账调整",
                source: "来源",
                tokenSettle: "Token 结算",
                unitPrice: {
                    cached: "缓存",
                    input: "输入",
                    output: "输出",
                    summary: "单价汇总"
                },
                upstreamStatus: "上游 {{status}}"
            },
            empty: "暂无账本流水",
            requestTypes: {
                nonStream: "非流式",
                stream: "流式",
                unknown: "-"
            },
            showRaw: "显示原始账本",
            title: "账本流水"
        },
        messages: {
            checkinFailed: "签到失败",
            checkinReward: "签到奖励",
            checkinSuccess: "签到成功",
            retryLater: "稍后重试"
        },
        releaseReason: {
            billingSettleFailed: "计费结算失败",
            failoverExhausted: "故障转移已耗尽",
            invalidUpstreamUrl: "无效的上游网址",
            noUpstreamAccount: "无上游账号",
            streamPreludeError: "流前奏错误",
            streamUsageMissing: "流使用缺失",
            transportError: "传输错误",
            upstreamRequestFailed: "上游请求失败",
            unknown: "未知"
        },
        snapshot: {
            columns: {
                consumed: "消耗积分",
                date: "日期",
                eventCount: "扣减事件数",
                month: "月"
            },
            description: "按周期汇总扣减事件，用于结算与对账。",
            empty: "暂无结算快照",
            title: "结算快照"
        },
        subtitle: "查看余额、消耗趋势与账本明细。",
        summary: {
            balance: "当前余额",
            monthConsumed: "本月消耗",
            negativeOnly: "仅统计负向扣减",
            todayConsumed: "今日消耗",
            unitCredits: "单位：积分"
        },
        title: "账务中心",
        tokenSegment: {
            cached: "缓存",
            input: "输入",
            output: "输出"
        },
        trend: {
            description: "按时间粒度展示租户积分消耗趋势。",
            empty: "暂无趋势数据",
            series: {
                consumed: "消耗积分"
            },
            title: "消耗趋势"
        }
    },
    groupsPage: {
        actions: {
            create: "新建分组",
            deleteGroup: "删除分组",
            deletePolicy: "删除策略",
            saveGroup: "保存分组",
            savePolicy: "保存模型策略"
        },
        columns: {
            actions: "操作",
            apiKeysCount: "API Key {{count}} 个",
            modelsCount: "模型 {{count}} 个",
            multipliers: "倍率",
            name: "分组",
            status: "状态",
            usage: "使用情况"
        },
        editor: {
            createTitle: "新建分组",
            description: "配置分组倍率以及模型级价格覆盖。",
            editTitle: "编辑分组"
        },
        empty: "暂无分组",
        form: {
            allowAllModels: "允许全部目录模型",
            cachedInputMultiplier: "缓存输入倍率（ppm）",
            default: "默认分组",
            description: "描述",
            enabled: "启用",
            inputMultiplier: "输入倍率（ppm）",
            name: "分组名称",
            outputMultiplier: "输出倍率（ppm）"
        },
        messages: {
            groupDeleted: "分组已删除。",
            groupDeleteFailed: "删除分组失败。",
            groupSaved: "分组已保存：{{name}}",
            groupSaveFailed: "保存分组失败。",
            policyDeleted: "模型策略已删除。",
            policyDeleteFailed: "删除模型策略失败。",
            policySaved: "模型策略已保存。",
            policySaveFailed: "保存模型策略失败。"
        },
        policy: {
            cachedInputAbsolutePrice: "缓存输入绝对价格",
            cachedInputMultiplier: "缓存输入倍率（ppm）",
            description: "从统一模型目录中选择模型，然后配置倍率或绝对价格。",
            enabled: "启用策略",
            inputAbsolutePrice: "输入绝对价格",
            inputMultiplier: "输入倍率（ppm）",
            model: "模型",
            outputAbsolutePrice: "输出绝对价格",
            outputMultiplier: "输出倍率（ppm）",
            title: "模型策略"
        },
        preview: {
            columns: {
                finalPrice: "最终价格",
                formulaPrice: "公式价格",
                mode: "模式",
                model: "模型"
            },
            description: "展示当前分组下对租户可见的最终价格。",
            moreHidden: "还有 {{count}} 个模型已折叠",
            mode: {
                absolute: "绝对价覆盖",
                formula: "倍率公式"
            },
            title: "有效模型预览"
        },
        searchPlaceholder: "按名称、描述或状态搜索分组",
        status: {
            default: "默认",
            deleted: "已删除",
            disabled: "已禁用",
            enabled: "已启用"
        },
        subtitle: "管理 API Key 分组、模型白名单、倍率和分组级绝对定价。",
        title: "分组管理"
    },
    tenantDashboard: {
        actions: {
            manageApiKeys: "管理 API 密钥",
            refresh: "刷新",
            viewBilling: "查看账单",
            viewRequestLogs: "查看请求日志"
        },
        kpi: {
            avgFirstTokenSpeed: "平均首字速度",
            avgFirstTokenSpeedDesc: "TTFT（流式精确 / 非流式近似）",
            rpm: "RPM",
            rpmDesc: "每分钟请求数",
            totalRequests: "总请求数",
            totalRequestsDesc: "所选时间范围",
            totalTokens: "Token 消耗总量",
            totalTokensDesc: "输入 + 缓存 + 输出 + 推理",
            tpm: "TPM",
            tpmDesc: "每分钟 Token 数"
        },
        cards: {
            activeKeys: {
                description: "注意：仅计算有请求的键",
                title: "活跃 API 密钥计数（选定期间）"
            },
            availableKeys: {
                description: "基于租户密钥管理设置",
                title: "当前可用的 API 密钥"
            },
            keyEnableRate: {
                description: "启用比例：{{rate}}%（{{enabled}} / {{total}} 个密钥）",
                title: "密钥启用比例"
            },
            peakHour: {
                description: "请求量最高的时间窗口",
                empty: "暂无数据",
                title: "峰值时段"
            },
            requestVelocity: {
                description: "所选时间范围内的每小时平均请求数",
                title: "请求速率（每小时）"
            },
            totalRequests: {
                scopeAllKeys: " / 全部密钥",
                scopePrefix: "范围：当前租户",
                scopeSingleKey: " / 单个密钥",
                title: "租户 API 密钥请求总数（所选期间）"
            }
        },
        filters: {
            apiKeyAll: "所有 API 密钥",
            apiKeyAriaLabel: "API 密钥筛选",
            apiKeyHint: "提示：使用 API 密钥筛选可以快速定位热点。",
            range: {
                last24Hours: "过去 24 小时",
                last30Days: "过去 30 天",
                last7Days: "过去 7 天"
            },
            rangeAriaLabel: "时间范围"
        },
        hero: {
            badge: "租户工作区总览",
            summaryAllApiKeys: "（所有 API 密钥）",
            summaryPrefix: "范围：当前租户",
            summarySingleApiKey: "（单个 API 密钥）"
        },
        subtitle: {
            allApiKeys: "（所有 API 密钥）",
            metricsFocus: "关注指标：TPM、RPM、Token 消耗总量、总请求数与首字速度。",
            scopePrefix: "范围：当前租户",
            singleApiKey: "（单个 API 密钥）",
            timeWindow: "，时间窗口："
        },
        modelDistribution: {
            description: "按请求数或 Token 用量查看模型 Top 分布。",
            empty: "暂无模型分布数据",
            modeRequests: "按请求数",
            modeTokens: "按 Token",
            other: "其他",
            title: "模型请求分布"
        },
        tokenComponents: {
            cached: "缓存输入",
            input: "输入",
            output: "输出",
            reasoning: "推理"
        },
        tokenSummary: {
            title: "Token 组件汇总"
        },
        tokenTrend: {
            description: "按小时展示 Token 组件趋势，可按组件开关聚焦消耗来源。",
            empty: "暂无 Token 趋势数据",
            title: "Token 使用趋势"
        },
        topKeys: {
            description: "基于所选时间范围内的请求量",
            empty: "暂无 API 密钥使用排行",
            requests: "{{count}} 次请求",
            share: "占比 {{percent}}%",
            title: "Top API 密钥",
            unknownKey: "未命名密钥"
        },
        groupOverview: {
            allDescription: "查看当前 API Key 在各个计价分组中的分布情况。",
            empty: "暂无可展示的 API Key 分组。",
            invalid: "失效",
            keysBound: "绑定了 {{count}} 个 API Key",
            singleDescription: "查看当前 API Key 的分组绑定与有效性状态。",
            title: "API Key 分组概览",
            valid: "有效"
        },
        title: "租户仪表板",
        trend: {
            description: "范围：租户 API 密钥请求量（每小时粒度）",
            empty: "还没有请求数据",
            series: {
                requests: "请求数"
            },
            title: "请求趋势"
        }
    },
    tenantLogs: {
        audit: {
            actionValues: {
                adminOperation: "管理操作",
                authOperation: "认证操作",
                requestOperation: "请求操作",
                tenantOperation: "租户操作",
                unknown: "未知操作"
            },
            actorTypes: {
                adminUser: "管理员用户",
                apiKey: "API 密钥",
                system: "系统",
                tenantUser: "租户用户",
                unknown: "未知操作者"
            },
            columns: {
                action: "操作",
                actor: "操作者",
                reason: "细节",
                result: "结果",
                target: "目标",
                time: "时间"
            },
            description: "定义：控制平面审计事件（仅限当前租户）",
            empty: "无审计日志数据",
            filters: {
                actionPlaceholder: "操作",
                actorIdPlaceholder: "操作者 ID",
                actorTypePlaceholder: "操作者类型",
                keywordPlaceholder: "关键字（原因/有效负载）",
                resultStatusPlaceholder: "结果状态",
                actionAriaLabel: "操作筛选",
                actorIdAriaLabel: "操作者 ID 筛选",
                actorTypeAriaLabel: "操作者类型筛选",
                keywordAriaLabel: "关键字筛选",
                rangeAriaLabel: "时间范围",
                resultStatusAriaLabel: "结果状态筛选"
            },
            resultStatuses: {
                denied: "已拒绝",
                failed: "失败",
                ok: "成功",
                unknown: "未知结果"
            },
            title: "审计日志"
        },
        filters: {
            range: {
                last24Hours: "过去 24 小时",
                last30Days: "过去 30 天",
                last7Days: "过去 7 天"
            }
        },
        request: {
            columns: {
                apiKey: "API 密钥",
                error: "错误",
                latency: "延迟（毫秒）",
                path: "路径",
                requestId: "请求 ID",
                status: "状态",
                time: "时间"
            },
            description: "定义：数据平面原始请求事件（仅限当前租户）",
            empty: "无日志数据",
            filters: {
                apiKeyIdPlaceholder: "API 密钥 ID",
                keywordPlaceholder: "关键字（路径/错误/模型）",
                requestIdPlaceholder: "请求 ID",
                statusCodePlaceholder: "状态代码（例如 429）",
                apiKeyAriaLabel: "API 密钥筛选",
                keywordAriaLabel: "关键字筛选",
                rangeAriaLabel: "时间范围",
                requestIdAriaLabel: "请求 ID 筛选",
                statusCodeAriaLabel: "状态码筛选"
            },
            title: "请求日志"
        },
        scope: "范围：仅限当前租户",
        tabs: {
            audit: "审计日志",
            request: "请求日志"
        },
        title: "日志"
    },
    tenantUsage: {
        columns: {
            apiKey: "API 密钥",
            requests: "请求数",
            tenantLabel: "租户：{{tenantId}}",
            time: "时间"
        },
        filters: {
            apiKeyAll: "全部 API 密钥",
            apiKeyAriaLabel: "API 密钥筛选",
            range: {
                last24Hours: "过去 24 小时",
                last30Days: "过去 30 天",
                last7Days: "最后 7 天"
            },
            rangeAriaLabel: "时间范围"
        },
        hourly: {
            description: "按小时查看请求量变化。",
            empty: "暂无数据",
            title: "每小时请求量"
        },
        leaderboard: {
            description: "按请求量统计当前租户 API 密钥。",
            empty: "暂无排行数据",
            title: "API 密钥排行"
        },
        subtitle: "查看请求趋势与 API 密钥排行。",
        title: "用量分析",
        trend: {
            description: "按所选时间范围汇总请求量。",
            empty: "暂无趋势数据",
            title: "请求趋势"
        }
    },
    tenants: {
        create: {
            fields: {
                expiresAt: "到期时间",
                name: "租户名",
                plan: "计划（credit）",
                status: "状态（active/inactive）"
            },
            submit: "创建租户",
            title: "新增租户"
        },
        impersonation: {
            copyToken: "复制 token",
            create: "创建代操作",
            fields: {
                reason: "reason（必填）"
            },
            revoke: "撤销会话",
            sessionIdLabel: "会话 ID：",
            tokenLabel: "令牌：",
            title: "管理员代操作"
        },
        keys: {
            create: {
                fields: {
                    name: "Key 名称",
                    namePlaceholder: "例如：admin-main-key"
                },
                submit: "创建密钥",
                title: "创建 API 密钥"
            },
            created: {
                copyPlaintext: "复制明文密钥",
                notice: "明文密钥仅展示一次，请及时保存"
            },
            list: {
                caption: "租户 API 密钥列表",
                columns: {
                    actions: "操作",
                    createdAt: "创建时间",
                    name: "名称",
                    prefix: "前缀",
                    status: "状态"
                },
                copyPrefix: "复制 key 前缀",
                disable: "禁用",
                empty: "当前租户暂无 API 密钥",
                enable: "启用",
                status: {
                    active: "生效",
                    revoked: "已撤销"
                },
                title: "API 密钥列表"
            }
        },
        list: {
            caption: "租户池列表",
            columns: {
                actions: "操作",
                apiKeys: "API 密钥",
                expiresAt: "到期时间",
                plan: "计划",
                status: "状态",
                tenant: "租户",
                tenantId: "租户 ID",
                updatedAt: "更新时间"
            },
            planValues: {
                credit: "积分计划",
                unknown: "自定义（{{value}}）"
            },
            statusValues: {
                active: "生效",
                inactive: "停用",
                unknown: "未知（{{value}}）"
            },
            defaultBadge: "默认",
            empty: "暂无租户数据",
            openProfile: "打开租户资料",
            searchPlaceholder: "按租户名称或 ID 搜索",
            title: "租户池列表"
        },
        messages: {
            apiKeyCreateFailed: "创建 API 密钥失败",
            apiKeyCreateSuccess: "已为租户 {{tenantName}} 创建 API 密钥：{{keyName}}",
            apiKeyNameRequired: "请输入 API 密钥名称",
            apiKeyToggleFailed: "更新 API 密钥状态失败",
            createFailed: "创建租户失败",
            createSuccess: "租户创建成功：{{name}} ({{id}})",
            impersonationCreateFailed: "创建代操作失败",
            impersonationCreated: "代操作会话已创建（token 已返回）",
            impersonationRevokeFailed: "撤销代操作失败",
            impersonationRevoked: "代操作会话已撤销",
            rechargeFailed: "租户充值失败",
            rechargeSuccess: "充值成功：+{{amount}}，当前余额 {{balance}}",
            updateFailed: "更新租户失败",
            updateSuccess: "租户更新成功：{{name}}"
        },
        profile: {
            dialogDescription: "统一在一个弹窗分页管理资料、API 密钥和用量。",
            dialogTitle: "租户资料",
            dialogTitleWithName: "租户资料 · {{name}}",
            fields: {
                expiresAt: "到期时间",
                plan: "计划",
                status: "状态"
            },
            meta: {
                createdAt: "创建时间",
                tenantId: "租户 ID",
                updatedAt: "更新时间"
            },
            save: "保存资料",
            section: {
                title: "租户资料"
            },
            tabs: {
                ariaLabel: "租户资料分页",
                keys: "API 密钥",
                profile: "资料",
                usage: "用量"
            }
        },
        recharge: {
            fields: {
                amount: "微积分（整数）",
                reason: "原因"
            },
            submit: "执行充值",
            title: "租户充值"
        },
        subtitle: "在这里查看租户是否可用，并管理租户资料、API 密钥与用量。",
        title: "租户池",
        usage: {
            filter: {
                allKeys: "所有 API 密钥",
                currentView: "当前视图",
                label: "API 密钥过滤器",
                noKeys: "当前租户没有 API 密钥",
                noMatches: "没有匹配的 API 密钥",
                placeholder: "搜索名称/前缀/key_id"
            },
            meta: {
                tenantId: "租户 ID"
            },
            metrics: {
                accountRequests: "账号请求",
                activeAccounts: "活跃账号",
                activeApiKeys: "活动 API 密钥",
                apiKeyRequests: "API 密钥请求",
                tenantApiKeyRequests: "租户 API 密钥请求"
            },
            sectionTitle: "过去 24 小时内的使用情况",
            status: {
                error: "无法加载使用数据",
                loading: "正在加载使用数据…"
            }
        }
    },
    theme: {
        aurora: "极光",
        colorful: "彩色",
        dark: "深色",
        light: "浅色"
    },
    usage: {
        actions: {
            export: "导出报表",
            filters: "高级筛选"
        },
        chart: {
            empty: "此时段内无数据记录。",
            requests: "请求次数",
            subtitle: "跨所有上游服务商的数据汇聚",
            title: "近30日 Token 消耗"
        },
        subtitle: "网关请求消耗与基础设施调用量聚合统计",
        title: "用量分析",
        topKeys: {
            columns: {
                apiKey: "密钥 ID",
                name: "租户 / 密钥",
                requests: "请求量",
                share: "占比",
                tenant: "租户"
            },
            empty: "暂无用量记录。",
            keyFallback: "API 密钥 {{keyId}}",
            searchPlaceholder: "搜索 API 密钥或租户…",
            subtitle: "按请求调用量统计",
            title: "API 密钥排行",
            reqs: "次"
        }
    },
    cleanup: {
        title: "凭证清理机制",
        subtitle: "自动化的账号治理与生命周期策略",
        save: "保存策略",
        policy: {
            title: "自动治理策略",
            desc: "当 refresh_token 复用、吊销或长期失效时，可通过此策略自动隔离并降低污染扩散。",
            refreshEnabled: "启用 OAuth 自动刷新",
            refreshEnabledDesc: "关闭后账号不会自动续签 access_token。",
            intervalSec: "刷新间隔（秒）",
            notes: "策略备注"
        },
        workspace: {
            title: "OAuth 账号治理工作台",
            desc: "按账号查看登录刷新状态，支持立即刷新与暂停/恢复同组账号。",
            searchPlaceholder: "搜索账号名 / 账号 ID",
            onlyDisabled: "只看已禁用账号",
            loadingAccounts: "加载账号中…",
            noAccounts: "没有匹配的 OAuth 账号。",
            enabled: "已启用",
            disabled: "已停用",
            selectHint: "请选择左侧账号查看状态。",
            loadingStatus: "加载 OAuth 状态中…",
            noStatus: "当前账号暂未获取到 OAuth 状态。",
            refreshNow: "立即刷新",
            disableFamily: "暂停同组账号",
            enableFamily: "恢复同组账号",
            status: {
                never: "未刷新",
                ok: "正常",
                failed: "失败"
            },
            fields: {
                refreshStatus: "刷新状态",
                reuseDetected: "检测到重复刷新",
                groupId: "账号组 ID",
                tokenVersion: "令牌版本",
                expiresAt: "令牌过期时间",
                errorCode: "错误码",
                errorMessage: "错误详情"
            }
        },
        quarantine: {
            title: "自动隔离策略 (Quarantine)",
            desc: "发生未授权错误时自动隔离底层账号",
            threshold: "错误阈值",
            thresholdDesc: "触发隔离前允许的连续 401/403 错误次数",
            action: "刷新令牌吊销操作",
            actionDesc: "当基础 refresh_token 被判定为失效时",
            options: {
                family: "隔离整个账号族",
                disable: "仅禁用当前项",
                nothing: "不做处理"
            }
        },
        purge: {
            title: "自动擦除策略 (Purge)",
            desc: "永久清除死亡凭证以节省数据库空间",
            retention: "保留期限",
            retentionDesc: "在执行擦除前保留死亡账号的天数"
        }
    },
    apiKeys: {
        title: "API 密钥",
        subtitle: "为客户端应用程序签发和管理安全访问凭据。",
        create: "创建密钥",
        search: "搜索密钥名称或前缀…",
        loading: "加载凭据中…",
        empty: "未找到符合条件的 API 密钥。",
        columns: {
            name: "应用名称",
            tenant: "租户 ID",
            key: "API 密钥",
            status: "状态",
            issued: "签发时间",
            actions: "操作"
        },
        status: {
            active: "活跃",
            revoked: "已撤销"
        },
        defaultTenant: "默认租户",
        filters: {
            label: "状态筛选",
            all: "全部密钥",
            active: "活跃",
            revoked: "已撤销"
        },
        actions: {
            copyPrefixTitle: "复制前缀",
            menu: "密钥操作",
            copyPrefix: "复制 key 前缀",
            processing: "处理中…",
            disable: "禁用该密钥",
            enable: "重新启用"
        },
        messages: {
            createFailed: "创建 API Key 失败",
            missingName: "请先填写 Key 名称"
        },
        dialog: {
            create: {
                title: "创建 API Key",
                desc: "为租户创建可调用 Data Plane 的访问密钥。创建后会返回一次明文 key，请立即保存。",
                nameLabel: "Key 名称",
                namePlaceholder: "例如：prod-codex-clients",
                tenantLabel: "租户名称（可选）",
                tenantPlaceholder: "留空则使用 default",
                confirm: "创建",
                creating: "创建中…"
            },
            created: {
                title: "新密钥已创建",
                desc: "明文密钥只会显示一次，请立即复制保存。",
                securityTip: "安全提醒：关闭此窗口后将无法再次查看明文 key。",
                nameLabel: "Key 名称",
                plaintextLabel: "明文密钥",
                close: "关闭",
                copyPlaintext: "复制明文密钥"
            }
        }
    }
}
