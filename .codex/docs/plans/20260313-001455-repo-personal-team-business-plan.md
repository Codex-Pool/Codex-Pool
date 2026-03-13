# Personal / Team / Business 重构执行计划

> **For Codex:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** 将项目重构为 `personal`、`team`、`business` 三档产品线，并支持明确的升级/受限降级路径。

**Architecture:** 采用“共享核心 + 三个发行版”路线。先抽出 edition/capabilities 与跨版本共享接口，再逐步把当前 PostgreSQL / Redis / ClickHouse / 多租户 / 信用计费耦合拆开；第一阶段优先落地骨架与能力契约。

**Tech Stack:** Rust workspace（Axum / Tokio / SQLx）、React + Vite、SQLite / PostgreSQL、现有 Redis / ClickHouse（business 保留）。

---

## Summary

- 版本名统一为：`personal`、`team`、`business`
- 升级链路优先支持：
  - `personal -> team`
  - `personal -> business`
  - `team -> business`
- 降级采用“受限降级”：
  - `business -> team`
  - `team -> personal`
  - `business -> personal` 通过 staged migration，而不是原地热降级
- 第一阶段实现重点：
  - edition/capabilities 后端骨架
  - 前端 capability 感知
  - 版本命名与契约稳定化
  - 为后续 SQLite / Postgres-only / business-full 拆分准备接口层

## Important Changes

- 新增 Edition 模型：`personal | team | business`
- 新增 system capabilities 契约，前后端统一依据 capability 控制功能暴露
- 计费模式分离：
  - `cost_report_only`
  - `credit_enforced`
- `personal`：
  - 单 workspace
  - 多上游账号池
  - 无租户门户
  - SQLite
  - 单二进制
- `team`：
  - 轻量多租户
  - 简化 tenant portal
  - 默认 `app + postgres`
  - 不依赖 Redis / ClickHouse / PgBouncer
- `business`：
  - 保留现有多服务和全功能能力
  - PostgreSQL + Redis + ClickHouse

## Test Cases And Scenarios

- 基线回归
  - `cargo test --workspace --lib --bins --locked`
  - `frontend npm run build`
- 后端骨架
  - capabilities 接口返回 edition 与功能开关
  - 默认 edition 为 `business`，保持当前行为不变
  - capability 与 edition 组合符合约定
- 前端集成
  - admin app 能读取 capabilities
  - 被关闭能力的菜单/入口不显示
  - 缺失 capability 时 UI 有安全兜底

## Assumptions And Defaults

- `personal` 的“单账户”指单 workspace，不是单上游账号；账号池保留
- `team` 为轻量多租户，并保留简化租户门户
- `personal` / `team` 的计费仅做美元消耗展示，不做充值和余额控制
- `team` 默认部署形态为一个应用容器 + 一个 PostgreSQL
- `business -> personal` 不做无损原地降级

## Todo

