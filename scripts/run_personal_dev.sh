#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNTIME_ENV_FILE="${PERSONAL_DEV_ENV_FILE:-${REPO_ROOT}/.env.runtime}"
PERSONAL_DATA_DIR="${REPO_ROOT}/.codex/data/personal"
PERSONAL_DB_PATH="${PERSONAL_DATA_DIR}/codex-pool-personal.sqlite"
PERSONAL_RESPONSES_CURSOR_PATH="${PERSONAL_DATA_DIR}/responses-continuation-cursors.json"

incoming_admin_username="${ADMIN_USERNAME-}"
incoming_admin_password="${ADMIN_PASSWORD-}"
incoming_rust_log="${RUST_LOG-}"
declare -a preserved_env_names=()
declare -a preserved_env_values=()

for env_name in $(compgen -e); do
  case "$env_name" in
    ADMIN_*|RUST_LOG|CONTROL_PLANE_*|DATA_PLANE_*|USAGE_*|OPENAI_*|OUTBOUND_PROXY_*|TENANT_*|API_*|MODEL_*)
      preserved_env_names+=("$env_name")
      preserved_env_values+=("${!env_name}")
      ;;
  esac
done

if [[ ! -f "$RUNTIME_ENV_FILE" ]]; then
  echo "[run_personal_dev] missing runtime env file: $RUNTIME_ENV_FILE" >&2
  exit 1
fi

mkdir -p "$PERSONAL_DATA_DIR"

set -a
source "$RUNTIME_ENV_FILE"
set +a

for idx in "${!preserved_env_names[@]}"; do
  export "${preserved_env_names[$idx]}=${preserved_env_values[$idx]}"
done

if [[ -n "$incoming_admin_username" ]]; then
  export ADMIN_USERNAME="$incoming_admin_username"
fi
if [[ -n "$incoming_admin_password" ]]; then
  export ADMIN_PASSWORD="$incoming_admin_password"
fi
if [[ -n "$incoming_rust_log" ]]; then
  export RUST_LOG="$incoming_rust_log"
fi

export CODEX_POOL_EDITION=personal
export CONTROL_PLANE_DATABASE_URL="sqlite://${PERSONAL_DB_PATH}?mode=rwc"
export DATA_PLANE_RESPONSES_CURSOR_STORE_PATH="${DATA_PLANE_RESPONSES_CURSOR_STORE_PATH:-$PERSONAL_RESPONSES_CURSOR_PATH}"
export RUST_LOG="${RUST_LOG:-debug}"

cd "$REPO_ROOT"

cat <<EOF
[run_personal_dev] repo root: $REPO_ROOT
[run_personal_dev] runtime env: $RUNTIME_ENV_FILE
[run_personal_dev] sqlite path: $PERSONAL_DB_PATH
[run_personal_dev] responses cursor path: ${DATA_PLANE_RESPONSES_CURSOR_STORE_PATH:-unset}
[run_personal_dev] control plane base url: ${CONTROL_PLANE_BASE_URL:-http://127.0.0.1:8090}
[run_personal_dev] admin username: ${ADMIN_USERNAME:-admin}
[run_personal_dev] RUST_LOG=$RUST_LOG
[run_personal_dev] CONTROL_PLANE_ACTIVE_POOL_TARGET=${CONTROL_PLANE_ACTIVE_POOL_TARGET:-unset}
[run_personal_dev] CONTROL_PLANE_ACTIVE_POOL_MIN=${CONTROL_PLANE_ACTIVE_POOL_MIN:-unset}
[run_personal_dev] CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_ENABLED=${CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_ENABLED:-unset}
[run_personal_dev] CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC=${CONTROL_PLANE_RATE_LIMIT_CACHE_REFRESH_INTERVAL_SEC:-unset}
[run_personal_dev] note: codex-pool-personal 会继续服务内置前端；开发新前端请单独在 .worktrees/frontend-antigravity/frontend 中启动 vite
EOF

exec cargo run -p control-plane --no-default-features --features sqlite-backend --bin codex-pool-personal
