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
