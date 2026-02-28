# 10 万账号健康管理改造蓝图（OAuth Vault + Active Pool，单实例）

**Goal:** 在单实例（非分布式）部署下，支持 10 万级 Codex/ChatGPT OAuth 账号存量管理；对外稳定维持 `Active=5000` 可路由账号；尽量降低封号/风控概率（避免后台刷新/探测风暴）。

**Core Idea:** 把“账号存量（10 万）”与“对外路由池（Active 5000）”彻底解耦：

- 大部分账号仅冷存于 `oauth_refresh_token_vault`（导入不打上游）
- 只有少量账号被“物化”为 `upstream_accounts + upstream_account_oauth_credentials`，并按预算维持活跃
- 账号健康信号以 **业务请求被动观测** 为主（data-plane 的错误分类 + ejection），主动探测（rate-limit fetch）仅用于少量热账号或 UI 按需

---

## 0) 现状观测（用于约束设计）

在当前 Postgres（开发库）中观察到：

- `upstream_accounts` 总数 `545`，`auth_provider` 全为 `oauth_refresh_token`
- `upstream_account_oauth_credentials.last_refresh_status` 全为 `ok`
- access token TTL 约 `10 天`（`token_expires_at - last_refresh_at ≈ 864000s`）
- `next_refresh_at` 仍有大量 `NULL`（需要回填/纠偏策略，否则规模放大会“同刻集中刷新”）
- `upstream_account_rate_limit_snapshots` 存在较多 `invalid_refresh_token` 错误码；若用于阻断路由，会显著缩小可用池并加剧集中打击风险

---

## 1) 目标行为与不做的事（Non-Goals）

### 目标行为

- 任意时刻对外可路由的账号规模稳定在 `4500~5500`（目标 5000）
- 后台对上游请求速率（refresh / activate / probe）由全局预算严格约束，与存量规模无关
- 账号失效/限流/额度不足会自动隔离，隔离期结束后再以低频重试恢复

### 不做的事

- 不尝试让 10 万账号“都保持 token 永远新鲜”
- 不对 10 万账号做全量的 rate-limit 探测与周期性查询套餐信息

---

## 2) 数据模型改造

### 2.1 新增：`oauth_refresh_token_vault`

用途：冷存 refresh token 及导入元信息，支持“导入不打上游”，并为后续预算激活提供队列。

建议字段（可按最小可用裁剪）：

- `id UUID PRIMARY KEY`
- `refresh_token_enc TEXT NOT NULL`
- `refresh_token_sha256 TEXT NOT NULL DEFAULT ''`
- `base_url TEXT NOT NULL`
- `label TEXT NOT NULL`
- `email TEXT NULL`
- `chatgpt_account_id TEXT NULL`
- `chatgpt_plan_type TEXT NULL`
- `source_type TEXT NULL`
- `desired_mode TEXT NOT NULL`（如 `codex_oauth` / `chat_gpt_session`，与现有 `UpstreamMode` 对齐）
- `desired_enabled BOOLEAN NOT NULL DEFAULT true`
- `desired_priority INT NOT NULL DEFAULT 100`
- `status TEXT NOT NULL DEFAULT 'queued'`
- `failure_count INT NOT NULL DEFAULT 0`
- `backoff_until TIMESTAMPTZ NULL`
- `next_attempt_at TIMESTAMPTZ NULL`
- `last_error_code TEXT NULL`
- `last_error_message TEXT NULL`
- `created_at TIMESTAMPTZ NOT NULL`
- `updated_at TIMESTAMPTZ NOT NULL`

索引建议：

- `UNIQUE (refresh_token_sha256)`（导入去重的最小成本方案）
- `INDEX (chatgpt_account_id)`（当导入数据带 account_id 时可加速合并）
- `INDEX (status, next_attempt_at)`（激活队列）

> refresh token 旋转：当 vault 记录被“激活物化”后，建议 **删除 vault 记录**（因为最新 refresh token 会被写入 `upstream_account_oauth_credentials.refresh_token_enc`；后续不再依赖 vault）。

### 2.2 新增：`upstream_accounts.pool_state`

用途：让 control-plane 快照与后台任务只关注“可路由池”，避免 10 万账号参与解密、序列化、下发与刷新扫描。

- `pool_state TEXT NOT NULL DEFAULT 'active'`
- 取值：`active | standby | cold`（最小版本可只用 `active | standby`，cold 只存在于 vault）
- 索引：`INDEX (pool_state, created_at)` 或 `INDEX (pool_state, id)`

快照约束：

- `snapshot_inner` 只下发 `pool_state='active'`
- outbox event 查询单条账号时：若 `pool_state!='active'`，返回 `None`，让 data-plane 将其视为删除

---

## 3) 后台调度（单实例）

### 3.1 Vault 激活器（核心）

目标：维持 Active=5000，低水位触发补位；所有激活均受 **全局 RPS 预算** 控制。

流程：

1. 周期性（例如每 30s）统计 `pool_state='active'` 的“有效可用数”（SQL 过滤 + 轻量逻辑即可）
2. 若低于 `active_min`，从 vault 里按 `status='queued' AND (next_attempt_at<=now OR NULL) AND (backoff_until<=now OR NULL)` 抓取一批
3. 对每条执行一次 `oauth_client.refresh_token(refresh_token)` 以获取 access token +（可能旋转的）refresh token
4. 将账号 upsert 到 `upstream_accounts + upstream_account_oauth_credentials`，并将 `pool_state` 设为 `active`
5. 成功后删除 vault 记录；失败则更新 `failure_count/backoff_until/next_attempt_at/last_error_*`

预算建议（Active=5000）：

