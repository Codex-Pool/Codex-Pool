# AGENTS

## 语言约束（必读）

- 永远用中文思考、总结和回答问题。
- 面向用户的最终说明、计划、风险提示、提交说明默认使用中文；命令、路径、类型名、接口名保留原文。

## 当前项目状态（必读）

- 项目现在是 `admin / tenant` 双门户，不是只有一个后台。
- 项目同时支撑 `personal / team / business` 三层产品形态，不允许默认按 `business` 思路写功能。
- Codex-Pool 是高频使用的 B2B 控制台产品，不是营销站点，不是极客实验工具，也不是可视化玩具。
- 默认设计目标仍然是：高端精致、可信克制、专业清晰。
- 明确避免两类偏差：
  - 老式企业 OA：厚重、陈旧、机械、没有呼吸感。
  - 极客工具页：过度终端化、监控屏化、只对开发者友好。
- 页面视觉强度默认中等，任务效率、信息层级、状态清晰度优先于装饰效果。
- 当前前端主工作台已经收敛为：
  - `Dashboard`
  - `Accounts`
  - `Logs`
  - `Imports`
  - `OAuth Import`
  - `Model Routing`
  - `Models`
  - `Usage`
  - `Billing`
  - `Proxies`
  - `Config`
  - `System`
- 当前账号资产主工作流是 `/accounts`，不是 `/inventory`。
- `Inventory` 现在只是兼容跳转壳，不再是独立主工作流。
- `Logs` 已经是统一事件工作台，不再只是普通系统日志页。
- `OAuth Probe` 已经被前后端彻底删除；后续不要恢复页面、路由、API、语言包文案或后端状态。

## 三层架构（必读）

### 1) `personal`

- 定位：个人/单机自托管，单 workspace，单管理员入口。
- 运行形态：单二进制/单容器，admin UI + control-plane API + `/v1/*` data-plane 代理统一入口。
- 存储：SQLite。
- 依赖边界：不依赖 PostgreSQL、Redis、ClickHouse、独立 frontend 容器。
- 功能边界：
  - 不支持多租户管理工作流作为主要形态。
  - 不支持 tenant self-service / tenant recharge。
  - 计费模式以 `cost_report_only` 为主。
  - 管理员能力通过 admin 侧完成，不要强行引入 business 端的租户自助假设。

### 2) `team`

- 定位：2-10 人小团队，自托管优先，尽量轻依赖。
- 运行形态：`app + postgres` 优先。
- 依赖边界：避免把 Redis / ClickHouse 这类重依赖当基础必需项。
- 功能边界：
  - 支持多租户与 tenant portal。
  - 不支持 tenant self-service / tenant recharge。
  - 计费默认仍以 `cost_report_only` 为主。

### 3) `business`

- 定位：高并发、上万级用户、可水平扩展与分布式部署。
- 运行形态：多服务栈，可拆分 control-plane / data-plane / usage pipeline / frontend。
- 依赖边界：可使用 PostgreSQL、PgBouncer、Redis、ClickHouse 等完整生产依赖。
- 功能边界：保留全功能，多租户、租户门户、充值、完整计费、高可用能力都默认在这一层成立。

### 4) Edition 开发铁则

- 新功能默认先判断：支持 `personal / team / business` 的哪几层。
- 不允许只在 UI 隐藏入口，而不处理路由/API/capability gating。
- 不允许把 `business` 依赖反向泄漏到 `personal / team`。
- 凡是改动 edition 行为，至少同步检查：
  - `README.md`
  - `docs/editions-and-migration.md`
  - 相关 capability tests

## 当前后端核心域（必读）

### 1) 统一账号池是正式主模型

- 当前账号池不要再依赖旧的 `enabled / effective_enabled / pool_state / vault_status` 拼接语义。
- 前后端统一应以 `AccountPoolRecord` / `account-pool` 相关契约为准。
- 当前运营主状态是四态：
  - `inventory`
  - `routable`
  - `cooling`
  - `pending_delete`
- 当前账号池主动作是：
  - `reprobe`
  - `restore`
  - `delete`
