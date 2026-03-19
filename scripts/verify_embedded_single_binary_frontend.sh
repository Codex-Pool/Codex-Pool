#!/usr/bin/env bash
set -euo pipefail

IMAGE_TAG="${1:-codex-pool-rust:ci}"
PORT_BASE="${PORT_BASE:-18090}"
TIMEOUT_SEC="${TIMEOUT_SEC:-30}"

run_check() {
  local edition="$1"
  local port="$2"
  local container_name="codex-pool-${edition}-frontend-smoke-$$"
  local root_body

  cleanup() {
    docker rm -f "$container_name" >/dev/null 2>&1 || true
  }
  trap cleanup RETURN

  docker run -d --rm \
    --name "$container_name" \
    -p "127.0.0.1:${port}:8090" \
    -e "CODEX_POOL_EDITION=${edition}" \
    -e "CONTROL_PLANE_LISTEN=0.0.0.0:8090" \
    -e "CONTROL_PLANE_DATABASE_URL=/tmp/${edition}.sqlite" \
    -e "CONTROL_PLANE_INTERNAL_AUTH_TOKEN=test-internal-token" \
    -e "CONTROL_PLANE_API_KEY_HMAC_KEYS=k1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=" \
    -e "CREDENTIALS_ENCRYPTION_KEY=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=" \
    -e "ADMIN_USERNAME=admin" \
    -e "ADMIN_PASSWORD=admin123456" \
    -e "ADMIN_JWT_SECRET=test-admin-jwt-secret" \
    "$IMAGE_TAG" \
    control-plane >/dev/null

  local deadline=$((SECONDS + TIMEOUT_SEC))
  while (( SECONDS < deadline )); do
    if curl -fsS "http://127.0.0.1:${port}/livez" >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done

  curl -fsS "http://127.0.0.1:${port}/livez" >/dev/null
  root_body="$(curl -fsS "http://127.0.0.1:${port}/")"

  if [[ "$root_body" == *"The frontend bundle has not been built yet."* ]]; then
    echo "${edition} embedded frontend is still serving the placeholder page" >&2
    return 1
  fi

  if [[ "$root_body" != *"/assets/"* ]]; then
    echo "${edition} embedded frontend root page does not look like a built SPA shell" >&2
    return 1
  fi
}

run_check "personal" "${PORT_BASE}"
