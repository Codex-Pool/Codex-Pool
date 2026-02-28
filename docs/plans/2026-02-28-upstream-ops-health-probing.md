# 账号池主动测活 + UpstreamOps 集中管理（Probe / RT 轮转 / RateLimit）Implementation Plan

**Goal:** 为账号池引入“主动测活（PING /responses）”能力，让 **最鲜活的账号优先被调度**，显著降低“死号太多导致用户请求长时间轮询/切号”的等待；同时抽象出一个集中管理层（UpstreamOps）统一承载 control-plane 侧所有“后台向上游请求”（测活、RT 轮转/刷新、RateLimit 获取等），避免分散实现带来的策略不一致与上游请求风暴风险。

**Architecture:**  
1) control-plane 新增 `UpstreamOps` 子系统：统一队列化/预算化所有后台上游请求，并通过 per-account op lock 保证互斥与可控并发。  
2) control-plane 新增账号健康状态表（持久化）+ Redis 健康缓存（`alive_ring`）：测活/seen_ok 的“真实信号”写入 Redis，用于 data-plane 优先选活号。  
3) data-plane 保持现有用户真实请求链路不变（sticky + failover + ejection），仅在“挑选候选账号”阶段新增“优先从 alive_ring 取活号”的路径；用户真实请求成功会 best-effort 回灌 `seen_ok` 给 control-plane，用于抑制对热账号的重复测活。

**Tech Stack:** Rust, axum, reqwest, tokio, sqlx(Postgres), redis, wiremock tests

---

## 0) 现状梳理（当前项目怎么做）

### 0.1 data-plane：轮询/粘性/故障切换/健康隔离

- **账号挑选**：`RoundRobinRouter`（RR cursor） + sticky session（`session_id`/`conversation_id`）优先复用同账号。
  - 代码：`services/data-plane/src/router.rs`（`pick_with_policy`、`sticky_sessions`）
- **临时隔离（unhealthy/ejection）**：
  - 本地：`router.mark_unhealthy(account_id, ttl)`（进程内 `HashMap<Uuid, Instant>`）
  - 共享：`RoutingCache`（Redis）保存 `unhealthy:{account_id}` 与 `sticky:{sticky_key}`，并在 pick 前查询 `is_unhealthy` 跳过。
  - 代码：`services/data-plane/src/routing_cache.rs`、`services/data-plane/src/proxy/entry.rs`（会先把 Redis sticky rehydrate 到 router）
- **失败驱动**：请求失败后会触发 `mark_unhealthy + TTL`，并在 failover 窗口内跨账号重试（但当死号存量很大时，首次遇到每个死号仍会消耗时间）。
  - 代码：`services/data-plane/src/proxy/entry.rs`（failover loop）
- **结论**：当前 data-plane 没有“主动测活/全局活号优先”的信号源，主要依赖请求失败后的被动隔离；当死号比例高时，会出现“用户请求需要撞很多个死号才成功”的体验问题。

### 0.2 control-plane：RT 轮转、RateLimit 查询、探测现状

- **账号存储与下发**：control-plane 存于 Postgres，data-plane 通过 snapshot/outbox 拉取账号列表（`enabled` 等）。
- **OAuth / RT 轮转**：control-plane 存在 OAuth refresh 相关后台逻辑（用于刷新 access token、处理 refresh token 旋转、记录状态等）。
- **RateLimit 查询**：control-plane 有 rate-limit snapshot/cache 与刷新 job（主要用于 UI/运营侧观测，不是 data-plane 的实时路由前置条件）。
- **已有“模型探测 loop”**：control-plane 已存在 `models_probe` 的后台探测循环，可复用其“定时循环 + 并发请求 + 缓存写回”的工程模式。
  - 代码：`services/control-plane/src/app/core_handlers/models_probe.rs`
- **上游 URL 构建耦合点**：`build_upstream_responses_url` 当前定义在 `import_jobs` handler 内，但被 `models_probe.rs` 复用（存在隐式耦合），建议抽离为独立模块以便 `UpstreamOps` 复用。
  - 代码：`services/control-plane/src/app/tail_handlers/import_jobs.rs`

