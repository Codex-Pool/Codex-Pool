# SQLite 冷库热池实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 `personal/sqlite` 路径补齐可用的 `cold / active / quarantine` 号池能力，让导入走冷库、热池按预算激活、隔离账号不进入 snapshot。

**Architecture:** 在 `InMemoryStore + SqliteBackedStore` 上新增 vault 与 pool state overlay。导入任务继续复用现有 API，但 SQLite 的 `queue_oauth_refresh_token()` 改成真正入 vault；snapshot 只下发 `active` 账号，quarantine 由 store 在 control-plane 侧过滤。

**Tech Stack:** Rust, axum, tokio, sqlx sqlite, serde, existing control-plane import job + snapshot pipeline

---

## Todo

- [x] 扩展 SQLite 持久化状态，新增 vault 与 account pool state
- [x] 为 SQLite queue/activate 行为补 failing tests
- [x] 实现 SQLite vault 入库与预算激活
- [x] 让 snapshot 只下发 active 且非 quarantine 账号
- [x] 扩展 OAuth 状态响应以暴露 pool/quarantine 信息
- [x] 跑 control-plane 定向测试与本地 personal 验证
- [x] 回填本计划执行结果

## Task 1: 扩展数据结构

**Files:**
- Modify: `services/control-plane/src/store/defs.rs`
- Modify: `services/control-plane/src/store/in_memory_core.rs`
- Modify: `services/control-plane/src/store/sqlite_backed.rs`

**Step 1: 写 failing test**

在 `services/control-plane/src/store/sqlite_backed.rs` 的 `sqlite_backed_store_tests` 新增一个最小测试，断言：

- 导入一条 refresh token 后，不会立即出现在 `list_upstream_accounts()`
- 新增的持久化状态中存在 vault 记录

**Step 2: 运行测试确认失败**

Run:

```bash
cargo test -p control-plane sqlite_backed_store_queue_oauth_refresh_token_keeps_account_in_vault -- --nocapture
```

**Step 3: 写最小实现**

- 在 `defs.rs` 新增：
  - `SqliteVaultOAuthRecord`
  - `AccountPoolStateRecord`
  - `AccountPoolState`
- 把它们接入 `InMemoryStore` 与 `SqlitePersistedStoreState`

**Step 4: 重新运行测试**

Run:

```bash
cargo test -p control-plane sqlite_backed_store_queue_oauth_refresh_token_keeps_account_in_vault -- --nocapture
```

## Task 2: 让 SQLite 的 queue 真正入 vault

**Files:**
- Modify: `services/control-plane/src/store/defs.rs`
- Modify: `services/control-plane/src/store/trait_impl.rs`
- Modify: `services/control-plane/src/store/sqlite_backed.rs`

**Step 1: 写 failing test**

补一个测试，断言 `queue_oauth_refresh_token()` 返回 `created=true` 时不会调用直接 upsert 后的账号可见路径。

**Step 2: 运行测试确认失败**

Run:

```bash
cargo test -p control-plane sqlite_backed_store_queue_oauth_refresh_token_keeps_account_in_vault -- --nocapture
```

**Step 3: 写最小实现**

- 在 `InMemoryStore` 实现真正的 `queue_oauth_refresh_token`
- `SqliteBackedStore` 改为持久化 queue 结果

**Step 4: 重新运行测试**

同上。

## Task 3: 实现 SQLite vault 激活器

**Files:**
- Modify: `services/control-plane/src/store/oauth_ops.rs`
- Modify: `services/control-plane/src/store/trait_impl.rs`
- Modify: `services/control-plane/src/store/sqlite_backed.rs`

**Step 1: 写 failing tests**

新增测试覆盖：

- `activate_oauth_refresh_token_vault()` 可以把 vault 记录物化成 active 账号
- 激活失败会写回 `failure_count/backoff`

**Step 2: 运行测试确认失败**

Run:

```bash
cargo test -p control-plane sqlite_backed_store_activates_vault_accounts -- --nocapture
cargo test -p control-plane sqlite_backed_store_backoffs_failed_vault_activation -- --nocapture
```

**Step 3: 写最小实现**

- 从 vault 中选取 `queued` 且已过 backoff 的记录
- 按当前配置执行 refresh
- 成功后 upsert 为账号并打 `active`
- 失败后写回错误与 backoff

**Step 4: 重新运行测试**

同上。

## Task 4: 接入 quarantine 与 snapshot 过滤