- [x] 修复最新 `main` 上已存在的 4 个 `control-plane` 基线失败测试
- [x] 新增 edition/capabilities 核心类型与默认策略
- [x] 新增 capabilities API，并保证默认 `business` 向后兼容
- [x] 为 capabilities API 添加后端回归测试
- [x] 前端接入 capabilities 查询与缓存
- [x] 第一批基于 capability 的导航裁剪落地
- [x] 受影响范围验证通过，并回填本计划状态
- [x] 后端按 edition 收口 tenant portal / recharge / internal billing 路由暴露面
- [x] `auth validate` 在非 `business` 版本隐藏 `balance_microcredits`
- [x] `data-plane` 在非 `business` 版本强制关闭 credit billing 默认开关
- [x] `control-plane` 在非 `business` 版本禁用 billing reconcile 后台循环
- [x] 第二阶段运行时 edition 收口验证通过，并准备进入下一阶段
- [x] 为 `team` 版新增 PostgreSQL usage/log schema 与 query/ingest repo
- [x] 新增 `control-plane` 内部 usage ingest 路由与 TDD 回归
- [x] `data-plane` 在 `team` 且无 Redis 时切换到 control-plane HTTP event sink
- [x] 第三阶段 team Postgres-only usage pipeline 验证通过，并准备进入下一阶段
- [x] 为 usage summary / request logs 增加 `estimated_cost_microusd` 跨版本契约
- [x] `team` 版 PostgreSQL usage ingest/query 基于 pricing 解析写入请求成本
- [x] 非 `business` 的 admin / tenant Billing 页面切换为只读 cost report
- [x] 收口 `team/personal` 下的 recharge 与 billing-only system 指标入口
- [x] 第四阶段 non-business cost report 骨架验证通过，并准备进入下一阶段
- [x] 为 `personal` 版新增 SQLite 持久化 control-plane store
- [x] 为 `personal` 版新增 SQLite usage ingest/query repo
- [x] `control-plane` 在 `personal` edition 下自动接入 SQLite store 与 usage repo
- [x] `data-plane` 在 `personal` 且无 Redis 时切换到 control-plane HTTP event sink
- [x] 第五阶段 personal SQLite runtime foundation 验证通过，并准备进入下一阶段
- [x] `personal` 在单进程下合并 control-plane / data-plane 路由
- [x] `personal` 单进程内嵌前端静态资源并提供 SPA fallback
- [x] `personal` 强制收口 self-loopback 的 auth/snapshot/usage 运行时地址
- [x] 第六阶段 personal single-binary foundation 验证通过，并准备进入下一阶段
- [x] 将 single-binary 路由合并能力扩展到 `team`
- [x] 为 `team` 新增 `app + postgres` 最小 Docker Compose 产物
- [x] 修复 Rust Docker builder 对嵌入式前端产物的依赖链
- [x] 第七阶段 team single-binary deployment foundation 验证通过，并准备进入下一阶段
- [x] 设计并实现 edition migration 数据包格式与 preflight 规则
- [x] 为 `personal` 新增 SQLite control-plane / usage 导出导入回环能力
- [x] 为 `team/business` 新增 PostgreSQL control-plane / usage 导出导入骨架
- [x] 新增 `edition-migrate` CLI，支持 `export / preflight / import / archive inspect`
- [x] 第八阶段 edition migration upgrade foundation 验证通过，并准备进入下一阶段
- [x] 为 archive manifest 导出 raw payload rows，并让 `archive inspect` 返回样例摘要
- [x] 支持 `team -> personal` 单 tenant 受限降级导入
- [x] 支持 `business -> team` 的 archive-backed 受限降级 preflight
- [x] 第九阶段 archive-backed downgrade foundation 验证通过，并准备进入下一阶段
- [x] 放宽 `business -> personal` 的单 tenant archive-backed 受限降级
- [x] 补齐 `business -> personal` 的 SQLite 导入回归
- [x] 第十阶段 single-tenant business downgrade 验证通过，并准备进入下一阶段

## Progress Notes

- 已新增 `GET /api/v1/system/capabilities`，默认 edition 为 `business`，并覆盖 `personal` / `team` / `business` 三档能力矩阵。
- 管理端前端已接入 capabilities 查询，并基于 `multi_tenant`/`tenant_portal`/`tenant_self_service` 做第一批入口裁剪。
- `personal` 下已隐藏租户入口并停止默认租户 warmup；租户路径在 capability 关闭时不会再进入 tenant portal。
- `team` 下租户端已关闭注册、找回密码等自助入口，仅保留登录与已有页面导航。
- 当前仍属于第一阶段骨架实现，尚未开始 SQLite store、team Postgres-only pipeline、business/full 分布式拆分等后续工作。
- 第二阶段已把 capability 从“展示层”推进到“运行时边界”：
  - `personal` 不再注册 tenant portal、admin tenant credits、internal credit billing 等 business/team-only 路由
  - `team` 保留 tenant login/key/usage/logs，但关闭 self-service 注册找回与 recharge/credit 路由
  - `business` 保持完整 credit billing 路由面
- `/internal/v1/auth/validate` 现在会按 edition 裁剪 `balance_microcredits`，让 `personal/team` 自动退出 data-plane 的 credit enforcement 主链路。
- `data-plane` 配置已按 edition 强制关闭 metered stream billing / authorize for stream / dynamic preauth，避免非 `business` 版本被环境变量误开启 credit billing。
- `control-plane` 的 billing reconcile 后台循环已限制为仅 `business` 版本可启动。
- 第三阶段已打通 `team` 版的无 Redis / 无 ClickHouse usage 主链路：
  - `control-plane` 新增 `UsageIngestRepository`、`POST /internal/v1/usage/request-logs` 和 `PostgresUsageRepo`
  - PostgreSQL schema 新增 `usage_request_logs`、`usage_hourly_account`、`usage_hourly_tenant_api_key`、`usage_hourly_tenant_account`
  - `team` 版启动时会优先把 usage query/ingest 都接到 PostgreSQL，而 `business` 仍保持 ClickHouse 查询链路
