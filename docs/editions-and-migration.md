# Editions And Migration

本文档整理 `personal`、`team`、`business` 三档版本的部署方式、升级/降级命令，以及 `business` 的扩容建议。

## 版本矩阵

| 版本 | 默认部署文件 | 默认存储/依赖 | 典型规模 |
| --- | --- | --- | --- |
| `personal` | `docker-compose.personal.yml` | SQLite，无 PostgreSQL / Redis / ClickHouse | 个人、自托管单 workspace |
| `team` | `docker-compose.team.yml` | PostgreSQL，无 Redis / ClickHouse | 2-10 人小团队 |
| `business` | `docker-compose.yml` | PostgreSQL + PgBouncer + Redis + ClickHouse | 高并发、多租户、可水平扩容 |

## 产品名二进制入口

`control-plane` crate 现在额外提供三个产品名二进制：

```bash
cargo build --release -p control-plane --bin codex-pool-personal
cargo build --release -p control-plane --bin codex-pool-team
cargo build --release -p control-plane --bin codex-pool-business
```

约定：

- 当 `CODEX_POOL_EDITION` 未显式设置时，这三个二进制会分别默认推断为 `personal`、`team`、`business`。
- 如果你显式设置了 `CODEX_POOL_EDITION`，环境变量优先级更高。
- `codex-pool-personal` 和 `codex-pool-team` 适合单机/单容器直接启动；`business` 仍建议使用 Compose 或编排平台管理多服务。

## edition-migrate 命令

`edition-migrate` 现在支持以下核心动作：

```bash
edition-migrate export --source-edition <personal|team|business> --source-database-url <url> --output <path>
edition-migrate preflight --input <package.json> --target-edition <personal|team|business>
edition-migrate import --input <package.json> --target-edition <personal|team|business> --target-database-url <url>
edition-migrate archive inspect --input <package.json|archive.json>
edition-migrate shrink --input <package.json> --target-edition personal --tenant-id <uuid> --output <path>
```

约定：

- `export` 产出的原始 package 要保留，它本身就是最完整的归档。
- `archive inspect` 用于查看受限降级时不会进入目标运行态的数据摘要和样例。
- `shrink` 用于把多 tenant 的 `team/business` package 收缩成一个 `personal` 兼容包。

## 升级路径

### `personal -> team`

```bash
edition-migrate export \
  --source-edition personal \
  --source-database-url ./codex-pool-personal.sqlite \
  --output /tmp/personal-package.json

edition-migrate preflight \
  --input /tmp/personal-package.json \
  --target-edition team

edition-migrate import \
  --input /tmp/personal-package.json \
  --target-edition team \
  --target-database-url "postgres://postgres:password@team-postgres:5432/codex_pool?statement-cache-capacity=0"
```

### `personal -> business`

流程与 `personal -> team` 一样，只是 `preflight` / `import` 的目标版本改成 `business`。

### `team -> business`

```bash
edition-migrate export \
  --source-edition team \
  --source-database-url "postgres://postgres:password@team-postgres:5432/codex_pool?statement-cache-capacity=0" \
  --output /tmp/team-package.json

edition-migrate preflight \
  --input /tmp/team-package.json \
  --target-edition business

edition-migrate import \
  --input /tmp/team-package.json \
  --target-edition business \
  --target-database-url "postgres://postgres:password@business-postgres:5432/codex_pool?statement-cache-capacity=0"
```

## 降级路径

### `team -> personal`（单 tenant）

如果 package 里只有一个 tenant，可以直接导入：

```bash
edition-migrate export \
  --source-edition team \
  --source-database-url "postgres://postgres:password@team-postgres:5432/codex_pool?statement-cache-capacity=0" \
  --output /tmp/team-package.json

edition-migrate preflight \
  --input /tmp/team-package.json \
  --target-edition personal

edition-migrate import \
  --input /tmp/team-package.json \
  --target-edition personal \
  --target-database-url ./codex-pool-personal.sqlite
```

### `team -> personal`（多 tenant）

先导出完整 package，再挑一个 tenant 收缩：

```bash
edition-migrate shrink \
  --input /tmp/team-package.json \
  --target-edition personal \
  --tenant-id <tenant-uuid> \
  --output /tmp/team-personal-package.json

edition-migrate import \
  --input /tmp/team-personal-package.json \
  --target-edition personal \
  --target-database-url ./codex-pool-personal.sqlite
```

### `business -> team`

`business` 到 `team` 可以直接走 archive-backed 受限降级。credit 账本、授权记录等 business 专属数据会留在 archive 中，不会进入 `team` 运行态。

```bash
edition-migrate export \
  --source-edition business \
  --source-database-url "postgres://postgres:password@business-postgres:5432/codex_pool?statement-cache-capacity=0" \
  --output /tmp/business-package.json

edition-migrate preflight \
  --input /tmp/business-package.json \
  --target-edition team

edition-migrate archive inspect --input /tmp/business-package.json

edition-migrate import \
  --input /tmp/business-package.json \
  --target-edition team \
  --target-database-url "postgres://postgres:password@team-postgres:5432/codex_pool?statement-cache-capacity=0"
```

### `business -> personal`（单 tenant）

如果 `business` package 里只有一个 tenant，可以直接导入 `personal`，credit 数据会留在 archive。

### `business -> personal`（多 tenant）

先保留原始 `business` package，再用 `shrink` 选出要保留的 tenant：

```bash
edition-migrate shrink \
  --input /tmp/business-package.json \
  --target-edition personal \
  --tenant-id <tenant-uuid> \
  --output /tmp/business-personal-package.json

edition-migrate import \
  --input /tmp/business-personal-package.json \
  --target-edition personal \
  --target-database-url ./codex-pool-personal.sqlite
```

## 归档与可恢复性

- 受限降级时不会导入目标版本的数据会放进 `archive`。
- `archive inspect` 默认只输出摘要和样例行，避免大 package 直接刷满终端。
- 如果你需要未来再升级回更高版本，应该保留最原始的 export package，而不是只保留 shrink 后的 personal package。

## `business` 扩容建议

`business` 版当前推荐把应用层做成无状态或弱状态服务：

- `control-plane`：多实例部署，共享 PostgreSQL / PgBouncer / ClickHouse。
- `data-plane`：可水平扩容，多实例共享同一套 `CONTROL_PLANE_INTERNAL_AUTH_TOKEN` 与 Redis Stream。
- `usage-worker`：可按 consumer group 横向扩展，但要统一 `REQUEST_LOG_CONSUMER_GROUP`。
- `frontend`：纯静态资源，可单独放 CDN/Nginx，也可继续随 Compose 运行。

基础设施建议：

- PostgreSQL、Redis、ClickHouse 优先使用托管版或独立集群，不建议把 Compose 直接当最终大规模生产编排。
- PgBouncer 建议保留在 `business` 路径，避免高并发下直连 PostgreSQL。
- 如果部署到 Kubernetes，建议把 `control-plane`、`data-plane`、`usage-worker` 拆成独立 Deployment，并把 secrets 统一放到 Secret 管理系统。