- 后续不要再把“禁用”“隔离”“待删除”“当前不可路由”混成一个模糊状态词。

### 2) 账号健康循环已经是正式机制

- `active patrol`
- `rate-limit refresh`
- `pending delete`
- `inventory admission`

这些都已经是正式后台作业，不是临时脚本。

- 后续开发必须保留：
  - 状态变化
  - 原因分类
  - 事件记录
  - 对前端可见的统一读模型
- 当前 active patrol 已覆盖：
  - `oauth_refresh_token`
  - 可用的 `legacy_bearer + codex_oauth`
- 当前路由优先级已经是：
  - `recent_success`
  - `fresh_probe`
  - 普通轮询回退

### 3) 统一事件流已经是主排障面

- 当前统一事件流不是附属日志，而是正式主能力。
- 当前事件分类固定为：
  - `request`
  - `account_pool`
  - `patrol`
  - `import`
  - `infra`
  - `admin_action`
- 当前前端 `Logs` 页已经围绕统一事件流工作，而不是只看 stdout/tmux。
- 后端新增能力如果涉及状态变化、路由决策、基础设施异常、后台批处理，默认都应考虑是否要进入统一事件流。

### 4) Responses 兼容层是高风险核心链路

- `/v1/responses`
- `/v1/responses/compact`
- `/v1/responses/{id}`
- `/backend-api/codex/responses`
- `ws_http_fallback`
- `previous_response_id`
- request adaptation / compact adaptation

这些都属于 data-plane 的核心职责，不是旁支。

- 后续改动这条链时，必须优先补或更新回归测试。
- 不要依赖“手工点点看好像能用”来判断 Responses 兼容是否安全。
- 当前项目**不承诺**“重启后自动恢复进行中的 Codex WS 会话”；围绕自动 continuation 的尝试已经回退。若后续要重做，必须重新设计契约，不要在现有回退状态上继续叠补丁。

### 5) 代理与上游接入也是正式能力

- 出站代理池、系统代理支持、`socks5h`、代理节点测试都已经是正式能力。
- 后续修改代理选择、显式代理/系统代理、TLS/root store、WebSocket fallback 时，必须同步考虑：
  - 请求链是否仍能产生日志与统一事件
  - `Personal` 单实例路径是否还能工作

## 当前前端工作台基线（必读）

### 1) 主工作流

- `Dashboard`：总览与分流，不是 KPI 模板页。
- `Accounts`：统一账号池工作台，是账号运营主入口。
- `Logs`：统一事件工作台，是排障主入口。
- `Imports`：导入批次与结果审计。
- `OAuth Import`：仍是有效入口，不要擅自删除或降级。

### 2) 已废弃或已弱化的能力

- `OAuth Probe`：已删除，禁止恢复。
- `Inventory`：仅作为 `/accounts` 的兼容入口，不再按独立主流程设计。

### 3) 前端开发铁则

- capability gating 不能只隐藏菜单；必须同时处理：
  - 路由
  - shell 分流
  - redirect
  - 后端能力挂载
- 所有面向用户的状态、枚举、错误都必须通过 i18n 和映射函数呈现，不要直接显示技术 code。
- 新后端能力如果已经成为正式工作流，前端必须评估是否需要在：
  - `Dashboard`
  - `Accounts`
  - `Logs`
  - `Imports`
  - `System`
  中体现，而不是只做 API 不做工作台。

## Git 提交规范（必需）

所有提交都必须遵循以下格式：

```text
action(scope): title
description
```

提交时必须使用如下命令形式：

```bash
git commit -m "action(scope): title" -m "description"
```

规则：
- `action` 使用动词型类别，例如：`feat`、`fix`、`refactor`、`test`、`docs`、`chore`、`perf`、`build`、`ci`。
- `scope` 为必填，应为明确模块，例如：`data-plane`、`control-plane`、`frontend`、`core`、`repo`。
- `title` 要简洁、使用祈使语气，并描述主要变更。
- `description` 必须是一句简短说明，用于交代上下文或意图。
- 不允许空的 commit body。

## i18n 与错误契约（必需）

### 1) 通用规则