---

## 1) 目标行为与 Non-Goals

### 1.1 目标行为

- data-plane 在挑选账号时优先使用“最近被测活为 OK 或刚被真实请求验证 OK”的账号集合，减少死号碰撞。
- 账号健康信号遵循“真实信号优先”：
  - 一级：主动测活 `probe_ok/probe_fail`（通过上游 `POST /responses` 得到）
  - 一级补充：用户真实请求成功回灌 `seen_ok`（只回灌成功，不回灌失败）
  - 二级：401/429/refresh/rate-limit 等“推断信号”不直接驱动 alive_ring，但可影响下一次 probe 调度紧迫度（减少无意义探测/避免风暴）。
- 对上游的后台请求严格受预算控制（RPS + 并发 + backoff），避免因规模增长或 UI 操作导致探测/刷新风暴。

### 1.2 Non-Goals（本期不做）

- 不把“用户真实请求”纳入 UpstreamOps 集中层（用户请求仍由 data-plane 直接 proxy），本方案只管理后台请求。
- 不承诺“完全无死号”，只保证“活号优先 + 死号快速降权”。
- UI 侧健康状态只展示“进行中/完成”的导入任务需求不在本文范围（本文聚焦账号池健康与调度）。

---

## 2) 关键决策记录（已确认）

- `seen_ok` 回灌策略：**B**（只回灌成功，不回灌失败；成功用于抑制 probe，不直接判死）
- `seen_ok` 上报方式：**1**（data-plane 在用户请求成功后异步 best-effort 调 control-plane internal API，上报需节流/去重）
- 探活结果下发方式：**2**（不走 snapshot/outbox，走 Redis 健康缓存，避免 outbox 风暴）
- Redis 方案：**2b**（`alive_ring` 活号环；可选 per-account TTL key 作为排障/兜底）

---

## 3) 核心设计

### 3.1 信号模型与“真实优先”原则

我们把账号健康相关信号分两层：

- **真实信号（Truth）**：来自“确实向上游发送请求并得到结果”的事件。
  - `probe_ok`：control-plane 主动 `PING /responses` 成功
  - `probe_fail`：control-plane 主动 `PING /responses` 失败
  - `seen_ok`：data-plane 用户真实请求成功回灌（只记录 ok，不记录 fail）
- **推断信号（Inferred）**：来自错误分类、刷新失败、rate-limit 拉取失败等间接事件。
  - 这些信号在本期 **不直接更新 alive_ring 的排序**（避免误判导致活号被过度降权），但可用于：
    - 缩短/延长 `next_probe_at`
    - 触发“尽快复查”（把账号放入更靠前的 probe 队列）

同一层信号按时间单调覆盖（新事件覆盖旧事件），避免旧事件倒灌。

### 3.2 UpstreamOps：集中管理后台上游请求

目标：把 control-plane 中所有“后台向上游请求”的执行策略收敛到一个地方，统一处理：

- 全局预算：`max_rps` / `max_concurrency`
- per-account 互斥：同一账号同一时间最多执行一个后台 op（probe/refresh/rate_limit）
- backoff + jitter：失败后指数退避，避免集中重试
- 观测：统一埋点与审计日志

建议落地形态：

- `UpstreamOpsExecutor`：提供 `enqueue(op)` / `run_loop()` / `run_once()` 等，内部做节拍器 + 并发池
- `UpstreamOpType`：`probe | oauth_refresh | rate_limit_fetch`
- `UpstreamOp`：`{ account_id, op_type, reason, deadline, created_at }`
- `UpstreamOpLock`：DB 级别“可恢复互斥锁”，防止多实例/多线程重复对同账号打上游

> 迁移策略：本期先把 **probe** 纳入 UpstreamOps；后续逐步把 OAuth refresh 与 rate-limit refresh job 迁入。

### 3.3 主动测活（Probe）机制

**探活方式：**对每个候选账号向其上游发送一次最小 `POST /responses`：

