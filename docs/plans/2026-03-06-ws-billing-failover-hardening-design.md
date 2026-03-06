# WS / Billing / Failover Hardening Design

## 背景

当前 `Codex-Pool` 的主链路已经具备：

- HTTP / SSE / WS 代理
- 预扣 / 捕获 / 释放 / reconcile 计费闭环
- 同账号快速重试与跨账号 failover
- `previous_response_id` / `x-codex-turn-state` / `session_id` 参与路由粘性

但近期审查暴露出三个系统性问题：

1. **账务幂等键与客户端输入耦合过深**，`request_id` 可被客户端影响，且会命中已 `released/captured` 授权。
2. **WS logical request 与 transport connection 的边界不够硬**，在弱 ID / close / replay 场景下，usage、capture、release 容易错绑或漏清理。
3. **续链语义利用不足**，`previous_response_id` 目前更多是弱粘性提示，没有成为“账号续链锚点 + 错误后重建策略”的一等公民。

## 目标

本次 hardening 目标不是重做整套代理，而是在尽量小的改动下实现：

- 将**账务幂等**与**客户端 request id**彻底脱钩
- 将 WS 的 **logical request lifecycle** 做成可证明收敛的状态机
- 将 `previous_response_id / turn-state / session` 明确区分为**续链锚点**而不是账务键
- 在不破坏现有接口的前提下，提高 request log / usage / failover 归因的真实性

## 方案对比

### 方案 A：最小补丁

- 保留现有 `request_id` 语义
- 仅修补 `released` 授权复用、WS close 清理、`response.failed` 处理

优点：

- 改动最小
- 风险最低

缺点：

- 客户端可控 `request_id` 仍会继续污染账务边界
- 后续还会再次遇到“logical request vs transport request”混淆问题

### 方案 B：推荐方案（分层键语义）

- 引入**服务端生成的 billing operation key / logical request key**
- 将 `x-request-id` 保留为 tracing / correlation 字段
- 将 `previous_response_id`、`session_id`、`x-codex-turn-state` 只用于续链 / 路由 / 账号选择
- 明确 WS logical request 状态流：`registered -> started -> completed | failed | interrupted`

优点：

- 根治当前高风险计费漏洞
- 结构清晰，后续便于补更多 WS 回归测试
- 与 `~/codex`、`~/sub2api` 的优点兼容

缺点：

- 需要调整 data-plane 与 control-plane 的内部账务契约
- 需要补一批测试

### 方案 C：完全重构为 continuation-aware account state machine

- 把 continuation、routing、billing、health 全部抽象成新的状态机层

优点：

- 长期最优

缺点：

- 当前范围过大
- 容易把一次 hardening 演变成长期重构

## 选择

采用 **方案 B**，并按两阶段落地：

1. **Phase 1：止血**
   - 修账务幂等键
   - 修 WS close 清理
   - 修 `response.failed`
   - 补高风险回归测试
2. **Phase 2：增强**
   - 提升 `previous_response_id` 为续链选账号主锚点
   - 改善 WS 弱 ID 关联策略
   - 改善 stream failover 账号归因

## 设计决策

### 1. 键语义分层

引入四类键，禁止混用：

- **trace request id**：来自 `x-request-id` 或中间件生成；只用于日志与关联排障
- **logical request key**：服务端为每个 billable logical request 生成；用于计费 authorize/capture/release
- **continuation key**：来自 `previous_response_id`
- **routing sticky key**：来自 `session_id` / `x-codex-turn-state` / prompt cache key

### 2. control-plane 账务幂等规则

- `billing_authorize` 只应复用**仍处于可复用状态**的未终结授权
- 对已经 `released` / `captured` 的记录，不得直接返回为“当前请求授权”
- 必要时新增独立列或 meta 字段承载 `logical_request_key`

### 3. WS logical request 生命周期

对于每条 `response.create`：

- 注册独立 logical request 记录
- 若收到 `response.created`，标记 started
- 若收到 `response.completed|response.done`，capture / finalize
- 若收到 `response.failed|response.incomplete|error`，按原因 release
- 若连接结束且仍未完成，统一走 interrupted release

重点是：**任何退出路径都必须进入统一清理段**，不能在 close 上直接跳出并绕过 billing cleanup。

### 4. continuation-aware failover

借鉴 `~/sub2api`：

- `previous_response_id` 先于 session sticky 参与选账号
- 若 continuation 锚点失效，允许明确的一次性恢复分支
- 但若请求含 `function_call_output` 或等价强状态依赖，不允许随意丢 continuation 重放

### 5. request log / usage 归因

- request log 继续记录 `x-request-id`
- usage / billing 事件记录新的 logical request key 与 authorization id
- stream failover 成功后，最终账单日志必须以**实际 capture 所在账号**记账

## 非目标

- 不在本次变更中重做整套 router / scheduler
- 不引入新的外部基础设施
- 不修改对外公开 API 形状，除非内部调试字段增加不影响兼容性

## 验收标准

- 重复客户端 `x-request-id` 不再造成逃费
- WS replay / rebind 不再命中旧 `released` 授权漏扣费
- WS 上游 close / failed / incomplete 都能可靠 release 未完成 hold
- `response.failed` 明确进入失败状态处理
- stream failover 后 usage / request log 的账号归因正确
- 新增回归测试稳定覆盖上述场景