**Files:**
- Modify: `services/control-plane/src/store/family_snapshot.rs`
- Modify: `services/control-plane/src/store/in_memory_core.rs`
- Modify: `services/control-plane/src/store/oauth_ops.rs`

**Step 1: 写 failing tests**

新增测试覆盖：

- `quarantine` 账号不会进入 snapshot
- `quarantine_until` 到期后账号会恢复进入 snapshot

**Step 2: 运行测试确认失败**

Run:

```bash
cargo test -p control-plane sqlite_backed_store_snapshot_excludes_quarantined_accounts -- --nocapture
cargo test -p control-plane sqlite_backed_store_snapshot_recovers_expired_quarantine -- --nocapture
```

**Step 3: 写最小实现**

- 增加 pool state 过滤函数
- snapshot 只发 `active`
- 到期 quarantine 在读取或 snapshot 前自动恢复

**Step 4: 重新运行测试**

同上。

## Task 5: 扩展状态接口

**Files:**
- Modify: `services/control-plane/src/contracts.rs`
- Modify: `services/control-plane/src/store/in_memory_core.rs`
- Modify: `services/control-plane/src/store/postgres/impl_oauth_snapshot/rate_limit_jobs.rs`

**Step 1: 写 failing test**

新增状态测试，断言 `oauth_account_status()` 返回 `pool_state/quarantine_until/quarantine_reason`。

**Step 2: 运行测试确认失败**

Run:

```bash
cargo test -p control-plane oauth_status_includes_pool_state -- --nocapture
```

**Step 3: 写最小实现**

- 在状态响应上补字段
- SQLite 与 Postgres 构造函数都填充兼容值

**Step 4: 重新运行测试**

同上。

## Task 6: 回归验证

**Files:**
- Modify: `services/control-plane/src/store/sqlite_backed.rs`
- Modify: `docs/plans/2026-03-22-sqlite-oauth-active-pool.md`

**Step 1: 跑定向测试**

Run:

```bash
cargo test -p control-plane sqlite_backed_store_ -- --nocapture
cargo test -p control-plane oauth_status_includes_pool_state -- --nocapture
```

**Step 2: 跑更宽的 control-plane 校验**

Run:

```bash
cargo test -p control-plane
```

**Step 3: 做本地 personal 运行态验证**

Run:

```bash
cargo check -p control-plane --no-default-features --features sqlite-backend --bin codex-pool-personal
```

**Step 4: 回填计划**

- 勾选完成项
- 在本文件补“执行结果”

## 执行结果

### 已完成实现

- 为 SQLite 持久化状态新增 `oauth_refresh_token_vault`
- 为账号健康态新增 `pool_state / quarantine_until / quarantine_reason / last_pool_transition_at`
- `SqliteBackedStore::queue_oauth_refresh_token()` 改为真正入冷库，不再直接物化账号
- `SqliteBackedStore::activate_oauth_refresh_token_vault()` 改为真实激活冷库账号，并持久化成功/失败状态
- `snapshot_inner()` 只下发 `active` 账号，`quarantine` 账号不会进入 data-plane snapshot
- `refresh_expiring_oauth_accounts_inner()` 只处理 `active` 账号
- `OAuthAccountStatusResponse` 暴露 `pool_state / quarantine_until / quarantine_reason`
- 保持 `InMemoryStore` 旧语义不变，把冷库热池能力限定在 SQLite 路径，避免破坏现有 API/集成行为

### 新增验证覆盖

- 冷库导入后账号不会立刻出现在 runtime account 列表
- 冷库记录在 SQLite reopen 后仍保持冷存，直到激活
- `quarantine` 账号不会进入 snapshot
- 已过期的 `quarantine` 会自动恢复进入 snapshot
- `refresh_expiring_oauth_accounts()` 会跳过 `quarantine` 账号
- `oauth_account_status()` 会暴露 pool/quarantine 状态
- 非 fatal 激活失败会写回 `failure_count/backoff/next_attempt_at`
- fatal 激活失败会将 vault 记录标为 `failed` 并停止继续尝试

### 最终验证命令与结果

已 fresh 执行并通过：

```bash
cargo test -p control-plane sqlite_backed_store_ -- --nocapture
cargo test -p control-plane -- --nocapture
cargo check -p control-plane --no-default-features --features sqlite-backend --bin codex-pool-personal
```

结果：

- `sqlite_backed_store_` 定向测试：`12 passed; 0 failed`
- `control-plane` 全量测试：单测 `133 passed; 0 failed`，integration `113 passed; 0 failed`
- `codex-pool-personal` SQLite 形态编译校验：`Finished dev profile`