- model：`gpt-5.1-codex-mini`
- input：`"PING"`
- `max_output_tokens`：尽量小（例如 `1`）
- 超时：短（例如 2s-5s，配置化）
- header：`authorization: Bearer ...`；若账号带 `chatgpt_account_id` 则附加 `chatgpt-account-id`（与现有 model probe 逻辑一致）
- URL 构建：复用/抽离 `build_upstream_responses_url(base_url, mode)`

**成功条件：**

- HTTP 2xx 且能解析出标准 responses 返回（不强依赖内容），视为 `probe_ok`

**失败条件：**

- 网络错误/超时/非 2xx/解析失败 视为 `probe_fail`（记录分类与时间）

**调度规则：**

- `next_probe_at` 到期才会被探测（避免全量扫）
- 若 `seen_ok_at` 在抑制窗口内（例如 10 分钟），跳过 probe（热账号用真实流量替代探活）
- 若账号处于 ops_lock inflight 中，跳过
- probe 成功：把账号写入 `alive_ring` 并置顶；`next_probe_at = now + ok_interval + jitter`
- probe 失败：从 `alive_ring` 移除（或至少不置顶）；`next_probe_at = now + backoff(failure_count) + jitter`

### 3.4 `seen_ok` 回灌（抑制热账号测活）

当用户真实请求已经证明账号可用时，该账号 **不需要再频繁参与主动测活**。

- data-plane 在一次用户真实请求成功后，异步 best-effort 调用 control-plane internal API：
  - `POST /internal/v1/upstream-accounts/{account_id}/health/seen-ok`
  - 语义：`seen_ok_at = max(seen_ok_at, now)`
  - data-plane 侧按 `account_id` 做节流（例如同一账号 60s 内最多上报一次）
- control-plane 收到 `seen_ok` 后：
  - 更新 `upstream_account_health_state.seen_ok_at`
  - 把账号写入 `alive_ring` 并置顶（因为真实流量已证明可用）
  - 在未来一段窗口内抑制该账号 probe

> 注意：本期 `seen_ok` 不回灌失败，因此不会造成“误伤”活号；失败仍由 data-plane 的 ejection 与 shared unhealthy key 发挥作用。

### 3.5 Redis `alive_ring`（活号优先调度）

**Key 设计：**

- `codex_pool:health:alive_ring:v1`（Redis LIST）
  - 头部：最新 `probe_ok/seen_ok` 的账号
  - 容量：`N`（配置化，例如 5k 或更小）
- 可选：
  - `codex_pool:health:alive:{account_id}`（STRING + TTL）
  - `codex_pool:health:dead:{account_id}`（STRING + TTL）

**原子更新：**使用 Lua 保证“去重 + 置顶 + 截断”原子性：

- `LREM key 0 {account_id}`
- `LPUSH key {account_id}`
- `LTRIM key 0 N-1`

probe_fail 时可做：

- `LREM key 0 {account_id}`
- （可选）`SET dead:{id} 1 EX ttl`

### 3.6 data-plane 调度策略（如何用 alive_ring，不破坏现有行为）

data-plane 的策略目标：在不破坏 sticky/failover/ejection 的前提下，让 pick 的候选更“活”。

建议策略（可配置开关）：

1) **若存在 sticky 命中**：继续优先 sticky（保持会话一致性与成本可控），但仍要检查 `routing_cache.is_unhealthy`。  
2) **否则优先 alive_ring**：从 alive_ring 取一批候选（例如 top_k=200），在本进程内做 RR cursor（避免永远打同一个头部账号），并逐个验证：
   - 账号仍存在于 router 账号表，且 `enabled=true`
   - 未被 `routing_cache.is_unhealthy` 标记
3) 若 alive_ring 不可用/为空：完全回退到现有 `router.pick_with_policy`（RR + sticky_conflict_avoid + 本地健康）

> 与现有 shared unhealthy 的配合：即便 alive_ring 中混入刚“变死”的账号，首次失败会触发 data-plane `set_unhealthy`（Redis），从而让其他 data-plane 实例在短 TTL 内跳过它，直到 control-plane 下次 probe 复查并最终从 alive_ring 移除/再置顶。

