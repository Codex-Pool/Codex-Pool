# Codex-Pool

<p align="center">
  <img src="./assets/logo.png" alt="Codex-Pool Logo" width="200" />
</p>

<p align="center">
  <strong>面向多租户与可观测场景的 Codex/OpenAI 兼容代理系统</strong><br/>
  Rust 双平面架构（Data Plane + Control Plane）+ React 管理台 + Docker 生产编排
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.93%2B-000000?logo=rust" alt="Rust" />
  <img src="https://img.shields.io/badge/Frontend-React%20%2B%20Vite-61dafb?logo=react" alt="Frontend" />
  <img src="https://img.shields.io/badge/License-Apache--2.0-blue" alt="License" />
  <img src="https://img.shields.io/badge/CI-GitHub%20Actions-2088ff?logo=githubactions" alt="CI" />
</p>

---

## 为什么是 Codex-Pool

Codex-Pool 用于把多个上游账号池化成一个统一入口，对外提供 **OpenAI/Codex 兼容接口**，并同时具备：

- 多租户隔离与 API Key 管理
- 账号路由、故障切换、快照热更新
- 请求日志事件流 + 用量聚合（Redis Stream + ClickHouse）
- 管理台（Admin Portal）与租户门户（Tenant Portal）
- 可生产部署的 Docker Compose 拆分

当前仓库同时维护三档交付形态：

- `personal`：单二进制/单容器、SQLite、无 Redis/ClickHouse/独立 frontend
- `team`：`app + postgres`，适合 2-10 人小团队
- `business`：全功能多服务栈，默认使用 PostgreSQL + PgBouncer + Redis + ClickHouse

---

## 目录

