export default {
    accounts: {
        actions: {
            add: "Добавить аккаунт",
            apiKeyNoGroupAction: "Для API-ключа групповое действие недоступно",
            batchDelete: "Массово удалить",
            batchDeleteConfirm: "Удалить {{count}} выбранных аккаунтов?",
            batchDisable: "Массово отключить",
            batchEnable: "Массово включить",
            batchMenu: "Массовые действия",
            batchPauseFamily: "Массово приостановить группу ({{count}})",
            batchRefreshLogin: "Массовое обновление входа ({{count}})",
            batchResumeFamily: "Массово возобновить группу ({{count}})",
            comfortableMode: "Обычный режим",
            compactMode: "Компактный режим",
            delete: "Удалить аккаунт",
            deleteConfirm: "Удалить аккаунт {{label}}?",
            disableAccount: "Отключить аккаунт",
            enableAccount: "Включить аккаунт",
            export: "Экспорт CSV",
            filter: "Фильтр списка",
            oneTimeNoGroupAction: "Для одноразовой сессии групповое действие недоступно",
            pauseGroup: "Отключить связанную группу",
            refreshAccounts: "Обновить",
            refreshLogin: "Обновить вход",
            refreshingAccounts: "Обновление",
            resumeGroup: "Включить связанную группу",
            selectAll: "Выбрать все отфильтрованные",
            selectOne: "Выбрать аккаунт {{label}}",
            selectedCount: "Выбрано {{count}}",
            viewDetails: "Посмотреть детали",
            edit: "Изменить свойства",
            refresh: "Принудительно обновить",
            suspend: "Приостановить",
            exportSuccess: "Экспорт успешен",
            refreshTriggered: "Обновление аккаунтов запущено"
        },
        columns: {
            actions: "Действия",
            account: "Аккаунт",
            added: "Добавлено",
            credentialType: "Тип учетных данных",
            health: "Состояние",
            id: "ID аккаунта",
            loginStatus: "Статус входа",
            nextRefresh: "Следующее обновление",
            plan: "План",
            provider: "Провайдер / Режим",
            rateLimit: "Использование Rate Limit",
            binding: "Привязка",
            unbound: "отвязано"
        },
        details: {
            description: "Описание",
            officialDescription: "Official OpenAI model metadata is read-only here. Manual override pricing can be edited below.",
            limitsTitle: "Название лимитов",
            noOauthStatus: "Нет статуса OAuth",
            oauthNotApplicable: "Оаутентификация не применима",
            oauthTitle: "Название авторизации",
            profileTitle: "Название профиля",
            rawTitle: "Необработанное название",
            tabAria: "Вкладки деталей аккаунта",
            tabs: {
                limits: "Пределы",
                oauth: "OAuth",
                profile: "Профиль",
                raw: "Сырой"
            },
            sections: {
                cache: "Кэш лимитов",
                connection: "Подключение",
                credentials: "Учетные данные",
                identity: "Идентификация",
                refresh: "Состояние обновления",
                subscription: "Подписка"
            },
            fields: {
                email: "Email",
                label: "Метка",
                mode: "Режим",
                accountId: "ID аккаунта",
                enabled: "Состояние включения",
                baseUrl: "Базовый URL",
                chatgptAccountId: "ID аккаунта ChatGPT",
                priority: "Приоритет",
                createdAt: "Время создания",
                bearerToken: "Bearer-токен",
                authProvider: "Провайдер авторизации",
                credentialKind: "Тип учетных данных",
                lastRefreshStatus: "Статус последнего обновления",
                effectiveEnabled: "Фактическое состояние",
                chatgptPlanType: "Тип плана ChatGPT",
                sourceType: "Тип источника",
                tokenFamilyId: "ID семейства токенов",
                tokenVersion: "Версия токена",
                tokenExpiresAt: "Срок действия токена",
                nextRefreshAt: "Следующее обновление",
                lastRefreshAt: "Последнее обновление",
                refreshReusedDetected: "Обнаружено повторное использование refresh",
                lastRefreshErrorCode: "Код ошибки последнего обновления",
                lastRefreshError: "Ошибка последнего обновления",
                rateLimitsFetchedAt: "Время получения лимитов",
                rateLimitsExpiresAt: "Срок действия лимитов",
                rateLimitsLastErrorCode: "Код последней ошибки лимитов",
                rateLimitsLastError: "Последняя ошибка лимитов",
                rawAccount: "Сырые данные аккаунта",
                rawOauthStatus: "Сырые данные статуса OAuth"
            }
        },
        filters: {
            active: "Активные",
            all: "Все",
            credential: "Тип учетных данных",
            credentialAll: "Все учетные данные",
            credentialAt: "AT",
            credentialRt: "RT",
            credentialUnknown: "Неизвестно",
            disabled: "Отключенные",
            mode: "Режим",
            modeAll: "Все режимы",
            modeApiKey: "API-ключ",
            modeOAuth: "OAuth-сессия",
            plan: "Фильтр плана",
            planAll: "Все планы",
            planUnknown: "Не сообщено",
            total: "Найдено {{count}}",
            suspended: "Остановленные"
        },
        messages: {
            batchAllFailed: "{{action}} не удалось",
            batchAllSuccess: "{{action}} завершено",
            batchPartialFailed: "{{failed}} операций не удалось{{error}}",
            batchPartialFailedTitle: "{{action}} частично не удалось",
            batchSuccessCount: "{{count}} успешно",
            batchUnknownError: "Массовая операция не удалась",
            deleteFailed: "Не удалось удалить аккаунт",
            deleteSuccess: "Аккаунт удален",
            disableFailed: "Не удалось отключить аккаунт",
            disableSuccess: "Аккаунт отключен",
            enableFailed: "Не удалось включить аккаунт",
            enableSuccess: "Аккаунт включен",
            exportSuccess: "Экспорт успешен",
            pauseFamilyFailed: "Не удалось приостановить связанные аккаунты",
            pauseFamilySuccess: "Связанные аккаунты приостановлены",
            rateLimitPollingTimeout: "Тайм-аут опроса обновления ограничения скорости.",
            rateLimitRefreshFailedStatus: "Задание обновления ограничения скорости не выполнено, статус = {{status}}",
            rateLimitRefreshFailedSummary: "Не удалось выполнить задание обновления ограничения скорости: {{summary}}",
            batchRefreshStarted: "Запущено обновление входа для {{count}} аккаунтов",
            refreshFailed: "Не удалось обновить вход",
            refreshFailedStatus: "Не удалось обновить вход, статус={{status}}",
            refreshFailedSummary: "Не удалось обновить вход: {{summary}}",
            refreshPollingTimeout: "Тайм-аут опроса задания обновления входа.",
            refreshJobId: "Идентификатор вакансии: {{jobId}}",
            refreshJobSummary: "Идентификатор вакансии: {{jobId}} · {{processed}}/{{total}}",
            refreshListFailed: "Не удалось обновить использование",
            refreshListSuccess: "Использование обновлено",
            refreshSuccess: "Обновление входа завершено",
            requestFailed: "Запрос не удался. Попробуйте позже.",
            resumeFamilyFailed: "Не удалось возобновить связанные аккаунты",
            resumeFamilySuccess: "Связанные аккаунты возобновлены",
            toggleUnsupported: "Текущая версия бэкенда не поддерживает вкл./откл. аккаунтов. Обновите control-plane.",
            refreshTriggered: "Обновление входа запущено"
        },
        rateLimitRefreshJobStatus: {
            queued: "В очереди",
            running: "Выполняется",
            completed: "Завершено",
            failed: "Ошибка",
            cancelled: "Отменено",
            unknown: "Неизвестно"
        },
        mode: {
            apiKey: "API-ключ",
            chatgptSession: "ЧатGPT OAuth",
            codexOauth: "Кодекс OAuth",
            unknown: "Другое"
        },
        nextRefresh: {
            none: "Не запланировано"
        },
        oauth: {
            kindShort: {
                oneTime: "AT",
                refreshRotatable: "RT",
                unknown: "Неизв."
            },
            loading: "Загрузка",
            notApplicable: "-",
            provider: {
                legacyBearer: "Старый bearer-токен",
                refreshToken: "Refresh token"
            },
            sourceType: {
                codex: "Codex",
                unknown: "Неизвестный источник"
            },
            status: {
                failed: "Ошибка",
                never: "Не обновлялся",
                ok: "В норме"
            },
            unknownError: "неизвестно",
            versionPrefix: "в",
            planPrefix: "План: ",
            kind: {
                refreshRotatable: "Аккаунт с обновляемым Refresh Token",
                oneTime: "Одноразовый Access Token аккаунт",
                unknown: "Неизвестный тип учетных данных"
            }
        },
        rateLimits: {
            labels: {
                fiveHours: "Лимит 5 часов",
                github: "GitHub",
                oneWeek: "Недельный лимит"
            },
            shortLabels: {
                fiveHours: "5h",
                oneWeek: "7d"
            },
            moreDetails: "Подробнее (+{{count}})",
            noReset: "Нет времени сброса",
            remainingPrefix: "Осталось",
            resetAt: "Сброс {{absolute}} ({{relative}})",
            unavailable: "Нет данных о лимитах",
            usedPrefix: "Использовано"
        },
        searchPlaceholder: "Поиск по email, метке и URL…",
        status: {
            active: "Активен",
            disabled: "Отключен"
        },
        subtitle: "Управление учетными данными API и биллингом.",
        syncing: "Синхронизация статуса аккаунтов…",
        title: "Пул аккаунтов"
    },
    billing: {
        columns: {
            balanceAfter: "Баланс после изменения",
            billingDetail: "Платежные данные",
            deductedCredits: "Списанные кредиты",
            deductionEvents: "События вычета",
            delta: "Дельта Кредиты",
            eventType: "Событие",
            model: "Модель",
            periodDay: "Дата",
            periodMonth: "Месяц",
            requestType: "Тип запроса",
            source: "Источник",
            timestamp: "Время"
        },
        exportCsv: "Экспортировать CSV",
        filters: {
            granularityAriaLabel: "Гранулярность биллинга",
            tenantAriaLabel: "Фильтр по арендатору",
            tenantPlaceholder: "Выберите арендатора"
        },
        granularity: {
            day: "Ежедневно",
            month: "Ежемесячно"
        },
        ledger: {
            codeLabels: {
                accountDeactivated: "Аккаунт деактивирован",
                billingUsageMissing: "Отсутствуют поля расчета использования.",
                failoverExhausted: "Повторная попытка/отработка отказа исчерпаны",
                noUpstreamAccount: "Нет доступной учетной записи восходящего канала",
                streamPreludeError: "Ошибка прелюдии потока",
                tokenInvalidated: "Токен признан недействительным",
                transportError: "Ошибка восходящей сети",
                upstreamRequestFailed: "Восходящий запрос не выполнен",
                unknown: "Неизвестно"
            },
            details: {
                accrued: "Начислено: {{value}} кредитов",
                adjustment: "Корректировка: {{value}}",
                extraCharge: "Дополнительная плата: {{value}} кредитов.",
                failoverAction: "Действие: {{action}}",
                failure: "Ошибка: {{summary}}",
                failurePrefix: "Отказ:",
                source: "Источник: {{source}}",
                upstreamStatus: "Апстрим {{status}}",
                tokenSettlement: "Расчет токеном: ввод {{input}} + кэшированный {{cached}} + вывод {{output}}",
                unitPrice: "Цена за единицу: {{prices}} кредитов/1 миллион токенов."
            },
            empty: "Записей в бухгалтерской книге пока нет.",
            failoverActions: {
                crossAccountFailover: "Отработка отказа между аккаунтами",
                retrySameAccount: "Повторить ту же учетную запись",
                returnFailure: "Ошибка возврата",
                unknown: "Неизвестно"
            },
            releaseReasons: {
                billingSettleFailed: "Оплата по счету не удалась",
                failoverExhausted: "Повторная попытка/отработка отказа исчерпаны",
                invalidUpstreamUrl: "Неверная конфигурация URL-адреса восходящего потока.",
                noUpstreamAccount: "Нет доступной учетной записи восходящего канала",
                streamPreludeError: "Ошибка прелюдии потока",
                streamUsageMissing: "Использование потока отсутствует",
                transportError: "Ошибка восходящей сети",
                upstreamRequestFailed: "Восходящий запрос не выполнен",
                unknown: "Неизвестно"
            },
            showRaw: "Показать необработанные записи",
            subtitle: "Отфильтровано по текущему арендатору.",
            title: "Записи в бухгалтерской книге",
            requestTypes: {
                nonStream: "Не поток",
                stream: "Поток",
                unknown: "-"
            },
            tokenSegments: {
                cached: "Кэшированный",
                input: "Вход",
                output: "Выход"
            }
        },
        messages: {
            rechargeFailedTitle: "Пополнение не удалось",
            rechargeSuccessDetail: "+{{amount}}, баланс {{balance}}",
            rechargeSuccessTitle: "Пополнение прошло успешно",
            retryLater: "Пожалуйста, повторите попытку позже"
        },
        recharge: {
            amountAriaLabel: "Сумма пополнения",
            amountPlaceholder: "Кредиты пополнения (микрокредиты)",
            reasonAriaLabel: "Причина пополнения",
            reasonPlaceholder: "Причина пополнения счета",
            submit: "Выполнить перезарядку",
            subtitle: "Пополните счет текущего выбранного арендатора.",
            title: "Пополнение баланса администратора"
        },
        snapshot: {
            empty: "Снимков поселений пока нет.",
            subtitle: "Совокупные события вычета до {{granularity}} для расчета и сверки в конце месяца.",
            title: "Снимок поселения"
        },
        subtitle: "Основной вид: кредитная книга (фактические расходы) с административной фильтрацией на уровне клиента.",
        summary: {
            currentBalance: "Текущий баланс",
            deductionHint: "Учитываются только отрицательные события вычетов из бухгалтерской книги.",
            monthConsumed: "Потребление в этом месяце",
            todayConsumed: "Сегодняшнее потребление",
            unitCredits: "Единица: кредиты"
        },
        title: "Биллинговый центр",
        trend: {
            noData: "Данных о тенденциях пока нет.",
            seriesConsumed: "Потребленные кредиты",
            subtitle: "Показать вычеты из бухгалтерской книги, агрегированные по {{granularity}}.",
            title: "Тенденция потребления"
        }
    },
    common: {
        cancel: "Отмена",
        close: "Закрывать",
        collapseSidebar: "Свернуть боковую панель",
        confirm: "Подтвердить",
        delete: "Удалить",
        edit: "Редактировать",
        expand: "Expand",
        collapse: "Collapse",
        expandSidebar: "Развернуть боковую панель",
        loading: "Загрузка…",
        logout: "Выйти",
        no: "Нет",
        noData: "Нет данных.",
        openMenu: "Открыть меню",
        refresh: "Обновить",
        skipToMainContent: "Перейти к основному содержанию",
        table: {
            firstPage: "Первая страница",
            go: "Перейти",
            jumpToPage: "Перейти к странице",
            lastPage: "Последняя страница",
            nextPage: "Следующая страница",
            pageOf: "Страница {{page}} / {{total}}",
            previousPage: "Предыдущая страница",
            range: "{{start}}-{{end}} / {{total}}",
            rowsPerPage: "Строк на странице",
            searchLabel: "Поиск по таблице",
            searchPlaceholder: "Поиск по текущему списку…"
        },
        toggleLanguage: "Сменить язык",
        toggleTheme: "Сменить тему",
        yes: "Да",
        save: "Сохранить",
        search: "Поиск…",
        routeLoading: "Загрузка страницы…",
        status: {
            healthy: "В норме",
            degraded: "Ухудшено",
            offline: "Офлайн",
            disabled: "Отключено",
            available: "Доступно"
        }
    },
    config: {
        controlPlane: {
            authValidateUrl: "URL проверки авторизации",
            dataPlaneUrl: "URL сервиса пересылки",
            desc: "Настройте, как control-plane обращается к сервисам пересылки",
            listen: "Адрес прослушивания control-plane (только чтение)",
            title: "Подключение control-plane"
        },
        refreshSettings: {
            desc: "Включение и интервал автоматического обновления входа",
            enableDesc: "Если отключить, истекающие учетные данные больше не будут обновляться автоматически.",
            enableLabel: "Включить автообновление",
            intervalSec: "Интервал обновления (сек)",
            notes: "Примечания",
            title: "Настройки автообновления"
        },
        runtimeHint: {
            desc: "Изменения применяются сразу. После перезапуска приоритет снова у переменных окружения и config.toml.",
            title: "Подсказка по runtime-настройкам"
        },
        save: "Сохранить изменения",
        subtitle: "Настройки среды выполнения и глобальные переменные",
        success: "Конфигурация успешно сохранена.",
        title: "Конфигурация",
        reload: {
            title: "Включена перезагрузка во время выполнения",
            desc: "Изменения кэширования и тайм-аутов будут синхронизированы глобально и вступят в силу немедленно, без необходимости перезапуска служб."
        },
        system: {
            title: "Стратегия системы",
            desc: "Настройте глобальный контекст операций.",
            maintenance: "Режим обслуживания",
            maintenanceDesc: "Глобально отклонять все новые запросы, удерживая 503.",
            logRetention: "Хранение журналов (Дней)",
            logRetentionDesc: "Скользящее окно для хранения трассировок clickhouse."
        },
        network: {
            title: "Сетевое профилирование",
            desc: "Определите глобальные ограничения скорости для предотвращения блокировки выше по потоку.",
            tpm: "Глобальное кол-во токенов в мин",
            tpmDesc: "Максимальное количество одновременных запросов к конечным точкам ИИ.",
            scaling: "Масштабирование облачных ресурсов",
            scalingDesc: "Разрешить динамические границы выделения узлов."
        }
    },
    dashboard: {
        actions: {
            viewBilling: "Посмотреть платежные данные",
            viewLogs: "Просмотр журналов запросов"
        },
        alerts: {
            checkRoutes: "Проверить маршруты",
            columns: {
                action: "Рекомендуемое действие",
                message: "Сообщение",
                severity: "Уровень",
                source: "Источник",
                status: "Статус",
                time: "Время"
            },
            empty: "Система работает стабильно.",
            resolve: "Решено",
            searchPlaceholder: "Поиск по оповещениям…",
            subtitle: "Требуется вмешательство",
            title: "Внимание",
            usageRepoUnavailable: "Хранилище аналитики использования недоступно",
            dataPlaneDisconnected: "Соединение с data plane потеряно",
            now: "Сейчас",
            severity: {
                critical: "Критично",
                warning: "Предупреждение",
                info: "Инфо"
            },
            source: {
                data_plane: "Плоскость данных",
                usage_repo: "Репо использования"
            },
            status: {
                open: "Открыто",
                resolved: "Решено"
            }
        },
        currentScope: "Текущий: {{scope}}",
        filters: {
            apiKeyAriaLabel: "Фильтр API-ключа",
            apiKeyPlaceholder: "Выберите ключ API",
            range: {
                last24Hours: "Последние 24 часа",
                last30Days: "Последние 30 дней",
                last7Days: "Последние 7 дней"
            },
            rangeAriaLabel: "Диапазон времени",
            scopeAriaLabel: "Фильтр области",
            tenantAriaLabel: "Фильтр по арендатору",
            tenantPlaceholder: "Выберите арендатора"
        },
        kpi: {
            activeApiKeysInRange: "Активные ключи API (выбранный диапазон)",
            accounts: "Аккаунты",
            accountsDesc: "Операционный показатель только для администратора",
            apiKeys: "API-ключи",
            apiKeysDesc: "Количество настроенных ключей в системе",
            avgFirstTokenSpeed: "Средняя скорость первого токена",
            avgFirstTokenSpeedDesc: "TTFT (точно для стрима / приблизительно для non-stream)",
            globalScope: "Глобальная область действия",
            rpm: "RPM",
            rpmDesc: "Запросов в минуту",
            requests: {
                apiKey: "Текущие запросы ключей API (выбранный диапазон)",
                global: "Всего запросов к аккаунту (выбранный диапазон)",
                tenant: "Текущие запросы ключей API клиента (выбранный диапазон)"
            },
            tenants: "Арендаторы",
            tenantsDesc: "Операционный показатель только для администратора",
            totalRequests: "Всего запросов",
            totalTokens: "Общий расход Token",
            totalTokensDesc: "Вход + кэш + выход + reasoning",
            tpm: "TPM",
            tpmDesc: "Token в минуту",
            running: "Работает",
            totalConfigured: "Всего настроено",
            uptime: "99.99% Выдано",
            reqs: "Запросов",
            reqsDesc: "+12.5% к прошлому месяцу",
            failed: "Ошибок",
            failedDesc: "+180 сегодня",
            avgLatency: "Средняя задержка",
            avgLatencyDesc: "-5 мс за неделю",
            activeTokens: "Активных токенов",
            activeTokensDesc: "+24 новых токена",
            oauthLabel: "OAuth"
        },
        scope: {
            apiKey: "Вид ключа API",
            global: "Глобальный вид",
            tenant: "Вид арендатора"
        },
        subtitle: "Показатели глобального шлюза",
        table: {
            apiKey: "API-ключ",
            requests: "Запросы"
        },
        modelDistribution: {
            description: "Топ моделей по числу запросов или расходу Token.",
            empty: "Пока нет данных о распределении моделей",
            modeRequests: "По запросам",
            modeTokens: "По Token",
            other: "Другое",
            title: "Распределение запросов по моделям"
        },
        tokenComponents: {
            cached: "Кэшированный ввод",
            input: "Ввод",
            output: "Вывод",
            reasoning: "Reasoning"
        },
        tokenTrend: {
            description: "Почасовой тренд Token по компонентам. Переключайте компоненты для фокуса.",
            empty: "Пока нет данных по тренду Token",
            title: "Тренд использования Token"
        },
        title: "Обзор системы",
        topApiKeys: {
            empty: "Данных о рейтинге пока нет",
            scopeDescription: "Область действия: {{scope}} / выбранное временное окно",
            title: "Лучшие ключи API"
        },
        trafficChart: {
            scope: {
                apiKey: "Область применения: текущие запросы ключей API.",
                global: "Область применения: глобальные запросы учетных записей + глобальные запросы ключей API клиента.",
                tenant: "Область применения: текущие запросы ключей API клиента."
            },
            series: {
                accountRequests: "Запросы на аккаунт",
                tenantApiKeyRequests: "Запросы ключей API арендатора",
                tenantApiKeyRequestsSingle: "Запросы ключей API арендатора"
            },
            title: "Трафик за сутки",
            subtitle: "Объем прошедших через шлюз пакетов",
            success: "Успешно",
            blocked: "Заблокировано"
        }
    },
    importJobs: {
        actions: {
            cancel: "Отменить",
            cancelJob: "Отменить задание",
            confirmCancelJob: "Отменить это задание импорта?",
            exportFailed: "Экспорт ошибок",
            refreshItems: "Обновить элементы",
            remove: "Удалить",
            retryFailed: "Повторить ошибки",
            removeFromList: "Удалить из списка"
        },
        detail: {
            columns: {
                error: "Ошибка",
                label: "Метка",
                line: "Строка",
                status: "Статус"
            },
            filterLabel: "Фильтр статуса",
            itemsEmpty: "Подходящих элементов нет.",
            itemsLoading: "Загрузка элементов задания…",
            jobIdLabel: "Идентификатор вакансии: {{jobId}}",
            loadMore: "Загрузить еще",
            loadedCount: "Загружено {{count}} записей",
            loadingMore: "Загрузка…",
            retryQuery: "Повторить запрос",
            searchPlaceholderModern: "Поиск по label / email / error…",
            selectHint: "Выберите задание для просмотра деталей.",
            summaryLoading: "Загрузка сводки задания…",
            title: "Детали задания",
            unreadable: "Невозможно прочитать это задание (возможно, истек срок или неверный ID).",
            searchPlaceholder: "Поиск по label / email / error"
        },
        dropzone: {
            acceptsNew: "Можно загрузить несколько .json/.jsonl за один раз. Бэкенд объединит их в одно задание.",
            creatingTitle: "Создание задания импорта…",
            selectFiles: "Выбрать файлы",
            titleNew: "Перетащите файлы импорта сюда",
            uploading: "Загрузка…",
            wait: "Пожалуйста, подождите, файл безопасно передается.",
            title: "Нажмите для загрузки или перетащите файл сюда",
            accepts: "Допускаются файлы <code>.csv</code> или <code>.txt</code>. Максимум 50 000 строк.",
            browse: "Обзор файлов",
            creatingDesc: "Подождите немного. После создания задание автоматически появится в очереди справа."
        },
        error: "Ошибка загрузки",
        manual: {
            add: "Добавить",
            placeholderModern: "Вставьте job_id…",
            title: "Ручное отслеживание задания",
            placeholder: "Вставьте job_id"
        },
        messages: {
            jobNotFound: "Задание не найдено или недоступно",
            queryFailed: "Запрос не выполнен",
            unknownError: "неизвестная ошибка",
            uploadFailedTitle: "Импорт не выполнен"
        },
        errors: {
            importFailed: "Импорт не выполнен",
            invalidRecord: "Некорректная запись",
            invalidRefreshToken: "Недействительный refresh_token",
            missingCredentials: "Отсутствуют учетные данные",
            oauthProviderNotConfigured: "OAuth-провайдер не настроен",
            rateLimited: "Сработало ограничение частоты",
            refreshTokenReused: "refresh_token уже использован",
            unknown: "Неизвестная ошибка импорта",
            upstreamNetworkError: "Сетевая ошибка апстрима",
            upstreamUnavailable: "Сервис апстрима недоступен"
        },
        metrics: {
            created: "Создано",
            failed: "Ошибки",
            processed: "Обработано",
            status: "Статус",
            throughput: "Пропускная способность",
            total: "Всего",
            updated: "Обновлено"
        },
        precheck: {
            createdNotice: "Задание импорта создано: {{id}}",
            defaultReady: "Проверка формата и размера пройдена.",
            duplicateBatch: "Эти файлы уже есть в списке ожидания.",
            duplicateName: "Обнаружено совпадение имен файлов. Проверьте источник перед импортом.",
            emptyPreview: "В предварительном просмотре нет валидных строк данных. Возможно, файл пустой.",
            firstLineInvalid: "Первая строка JSONL невалидна. Импорт может завершиться ошибкой.",
            firstLineObject: "Первая строка не является JSON-объектом. Импорт может завершиться ошибкой.",
            firstLineValid: "Проверка первой строки JSONL пройдена.",
            jsonEmpty: "JSON-файл пуст.",
            jsonInvalid: "JSON-файл невалиден. Импорт может завершиться ошибкой.",
            jsonValid: "Проверка структуры JSON пройдена.",
            noneImportable: "Нет файлов для импорта. Сначала исправьте блокирующие проблемы.",
            skipLargeJson: "Файл большой: локальный разбор JSON пропущен, проверка будет на бэкенде.",
            status: {
                invalid: "Блокирующее",
                ready: "Готово",
                warning: "Нужна проверка"
            }
        },
        progress: {
            done: "Завершено",
            etaLabel: "Осталось примерно: ",
            etaMinutes: "Около {{count}} мин.",
            jobIdLabel: "Идентификатор вакансии: {{jobId}}",
            lessThanMinute: "Менее 1 минуты",
            noJobSelected: "Создайте или выберите задание импорта, и здесь появится прогресс.",
            title: "Прогресс импорта в реальном времени",
            topErrors: "Основные ошибки"
        },
        queue: {
            columns: {
                jobId: "Идентификатор вакансии"
            },
            descRecent: "Очередь автоматически опрашивает статус. Нажмите задание, чтобы посмотреть элементы и ошибки.",
            emptyRecent: "Пока нет отслеживаемых заданий. Загрузите файлы или добавьте job_id вручную.",
            titleRecent: "Недавние задания импорта",
            tracked: "отслеживается локально",
            title: "Недавние задания",
            empty: "В этом сеансе пока не загружено ни одного задания.",
            card: {
                processed: "обработано",
                new: "новых",
                errors: "ошибок"
            }
        },
        status: {
            all: "Все",
            cancelled: "Отменено",
            completed: "Завершено",
            created: "Создано",
            failed: "Ошибка",
            pending: "Ожидает",
            processing: "Обработка",
            queued: "В очереди",
            running: "В работе",
            skipped: "Пропущено",
            updated: "Обновлено"
        },
        subtitleModern: "Добавьте файлы пачкой, проверьте перед импортом и запустите импорт в один клик с живым прогрессом и деталями ошибок.",
        template: {
            downloadJsonl: "Скачать шаблон",
            title: "Шаблон импорта",
            desc: "Загрузите наш рекомендуемый шаблон, чтобы столбцы строго соответствовали системным требованиям.",
            download: "Скачать шаблон CSV",
            titleNew: "Шаблон импорта",
            descNew: "Скачайте шаблон JSONL и заполните refresh_token для массового импорта."
        },
        title: "Пакетный импорт",
        validation: {
            fileTooLarge: "Файл {{name}} превышает 20 МБ. Разделите файл и повторите.",
            unsupportedFormat: "Файл {{name}} не поддерживается. Разрешены только .json / .jsonl.",
            format: "Допускаются только файлы .csv или .txt.",
            size: "Файл слишком большой. Максимальный размер 10 МБ."
        },
        workspace: {
            clearQueue: "Очистить список",
            columns: {
                action: "Действие",
                check: "Проверка",
                file: "Файл",
                size: "Размер",
                status: "Статус"
            },
            confirmClear: "Очистить текущий список файлов?",
            desc: "Перетащите или выберите файлы пачкой, выполните проверки и затем нажмите «Начать импорт».",
            empty: "Пока нет файлов в очереди. Перетащите файлы в область выше.",
            invalidFiles: "Блокирует: {{count}}",
            invalidHint: "{{count}} файлов заблокированы и будут исключены при импорте.",
            moreChecks: "доп. проверок",
            readyFiles: "Готово: {{count}}",
            readyHint: "Предпроверка завершена, можно запускать импорт.",
            startImportWithCount: "Начать импорт ({{count}} файлов)",
            stepCheck: "Проверка",
            stepCheckDesc: "Проверка формата, размера и дубликатов имен",
            stepImport: "Начать импорт",
            stepImportDesc: "Создать задание и смотреть прогресс",
            stepSelect: "Добавить файлы",
            title: "Рабочая область импорта файлов",
            totalFiles: "Файлов: {{count}}",
            totalSize: "Общий размер: {{size}}",
            warningFiles: "Проверить: {{count}}"
        },
        subtitle: "Безопасная загрузка учетных данных в файлах формата CSV/TXT."
    },
    oauthImport: {
        title: "Импорт через OAuth-вход",
        subtitle: "Войдите через Codex OAuth и импортируйте авторизованный аккаунт напрямую в пул.",
        start: {
            title: "Запустить вход Codex OAuth",
            description: "Создайте сессию входа, завершите OAuth-авторизацию и импорт выполнится автоматически."
        },
        form: {
            label: "Метка аккаунта (необязательно)",
            labelPlaceholder: "Если пусто, метка будет создана из email или ID аккаунта",
            baseUrl: "Базовый URL",
            priority: "Приоритет",
            enabled: "Включить аккаунт после импорта"
        },
        actions: {
            startLogin: "Начать OAuth-вход",
            reopenAuth: "Открыть OAuth-окно снова",
            goAccounts: "Перейти к аккаунтам",
            submitCallback: "Отправить callback URL"
        },
        status: {
            label: "Статус сессии",
            idle: "Не запущено",
            waiting_callback: "Ожидание callback",
            exchanging: "Обмен токена",
            importing: "Импорт аккаунта",
            completed: "Завершено",
            failed: "Ошибка",
            expired: "Истекло",
            sessionId: "ID сессии: {{id}}",
            callbackUrl: "Callback URL: {{url}}",
            expiresAt: "Истекает: {{time}}"
        },
        error: {
            failed: "Не удалось выполнить OAuth-импорт."
        },
        result: {
            success: "Аккаунт успешно импортирован.",
            accountId: "ID аккаунта: {{id}}",
            accountLabel: "Метка: {{label}}",
            email: "Email: {{email}}",
            created: "Создан",
            updated: "Обновлен"
        },
        manual: {
            title: "Резервный ручной callback",
            description: "Если автоматический callback недоступен, вставьте сюда полный redirect URL.",
            placeholder: "Вставьте полный callback URL с code/state…",
            hint: "Используйте только если автоматический callback не сработал."
        },
        notifications: {
            popupBlockedTitle: "Всплывающее окно заблокировано",
            popupBlockedDescription: "Разрешите всплывающие окна и снова откройте OAuth-окно.",
            sessionCreatedTitle: "Сессия создана",
            sessionCreatedDescription: "OAuth-окно открыто. Завершите вход, чтобы продолжить.",
            sessionCreateFailedTitle: "Не удалось создать сессию",
            manualSubmitTitle: "Callback отправлен",
            manualSubmitSuccess: "Ручной callback завершен, аккаунт импортирован.",
            manualSubmitAccepted: "Ручной callback принят в обработку.",
            manualSubmitFailedTitle: "Ошибка ручного callback",
            unknownError: "Неизвестная ошибка. Повторите позже."
        }
    },
    oauthProbe: {
        title: "Проба OAuth payload",
        subtitle: "Запустите отдельный вход через Codex OAuth, сохраните возвращенный payload и не импортируйте аккаунт в пул.",
        start: {
            title: "Запустить probe-сессию",
            description: "Создайте временную probe-сессию, завершите OAuth-авторизацию и просмотрите или скачайте полученный JSON."
        },
        form: {
            baseUrl: "Базовый URL"
        },
        actions: {
            startProbe: "Запустить OAuth Probe",
            reopenAuth: "Открыть OAuth-окно снова",
            downloadJson: "Скачать JSON",
            submitCallback: "Отправить callback URL"
        },
        status: {
            label: "Статус probe-сессии",
            idle: "Не запущено",
            waiting_callback: "Ожидание callback",
            exchanging: "Обмен токена",
            importing: "Обработка payload",
            completed: "Завершено",
            failed: "Ошибка",
            expired: "Истекло",
            sessionId: "ID сессии: {{id}}",
            callbackUrl: "Callback URL: {{url}}",
            expiresAt: "Истекает: {{time}}",
            memoryOnly: "Результат probe хранится только в памяти и исчезнет после истечения сессии или перезапуска сервиса."
        },
        error: {
            failed: "OAuth probe завершился ошибкой."
        },
        result: {
            success: "Probe payload успешно получен.",
            email: "Email: {{email}}",
            accountId: "ChatGPT Account ID: {{id}}",
            plan: "План: {{plan}}",
            expiresAt: "Токен истекает: {{time}}",
            accessTokenPreview: "Предпросмотр access token: {{value}}",
            refreshTokenPreview: "Предпросмотр refresh token: {{value}}"
        },
        payload: {
            title: "Полученный JSON",
            description: "Здесь показан результат probe, сохраненный в памяти после OAuth code exchange.",
            empty: "Probe payload пока не получен."
        },
        manual: {
            title: "Резервный ручной callback",
            description: "Если автоматический callback недоступен, вставьте сюда полный redirect URL.",
            placeholder: "Вставьте полный callback URL с code/state…",
            hint: "Используйте только если автоматический callback не сработал."
        },
        notifications: {
            popupBlockedTitle: "Всплывающее окно заблокировано",
            popupBlockedDescription: "Разрешите всплывающие окна и снова откройте OAuth-окно.",
            sessionCreatedTitle: "Probe-сессия создана",
            sessionCreatedDescription: "OAuth-окно открыто. Завершите вход, чтобы захватить payload.",
            sessionCreateFailedTitle: "Не удалось создать probe-сессию",
            manualSubmitTitle: "Callback отправлен",
            manualSubmitSuccess: "Ручной callback завершен, payload захвачен.",
            manualSubmitAccepted: "Ручной callback принят в обработку.",
            manualSubmitFailedTitle: "Ошибка ручного callback",
            unknownError: "Неизвестная ошибка. Повторите позже."
        }
    },
    login: {
        brand: {
            badge: "Control Plane Access",
            points: {
                audit: "Все критичные действия отслеживаются сквозным request id.",
                resilience: "Высокодоступная маршрутизация сохраняет стабильность админ-операций.",
                security: "Изоляция арендаторов и управление учетными данными включены по умолчанию."
            },
            subtitle: "Усиленная точка входа для системных администраторов.",
            title: "Управляйте Codex Pool с предсказуемой надежностью"
        },
        messages: {
            failed: "Не удалось войти. Проверьте имя пользователя и пароль.",
            invalidCredentials: "Неверное имя пользователя или пароль. Попробуйте снова.",
            sessionExpired: "Сеанс истёк. Пожалуйста, войдите снова."
        },
        password: "Пароль",
        passwordPlaceholder: "Введите пароль администратора",
        securityHint: "Подсказка безопасности: повторные ошибки связываются в журнале аудита.",
        submit: "Войти",
        subtitle: "Войдите под учетной записью администратора",
        title: "Консоль Codex-Pool",
        username: "Имя пользователя",
        usernamePlaceholder: "Введите имя пользователя администратора"
    },
    logs: {
        audit: {
            actionValues: {
                adminOperation: "Действие администратора",
                authOperation: "Действие аутентификации",
                requestOperation: "Операция запроса",
                tenantOperation: "Действие арендатора",
                unknown: "Неизвестное действие"
            },
            actorTypes: {
                adminUser: "Пользователь-администратор",
                apiKey: "API-ключ",
                system: "Система",
                tenantUser: "Пользователь арендатора",
                unknown: "Неизвестный исполнитель"
            },
            columns: {
                action: "Действие",
                actor: "Актер",
                createdAt: "Время",
                reason: "Причина",
                result: "Результат",
                target: "Цель",
                tenant: "Жилец"
            },
            description: "Область применения: события аудита плоскости управления (роль/действие/результат/цель/полезная нагрузка).",
            empty: "Данные журнала аудита недоступны.",
            filters: {
                actionPlaceholder: "Действие",
                actorIdPlaceholder: "Идентификатор актера",
                actorTypePlaceholder: "Тип актера",
                keywordPlaceholder: "Ключевое слово (причина/полезная нагрузка)",
                resultStatusPlaceholder: "Статус результата",
                actionAriaLabel: "Фильтр по действию",
                actorIdAriaLabel: "Фильтр ID актера",
                actorTypeAriaLabel: "Фильтр типа актера",
                keywordAriaLabel: "Фильтр по ключевому слову",
                rangeAriaLabel: "Диапазон времени",
                resultStatusAriaLabel: "Фильтр статуса результата",
                tenantAriaLabel: "Фильтр по арендатору"
            },
            resultStatuses: {
                denied: "Отклонено",
                failed: "Ошибка",
                ok: "Успешно",
                unknown: "Неизвестный результат"
            },
            title: "Журналы аудита"
        },
        columns: {
            level: "Уровень",
            message: "Сообщение",
            service: "Узел службы",
            timestamp: "Временная метка"
        },
        export: "Экспорт журналов",
        filters: {
            allTenants: "Все арендаторы"
        },
        focus: "Фокус:",
        levels: {
            all: "Все уровни",
            error: "Ошибка",
            info: "Информация",
            warn: "Предупреждение"
        },
        messages: {
            configUpdated: "Снимок runtime-конфигурации в памяти обновлён",
            empty: "Нет деталей сообщения",
            keyCreated: "Создан API-ключ {{keyId}}",
            keyPatched: "Для API-ключа {{keyId}} установлено enabled={{enabled}}",
            modelsLoaded: "Модели загружены из верхнего аккаунта {{label}}",
            modelsProbed: "Проверка моделей ({{trigger}}): через {{label}} протестировано {{tested}} (доступно {{available}}, недоступно {{unavailable}})",
            proxiesTested: "Проверено прокси-узлов: {{count}}",
            systemState: "Состояние системы запрошено: аккаунтов {{count}}",
            unmappedAction: "{{action}} · {{message}}"
        },
        range: {
            last24Hours: "Последние 24 часа",
            last30Days: "Последние 30 дней",
            last7Days: "Последние 7 дней"
        },
        request: {
            columns: {
                apiKey: "API-ключ",
                createdAt: "Время",
                errorCode: "Ошибка",
                latency: "Задержка (мс)",
                path: "Путь",
                requestId: "Идентификатор запроса",
                status: "Статус",
                tenant: "Жилец"
            },
            description: "Область: события запроса плоскости необработанных данных (статус/задержка/путь/клиент/ключ API/идентификатор запроса).",
            empty: "Данные журнала запросов недоступны.",
            filters: {
                apiKeyIdPlaceholder: "Идентификатор ключа API",
                keywordPlaceholder: "Ключевое слово (путь/ошибка/модель)",
                requestIdPlaceholder: "Идентификатор запроса",
                statusCodePlaceholder: "Код состояния (например, 500)",
                apiKeyAriaLabel: "Фильтр API-ключа",
                keywordAriaLabel: "Фильтр по ключевому слову",
                rangeAriaLabel: "Диапазон времени",
                requestIdAriaLabel: "Фильтр ID запроса",
                statusCodeAriaLabel: "Фильтр кода статуса",
                tenantAriaLabel: "Фильтр по арендатору"
            },
            title: "Запрос журналов"
        },
        search: "Поиск полезной нагрузки или службы…",
        subtitle: "Аудиторские следы в реальном времени и операционный контекст.",
        tabs: {
            audit: "Журналы аудита",
            request: "Запрос журналов",
            system: "Системные журналы"
        },
        title: "Системные журналы",
        waiting: "Ожидание входящих потоков…",
        actions: {
            systemState: "Состояние системы",
            configUpdate: "Обновление конфигурации",
            proxiesTest: "Проверка прокси",
            keyCreate: "Создание API-ключа",
            keyPatch: "Обновление API-ключа",
            modelsList: "Получение списка моделей",
            modelsProbe: "Проверка моделей",
            unknown: "Неизвестное действие ({{action}})"
        }
    },
    models: {
        actions: {
            copyModelId: "Копировать имя модели",
            createModel: "Создать модель",
            syncOpenAiCatalog: "Sync OpenAI catalog",
            probeAvailability: "Probe availability",
            openDetails: "Details",
            deleteModel: "Удалить модель",
            deletePricing: "Удалить цены",
            deleteBillingRule: "Delete rule",
            editBillingRule: "Edit rule",
            editModel: "Редактировать модель",
            probeNow: "Проверить вручную",
            saveModelProfile: "Сохранить профиль модели",
            savePricing: "Сохранить цену",
            saveBillingRule: "Save rule",
            newBillingRule: "New rule",
            search: "Поиск ID моделей…",
            sync: "Синхронизировать"
        },
        availability: {
            available: "Доступна",
            issueHint: "Показать причину недоступности",
            neverChecked: "Никогда не проверялась",
            noErrorDetail: "Нет деталей ошибки",
            unavailable: "Недоступна",
            unknown: "Не проверена",
            viewIssue: "Показать причину"
        },
        cache: {
            fresh: "кэш актуален",
            stale: "кэш устарел"
        },
        catalog: {
            customOnly: "Пользовательская модель",
            hidden: "Скрыта в каталоге",
            listed: "В каталоге",
            unlisted: "Вне каталога"
        },
        columns: {
            actions: "Подробности",
            availability: "Доступность",
            cachedInputPrice: "Кэшированная входная цена",
            context: "Context / Max output",
            modalities: "Modalities",
            syncedAt: "Synced",
            catalog: "Статус в каталоге",
            checkedAt: "Последняя проверка",
            id: "Имя модели",
            inputPrice: "Входная цена",
            outputPrice: "Выходная цена",
            pricingStatus: "Статус ценообразования",
            provider: "Провайдер / Hub"
        },
        description: "Здесь можно просматривать доступность моделей и управлять профилями моделей и ценами.",
        dialog: {
            description: "Измените профиль и цены в этом диалоговом окне. Сохраненные цены будут немедленно записаны обратно в список пула моделей.",
            officialDescription: "Official OpenAI model metadata is read-only here. Manual override pricing can be edited below.",
            tabListAriaLabel: "Вкладки профиля модели",
            titleWithId: "Профиль модели · {{modelId}}"
        },
        empty: "Модели не найдены.",
        emptySyncRequired: "No official catalog yet. Sync OpenAI catalog first.",
        emptyActions: {
            goAccounts: "Перейти к аккаунтам",
            importAccount: "Импортировать аккаунты"
        },
        errors: {
            deleteModelEntityFailed: "Не удалось удалить объект модели.",
            deleteModelPricingFailed: "Не удалось удалить цены на модели.",
            openAiCatalogSyncFailed: "Failed to sync OpenAI catalog.",
            deleteBillingRuleFailed: "Failed to delete tiered pricing rule.",
            modelIdRequired: "Идентификатор модели не может быть пустым.",
            probeFailed: "Проверка модели не удалась.",
            saveBillingRuleFailed: "Failed to save tiered pricing rule.",
            saveModelPricingFailed: "Не удалось сохранить цену модели.",
            saveModelProfileFailed: "Не удалось сохранить профиль модели."
        },
        filters: {
            allProviders: "Все провайдеры",
            providerLabel: "Фильтр провайдера"
        },
        form: {
            modelId: "Идентификатор модели",
            modelIdLockedHint: "Существующие модели не могут изменить идентификатор. Используйте «Создать модель», чтобы добавить новую.",
            modelIdPlaceholder: "Пример: gpt-5.3-кодекс",
            provider: "Поставщик",
            providerPlaceholder: "Пример: openai/custom",
            source: "Источник",
            sourceValues: {
                entityOnly: "Только сущность",
                pricingOnly: "Только цены",
                upstream: "вверх по течению"
            },
            visibility: "Видимость",
            visibilityPlaceholder: "Пример: список/скрыть"
        },
        hints: {
            cannotDeleteMissingPricing: "Текущая модель не имеет местных цен. Прежде чем удалять, сохраните цену.",
            cannotDeleteNonLocalEntity: "Текущая модель не является моделью локальной сущности, поэтому ее сущность нельзя удалить."
        },
        loadingHint: "Проверяем каталог и доступность. После завершения актуальный список моделей появится автоматически.",
        notice: {
            modelEntityDeleted: "Объект модели удален.",
            modelPricingDeleted: "Запись о ценах на модели удалена.",
            billingRuleDeleted: "Tiered pricing rule deleted.",
            modelPricingSaved: "Сохранена цена модели: {{model}}.",
            openAiCatalogSynced: "OpenAI catalog synced: {{count}} models updated.",
            billingRuleSaved: "Tiered pricing rule saved: {{model}}",
            modelProfileSaved: "Профиль модели сохранен: {{model}}",
            probeCompleted: "Исследование модели завершено. Последний пул моделей синхронизирован."
        },
        pricing: {
            cachedInputPrice: "Кэшированная входная цена",
            creditsPerMillionTokens: "кредиты / 1 миллион токенов",
            disabled: "Неполноценный",
            enablePricing: "Включить ценообразование",
            enabled: "Включено",
            inputPrice: "Входная цена",
            notConfigured: "Не настроено",
            outputPrice: "Выходная цена",
            perMillionTokensMicrocredits: "За 1М токенов, в микрокредитах",
            sectionTitle: "Цены на модели",
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
        probeSourceUnknown: "неизвестный аккаунт",
        probeSummary: "Кэш проверки: {{stale}}, проверено {{checkedAt}}, TTL {{ttlHours}} ч, источник {{source}}",
        syncHint: {
            notSynced: "OpenAI catalog has not been synced yet.",
            syncedAt: "Catalog synced {{time}}"
        },
        profile: {
            sectionTitle: "Профиль модели"
        },
        syncing: "Проверка возможностей эндпоинтов…",
        tabs: {
            pricing: "Цены",
            profile: "Профиль"
        },
        title: "Модели",
        subtitle: "Каталог и фактическая доступность моделей из пула аккаунтов.",
        detail: {
            title: "Детали модели",
            notFound: "Модель не найдена. Возможно, она удалена или скрыта фильтром.",
            httpStatus: "HTTP статус",
            error: "Ошибка",
            noError: "Нет деталей ошибки",
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
        accounts: "Пул аккаунтов",
        apiKeyGroups: "Group Management",
        apiKeys: "Ключи API",
        billing: "Биллинг",
        config: "Настройки",
        dashboard: "Обзор",
        groups: {
            analytics: "Аналитика",
            assets: "Активы",
            operations: "Операции",
            system: "Система"
        },
        importJobs: "Импорт",
        oauthImport: "Импорт через OAuth",
        oauthProbe: "Проба OAuth payload",
        logs: "Журнал",
        mainNavigation: "Основная навигация",
        models: "Модели",
        online: "В сети",
        openNavigation: "Открыть навигацию",
        proxies: "Прокси",
        system: "Состояние",
        tenants: "Пул арендаторов",
        usage: "Статистика",
        cleanup: "Очистка",
        closeNavigation: "Закрыть навигацию"
    },
    notifications: {
        dismiss: "Закрыть уведомление",
        loginFailed: {
            title: "Не удалось войти"
        },
        sessionExpired: {
            title: "Сеанс истёк",
            description: "Войдите снова, чтобы продолжить."
        }
    },
    errors: {
        common: {
            failed: "Ошибка",
            network: "Ошибка сети. Проверьте подключение.",
            timeout: "Время ожидания истекло. Повторите попытку позже."
        },
        api: {
            unauthorized: "Не авторизовано. Войдите снова.",
            invalidRequest: "Некорректный запрос.",
            notFound: "Ресурс не найден.",
            serviceUnavailable: "Сервис недоступен.",
            internalError: "Внутренняя ошибка сервера.",
            oauthProviderNotConfigured: "Провайдер OAuth не настроен.",
            oauthCallbackListenerUnavailable: "Слушатель OAuth callback недоступен.",
            invalidRefreshToken: "Refresh token недействителен или истёк.",
            refreshTokenReused: "Refresh token был повторно использован. Получите актуальный refresh token.",
            refreshTokenRevoked: "Refresh token был отозван.",
            oauthMissingClientId: "OAuth настроен неверно (нет client_id).",
            oauthUnauthorizedClient: "OAuth клиент не авторизован.",
            upstreamUnavailable: "Вышестоящий сервис недоступен.",
            upstreamNetworkError: "Сетевая ошибка вышестоящего сервиса.",
            oauthExchangeFailed: "Не удалось выполнить OAuth обмен."
        },
        http: {
            badRequest: "Некорректный запрос",
            unauthorized: "Не авторизовано",
            forbidden: "Доступ запрещён",
            notFound: "Не найдено",
            conflict: "Конфликт",
            payloadTooLarge: "Слишком большой запрос",
            rateLimited: "Превышен лимит",
            internalServerError: "Ошибка сервера",
            badGateway: "Плохой шлюз",
            serviceUnavailable: "Сервис недоступен",
            gatewayTimeout: "Тайм-аут шлюза"
        }
    },
    proxies: {
        check: "Запустить проверку",
        columns: {
            actions: "Действия",
            health: "Здоровье",
            lastPing: "Последний Ping",
            latency: "Ср. задержка",
            url: "URL прокси-узла",
            weight: "Маршрутизация"
        },
        empty: "Внутренние прокси-серверы не настроены.",
        filters: {
            all: "Все узлы",
            degraded: "Ухудшено",
            disabled: "Отключено",
            healthy: "В норме",
            label: "Фильтр состояния",
            offline: "Офлайн"
        },
        health: {
            degraded: "Ухудшено",
            disabled: "Отключено",
            healthy: "В норме",
            offline: "Офлайн"
        },
        loading: "Сканирование топологии сети…",
        manage: "Управление",
        pending: "В ожидании",
        retry: "Повторить",
        searchPlaceholder: "Поиск URL узла или ярлыка…",
        subtitle: "Управление узлами обратного прокси и топологией маршрутизации трафика.",
        title: "Прокси-узлы"
    },
    system: {
        columns: {
            component: "Компонент",
            details: "Подробности",
            status: "Статус",
            uptime: "Аптайм",
            version: "Версия"
        },
        components: {
            controlPlane: "Панель управления",
            dataPlane: "Маршруты данных",
            usageRepo: "Хранилище использования"
        },
        details: {
            analyticsUnavailable: "Аналитика недоступна",
            apiActive: "Шлюз API активен",
            checkingAPI: "Проверка API…",
            dbConnected: "БД временных рядов подключена",
            endpointsResponding: "Прокси-серверы отвечают"
        },
        labels: {
            local: "Локально",
            remote: "Удаленно",
            storage: "Хранилище",
            version: "Версия:",
            uptime: "Аптайм"
        },
        observability: {
            badges: {
                failoverOff: "Failover: выкл",
                failoverOn: "Failover: вкл",
                failoverWait: "Ожидание failover {{value}} мс",
                quickRetry: "Быстрый ретрай ≤ {{value}}",
                retryPoll: "Интервал опроса {{value}} мс",
                sharedCacheOff: "Общий кэш: выкл",
                sharedCacheOn: "Общий кэш: вкл",
                stickyConflictAvoidOff: "Избежание sticky-конфликтов: выкл",
                stickyConflictAvoidOn: "Избежание sticky-конфликтов: вкл"
            },
            hints: {
                billingPreauthCaptureMissingTotal: "Всего не хватает данных для предварительной аутентификации платежных данных",
                billingPreauthErrorRatioAvg: "Средний коэффициент ошибок при предварительной аутентификации для выставления счетов",
                billingPreauthErrorRatioP95: "Коэффициент ошибок предварительной аутентификации при выставлении счетов p95",
                billingPreauthTopModelP95: "Предварительная аутентификация для выставления счетов, топ-модель p95",
                billingReconcileAdjust: "Сколько авто-корректировок баланса выполнено сверкой.",
                billingReconcileFailed: "Сколько операций сверки завершились ошибкой.",
                billingReconcileReleased: "Сколько авторизаций сверка автоматически закрыла.",
                billingReconcileScanned: "Сколько фактов сверки просмотрено из request_log и ledger.",
                billingReleaseWithoutCaptureRatio: "Выставление счетов без коэффициента захвата",
                billingSettleCompleteRatio: "Полный коэффициент расчета платежа",
                cacheHitRate: "Суммарная доля попаданий local + shared sticky-кэша.",
                failoverAttempts: "Общее число попыток переключения между аккаунтами.",
                failoverExhausted: "Сколько запросов упало после исчерпания retry/failover бюджета.",
                failoverSuccess: "Сколько запросов восстановлено после переключения аккаунта.",
                failoverSuccessRate: "Доля успешных переключений среди всех попыток failover.",
                sameAccountRetry: "Сколько быстрых ретраев сделано до переключения аккаунта.",
                stickyHitRate: "Доля попаданий по sticky-привязке сессий."
            },
            metrics: {
                billingPreauthCaptureMissingTotal: "Всего не хватает данных для предварительной аутентификации платежных данных",
                billingPreauthErrorRatioAvg: "Средний коэффициент ошибок при предварительной аутентификации для выставления счетов",
                billingPreauthErrorRatioP95: "Коэффициент ошибок предварительной аутентификации при выставлении счетов p95",
                billingPreauthTopModelP95: "Предварительная аутентификация для выставления счетов, топ-модель p95",
                billingReconcileAdjust: "Корректировки billing-reconcile",
                billingReconcileFailed: "Ошибки billing-reconcile",
                billingReconcileReleased: "Освобождения billing-reconcile",
                billingReconcileScanned: "Сканировано billing-reconcile",
                billingReleaseWithoutCaptureRatio: "Выставление счетов без коэффициента захвата",
                billingSettleCompleteRatio: "Полный коэффициент расчета платежа",
                cacheHitRate: "Доля попаданий routing-кэша",
                failoverAttempts: "Попытки failover",
                failoverExhausted: "Failover исчерпан",
                failoverSuccess: "Успешные failover",
                failoverSuccessRate: "Успешность failover",
                sameAccountRetry: "Быстрые ретраи в том же аккаунте",
                stickyHitRate: "Доля попаданий sticky"
            },
            na: "Н/Д",
            subtitle: "Показатели автопереключения data-plane, sticky-сессий и эффективности кэша.",
            title: "Наблюдаемость failover и кэша",
            unavailableDesc: "Проверьте доступ и права control-plane к /internal/v1/debug/state.",
            unavailableLoading: "Ожидание свежего debug-снимка...",
            unavailableTitle: "Недоступен debug state data-plane"
        },
        searchPlaceholder: "Поиск компонента, версии или деталей…",
        status: {
            checking: "Проверка",
            degraded: "Ухудшено",
            healthy: "В норме",
            offline: "Офлайн",
            unknown: "Неизвестно"
        },
        subtitle: "Зависимости инфраструктуры и самопроверка работоспособности.",
        title: "Состояние системы"
    },
    tenantApiKeys: {
        actions: {
            changeGroup: "Change Group",
            disable: "Запрещать",
            enable: "Давать возможность"
        },
        columns: {
            actions: "Действия",
            group: "Group",
            ipAllowlist: "Белый список IP-адресов",
            modelAllowlist: "Белый список моделей",
            name: "Имя",
            prefix: "Префикс",
            status: "Статус"
        },
        create: {
            description: "Описание",
            groupLabel: "API key group",
            ipAllowlistAriaLabel: "Список разрешенных IP",
            ipAllowlistPlaceholder: "Заполнитель белого списка IP-адресов",
            modelAllowlistAriaLabel: "Список разрешенных моделей",
            modelAllowlistPlaceholder: "Заполнитель белого списка модели",
            nameAriaLabel: "Имя ключа",
            namePlaceholder: "Заполнитель имени",
            submit: "Представлять на рассмотрение",
            title: "Заголовок"
        },
        list: {
            description: "Описание",
            empty: "API-ключей пока нет.",
            searchPlaceholder: "Поиск API-ключей по имени или префиксу",
            title: "Заголовок"
        },
        messages: {
            createFailed: "Создать не удалось",
            createSuccess: "Создать успех",
            plaintextShownOnce: "Открытый текст отображается один раз",
            retryLater: "Повторить попытку позже",
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
            disabled: "Неполноценный",
            enabled: "Включено",
            groupInvalid: "Group invalid"
        },
        subtitle: "Субтитры"
    },
    tenantApp: {
        appName: "Арендатор пула Кодекса",
        auth: {
            actions: {
                backToLogin: "Назад ко входу",
                login: "Войти",
                openForgot: "Забыли пароль?",
                register: "Зарегистрироваться",
                resetPassword: "Сбросить пароль",
                sendResetCode: "Отправить код сброса",
                switchToLogin: "Уже есть аккаунт? Войти",
                switchToRegister: "Нет аккаунта? Зарегистрироваться",
                verifyEmail: "Подтвердить адрес электронной почты"
            },
            brand: {
                badge: "Tenant Workspace Access",
                points: {
                    audit: "Решения по политике и биллингу наблюдаемы на всём пути.",
                    resilience: "Маршрутизация с учётом failover поддерживает доступность.",
                    security: "Сессии и учетные данные изолированы по арендаторам."
                },
                subtitle: "После входа управляйте использованием, биллингом и ключами в едином защищенном рабочем пространстве.",
                title: "Стабильный доступ для enterprise AI-операций"
            },
            error: {
                invalidCredentialsOrUnverified: "Не удалось войти в систему: неверный адрес электронной почты или пароль, либо адрес электронной почты еще не подтвержден.",
                loginFailed: "Не удалось войти в систему.",
                passwordMismatch: "Пароль и подтверждение пароля не совпадают.",
                passwordResetFailed: "Сбросить пароль не удалось.",
                registerFailed: "Регистрация не удалась.",
                sendResetCodeFailed: "Не удалось отправить код сброса.",
                verificationFailed: "Проверка не удалась."
            },
            fields: {
                confirmPassword: "Подтверждение пароля",
                email: "Электронная почта",
                newPassword: "Новый пароль",
                password: "Пароль",
                passwordMin8: "Пароль (минимум 8 символов)",
                resetCode: "Сбросить код",
                tenantName: "Имя арендатора",
                verificationCode: "Проверочный код"
            },
            forgot: {
                drawerHint: "После отправки кода снизу появляется drawer с полями «код сброса + новый пароль».",
                stepResetPassword: "Установить новый пароль",
                stepSendCode: "Отправить код"
            },
            notice: {
                emailVerified: "Проверка электронной почты прошла успешно. Пожалуйста, войдите в систему с помощью этой учетной записи.",
                loginSuccess: "Вход успешен.",
                passwordResetSuccess: "Сброс пароля успешен. Пожалуйста, войдите снова.",
                registerDebugCode: "Регистрация прошла успешно, код подтверждения (отладка): {{code}}",
                registerSuccess: "Регистрация прошла успешно. Введите код подтверждения электронной почты, чтобы активировать свою учетную запись.",
                resetCodeDebug: "Код сброса пароля (отладка): {{code}}",
                resetCodeSentIfExists: "Если адрес электронной почты существует, будет отправлен код сброса.",
                sessionExpired: "Сессия арендатора истекла. Пожалуйста, войдите снова.",
                verifyCodeHint: "Не получили код? Подождите 60 секунд и отправьте повторно."
            },
            placeholders: {
                confirmPassword: "Повторите пароль",
                email: "name@company.com",
                newPassword: "Введите новый пароль",
                password: "Введите пароль",
                resetCode: "Введите код сброса",
                tenantName: "Введите имя арендатора",
                verificationCode: "Введите код подтверждения"
            },
            sections: {
                authSubtitle: "Переключайте вход и регистрацию в одной карточке.",
                forgotPasswordTitle: "Сбросить пароль",
                forgotPasswordSubtitle: "Двухшаговый drawer-процесс: сначала код, затем новый пароль.",
                loginTitle: "Вход арендатора",
                registerTitle: "Регистрация арендатора",
                verifyEmailSubtitle: "Введите код из письма, чтобы активировать аккаунт.",
                verifyEmailTitle: "Проверка электронной почты"
            },
            social: {
                comingSoon: "Вход через сторонние сервисы (скоро)",
                github: "GitHub",
                google: "Google"
            },
            tabs: {
                login: "Войти",
                register: "Регистрация"
            }
        },
        loadingPortal: "Загрузка портала арендатора…",
        menu: {
            analytics: "Аналитика",
            apiKeys: "API-ключи",
            assets: "Ресурсы",
            billing: "Биллинговый центр",
            dashboard: "Панель управления",
            logs: "Журналы",
            usage: "Использование"
        }
    },
    tenantBilling: {
        actions: {
            dailyCheckin: "Ежедневная регистрация",
            exportCsv: "Экспорт CSV-файла"
        },
        failoverAction: {
            crossAccountFailover: "Переключение при отказе между учетными записями",
            retrySameAccount: "Повторить ту же учетную запись",
            returnFailure: "Ошибка возврата",
            unknown: "Неизвестно"
        },
        failureReason: {
            accountDeactivated: "Аккаунт деактивирован",
            billingUsageMissing: "Использование биллинга отсутствует",
            failoverExhausted: "Аварийное переключение исчерпано",
            noUpstreamAccount: "Нет вышестоящей учетной записи",
            streamPreludeError: "Ошибка прелюдии потока",
            tokenInvalidated: "Токен недействителен",
            transportError: "Ошибка транспорта",
            upstreamRequestFailed: "Восходящий запрос не выполнен",
            unknown: "Неизвестно"
        },
        filters: {
            day: "День",
            dayShort: "Короткий день",
            granularityAriaLabel: "Гранулярность биллинга",
            month: "Месяц",
            monthShort: "Месяц короткий"
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
                balanceAfter: "Баланс после",
                delta: "Дельта",
                detail: "Деталь",
                event: "Событие",
                model: "Модель",
                requestType: "Тип запроса",
                time: "Время"
            },
            description: "Описание",
            detail: {
                charged: "Заряжено",
                extraCharge: "Дополнительная плата",
                failoverAction: "Действие аварийного переключения",
                failure: "Отказ",
                failureKeyword: "Ключевое слово «Отказ»",
                failureSummary: "{{failure}}（{{reason}}）",
                reconcileAdjust: "Согласовать Корректировать",
                source: "Источник",
                tokenSettle: "Расчет токенов",
                unitPrice: {
                    cached: "Кэшированный",
                    input: "Вход",
                    output: "Выход",
                    summary: "Краткое содержание"
                },
                upstreamStatus: "Апстрим {{status}}"
            },
            empty: "Пустой",
            requestTypes: {
                nonStream: "Не поток",
                stream: "Поток",
                unknown: "-"
            },
            showRaw: "Показать сырье",
            title: "Заголовок"
        },
        messages: {
            checkinFailed: "Регистрация не удалась",
            checkinReward: "Награда за регистрацию",
            checkinSuccess: "Регистрация прошла успешно",
            retryLater: "Повторить попытку позже"
        },
        releaseReason: {
            billingSettleFailed: "Сопоставление счетов не выполнено",
            failoverExhausted: "Аварийное переключение исчерпано",
            invalidUpstreamUrl: "Неверный URL-адрес восходящего потока",
            noUpstreamAccount: "Нет вышестоящей учетной записи",
            streamPreludeError: "Ошибка прелюдии потока",
            streamUsageMissing: "Использование потока отсутствует",
            transportError: "Ошибка транспорта",
            upstreamRequestFailed: "Восходящий запрос не выполнен",
            unknown: "Неизвестно"
        },
        snapshot: {
            columns: {
                consumed: "Потреблено",
                date: "Дата",
                eventCount: "Количество событий",
                month: "Месяц"
            },
            description: "Описание",
            empty: "Пустой",
            title: "Заголовок"
        },
        subtitle: "Субтитры",
        summary: {
            balance: "Баланс",
            monthConsumed: "Месяц потребления",
            negativeOnly: "Только отрицательный",
            todayConsumed: "Сегодня потребляется",
            unitCredits: "Единичные кредиты"
        },
        title: "Заголовок",
        tokenSegment: {
            cached: "Кэшированный",
            input: "Вход",
            output: "Выход"
        },
        trend: {
            description: "Описание",
            empty: "Пустой",
            series: {
                consumed: "Потреблено"
            },
            title: "Заголовок"
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
            manageApiKeys: "Управлять API-ключами",
            refresh: "Обновить",
            viewBilling: "Посмотреть платежные данные",
            viewRequestLogs: "Просмотр журналов запросов"
        },
        kpi: {
            avgFirstTokenSpeed: "Средняя скорость первого токена",
            avgFirstTokenSpeedDesc: "TTFT (точно для стрима / приблизительно для non-stream)",
            rpm: "RPM",
            rpmDesc: "Запросов в минуту",
            totalRequests: "Всего запросов",
            totalRequestsDesc: "Выбранный диапазон времени",
            totalTokens: "Общий расход Token",
            totalTokensDesc: "Вход + кэш + выход + reasoning",
            tpm: "TPM",
            tpmDesc: "Token в минуту"
        },
        cards: {
            activeKeys: {
                description: "Примечание: учитываются только ключи с запросами.",
                title: "Количество активных ключей API (выбранный период)"
            },
            availableKeys: {
                description: "На основе настроек управления ключами клиента",
                title: "Доступные на данный момент ключи API"
            },
            keyEnableRate: {
                description: "Доля включенных: {{rate}}% ({{enabled}} / {{total}} ключей)",
                title: "Доля включенных ключей"
            },
            peakHour: {
                description: "Окно с максимальным числом запросов",
                empty: "Нет данных",
                title: "Пиковый час"
            },
            requestVelocity: {
                description: "Среднее количество запросов в час за выбранный период",
                title: "Скорость запросов (в час)"
            },
            totalRequests: {
                scopeAllKeys: "/ все ключи",
                scopePrefix: "Объем: текущий арендатор",
                scopeSingleKey: "/ один ключ",
                title: "Общее количество запросов ключей API клиента (выбранный период)"
            }
        },
        filters: {
            apiKeyAll: "Все ключи API",
            apiKeyAriaLabel: "Фильтр API-ключа",
            apiKeyHint: "Подсказка: фильтр API-ключа помогает быстро найти точки нагрузки.",
            range: {
                last24Hours: "Последние 24 часа",
                last30Days: "Последние 30 дней",
                last7Days: "Последние 7 дней"
            },
            rangeAriaLabel: "Диапазон времени"
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
            badge: "Обзор рабочего пространства арендатора",
            summaryAllApiKeys: "(все ключи API)",
            summaryPrefix: "Объем: текущий арендатор ",
            summarySingleApiKey: "(один ключ API)"
        },
        subtitle: {
            allApiKeys: "(все ключи API)",
            metricsFocus: "Ключевые метрики: TPM, RPM, общий расход Token, число запросов и скорость первого токена.",
            scopePrefix: "Объем: текущий арендатор",
            singleApiKey: "(один ключ API)",
            timeWindow: ", временное окно:"
        },
        modelDistribution: {
            description: "Топ моделей по числу запросов или расходу Token.",
            empty: "Пока нет данных о распределении моделей",
            modeRequests: "По запросам",
            modeTokens: "По Token",
            other: "Другое",
            title: "Распределение запросов по моделям"
        },
        tokenComponents: {
            cached: "Кэшированный ввод",
            input: "Ввод",
            output: "Вывод",
            reasoning: "Reasoning"
        },
        tokenSummary: {
            title: "Сводка по компонентам Token"
        },
        tokenTrend: {
            description: "Почасовой тренд Token по компонентам. Переключайте компоненты для фокуса.",
            empty: "Пока нет данных по тренду Token",
            title: "Тренд использования Token"
        },
        topKeys: {
            description: "Основано на объеме запросов за выбранный период",
            empty: "Пока нет рейтинга использования API-ключей",
            requests: "{{count}} запросов",
            share: "Доля {{percent}}%",
            title: "Топ API-ключей",
            unknownKey: "Ключ без имени"
        },
        title: "Панель управления арендатора",
        trend: {
            description: "Область применения: объем запросов ключей API клиента (с почасовой детализацией).",
            empty: "Данных по запросу пока нет",
            series: {
                requests: "Запросы"
            },
            title: "Тенденция запроса"
        }
    },
    tenantLogs: {
        audit: {
            actionValues: {
                adminOperation: "Действие администратора",
                authOperation: "Действие аутентификации",
                requestOperation: "Операция запроса",
                tenantOperation: "Действие арендатора",
                unknown: "Неизвестное действие"
            },
            actorTypes: {
                adminUser: "Пользователь-администратор",
                apiKey: "API-ключ",
                system: "Система",
                tenantUser: "Пользователь арендатора",
                unknown: "Неизвестный исполнитель"
            },
            columns: {
                action: "Действие",
                actor: "Актер",
                reason: "Подробности",
                result: "Результат",
                target: "Цель",
                time: "Время"
            },
            description: "Определение: события аудита плоскости управления (только текущий клиент)",
            empty: "Нет данных журнала аудита",
            filters: {
                actionPlaceholder: "Действие",
                actorIdPlaceholder: "Идентификатор актера",
                actorTypePlaceholder: "Тип актера",
                keywordPlaceholder: "Ключевое слово (причина/полезная нагрузка)",
                resultStatusPlaceholder: "Статус результата",
                actionAriaLabel: "Фильтр по действию",
                actorIdAriaLabel: "Фильтр ID актера",
                actorTypeAriaLabel: "Фильтр типа актера",
                keywordAriaLabel: "Фильтр по ключевому слову",
                rangeAriaLabel: "Диапазон времени",
                resultStatusAriaLabel: "Фильтр статуса результата"
            },
            resultStatuses: {
                denied: "Отклонено",
                failed: "Ошибка",
                ok: "Успешно",
                unknown: "Неизвестный результат"
            },
            title: "Журналы аудита"
        },
        filters: {
            range: {
                last24Hours: "Последние 24 часа",
                last30Days: "Последние 30 дней",
                last7Days: "Последние 7 дней"
            }
        },
        request: {
            columns: {
                apiKey: "API-ключ",
                error: "Ошибка",
                latency: "Задержка (мс)",
                path: "Путь",
                requestId: "Идентификатор запроса",
                status: "Статус",
                time: "Время"
            },
            description: "Определение: события необработанного запроса Data Plane (только текущий клиент)",
            empty: "Нет данных журнала",
            filters: {
                apiKeyIdPlaceholder: "Идентификатор ключа API",
                keywordPlaceholder: "Ключевое слово (путь/ошибка/модель)",
                requestIdPlaceholder: "Идентификатор запроса",
                statusCodePlaceholder: "Код состояния (например, 429)",
                apiKeyAriaLabel: "Фильтр API-ключа",
                keywordAriaLabel: "Фильтр по ключевому слову",
                rangeAriaLabel: "Диапазон времени",
                requestIdAriaLabel: "Фильтр ID запроса",
                statusCodeAriaLabel: "Фильтр кода статуса"
            },
            title: "Запрос журналов"
        },
        scope: "Область применения: только текущий арендатор",
        tabs: {
            audit: "Журналы аудита",
            request: "Запрос журналов"
        },
        title: "Журналы"
    },
    tenantUsage: {
        columns: {
            apiKey: "API-ключ",
            requests: "Запросы",
            tenantLabel: "Арендатор: {{tenantId}}",
            time: "Время"
        },
        filters: {
            apiKeyAll: "Ключ API Все",
            apiKeyAriaLabel: "Фильтр API-ключа",
            range: {
                last24Hours: "Последние 24 часа",
                last30Days: "Последние30 дней",
                last7Days: "Последние7 дней"
            },
            rangeAriaLabel: "Диапазон времени"
        },
        hourly: {
            description: "Описание",
            empty: "Пустой",
            title: "Заголовок"
        },
        leaderboard: {
            description: "Описание",
            empty: "Пустой",
            title: "Заголовок"
        },
        subtitle: "Субтитры",
        title: "Заголовок",
        trend: {
            description: "Описание",
            empty: "Пустой",
            title: "Заголовок"
        }
    },
    tenants: {
        create: {
            fields: {
                expiresAt: "Срок действия истекает в",
                name: "Имя арендатора",
                plan: "План (кредит)",
                status: "Статус (активный/неактивный)"
            },
            submit: "Создать арендатора",
            title: "Создать арендатора"
        },
        impersonation: {
            copyToken: "Копировать токен",
            create: "Создать олицетворение",
            fields: {
                reason: "Причина (обязательно)"
            },
            revoke: "Отозвать сеанс",
            sessionIdLabel: "ID сессии:",
            tokenLabel: "Токен:",
            title: "Олицетворение администратора"
        },
        keys: {
            create: {
                fields: {
                    name: "Имя ключа",
                    namePlaceholder: "например основной ключ администратора"
                },
                submit: "Создать ключ",
                title: "Создать ключ API"
            },
            created: {
                copyPlaintext: "Копировать ключ открытого текста",
                notice: "Ключ открытого текста отображается только один раз. Сохраните это сейчас."
            },
            list: {
                caption: "Список ключей API арендатора",
                columns: {
                    actions: "Действия",
                    createdAt: "Создано в",
                    name: "Имя",
                    prefix: "Префикс",
                    status: "Статус"
                },
                copyPrefix: "Скопировать префикс ключа",
                disable: "Запрещать",
                empty: "Для этого клиента нет ключей API.",
                enable: "Давать возможность",
                status: {
                    active: "Активен",
                    revoked: "Отозван"
                },
                title: "Список ключей API"
            }
        },
        list: {
            caption: "Список арендаторов",
            columns: {
                actions: "Действия",
                apiKeys: "API-ключи",
                expiresAt: "Срок действия истекает в",
                plan: "План",
                status: "Статус",
                tenant: "Жилец",
                tenantId: "Идентификатор арендатора",
                updatedAt: "Обновлено в"
            },
            planValues: {
                credit: "Кредитный план",
                unknown: "Пользовательский ({{value}})"
            },
            statusValues: {
                active: "Активен",
                inactive: "Неактивен",
                unknown: "Неизвестно ({{value}})"
            },
            defaultBadge: "По умолчанию",
            empty: "Нет данных об арендаторе",
            openProfile: "Открыть профиль арендатора",
            searchPlaceholder: "Поиск арендатора по имени или ID",
            title: "Пул арендаторов"
        },
        messages: {
            apiKeyCreateFailed: "Не удалось создать ключ API.",
            apiKeyCreateSuccess: "Создан ключ API для клиента {{tenantName}}: {{keyName}}.",
            apiKeyNameRequired: "Введите имя ключа API",
            apiKeyToggleFailed: "Не удалось обновить статус ключа API.",
            createFailed: "Не удалось создать арендатора",
            createSuccess: "Арендатор создан: {{name}} ({{id}})",
            impersonationCreateFailed: "Не удалось создать олицетворение.",
            impersonationCreated: "Сеанс олицетворения создан (токен возвращен)",
            impersonationRevokeFailed: "Не удалось отменить выдачу себя за другое лицо.",
            impersonationRevoked: "Сеанс выдачи себя за другое лицо отменен",
            rechargeFailed: "Не удалось пополнить счет арендатора.",
            rechargeSuccess: "Пополнение выполнено: +{{amount}}, текущий баланс {{balance}}.",
            updateFailed: "Не удалось обновить арендатора.",
            updateSuccess: "Арендатор обновлен: {{name}}"
        },
        profile: {
            dialogDescription: "Управляйте профилем, ключами API и их использованием в одном диалоговом окне с вкладками.",
            dialogTitle: "Профиль арендатора",
            dialogTitleWithName: "Профиль арендатора · {{name}}",
            fields: {
                expiresAt: "Срок действия истекает в",
                plan: "План",
                status: "Статус"
            },
            meta: {
                createdAt: "Создано в",
                tenantId: "Идентификатор арендатора",
                updatedAt: "Обновлено в"
            },
            save: "Сохранить профиль",
            section: {
                title: "Профиль арендатора"
            },
            tabs: {
                ariaLabel: "Вкладки профиля арендатора",
                keys: "API-ключи",
                profile: "Профиль",
                usage: "Использование"
            }
        },
        recharge: {
            fields: {
                amount: "Микрокредиты (целое число)",
                reason: "Причина"
            },
            submit: "Применить пополнение",
            title: "Пополнение счета арендатора"
        },
        subtitle: "Проверяйте доступность арендатора и управляйте профилями, ключами API и использованием.",
        title: "Арендаторы",
        usage: {
            filter: {
                allKeys: "Все ключи API",
                currentView: "Текущий вид",
                label: "Фильтр ключей API",
                noKeys: "Нет ключей API для текущего арендатора",
                noMatches: "Нет подходящих ключей API.",
                placeholder: "Имя поиска/префикс/key_id"
            },
            meta: {
                tenantId: "Идентификатор арендатора"
            },
            metrics: {
                accountRequests: "Запросы на аккаунт",
                activeAccounts: "Активные аккаунты",
                activeApiKeys: "Активные ключи API",
                apiKeyRequests: "Запросы ключей API",
                tenantApiKeyRequests: "Запросы ключей API арендатора"
            },
            sectionTitle: "Использование за последние 24 часа",
            status: {
                error: "Не удалось загрузить данные об использовании.",
                loading: "Загрузка данных об использовании…"
            }
        }
    },
    theme: {
        aurora: "Аврора",
        colorful: "Яркая",
        dark: "Темная",
        light: "Светлая"
    },
    usage: {
        actions: {
            export: "Экспорт CSV",
            filters: "Фильтры"
        },
        chart: {
            empty: "Нет данных за этот период.",
            requests: "Запросы",
            subtitle: "Сводная статистика по всем провайдерам",
            title: "Потребление токенов (30 дней)"
        },
        subtitle: "Потребление запросов и профилирование инфраструктуры.",
        title: "Анализ статистики",
        topKeys: {
            columns: {
                apiKey: "ID ключа",
                name: "Арендатор / Ключ",
                requests: "Запросы",
                share: "Доля",
                tenant: "Арендатор"
            },
            empty: "Нет данных об использовании.",
            keyFallback: "API-ключ {{keyId}}",
            searchPlaceholder: "Поиск API-ключа или арендатора…",
            subtitle: "По объему запросов",
            title: "Топ ключей API",
            reqs: "запросов"
        }
    },
    cleanup: {
        title: "Очистка учетных данных",
        subtitle: "Автоматизированные политики управления и жизненного цикла",
        save: "Сохранить политики",
        policy: {
            title: "Политика автоматического управления",
            desc: "При повторном использовании, отзыве или длительной невалидности refresh_token аккаунты автоматически изолируются для снижения риска.",
            refreshEnabled: "Включить автообновление OAuth",
            refreshEnabledDesc: "Если выключить, access_token больше не будет обновляться автоматически.",
            intervalSec: "Интервал обновления (сек)",
            notes: "Примечания к политике"
        },
        workspace: {
            title: "Рабочая зона управления OAuth-аккаунтами",
            desc: "Проверка статуса обновления входа и быстрые действия: обновить вход, приостановить или возобновить связанные аккаунты.",
            searchPlaceholder: "Поиск по label / account id",
            onlyDisabled: "Только отключенные аккаунты",
            loadingAccounts: "Загрузка аккаунтов…",
            noAccounts: "Нет подходящих OAuth-аккаунтов.",
            enabled: "Включен",
            disabled: "Отключен",
            selectHint: "Выберите аккаунт слева, чтобы увидеть статус.",
            loadingStatus: "Загрузка статуса OAuth…",
            noStatus: "Для этого аккаунта пока нет статуса OAuth.",
            refreshNow: "Обновить сейчас",
            disableFamily: "Приостановить связанные аккаунты",
            enableFamily: "Возобновить связанные аккаунты",
            status: {
                never: "Не обновлялся",
                ok: "В норме",
                failed: "Ошибка"
            },
            fields: {
                refreshStatus: "Статус обновления",
                reuseDetected: "Обнаружено повторное использование refresh_token",
                groupId: "ID группы",
                tokenVersion: "Версия токена",
                expiresAt: "Срок действия",
                errorCode: "Код ошибки",
                errorMessage: "Подробности ошибки"
            }
        },
        quarantine: {
            title: "Политика авто-карантина",
            desc: "Автоматическая изоляция аккаунтов, не прошедших авторизацию",
            threshold: "Порог ошибок",
            thresholdDesc: "Последовательные ошибки 401/403 перед изоляцией",
            action: "Действие при отзыве",
            actionDesc: "При отзыве общего refresh_token",
            options: {
                family: "Изолировать семейство",
                disable: "Только отключить аккаунт",
                nothing: "Ничего не делать"
            }
        },
        purge: {
            title: "Политика авто-очистки",
            desc: "Безвозвратное удаление неактивных учетных данных для экономии места",
            retention: "Период хранения",
            retentionDesc: "Дней хранения отключенных аккаунтов перед удалением"
        }
    },
    apiKeys: {
        title: "Ключи API",
        subtitle: "Выдача и управление безопасными учетными данными для клиентских приложений.",
        create: "Создать секретный ключ",
        search: "Поиск названия ключа или префикса…",
        loading: "Загрузка учетных данных…",
        empty: "Не найдено действующих ключей API, соответствующих вашим критериям.",
        columns: {
            name: "Название приложения",
            tenant: "ID арендатора",
            key: "Ключ API",
            status: "Статус",
            issued: "Выпущен в",
            actions: "Действия"
        },
        status: {
            active: "Активен",
            revoked: "Отозван"
        },
        defaultTenant: "По умолчанию",
        filters: {
            label: "Фильтр статуса",
            all: "Все ключи",
            active: "Активные",
            revoked: "Отозванные"
        },
        actions: {
            copyPrefixTitle: "Копировать префикс",
            menu: "Действия с ключом",
            copyPrefix: "Копировать префикс ключа",
            processing: "Обработка…",
            disable: "Отключить ключ",
            enable: "Включить ключ"
        },
        messages: {
            createFailed: "Не удалось создать API-ключ",
            missingName: "Введите имя ключа"
        },
        dialog: {
            create: {
                title: "Создать API-ключ",
                desc: "Создает ключ доступа Data Plane для арендатора. Ключ в открытом виде показывается только один раз.",
                nameLabel: "Имя ключа",
                namePlaceholder: "например: prod-codex-clients",
                tenantLabel: "Имя арендатора (необязательно)",
                tenantPlaceholder: "Пусто = использовать default",
                confirm: "Создать",
                creating: "Создание…"
            },
            created: {
                title: "Новый ключ создан",
                desc: "Открытый ключ показывается только один раз. Скопируйте и сохраните его сейчас.",
                securityTip: "Внимание: после закрытия этого окна ключ в открытом виде больше не будет доступен.",
                nameLabel: "Имя ключа",
                plaintextLabel: "Ключ в открытом виде",
                close: "Закрыть",
                copyPlaintext: "Копировать ключ"
            }
        }
    }
}