---

## 4) 数据结构与接口契约

### 4.1 Postgres：`upstream_account_health_state`（新增）

用途：持久化健康状态与 probe 调度信息，支撑 UpstreamOps 的选取与 backoff。

建议字段（最小可用版本）：

- `account_id UUID PRIMARY KEY REFERENCES upstream_accounts(id)`
- `seen_ok_at TIMESTAMPTZ NULL`
- `last_probe_at TIMESTAMPTZ NULL`
- `last_probe_status TEXT NULL`（`ok|fail`）
- `last_probe_http_status INT NULL`
- `last_probe_error_code TEXT NULL`
- `last_probe_error_message TEXT NULL`（截断保存，避免泄露上游细节）
- `failure_count INT NOT NULL DEFAULT 0`
- `next_probe_at TIMESTAMPTZ NULL`
- `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`
- `updated_at TIMESTAMPTZ NOT NULL DEFAULT now()`

索引建议：

- `INDEX (next_probe_at)`
- `INDEX (seen_ok_at)`

### 4.2 Postgres：`upstream_account_ops_locks`（新增）

用途：跨实例互斥后台 op，避免同一账号同时 probe + refresh + rate_limit 等。

建议字段：

- `account_id UUID NOT NULL REFERENCES upstream_accounts(id)`
- `op_type TEXT NOT NULL`（`probe|oauth_refresh|rate_limit_fetch`）
- `inflight_until TIMESTAMPTZ NOT NULL`
- `claimed_at TIMESTAMPTZ NOT NULL`
- `claimed_by TEXT NOT NULL`（实例标识）
- `PRIMARY KEY (account_id, op_type)`

claim 语义（示意）：

- `UPDATE ... SET inflight_until = now()+ttl, claimed_at=now(), claimed_by=$me WHERE inflight_until < now() RETURNING ...`
- 或者使用 `SELECT ... FOR UPDATE SKIP LOCKED` 方式从候选集中挑选后再更新

### 4.3 Internal API：`seen_ok` 上报（新增）

- `POST /internal/v1/upstream-accounts/{account_id}/health/seen-ok`
- Auth：复用现有 internal token（`CONTROL_PLANE_INTERNAL_AUTH_TOKEN`）
- 行为：
  - 幂等：多次调用只会把 `seen_ok_at` 推进到更晚
  - 节流：control-plane 对同一账号可做最小写入间隔（例如 10s）以防写放大
- Response：`200 { ok: true }`（并带 `x-request-id`）

---

## 5) 配置与可观测性

### 5.1 建议新增配置（示例命名）

control-plane：

- `CONTROL_PLANE_UPSTREAM_OPS_ENABLED=true`
- `CONTROL_PLANE_UPSTREAM_PROBE_ENABLED=true`
- `CONTROL_PLANE_UPSTREAM_PROBE_TICK_SEC=5~30`
- `CONTROL_PLANE_UPSTREAM_PROBE_BATCH_SIZE=50~500`
- `CONTROL_PLANE_UPSTREAM_PROBE_MAX_RPS=1~5`
- `CONTROL_PLANE_UPSTREAM_PROBE_CONCURRENCY=8~64`
- `CONTROL_PLANE_UPSTREAM_PROBE_TIMEOUT_MS=2000~5000`
- `CONTROL_PLANE_UPSTREAM_PROBE_OK_INTERVAL_SEC=300~1800`
- `CONTROL_PLANE_UPSTREAM_PROBE_FAIL_MIN_INTERVAL_SEC=30`
- `CONTROL_PLANE_UPSTREAM_PROBE_FAIL_MAX_INTERVAL_SEC=3600`
- `CONTROL_PLANE_UPSTREAM_PROBE_SEEN_OK_SUPPRESS_SEC=600`
- `CONTROL_PLANE_HEALTH_REDIS_PREFIX=codex_pool:health`
- `CONTROL_PLANE_ALIVE_RING_SIZE=5000`

data-plane：

