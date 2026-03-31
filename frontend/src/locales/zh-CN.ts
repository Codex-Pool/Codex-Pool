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
            disable: "禁用",
            enableAccount: "启用账号",
            export: "导出 CSV",
            filter: "筛选列表",
            oneTimeNoGroupAction: "一次性会话账号不支持同组操作",
            pauseGroup: "暂停同组账号",
            refreshAccounts: "刷新",
            refreshLogin: "刷新登录",
            refreshingAccounts: "刷新中",
            resumeGroup: "恢复同组账号",
            selectAll: "全选当前筛选结果",
            selectOne: "选择账号 {{label}}",
            selectedCount: "已选 {{count}} 条",
            view: "查看详情",
            viewDetails: "查看详情",
            edit: "编辑属性",
            refresh: "强制刷新",
            suspend: "挂起暂停",
            exportSuccess: "导出成功",
            refreshTriggered: "已触发账号刷新"
        },
        columns: {
            actions: "操作",
            account: "账号",
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
            noSupportedModels: "这个账号还没有采集到可用模型列表。",
            oauthNotApplicable: "该账号类型不支持 OAuth 详情",
            oauthTitle: "OAuth 状态",
            profileTitle: "账号资料",
            rawTitle: "原始数据",
            supportedModelsCount: "{{count}} 个模型",
            tabAria: "账号详情标签页",
            tabs: {
                limits: "限额",
                oauth: "OAuth",
                profile: "资料",
                raw: "原始"
            },
            sections: {
                cache: "限额缓存",
                connection: "连接信息",
                credentials: "凭据",
                identity: "身份信息",
                refresh: "刷新状态",
                supportedModels: "可用模型",
                subscription: "订阅信息"
            },
            fields: {
                email: "邮箱",
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
            batchRefreshStarted: "已开始为 {{count}} 个账号刷新登录",
            refreshFailed: "登录刷新失败",
            refreshFailedStatus: "登录刷新失败，状态={{status}}",
            refreshFailedSummary: "登录刷新失败：{{summary}}",
            refreshPollingTimeout: "轮询登录刷新任务超时。",
            refreshJobId: "任务 ID：{{jobId}}",
            refreshJobSummary: "任务 ID：{{jobId}} · {{processed}}/{{total}}",
            refreshListFailed: "刷新用量失败",
            refreshListSuccess: "用量已刷新",
            refreshSuccess: "登录刷新完成",
            requestFailed: "请求失败，请稍后重试",
            resumeFamilyFailed: "恢复同家族账号失败",
            resumeFamilySuccess: "同家族账号已恢复",
            toggleUnsupported: "当前后端版本不支持账号启用/禁用接口，请升级 control-plane。",
            refreshTriggered: "已开始刷新登录"
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
            provider: {
                legacyBearer: "旧版 Bearer 令牌",
                refreshToken: "刷新令牌",
                unknown: "未知提供方"
            },
            sourceType: {
                codex: "Codex",
                unknown: "未知来源"
            },
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
            resetAt: "{{absolute}}",
            unavailable: "暂无限额数据",
            usedPrefix: "已用"
        },
        planValues: {
            business: "商业版",
            enterprise: "企业版",
            free: "免费版",
            plus: "Plus",
            pro: "Pro",
            team: "团队版"
        },
        searchPlaceholder: "按邮箱、标签、URL 搜索…",
        notAvailable: "暂无",
        health: {
            ok: "正常",
            failed: "失败",
            disabled: "已禁用"
        },
        status: {
            active: "正常",
            disabled: "已禁用"
        },
        plan: {
            enterprise: "企业版",
            free: "免费版",
            plus: "Plus",
            pro: "Pro",
            team: "团队版",
            unknown: "未知计划"
        },
        subtitle: "在这里查看账号是否可用，并管理登录状态",
        syncing: "正在同步账号状态…",
        title: "账号池"
    },
    billing: {
        antigravity: {
            activeGranularity: "当前粒度",
            activeScope: "当前范围",
            activeTenant: "当前租户",
            averageDeduction: "平均单次扣减",
            averageDeductionHint: "按账本中的负向积分变动估算每次用量事件的平均扣减强度。",
            balanceHint: "当前租户可继续用于后续请求的可用积分余额。",
            balanceAfter: "余额 {{value}}",
            contractHint: "当前账单页面与 main 分支保持同一套租户 credits 契约。",
            currentBalanceChip: "余额快照",
            eventType: {
                adminRecharge: "管理员充值",
                unknown: "未知事件",
                usage: "用量扣费"
            },
            ledgerRows: "流水条目",
            ledgerSearch: "搜索账本流水、请求 ID 或模型…",
            ledgerSignalsDescription: "快速查看充值事件、扣费事件、账本条目数量和当前余额快照。",
            ledgerSignalsTitle: "账本信号",
            lastUpdated: "最近更新：{{time}}",
            lastUpdatedLabel: "最近更新时间",
            loading: "正在加载租户账单…",
            logCoverage: "日志覆盖",
            logCoverageHint: "当前时间窗口内可追溯的请求日志条目数。",
            monthConsumedHint: "当前自然月内已经记录的累计扣减额度。",
            noActivityTitle: "这个租户还没有产生账本活动",
            noActivityDescription: "当前租户 {{tenant}} 还没有充值或用量扣费记录，所以趋势图和账本会保持为空。先准备账号池或发起请求后，这里会开始出现真实账单轨迹。",
            noTenant: "暂无可用租户",
            noTenantDescription: "请先创建或同步一个租户，然后这里才能展示真实的 credits 汇总与账本流水。",
            rechargeEvents: "充值事件",
            requestCountHint: "当前时间窗口内纳入成本汇总的请求总数。",
            scopeTitle: "范围",
            scopePanelDescription: "在租户与统计粒度之间切换，快速对齐你正在查看的费用窗口。",
            scopePanelTitle: "账单范围",
            tenantCreditEvent: "租户积分事件",
            tenantLabel: "租户",
            todayConsumedHint: "当前自然日内由请求处理产生的积分消耗。",
            totalCostHint: "当前时间窗口内估算的总成本。",
            usageEvents: "扣费事件",
            avgCostHint: "按总成本除以请求数得到的平均单次请求成本。"
        },
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
                serviceTier: "服务层级：{{tier}}",
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
        never: "从不",
        openMenu: "打开菜单",
        closeMenu: "关闭菜单",
        refresh: "刷新",
        skipToMainContent: "跳至主要内容",
        table: {
            allItemsSelected: "已选择全部条目",
            columns: "列",
            dataTableAria: "数据表格",
            firstPage: "第一页",
            go: "跳转",
            jumpToPage: "跳转页码",
            lastPage: "最后一页",
            nextPage: "下一页",
            pageOf: "第 {{page}} / {{total}} 页",
            previousPage: "上一页",
            range: "{{start}}-{{end}} / 共 {{total}} 条",
            rowActions: "行操作",
            rowsPerPage: "每页条数",
            searchLabel: "搜索表格",
            searchPlaceholder: "搜索当前列表…",
            selectedCount: "已选 {{count}} / {{total}} 条",
            statusLabel: "状态",
            statusFilter: "状态筛选",
            totalItems: "共 {{count}} 条"
        },
        languages: {
            english: "English",
            simplifiedChinese: "简体中文"
        },
        tokenSegments: {
            cached: "缓存",
            input: "输入",
            output: "输出"
        },
        units: {
            millisecondsShort: "{{value}}ms",
            secondsShort: "{{value}}s"
        },
        current: "当前",
        actions: "操作",
        toggleLanguage: "切换语言",
        toggleTheme: "切换主题",
        uiPreferences: {
            title: "界面偏好",
            drawerPlacement: "抽屉弹出方位",
            drawerPlacementOptions: {
                bottom: "底部",
                right: "右侧",
                left: "左侧",
                top: "顶部"
            }
        },
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
    serviceTier: {
        default: "默认",
        flex: "弹性",
        priority: "优先"
    },
    config: {
        antigravity: {
            loading: "正在加载配置…",
            metrics: {
                authValidation: "鉴权校验",
                controlPlane: "控制面监听",
                dataPlane: "数据平面地址",
                refreshStatus: "刷新状态"
            },
            notesEmpty: "未填写备注",
            notesPresent: "已填写备注",
            notesStatus: "备注状态",
            refreshDisabled: "自动刷新已关闭",
            refreshEnabled: "自动刷新已开启",
            refreshIntervalLabel: "刷新间隔",
            reset: "重置",
            saveFailed: "保存配置失败。",
            section: {
                connection: "连接",
                refresh: "刷新"
            },
            runtimePanelTitle: "运行时同步状态",
            synced: "与服务端配置一致",
            unsavedChanges: "有未保存更改"
        },
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
        themeLab: {
            title: "Theme Lab",
            description: "让默认视觉更贴近 HeroUI Pro 原生，并在当前浏览器里做轻量微调。",
            localOnly: "仅当前浏览器生效",
            currentMode: "当前模式：{{value}}",
            resolvedMode: "实际生效：{{value}}",
            modeTitle: "主题模式",
            modeDescription: "切换浅色、深色或跟随系统。这里会直接驱动全局 HeroUI 主题，而不是页面局部样式。",
            modeLightHint: "始终使用 HeroUI 浅色主题。",
            modeDarkHint: "始终使用 HeroUI 深色主题。",
            modeSystemHint: "跟随系统外观偏好自动切换。",
            radiusTitle: "圆角密度",
            radiusDescription: "调整全局卡片、输入框和面板的弧度强弱。",
            densityTitle: "页面密度",
            densityDescription: "调整全局间距与壳层留白，观察 HeroUI Pro 工作台的呼吸感。",
            footer: "这些设置会即时作用到当前管理台，并保存在本地浏览器。运行时配置的保存与重置不会影响这里。",
            reset: "恢复 HeroUI Pro 默认",
            previewTitle: "实时预览",
            previewDescription: "用一组标准卡片、表单和操作按钮，快速检查当前基线是否贴近官方风格。",
            previewMetric: "预览指标",
            previewMetricHint: "观察背景层级、字重、边框与阴影关系。",
            previewInputLabel: "预览输入",
            previewInputValue: "当前主题已经开始影响这一行。",
            previewPrimaryAction: "主操作",
            previewSecondaryAction: "次操作",
            previewChipHealthy: "状态健康",
            previewChipAttention: "需要关注",
            previewResolvedChip: "已套用：{{value}}",
            previewChipRadius: "圆角：{{value}}",
            previewChipDensity: "密度：{{value}}",
            density: {
                compact: "紧凑",
                comfortable: "舒展"
            },
            radius: {
                compact: "利落",
                default: "原生默认",
                relaxed: "更柔和"
            }
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
            openMenu: "打开操作菜单",
            viewAccounts: "查看账号池",
            viewBilling: "查看账单",
            viewImports: "查看导入批次",
            viewLogs: "查看请求日志"
        },
        antigravity: {
            activeAccounts: "{{count}} 个活跃账号",
            last24Hours: "过去 24 小时",
            loading: "正在加载总览数据…",
            signal: "运行信号"
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
            emptyDescription: "当前时间窗口内没有活跃的基础设施或用量链路告警。",
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
            description: "先看全局，再在告警、尖峰或成本波动需要解释时收窄到单个租户或 API 密钥。",
            eyebrow: "上下文",
            range: {
                last24Hours: "过去 24 小时",
                last30Days: "过去 30 天",
                last7Days: "过去 7 天"
            },
            rangeAriaLabel: "时间范围",
            scopeAriaLabel: "范围筛选",
            tenantAriaLabel: "租户筛选",
            tenantPlaceholder: "选择租户",
            title: "范围与筛选"
        },
        hero: {
            eyebrow: "运行总览"
        },
        meta: {
            autoRefresh: "每 30 秒自动刷新"
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
        subtitle: "在一个总览视角里查看网关健康、用量变化与受管资源。",
        table: {
            apiKey: "API 密钥",
            requests: "请求数"
        },
        modelDistribution: {
            a11y: {
                model: "模型",
                summary: "模型分布包含 {{count}} 行，按 {{mode}} 排序。领先模型：{{top}}。后附无障碍数据表。",
                summaryEmpty: "当前筛选条件下暂无模型分布数据。",
                tableLabel: "模型分布数据表"
            },
            description: "按请求数或 Token 用量查看模型 Top 分布。",
            empty: "暂无模型分布数据",
            modeRequests: "按请求数",
            modeTokens: "按 Token",
            other: "其他",
            title: "模型请求分布"
        },
        overview: {
            attentionNeeded: "建议尽快处理",
            autoRefresh: "每 30 秒自动刷新",
            degraded: "已降级",
            description: "先读这里，快速判断告警、用量链路或库存哪一项需要你先处理。",
            eyebrow: "运行脉搏",
            inventory: "可用上游库存",
            managedScope: "当前纳管范围",
            openAlerts: "待处理告警",
            stable: "当前无活跃事故",
            title: "运行脉搏",
            usagePipeline: "用量链路"
        },
        poolOverview: {
            title: "账号池总览",
            description: "查看库存、可路由容量、冷却中账号与待删除记录的当前分布。",
            totalLabel: "总计",
            inventory: "库存中",
            inventoryDesc: "已纳入池子但暂未进入可路由集合。",
            routable: "可路由",
            routableDesc: "当前健康并可参与路由的账号。",
            cooling: "冷却中",
            coolingDesc: "正在等待限额恢复或重新探测的账号。",
            pendingDelete: "待删除",
            pendingDeleteDesc: "已进入待删除清理流程的记录。",
            shareOfPool: "账号池占比"
        },
        healthSignals: {
            title: "健康信号",
            description: "按原因分类区分健康容量、限额冷却、瞬时故障、致命问题和人工操作。",
            healthy: "健康",
            healthyDesc: "当前带有健康运行信号的账号。",
            quota: "限额",
            quotaDesc: "因限流或额度耗尽而进入冷却的账号。",
            fatal: "致命",
            fatalDesc: "通常需要恢复或删除决策的致命信号。",
            transient: "瞬时",
            transientDesc: "可以通过重新探测恢复的临时故障。",
            admin: "人工",
            adminDesc: "由运营动作或人工决策导致的状态。"
        },
        tokenComponents: {
            cached: "缓存输入",
            input: "输入",
            output: "输出",
            reasoning: "推理"
        },
        tokenTrend: {
            a11y: {
                summary: "Token 趋势覆盖 {{count}} 个时间点，范围从 {{start}} 到 {{end}}。后附无障碍数据表。",
                summaryEmpty: "当前筛选条件下暂无 Token 趋势数据。",
                tableLabel: "Token 使用趋势数据表",
                timestamp: "时间"
            },
            description: "对比输入、缓存、输出和推理 Token 的时间变化，可按组件开关定位消耗从哪里升高。",
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
        antigravity: {
            loading: "正在加载导入任务…",
            intakeTitle: "导入入口",
            intakeDescription: "在这里选择凭证模式并发起新的文件导入批次。",
            intakeHint: "上传多份 .json/.jsonl 文件后，后端会把它们合并为同一个导入任务，并继续在左侧队列里追踪状态。",
            newImport: "新建导入",
            pause: "暂停",
            recentHint: "集中查看最近追踪的导入任务，并从同一队列恢复已暂停任务或重试失败项。",
            resume: "恢复",
            status: {
                paused: "已暂停"
            }
        },
        admission: {
            ready: "可投入",
            readyDesc: "准入已通过，下一步可以进入运行态。",
            needsRefresh: "待刷新",
            needsRefreshDesc: "已接受，但进入运行态前还需要 refresh。",
            noQuota: "无配额",
            noQuotaDesc: "准入阶段已识别到 quota 或 rate limit 阻塞。",
            failed: "准入失败",
            failedDesc: "准入或激活阶段已经进入终态失败。",
            status: {
                ready: "可投入",
                needsRefresh: "待刷新",
                noQuota: "无配额",
                failed: "失败",
                queued: "排队中",
                unknown: "未知"
            },
            failureStage: {
                admissionProbe: "准入探测",
                activationRefresh: "激活刷新",
                activationRateLimits: "激活配额检查",
                runtimeRefresh: "运行时刷新"
            }
        },
        detail: {
            columns: {
                admission: "准入",
                error: "错误信息",
                failure: "失败阶段",
                label: "标签",
                line: "行号",
                message: "结果说明",
                retry: "重试",
                source: "来源",
                status: "状态"
            },
            filterLabel: "状态筛选",
            description: "查看所选导入任务的准入统计与逐条结果。",
            filters: {
                admission: "准入筛选",
                allAdmissions: "全部准入结果",
                allStatuses: "全部导入状态",
                status: "导入状态"
            },
            itemsEmpty: "没有匹配的任务条目。",
            itemsDescription: "按准入结果、失败阶段和可重试性检查每一条导入记录。",
            itemsLoading: "正在加载任务条目…",
            itemsTitle: "导入条目审计",
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
            missingAccessToken: "缺少 access_token",
            missingCredentials: "缺少凭据字段",
            missingRefreshToken: "缺少 refresh_token",
            oauthProviderNotConfigured: "OAuth 提供方未配置",
            rateLimited: "请求频率受限",
            refreshTokenReused: "refresh_token 已被使用",
            unknown: "未知导入错误",
            upstreamNetworkError: "上游网络错误",
            upstreamUnavailable: "上游服务不可用"
        },
        credentialMode: {
            title: "导入凭证模式",
            description: "选择这一批账号按可轮转的 refresh_token 导入，还是按一次性的 access_token 导入。OAuth 登录导入保持不变。",
            refreshToken: "导入 RT",
            refreshTokenHint: "适合需要平台托管续签和轮转的账号。",
            accessToken: "导入 AK",
            accessTokenHint: "适合只做一次性导入，避免 refresh 轮转压力。"
        },
        metrics: {
            created: "新建",
            createdDesc: "本批次新建的账号池记录数量。",
            failed: "失败",
            failedDesc: "最终停在失败状态的条目数量。",
            processed: "已处理",
            status: "状态",
            throughput: "吞吐",
            throughputDesc: "任务运行期间的近似每分钟导入速度。",
            total: "总数",
            updated: "已存在/已更新",
            updatedDesc: "已存在记录被刷新或合并的数量。"
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
            description: "浏览器会保留最近追踪的任务，点选即可切换批次详情。",
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
            paused: "已暂停",
            processing: "处理中",
            queued: "排队中",
            running: "处理中",
            skipped: "跳过",
            updated: "已存在/已更新"
        },
        subtitleModern: "先批量加入文件，预检查后再一键导入，并实时查看每个任务的进度和错误明细。",
        template: {
            downloadJsonl: "下载模板",
            title: "导入模板",
            desc: "下载我们推荐的模板，以确保列名严格符合系统要求。",
            download: "下载 CSV 模板",
            titleNew: "导入模板",
            descNew: "下载 JSONL 模板，并根据当前选择的凭证模式填写 refresh_token 或 access_token。"
        },
        title: "批量导入任务",
        description: "上传凭据文件、跟踪准入结果，并在同一个工作台审计每一条导入记录。",
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
        wizard: {
            label: "导入向导",
            description: "按步骤完成 OAuth 登录、回调兜底和结果确认，让账号顺畅进入账号池。",
            progress: "流程进度",
            current: "当前步骤",
            completed: "已完成",
            noSession: "当前还没有登录会话，请从第一步开始打开 OAuth 流程。",
            setupHint: "创建会话后会立即打开 OAuth 窗口，并开始轮询回调结果。"
        },
        start: {
            title: "开始 Codex OAuth 登录",
            description: "先创建登录会话，完成 OAuth 授权后自动导入账号。"
        },
        authorize: {
            title: "完成 OAuth 授权",
            description: "打开授权窗口，完成登录后等待回调送达后端。",
            callbackLabel: "回调地址",
            helperTitle: "处理提示",
            helperDescription: "如果浏览器拦截了弹窗，或者自动回调迟迟没有到达，下一块里就可以直接提交手动回调 URL。"
        },
        form: {
            label: "账号标签（可选）",
            labelPlaceholder: "留空将根据邮箱或账号 ID 自动生成",
            baseUrl: "基础 URL",
            priority: "优先级",
            enabled: "导入后立即启用账号"
        },
        monitor: {
            title: "观察换令牌与导入过程",
            description: "后端正在换取凭据，并把账号写入账号池。",
            activeLabel: "导入仍在进行中",
            activeHint: "可以留在此页继续观察，也可以稍后去账号池查看。左侧状态卡会持续自动刷新。"
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
            reviewTitle: "确认导入结果",
            reviewDescription: "确认账号是新建、更新，还是需要重新发起一次导入。",
            chatgptAccountId: "ChatGPT 账号 ID：{{id}}",
            chatgptPlanType: "套餐类型：{{plan}}",
            pending: "流程已经进入最后一步，但后端结果负载还没有完全返回。",
            created: "新建",
            updated: "已存在"
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
            badge: "管理员工作区入口",
            points: {
                audit: "登录、路由调整和高风险操作都能按 request id 回溯。",
                resilience: "查看租户、密钥、用量和账单时，管理链路保持稳定可用。",
                security: "租户边界与凭据控制默认保持生效。"
            },
            subtitle: "面向系统管理员的受控登录入口。",
            title: "登录后即可安心处理 Codex Pool 日常运维"
        },
        messages: {
            failed: "登录失败，请检查账号密码",
            invalidCredentials: "账号或密码不正确，请重新输入。",
            sessionExpired: "登录状态已过期，请重新登录。"
        },
        password: "密码",
        hidePassword: "隐藏密码",
        showPassword: "显示密码",
        passwordPlaceholder: "请输入管理员密码",
        securityHint: "连续登录失败会写入审计日志，方便后续排查。",
        submit: "登录",
        subtitle: "OpenAI 兼容代理 · 账号池化管理台",
        title: "登录",
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
                latency: "延迟",
                path: "路径",
                requestId: "请求 ID",
                serviceTier: "服务层级",
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
        events: {
            title: "统一事件流",
            description: "在一个工作台里查看请求、账号池、巡检、导入、基础设施和管理动作事件。",
            meta: "排查问题时先从 request_id 出发，再向关联事件和详细载荷钻取。",
            summaryTitle: "当前信号窗口",
            summaryDescription: "快速判断控制面此刻流经的运行噪音规模与重点类别。",
            tableTitle: "事件流",
            tableDescription: "按分类和严重级别筛选事件，再查看载荷与关联链。",
            searchPlaceholder: "搜索事件、请求 ID、账号、原因…",
            empty: "当前时间窗口内没有事件。",
            categories: {
                request: "请求",
                accountPool: "账号池",
                patrol: "巡检",
                import: "导入",
                infra: "基础设施",
                adminAction: "管理操作",
                unknown: "未知"
            },
            severities: {
                debug: "调试",
                info: "信息",
                warn: "警告",
                error: "错误",
                unknown: "未知"
            },
            metrics: {
                total: "事件数",
                totalDesc: "统一收录的请求、账号池、巡检、导入、基础设施和管理事件。",
                error: "错误",
                errorDesc: "当前窗口内严重级别为 error 的事件数。",
                accountPool: "账号池",
                accountPoolDesc: "状态迁移、恢复、删除与人工运营动作。",
                request: "请求",
                requestDesc: "当前窗口内属于 request 分类的事件数。"
            },
            columns: {
                time: "时间",
                category: "分类",
                severity: "级别",
                event: "事件",
                context: "上下文"
            },
            actions: {
                inspect: "查看详情"
            },
            filters: {
                category: "分类",
                range: "时间范围",
                severity: "严重级别",
                allCategories: "全部分类",
                allSeverities: "全部级别"
            },
            fields: {
                requestId: "请求 ID",
                account: "账号",
                reasonCode: "原因码",
                time: "时间",
                path: "路径",
                method: "方法",
                jobId: "任务 ID",
                model: "模型",
                tenant: "租户",
                authProvider: "鉴权方式",
                reasonClass: "原因分类",
                routeDecision: "路由决策",
                nextActionAt: "下一步动作",
                statusCode: "状态码",
                upstreamStatusCode: "上游状态码",
                latency: "延迟"
            },
            eventTypes: {
                requestReceived: "收到请求",
                requestCompleted: "请求完成",
                requestFailed: "请求失败",
                routingCandidateSelected: "已选择路由候选",
                sameAccountRetry: "同账号重试",
                crossAccountFailover: "跨账号故障切换",
                continuationCursorSaved: "已保存 continuation cursor",
                continuationCursorRestored: "已恢复 continuation cursor",
                wsHttpFallback: "WebSocket 已回退到 HTTP",
                proxySelectionFailed: "代理选择失败",
                probeSucceeded: "探测成功",
                activePatrolBatchCompleted: "主动巡检批次完成",
                rateLimitRefreshBatchCompleted: "用量刷新批次完成",
                pendingDeleteBatchCompleted: "待删除清理批次完成",
                accountPoolStateTransition: "账号池状态已变更",
                accountDeleted: "账号已删除",
                importJobCreated: "导入任务已创建",
                importJobCompleted: "导入任务已完成",
                importJobFailed: "导入任务失败",
                upstreamEvent: "上游事件",
                unknown: "未知事件"
            },
            reasonCodes: {
                requestReceived: "请求已进入系统",
                routingCandidateSelected: "已选定候选路由",
                rateLimited: "触发速率限制",
                transportError: "网络传输异常",
                proxyUnavailable: "代理不可用",
                continuationCursorRestored: "continuation cursor 已恢复",
                rateLimitRefreshBatchCompleted: "用量刷新批次完成",
                pendingDeleteBatchCompleted: "待删除清理批次完成",
                accountDeactivated: "账号已停用",
                previousResponseNotFound: "未找到上一个响应",
                upstreamRequestFailed: "上游请求失败",
                invalidRefreshToken: "Refresh Token 无效",
                refreshTokenReused: "Refresh Token 已复用",
                upstreamUnavailable: "上游不可用",
                unknown: "未知原因"
            },
            reasonClasses: {
                healthy: "健康",
                quota: "限额",
                fatal: "致命",
                transient: "瞬时",
                admin: "人工",
                unknown: "未知分类"
            },
            routingDecisions: {
                recentSuccess: "优先最近成功账号",
                freshProbe: "优先新鲜探测结果",
                roundRobin: "轮询分配",
                sameAccountRetry: "同账号重试",
                crossAccountFailover: "跨账号故障切换",
                requestReceived: "请求刚进入系统",
                unknown: "未知决策"
            },
            authProviders: {
                oauthRefreshToken: "OAuth Refresh Token",
                legacyBearer: "Legacy Bearer",
                codexOauth: "Codex OAuth",
                unknown: "未知鉴权方式"
            },
            insightsTitle: "事件聚焦",
            insightsDescription: "先看最常见的事件类型和原因码，再决定是追 request_id 还是直接打开详情。",
            topEventTypes: "高频事件类型",
            topReasons: "高频原因码",
            noInsights: "当前筛选条件下还没有足够的事件摘要。",
            previewTitle: "摘要预览",
            detailTitle: "事件详情",
            detailDescription: "查看事件原始载荷、消息摘要与关联链。",
            detailDescriptionWithRequest: "请求 ID：{{requestId}}",
            payloadTitle: "载荷 JSON",
            payloadDescription: "统一事件流中记录的原始载荷。",
            correlationTitle: "关联链",
            correlationDescription: "同一个 request_id 下的全部事件。"
        },
        search: "搜索载荷或服务名…",
        subtitle: "实时的审计追踪与运行时上下文。",
        time: {
            displayMode: "当前按本地时间（{{timezone}}）显示，悬浮提示和导出中保留 UTC 原值。",
            tooltip: "本地时间：{{local}} | UTC：{{utc}}"
        },
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
        },
        antigravity: {
            stream: "流式"
        }
    },
    models: {
        actions: {
            copyModelId: "复制模型 ID",
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
            providerLabel: "提供商筛选",
            availabilityLabel: "可用性筛选",
            allAvailability: "全部可用性"
        },
        antigravity: {
            notAvailable: "暂无",
            summaryTitle: "目录总览",
            summaryDescription: "从可用性、提供商和目录覆盖面上快速判断当前模型池状态。",
            catalogTitle: "探测与同步状态",
            catalogDescription: "这里集中展示 probe cache、新鲜度与官方目录同步状态。",
            maintenance: "目录维护",
            maintenanceProbeDescription: "立即刷新当前账号池的模型可用性探测。",
            maintenanceSyncDescription: "重新同步官方模型目录与元数据快照。",
            cacheFresh: "探测缓存新鲜",
            cacheStale: "探测缓存过期",
            catalogNeedsSync: "目录需要同步",
            catalogReady: "目录已同步",
            cacheUpdatedAt: "缓存更新时间",
            probeSource: "探测来源账号",
            catalogSyncedAt: "目录同步时间",
            cacheTtl: "缓存时长",
            cacheTtlHours: "{{hours}} 小时",
            catalogLastError: "最近目录错误",
            catalogAttentionTitle: "需要运维关注",
            catalogAttentionSyncRequired: "官方目录还没有完成最新同步。请执行目录同步，再检查模型目录是否恢复完整。",
            catalogAttentionRetry: "最近一次目录同步或探测没有完成。当前页面保留了状态摘要，详细技术原因请到统一事件流继续排查。",
            catalogAttentionCacheStale: "探测缓存已经过期。建议重新执行一次可用性探测，避免按过期结果判断模型状态。",
            directoryTitle: "模型目录",
            directoryDescription: "按提供商和可用性筛选模型，再查看定价与官方元数据。",
            noDescription: "暂无官方描述。",
            ungroupedFamily: "未分组模型",
            deprecated: "已弃用",
            copyModelIdSuccess: "已复制模型 ID：{{modelId}}",
            copyModelIdFailed: "复制模型 ID 失败。",
            effectivePricingSource: "实际定价来源",
            officialPageStatus: "官方目录页",
            officialPageReady: "可打开",
            officialPageMissing: "未提供",
            emptyFilteredTitle: "当前筛选下没有匹配模型",
            emptyFilteredDescription: "调整提供商、可用性或搜索关键词后，再继续查看模型目录。",
            noSectionItems: "当前没有可展示的官方结构化信息。",
            availabilityOutcomeLabel: "最近探测结果",
            availabilityOutcome: {
                available: "最近一次探测显示可用。",
                unavailable: "最近一次探测显示暂不可用，请结合事件流继续排查。",
                unavailableWithStatus: "最近一次探测返回 HTTP {{status}}，当前暂不可用。",
                unknown: "当前还没有稳定的探测结论。"
            },
            metrics: {
                total: "模型总数",
                totalDesc: "当前模型池暴露和映射的目录总量。",
                available: "可用模型",
                availableDesc: "最近探测返回可用的模型。",
                unavailable: "不可用模型",
                unavailableDesc: "最近探测仍存在错误或不可达的模型。",
                providers: "提供商数",
                providersDesc: "当前目录覆盖的 provider / hub 数量。"
            },
            sections: {
                operational: "运行状态",
                pricing: "价格快照",
                identity: "模型身份",
                limits: "上下文与限制",
                modalities: "模态",
                endpoints: "端点",
                features: "能力特性",
                tools: "工具",
                snapshots: "快照",
                pricingNotes: "价格说明"
            }
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
                probeOnly: "仅探测结果",
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
            maxInputTokens: "最大输入 Token",
            capabilityDetails: {
                supported: "支持",
                notSupported: "不支持",
                inputAndOutput: "输入与输出",
                inputOnly: "仅输入",
                outputOnly: "仅输出"
            },
            rawText: "官方文本快照",
        }
    },
    costReports: {
        admin: {
            title: "成本报表",
            description: "最近 30 天的只读美元消耗估算。"
        },
        tenant: {
            title: "用量成本报表",
            description: "当前租户的只读美元消耗估算。"
        },
        summary: {
            totalCost: "预估成本",
            totalRequests: "总请求数",
            avgCostPerRequest: "平均单次请求成本"
        },
        chart: {
            title: "成本趋势",
            description: "基于请求日志聚合的美元消耗估算。",
            empty: "当前范围内暂无成本数据。",
            series: {
                cost: "预估成本"
            }
        },
        logs: {
            title: "请求日志",
            empty: "当前范围内暂无请求日志。",
            searchPlaceholder: "搜索请求 ID、模型、租户或状态",
            columns: {
                time: "时间",
                tenant: "租户",
                requestId: "请求 ID",
                model: "模型",
                status: "状态",
                cost: "预估成本"
            }
        },
        filters: {
            tenant: "租户",
            allTenants: "全部租户",
            apiKey: "API Key",
            allApiKeys: "全部 API Key",
            day: "按日",
            month: "按月"
        }
    },
    nav: {
        accounts: "账号池",
        modelRouting: "模型调度",
        apiKeyGroups: "分组管理",
        apiKeys: "密钥池",
        billing: "计费",
        config: "全局配置",
        dashboard: "服务总览",
        inventory: "库存",
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
    accountPool: {
        title: "账号池",
        subtitle: "用统一的四态模型查看库存、可路由、冷却中和待删除账号。",
        meta: "当前正式运营状态为 inventory / routable / cooling / pending_delete。",
        empty: "当前筛选下没有账号池记录。",
        searchPlaceholder: "搜索邮箱、标签、账号 ID、原因…",
        filters: {
            state: "状态",
            scope: "范围",
            reasonClass: "原因分类",
            allStates: "全部状态",
            allScopes: "全部范围",
            allReasons: "全部原因分类"
        },
        state: {
            inventory: "库存中",
            routable: "可路由",
            cooling: "冷却中",
            pendingDelete: "待删除",
            unknown: "未知状态"
        },
        scope: {
            runtime: "运行池",
            inventory: "库存"
        },
        reasonClass: {
            healthy: "健康",
            quota: "限额",
            fatal: "致命",
            transient: "瞬时",
            admin: "人工",
            unknown: "未知原因分类"
        },
        refreshState: {
            healthy: "健康",
            degraded: "降级",
            missing: "缺失",
            invalid: "无效",
            unknown: "未知"
        },
        healthFreshness: {
            fresh: "新鲜",
            stale: "过期",
            unknown: "未知"
        },
        probeOutcome: {
            ok: "通过",
            quota: "限额",
            transient: "瞬时异常",
            fatal: "致命异常",
            unknown: "未探测"
        },
        signalSource: {
            active: "主动巡检",
            passive: "用户请求",
            unknown: "未知"
        },
        reasonCode: {
            none: "暂无阻断原因",
            tokenInvalidated: "令牌已失效",
            accountDeactivated: "账号已停用",
            invalidRefreshToken: "Refresh Token 无效",
            refreshTokenRevoked: "Refresh Token 已撤销",
            refreshTokenReused: "Refresh Token 已复用",
            rateLimited: "触发速率限制",
            quotaExhausted: "用量已耗尽",
            upstreamUnavailable: "上游不可用",
            transportError: "网络传输异常",
            overloaded: "上游过载",
            operatorRetiredInvalidRefreshToken: "账号因无效 Refresh Token 被运营下线",
            unknown: "未知阻断原因"
        },
        routeEligible: {
            yes: "可参与路由",
            no: "不可路由"
        },
        actions: {
            inspect: "查看",
            more: "更多操作",
            reprobe: "重新探测",
            restore: "恢复",
            delete: "删除"
        },
        columns: {
            account: "账号",
            state: "状态",
            reason: "原因",
            credentials: "凭证",
            operationalStatus: "运营状态",
            quota: "用量",
            updatedAt: "更新时间",
            recentSignal: "最近信号",
            actions: "操作"
        },
        metrics: {
            inventory: "库存中",
            routable: "可路由",
            cooling: "冷却中",
            pendingDelete: "待删除",
            records: "{{count}} 条记录",
            totalRecords: "总记录数",
            filteredRecords: "筛选后记录数",
            inventoryDesc: "仍在库存阶段，尚未进入可路由集合。",
            routableDesc: "当前健康且可参与路由的记录。",
            coolingDesc: "因限额或瞬时问题处于冷却窗口的记录。",
            pendingDeleteDesc: "已进入待删除清理阶段的记录。"
        },
        rateLimits: {
            defaultLabel: "额度",
            empty: "暂无额度快照",
            unavailable: "不可用",
            fiveHoursShort: "5h",
            oneWeekShort: "7d",
            githubShort: "GitHub"
        },
        recentSignal: {
            updatedFallback: "最近更新",
            window12h: "近 12h",
            window24h: "近 24h",
            active: "活跃",
            sparse: "稀疏",
            silent: "静默",
            busy: "高频",
            noHeatmap: "暂无可展示的信号热图",
            summaryWithDetail: "{{relative}} · {{detail}}",
            bucketTooltip: "{{time}} · {{count}} 条信号 · 主动 {{active}} / 被动 {{passive}}",
            legend: {
                success: "成功",
                mixed: "部分失败",
                error: "全部失败",
                request: "用户请求",
                patrol: "主动巡检"
            }
        },
        cooling: {
            imminent: "即将恢复",
            thawIn: "{{hours}}h {{minutes}}m 后解冻"
        },
        sections: {
            stateOverviewTitle: "状态总览",
            stateOverviewDescription: "先看库存、可路由、冷却中与待删除的分布。",
            reasonOverviewTitle: "原因分类",
            reasonOverviewDescription: "再区分健康、限额、致命、瞬时与人工原因。",
            recordsTitle: "账号池记录",
            recordsDescription: "按状态、范围和原因分类筛选记录并执行动作。"
        },
        detail: {
            modalTitle: "账号池记录详情",
            description: "查看选中记录的当前运营状态、凭证和用量摘要。",
            empty: "暂未加载到该记录详情。",
            sections: {
                status: "运营状态",
                profile: "账号画像",
                recentSignal: "最近信号热图",
                credentials: "凭证状态",
                quota: "用量概览",
                realTest: "真实测试"
            },
            test: {
                description: "对当前 runtime 账号发起一次真实的 Responses 会话请求，结果按用户请求记账，不计入主动巡检。",
                runtimeOnly: "首版真实测试仅支持 runtime 记录",
                modelLabel: "测试模型",
                promptLabel: "测试输入",
                promptPlaceholder: "输入要发送给该账号的 prompt…",
                send: "发送测试",
                clearSession: "清空会话",
                noModels: "暂无可选模型",
                emptyAssistant: "本次响应未返回可展示的文本内容。",
                assistantLabel: "助手",
                userLabel: "用户",
                requestId: "请求 ID",
                responseId: "响应 ID",
                latency: "耗时",
                rawError: "原始报错",
                statusCode: "状态码",
                errorCode: "错误码",
                inputTokens: "输入 tokens",
                cachedInputTokens: "缓存输入 tokens",
                outputTokens: "输出 tokens",
                reasoningTokens: "推理 tokens"
            }
        },
        fields: {
            nextAction: "下一步动作",
            routeEligible: "路由资格",
            healthFreshness: "健康新鲜度",
            lastSignalAt: "最近信号",
            lastSignalSource: "信号来源",
            lastProbeAt: "最近探测时间",
            lastProbeOutcome: "最近探测结果",
            updatedAt: "更新时间",
            createdAt: "创建时间",
            email: "邮箱",
            chatgptAccountId: "ChatGPT 账号 ID",
            plan: "套餐",
            sourceType: "来源类型",
            recordScope: "记录范围",
            mode: "模式",
            authProvider: "认证提供方",
            credentialKind: "凭证类型",
            refreshState: "刷新凭证状态",
            reasonCode: "原因码",
            hasRefreshCredential: "持有 Refresh 凭证",
            accessTokenFallback: "保留 Access Token 回退",
            rateLimitsFetchedAt: "用量快照时间"
        },
        messages: {
            confirmDeleteTitle: "确认删除 {{label}}？",
            confirmDeleteDescription: "删除后该记录会从账号池移除。",
            actionSuccessTitle: "已完成{{action}}",
            actionSuccessDescription: "{{label}} 已更新。",
            actionPartialTitle: "{{action}}部分失败",
            actionFailedTitle: "{{action}}失败",
            actionFailed: "操作失败，请稍后重试。"
        }
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
            invalidProxyUrl: "代理 URL 无效。",
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
        actions: {
            add: "新增代理",
            delete: "删除",
            edit: "编辑",
            test: "测试",
            testAll: "测试全部"
        },
        antigravity: {
            auth: "鉴权",
            authConfigured: "已配置",
            lastErrorSummary: "最近一次探测失败，请到统一事件流继续排查。",
            authNone: "无",
            emptyDescription: "添加第一个代理节点后，系统才能通过管理员维护的代理池转发流量。",
            latency: "延迟",
            scheme: "协议"
        },
        badges: {
            auth: "带鉴权"
        },
        columns: {
            actions: "操作",
            lastTest: "最近测试",
            latency: "延迟",
            proxy: "代理节点",
            status: "状态",
            weight: "权重"
        },
        deleteDialog: {
            confirm: "删除代理",
            description: "确定要从全局出站代理池中删除 {{label}} 吗？现有请求会在下一次刷新后停止使用它。",
            title: "删除代理"
        },
        editor: {
            create: "创建代理",
            createTitle: "新增出站代理",
            description: "配置一个全局出站代理节点。编辑时如果留空代理 URL，则继续沿用当前密钥与凭证。",
            editTitle: "编辑出站代理",
            enabledHint: "禁用后的节点仍会保留在列表中，但不会参与选路，也不会自动测试。",
            errors: {
                labelRequired: "请输入代理标签。",
                proxyUrlRequired: "请输入代理 URL。",
                weightInvalid: "权重必须大于 0。"
            },
            fields: {
                enabled: "启用节点",
                label: "标签",
                proxyUrl: "代理 URL",
                weight: "权重"
            },
            proxyUrlHint: "支持 http://、https://、socks5://。必须包含主机和端口，用户名密码可直接写在 URL 中。",
            proxyUrlPlaceholder: "http://user:password@127.0.0.1:6152",
            save: "保存修改"
        },
        empty: "还没有配置任何出站代理。",
        failModeDescriptions: {
            allowDirectFallback: "当所有健康代理都失败时，平台可以回退为直连。",
            strictProxy: "当没有可用健康代理时，请求会立即失败，不会绕过代理池。"
        },
        failModes: {
            allowDirectFallback: "允许直连回退",
            strictProxy: "严格走代理"
        },
        filters: {
            all: "全部节点",
            degraded: "降级",
            disabled: "已禁用",
            healthy: "健康",
            label: "状态筛选",
            offline: "离线"
        },
        health: {
            degraded: "降级",
            disabled: "已禁用",
            healthy: "健康",
            offline: "离线"
        },
        list: {
            description: "在这里增删改测加权代理节点。管理端会保存明文密钥，但对外只返回脱敏后的 URL。",
            title: "代理节点列表"
        },
        loading: "正在加载出站代理池…",
        meta: {
            enabled: "{{count}} 个启用",
            healthy: "{{count}} 个健康",
            total: "{{count}} 个节点"
        },
        notifications: {
            nodeCreateFailedTitle: "创建代理失败",
            nodeCreatedDescription: "该代理节点已加入全局代理池。",
            nodeCreatedTitle: "代理已创建",
            nodeDeleteFailedTitle: "删除代理失败",
            nodeDeletedDescription: "该代理节点已从全局代理池移除。",
            nodeDeletedTitle: "代理已删除",
            nodeUpdateFailedTitle: "更新代理失败",
            nodeUpdatedDescription: "该代理节点已更新。",
            nodeUpdatedTitle: "代理已更新",
            settingsFailedTitle: "保存代理设置失败",
            settingsSavedDescription: "全局出站代理池设置已保存。",
            settingsSavedTitle: "代理设置已保存",
            singleTestCompletedDescription: "单节点代理测试已完成。",
            testCompletedDescription: "已完成 {{count}} 个代理节点的测试。",
            testCompletedTitle: "代理测试完成",
            testFailedTitle: "代理测试失败",
            validationFailedTitle: "请检查代理表单"
        },
        pending: "尚未测试",
        searchPlaceholder: "搜索标签、脱敏 URL 或最近错误…",
        settings: {
            description: "这些设置会作用于平台内所有外部 HTTP 与 WebSocket 出站请求。",
            enabled: "启用出站代理池",
            enabledHint: "关闭后，所有外部流量保持直连；开启后，流量会从下方的加权代理池中选择。",
            failMode: "失败策略",
            save: "保存设置",
            title: "全局代理池设置"
        },
        stats: {
            enabled: "已启用节点",
            healthy: "健康节点",
            total: "总节点数"
        },
        subtitle: "为所有上游流量配置统一的全局出站代理池。这一页已经不再是旧的数据面节点占位页。",
        title: "出站代理池"
    },
    system: {
        antigravity: {
            billingMode: {
                costReportOnly: "仅成本报表",
                creditEnforced: "积分强校验"
            },
            capabilitiesDescription: "把 edition 能力边界和租户相关开关放在同一块里查看。",
            capabilitiesTitle: "能力边界",
            category: {
                config: "配置",
                diagnostics: "诊断",
                runtime: "运行时"
            },
            componentsDescription: "确认控制面、数据面和用量仓库都在按预期响应。",
            componentsTitle: "核心组件",
            config: {
                authValidateUrl: "鉴权校验地址",
                controlPlaneListen: "控制面监听地址",
                dataPlaneUrl: "数据面地址",
                oauthRefresh: "OAuth 自动刷新",
                refreshInterval: "刷新间隔"
            },
            counts: {
                apiKeys: "API 密钥数",
                enabledAccounts: "启用账号数",
                oauthAccounts: "OAuth 账号数",
                tenants: "租户数",
                totalAccounts: "账号总数"
            },
            dataPlane: {
                accounts: "账号数",
                active: "活跃"
            },
            dataPlaneIssue: "数据面运行诊断报告当前存在异常。",
            debug: {
                authValidatorEnabled: "鉴权校验器",
                billingReconcileAdjust: "计费对账调账数",
                billingReconcileFailed: "计费对账失败数",
                billingReconcileReleased: "计费对账释放数",
                billingReconcileScanned: "计费对账扫描数",
                failoverEnabled: "故障切换开关",
                quickRetryMax: "快速重试上限",
                requestFailoverWait: "切号等待时间",
                retryPollInterval: "重试轮询间隔",
                sharedRoutingCache: "共享路由缓存",
                snapshotRevision: "快照版本"
            },
            debugSignals: "调试信号",
            debugSignalsDescription: "把当前自动切号、鉴权校验、缓存与计费对账信号集中在一个视图里。",
            disabled: "已关闭",
            edition: {
                business: "企业版",
                personal: "个人版",
                team: "团队版"
            },
            enabled: "已开启",
            features: {
                costReports: "成本报表",
                costReportsHint: "适合 personal 和 team 的成本可见性模式。",
                creditBilling: "信用计费",
                creditBillingHint: "启用后才会进入积分强约束与预授权链路。",
                multiTenant: "多租户",
                multiTenantHint: "控制面是否按租户维度隔离资源与工作流。",
                tenantPortal: "租户门户",
                tenantPortalHint: "是否提供 tenant 自助工作台入口。",
                tenantRecharge: "租户充值",
                tenantRechargeHint: "是否允许租户侧自助充值与余额管理。",
                tenantSelfService: "租户自助",
                tenantSelfServiceHint: "是否开放注册、密码重置等租户自助流程。"
            },
            generatedAtHint: "最近一次运行时快照生成时间",
            loading: "正在加载系统状态…",
            milliseconds: "{{value}}ms",
            runtimeConfig: "运行时配置",
            runtimeConfigDescription: "这里聚合了当前实例真正生效的控制面、数据面与 OAuth 刷新配置。",
            runtimeCounts: "运行时计数",
            runtimeCountsDescription: "快速判断当前实例承载的账号、密钥和租户规模。",
            seconds: "{{value}} 秒",
            summary: {
                billingMode: "计费模式",
                billingModeHint: "当前生效的计费契约模式",
                edition: "产品层级",
                editionHint: "当前运行版本及能力边界",
                generatedAt: "生成时间",
                generatedAtHint: "最近一次运行时快照生成时间",
                uptimeHint: "控制面连续运行时长"
            }
        },
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
                failoverWait: "切号等待 {{value}}ms",
                quickRetry: "同账号快速重试 ≤ {{value}}",
                retryPoll: "轮询间隔 {{value}}ms",
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
                    audit: "当团队需要回溯时，用量、计费和策略变更都有据可查。",
                    resilience: "上游波动时，具备故障切换感知的路由会尽量保持租户请求可用。",
                    security: "密钥、会话和账号访问始终按租户隔离。"
                },
                subtitle: "登录一次，即可在同一安全工作区完成日常租户运营。",
                title: "一个租户工作区，处理日常用量、账单与密钥"
            },
            error: {
                invalidCredentialsOrUnverified: "登录失败。请检查邮箱和密码；如果是首次登录，请先完成邮箱验证。",
                loginFailed: "登录失败，请稍后重试。",
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
                drawerHint: "先向该邮箱发送重置码。收到后，在下方输入重置码和新密码。",
                stepResetPassword: "设置新密码",
                stepSendCode: "发送验证码"
            },
            notice: {
                emailVerified: "邮箱验证成功。请使用此账号登录。",
                loginSuccess: "登录成功。",
                passwordResetSuccess: "密码重置成功。请重新登录。",
                registerDebugCode: "注册成功，验证码（调试）：{{code}}",
                registerSuccess: "注册已完成。请输入邮件里的验证码以激活账号。",
                resetCodeDebug: "密码重置代码（调试）：{{code}}",
                resetCodeSentIfExists: "如果该邮箱存在，我们会尽快发送重置码。",
                sessionExpired: "租户会话已过期。请重新登录。",
                verifyCodeHint: "还没收到验证码？请等待 60 秒后再次发送。"
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
                authSubtitle: "在同一块工作区里选择登录或注册，然后继续完成后续操作。",
                forgotPasswordTitle: "重置密码",
                forgotPasswordSubtitle: "先申请重置码，再在当前流程里设置新密码。",
                loginTitle: "租户登录",
                registerTitle: "租户注册",
                verifyEmailSubtitle: "输入邮件中的验证码，完成激活后返回登录。",
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
            usage: "用量"
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
                serviceTier: "服务层级：{{tier}}",
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
    modelRouting: {
        actions: {
            add: "新增规则",
            delete: "删除",
            edit: "编辑规则"
        },
        columns: {
            actions: "操作",
            exactModels: "精确模型",
            fallbackProfiles: "回退配置",
            family: "模型族",
            modelPrefixes: "前缀",
            name: "策略",
            priority: "优先级",
            status: "状态"
        },
        empty: "后端暂未返回任何路由策略",
        loading: "正在加载路由策略…",
        searchPlaceholder: "搜索策略…",
        status: {
            disabled: "已禁用",
            enabled: "已启用"
        },
        subtitle: "这里展示的 profiles 与 policies 已对齐到真实的管理员路由契约。",
        summary: {
            enabledProfiles: "启用中的配置",
            enabledProfilesHint: "当前仍可参与选路的配置数",
            profiles: "配置总数",
            profilesHint: "后端返回的路由配置档案",
            selectedAccounts: "显式选定账号",
            selectedAccountsHint: "所有配置中显式绑定的账号总数"
        },
        title: "模型路由"
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
            updated: "最近更新",
            usage: "使用情况"
        },
        editor: {
            createTitle: "新建分组",
            description: "配置分组倍率以及模型级价格覆盖。",
            editTitle: "编辑分组",
            groupSettingsTitle: "分组设置",
            groupSettingsDescription: "维护分组名称、倍率和默认行为。"
        },
        empty: "暂无分组",
        antigravity: {
            allowAllModels: "允许全部目录模型",
            catalogCount: "目录 {{count}} 项",
            coverage: "覆盖 {{coverage}}",
            listTitle: "分组目录",
            listDescription: "用统一账号分组策略组织 API Key、模型覆盖范围和计费倍率。",
            metrics: {
                total: "分组总数",
                totalDesc: "当前工作区内可管理的 API Key 分组总量。",
                enabled: "启用中分组",
                enabledDesc: "当前仍可供运营使用的活跃分组数量。",
                defaults: "默认分组",
                defaultsDesc: "作为默认承接策略生效的分组数量。",
                catalog: "目录模型数",
                catalogDesc: "可供分组策略选择的统一模型目录总量。"
            },
            scopedPolicy: "限定模型策略",
            serviceUnavailableDescription: "当前运行形态没有提供 API Key 分组存储，因此这一页暂时不可用。",
            updatedAt: "更新于 {{value}}"
        },
        filters: {
            statusLabel: "状态筛选",
            all: "全部分组",
            enabled: "已启用",
            disabled: "已禁用",
            deleted: "已删除"
        },
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
            basePricingSummary: "{{provider}} · {{title}} · 基准价：输入 {{input}} · 缓存 {{cached}} · 输出 {{output}}",
            columns: {
                finalPrice: "最终价格",
                formulaPrice: "公式价格",
                mode: "模式",
                model: "模型"
            },
            description: "展示当前分组下对租户可见的最终价格。",
            empty: "尚未配置任何模型。",
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
    modelRoutingPage: {
        title: "模型调度",
        subtitle: "配置路由画像、模型感知回退链，以及用于模型调度规划的规划模型链。",
        actions: {
            refresh: "刷新",
            createProfile: "新建画像",
            createPolicy: "新建策略",
            edit: "编辑",
            delete: "删除",
            deleteProfile: "删除画像",
            deletePolicy: "删除策略",
            saveSettings: "保存设置",
            saveProfile: "保存画像",
            savePolicy: "保存策略"
        },
        settings: {
            title: "模型调度设置",
            description: "控制模型调度的发布行为、安全开关，以及规划模型链配置。",
            enabled: "启用模型调度",
            enabledHint: "关闭后，编译路由计划只会保留人工配置路径。",
            autoPublish: "自动发布",
            autoPublishHint: "每次重算完成后自动发布最新路由计划。",
            killSwitch: "全局熔断",
            killSwitchHint: "立即停止规划器驱动的变更，但保留已保存的配置。",
            triggerMode: "触发模式",
            plannerModelChain: "规划模型链",
            plannerModelChainPlaceholder: "gpt-5.2-codex, gpt-4.1-mini",
            plannerModelChainHint: "从模型池中选择规划兜底模型，并按从高优先级到低优先级保留顺序。",
            updatedAt: "更新时间 {{value}}"
        },
        versions: {
            title: "已发布版本",
            description: "最近编译完成、可供 data-plane 消费的路由计划。",
            empty: "暂未发布任何路由计划版本。",
            noReason: "未记录发布原因。",
            defaultSegments: "默认分段 {{count}}",
            policyCount: "策略 {{count}} 条",
            showMore: "展开另外 {{count}} 个旧版本",
            showLess: "收起旧版本"
        },
        profiles: {
            title: "路由画像",
            description: "可复用的账号选择器，用来描述哪些 plan、模式和鉴权方式可以承接请求。",
            empty: "暂无路由画像。",
            summary: "计划 {{plans}} 项 · 模式 {{modes}} 项 · 鉴权 {{authProviders}} 项 · 强制包含 {{include}} 项 · 强制排除 {{exclude}} 项",
            anyMode: "全部账号模式"
        },
        policies: {
            title: "模型策略",
            description: "将模型族或精确模型 ID 匹配到一条显式的画像回退链。",
            empty: "暂无模型路由策略。",
            summary: "精确模型 {{exact}} 个 · 前缀 {{prefixes}} 个 · 回退画像 {{fallbacks}} 个",
            fallbackChain: "回退链：{{value}}",
            moreExactModels: "还有 {{count}} 个"
        },
        dialogs: {
            createProfile: "新建路由画像",
            editProfile: "编辑路由画像",
            profileDescription: "为一类上游账号组合可复用的筛选条件。",
            createPolicy: "新建模型策略",
            editPolicy: "编辑模型策略",
            policyDescription: "定义模型族如何映射到有序的路由画像链路。"
        },
        form: {
            name: "名称",
            description: "描述",
            priority: "优先级",
            enabled: "启用",
            enabledHint: "只有启用的画像才会进入编译后的路由计划。",
            policyEnabledHint: "禁用后会保留策略，但不会参与实际调度。",
            planTypes: "计划类型",
            planTypesPlaceholder: "free, plus, team",
            modes: "账号模式",
            authProviders: "鉴权方式",
            includeAccounts: "强制包含账号 ID",
            includeAccountsPlaceholder: "uuid-1, uuid-2",
            excludeAccounts: "强制排除账号 ID",
            excludeAccountsPlaceholder: "uuid-3, uuid-4",
            family: "模型族",
            familyPlaceholder: "gpt-5",
            exactModels: "精确模型",
            exactModelsPlaceholder: "gpt-5.4, gpt-5.2-codex",
            exactModelsHint: "从模型池中选择精确模型 ID，配置时可同时看到可用状态和价格摘要。",
            modelPrefixes: "模型前缀",
            modelPrefixesPlaceholder: "gpt-5, o3",
            modelPrefixesHint: "这里保留自由输入，用于像 gpt-5 或 o3 这样的前缀匹配规则。",
            fallbackProfiles: "回退画像",
            noProfilesAvailable: "请先创建路由画像，再为策略配置回退链。"
        },
        messages: {
            settingsSaved: "模型调度设置已保存。",
            settingsSaveFailed: "保存模型调度设置失败。",
            errorLearningSettingsSaved: "上游错误学习设置已保存。",
            errorLearningSettingsSaveFailed: "保存上游错误学习设置失败。",
            profileSaved: "路由画像已保存：{{name}}",
            profileSaveFailed: "保存路由画像失败。",
            profileDeleted: "路由画像已删除。",
            profileDeleteFailed: "删除路由画像失败。",
            policySaved: "模型路由策略已保存：{{name}}",
            policySaveFailed: "保存模型路由策略失败。",
            policyDeleted: "模型路由策略已删除。",
            policyDeleteFailed: "删除模型路由策略失败。",
            templateSaved: "上游错误模板已保存。",
            templateSaveFailed: "保存上游错误模板失败。",
            templateApproved: "上游错误模板已批准。",
            templateApproveFailed: "批准上游错误模板失败。",
            templateRejected: "上游错误模板已拒绝。",
            templateRejectFailed: "拒绝上游错误模板失败。",
            templateRewritten: "已使用 AI 重写上游错误模板。",
            templateRewriteFailed: "AI 重写上游错误模板失败。",
            builtinTemplateSaved: "内置错误模板已保存。",
            builtinTemplateSaveFailed: "保存内置错误模板失败。",
            builtinTemplateRewritten: "已使用 AI 重写内置错误模板。",
            builtinTemplateRewriteFailed: "AI 重写内置错误模板失败。",
            builtinTemplateReset: "内置错误模板已恢复默认。",
            builtinTemplateResetFailed: "恢复内置错误模板默认值失败。"
        },
        status: {
            enabled: "已启用",
            disabled: "已禁用",
            killSwitchOn: "熔断已开启",
            autoPublishOn: "自动发布已开启",
            autoPublishOff: "自动发布已关闭"
        },
        triggerModes: {
            hybrid: "混合触发",
            scheduledOnly: "仅定时",
            eventOnly: "仅事件"
        },
        modes: {
            apiKey: "API Key",
            chatGptSession: "ChatGPT 会话",
            codexOauth: "Codex OAuth"
        },
        authProviders: {
            legacyBearer: "传统 Bearer",
            oauthRefreshToken: "OAuth Refresh Token"
        },
        modelSelector: {
            addModel: "添加模型",
            searchPlaceholder: "按模型 ID 或标题搜索",
            emptyCatalog: "模型池里还没有可选模型。",
            emptySelection: "暂未选择任何模型。",
            noMatches: "没有匹配的模型。",
            unknownModel: "已保存但不在模型池中",
            moveUp: "上移",
            moveDown: "下移",
            remove: "移除模型"
        },
        errorLearning: {
            settings: {
                title: "上游错误学习",
                description: "审核首次发现的上游失败模板，在固化为确定性错误规则前先进行把关。",
                enabled: "启用上游错误学习",
                enabledHint: "关闭后，未知上游错误会直接回落到通用本地化错误文案。",
                firstSeenTimeoutMs: "首次发现超时（毫秒）",
                firstSeenTimeoutMsHint: "首次生成临时模板时，允许同步等待的最长时间。",
                reviewHitThreshold: "进入审核阈值",
                reviewHitThresholdHint: "临时模板累计命中达到该次数后转入审核队列。",
                updatedAt: "更新时间 {{value}}"
            },
            templates: {
                title: "模板审核队列",
                description: "查看临时模板和待审核模板，并对其进行批准、拒绝、编辑或 AI 重写。",
                empty: "暂无上游错误模板。",
                fingerprint: "错误指纹",
                normalizedStatusCode: "状态码 {{value}}",
                hitCount: "命中 {{count}} 次",
                semanticErrorCode: "语义错误码",
                action: "动作",
                retryScope: "重试范围",
                firstSeenAt: "首次出现",
                lastSeenAt: "最近出现",
                updatedAt: "最近更新",
                representativeSamples: "代表样本",
                samplesEmpty: "尚未记录归一化样本。",
                localizedTemplates: "多语言模板",
                localeEmpty: "该语言暂未生成模板。"
            },
            builtinTemplates: {
                title: "内置模板",
                description: "查看算法默认文案和网关错误文案，并支持编辑、AI 重写或恢复系统默认值。",
                empty: "暂无内置模板。",
                kind: "模板类型",
                code: "模板代码",
                scope: "作用范围",
                gatewayOnly: "仅网关本地返回",
                overridden: "已覆盖",
                defaultState: "默认",
                updatedAt: "覆盖更新时间 {{value}}",
                localizedTemplates: "当前生效模板",
                defaultTemplates: "系统默认模板",
                save: "保存内置模板",
                reset: "恢复默认",
                kinds: {
                    gatewayError: "网关错误",
                    heuristicUpstream: "启发式上游错误"
                }
            },
            actions: {
                saveSettings: "保存错误学习设置",
                approve: "批准",
                reject: "拒绝",
                rewrite: "AI 重写",
                saveTemplate: "保存模板",
                cancel: "取消"
            },
            statuses: {
                provisionalLive: "临时生效",
                reviewPending: "待审核",
                approved: "已批准",
                rejected: "已拒绝"
            },
            actionValues: {
                returnFailure: "直接失败",
                retrySameAccount: "重试同账号",
                retryCrossAccount: "重试其他账号"
            },
            retryScopes: {
                none: "不重试",
                sameAccount: "同账号",
                crossAccount: "跨账号"
            },
            locales: {
                en: "英语",
                zhCN: "简体中文"
            }
        },
        common: {
            none: "无",
            deletedProfile: "已删除画像",
            priority: "优先级 {{value}}"
        }
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
            a11y: {
                model: "模型",
                summary: "模型分布包含 {{count}} 行，按 {{mode}} 排序。领先模型：{{top}}。后附无障碍数据表。",
                summaryEmpty: "当前筛选条件下暂无模型分布数据。",
                tableLabel: "模型分布数据表"
            },
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
            a11y: {
                summary: "Token 趋势覆盖 {{count}} 个时间点，范围从 {{start}} 到 {{end}}。后附无障碍数据表。",
                summaryEmpty: "当前筛选条件下暂无 Token 趋势数据。",
                tableLabel: "Token 使用趋势数据表",
                timestamp: "时间"
            },
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
            payloadSummary: {
                empty: "无附加上下文",
                present: "包含附加上下文"
            },
            reasonValues: {
                none: "无附加说明",
                present: "包含操作备注"
            },
            targetTypes: {
                requestLogs: "请求日志",
                requestCorrelation: "请求关联链",
                auditLogs: "审计日志",
                usageSummary: "用量汇总",
                usageTrendsHourly: "逐小时用量趋势",
                upstreamErrorTemplate: "上游错误模板",
                builtinErrorTemplate: "内置错误模板",
                upstreamErrorLearningSettings: "错误学习设置",
                unknown: "未知目标"
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
                latency: "延迟",
                path: "路径",
                requestId: "请求 ID",
                serviceTier: "服务层级",
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
        time: {
            displayMode: "当前按本地时间（{{timezone}}）显示，悬浮提示中保留 UTC 原值。",
            tooltip: "本地时间：{{local}} | UTC：{{utc}}"
        },
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
            description: "按采样小时查看可核对的请求量。",
            empty: "暂无数据",
            title: "逐小时明细"
        },
        leaderboard: {
            description: "当前筛选条件下的 API 密钥请求量排名。",
            empty: "暂无排行数据",
            title: "API 密钥排行"
        },
        subtitle: "按 API 密钥筛选请求趋势，并查看逐小时明细。",
        title: "用量分析",
        trend: {
            description: "所选时间范围内的逐小时请求量。",
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
                standard: "标准计划",
                unknown: "未知计划"
            },
            statusValues: {
                active: "生效",
                inactive: "停用",
                suspended: "已暂停",
                disabled: "已禁用",
                unknown: "未知状态"
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
    layout: {
        theme: {
            quickSwitchFromSystem: "当前跟随系统：{{current}}，点击切换到 {{next}}"
        }
    },
    theme: {
        aurora: "极光",
        colorful: "彩色",
        dark: "深色",
        light: "浅色",
        system: "跟随系统"
    },
    usage: {
        actions: {
            export: "导出报表",
            filters: "高级筛选"
        },
        antigravity: {
            estimatedCost: "预估成本",
            estimatedCostHint: "基于当前请求结构推算的近似支出。",
            avgLatency: "首 Token 平均延迟",
            avgLatencyHint: "近 30 天请求的平均首 Token 响应时间。",
            last30Days: "过去 30 天",
            loading: "正在加载用量分析…",
            modelMixDescription: "观察当前请求量主要集中在哪些模型上。",
            modelMixEmpty: "当前没有可展示的模型分布数据。",
            modelMixTitle: "模型负载分布",
            peakDay: "峰值日",
            peakDayValue: "{{date}} · {{requests}} 次请求",
            requestsSummaryHint: "近 30 天总请求量。",
            requestsCount: "{{value}} 次请求",
            shareValue: "{{value}}%",
            signalsDescription: "把近 30 天最值得关注的峰值、头部模型和头部密钥收在一起。",
            signalsTitle: "关键观察点",
            timeWindow: "分析窗口",
            tokenTrendDescription: "展示输入、缓存与输出 token 在时间窗口里的变化。",
            tokenTrendEmpty: "当前没有 token 趋势数据。",
            tokenTrendTitle: "Token 趋势",
            topApiKey: "头部 API Key",
            topApiKeyValue: "{{value}} 次请求",
            topModel: "头部模型",
            topModelValue: "{{value}} 次请求",
            tokensCount: "{{value}} 个 Token",
            topKeysHint: "从请求次数和总 token 负载两个维度查看模型集中度。",
            totalTokensHint: "包含输入、缓存、输出和推理 token 的总量。"
        },
        chart: {
            empty: "此时段内无数据记录。",
            requests: "请求次数",
            subtitle: "按天汇总所有租户与上游提供方的请求量",
            title: "近 30 天请求量"
        },
        subtitle: "查看近 30 天的请求体量与 API 密钥集中度。",
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
            subtitle: "按请求量排序",
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
        title: "密钥池",
        subtitle: "管理当前单机工作区的密钥池，并签发安全访问凭据。",
        listTitle: "密钥池",
        createPanelDescription: "为当前单机工作区创建可调用 Data Plane 的访问密钥。创建后会返回一次明文 key，请立即保存。",
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
        },
        antigravity: {
            lastUsed: "最近使用",
            revoke: "撤销",
            rotate: "轮换密钥",
            view: "查看详情"
        }
    }
}
