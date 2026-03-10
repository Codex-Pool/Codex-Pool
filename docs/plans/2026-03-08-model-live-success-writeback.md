# Model Live Success Writeback Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让 data-plane 上一次真实成功调用的模型，立即回写到 control-plane 的模型可用性缓存，避免 `gpt-5.3-codex` 这类模型在前端继续显示为不可用。

**Architecture:** 复用现有 data-plane → control-plane 的 `seen_ok` 内部回报链路，新增一个 `model seen_ok` 内部接口。data-plane 在请求真正成功后，按 `(account_id, model)` 做最小频控并异步回报；control-plane 收到后直接更新内存中的 `model_probe_cache`，把对应模型标记为 `available`，并刷新 `checked_at/http_status/error`。

**Tech Stack:** Rust, Axum, reqwest, tokio, control-plane/data-plane internal API.

---

### Task 1: 为 control-plane 增加 model seen_ok 写回入口

**Files:**
- Modify: `services/control-plane/src/app/core_handlers/upstream_health.rs`
- Modify: `services/control-plane/src/app/core_handlers/models_probe.rs`
- Modify: `services/control-plane/src/app.rs`
- Test: `services/control-plane/src/app/core_handlers/models_probe.rs`
- Test: `services/control-plane/tests/api/base_and_core_part2.rs`

- [x] 写一个失败测试，覆盖“收到 model seen_ok 信号后把模型标记为 available”
- [x] 新增内部请求体与处理函数，要求 internal token
- [x] 在 `model_probe_cache` 中按 model id 回写 `available/checked_at/http_status/error`
- [x] 注册新的 internal route
- [x] 运行 control-plane 相关测试

### Task 2: 为 data-plane 增加 model seen_ok reporter

**Files:**
- Modify: `services/data-plane/src/upstream_health.rs`
- Modify: `services/data-plane/src/proxy/entry.rs`
- Test: `services/data-plane/src/upstream_health.rs` 或相邻测试文件

- [x] 写一个失败测试，覆盖 reporter 对 `(account_id, model)` 的频控逻辑
- [x] 扩展 reporter，新增 `report_model_seen_ok(account_id, model)`
- [x] 在 HTTP 成功路径带上 model 做异步回报；WS 暂保留账号级 `seen_ok`
- [x] 运行 data-plane 相关测试

### Task 3: 验证用户场景

**Files:**
- Modify: `docs/plans/2026-03-08-model-live-success-writeback.md`

- [x] 运行 `cargo test -p control-plane` 的针对性测试
- [x] 运行 `cargo test -p data-plane` 的针对性测试
- [x] 运行 `cargo check -p control-plane`
- [x] 运行 `cargo check -p data-plane`
- [x] 回填本计划中的 todo 完成状态


### Task 4: 为模型列表增加 recent-success 覆盖

**Files:**
- Modify: `services/control-plane/src/app/core_handlers/models_probe.rs`
- Modify: `services/control-plane/src/app/core_handlers/billing_runtime.rs`
- Test: `services/control-plane/src/app/core_handlers/models_probe.rs`

- [x] 让 `GET /api/v1/admin/models` 在读缓存时也叠加 recent-success
- [x] 让 recent-success 覆盖后的 `probe_cache_updated_at` 反映最新可见时间
- [x] 运行 recent-success 列表覆盖的定向测试

### Task 5: 细化 internal billing 错误映射

**Files:**
- Modify: `services/control-plane/src/app/core_handlers/billing_runtime.rs`
- Test: `services/control-plane/src/app/core_handlers/billing_runtime.rs`

- [x] 把 `model must not be empty` 映射到稳定的 `billing_model_missing`
- [x] 把 `billing authorization is in invalid status` 映射到稳定的 `billing_authorization_invalid_status`
- [x] 为未分类 internal billing 错误增加服务端日志
- [x] 运行错误映射的定向测试
