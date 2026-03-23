# SQLite 冷库热池号池设计

## 背景

当前 `personal/sqlite` 路径虽然导入任务已经调用 `queue_oauth_refresh_token()`，但默认实现仍会回落成 `upsert + refresh_token`。这导致：

- 大批量导入时会立刻触发上游 refresh
- SQLite 运行态没有真正的 `cold vault`
- `activate_oauth_refresh_token_vault()` 在 in-memory 默认实现里是空操作
- data-plane snapshot 会看到全部已导入账号，而不是仅看到热池

这条路径不适合承载上万级账号的冷存、慢激活和隔离恢复。

## 目标

为 SQLite 路径补一套可用的 `cold / active / quarantine` 号池能力，并尽量贴近 Postgres 已有的 `vault + active pool` 语义：

1. 导入 refresh token 时默认只入冷库，不立即 refresh。
2. 运行时按预算从冷库激活少量账号进入热池。
3. `quarantine` 账号不会出现在 data-plane snapshot 中。
4. 隔离到期后账号可以自动回到 `active`。
5. fatal 错误仍沿用现有 `enabled=false` / family disable 逻辑，不与 quarantine 混淆。

## 非目标

- 本轮不把 SQLite 做成与 Postgres 完全同构的数据库表模型。
- 本轮不大改前端页面，只保持现有接口可用并补最小状态字段。
- 本轮不引入 Redis alive ring；SQLite/personal 继续走单机 snapshot + 本地 unhealthy TTL。

## 方案

### 1. 冷库存储

在 SQLite 持久化状态中新增 `oauth_refresh_token_vault` 集合，记录：

- `refresh_token_enc`
- `fallback_access_token_enc`
- `fallback_token_expires_at`
- `refresh_token_sha256`
- `label`
- `base_url`
- `chatgpt_account_id`
- `chatgpt_plan_type`
- `source_type`
- `desired_mode`
- `desired_enabled`
- `desired_priority`
- `status`
- `failure_count`
- `backoff_until`
- `next_attempt_at`
- `last_error_code`
- `last_error_message`
- `created_at`
- `updated_at`

SQLite 的 `queue_oauth_refresh_token()` 改为真正写入这份冷库，而不是立即 upsert 账号。

### 2. 热池与隔离状态

在 SQLite 运行态新增 `account_pool_states`，只作用于“已经物化为账号”的记录。

最小字段：

- `pool_state`：`active | quarantine`
- `quarantine_until`
- `quarantine_reason`
- `last_pool_transition_at`

约定：

- `cold` 账号不出现在 `accounts` 中，只存在于 vault。
- `active` 账号会进入 snapshot。
- `quarantine` 账号仍保留在 `accounts/oauth_credentials/session_profiles` 中，但不会进入 snapshot。

### 3. 激活循环

SQLite 实现 `activate_oauth_refresh_token_vault()`：

1. 统计当前 `active` 账号数。
2. 若低于目标值，从 vault 中选出一批 `queued` 且已过 backoff 的记录。
3. 对这批记录执行 refresh，成功后物化为账号并标记 `active`。
4. 失败时写回 `failure_count/backoff/last_error_*`。
5. fatal 错误把 vault 记录改成 `failed`，避免无限重试。

默认仍沿用现有全局配置：

- `CONTROL_PLANE_VAULT_ACTIVATE_ENABLED`
- `CONTROL_PLANE_VAULT_ACTIVATE_INTERVAL_SEC`
- `CONTROL_PLANE_ACTIVE_POOL_TARGET`
- `CONTROL_PLANE_VAULT_ACTIVATE_BATCH_SIZE`
- `CONTROL_PLANE_VAULT_ACTIVATE_CONCURRENCY`
- `CONTROL_PLANE_VAULT_ACTIVATE_MAX_RPS`

SQLite 只是在当前默认空实现上补齐真实行为。

### 4. quarantine 规则

`quarantine` 只表达“暂时不参与路由”，不表达永久死亡。

建议规则：

- `rate_limited` / `quota_exhausted`：写 `quarantine_until`，等到期后恢复 `active`
- `auth_expired` / `token_invalidated`：优先走现有 refresh 恢复；若仍不可用，则进入短期 `quarantine`
- `account_deactivated` / `refresh_token_revoked` / `refresh_token_reused`：直接禁用账号或 family disable，不走 `quarantine`
- 手工禁用：仍走 `enabled=false`

### 5. snapshot 过滤

`snapshot_inner()` 不扩 data-plane 协议，继续产出同样的 `DataPlaneSnapshot`，但只包含：

- `enabled=true`
- `pool_state=active`
- 不处于 `quarantine_until > now`

这样 data-plane 仍然只需要消费普通 `accounts + account_traits`，不需要理解新的池状态。

### 6. 状态观测

在现有 `OAuthAccountStatusResponse` 上补最小字段：

- `pool_state`
- `quarantine_until`
- `quarantine_reason`

这样管理端和调试接口可以看见“账号为什么当前不可路由”，但不需要本轮重做 UI。

## 风险与取舍

- SQLite 是单快照 JSON 持久化，不是明细表；大冷库会抬高单次持久化成本。
- 但当前目标是先让 `personal` 具备正确的语义，而不是把 SQLite 做成最终的 10 万级生产形态。
- 因此本轮优先保证：
  - 导入不打上游
  - 热池规模受控
  - 隔离恢复闭环正确

## 验证

至少覆盖以下场景：

- refresh token 导入后进入 vault，不立刻创建账号
- 激活循环可以把 vault 账号物化到 `active`
- 激活失败会写回 backoff
- `quarantine` 账号不会进入 snapshot
- `quarantine_until` 到期后会重新进入 `active`
- fatal refresh 错误会阻断继续激活

验证命令以 `cargo test -p control-plane` 下的定向测试为主，完成后再做本地 `personal` 运行态验证。