- `data-plane` 现在会在 `team` 且无 Redis 时自动改走 control-plane HTTP sink，把 `RequestLogEvent` 直接投递到 `/internal/v1/usage/request-logs`，并继续保留 Redis 优先策略。
- 第三阶段验证已覆盖：
  - `cargo test -p control-plane --lib --bins`
  - `cargo test -p data-plane --lib --bins`
  - `cargo test --workspace --lib --bins --locked`
- 第四阶段已把 `personal/team` 的“只展示成本”语义接到 usage 面：
  - `UsageSummaryQueryResponse`、dashboard token trends、request logs 统一新增 `estimated_cost_microusd`
  - `team` 的 `PostgresUsageRepo` 会在 ingest 时解析 pricing，并把估算美元成本写入 `usage_request_logs`
  - `business` 现有 credit ledger 语义保持不变，但成本计算底座已经抽到共享 `cost` 模块
- 管理端与租户端 Billing 页面在 `credit_billing=false && cost_reports=true` 时会切到只读 cost report，不再展示充值/签到/信用账本交互。
- `team/personal` 下已进一步隐藏 tenant recharge 区块和 system 页中的 billing-only 观测指标卡。
- 第四阶段修复并验证了 `usage_worker::dual_level_aggregation`，避免同一小时桶在 worker flush 时被拆成重复 upsert 行。
- 第四阶段验证已覆盖：
  - `cargo test -p control-plane --locked`
  - `cd frontend && npm run i18n:check`
  - `cd frontend && npm run i18n:hardcode -- --no-baseline`
  - `cd frontend && node scripts/i18n/check-missing-runtime-keys.mjs`
  - `cd frontend && npm run lint`
  - `cd frontend && npm run build`
- 第五阶段已打通 `personal` 的 SQLite 运行时底座：
  - `control-plane` 新增 `SqliteBackedStore`，把 tenant、API key、upstream accounts、routing/model settings 等 state 以单行 JSON snapshot 形式持久化到 SQLite
  - `SqliteBackedStore::data_plane_snapshot_events` 在 revision 变化后返回 `cursor_gone`，让 `data-plane` 通过全量 snapshot reload 保持一致性
  - `control-plane` 新增 `SqliteUsageRepo`，在 SQLite 中保存 request logs，并提供 summary、dashboard、leaderboard、request logs 查询
  - `personal` edition 启动时会强制选择 SQLite，并把 usage ingest/query 一并切到 `SqliteUsageRepo`
  - `data-plane` 现在会在 `personal` 且无 Redis 时自动改走 control-plane HTTP sink，避免 personal 版对 Redis 的运行时依赖
- 第五阶段验证已覆盖：
  - `cargo check -p control-plane`
  - `cargo check -p data-plane`
  - `cargo test -p control-plane sqlite_ -- --nocapture`
  - `cargo test -p control-plane --lib --bins`
  - `cargo test -p data-plane select_event_sink_kind -- --nocapture`
  - `cargo test -p data-plane --lib --bins`
- 第六阶段已把 `personal` 的部署形态从“SQLite 双进程”推进到“单二进制单端口”：
  - `control-plane` 现在会在 `personal` 下强制把 `CONTROL_PLANE_LISTEN`、`CODEX_OAUTH_CALLBACK_LISTEN`、`CONTROL_PLANE_BASE_URL`、`AUTH_VALIDATE_URL` 收口到同一个 loopback/self-base-url
  - `data-plane` 新增“不带 `/health` `/livez` `/readyz`”的子路由构造器，避免与 `control-plane` 主路由冲突
  - `control-plane` 在 `personal` 下会把 data-plane 的 `/v1/*`、`/backend-api/*`、`/api/codex/usage` 路由直接 merge 到同一个 Axum app 中
  - `control-plane` 新增 `personal` 前端 fallback：编译时会把 `frontend` 构建产物嵌入二进制，运行时统一由单端口返回 SPA shell 和静态资源
  - 为了避免 `frontend/dist` 过期，`control-plane` 的 `build.rs` 会在构建 personal 单二进制时自动执行 `frontend` 的打包流程，再复制产物做嵌入
