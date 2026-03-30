use std::sync::{LazyLock, Once};

const TEST_INTERNAL_AUTH_TOKEN: &str = "cp-internal-test-token";

static ENV_LOCK: LazyLock<tokio::sync::Mutex<()>> = LazyLock::new(|| tokio::sync::Mutex::new(()));
static INIT: Once = Once::new();

fn ensure_test_security_env_locked() {
    // Tests must not depend on ambient dev shell env. In particular, snapshot polling is enabled
    // via process-level env vars and can introduce cross-test flakiness when tests run in parallel.
    std::env::remove_var("CONTROL_PLANE_BASE_URL");
    std::env::remove_var("SNAPSHOT_POLL_INTERVAL_MS");
    std::env::remove_var("SNAPSHOT_EVENTS_WAIT_MS");
    std::env::remove_var("SNAPSHOT_EVENTS_LIMIT");
    INIT.call_once(|| {
        std::env::set_var(
            "CONTROL_PLANE_INTERNAL_AUTH_TOKEN",
            TEST_INTERNAL_AUTH_TOKEN,
        );
    });
}

#[allow(dead_code)]
pub async fn ensure_test_security_env() {
    let _guard = ENV_LOCK.lock().await;
    ensure_test_security_env_locked();
}

#[allow(dead_code)]
pub async fn lock_env() -> tokio::sync::MutexGuard<'static, ()> {
    let guard = ENV_LOCK.lock().await;
    ensure_test_security_env_locked();
    guard
}