- 面向用户的文案严禁在 UI 组件中硬编码，必须统一使用 i18n key。
- 不要把技术值直接展示给用户，例如：
  - `active`
  - `tenant_user`
  - `invalid_record`
  - 原始错误 code
- 所有枚举/状态/错误都必须经过 `code -> 本地化标签` 的映射函数。
- 默认分支不得直接回落原始 code，应返回如 `*.unknown` 的本地化兜底 key。

### 2) 语言包约束

- `zh-CN.ts` 是默认基准语言包。
- `en.ts` 必须和 `zh-CN.ts` 同步维护。
- 默认完成定义只要求：
  - `zh-CN.ts`
  - `en.ts`
- `zh-TW.ts`、`ja.ts`、`ru.ts` 当前不再作为默认阻塞项，后续按需要补齐。

### 3) 后端错误契约

- 后端对外错误必须使用稳定信封结构：

```json
{ "error": { "code": "...", "message": "..." } }
```

- `error.code` 必须稳定、可枚举。
- `error.message` 必须是可控短文本。
- 不要把上游原始错误体、完整错误消息、`err.to_string()` 直接透传给用户。

### 4) 前端/后端完成前检查

前端涉及 i18n、工作台、状态显示、错误展示变更后，执行：

```bash
cd frontend
npm run i18n:check
npm run i18n:hardcode -- --no-baseline
npm run i18n:runtime-check
npm run lint
npm run build
```

后端涉及契约、错误、账号池、事件流、Responses、代理等改动后，至少执行：

```bash
cargo check -p control-plane
cargo check -p data-plane
```

如改动 edition 或单二进制构建路径，还应补做对应 edition 的 `cargo check --features ...`。

## 请求关联与排障（必需）

- 每个 API 响应都应带 `x-request-id`；若客户端已提供则复用，否则生成。
- 排查失败问题时，必须从 `x-request-id` 开始。
- 深度排障优先使用：
  - `GET /api/v1/admin/request-correlation/{request_id}`
  - `GET /api/v1/admin/event-stream`
  - `GET /api/v1/admin/event-stream/correlation/{request_id}`
- 不要让用户直接面对上游原始错误；需要深挖时用 `request_id` 去关联：
  - request logs
  - unified event stream
  - audit logs

## 当前开发运行规则（必读）

### 1) 实例启动

- 不再依赖任何 `run*` / `restart*` 脚本启动实例。
- 根据当前目标 edition，直接运行对应二进制或直接使用 `cargo run` / `cargo build` 启动目标服务。
- `personal` 是单实例单二进制形态；`team / business` 才需要按具体服务拆分启动。

### 2) 前端开发

- embedded frontend 只用于随实例提供管理界面，不应当作新的前端工作台热更新开发入口。
- 如果要开发新的前端工作台体验，应在 `frontend` 目录或专门 worktree 中单独启动 Vite。

### 3) 调试与抓包

- 遇到上游兼容问题，先查：
  - `.codex/codex` 参考实现源码
  - 当前 data-plane 请求改写逻辑
  - 统一事件流与 request correlation
- 抓包默认可使用 `mitmdump`。
- 但不要只靠抓包；必须结合 `request_id`、event stream、request logs 一起判断。

## Plan Mode 铁则

- Plan 计划确认后，在开始执行开发前，必须将完整 Plan 原文保存到：
  - `.codex/docs/plans/{datetime}-{scope}-{feature}-plan.md`
- Plan 文档必须包含可执行 todo list。
- 开发完成后必须回填并勾选对应项。
- 每次会话压缩或新开会话，在接收相关任务后，必须先到 `.codex/docs/plans` 下找到对应 Plan 文件。

## 参考仓库与外部事实源

- 本项目本质上是在代理/模拟 `.codex/codex` 发往上游的方式，并把它转成我们想要的 API 形态。
- 遇到上游兼容、Responses、WS fallback、compact 之类问题时，应先去查看 `.codex/codex` 的源代码。
- 抓包可使用 `mitmdump`，但优先与：
  - `request_id`
  - unified event stream
  - request logs
  交叉验证。