- 第六阶段验证已覆盖：
  - `cargo check -p control-plane`
  - `cargo check -p data-plane`
  - `cargo test -p control-plane personal::tests -- --nocapture`
  - `cargo test -p data-plane build_app_without_status_routes -- --nocapture`
  - `cargo test -p control-plane --lib --bins`
  - `cargo test -p data-plane --lib --bins`
- 第七阶段已把 single-binary 能力从 `personal` 推广到 `team`：
  - `control-plane` 的 single-binary runtime defaults 与路由 merge 现在会同时覆盖 `personal` 和 `team`
  - `team` 版也会把 admin UI、tenant UI、control-plane API、`/v1/*` 代理统一挂到单端口应用上
  - `control-plane` 的嵌入式前端构建链路现在会在 `node_modules` 缺失时自动执行 `npm ci`，再执行 `npm run build`
  - `docker/rust-runtime.Dockerfile` 已补齐 `frontend` 目录复制与 Node/npm 依赖，避免嵌入式前端构建在 Docker builder 中失败
  - 新增 `docker-compose.team.yml` 与 `docker/.env.team.example`，支持 `team` 版默认 `app + postgres` 启动方式
- 第七阶段验证已覆盖：
  - `cargo test -p control-plane single_binary::tests -- --nocapture`
  - `cargo test -p control-plane --lib --bins`
  - `docker compose --env-file docker/.env.team.example -f docker-compose.team.yml config`
- 第八阶段已落地 edition migration 的第一版“可执行升级链路”：
  - `control-plane` 新增 `edition_migration` 模块，定义统一迁移包、archive manifest 与 preflight 报告结构
  - `store` 新增 SQLite / PostgreSQL control-plane bundle 导出导入能力，保留 tenant / api key / upstream account / routing / OAuth credential 等原始 ID
  - `usage` 新增跨版本 request-log 迁移包，`personal -> team/business` 与 `team -> business` 导入时会按 request logs 重建 PostgreSQL 小时聚合表
  - 新增 `edition-migrate` 二进制，支持 `export`、`preflight`、`import`、`archive inspect`
  - `team -> personal` 与 `business -> team/personal` 当前仍是受限降级 preflight；本阶段只导出了 archive manifest，还没有落 raw archive payload
- 第八阶段验证已覆盖：
  - `cargo test -p control-plane edition_migration::tests -- --nocapture`
  - `cargo test -p control-plane --bin edition-migrate -- --nocapture`
  - `cargo test -p control-plane --lib --bins`
- 第九阶段已把受限降级从“manifest 提示”推进到“archive-backed 可执行链路”：
  - `archive manifest` 现在会携带 `tenant_users`、`tenant_credit_*` 的 raw payload rows，后续 staged migration 不再只剩 count
  - `archive inspect` 改为返回摘要和样例行，避免大包直接刷满终端，同时仍保留被归档数据的结构证据
  - `preflight` 现在允许 `team -> personal` 在单 tenant 条件下继续导入，并把 tenant users 作为 archive warning 保留
  - `preflight` 现在允许 `business -> team` 继续导入，并把 credit accounts / ledger / authorizations 作为 archive warning 保留
  - `business -> personal` 仍明确要求 staged migration，不支持直接导入
- 第九阶段验证已覆盖：
  - `cargo test -p control-plane edition_migration::tests -- --nocapture`
  - `cargo test -p control-plane --bin edition-migrate -- --nocapture`
  - `cargo test -p control-plane --lib --bins`
- 第十阶段已把 `business -> personal` 从“必须 staged migration”放宽到“单 tenant 可直接 archive-backed 导入”：
  - `preflight` 现在会在 `business` 迁移包只有一个 tenant 时允许直接导入 `personal`
  - 多 tenant 的 `business -> personal` 仍保持 blocker，避免无提示地把多个 workspace 挤进单 workspace 运行态
  - 新增 `business -> personal` SQLite 导入回归，确保 archive warning 不会阻塞核心 control-plane state 落地
- 第十阶段验证已覆盖：
  - `cargo test -p control-plane edition_migration::tests -- --nocapture`
  - `cargo test -p control-plane --lib --bins`
