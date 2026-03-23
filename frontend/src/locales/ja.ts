export default {
    accounts: {
        actions: {
            add: "アカウントの追加",
            apiKeyNoGroupAction: "API キーのアカウントは関連アクション対象外です",
            batchDelete: "一括削除",
            batchDeleteConfirm: "選択した {{count}} 件のアカウントを削除しますか？",
            batchDisable: "一括無効化",
            batchEnable: "一括有効化",
            batchMenu: "一括操作",
            batchPauseFamily: "一括関連アカウント停止（{{count}}）",
            batchRefreshLogin: "一括ログイン更新（{{count}}）",
            batchResumeFamily: "一括関連アカウント再開（{{count}}）",
            comfortableMode: "標準表示",
            compactMode: "コンパクト表示",
            delete: "アカウントを削除",
            deleteConfirm: "アカウント {{label}} を削除しますか？",
            disableAccount: "アカウントを無効化",
            enableAccount: "アカウントを有効化",
            export: "CSV エクスポート",
            filter: "リストのフィルタリング",
            oneTimeNoGroupAction: "ワンタイムセッションは関連アクション対象外です",
            pauseGroup: "関連アカウントを停止",
            refreshAccounts: "更新",
            refreshLogin: "ログインを更新",
            refreshingAccounts: "更新中",
            resumeGroup: "関連アカウントを再開",
            selectAll: "現在のフィルター結果をすべて選択",
            selectOne: "アカウント {{label}} を選択",
            selectedCount: "{{count}} 件選択中",
            viewDetails: "詳細を見る",
            edit: "プロパティの編集",
            refresh: "強制更新",
            suspend: "一時停止",
            exportSuccess: "エクスポート成功",
            refreshTriggered: "アカウント更新をトリガーしました"
        },
        columns: {
            actions: "アクション",
            account: "アカウント",
            added: "追加日",
            credentialType: "資格情報タイプ",
            health: "健康状態",
            id: "アカウント ID",
            loginStatus: "ログイン状態",
            nextRefresh: "次回更新",
            plan: "プラン",
            provider: "プロバイダ / モード",
            rateLimit: "Rate Limit 使用状況",
            runtimePool: "ランタイムプール",
            binding: "バインディング",
            unbound: "未バインド"
        },
        details: {
            description: "説明",
            officialDescription: "Official OpenAI model metadata is read-only here. Manual override pricing can be edited below.",
            limitsTitle: "制限タイトル",
            noOauthStatus: "認証ステータスなし",
            noSupportedModels: "このアカウントでは、利用可能なモデル一覧がまだ取得されていません。",
            oauthNotApplicable: "Oauth は適用されません",
            oauthTitle: "認証タイトル",
            profileTitle: "プロフィールのタイトル",
            rawTitle: "生のタイトル",
            supportedModelsCount: "{{count}} モデル",
            tabAria: "アカウント詳細タブ",
            tabs: {
                limits: "限界",
                oauth: "OAuth",
                profile: "プロフィール",
                raw: "生"
            },
            sections: {
                cache: "レート制限キャッシュ",
                connection: "接続情報",
                credentials: "資格情報",
                identity: "識別情報",
                refresh: "更新状態",
                runtimeHealth: "ランタイム健全性",
                supportedModels: "利用可能なモデル",
                subscription: "契約情報"
            },
            fields: {
                email: "メールアドレス",
                label: "ラベル",
                mode: "モード",
                accountId: "アカウント ID",
                enabled: "有効状態",
                baseUrl: "ベース URL",
                chatgptAccountId: "ChatGPT アカウント ID",
                priority: "優先度",
                createdAt: "作成日時",
                bearerToken: "Bearer トークン",
                authProvider: "認証プロバイダー",
                credentialKind: "資格情報種別",
                lastRefreshStatus: "最終更新ステータス",
                effectiveEnabled: "実効有効状態",
                chatgptPlanType: "ChatGPT プラン種別",
                sourceType: "ソース種別",
                tokenFamilyId: "トークンファミリー ID",
                tokenVersion: "トークンバージョン",
                tokenExpiresAt: "トークン有効期限",
                nextRefreshAt: "次回更新日時",
                lastRefreshAt: "最終更新日時",
                refreshReusedDetected: "Refresh 再利用検知",
                lastRefreshErrorCode: "最終更新エラーコード",
                lastRefreshError: "最終更新エラー",
                rateLimitsFetchedAt: "Rate Limit 取得日時",
                rateLimitsExpiresAt: "Rate Limit 有効期限",
                rateLimitsLastErrorCode: "Rate Limit 最終エラーコード",
                rateLimitsLastError: "Rate Limit 最終エラー",
                poolState: "ランタイムプール",
                refreshCredentialState: "更新資格情報状態",
                quarantineReason: "隔離理由",
                quarantineUntil: "隔離終了時刻",
                pendingPurgeReason: "削除待ち理由",
                pendingPurgeAt: "削除待ち時刻",
                hasRefreshCredential: "更新資格情報あり",
                hasAccessTokenFallback: "アクセストークン fallback あり",
                rawAccount: "アカウント生データ",
                rawOauthStatus: "OAuth ステータス生データ"
            }
        },
        filters: {
            active: "アクティブ",
            all: "すべて",
            credential: "資格情報タイプ",
            credentialAll: "すべての資格情報",
            credentialAt: "AT",
            credentialRt: "RT",
            credentialUnknown: "不明",
            disabled: "無効",
            mode: "モード",
            modeAll: "すべてのモード",
            modeApiKey: "API キー",
            modeOAuth: "OAuth セッション",
            plan: "プランフィルター",
            planAll: "すべてのプラン",
            planUnknown: "未報告",
            total: "一致 {{count}} 件",
            suspended: "一時停止中"
        },
        messages: {
            batchAllFailed: "{{action}}失敗",
            batchAllSuccess: "{{action}}完了",
            batchPartialFailed: "{{failed}} 件の操作が失敗しました{{error}}",
            batchPartialFailedTitle: "{{action}}一部失敗",
            batchSuccessCount: "{{count}} 件成功",
            batchUnknownError: "一括操作に失敗しました",
            deleteFailed: "アカウントの削除に失敗しました",
            deleteSuccess: "アカウントを削除しました",
            disableFailed: "アカウントの無効化に失敗しました",
            disableSuccess: "アカウントを無効化しました",
            enableFailed: "アカウントの有効化に失敗しました",
            enableSuccess: "アカウントを有効化しました",
            exportSuccess: "エクスポート成功",
            pauseFamilyFailed: "関連アカウントの停止に失敗しました",
            pauseFamilySuccess: "関連アカウントを停止しました",
            rateLimitPollingTimeout: "レート制限更新ジョブのポーリングがタイムアウトしました。",
            rateLimitRefreshFailedStatus: "レート制限更新ジョブが失敗しました。ステータス={{status}}",
            rateLimitRefreshFailedSummary: "レート制限更新ジョブが失敗しました: {{summary}}",
            batchRefreshStarted: "{{count}} 件のログイン更新を開始しました",
            refreshFailed: "ログイン更新失敗",
            refreshFailedStatus: "ログイン更新に失敗しました。状態={{status}}",
            refreshFailedSummary: "ログイン更新に失敗しました: {{summary}}",
            refreshPollingTimeout: "ログイン更新ジョブのポーリングがタイムアウトしました。",
            refreshJobId: "ジョブ ID: {{jobId}}",
            refreshJobSummary: "ジョブ ID: {{jobId}} · {{processed}}/{{total}}",
            refreshListFailed: "使用量の更新に失敗しました",
            refreshListSuccess: "使用量を更新しました",
            refreshSuccess: "ログイン更新が完了しました",
            requestFailed: "リクエストに失敗しました。しばらくしてから再試行してください。",
            resumeFamilyFailed: "関連アカウントの再開に失敗しました",
            resumeFamilySuccess: "関連アカウントを再開しました",
            toggleUnsupported: "現在のバックエンドバージョンではアカウントの有効化/無効化はサポートされていません。control-plane をアップグレードしてください。",
            refreshTriggered: "ログイン更新を開始しました"
        },
        rateLimitRefreshJobStatus: {
            queued: "待機中",
            running: "実行中",
            completed: "完了",
            failed: "失敗",
            cancelled: "キャンセル",
            unknown: "不明"
        },
        mode: {
            apiKey: "API キー",
            chatgptSession: "ChatGPT OAuth",
            codexOauth: "コーデックスOAuth",
            unknown: "その他"
        },
        nextRefresh: {
            none: "未スケジュール"
        },
        oauth: {
            kindShort: {
                oneTime: "AT",
                refreshRotatable: "RT",
                unknown: "不明"
            },
            loading: "読み込み中",
            notApplicable: "-",
            provider: {
                legacyBearer: "旧 Bearer トークン",
                refreshToken: "リフレッシュトークン"
            },
            sourceType: {
                codex: "Codex",
                unknown: "不明なソース"
            },
            status: {
                failed: "失敗",
                never: "未更新",
                ok: "正常"
            },
            unknownError: "不明なエラー",
            versionPrefix: "v",
            planPrefix: "プラン: ",
            kind: {
                refreshRotatable: "更新可能な Refresh Token",
                oneTime: "ワンタイム Access Token",
                unknown: "不明な資格情報タイプ"
            }
        },
        refreshCredentialState: {
            healthy: "正常",
            degraded: "劣化",
            invalid: "無効",
            missing: "欠落",
            unknown: "不明"
        },
        rateLimits: {
            labels: {
                fiveHours: "5時間制限",
                github: "GitHub",
                oneWeek: "週次制限"
            },
            shortLabels: {
                fiveHours: "5h",
                oneWeek: "7d"
            },
            moreDetails: "詳細を表示（+{{count}}）",
            noReset: "リセット時刻なし",
            remainingPrefix: "残り",
            resetAt: "{{absolute}}（{{relative}}）でリセット",
            unavailable: "Rate Limit 情報なし",
            usedPrefix: "使用済み"
        },
        searchPlaceholder: "メール、ラベル、URL で検索…",
        runtimePool: {
            eyebrow: "ランタイム健全性",
            title: "オンラインプールの状態",
            description: "Accounts はオンラインプールのみを表示します。queued、ready、no_quota などの在庫状態は Inventory で確認します。",
            openInventory: "Inventory を開く",
            active: "アクティブ",
            activeDesc: "現在ランタイムルーティング可能です。",
            quarantine: "隔離",
            quarantineDesc: "再試行またはクォータ回復待ちで一時隔離されています。",
            pendingPurge: "削除待ち",
            pendingPurgeDesc: "ルーティングから外され、非同期クリーンアップ待ちです。",
            vaultReady: "在庫 ready",
            vaultReadyDesc: "refresh なしで active プールに入れる在庫です。",
            unknown: "不明"
        },
        status: {
            active: "アクティブ",
            disabled: "無効"
        },
        subtitle: "API 資格情報と課金状態を管理します。",
        syncing: "アカウント状態を同期中…",
        title: "アカウントプール"
    },
    billing: {
        columns: {
            balanceAfter: "変更後の残高",
            billingDetail: "請求の詳細",
            deductedCredits: "差し引かれるクレジット",
            deductionEvents: "控除イベント",
            delta: "デルタクレジット",
            eventType: "イベント",
            model: "モデル",
            periodDay: "日付",
            periodMonth: "月",
            requestType: "リクエスト種別",
            source: "ソース",
            timestamp: "時間"
        },
        exportCsv: "CSVのエクスポート",
        filters: {
            granularityAriaLabel: "請求粒度",
            tenantAriaLabel: "テナントフィルター",
            tenantPlaceholder: "テナントを選択"
        },
        granularity: {
            day: "毎日",
            month: "毎月"
        },
        ledger: {
            codeLabels: {
                accountDeactivated: "アカウントが無効化されました",
                billingUsageMissing: "使用量決済フィールドが欠落しています",
                failoverExhausted: "再試行/フェイルオーバーが枯渇した",
                noUpstreamAccount: "利用可能な上流アカウントがありません",
                streamPreludeError: "ストリームプレリュードエラー",
                tokenInvalidated: "トークンが無効になりました",
                transportError: "上流ネットワークエラー",
                upstreamRequestFailed: "アップストリームリクエストが失敗しました",
                unknown: "不明"
            },
            details: {
                accrued: "獲得済み: {{value}} クレジット",
                adjustment: "調整: {{value}}",
                extraCharge: "追加料金: {{value}} クレジット",
                failoverAction: "アクション: {{action}}",
                failure: "失敗: {{summary}}",
                failurePrefix: "失敗：",
                serviceTier: "サービスティア: {{tier}}",
                source: "出典: {{source}}",
                upstreamStatus: "上流 {{status}}",
                tokenSettlement: "トークン決済: 入力 {{input}} + キャッシュされた {{cached}} + 出力 {{output}}",
                unitPrice: "単価: {{prices}} クレジット/100 万トークン"
            },
            empty: "まだ帳簿エントリがありません。",
            failoverActions: {
                crossAccountFailover: "クロスアカウントフェイルオーバー",
                retrySameAccount: "同じアカウントを再試行する",
                returnFailure: "返品失敗",
                unknown: "不明"
            },
            releaseReasons: {
                billingSettleFailed: "請求決済に失敗しました",
                failoverExhausted: "再試行/フェイルオーバーが枯渇した",
                invalidUpstreamUrl: "無効なアップストリーム URL 構成",
                noUpstreamAccount: "利用可能な上流アカウントがありません",
                streamPreludeError: "ストリームプレリュードエラー",
                streamUsageMissing: "ストリームの使用状況がありません",
                transportError: "上流ネットワークエラー",
                upstreamRequestFailed: "アップストリームリクエストが失敗しました",
                unknown: "不明"
            },
            showRaw: "生のエントリを表示",
            subtitle: "現在のテナントによってフィルタリングされます。",
            title: "台帳エントリ",
            requestTypes: {
                nonStream: "非ストリーム",
                stream: "ストリーム",
                unknown: "-"
            },
            tokenSegments: {
                cached: "キャッシュされた",
                input: "入力",
                output: "出力"
            }
        },
        messages: {
            rechargeFailedTitle: "再充電に失敗しました",
            rechargeSuccessDetail: "+{{amount}}、残高 {{balance}}",
            rechargeSuccessTitle: "リチャージ成功",
            retryLater: "後でもう一度試してください"
        },
        recharge: {
            amountAriaLabel: "リチャージ金額",
            amountPlaceholder: "クレジットのリチャージ (マイクロクレジット)",
            reasonAriaLabel: "リチャージ理由",
            reasonPlaceholder: "リチャージ理由",
            submit: "リチャージの実行",
            subtitle: "現在選択されているテナントをリチャージします。",
            title: "管理者のリチャージ"
        },
        snapshot: {
            empty: "決済スナップショットはまだありません。",
            subtitle: "月末の決済と調整のために、{{granularity}} までに控除イベントを集計します。",
            title: "決済スナップショット"
        },
        subtitle: "プライマリ ビュー: テナント レベルの管理者フィルタリングを備えたクレジット台帳 (実際の請求)。",
        summary: {
            currentBalance: "現在の残高",
            deductionHint: "負の元帳控除イベントのみがカウントされます。",
            monthConsumed: "今月の消費量",
            todayConsumed: "今日の消費量",
            unitCredits: "単位：クレジット"
        },
        title: "請求センター",
        trend: {
            noData: "傾向データはまだありません。",
            seriesConsumed: "消費クレジット",
            subtitle: "{{granularity}} によって集計された元帳控除を表示します。",
            title: "消費動向"
        }
    },
    common: {
        cancel: "キャンセル",
        close: "近い",
        collapseSidebar: "サイドバーを折りたたむ",
        confirm: "確認",
        delete: "削除",
        edit: "編集",
        expand: "Expand",
        collapse: "Collapse",
        expandSidebar: "サイドバーを展開",
        loading: "読み込み中…",
        logout: "ログアウト",
        no: "いいえ",
        noData: "データがありません。",
        openMenu: "メニューを開く",
        refresh: "更新",
        skipToMainContent: "メインコンテンツにスキップ",
        table: {
            firstPage: "最初のページ",
            go: "移動",
            jumpToPage: "ページへ移動",
            lastPage: "最後のページ",
            nextPage: "次のページ",
            pageOf: "{{page}} / {{total}} ページ",
            previousPage: "前のページ",
            range: "{{start}}-{{end}} / {{total}} 件",
            rowsPerPage: "1ページの件数",
            searchLabel: "テーブルを検索",
            searchPlaceholder: "現在の一覧を検索…"
        },
        toggleLanguage: "言語を切り替え",
        toggleTheme: "テーマを切り替え",
        yes: "はい",
        save: "保存",
        search: "検索…",
        routeLoading: "ページを読み込み中…",
        status: {
            healthy: "正常",
            degraded: "低下",
            offline: "オフライン",
            disabled: "無効",
            available: "利用可能"
        }
    },
    serviceTier: {
        default: "デフォルト",
        flex: "フレックス",
        priority: "優先"
    },
    config: {
        controlPlane: {
            authValidateUrl: "認証検証 URL",
            dataPlaneUrl: "転送サービス URL",
            desc: "コントロールプレーンと転送サービスの接続先を設定します",
            listen: "コントロールプレーン待受アドレス（読み取り専用）",
            title: "コントロールプレーン接続"
        },
        refreshSettings: {
            desc: "ログイン資格情報の自動更新のオン/オフと間隔を設定します",
            enableDesc: "無効にすると期限切れ間近の資格情報は自動更新されません。",
            enableLabel: "自動更新を有効化",
            intervalSec: "更新間隔（秒）",
            notes: "メモ",
            title: "自動更新設定"
        },
        runtimeHint: {
            desc: "変更はすぐに反映されます。再起動後は環境変数と config.toml が優先されます。",
            title: "ランタイム設定の注意"
        },
        save: "変更を保存",
        subtitle: "ランタイム設定とグローバル変数",
        success: "設定が正常に保存されました。",
        title: "構成",
        reload: {
            title: "ランタイムリロード有効",
            desc: "キャッシュとタイムアウトへの変更はグローバルに同期され、サービスの再起動を必要とせずに即座に有効になります。"
        },
        system: {
            title: "システム戦略",
            desc: "グローバルな操作コンテキストを構成します。",
            maintenance: "メンテナンスモード",
            maintenanceDesc: "503 を保持しているすべての新規リクエストをグローバルに拒否します。",
            logRetention: "ログの保存 (日数)",
            logRetentionDesc: "ClickHouse トレースストレージのローリングウィンドウ。"
        },
        network: {
            title: "ネットワークプロファイリング",
            desc: "アップストリームの禁止を防ぐためにグローバルなレート制限を定義します。",
            tpm: "グローバルの 1 分あたりのトークン",
            tpmDesc: "AI エンドポイントへの同時リクエストの最大数。",
            scaling: "クラウド リソース スケーリング",
            scalingDesc: "動的なノード割り当ての境界を許可します。"
        }
    },
    dashboard: {
        actions: {
            viewBilling: "請求内容を表示する",
            viewLogs: "リクエストログの表示"
        },
        alerts: {
            checkRoutes: "ルートを確認",
            columns: {
                action: "推奨アクション",
                message: "メッセージ",
                severity: "重要度",
                source: "ソース",
                status: "状態",
                time: "時刻"
            },
            empty: "システムは正常に動作しています。",
            resolve: "解決済みにする",
            searchPlaceholder: "アラートを検索…",
            subtitle: "対応が必要なシステム状態",
            title: "要注意",
            usageRepoUnavailable: "使用量分析ストレージが利用できません",
            dataPlaneDisconnected: "データプレーンの接続が切断されました",
            now: "今",
            severity: {
                critical: "重大",
                warning: "警告",
                info: "情報"
            },
            source: {
                data_plane: "データプレーン",
                usage_repo: "使用量リポジトリ"
            },
            status: {
                open: "未対応",
                resolved: "解決済み"
            }
        },
        currentScope: "現在: {{scope}}",
        filters: {
            apiKeyAriaLabel: "API キーフィルター",
            apiKeyPlaceholder: "APIキーを選択",
            description: "まず全体を見て、アラートや急増、コスト変化の理由を確認したいときにテナントや API キーまで絞り込みます。",
            eyebrow: "コンテキスト",
            range: {
                last24Hours: "過去 24 時間",
                last30Days: "過去 30 日間",
                last7Days: "過去 7 日間"
            },
            rangeAriaLabel: "期間",
            scopeAriaLabel: "表示範囲フィルター",
            tenantAriaLabel: "テナントフィルター",
            tenantPlaceholder: "テナントの選択",
            title: "表示範囲とフィルター"
        },
        hero: {
            eyebrow: "運用の概要"
        },
        meta: {
            autoRefresh: "30 秒ごとに自動更新"
        },
        kpi: {
            activeApiKeysInRange: "アクティブな API キー (選択された範囲)",
            accounts: "アカウント数",
            accountsDesc: "管理者向け運用メトリクス",
            apiKeys: "API キー数",
            apiKeysDesc: "システムに設定済みのキー数",
            avgFirstTokenSpeed: "平均ファーストトークンスピード",
            avgFirstTokenSpeedDesc: "TTFT（ストリーミング精密 / 非ストリーミング近似）",
            globalScope: "グローバルな範囲",
            rpm: "RPM",
            rpmDesc: "1分あたりリクエスト数",
            requests: {
                apiKey: "現在の API キー リクエスト (選択された範囲)",
                global: "アカウントリクエストの総数（選択した範囲）",
                tenant: "現在のテナント API キー リクエスト (選択された範囲)"
            },
            tenants: "テナント数",
            tenantsDesc: "管理者向け運用メトリクス",
            totalRequests: "総リクエスト数",
            totalTokens: "Token 消費総量",
            totalTokensDesc: "入力 + キャッシュ + 出力 + 推論",
            tpm: "TPM",
            tpmDesc: "1分あたり Token 数",
            running: "稼働中",
            totalConfigured: "設定済み合計",
            uptime: "稼働率 99.99%",
            reqs: "総リクエスト数",
            reqsDesc: "先月比 +12.5%",
            failed: "ブロック数",
            failedDesc: "本日 180 件",
            avgLatency: "平均遅延",
            avgLatencyDesc: "先月比 -5ms",
            activeTokens: "有効なトークン",
            activeTokensDesc: "新規 24 個追加",
            oauthLabel: "OAuth"
        },
        scope: {
            apiKey: "APIキービュー",
            global: "グローバルビュー",
            tenant: "テナントビュー"
        },
        subtitle: "ゲートウェイの健全性、使用量の変化、管理対象リソースを 1 つの運用ビューで確認します。",
        table: {
            apiKey: "APIキー",
            requests: "リクエスト"
        },
        modelDistribution: {
            a11y: {
                model: "モデル",
                summary: "モデル分布は {{mode}} で並んだ {{count}} 行です。先頭のモデルは {{top}} です。続けてアクセシブルなデータ表を表示します。",
                summaryEmpty: "現在の条件ではモデル分布データがありません。",
                tableLabel: "モデル分布データ表"
            },
            description: "リクエスト数または Token 使用量でモデル Top を表示します。",
            empty: "モデル分布データはまだありません",
            modeRequests: "リクエスト数で表示",
            modeTokens: "Token 量で表示",
            other: "その他",
            title: "モデルリクエスト分布"
        },
        overview: {
            attentionNeeded: "対応を推奨",
            autoRefresh: "30 秒ごとに自動更新",
            degraded: "劣化",
            description: "最初にここを見れば、アラート、利用パイプライン、在庫のどこに先に手を付けるべきか分かります。",
            eyebrow: "パルス",
            inventory: "利用可能なアップストリーム在庫",
            managedScope: "現在の管理対象範囲",
            openAlerts: "未対応アラート",
            stable: "進行中の障害はありません",
            title: "運用パルス",
            usagePipeline: "利用パイプライン"
        },
        poolOverview: {
            eyebrow: "プール概要",
            title: "在庫とランタイムプール",
            description: "vault admission と runtime pool を並べて見ることで、アクティベーション圧力を早く把握できます。",
            queued: "Vault queued",
            queuedDesc: "取り込み済みで admission probe 待ちです。",
            ready: "Vault ready",
            readyDesc: "refresh なしで active に入れます。",
            needsRefresh: "Vault needs refresh",
            needsRefreshDesc: "active に入る前に 1 回 refresh が必要です。",
            noQuota: "Vault no quota",
            noQuotaDesc: "probe は成功したが現在クォータ不足です。",
            active: "アクティブ",
            activeDesc: "現在ルーティング可能なランタイムアカウントです。",
            quarantine: "隔離",
            quarantineDesc: "再試行またはリセット待ちのランタイムアカウントです。",
            pendingPurge: "削除待ち",
            pendingPurgeDesc: "致命判定済みでルーティングから外されています。"
        },
        tokenComponents: {
            cached: "キャッシュ入力",
            input: "入力",
            output: "出力",
            reasoning: "推論"
        },
        tokenTrend: {
            a11y: {
                summary: "{{start}} から {{end}} までの {{count}} 時点を含む Token 推移です。続けてアクセシブルなデータ表を表示します。",
                summaryEmpty: "現在の条件では Token 推移データがありません。",
                tableLabel: "Token 使用推移データ表",
                timestamp: "時刻"
            },
            description: "入力・キャッシュ・出力・推論 Token の推移を比較できます。表示を切り替えて、どこで消費が増えているかを切り分けます。",
            empty: "Token トレンドデータはまだありません",
            title: "Token 使用トレンド"
        },
        title: "概要",
        topApiKeys: {
            empty: "ランキングデータはまだありません",
            scopeDescription: "スコープ: {{scope}} / 選択した時間枠",
            title: "上位の API キー"
        },
        trafficChart: {
            scope: {
                apiKey: "範囲: 現在の API キーリクエスト",
                global: "範囲: グローバル アカウント リクエスト + グローバル テナント API キー リクエスト",
                tenant: "スコープ: 現在のテナント API キー リクエスト"
            },
            series: {
                accountRequests: "アカウントリクエスト",
                tenantApiKeyRequests: "テナント API キーのリクエスト",
                tenantApiKeyRequestsSingle: "テナント API キーのリクエスト"
            },
            title: "トラフィックの概要",
            subtitle: "過去24時間の時間ごとのプロキシ通信量",
            success: "成功",
            blocked: "ブロック"
        }
    },
    importJobs: {
        actions: {
            cancel: "キャンセル",
            cancelJob: "ジョブをキャンセル",
            confirmCancelJob: "このインポートジョブをキャンセルしますか？",
            exportFailed: "失敗項目をエクスポート",
            refreshItems: "項目を更新",
            remove: "削除",
            retryFailed: "失敗分を再試行",
            removeFromList: "一覧から削除"
        },
        detail: {
            columns: {
                admission: "Admission",
                error: "エラー",
                label: "ラベル",
                line: "行",
                quota: "Quota",
                reason: "理由",
                status: "状態"
            },
            admissionFilterAll: "すべての結果",
            admissionFilterLabel: "Admission フィルター",
            filterLabel: "ステータス絞り込み",
            itemsEmpty: "一致する項目がありません。",
            itemsLoading: "ジョブ項目を読み込み中…",
            jobIdLabel: "ジョブ ID: {{jobId}}",
            loadMore: "さらに読み込む",
            loadedCount: "{{count}} 件を読み込み済み",
            loadingMore: "読み込み中",
            retryQuery: "再取得",
            searchPlaceholderModern: "label / email / error で検索…",
            selectHint: "詳細を表示するジョブを選択してください。",
            summaryLoading: "ジョブ概要を読み込み中…",
            title: "ジョブ詳細",
            unreadable: "このジョブを読み取れません（期限切れまたは無効な ID の可能性）。",
            searchPlaceholder: "label / email / error で検索"
        },
        dropzone: {
            acceptsNew: "複数の .json/.jsonl を一括アップロードできます。バックエンドで 1 つのジョブに統合されます。",
            creatingTitle: "インポートジョブを作成中…",
            selectFiles: "ファイルを選択",
            titleNew: "ここにインポートファイルをドロップ",
            uploading: "アップロード中…",
            wait: "ファイルが安全に送信されるまでお待ちください。",
            title: "クリックしてアップロードするか、ファイルをここにドラッグ",
            accepts: "サポート形式: <code>.csv</code> または行区切りの <code>.txt</code>。最大 50,000 行。",
            browse: "ファイルを参照",
            creatingDesc: "少々お待ちください。作成後に右側キューへ自動追加されます。"
        },
        error: "アップロード失敗",
        manual: {
            add: "追加",
            placeholderModern: "job_id を貼り付け…",
            title: "手動ジョブ追跡",
            placeholder: "job_id を貼り付け"
        },
        messages: {
            jobNotFound: "ジョブが存在しないか、アクセスできません",
            queryFailed: "クエリが失敗しました",
            unknownError: "不明なエラー",
            uploadFailedTitle: "インポートに失敗しました"
        },
        errors: {
            importFailed: "インポートに失敗しました",
            invalidRecord: "レコード形式が不正です",
            invalidRefreshToken: "refresh_token が無効です",
            missingAccessToken: "access_token が不足しています",
            missingCredentials: "認証情報が不足しています",
            missingRefreshToken: "refresh_token が不足しています",
            oauthProviderNotConfigured: "OAuth プロバイダーが未設定です",
            rateLimited: "リクエストがレート制限されました",
            refreshTokenReused: "refresh_token はすでに使用済みです",
            unknown: "不明なインポートエラー",
            upstreamNetworkError: "上流ネットワークエラー",
            upstreamUnavailable: "上流サービスが利用できません"
        },
        credentialMode: {
            title: "インポート認証モード",
            description: "このバッチを可変更新可能な refresh_token として取り込むか、単発の access_token として取り込むかを選択します。OAuth ログイン取り込みは変更されません。",
            refreshToken: "RT を取り込む",
            refreshTokenHint: "プラットフォーム側で更新とローテーションを管理したい場合に使います。",
            accessToken: "AK を取り込む",
            accessTokenHint: "更新ローテーションを行わない単発取り込みに向いています。"
        },
        admission: {
            eyebrow: "Admission outcome",
            quotaExhausted: "クォータ切れのため再 probe 待ちです。",
            quotaReady: "probe 成功、現在クォータあり。",
            quotaNotApplicable: "クォータ概要はありません。",
            status: {
                queued: "Queued",
                ready: "Ready",
                needsRefresh: "Needs refresh",
                noQuota: "No quota",
                failed: "Failed",
                unknown: "Unknown"
            }
        },
        metrics: {
            created: "新規",
            failed: "失敗",
            processed: "処理済み",
            status: "状態",
            throughput: "スループット",
            total: "合計",
            updated: "既存／更新"
        },
        precheck: {
            createdNotice: "インポートジョブを作成しました: {{id}}",
            defaultReady: "ファイル形式とサイズのチェックに合格しました。",
            duplicateBatch: "これらのファイルはすでに待機リストにあります。",
            duplicateName: "同名ファイルが検出されました。インポート前に内容を確認してください。",
            emptyPreview: "プレビューに有効なデータ行がありません。空ファイルの可能性があります。",
            firstLineInvalid: "JSONL の先頭行が不正な JSON です。インポートに失敗する可能性があります。",
            firstLineObject: "先頭行が JSON オブジェクトではありません。インポートに失敗する可能性があります。",
            firstLineValid: "JSONL の先頭行チェックに合格しました。",
            jsonEmpty: "JSON ファイルの内容が空です。",
            jsonInvalid: "JSON ファイルが不正です。インポートに失敗する可能性があります。",
            jsonValid: "JSON 構造チェックに合格しました。",
            noneImportable: "インポート可能なファイルがありません。まずブロック項目を解消してください。",
            skipLargeJson: "ファイルが大きいためローカル JSON 解析をスキップしました。サーバー側で検証されます。",
            status: {
                invalid: "ブロック",
                ready: "インポート可能",
                warning: "確認が必要"
            }
        },
        progress: {
            done: "完了",
            etaLabel: "推定残り時間: ",
            etaMinutes: "約 {{count}} 分",
            jobIdLabel: "ジョブ ID: {{jobId}}",
            lessThanMinute: "1 分未満",
            noJobSelected: "インポートジョブを作成または選択すると、ここに進捗が表示されます。",
            title: "リアルタイム進捗",
            topErrors: "主なエラー内訳"
        },
        queue: {
            columns: {
                jobId: "ジョブID"
            },
            descRecent: "キューはステータスを自動ポーリングします。ジョブをクリックして明細とエラーを確認できます。",
            emptyRecent: "追跡中のジョブがありません。ファイルをアップロードするか job_id を手動追加してください。",
            titleRecent: "最近のインポートジョブ",
            tracked: "ローカルで追跡中",
            title: "最近のジョブ追跡",
            empty: "このセッションでアップロードされたジョブはありません。",
            card: {
                processed: "処理済み",
                new: "新規",
                errors: "エラー"
            }
        },
        status: {
            all: "すべて",
            cancelled: "キャンセル済み",
            completed: "完了",
            created: "新規作成",
            failed: "失敗",
            pending: "保留",
            processing: "処理中",
            queued: "待機中",
            running: "実行中",
            skipped: "スキップ",
            updated: "既存／更新"
        },
        subtitleModern: "ファイルを一括追加して事前チェックし、ワンクリックでインポート。進捗とエラー詳細をリアルタイムで確認できます。",
        template: {
            downloadJsonl: "テンプレートをダウンロード",
            title: "インポートテンプレート",
            desc: "システム要件に準拠するために推奨されるテンプレートをダウンロードしてください。",
            download: "CSV テンプレートをダウンロード",
            titleNew: "インポートテンプレート",
            descNew: "JSONL テンプレートをダウンロードし、選択した認証モードに応じて refresh_token または access_token を入力して一括インポートします。"
        },
        title: "バッチインポートジョブ",
        validation: {
            fileTooLarge: "ファイル {{name}} は 20MB を超えています。分割して再試行してください。",
            unsupportedFormat: "ファイル {{name}} は非対応です。.json / .jsonl のみ利用できます。",
            format: ".csv または .txt ファイルのみ許可されています。",
            size: "ファイルが大きすぎます。最大サイズは 10MB です。"
        },
        workspace: {
            clearQueue: "一覧をクリア",
            columns: {
                action: "操作",
                check: "事前チェック",
                file: "ファイル",
                size: "サイズ",
                status: "状態"
            },
            confirmClear: "現在の待機ファイル一覧をクリアしますか？",
            desc: "ドラッグ＆ドロップまたは一括選択でファイルを追加し、事前チェック後に「インポート開始」を押します。",
            empty: "待機中のファイルはありません。上のエリアにファイルをドラッグしてください。",
            invalidFiles: "ブロック {{count}}",
            invalidHint: "{{count}} 件のファイルはブロックされており、インポート時に除外されます。",
            moreChecks: "件のチェック結果",
            readyFiles: "インポート可能 {{count}}",
            readyHint: "事前チェック完了。すぐにインポートを開始できます。",
            startImportWithCount: "インポート開始（{{count}} ファイル）",
            stepCheck: "事前チェック",
            stepCheckDesc: "形式・サイズ・重複名を確認",
            stepImport: "インポート開始",
            stepImportDesc: "ジョブを作成して進捗を確認",
            stepSelect: "ファイル追加",
            title: "ファイルインポートワークスペース",
            totalFiles: "{{count}} ファイル",
            totalSize: "合計 {{size}}",
            warningFiles: "確認要 {{count}}"
        },
        subtitle: "厳密な形式の CSV/TXT ファイルでアカウントのシークレットを安全にアップロードします。"
    },
    inventory: {
        eyebrow: "Inventory",
        title: "OAuth Inventory",
        subtitle: "アクティベーション前に vault 内の OAuth 在庫を確認し、queued、ready、no_quota がオンラインプールと混ざらないようにします。",
        loading: "在庫を読み込み中…",
        empty: "現在のフィルターに一致する在庫レコードはありません。",
        searchPlaceholder: "メール、ラベル、アカウント ID、admission reason で検索…",
        meta: {
            total: "合計 {{count}}",
            filtered: "{{count}} 件を表示"
        },
        metrics: {
            total: "総レコード"
        },
        filters: {
            status: "在庫状態",
            all: "すべての在庫"
        },
        status: {
            queued: "Queued",
            ready: "Ready",
            needsRefresh: "Needs refresh",
            noQuota: "No quota",
            failed: "Failed",
            unknown: "Unknown"
        },
        credentials: {
            hasRt: "RT ready",
            noRt: "RT なし",
            hasAk: "AK fallback",
            noAk: "AK なし"
        },
        columns: {
            account: "アカウント",
            chatgptAccountId: "ChatGPT アカウント ID",
            vaultStatus: "Vault 状態",
            credentials: "資格情報",
            quota: "クォータ概要",
            timeline: "Admission タイムライン",
            reason: "理由"
        },
        fields: {
            checkedAt: "確認時刻",
            retryAfter: "再試行時刻",
            source: "ソース"
        },
        table: {
            eyebrow: "Vault view",
            title: "Admission inventory records",
            description: "このテーブルは vault 在庫のみを表示します。ランタイムの activation と quarantine は Accounts で確認します。"
        }
    },
    oauthImport: {
        title: "OAuth ログインインポート",
        subtitle: "Codex OAuth でサインインし、ログイン済みアカウントを直接プールへ取り込みます。",
        start: {
            title: "Codex OAuth ログインを開始",
            description: "まずログインセッションを作成し、OAuth 認可完了後にアカウントを自動で取り込みます。"
        },
        form: {
            label: "アカウントラベル（任意）",
            labelPlaceholder: "空欄の場合はメールまたはアカウント ID から自動生成",
            baseUrl: "ベース URL",
            priority: "優先度",
            enabled: "取り込み後にアカウントを有効化"
        },
        actions: {
            startLogin: "OAuth ログイン開始",
            reopenAuth: "OAuth ウィンドウを再表示",
            goAccounts: "アカウント一覧へ",
            submitCallback: "コールバック URL を送信"
        },
        status: {
            label: "セッション状態",
            idle: "未開始",
            waiting_callback: "コールバック待機中",
            exchanging: "トークン交換中",
            importing: "アカウント取り込み中",
            completed: "完了",
            failed: "失敗",
            expired: "期限切れ",
            sessionId: "セッション ID: {{id}}",
            callbackUrl: "コールバック URL: {{url}}",
            expiresAt: "有効期限: {{time}}"
        },
        error: {
            failed: "OAuth 取り込みに失敗しました。"
        },
        result: {
            success: "アカウントの取り込みに成功しました。",
            accountId: "アカウント ID: {{id}}",
            accountLabel: "ラベル: {{label}}",
            email: "メール: {{email}}",
            created: "新規作成",
            updated: "既存アカウント"
        },
        manual: {
            title: "手動コールバックの代替手順",
            description: "自動コールバックに到達できない場合は、リダイレクト URL 全体を貼り付けて送信します。",
            placeholder: "code/state を含む完全なコールバック URL を貼り付け…",
            hint: "自動コールバックが失敗した場合のみ使用してください。"
        },
        notifications: {
            popupBlockedTitle: "ポップアップがブロックされました",
            popupBlockedDescription: "ポップアップを許可して OAuth ウィンドウを再度開いてください。",
            sessionCreatedTitle: "セッションを作成しました",
            sessionCreatedDescription: "OAuth ウィンドウを開きました。ログインを完了してください。",
            sessionCreateFailedTitle: "セッション作成に失敗しました",
            manualSubmitTitle: "コールバックを送信しました",
            manualSubmitSuccess: "手動コールバックが完了し、アカウントを取り込みました。",
            manualSubmitAccepted: "手動コールバックを受け付けました。現在処理中です。",
            manualSubmitFailedTitle: "手動コールバックに失敗しました",
            unknownError: "予期しないエラーです。しばらくして再試行してください。"
        }
    },
    oauthProbe: {
        title: "OAuth ペイロードプローブ",
        subtitle: "独立した Codex OAuth ログインフローを実行し、返却ペイロードだけを取得してアカウントはプールに取り込みません。",
        start: {
            title: "プローブセッションを開始",
            description: "一時的なプローブセッションを作成し、OAuth 認可完了後に取得した JSON を確認・ダウンロードします。"
        },
        form: {
            baseUrl: "ベース URL"
        },
        actions: {
            startProbe: "OAuth プローブ開始",
            reopenAuth: "OAuth ウィンドウを再表示",
            downloadJson: "JSON をダウンロード",
            submitCallback: "コールバック URL を送信"
        },
        status: {
            label: "プローブ状態",
            idle: "未開始",
            waiting_callback: "コールバック待機中",
            exchanging: "トークン交換中",
            importing: "ペイロード処理中",
            completed: "完了",
            failed: "失敗",
            expired: "期限切れ",
            sessionId: "セッション ID: {{id}}",
            callbackUrl: "コールバック URL: {{url}}",
            expiresAt: "有効期限: {{time}}",
            memoryOnly: "プローブ結果はメモリ内のみに保持され、セッション期限切れまたはサービス再起動で消えます。"
        },
        error: {
            failed: "OAuth プローブに失敗しました。"
        },
        result: {
            success: "プローブ結果を取得しました。",
            email: "メール: {{email}}",
            accountId: "ChatGPT Account ID: {{id}}",
            plan: "プラン: {{plan}}",
            expiresAt: "トークン有効期限: {{time}}",
            accessTokenPreview: "Access Token プレビュー: {{value}}",
            refreshTokenPreview: "Refresh Token プレビュー: {{value}}"
        },
        payload: {
            title: "取得した JSON",
            description: "OAuth code exchange 完了後にメモリへ保存されたプローブ結果です。",
            empty: "まだプローブペイロードは取得されていません。"
        },
        manual: {
            title: "手動コールバックの代替手順",
            description: "自動コールバックに到達できない場合は、リダイレクト URL 全体を貼り付けて送信します。",
            placeholder: "code/state を含む完全なコールバック URL を貼り付け…",
            hint: "自動コールバックが失敗した場合のみ使用してください。"
        },
        notifications: {
            popupBlockedTitle: "ポップアップがブロックされました",
            popupBlockedDescription: "ポップアップを許可して OAuth ウィンドウを再度開いてください。",
            sessionCreatedTitle: "プローブセッションを作成しました",
            sessionCreatedDescription: "OAuth ウィンドウを開きました。ログインを完了するとペイロードを取得します。",
            sessionCreateFailedTitle: "プローブセッション作成に失敗しました",
            manualSubmitTitle: "コールバックを送信しました",
            manualSubmitSuccess: "手動コールバックが完了し、プローブ結果を取得しました。",
            manualSubmitAccepted: "手動コールバックを受け付けました。現在処理中です。",
            manualSubmitFailedTitle: "手動コールバックに失敗しました",
            unknownError: "予期しないエラーです。しばらくして再試行してください。"
        }
    },
    login: {
        brand: {
            badge: "管理ワークスペースへのアクセス",
            points: {
                audit: "サインイン、ルート変更、高リスク操作はあとから request id で追跡できます。",
                resilience: "テナント、キー、使用量、請求を確認するときも管理系の操作を安定して保ちます。",
                security: "テナント境界と資格情報の制御は常に有効です。"
            },
            subtitle: "システム管理者向けの統制されたサインイン入口です。",
            title: "毎日の運用作業を安心して進めるための Codex Pool 管理入口"
        },
        messages: {
            failed: "ログインに失敗しました。ユーザー名とパスワードを確認してください。",
            invalidCredentials: "ユーザー名またはパスワードが正しくありません。もう一度お試しください。",
            sessionExpired: "ログイン状態の有効期限が切れました。再度ログインしてください。"
        },
        password: "パスワード",
        passwordPlaceholder: "管理者パスワードを入力",
        securityHint: "サインイン失敗が続くと、あとで確認できるよう監査ログに記録されます。",
        submit: "ログイン",
        subtitle: "管理者アカウントでログインしてください",
        title: "Codex-Pool 管理コンソール",
        username: "ユーザー名",
        usernamePlaceholder: "管理者ユーザー名を入力"
    },
    logs: {
        audit: {
            actionValues: {
                adminOperation: "管理操作",
                authOperation: "認証操作",
                requestOperation: "リクエスト操作",
                tenantOperation: "テナント操作",
                unknown: "不明な操作"
            },
            actorTypes: {
                adminUser: "管理者ユーザー",
                apiKey: "APIキー",
                system: "システム",
                tenantUser: "テナントユーザー",
                unknown: "不明な実行者"
            },
            columns: {
                action: "アクション",
                actor: "俳優",
                createdAt: "時間",
                reason: "理由",
                result: "結果",
                target: "ターゲット",
                tenant: "テナント"
            },
            description: "範囲: コントロール プレーン監査イベント (役割/アクション/結果/ターゲット/ペイロード)。",
            empty: "利用可能な監査ログ データがありません",
            filters: {
                actionPlaceholder: "アクション",
                actorIdPlaceholder: "アクターID",
                actorTypePlaceholder: "俳優のタイプ",
                keywordPlaceholder: "キーワード（理由/ペイロード）",
                resultStatusPlaceholder: "結果ステータス",
                actionAriaLabel: "アクションフィルター",
                actorIdAriaLabel: "アクター ID フィルター",
                actorTypeAriaLabel: "アクター種別フィルター",
                keywordAriaLabel: "キーワードフィルター",
                rangeAriaLabel: "期間",
                resultStatusAriaLabel: "結果ステータスフィルター",
                tenantAriaLabel: "テナントフィルター"
            },
            resultStatuses: {
                denied: "拒否",
                failed: "失敗",
                ok: "成功",
                unknown: "不明な結果"
            },
            title: "監査ログ"
        },
        columns: {
            level: "レベル",
            message: "メッセージ",
            service: "サービスノード",
            timestamp: "タイムスタンプ"
        },
        export: "ログをエクスポート",
        filters: {
            allTenants: "すべてのテナント"
        },
        focus: "フォーカス:",
        levels: {
            all: "すべてのレベル",
            error: "エラー",
            info: "情報",
            warn: "警告"
        },
        messages: {
            configUpdated: "実行中設定のスナップショットを更新しました",
            empty: "ログ詳細はありません",
            keyCreated: "API キー {{keyId}} を作成しました",
            keyPatched: "API キー {{keyId}} の有効状態を {{enabled}} に設定しました",
            modelsLoaded: "上流アカウント {{label}} からモデル一覧を読み込みました",
            modelsProbed: "モデルプローブ（{{trigger}}）：{{label}} 経由で {{tested}} 件を確認（利用可 {{available}}、利用不可 {{unavailable}}）",
            proxiesTested: "{{count}} 個のプロキシノードを検査しました",
            systemState: "システム状態を確認しました: {{count}} 件のアカウント",
            unmappedAction: "{{action}} · {{message}}"
        },
        range: {
            last24Hours: "過去 24 時間",
            last30Days: "過去 30 日間",
            last7Days: "過去 7 日間"
        },
        request: {
            columns: {
                apiKey: "APIキー",
                createdAt: "時間",
                errorCode: "エラー",
                latency: "レイテンシー (ミリ秒)",
                path: "パス",
                requestId: "リクエストID",
                serviceTier: "サービスティア",
                status: "状態",
                tenant: "テナント"
            },
            description: "範囲: Raw Data Plane リクエスト イベント (ステータス / レイテンシー / パス / テナント / API キー / リクエスト ID)。",
            empty: "利用可能なリクエスト ログ データがありません",
            filters: {
                apiKeyIdPlaceholder: "APIキーID",
                keywordPlaceholder: "キーワード（パス/エラー/モデル）",
                requestIdPlaceholder: "リクエストID",
                statusCodePlaceholder: "ステータスコード (例: 500)",
                apiKeyAriaLabel: "API キーフィルター",
                keywordAriaLabel: "キーワードフィルター",
                rangeAriaLabel: "期間",
                requestIdAriaLabel: "リクエスト ID フィルター",
                statusCodeAriaLabel: "ステータスコードフィルター",
                tenantAriaLabel: "テナントフィルター"
            },
            title: "リクエストログ"
        },
        search: "ペイロードまたはサービスを検索…",
        subtitle: "リアルタイムの監査証跡と操作コンテキスト。",
        time: {
            displayMode: "現在はローカル時刻（{{timezone}}）で表示しています。UTC はツールチップとエクスポートに保持されます。",
            tooltip: "ローカル: {{local}} | UTC: {{utc}}"
        },
        tabs: {
            audit: "監査ログ",
            request: "リクエストログ",
            system: "システムログ"
        },
        title: "システムログ",
        waiting: "受信ストリームを待機しています…",
        actions: {
            systemState: "システム状態の確認",
            configUpdate: "設定更新",
            proxiesTest: "プロキシ健全性チェック",
            keyCreate: "API キー作成",
            keyPatch: "API キー更新",
            modelsList: "モデル一覧の取得",
            modelsProbe: "モデルプローブ",
            unknown: "不明な操作（{{action}}）"
        }
    },
    models: {
        actions: {
            copyModelId: "モデル名をコピー",
            createModel: "モデルの作成",
            syncOpenAiCatalog: "Sync OpenAI catalog",
            probeAvailability: "Probe availability",
            openDetails: "Details",
            deleteModel: "モデルの削除",
            deletePricing: "価格設定の削除",
            deleteBillingRule: "Delete rule",
            editBillingRule: "Edit rule",
            editModel: "モデルの編集",
            probeNow: "手動テスト",
            saveModelProfile: "モデルプロファイルの保存",
            savePricing: "価格を節約する",
            saveBillingRule: "Save rule",
            newBillingRule: "New rule",
            search: "モデル ID を検索…",
            sync: "状態同期"
        },
        availability: {
            available: "利用可能",
            issueHint: "利用不可の理由を表示",
            neverChecked: "未チェック",
            noErrorDetail: "エラー詳細なし",
            unavailable: "利用不可",
            unknown: "未テスト",
            viewIssue: "原因を表示"
        },
        cache: {
            fresh: "キャッシュ有効",
            stale: "キャッシュ期限切れ"
        },
        catalog: {
            customOnly: "カスタムモデル",
            hidden: "カタログ非表示",
            listed: "カタログ掲載",
            unlisted: "未収録"
        },
        columns: {
            actions: "詳細",
            availability: "可用性",
            cachedInputPrice: "キャッシュされた入力価格",
            context: "Context / Max output",
            modalities: "Modalities",
            syncedAt: "Synced",
            catalog: "カタログ状態",
            checkedAt: "最終チェック",
            id: "モデル名",
            inputPrice: "投入価格",
            outputPrice: "出力価格",
            pricingStatus: "価格設定ステータス",
            provider: "プロバイダ / Hub"
        },
        description: "ここでモデルの入手可能性を確認し、モデルのプロファイルと価格を管理します。",
        dialog: {
            description: "このダイアログでプロファイルと価格を編集します。保存された価格設定は、モデル プール リストにすぐに書き戻されます。",
            officialDescription: "Official OpenAI model metadata is read-only here. Manual override pricing can be edited below.",
            tabListAriaLabel: "モデルプロファイルタブ",
            titleWithId: "モデルプロフィール · {{modelId}}"
        },
        empty: "設定されたモデルはありません。",
        emptySyncRequired: "No official catalog yet. Sync OpenAI catalog first.",
        emptyActions: {
            goAccounts: "アカウント一覧へ",
            importAccount: "アカウントをインポート"
        },
        errors: {
            deleteModelEntityFailed: "モデル エンティティの削除に失敗しました。",
            deleteModelPricingFailed: "モデル価格の削除に失敗しました。",
            openAiCatalogSyncFailed: "Failed to sync OpenAI catalog.",
            deleteBillingRuleFailed: "Failed to delete tiered pricing rule.",
            modelIdRequired: "モデル ID を空にすることはできません。",
            probeFailed: "モデルの調査に失敗しました。",
            saveBillingRuleFailed: "Failed to save tiered pricing rule.",
            saveModelPricingFailed: "モデル価格を保存できませんでした。",
            saveModelProfileFailed: "モデルプロファイルの保存に失敗しました。"
        },
        filters: {
            allProviders: "すべてのプロバイダー",
            providerLabel: "プロバイダーフィルター"
        },
        form: {
            modelId: "モデルID",
            modelIdLockedHint: "既存モデルはIDを変更できません。新しいモデルを追加するには、「モデルの作成」を使用します。",
            modelIdPlaceholder: "例: gpt-5.3-codex",
            provider: "プロバイダー",
            providerPlaceholder: "例: openai / カスタム",
            source: "ソース",
            sourceValues: {
                entityOnly: "エンティティのみ",
                pricingOnly: "価格のみ",
                upstream: "上流"
            },
            visibility: "可視性",
            visibilityPlaceholder: "例: リスト/非表示"
        },
        hints: {
            cannotDeleteMissingPricing: "現在のモデルには現地価格の記録がありません。価格を削除する前に、まず価格を保存してください。",
            cannotDeleteNonLocalEntity: "現在のモデルはローカル エンティティ モデルではないため、そのエンティティは削除できません。"
        },
        loadingHint: "ディレクトリと利用可能状態を確認中です。完了後、最新のモデルリストが自動的に表示されます。",
        notice: {
            modelEntityDeleted: "モデル エンティティが削除されました。",
            modelPricingDeleted: "モデル価格レコードが削除されました。",
            billingRuleDeleted: "Tiered pricing rule deleted.",
            modelPricingSaved: "保存されたモデル価格: {{model}}",
            openAiCatalogSynced: "OpenAI catalog synced: {{count}} models updated.",
            billingRuleSaved: "Tiered pricing rule saved: {{model}}",
            modelProfileSaved: "モデル プロファイルが保存されました: {{model}}",
            probeCompleted: "モデルのプローブが完了しました。最新モデルのプールが同期されました。"
        },
        pricing: {
            cachedInputPrice: "キャッシュされた入力価格",
            creditsPerMillionTokens: "クレジット / 100万トークン",
            disabled: "無効",
            enablePricing: "価格設定を有効にする",
            enabled: "有効",
            inputPrice: "入力価格",
            notConfigured: "未設定",
            outputPrice: "出力価格",
            perMillionTokensMicrocredits: "100万トークンあたり、マイクロクレジット単位",
            sectionTitle: "モデルの価格設定",
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
        probeSourceUnknown: "不明なアカウント",
        probeSummary: "プローブキャッシュ：{{stale}}、最終 {{checkedAt}}、保持 {{ttlHours}} 時間、ソース {{source}}",
        syncHint: {
            notSynced: "OpenAI catalog has not been synced yet.",
            syncedAt: "Catalog synced {{time}}"
        },
        profile: {
            sectionTitle: "モデルプロフィール"
        },
        syncing: "エンドポイント機能をマッピングしています…",
        tabs: {
            pricing: "価格設定",
            profile: "プロフィール"
        },
        title: "モデル",
        subtitle: "アカウントプールから取得したモデルのカタログ状態と実測可用性を表示します",
        detail: {
            title: "モデル詳細",
            notFound: "モデルが見つかりません。削除されたか、現在のフィルター外です。",
            httpStatus: "HTTP ステータス",
            error: "エラー詳細",
            noError: "エラー詳細なし",
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
    costReports: {
        admin: {
            title: "コストレポート",
            description: "直近 30 日間の推定 USD 利用額を読み取り専用で表示します。"
        },
        tenant: {
            title: "利用コストレポート",
            description: "現在のテナントの推定 USD 利用額を読み取り専用で表示します。"
        },
        summary: {
            totalCost: "推定コスト",
            totalRequests: "総リクエスト数",
            avgCostPerRequest: "1 リクエストあたり平均コスト"
        },
        chart: {
            title: "コスト推移",
            description: "リクエストログから集計した推定 USD コストです。",
            empty: "選択した範囲にはコストデータがありません。",
            series: {
                cost: "推定コスト"
            }
        },
        logs: {
            title: "リクエストログ",
            empty: "この範囲にはリクエストログがありません。",
            searchPlaceholder: "リクエスト ID、モデル、テナント、またはステータスで検索",
            columns: {
                time: "時刻",
                tenant: "テナント",
                requestId: "リクエスト ID",
                model: "モデル",
                status: "ステータス",
                cost: "推定コスト"
            }
        },
        filters: {
            tenant: "テナント",
            allTenants: "すべてのテナント",
            apiKey: "API キー",
            allApiKeys: "すべての API キー",
            day: "日次",
            month: "月次"
        }
    },
    nav: {
        accounts: "アカウントプール",
        modelRouting: "Model Routing",
        apiKeyGroups: "Group Management",
        apiKeys: "API キー",
        billing: "請求する",
        config: "設定",
        dashboard: "ダッシュボード",
        inventory: "Inventory",
        groups: {
            analytics: "分析",
            assets: "プール資産",
            operations: "運用",
            system: "システム"
        },
        importJobs: "インポート",
        oauthImport: "OAuth ログインインポート",
        oauthProbe: "OAuth ペイロードプローブ",
        logs: "システムログ",
        mainNavigation: "メインナビゲーション",
        models: "モデル",
        online: "オンライン",
        openNavigation: "ナビゲーションを開く",
        proxies: "プロキシ",
        system: "システム状態",
        tenants: "テナントプール",
        usage: "利用状況",
        cleanup: "クリーンアップ",
        closeNavigation: "ナビゲーションを閉じる"
    },
    notifications: {
        dismiss: "通知を閉じる",
        loginFailed: {
            title: "ログイン失敗"
        },
        sessionExpired: {
            title: "ログイン状態が期限切れです",
            description: "続行するには再度ログインしてください。"
        }
    },
    errors: {
        common: {
            failed: "失敗",
            network: "ネットワークエラーです。接続を確認してください。",
            timeout: "タイムアウトしました。しばらくしてから再試行してください。"
        },
        api: {
            unauthorized: "認証に失敗しました。もう一度ログインしてください。",
            invalidRequest: "無効なリクエストです。",
            invalidProxyUrl: "プロキシ URL が無効です。",
            notFound: "リソースが見つかりません。",
            serviceUnavailable: "サービスを利用できません。",
            internalError: "サーバー内部エラーです。",
            oauthProviderNotConfigured: "OAuth プロバイダーが設定されていません。",
            oauthCallbackListenerUnavailable: "OAuth コールバックリスナーが利用できません。",
            invalidRefreshToken: "リフレッシュトークンが無効または期限切れです。",
            refreshTokenReused: "リフレッシュトークンが再利用されています。最新のトークンを取得してください。",
            refreshTokenRevoked: "リフレッシュトークンが失効しました。",
            oauthMissingClientId: "OAuth プロバイダーの設定が不完全です（client_id がありません）。",
            oauthUnauthorizedClient: "OAuth クライアントが未認可です。",
            upstreamUnavailable: "上流サービスを利用できません。",
            upstreamNetworkError: "上流ネットワークエラーです。",
            oauthExchangeFailed: "OAuth 交換に失敗しました。"
        },
        http: {
            badRequest: "不正なリクエスト",
            unauthorized: "未認証",
            forbidden: "権限がありません",
            notFound: "見つかりません",
            conflict: "競合",
            payloadTooLarge: "ペイロードが大きすぎます",
            rateLimited: "レート制限",
            internalServerError: "サーバーエラー",
            badGateway: "不正なゲートウェイ",
            serviceUnavailable: "サービス利用不可",
            gatewayTimeout: "ゲートウェイタイムアウト"
        }
    },
    proxies: {
        actions: {
            add: "プロキシを追加",
            delete: "削除",
            edit: "編集",
            test: "テスト",
            testAll: "すべてテスト"
        },
        badges: {
            auth: "認証あり"
        },
        columns: {
            actions: "アクション",
            lastTest: "最終テスト",
            latency: "遅延",
            proxy: "プロキシ",
            status: "状態",
            weight: "重み"
        },
        deleteDialog: {
            confirm: "プロキシを削除",
            description: "{{label}} をグローバルな送信プロキシプールから削除しますか。既存クライアントは次回更新後にこのノードを使わなくなります。",
            title: "プロキシを削除"
        },
        editor: {
            create: "プロキシを作成",
            createTitle: "送信プロキシを追加",
            description: "グローバル送信プロキシノードを設定します。編集時に URL を空欄のまま保存すると、現在のシークレットは保持されます。",
            editTitle: "送信プロキシを編集",
            enabledHint: "無効化したノードは一覧に残りますが、選択や自動テストの対象にはなりません。",
            errors: {
                labelRequired: "プロキシラベルを入力してください。",
                proxyUrlRequired: "プロキシ URL を入力してください。",
                weightInvalid: "重みは 0 より大きい値にしてください。"
            },
            fields: {
                enabled: "有効",
                label: "ラベル",
                proxyUrl: "プロキシ URL",
                weight: "重み"
            },
            proxyUrlHint: "http://、https://、socks5:// をサポートします。ホストとポートは必須で、認証情報は URL に埋め込めます。",
            proxyUrlPlaceholder: "http://user:password@127.0.0.1:6152",
            save: "変更を保存"
        },
        empty: "送信プロキシはまだ設定されていません。",
        failModeDescriptions: {
            allowDirectFallback: "正常なプロキシがすべて失敗した場合、プラットフォームは直接接続へフォールバックできます。",
            strictProxy: "正常なプロキシがない場合、送信リクエストは即座に失敗し、プールを迂回しません。"
        },
        failModes: {
            allowDirectFallback: "直接接続へフォールバック",
            strictProxy: "必ずプロキシを使用"
        },
        filters: {
            all: "すべてのノード",
            degraded: "劣化",
            disabled: "無効",
            healthy: "正常",
            label: "状態フィルター",
            offline: "オフライン"
        },
        health: {
            degraded: "劣化",
            disabled: "無効",
            healthy: "正常",
            offline: "オフライン"
        },
        list: {
            description: "重み付きプロキシノードの追加、編集、削除、テストを行います。管理 API はシークレットを保存しますが、返す URL は常にマスク済みです。",
            title: "プロキシノード一覧"
        },
        loading: "送信プロキシプールを読み込み中…",
        meta: {
            enabled: "{{count}} 個が有効",
            healthy: "{{count}} 個が正常",
            total: "{{count}} ノード"
        },
        notifications: {
            nodeCreateFailedTitle: "プロキシの作成に失敗しました",
            nodeCreatedDescription: "プロキシノードをグローバルプールに追加しました。",
            nodeCreatedTitle: "プロキシを作成しました",
            nodeDeleteFailedTitle: "プロキシの削除に失敗しました",
            nodeDeletedDescription: "プロキシノードをグローバルプールから削除しました。",
            nodeDeletedTitle: "プロキシを削除しました",
            nodeUpdateFailedTitle: "プロキシの更新に失敗しました",
            nodeUpdatedDescription: "プロキシノードを更新しました。",
            nodeUpdatedTitle: "プロキシを更新しました",
            settingsFailedTitle: "プロキシ設定の保存に失敗しました",
            settingsSavedDescription: "グローバル送信プロキシプール設定を保存しました。",
            settingsSavedTitle: "プロキシ設定を保存しました",
            singleTestCompletedDescription: "単一ノードのプロキシテストが完了しました。",
            testCompletedDescription: "{{count}} 個のプロキシノードのテストが完了しました。",
            testCompletedTitle: "プロキシテスト完了",
            testFailedTitle: "プロキシテストに失敗しました",
            validationFailedTitle: "プロキシフォームを確認してください"
        },
        pending: "まだテストされていません",
        searchPlaceholder: "ラベル、マスク済み URL、最新エラーで検索…",
        settings: {
            description: "この設定は、プラットフォーム内のすべての外部 HTTP / WebSocket 送信リクエストに適用されます。",
            enabled: "送信プロキシプールを有効化",
            enabledHint: "無効時はすべての送信トラフィックが直接接続のままです。有効にすると、下の重み付きプロキシプールから選択されます。",
            failMode: "失敗時の動作",
            save: "設定を保存",
            title: "グローバルプロキシプール設定"
        },
        stats: {
            enabled: "有効ノード",
            healthy: "正常ノード",
            total: "総ノード数"
        },
        subtitle: "すべての上流トラフィック向けにグローバル送信プロキシプールを設定します。このページは以前のノード健全性プレースホルダーではありません。",
        title: "送信プロキシプール"
    },
    system: {
        columns: {
            component: "コンポーネント",
            details: "詳細",
            status: "状態",
            uptime: "稼働時間",
            version: "バージョン"
        },
        components: {
            controlPlane: "コントロールプレーン",
            dataPlane: "データプレーンルート",
            usageRepo: "使用量リポジトリ"
        },
        details: {
            analyticsUnavailable: "アナリティクスは利用できません",
            apiActive: "APIゲートウェイがアクティブです",
            checkingAPI: "APIを確認中…",
            dbConnected: "時系列DBに接続しました",
            endpointsResponding: "プロキシエンドポイントが応答しています"
        },
        labels: {
            local: "ローカル",
            remote: "リモート",
            storage: "ストレージ",
            version: "バージョン：",
            uptime: "稼働時間"
        },
        observability: {
            badges: {
                failoverOff: "フェイルオーバー: OFF",
                failoverOn: "フェイルオーバー: ON",
                failoverWait: "切替待機 {{value}} ms",
                quickRetry: "高速リトライ ≤ {{value}}",
                retryPoll: "リトライ間隔 {{value}} ms",
                sharedCacheOff: "共有キャッシュ: OFF",
                sharedCacheOn: "共有キャッシュ: ON",
                stickyConflictAvoidOff: "粘性競合回避: OFF",
                stickyConflictAvoidOn: "粘性競合回避: ON"
            },
            hints: {
                billingPreauthCaptureMissingTotal: "請求の事前認証キャプチャに合計がありません",
                billingPreauthErrorRatioAvg: "課金事前認証エラー率の平均",
                billingPreauthErrorRatioP95: "請求事前認証エラー率 p95",
                billingPreauthTopModelP95: "課金事前認証トップモデル p95",
                billingReconcileAdjust: "対帳で自動実行した残高調整件数。",
                billingReconcileFailed: "対帳処理で失敗し確認が必要な件数。",
                billingReconcileReleased: "対帳で自動クローズした認可件数。",
                billingReconcileScanned: "request_log と台帳から対帳で走査した件数。",
                billingReleaseWithoutCaptureRatio: "回収率なしの請求リリース",
                billingSettleCompleteRatio: "請求決済完了率",
                cacheHitRate: "ローカル + 共有 sticky キャッシュ参照の総合命中割合。",
                failoverAttempts: "アカウント切替フェイルオーバーの総試行回数。",
                failoverExhausted: "再試行/切替予算を使い切って失敗した回数。",
                failoverSuccess: "アカウント切替後に復旧成功したリクエスト回数。",
                failoverSuccessRate: "フェイルオーバー試行に対する成功割合。",
                sameAccountRetry: "切替前に同一アカウントで実施した高速リトライ総数。",
                stickyHitRate: "粘性セッションマッピングの命中割合。"
            },
            metrics: {
                billingPreauthCaptureMissingTotal: "請求の事前認証キャプチャに合計がありません",
                billingPreauthErrorRatioAvg: "課金事前認証エラー率の平均",
                billingPreauthErrorRatioP95: "請求事前認証エラー率 p95",
                billingPreauthTopModelP95: "課金事前認証トップモデル p95",
                billingReconcileAdjust: "請求対帳調整件数",
                billingReconcileFailed: "請求対帳失敗件数",
                billingReconcileReleased: "請求対帳解放件数",
                billingReconcileScanned: "請求対帳スキャン件数",
                billingReleaseWithoutCaptureRatio: "回収率なしの請求リリース",
                billingSettleCompleteRatio: "請求決済完了率",
                cacheHitRate: "ルーティングキャッシュ命中率",
                failoverAttempts: "フェイルオーバー試行回数",
                failoverExhausted: "フェイルオーバー枯渇回数",
                failoverSuccess: "フェイルオーバー成功回数",
                failoverSuccessRate: "フェイルオーバー成功率",
                sameAccountRetry: "同一アカウント高速リトライ回数",
                stickyHitRate: "粘性セッション命中率"
            },
            na: "該当なし",
            subtitle: "データプレーンの自動切替、セッション粘性、キャッシュ効果を可視化します。",
            title: "フェイルオーバーとキャッシュの可観測性",
            unavailableDesc: "control-plane から /internal/v1/debug/state へのアクセスと権限設定を確認してください。",
            unavailableLoading: "最新のデバッグスナップショットを待機中...",
            unavailableTitle: "データプレーンのデバッグ状態を取得できません"
        },
        searchPlaceholder: "コンポーネント、詳細、バージョンを検索…",
        status: {
            checking: "確認中",
            degraded: "低下",
            healthy: "正常",
            offline: "オフライン",
            unknown: "不明"
        },
        subtitle: "インフラストラクチャーの依存関係と健全性のセルフチェック。",
        title: "システムステータス"
    },
    tenantApiKeys: {
        actions: {
            changeGroup: "Change Group",
            disable: "無効にする",
            enable: "有効にする"
        },
        columns: {
            actions: "アクション",
            group: "Group",
            ipAllowlist: "IP許可リスト",
            modelAllowlist: "モデル許可リスト",
            name: "名前",
            prefix: "プレフィックス",
            status: "状態"
        },
        create: {
            description: "説明",
            groupLabel: "API key group",
            ipAllowlistAriaLabel: "IP 許可リスト",
            ipAllowlistPlaceholder: "IP 許可リストのプレースホルダー",
            modelAllowlistAriaLabel: "モデル許可リスト",
            modelAllowlistPlaceholder: "モデル許可リストのプレースホルダー",
            nameAriaLabel: "キー名",
            namePlaceholder: "名前のプレースホルダー",
            submit: "提出する",
            title: "タイトル"
        },
        list: {
            description: "説明",
            empty: "API キーがありません。",
            searchPlaceholder: "名前またはプレフィックスで API キーを検索",
            title: "タイトル"
        },
        messages: {
            createFailed: "作成に失敗しました",
            createSuccess: "成功を生み出す",
            plaintextShownOnce: "平文を一度だけ表示",
            retryLater: "後で再試行してください",
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
            disabled: "無効",
            enabled: "有効",
            groupInvalid: "Group invalid"
        },
        subtitle: "字幕"
    },
    tenantApp: {
        appName: "コーデックス プール テナント",
        auth: {
            actions: {
                backToLogin: "サインインに戻る",
                login: "サインイン",
                openForgot: "パスワードをお忘れですか？",
                register: "登録する",
                resetPassword: "パスワードのリセット",
                sendResetCode: "リセットコードを送信する",
                switchToLogin: "アカウントをお持ちですか？ サインイン",
                switchToRegister: "アカウントがありませんか？ 新規登録",
                verifyEmail: "メールの確認"
            },
            brand: {
                badge: "テナントワークスペースへのアクセス",
                points: {
                    audit: "使用量、請求、ポリシー変更は、あとから文脈を追えるよう記録されます。",
                    resilience: "上流が不安定でも、フェイルオーバー対応ルーティングがテナントトラフィックの可用性を支えます。",
                    security: "キー、セッション、アカウントアクセスはテナントごとに分離されます。"
                },
                subtitle: "一度サインインすれば、使用量、請求、API キー管理を同じ安全なワークスペースで進められます。",
                title: "日々の運用に必要なものを 1 つにまとめたテナントアクセス"
            },
            error: {
                invalidCredentialsOrUnverified: "サインインに失敗しました。メールアドレスとパスワードを確認し、初回ログインならメール認証を完了してください。",
                loginFailed: "サインインに失敗しました。時間をおいてもう一度お試しください。",
                passwordMismatch: "パスワードと確認用パスワードが一致しません。",
                passwordResetFailed: "パスワードのリセットに失敗しました。",
                registerFailed: "登録に失敗しました。",
                sendResetCodeFailed: "リセットコードの送信に失敗しました。",
                verificationFailed: "検証に失敗しました。"
            },
            fields: {
                confirmPassword: "確認用パスワード",
                email: "電子メール",
                newPassword: "新しいパスワード",
                password: "パスワード",
                passwordMin8: "パスワード（8文字以上）",
                resetCode: "リセットコード",
                tenantName: "テナント名",
                verificationCode: "検証コード"
            },
            forgot: {
                drawerHint: "まずこのメールアドレスにリセットコードを送信します。届いたら、下でコードと新しいパスワードを入力してください。",
                stepResetPassword: "新しいパスワード設定",
                stepSendCode: "コード送信"
            },
            notice: {
                emailVerified: "メール認証が成功しました。このアカウントでサインインしてください。",
                loginSuccess: "サインインに成功しました。",
                passwordResetSuccess: "パスワードのリセットが成功しました。再度サインインしてください。",
                registerDebugCode: "登録は成功しました。確認コード (デバッグ): {{code}}",
                registerSuccess: "登録が完了しました。メール内の確認コードを入力してアカウントを有効化してください。",
                resetCodeDebug: "パスワード リセット コード (デバッグ): {{code}}",
                resetCodeSentIfExists: "このメールアドレスが存在する場合、まもなくリセットコードを送信します。",
                sessionExpired: "テナントセッションの有効期限が切れました。再度サインインしてください。",
                verifyCodeHint: "まだ届かない場合は、60 秒待ってから再送してください。"
            },
            placeholders: {
                confirmPassword: "パスワードを再入力",
                email: "name@company.com",
                newPassword: "新しいパスワードを入力",
                password: "パスワードを入力",
                resetCode: "リセットコードを入力",
                tenantName: "テナント名を入力",
                verificationCode: "確認コードを入力"
            },
            sections: {
                authSubtitle: "サインインまたは登録を選び、そのまま同じ作業面で続きを進めます。",
                forgotPasswordTitle: "パスワードのリセット",
                forgotPasswordSubtitle: "先にリセットコードを受け取り、この画面のまま新しいパスワードを設定します。",
                loginTitle: "テナントのサインイン",
                registerTitle: "テナント登録",
                verifyEmailSubtitle: "メール内のコードを入力して有効化を完了し、そのままサインインに戻ります。",
                verifyEmailTitle: "メール認証"
            },
            social: {
                comingSoon: "サードパーティサインイン（近日対応）",
                github: "GitHub",
                google: "Google"
            },
            tabs: {
                login: "サインイン",
                register: "登録"
            }
        },
        loadingPortal: "テナントポータルを読み込み中…",
        menu: {
            analytics: "分析",
            apiKeys: "APIキー",
            assets: "資産",
            billing: "請求センター",
            dashboard: "ダッシュボード",
            logs: "ログ",
            usage: "使用量"
        }
    },
    tenantBilling: {
        actions: {
            dailyCheckin: "毎日チェックイン",
            exportCsv: "CSVをエクスポート"
        },
        failoverAction: {
            crossAccountFailover: "クロスアカウントフェイルオーバー",
            retrySameAccount: "同じアカウントを再試行",
            returnFailure: "返品失敗",
            unknown: "不明"
        },
        failureReason: {
            accountDeactivated: "アカウントが無効化されました",
            billingUsageMissing: "請求使用量がありません",
            failoverExhausted: "フェイルオーバーが枯渇しました",
            noUpstreamAccount: "上流アカウントがありません",
            streamPreludeError: "ストリームプレリュードエラー",
            tokenInvalidated: "トークンが無効になりました",
            transportError: "トランスポートエラー",
            upstreamRequestFailed: "アップストリームリクエストが失敗しました",
            unknown: "不明"
        },
        filters: {
            day: "日次",
            dayShort: "日次",
            granularityAriaLabel: "請求粒度",
            month: "月次",
            monthShort: "月次"
        },
        groupPricing: {
            allKeys: "すべての API キー",
            apiKeyAriaLabel: "API キーセレクター",
            columns: {
                apiKey: "API キー",
                finalPrice: "最終価格",
                formulaPrice: "計算価格",
                group: "グループ",
                model: "モデル",
                state: "状態"
            },
            description: "各 API キーが使用している価格グループを確認し、選択したキーの実効価格を点検します。",
            groupSummary: "設定済みモデル {{count}} 件 · 全モデル許可 {{allowAll}}",
            invalidGroup: "この API キーは削除済みグループに紐づいています。グループを変更するまでリクエストは失敗します。",
            state: {
                active: "有効",
                invalid: "無効（グループ削除済み）"
            },
            title: "API キーのグループ料金"
        },
        ledger: {
            columns: {
                balanceAfter: "後の残高",
                delta: "デルタ",
                detail: "詳細",
                event: "イベント",
                model: "モデル",
                requestType: "リクエスト種別",
                time: "時間"
            },
            description: "現在のテナントで絞り込んだ台帳明細。",
            detail: {
                charged: "充電済み",
                extraCharge: "追加料金",
                failoverAction: "フェイルオーバーアクション",
                failure: "失敗",
                failureKeyword: "失敗キーワード",
                failureSummary: "{{failure}}（{{reason}}）",
                reconcileAdjust: "調整調整",
                serviceTier: "サービスティア: {{tier}}",
                source: "ソース",
                tokenSettle: "トークン決済",
                unitPrice: {
                    cached: "キャッシュされた",
                    input: "入力",
                    output: "出力",
                    summary: "まとめ"
                },
                upstreamStatus: "上流 {{status}}"
            },
            empty: "台帳エントリはまだありません。",
            requestTypes: {
                nonStream: "非ストリーム",
                stream: "ストリーム",
                unknown: "-"
            },
            showRaw: "生データを表示",
            title: "台帳エントリ"
        },
        messages: {
            checkinFailed: "チェックインに失敗しました",
            checkinReward: "チェックイン特典",
            checkinSuccess: "チェックイン成功",
            retryLater: "後で再試行してください"
        },
        releaseReason: {
            billingSettleFailed: "請求決済が失敗しました",
            failoverExhausted: "フェイルオーバーが枯渇しました",
            invalidUpstreamUrl: "無効なアップストリーム URL",
            noUpstreamAccount: "上流アカウントがありません",
            streamPreludeError: "ストリームプレリュードエラー",
            streamUsageMissing: "ストリーム使用量がありません",
            transportError: "トランスポートエラー",
            upstreamRequestFailed: "アップストリームリクエストが失敗しました",
            unknown: "不明"
        },
        snapshot: {
            columns: {
                consumed: "消費された",
                date: "日付",
                eventCount: "イベント数",
                month: "月"
            },
            description: "控除イベントを集計し、請求締めと照合に備えます。",
            empty: "決済スナップショットはまだありません。",
            title: "決済スナップショット"
        },
        subtitle: "残高、消費トレンド、台帳明細を確認します。",
        summary: {
            balance: "現在の残高",
            monthConsumed: "今月の消費",
            negativeOnly: "負の控除のみ集計",
            todayConsumed: "今日の消費量",
            unitCredits: "単位：クレジット"
        },
        title: "請求センター",
        tokenSegment: {
            cached: "キャッシュされた",
            input: "入力",
            output: "出力"
        },
        trend: {
            description: "選択した粒度で集計した消費トレンドです。",
            empty: "トレンドデータはまだありません。",
            series: {
                consumed: "消費クレジット"
            },
            title: "消費トレンド"
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
    modelRoutingPage: {
        title: "Model Routing",
        subtitle: "Configure routing profiles, model-aware fallback chains, and planner model chain settings for model dispatching.",
        actions: {
            refresh: "Refresh",
            createProfile: "Create profile",
            createPolicy: "Create policy",
            edit: "Edit",
            delete: "Delete",
            deleteProfile: "Delete profile",
            deletePolicy: "Delete policy",
            saveSettings: "Save settings",
            saveProfile: "Save profile",
            savePolicy: "Save policy"
        },
        settings: {
            title: "Model Routing Settings",
            description: "Control model routing publish behavior, safety switches, and planner model chain settings.",
            enabled: "Enable model routing",
            enabledHint: "When disabled, compiled routing plans stay manual only.",
            autoPublish: "Auto publish",
            autoPublishHint: "Publish the latest routing plan automatically after recompute.",
            killSwitch: "Kill switch",
            killSwitchHint: "Stop planner-driven changes immediately without deleting saved rules.",
            triggerMode: "Trigger mode",
            plannerModelChain: "Planner model chain",
            plannerModelChainPlaceholder: "gpt-5.2-codex, gpt-4.1-mini",
            plannerModelChainHint: "モデルプールからプランナーのフォールバックモデルを選び、優先度順を保持します。",
            updatedAt: "Updated at {{value}}"
        },
        versions: {
            title: "Published Versions",
            description: "Latest compiled routing plans for the data plane.",
            empty: "No published routing plan versions yet.",
            noReason: "No publish reason recorded.",
            defaultSegments: "Default segments {{count}}",
            policyCount: "Policies {{count}}",
            showMore: "Show {{count}} older versions",
            showLess: "Show fewer versions"
        },
        profiles: {
            title: "Routing Profiles",
            description: "Reusable selectors describing which plans, modes, and auth types may serve a request.",
            empty: "No routing profiles yet.",
            summary: "Plans {{plans}} · modes {{modes}} · auth filters {{authProviders}} · include {{include}} · exclude {{exclude}}",
            anyMode: "All account modes"
        },
        policies: {
            title: "Model Policies",
            description: "Map model families or exact model IDs to an explicit fallback chain of routing profiles.",
            empty: "No model routing policies yet.",
            summary: "Exact models {{exact}} · prefixes {{prefixes}} · fallback profiles {{fallbacks}}",
            fallbackChain: "Fallback chain: {{value}}",
            moreExactModels: "+{{count}} more"
        },
        dialogs: {
            createProfile: "Create routing profile",
            editProfile: "Edit routing profile",
            profileDescription: "Compose a reusable selector for a class of upstream accounts.",
            createPolicy: "Create model policy",
            editPolicy: "Edit model policy",
            policyDescription: "Define how a model family maps onto ordered routing profiles."
        },
        form: {
            name: "Name",
            description: "Description",
            priority: "Priority",
            enabled: "Enabled",
            enabledHint: "Only enabled profiles can appear in compiled routing plans.",
            policyEnabledHint: "Disabled policies are saved but excluded from routing.",
            planTypes: "Plan types",
            planTypesPlaceholder: "free, plus, team",
            modes: "Account modes",
            authProviders: "Auth providers",
            includeAccounts: "Include account IDs",
            includeAccountsPlaceholder: "uuid-1, uuid-2",
            excludeAccounts: "Exclude account IDs",
            excludeAccountsPlaceholder: "uuid-3, uuid-4",
            family: "Model family",
            familyPlaceholder: "gpt-5",
            exactModels: "Exact models",
            exactModelsPlaceholder: "gpt-5.4, gpt-5.2-codex",
            exactModelsHint: "モデルプールから正確なモデル ID を選択し、可用性と価格を確認しながら設定します。",
            modelPrefixes: "Model prefixes",
            modelPrefixesPlaceholder: "gpt-5, o3",
            modelPrefixesHint: "gpt-5 や o3 のようなワイルドカード接頭辞は自由入力のまま保持します。",
            fallbackProfiles: "Fallback profiles",
            noProfilesAvailable: "Create a routing profile first."
        },
        messages: {
            settingsSaved: "Model routing settings saved.",
            settingsSaveFailed: "Failed to save model routing settings.",
            errorLearningSettingsSaved: "上流エラー学習設定を保存しました。",
            errorLearningSettingsSaveFailed: "上流エラー学習設定の保存に失敗しました。",
            profileSaved: "Routing profile saved: {{name}}",
            profileSaveFailed: "Failed to save routing profile.",
            profileDeleted: "Routing profile deleted.",
            profileDeleteFailed: "Failed to delete routing profile.",
            policySaved: "Model routing policy saved: {{name}}",
            policySaveFailed: "Failed to save model routing policy.",
            policyDeleted: "Model routing policy deleted.",
            policyDeleteFailed: "Failed to delete model routing policy.",
            templateSaved: "上流エラーテンプレートを保存しました。",
            templateSaveFailed: "上流エラーテンプレートの保存に失敗しました。",
            templateApproved: "上流エラーテンプレートを承認しました。",
            templateApproveFailed: "上流エラーテンプレートの承認に失敗しました。",
            templateRejected: "上流エラーテンプレートを却下しました。",
            templateRejectFailed: "上流エラーテンプレートの却下に失敗しました。",
            templateRewritten: "AI で上流エラーテンプレートを書き直しました。",
            templateRewriteFailed: "AI による上流エラーテンプレートの書き直しに失敗しました。",
            builtinTemplateSaved: "内蔵エラーテンプレートを保存しました。",
            builtinTemplateSaveFailed: "内蔵エラーテンプレートの保存に失敗しました。",
            builtinTemplateRewritten: "AI で内蔵エラーテンプレートを書き直しました。",
            builtinTemplateRewriteFailed: "AI による内蔵エラーテンプレートの書き直しに失敗しました。",
            builtinTemplateReset: "内蔵エラーテンプレートを既定値に戻しました。",
            builtinTemplateResetFailed: "内蔵エラーテンプレートを既定値に戻せませんでした。"
        },
        status: {
            enabled: "Enabled",
            disabled: "Disabled",
            killSwitchOn: "Kill switch on",
            autoPublishOn: "Auto publish on",
            autoPublishOff: "Auto publish off"
        },
        triggerModes: {
            hybrid: "Hybrid",
            scheduledOnly: "Scheduled only",
            eventOnly: "Event only"
        },
        modes: {
            apiKey: "API key",
            chatGptSession: "ChatGPT session",
            codexOauth: "Codex OAuth"
        },
        authProviders: {
            legacyBearer: "Legacy bearer",
            oauthRefreshToken: "OAuth refresh token"
        },
        modelSelector: {
            addModel: "Add model",
            searchPlaceholder: "Search models by ID or title",
            emptyCatalog: "No models are available from the model pool yet.",
            emptySelection: "No models selected yet.",
            noMatches: "No matching models found.",
            unknownModel: "Saved model not in pool",
            moveUp: "Move up",
            moveDown: "Move down",
            remove: "Remove model"
        },
        errorLearning: {
            settings: {
                title: "上流エラー学習",
                description: "初回に検出した上流失敗テンプレートを、確定ルールに昇格させる前にレビューします。",
                enabled: "上流エラー学習を有効化",
                enabledHint: "無効にすると、未知の上流エラーは汎用のローカライズ済みエラー文面にフォールバックします。",
                firstSeenTimeoutMs: "初回検出タイムアウト（ms）",
                firstSeenTimeoutMsHint: "初回の暫定テンプレート生成で同期的に待機する上限時間です。",
                reviewHitThreshold: "レビュー移行しきい値",
                reviewHitThresholdHint: "暫定テンプレートのヒット数がこの回数に達するとレビュー待ちへ移動します。",
                updatedAt: "更新日時 {{value}}"
            },
            templates: {
                title: "テンプレートレビューキュー",
                description: "暫定テンプレートとレビュー待ちテンプレートを確認し、承認・却下・編集・AI 再生成を行います。",
                empty: "上流エラーテンプレートはまだありません。",
                fingerprint: "フィンガープリント",
                normalizedStatusCode: "ステータス {{value}}",
                hitCount: "{{count}} 件ヒット",
                semanticErrorCode: "意味エラーコード",
                action: "アクション",
                retryScope: "再試行スコープ",
                firstSeenAt: "初回検出",
                lastSeenAt: "最終検出",
                updatedAt: "更新日時",
                representativeSamples: "代表サンプル",
                samplesEmpty: "正規化サンプルはまだありません。",
                localizedTemplates: "多言語テンプレート",
                localeEmpty: "この言語のテンプレートはまだありません。"
            },
            builtinTemplates: {
                title: "内蔵テンプレート",
                description: "アルゴリズム既定文言とゲートウェイのエラー文言を確認し、編集・AI 再生成・既定値への復元を行います。",
                empty: "利用可能な内蔵テンプレートはありません。",
                kind: "テンプレート種別",
                code: "テンプレートコード",
                scope: "適用範囲",
                gatewayOnly: "ゲートウェイのローカル応答のみ",
                overridden: "上書き済み",
                defaultState: "既定",
                updatedAt: "上書き更新日時 {{value}}",
                localizedTemplates: "現在の有効テンプレート",
                defaultTemplates: "システム既定テンプレート",
                save: "内蔵テンプレートを保存",
                reset: "既定値に戻す",
                kinds: {
                    gatewayError: "ゲートウェイエラー",
                    heuristicUpstream: "ヒューリスティック上流エラー"
                }
            },
            actions: {
                saveSettings: "エラー学習設定を保存",
                approve: "承認",
                reject: "却下",
                rewrite: "AI 再生成",
                saveTemplate: "テンプレートを保存",
                cancel: "キャンセル"
            },
            statuses: {
                provisionalLive: "暫定運用中",
                reviewPending: "レビュー待ち",
                approved: "承認済み",
                rejected: "却下済み"
            },
            actionValues: {
                returnFailure: "失敗を返す",
                retrySameAccount: "同じアカウントで再試行",
                retryCrossAccount: "別アカウントで再試行"
            },
            retryScopes: {
                none: "再試行しない",
                sameAccount: "同じアカウント",
                crossAccount: "アカウントを跨いで再試行"
            },
            locales: {
                en: "英語",
                zhCN: "中国語（簡体字）",
                zhTW: "中国語（繁体字）",
                ja: "日本語",
                ru: "ロシア語"
            }
        },
        common: {
            none: "None",
            deletedProfile: "Deleted profile",
            priority: "Priority {{value}}"
        }
    },
    tenantDashboard: {
        actions: {
            manageApiKeys: "API キーを管理",
            refresh: "リフレッシュ",
            viewBilling: "請求内容を表示する",
            viewRequestLogs: "リクエストログの表示"
        },
        kpi: {
            avgFirstTokenSpeed: "平均ファーストトークンスピード",
            avgFirstTokenSpeedDesc: "TTFT（ストリーミング精密 / 非ストリーミング近似）",
            rpm: "RPM",
            rpmDesc: "1分あたりリクエスト数",
            totalRequests: "総リクエスト数",
            totalRequestsDesc: "選択した時間範囲",
            totalTokens: "Token 消費総量",
            totalTokensDesc: "入力 + キャッシュ + 出力 + 推論",
            tpm: "TPM",
            tpmDesc: "1分あたり Token 数"
        },
        cards: {
            activeKeys: {
                description: "注: リクエストのあるキーのみがカウントされます",
                title: "アクティブな API キー数 (選択した期間)"
            },
            availableKeys: {
                description: "テナントのキー管理設定に基づく",
                title: "現在利用可能な API キー"
            },
            keyEnableRate: {
                description: "有効化率: {{rate}}%（{{enabled}} / {{total}} キー）",
                title: "キー有効化率"
            },
            peakHour: {
                description: "リクエスト量が最も多い時間帯",
                empty: "データなし",
                title: "ピーク時間帯"
            },
            requestVelocity: {
                description: "選択範囲での 1 時間あたり平均リクエスト数",
                title: "リクエスト速度（1時間あたり）"
            },
            totalRequests: {
                scopeAllKeys: "/ すべてのキー",
                scopePrefix: "範囲: 現在のテナント",
                scopeSingleKey: "/ 単一キー",
                title: "テナント API キー リクエストの合計数 (選択した期間)"
            }
        },
        filters: {
            apiKeyAll: "すべての API キー",
            apiKeyAriaLabel: "API キーフィルター",
            apiKeyHint: "ヒント: API キーフィルターでホットスポットを素早く特定できます。",
            range: {
                last24Hours: "過去 24 時間",
                last30Days: "過去 30 日間",
                last7Days: "過去 7 日間"
            },
            rangeAriaLabel: "期間"
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
            badge: "テナントワークスペース概要",
            summaryAllApiKeys: "（すべての API キー）",
            summaryPrefix: "範囲: 現在のテナント ",
            summarySingleApiKey: "（単一 API キー）"
        },
        subtitle: {
            allApiKeys: "(すべての API キー)",
            metricsFocus: "注目指標: TPM、RPM、Token 消費総量、総リクエスト数、ファーストトークンスピード。",
            scopePrefix: "範囲: 現在のテナント",
            singleApiKey: "(単一の API キー)",
            timeWindow: "、時間枠:"
        },
        modelDistribution: {
            a11y: {
                model: "モデル",
                summary: "モデル分布は {{mode}} で並んだ {{count}} 行です。先頭のモデルは {{top}} です。続けてアクセシブルなデータ表を表示します。",
                summaryEmpty: "現在の条件ではモデル分布データがありません。",
                tableLabel: "モデル分布データ表"
            },
            description: "リクエスト数または Token 使用量でモデル Top を表示します。",
            empty: "モデル分布データはまだありません",
            modeRequests: "リクエスト数で表示",
            modeTokens: "Token 量で表示",
            other: "その他",
            title: "モデルリクエスト分布"
        },
        tokenComponents: {
            cached: "キャッシュ入力",
            input: "入力",
            output: "出力",
            reasoning: "推論"
        },
        tokenSummary: {
            title: "Token コンポーネント集計"
        },
        tokenTrend: {
            a11y: {
                summary: "{{start}} から {{end}} までの {{count}} 時点を含む Token 推移です。続けてアクセシブルなデータ表を表示します。",
                summaryEmpty: "現在の条件では Token 推移データがありません。",
                tableLabel: "Token 使用推移データ表",
                timestamp: "時刻"
            },
            description: "Token コンポーネントごとの時間別トレンド。表示切替で消費元を絞り込めます。",
            empty: "Token トレンドデータはまだありません",
            title: "Token 使用トレンド"
        },
        topKeys: {
            description: "選択期間のリクエスト量に基づく",
            empty: "API キー利用ランキングはまだありません",
            requests: "{{count}} リクエスト",
            share: "シェア {{percent}}%",
            title: "上位 API キー",
            unknownKey: "名称未設定キー"
        },
        title: "テナントダッシュボード",
        trend: {
            description: "範囲: テナント API キーのリクエスト量 (時間単位の粒度)",
            empty: "まだリクエストデータがありません",
            series: {
                requests: "リクエスト"
            },
            title: "リクエストの傾向"
        }
    },
    tenantLogs: {
        audit: {
            actionValues: {
                adminOperation: "管理操作",
                authOperation: "認証操作",
                requestOperation: "リクエスト操作",
                tenantOperation: "テナント操作",
                unknown: "不明な操作"
            },
            actorTypes: {
                adminUser: "管理者ユーザー",
                apiKey: "APIキー",
                system: "システム",
                tenantUser: "テナントユーザー",
                unknown: "不明な実行者"
            },
            columns: {
                action: "アクション",
                actor: "俳優",
                reason: "詳細",
                result: "結果",
                target: "ターゲット",
                time: "時間"
            },
            description: "定義: コントロール プレーン監査イベント (現在のテナントのみ)",
            empty: "監査ログデータがありません",
            filters: {
                actionPlaceholder: "アクション",
                actorIdPlaceholder: "アクターID",
                actorTypePlaceholder: "俳優のタイプ",
                keywordPlaceholder: "キーワード（理由/ペイロード）",
                resultStatusPlaceholder: "結果ステータス",
                actionAriaLabel: "アクションフィルター",
                actorIdAriaLabel: "アクター ID フィルター",
                actorTypeAriaLabel: "アクター種別フィルター",
                keywordAriaLabel: "キーワードフィルター",
                rangeAriaLabel: "期間",
                resultStatusAriaLabel: "結果ステータスフィルター"
            },
            resultStatuses: {
                denied: "拒否",
                failed: "失敗",
                ok: "成功",
                unknown: "不明な結果"
            },
            title: "監査ログ"
        },
        filters: {
            range: {
                last24Hours: "過去 24 時間",
                last30Days: "過去 30 日間",
                last7Days: "過去 7 日間"
            }
        },
        request: {
            columns: {
                apiKey: "APIキー",
                error: "エラー",
                latency: "レイテンシー (ミリ秒)",
                path: "パス",
                requestId: "リクエストID",
                serviceTier: "サービスティア",
                status: "状態",
                time: "時間"
            },
            description: "定義: データ プレーンの生のリクエスト イベント (現在のテナントのみ)",
            empty: "ログデータがありません",
            filters: {
                apiKeyIdPlaceholder: "APIキーID",
                keywordPlaceholder: "キーワード（パス/エラー/モデル）",
                requestIdPlaceholder: "リクエストID",
                statusCodePlaceholder: "ステータスコード (例: 429)",
                apiKeyAriaLabel: "API キーフィルター",
                keywordAriaLabel: "キーワードフィルター",
                rangeAriaLabel: "期間",
                requestIdAriaLabel: "リクエスト ID フィルター",
                statusCodeAriaLabel: "ステータスコードフィルター"
            },
            title: "リクエストログ"
        },
        scope: "範囲: 現在のテナントのみ",
        time: {
            displayMode: "現在はローカル時刻（{{timezone}}）で表示しています。UTC はツールチップに保持されます。",
            tooltip: "ローカル: {{local}} | UTC: {{utc}}"
        },
        tabs: {
            audit: "監査ログ",
            request: "リクエストログ"
        },
        title: "ログ"
    },
    tenantUsage: {
        columns: {
            apiKey: "APIキー",
            requests: "リクエスト",
            tenantLabel: "テナント: {{tenantId}}",
            time: "時間"
        },
        filters: {
            apiKeyAll: "すべての API キー",
            apiKeyAriaLabel: "API キーフィルター",
            range: {
                last24Hours: "過去 24 時間",
                last30Days: "過去 30 日間",
                last7Days: "過去 7 日間"
            },
            rangeAriaLabel: "期間"
        },
        hourly: {
            description: "各時間帯の監査可能なリクエスト数を確認します。",
            empty: "データはまだありません",
            title: "時間別の詳細"
        },
        leaderboard: {
            description: "現在の条件での API キー別リクエスト数です。",
            empty: "ランキングデータはまだありません",
            title: "API キーランキング"
        },
        subtitle: "API キーで絞り込み、リクエスト推移と時間別の詳細を確認します。",
        title: "利用状況分析",
        trend: {
            description: "選択した期間の時間別リクエスト数です。",
            empty: "推移データはまだありません",
            title: "リクエスト推移"
        }
    },
    tenants: {
        create: {
            fields: {
                expiresAt: "有効期限は次のとおりです",
                name: "テナント名",
                plan: "プラン（クレジット）",
                status: "ステータス (アクティブ/非アクティブ)"
            },
            submit: "テナントの作成",
            title: "テナントの作成"
        },
        impersonation: {
            copyToken: "トークンのコピー",
            create: "偽装の作成",
            fields: {
                reason: "理由 (必須)"
            },
            revoke: "セッションの取り消し",
            sessionIdLabel: "セッション ID:",
            tokenLabel: "トークン:",
            title: "管理者のなりすまし"
        },
        keys: {
            create: {
                fields: {
                    name: "キー名",
                    namePlaceholder: "例えば管理者メインキー"
                },
                submit: "キーの作成",
                title: "APIキーの作成"
            },
            created: {
                copyPlaintext: "平文キーのコピー",
                notice: "平文キーは 1 回だけ表示されます。今すぐ保存してください。"
            },
            list: {
                caption: "テナントAPIキーリスト",
                columns: {
                    actions: "アクション",
                    createdAt: "作成日",
                    name: "名前",
                    prefix: "プレフィックス",
                    status: "状態"
                },
                copyPrefix: "キープレフィックスをコピーする",
                disable: "無効にする",
                empty: "このテナントには API キーがありません",
                enable: "有効にする",
                status: {
                    active: "有効",
                    revoked: "無効"
                },
                title: "APIキーリスト"
            }
        },
        list: {
            caption: "テナントプール一覧",
            columns: {
                actions: "アクション",
                apiKeys: "APIキー",
                expiresAt: "有効期限は次のとおりです",
                plan: "プラン",
                status: "状態",
                tenant: "テナント",
                tenantId: "テナントID",
                updatedAt: "更新日"
            },
            planValues: {
                credit: "クレジット",
                unknown: "カスタム（{{value}}）"
            },
            statusValues: {
                active: "有効",
                inactive: "無効",
                unknown: "不明（{{value}}）"
            },
            defaultBadge: "デフォルト",
            empty: "テナントデータなし",
            openProfile: "テナントプロファイルを開く",
            searchPlaceholder: "テナント名または ID で検索",
            title: "テナントプール"
        },
        messages: {
            apiKeyCreateFailed: "APIキーの作成に失敗しました",
            apiKeyCreateSuccess: "テナント {{tenantName}} の API キーを作成しました: {{keyName}}",
            apiKeyNameRequired: "API キー名を入力してください",
            apiKeyToggleFailed: "API キーのステータスを更新できませんでした",
            createFailed: "テナントの作成に失敗しました",
            createSuccess: "テナントが作成されました: {{name}} ({{id}})",
            impersonationCreateFailed: "偽装の作成に失敗しました",
            impersonationCreated: "偽装セッションが作成されました (トークンが返されました)",
            impersonationRevokeFailed: "偽装を取り消すことができませんでした",
            impersonationRevoked: "偽装セッションが取り消されました",
            rechargeFailed: "テナントのリチャージに失敗しました",
            rechargeSuccess: "再チャージが成功しました: +{{amount}}、現在の残高 {{balance}}",
            updateFailed: "テナントの更新に失敗しました",
            updateSuccess: "テナントが更新されました: {{name}}"
        },
        profile: {
            dialogDescription: "タブのある 1 つのダイアログでプロファイル、API キー、使用状況を管理します。",
            dialogTitle: "テナントプロフィール",
            dialogTitleWithName: "テナントプロフィール · {{name}}",
            fields: {
                expiresAt: "有効期限は次のとおりです",
                plan: "プラン",
                status: "状態"
            },
            meta: {
                createdAt: "作成日",
                tenantId: "テナントID",
                updatedAt: "更新日"
            },
            save: "プロファイルの保存",
            section: {
                title: "テナントプロフィール"
            },
            tabs: {
                ariaLabel: "テナントプロファイルタブ",
                keys: "APIキー",
                profile: "プロフィール",
                usage: "使用量"
            }
        },
        recharge: {
            fields: {
                amount: "マイクロクレジット (整数)",
                reason: "理由"
            },
            submit: "リチャージを適用する",
            title: "テナントのリチャージ"
        },
        subtitle: "テナントの可用性を確認し、プロファイル、API キー、使用状況を管理します。",
        title: "テナント",
        usage: {
            filter: {
                allKeys: "すべての API キー",
                currentView: "現在のビュー",
                label: "APIキーフィルター",
                noKeys: "現在のテナントには API キーがありません",
                noMatches: "一致する API キーがありません",
                placeholder: "検索名/プレフィックス/key_id"
            },
            meta: {
                tenantId: "テナントID"
            },
            metrics: {
                accountRequests: "アカウントリクエスト",
                activeAccounts: "アクティブなアカウント",
                activeApiKeys: "アクティブなAPIキー",
                apiKeyRequests: "APIキーリクエスト",
                tenantApiKeyRequests: "テナント API キーのリクエスト"
            },
            sectionTitle: "過去 24 時間の使用量",
            status: {
                error: "使用状況データのロードに失敗しました",
                loading: "使用状況データをロードしています…"
            }
        }
    },
    theme: {
        aurora: "オーロラ",
        colorful: "カラフル",
        dark: "ダーク",
        light: "ライト"
    },
    usage: {
        actions: {
            export: "CSV エクスポート",
            filters: "フィルター"
        },
        chart: {
            empty: "この期間のデータはありません。",
            requests: "リクエスト数",
            subtitle: "全テナントとすべての上流プロバイダの1日合計",
            title: "リクエスト量・過去30日"
        },
        subtitle: "直近 30 日間のリクエスト量と API キー集中度を確認します。",
        title: "利用状況分析",
        topKeys: {
            columns: {
                apiKey: "API キー ID",
                name: "テナント / キー",
                requests: "リクエスト数",
                share: "シェア",
                tenant: "テナント"
            },
            empty: "利用履歴はありません。",
            keyFallback: "API キー {{keyId}}",
            searchPlaceholder: "API キーまたはテナントを検索…",
            subtitle: "リクエスト数順",
            title: "API キーランキング",
            reqs: "回"
        }
    },
    cleanup: {
        title: "資格情報のクリーンアップ",
        subtitle: "自動化されたガバナンスとライフサイクルポリシー",
        save: "ポリシーを保存",
        policy: {
            title: "自動ガバナンスポリシー",
            desc: "refresh_token の再利用・失効・継続失敗を検知した際に、影響範囲を抑えるため自動隔離します。",
            refreshEnabled: "OAuth 自動更新を有効化",
            refreshEnabledDesc: "無効にすると access_token の自動更新が停止します。",
            intervalSec: "更新間隔（秒）",
            notes: "ポリシーメモ"
        },
        workspace: {
            title: "OAuth アカウント運用ワークスペース",
            desc: "アカウントごとのログイン更新状態を確認し、更新や関連アカウントの停止/再開を行えます。",
            searchPlaceholder: "label / account id で検索",
            onlyDisabled: "無効化済みのみ表示",
            loadingAccounts: "アカウントを読み込み中…",
            noAccounts: "一致する OAuth アカウントがありません。",
            enabled: "有効",
            disabled: "無効",
            selectHint: "左側でアカウントを選択してください。",
            loadingStatus: "OAuth 状態を読み込み中…",
            noStatus: "このアカウントの OAuth 状態はまだありません。",
            refreshNow: "今すぐ更新",
            disableFamily: "関連アカウントを停止",
            enableFamily: "関連アカウントを再開",
            status: {
                never: "未更新",
                ok: "正常",
                failed: "失敗"
            },
            fields: {
                refreshStatus: "更新状態",
                reuseDetected: "リフレッシュ再利用検知",
                groupId: "グループ ID",
                tokenVersion: "トークン版",
                expiresAt: "有効期限",
                errorCode: "エラーコード",
                errorMessage: "エラー詳細"
            }
        },
        quarantine: {
            title: "自動隔離ポリシー",
            desc: "認証に失敗したアカウントを自動的に隔離します",
            threshold: "失敗のしきい値",
            thresholdDesc: "隔離前の連続した 401/403 エラー",
            action: "無効化時のアクション",
            actionDesc: "汎用の refresh_token が無効になった場合",
            options: {
                family: "アカウントファミリーを隔離",
                disable: "アカウントのみを無効化",
                nothing: "何もしない"
            }
        },
        purge: {
            title: "自動削除ポリシー",
            desc: "スペースを節約するために無効な資格情報を完全に削除します",
            retention: "保持期間",
            retentionDesc: "パージする前に無効なアカウントを保持する日数"
        }
    },
    apiKeys: {
        title: "API キー",
        subtitle: "クライアントアプリケーションの安全なアクセス認証情報の発行と管理。",
        createPanelDescription: "この単一ワークスペース向けの Data Plane アクセスキーを作成します。平文キーは 1 回のみ表示されます。",
        create: "シークレットキーを作成",
        search: "キー名またはプレフィックスを検索…",
        loading: "認証情報を読み込んでいます…",
        empty: "条件に一致する有効な API キーが見つかりません。",
        columns: {
            name: "アプリケーション名",
            tenant: "テナント ID",
            key: "API キー",
            status: "ステータス",
            issued: "発行日",
            actions: "アクション"
        },
        status: {
            active: "有効",
            revoked: "無効"
        },
        defaultTenant: "デフォルト",
        filters: {
            label: "ステータスフィルター",
            all: "すべてのキー",
            active: "有効",
            revoked: "無効"
        },
        actions: {
            copyPrefixTitle: "プレフィックスをコピー",
            menu: "キー操作",
            copyPrefix: "キーのプレフィックスをコピー",
            processing: "処理中…",
            disable: "キーを無効化",
            enable: "キーを再有効化"
        },
        messages: {
            createFailed: "API キーの作成に失敗しました",
            missingName: "キー名を入力してください"
        },
        dialog: {
            create: {
                title: "API キーを作成",
                desc: "テナント用の Data Plane アクセスキーを作成します。平文キーは 1 回のみ表示されます。",
                nameLabel: "キー名",
                namePlaceholder: "例: prod-codex-clients",
                tenantLabel: "テナント名（任意）",
                tenantPlaceholder: "空欄の場合は default を使用",
                confirm: "作成",
                creating: "作成中…"
            },
            created: {
                title: "新しいキーを作成しました",
                desc: "平文キーは 1 回だけ表示されます。今すぐコピーして保存してください。",
                securityTip: "セキュリティ注意: このダイアログを閉じると平文キーは再表示できません。",
                nameLabel: "キー名",
                plaintextLabel: "平文キー",
                close: "閉じる",
                copyPlaintext: "平文キーをコピー"
            }
        }
    }
}