- [架构总览](#架构总览)
- [5 分钟快速开始](#5-分钟快速开始)
- [生产部署](#生产部署)
- [版本与迁移](#版本与迁移)
- [Docker 常见问题](#docker-常见问题)
- [安全基线](#安全基线)
- [CI/CD](#cicd)
- [API 兼容面](#api-兼容面)
- [开发者指南（后置）](#开发者指南后置)
- [项目结构](#项目结构)

---

## 架构总览

```mermaid
flowchart LR
    User[API Client / SDK] --> DP[Data Plane]
    Admin[Admin/Tenant UI] --> CP[Control Plane]
    UI[Frontend SPA] --> CP

    CP --> PG[(PostgreSQL)]
    CP --> CH[(ClickHouse)]
    DP --> CP
    DP --> Redis[(Redis Streams)]
    Worker[Usage Worker] --> Redis
    Worker --> CH
```

### 组件职责

| 组件 | 职责 |
| --- | --- |
| `data-plane` | 兼容 `/v1/responses`、`/v1/chat/completions` 等接口；路由到上游账号；请求级 failover；可选内部调试路由 |
| `control-plane` | 管理 API、租户与 API Key、上游账号管理、策略管理、快照发布、报表查询 |
| `usage-worker` | 消费 Redis Stream 请求日志，按小时聚合写入 ClickHouse |
| `frontend` | Admin + Tenant 双门户，统一通过 `/api/v1` 访问 control-plane |

补充约定：

- `x-request-id` 用于请求链路追踪与日志关联，不作为计费幂等键。
- data-plane 会为每个可计费 logical request 生成内部 billing key；control-plane 只对该内部键做授权/捕获/释放幂等。
- `previous_response_id`、`session_id`、`x-codex-turn-state` 只参与 continuation / sticky routing，不直接承担账务幂等语义。
- `Responses API` 在 Codex profile 下的兼容范围与已知限制，见 [docs/responses-api-compatibility-matrix.md](docs/responses-api-compatibility-matrix.md)。

---

## 5 分钟快速开始

### 1) 启动开发环境

```bash
docker compose -f docker-compose.dev.yml up -d --build
```

### 2) 检查服务健康

```bash
curl -fsS http://127.0.0.1:8090/health
curl -fsS http://127.0.0.1:8091/health
```

### 3) 打开前端

- 管理台：`http://127.0.0.1:5173`
- 默认管理员（仅开发）：`admin / admin123456`

### 4) 运行一键冒烟（可选）

```bash
./scripts/smoke_dashboard_logs_billing.sh
```

---

## 生产部署

### 方案 A：`personal` 单容器部署（SQLite）

`personal` 版会把 admin UI、control-plane API、`/v1/*` 代理统一收进一个容器，并把状态保存在 SQLite。

1. 复制示例环境变量：

```bash
cp docker/.env.personal.example docker/.env.personal
```

2. 启动：

```bash
docker compose --env-file docker/.env.personal -f docker-compose.personal.yml up -d --build
```

3. 访问：

- 统一入口：`http://127.0.0.1:${PERSONAL_APP_PORT:-8090}`
- 运行模式：`CODEX_POOL_EDITION=personal`
- 默认不启动 PostgreSQL、Redis、ClickHouse、独立 frontend 容器

### 方案 B：`business` 全功能部署（多服务）

`business` 版对应当前仓库里的完整生产编排，使用：

- `docker-compose.yml`：全功能运行栈
- `docker-compose.build.yml`：本机构建镜像 overlay

1. 先准备业务版变量：

```bash
cp docker/.env.business.example docker/.env.business
```

必须替换的关键变量：

- `POSTGRES_PASSWORD`
- `CONTROL_PLANE_INTERNAL_AUTH_TOKEN`
- `CONTROL_PLANE_API_KEY_HMAC_KEYS`
- `CREDENTIALS_ENCRYPTION_KEY`
- `ADMIN_PASSWORD`
- `ADMIN_JWT_SECRET`

> 建议用 `openssl rand -base64 32` 生成高强度密钥。

#### 单机构建并部署（不依赖镜像仓库）

```bash
docker compose \
  --env-file docker/.env.business \
  -f docker-compose.yml \
  -f docker-compose.build.yml \
  up -d --build
```

这个方案**不需要**先把镜像上传到 Docker Hub/GHCR。

#### 多机/集群部署（镜像先推仓库）

1. 在 CI 通过 tag 自动发布镜像（已提供 `docker-publish.yml`，默认发布到 GHCR）。
2. 在部署机设置镜像地址：
   - `CONTROL_PLANE_IMAGE`
   - `DATA_PLANE_IMAGE`
   - `USAGE_WORKER_IMAGE`
   - `FRONTEND_IMAGE`
3. 启动：

```bash
docker compose --env-file docker/.env.business -f docker-compose.yml up -d
```

### 方案 C：`team` 版最小 Docker 部署（`app + postgres`）

`team` 版现在支持单容器承载 admin UI、tenant UI、control-plane API 与 `/v1/*` 代理，只需要再配一个 PostgreSQL。

1. 复制示例环境变量：

```bash
cp docker/.env.team.example docker/.env.team
```

2. 启动：

```bash
docker compose --env-file docker/.env.team -f docker-compose.team.yml up -d --build
```

3. 访问：

- 管理端与租户端统一入口：`http://127.0.0.1:${TEAM_APP_PORT:-8090}`
- 运行模式：`CODEX_POOL_EDITION=team`
- 默认不启动 Redis、ClickHouse、独立 frontend 容器

### 版本交付矩阵

| 版本 | 启动文件 | 默认依赖 |
| --- | --- | --- |
| `personal` | `docker-compose.personal.yml` | 单个 `app` 容器 + SQLite volume |
| `team` | `docker-compose.team.yml` | `app + postgres` |
| `business` | `docker-compose.yml` | `control-plane + data-plane + usage-worker + frontend + postgres + pgbouncer + redis + clickhouse` |

### Cargo 后端家族构建矩阵

Cargo feature 现在只表达 backend family，不表达 edition。本仓库的推荐构建命令如下：

| 交付形态 | control-plane / worker 构建命令 |
| --- | --- |
| `personal` | `cargo build -p control-plane --no-default-features --features sqlite-backend --bin codex-pool-personal` |
| `team` | `cargo build -p control-plane --no-default-features --features postgres-backend --bin codex-pool-team` |
| `business` | `cargo build -p control-plane --no-default-features --features postgres-backend,redis-backend,clickhouse-backend,smtp-backend --bin codex-pool-business --bin usage-worker --bin edition-migrate` |

补充说明：

- `data-plane` 的默认轻依赖路径使用 `cargo check -p data-plane --no-default-features`。
- 只有 `business` 路径才需要 `cargo check -p data-plane --no-default-features --features redis-backend`。
- `edition-migrate` 依赖 PostgreSQL family，因此至少需要 `postgres-backend`。

## 版本与迁移

- 运维指南：[`docs/editions-and-migration.md`](./docs/editions-and-migration.md)
- 覆盖内容：
  - `personal / team / business` 部署矩阵
  - 产品名二进制：`codex-pool-personal / codex-pool-team / codex-pool-business`
  - `edition-migrate export / preflight / import / archive inspect / shrink` 命令
  - 升级路径：`personal -> team/business`、`team -> business`
  - 降级路径：`business -> team`、`team/business -> personal`
  - `business` 的横向扩容建议

---

## Docker 常见问题

### 我必须上传 Docker Hub 吗？

不是必须。

- 单机部署：用 `docker-compose.build.yml` 本机构建，直接起服务。
- 多机部署：需要把镜像推到“某个可访问仓库”，可以是 Docker Hub、GHCR、私有 Harbor/ECR/GCR。

### 生产编排现在有什么修复？

当前仓库已补齐以下关键点：

- `docker-compose.yml` 为 `control-plane/data-plane/usage-worker` 显式设置 `command`
- control-plane 补齐关键必填环境变量（API Key HMAC key ring、凭据加密密钥等）
- 新增 `docker/frontend.runtime.Dockerfile`（构建静态资源 + Nginx）
- 新增 `docker/nginx.frontend.conf`，前端容器内反向代理 `/api` 到 control-plane
- 新增 `docker-compose.build.yml`，支持“本机构建生产镜像”

---

## 安全基线

### 已落实的仓库安全策略

- `config.toml` 已加入 `.gitignore`，防止本地配置误提交
- `.env.*`（除明确示例文件）默认忽略
- CI 集成 `gitleaks` 做 secrets 扫描

### 发布前强烈建议

1. 轮换所有历史暴露过的 token/refresh token/JWT secret。
2. 在首次公开仓库前，执行一次完整 secret 扫描（含 git history）。
3. 生产环境关闭 `ENABLE_INTERNAL_DEBUG_ROUTES`。
4. 生产环境显式开启 `ADMIN_SESSION_COOKIE_SECURE=true`；启用租户门户时再同时开启 `TENANT_SESSION_COOKIE_SECURE=true`。
5. 将配置统一放到 `docker/.env.personal` / `docker/.env.team` / `docker/.env.business` 或密钥管理系统，不在仓库存明文。

---

## CI/CD

### 已新增工作流

- `CI`（`.github/workflows/ci.yml`）
  - `gitleaks` secrets 扫描
  - Rust `check`：`cargo check --workspace --all-targets --locked`
  - Rust `edition/backend matrix check`：
    - `cargo check -p control-plane --no-default-features --features sqlite-backend --bin codex-pool-personal --locked`
    - `cargo check -p control-plane --no-default-features --features postgres-backend --bin codex-pool-team --locked`
    - `cargo check -p control-plane --no-default-features --features postgres-backend,redis-backend,clickhouse-backend,smtp-backend --bin codex-pool-business --bin usage-worker --bin edition-migrate --locked`
  - Rust `fast tests`：`cargo test --workspace --lib --bins --locked`
  - Rust `integration tests`：`cargo test -p <control-plane|data-plane> --test integration --locked`
  - Rust `full clippy`：`cargo clippy --workspace --all-targets --locked`
  - Rust `audit`：`cargo audit`
  - Frontend：`i18n:check` + `i18n:hardcode -- --no-baseline` + `i18n:runtime-check` + `lint` + `build`
  - Docker：构建 Rust runtime 与 Frontend runtime 镜像

- `Docker Publish`（`.github/workflows/docker-publish.yml`）
  - tag（`v*`）或手动触发后发布到 GHCR
  - 发布两个镜像：
    - `ghcr.io/<owner>/codex-pool-rust`
    - `ghcr.io/<owner>/codex-pool-frontend`

---

## API 兼容面

### Data Plane（对外）

- `POST /v1/responses`
- `GET /v1/responses`（WebSocket Upgrade）
- `POST /backend-api/codex/responses`
- `GET /backend-api/codex/responses`（WebSocket Upgrade）
- `POST /v1/chat/completions`
- `GET /v1/models`
- `GET /api/codex/usage`

> WebSocket 兼容约定：  
> - 支持透传 `OpenAI-Beta: responses_websockets=2026-02-04/2026-02-06`。  
> - 当上游握手明确返回 `426 Upgrade Required` 时，Data Plane 会保持 `426` 返回，便于客户端快速回退到 HTTPS 传输。  

### Control Plane（管理）

- 管理员认证、租户管理、API Key 管理、上游账号管理
- 路由/重试策略管理
- 数据平面快照发布
- Usage / Billing / Logs / Dashboard 查询接口

---

## 开发者指南（后置）

### 本地运行（不使用 Docker）

```bash
# control-plane
set -a && source .env.runtime && set +a
cargo run -p control-plane --bin codex-pool-business

# data-plane
set -a && source .env.runtime && set +a
cargo run -p data-plane --bin data-plane

# usage-worker
set -a && source .env.runtime && set +a
cargo run -p control-plane --bin usage-worker
```

```bash
# frontend
cd frontend
npm ci --legacy-peer-deps
npm run dev
```


### 常用质量命令

```bash
# Rust
cargo fmt --all --check
cargo check --workspace --all-targets
cargo test --workspace --lib --bins
cargo test -p control-plane --test integration
cargo test -p data-plane --test integration
cargo clippy --workspace --all-targets --locked

# Frontend
cd frontend
npm run lint
npm run build
```

---

## 项目结构

```text
.
├── crates/
│   └── codex-pool-core/          # 共享模型与 DTO
├── services/
│   ├── control-plane/            # 管理面 API / 策略 / 认证 / 快照
│   └── data-plane/               # 代理面 / 路由 / failover / 鉴权
├── frontend/                     # Admin + Tenant 前端
├── docker/                       # Dockerfile 与 nginx 配置
├── scripts/                      # 冒烟、回填、压测脚本
├── docker-compose.dev.yml        # 开发编排
├── docker-compose.yml            # 生产编排
└── docker-compose.build.yml      # 生产本地构建覆盖
```

---

## License

Apache-2.0