- `DATA_PLANE_ALIVE_RING_ROUTING_ENABLED=true`
- `DATA_PLANE_ALIVE_RING_TOP_K=200`
- `DATA_PLANE_ALIVE_RING_CACHE_TTL_MS=1000~5000`
- `DATA_PLANE_SEEN_OK_REPORT_ENABLED=true`
- `DATA_PLANE_SEEN_OK_REPORT_MIN_INTERVAL_SEC=60`

### 5.2 指标（建议）

control-plane：

- `upstream_probe_total{result=ok|fail}`
- `upstream_probe_latency_ms`
- `upstream_probe_claim_total{result=claimed|skipped}`
- `upstream_ops_lock_contention_total{op_type}`
- `alive_ring_update_total{op=push|remove}`
- `seen_ok_ingest_total{result=accepted|throttled|error}`

data-plane：

- `alive_ring_fetch_total{result=hit|miss|error}`
- `pick_source_total{source=sticky|alive_ring|rr}`
- `seen_ok_report_total{result=ok|error|skipped_throttle}`

---

## 6) 落地任务拆分（推荐按 Phase）

### Phase 1：最小可用闭环（probe + alive_ring + seen_ok）

**Task 1：抽离上游 URL 构建工具**
- 把 `build_upstream_models_url` / `build_upstream_responses_url` 从 handler 抽到独立模块（供 probe/refresh/rate_limit 复用），消除隐式 include 依赖。

**Task 2：新增健康状态表与 ops lock 表**
- 增加 `upstream_account_health_state`
- 增加 `upstream_account_ops_locks`
- 提供最小 store API：`upsert_seen_ok`、`claim_probe_lock`、`save_probe_result`、`pick_probe_candidates`

**Task 3：实现 UpstreamOps（仅 probe op）**
- 引入 probe scheduler loop（可参考 `models_probe` 的工程结构）
- 预算（RPS+并发）+ backoff+jitter
- probe_ok/probe_fail 写 DB + 更新 Redis alive_ring

**Task 4：实现 internal seen_ok API**
- control-plane 新增 internal route + auth
- 写入 `seen_ok_at` + 更新 alive_ring（置顶）

**Task 5：data-plane alive_ring 优先调度 + seen_ok 上报**
- 增加 Redis 读取 alive_ring 的 client（复用 data-plane 既有 Redis 依赖与错误处理风格）
- 在 pick 账号阶段引入 alive_ring 优先路径，失败回退 RR
- 在用户请求成功后异步 best-effort 上报 seen_ok（带节流/去重）

### Phase 2：把 RateLimit 获取迁入 UpstreamOps

- 将现有 rate-limit refresh job 改造成 `UpstreamOpType::rate_limit_fetch`
- 和 probe 共用预算/互斥锁/回退策略

### Phase 3：把 OAuth refresh/RT 轮转迁入 UpstreamOps

- 将现有 OAuth refresh loop/job 改造为 `UpstreamOpType::oauth_refresh`
- 与 probe/rate_limit 共用互斥与预算，避免同账号多任务并发打上游

---

## 7) 测试策略（建议最小集）

- control-plane：
  - probe URL builder 单测（覆盖不同 base_url/mode）
  - seen_ok API：幂等 + 节流 + x-request-id
  - probe scheduler：候选过滤（seen_ok 抑制、next_probe_at、锁竞争）
  - Redis alive_ring Lua：去重置顶与 trim 的正确性
- data-plane：
  - alive_ring pick：命中/空/Redis 失败回退 RR
  - seen_ok 上报节流：不影响主请求路径（best-effort）

---

## 8) 风险与对策

- **上游探测成本/风控风险**：probe 必须严格预算化，默认低频；seen_ok 抑制热账号 probe，减少重复打上游。
- **alive_ring 热点账号过载**：data-plane 对 alive_ring 进行 RR（或 top_k 随机/轮转）避免永远命中 list 头部。
- **Redis 不可用**：data-plane 必须完全回退现有 RR+sticky+ejection；control-plane probe 仍可写 DB，但不影响路由正确性。
- **推断信号误伤**：本期二级信号不直接驱动 alive_ring；仅通过 probe 复查后决定活/死排序。