- `activate_max_rps=1`（满池冷启动约 5000 / 1rps ≈ 1.4h；可调到 2-3，但建议先保守）
- `activate_concurrency=8`（并发不等于 RPS；必须有全局 token-bucket/节拍器）

### 3.2 OAuth 刷新器（只刷新 Active）

调整点：

- 扫描条件加入 `pool_state='active'`，避免 10 万账号参与刷新循环
- `next_refresh_at` 调度不要使用 0-120 秒级抖动；建议改为 “提前 6-24 小时 + 小时级抖动”，避免同批激活在同一窗口到期集中刷新
- 刷新 tick 间隔建议从 5s 调整到 60s-300s

### 3.3 Rate-limit cache（默认关闭）

原则：

- `CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_ENABLED=false` 作为默认
- 若 UI 需要展示套餐/窗口：改成“按需拉取 + 缓存”，或仅对 `tier=hot` 的少量账号低频刷新
- 健康阻断不要依赖全量主动探测，优先使用 data-plane 的请求级错误分类 + ejection

---

## 4) data-plane 健康信号（被动优先）

data-plane 已具备：

- 请求失败时的 ejection TTL（按错误分类决定隔离时长）
- 触发 internal oauth refresh / disable 的恢复动作

增强建议：

- 增加一个 internal 上报接口（data-plane -> control-plane），把 `account_id + error_class + retry_after` 作为“隔离窗口”写入 control-plane（可复用 `upstream_account_rate_limit_snapshots.{last_error_code,expires_at}` 做 quarantine）
- 这样 control-plane 不需要全量探测，也能让快照的 `oauth_effective_enabled` 稳定阻断明显不可用账号

---

## 5) API 与管理台改造（10 万可用性）

必做：

- `list_upstream_accounts` 必须分页（keyset/cursor），不能一次返回全量
- 新增 vault 管理 API（分页 + 统计 + 手动激活/丢弃/重置 backoff）
- 导入任务（import job）语义调整为“写入 vault”，不再等同于“创建 upstream account”

建议调整 import job item 结构：

- 增加 `vault_id`（成功时返回）
- 保留 `account_id`（仅当激活阶段成功物化时才有意义）

---

## 6) 配置建议（Active=5000，单实例）

建议新增（示例命名）：

- `CONTROL_PLANE_ACTIVE_POOL_TARGET=5000`
- `CONTROL_PLANE_ACTIVE_POOL_MIN=4500`
- `CONTROL_PLANE_ACTIVE_POOL_MAX=5500`
- `CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS=1`
- `CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY=8`
- `CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE=200`
- `CONTROL_PLANE_VAULT_ACTIVATE_INTERVAL_SEC=30`

建议调整现有：

- `CONTROL_PLANE_OAUTH_REFRESH_INTERVAL_SEC=60`（或更大）
- `CONTROL_PLANE_OAUTH_REFRESH_MAX_RPS=1~2`
- `CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_ENABLED=false`

---

## 7) 分阶段落地路线（推荐）

### Phase 0：先保命（只改配置）

- 关闭全量 rate-limit cache 刷新
- 降低 OAuth 刷新 tick 频率与 RPS（并确保线上进程实际生效）

### Phase 1：引入 Vault（导入不打上游）

- 新增 `oauth_refresh_token_vault`
- 改造 import job：只写 vault + 去重 + backoff，不做 refresh_token 与 rate-limit 预填
- 提供 vault 列表/统计接口

### Phase 2：Active Pool（只下发/只刷新 Active）

- 新增 `upstream_accounts.pool_state` + 快照过滤
- OAuth 刷新循环只扫描 `pool_state='active'`
- 新增 Vault 激活器：按预算慢慢填满 Active=5000

### Phase 3：被动健康闭环

- data-plane 上报健康事件 -> control-plane 持久化 quarantine
- 将“主动探测”彻底降级为 UI/热池按需

---

## 8) 风险与对策

- **刷新风暴/封号风险**：所有后台上游调用必须走统一预算（token bucket），并避免全量扫描。
- **refresh token 旋转导致 vault 失效**：激活后删除 vault 记录，后续以 oauth_credentials 为准。
- **导入重复/脏数据**：以 `refresh_token_sha256` 去重，失败写入 backoff；必要时支持手动重置与丢弃。
- **10 万列表/管理不可用**：所有管理接口必须分页；前端虚拟滚动。

---

## 9) 本轮实施 Todo（已回填）

- [x] 新增 `oauth_refresh_token_vault` 表与索引（含去重 hash、队列扫描索引）
- [x] 新增 `upstream_accounts.pool_state` 字段与索引
- [x] 导入任务改为“写入 vault”（`queue_oauth_refresh_token`），导入阶段不再打上游
- [x] 导入成功结果支持 `account_id` 为空（仅入 vault 场景）
- [x] 新增 vault 激活器循环：按 `batch/concurrency/max_rps` 预算激活
- [x] 激活成功后删除 vault 记录；失败写回 `failure_count/backoff/error`
- [x] 快照仅下发 `pool_state='active'`
- [x] Data Plane outbox 单账号加载仅返回 `pool_state='active'`（否则按删除语义处理）
- [x] OAuth 刷新扫描仅处理 `pool_state='active'`
- [x] rate-limit 刷新目标扫描仅处理 `pool_state='active'`
- [x] 激活阈值逻辑修正为低于 `target` 即补位（并保留 `active_min` 告警）
- [x] 主进程接入 vault 激活 loop（`CONTROL_PLANE_VAULT_ACTIVATE_*`）
- [x] `.env.runtime` 落地 10 万冷池/5000 活跃池保守预算参数
- [x] 编译验证：`cargo check -p control-plane` / `cargo check -p data-plane`
