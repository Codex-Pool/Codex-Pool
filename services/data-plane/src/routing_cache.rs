use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use uuid::Uuid;

#[cfg(feature = "redis-backend")]
use anyhow::Context;

#[cfg(feature = "redis-backend")]
const DEFAULT_LOCAL_STICKY_REHYDRATE_TTL: Duration = Duration::from_secs(30 * 60);
#[cfg(feature = "redis-backend")]
const DEFAULT_LOCAL_UNHEALTHY_REHYDRATE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoutingCacheStats {
    pub local_sticky_hit_total: u64,
    pub local_sticky_miss_total: u64,
    pub shared_sticky_hit_total: u64,
    pub shared_sticky_miss_total: u64,
    pub shared_sticky_error_total: u64,
    pub local_unhealthy_hit_total: u64,
    pub local_unhealthy_miss_total: u64,
    pub shared_unhealthy_hit_total: u64,
    pub shared_unhealthy_miss_total: u64,
    pub shared_unhealthy_error_total: u64,
    pub shared_write_error_total: u64,
}

#[async_trait]
pub trait RoutingCache: Send + Sync {
    async fn get_sticky_account_id(&self, sticky_key: &str) -> anyhow::Result<Option<Uuid>>;
    async fn set_sticky_account_id(
        &self,
        sticky_key: &str,
        account_id: Uuid,
        ttl: Duration,
    ) -> anyhow::Result<()>;
    async fn delete_sticky_account_id(&self, sticky_key: &str) -> anyhow::Result<()>;
    async fn set_unhealthy(&self, account_id: Uuid, ttl: Duration) -> anyhow::Result<()>;
    async fn is_unhealthy(&self, account_id: Uuid) -> anyhow::Result<bool>;
    async fn clear_unhealthy(&self, account_id: Uuid) -> anyhow::Result<()>;

    fn stats_snapshot(&self) -> RoutingCacheStats {
        RoutingCacheStats::default()
    }
}

#[derive(Debug, Clone)]
struct StickyEntry {
    account_id: Uuid,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
pub struct InMemoryRoutingCache {
    sticky: Arc<RwLock<HashMap<String, StickyEntry>>>,
    unhealthy: Arc<RwLock<HashMap<Uuid, Instant>>>,
}

impl Default for InMemoryRoutingCache {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRoutingCache {
    pub fn new() -> Self {
        Self {
            sticky: Arc::new(RwLock::new(HashMap::new())),
            unhealthy: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl RoutingCache for InMemoryRoutingCache {
    async fn get_sticky_account_id(&self, sticky_key: &str) -> anyhow::Result<Option<Uuid>> {
        let now = Instant::now();
        let Ok(mut sticky) = self.sticky.write() else {
            return Ok(None);
        };
        sticky.retain(|_, entry| entry.expires_at > now);
        Ok(sticky.get(sticky_key).map(|entry| entry.account_id))
    }

    async fn set_sticky_account_id(
        &self,
        sticky_key: &str,
        account_id: Uuid,
        ttl: Duration,
    ) -> anyhow::Result<()> {
        if ttl.is_zero() {
            return Ok(());
        }
        if let Ok(mut sticky) = self.sticky.write() {
            sticky.insert(
                sticky_key.to_string(),
                StickyEntry {
                    account_id,
                    expires_at: Instant::now() + ttl,
                },
            );
        }
        Ok(())
    }

    async fn delete_sticky_account_id(&self, sticky_key: &str) -> anyhow::Result<()> {
        if let Ok(mut sticky) = self.sticky.write() {
            sticky.remove(sticky_key);
        }
        Ok(())
    }

    async fn set_unhealthy(&self, account_id: Uuid, ttl: Duration) -> anyhow::Result<()> {
        if ttl.is_zero() {
            return Ok(());
        }
        if let Ok(mut unhealthy) = self.unhealthy.write() {
            unhealthy.insert(account_id, Instant::now() + ttl);
        }
        Ok(())
    }

    async fn is_unhealthy(&self, account_id: Uuid) -> anyhow::Result<bool> {
        let now = Instant::now();
        let Ok(mut unhealthy) = self.unhealthy.write() else {
            return Ok(false);
        };
        unhealthy.retain(|_, until| *until > now);
        Ok(unhealthy.get(&account_id).is_some())
    }

    async fn clear_unhealthy(&self, account_id: Uuid) -> anyhow::Result<()> {
        if let Ok(mut unhealthy) = self.unhealthy.write() {
            unhealthy.remove(&account_id);
        }
        Ok(())
    }
}

#[cfg(feature = "redis-backend")]
#[derive(Debug, Clone)]
pub struct RedisRoutingCache {
    redis_url: String,
    key_prefix: String,
}

#[cfg(feature = "redis-backend")]
impl RedisRoutingCache {
    pub fn new(redis_url: impl Into<String>, key_prefix: impl Into<String>) -> Self {
        Self {
            redis_url: redis_url.into(),
            key_prefix: key_prefix.into(),
        }
    }

    fn sticky_key(&self, sticky_key: &str) -> String {
        format!("{}:sticky:{sticky_key}", self.key_prefix)
    }

    fn unhealthy_key(&self, account_id: Uuid) -> String {
        format!("{}:unhealthy:{account_id}", self.key_prefix)
    }

    async fn connection(&self) -> anyhow::Result<redis::aio::MultiplexedConnection> {
        let client = redis::Client::open(self.redis_url.as_str())
            .with_context(|| "failed to open redis client for routing cache")?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .with_context(|| "failed to connect redis for routing cache")?;
        Ok(conn)
    }
}

#[cfg(feature = "redis-backend")]
#[async_trait]
impl RoutingCache for RedisRoutingCache {
    async fn get_sticky_account_id(&self, sticky_key: &str) -> anyhow::Result<Option<Uuid>> {
        let mut conn = self.connection().await?;
        let raw = redis::cmd("GET")
            .arg(self.sticky_key(sticky_key))
            .query_async::<Option<String>>(&mut conn)
            .await
            .with_context(|| "failed to load sticky mapping from redis")?;
        raw.map(|value| Uuid::parse_str(&value).with_context(|| "invalid sticky uuid in redis"))
            .transpose()
    }

    async fn set_sticky_account_id(
        &self,
        sticky_key: &str,
        account_id: Uuid,
        ttl: Duration,
    ) -> anyhow::Result<()> {
        if ttl.is_zero() {
            return Ok(());
        }
        let mut conn = self.connection().await?;
        let _: String = redis::cmd("SET")
            .arg(self.sticky_key(sticky_key))
            .arg(account_id.to_string())
            .arg("EX")
            .arg(ttl.as_secs().max(1))
            .query_async(&mut conn)
            .await
            .with_context(|| "failed to store sticky mapping in redis")?;
        Ok(())
    }

    async fn delete_sticky_account_id(&self, sticky_key: &str) -> anyhow::Result<()> {
        let mut conn = self.connection().await?;
        let _: i64 = redis::cmd("DEL")
            .arg(self.sticky_key(sticky_key))
            .query_async(&mut conn)
            .await
            .with_context(|| "failed to delete sticky mapping in redis")?;
        Ok(())
    }

    async fn set_unhealthy(&self, account_id: Uuid, ttl: Duration) -> anyhow::Result<()> {
        if ttl.is_zero() {
            return Ok(());
        }
        let mut conn = self.connection().await?;
        let _: String = redis::cmd("SET")
            .arg(self.unhealthy_key(account_id))
            .arg("1")
            .arg("EX")
            .arg(ttl.as_secs().max(1))
            .query_async(&mut conn)
            .await
            .with_context(|| "failed to set unhealthy key in redis")?;
        Ok(())
    }

    async fn is_unhealthy(&self, account_id: Uuid) -> anyhow::Result<bool> {
        let mut conn = self.connection().await?;
        let exists: i64 = redis::cmd("EXISTS")
            .arg(self.unhealthy_key(account_id))
            .query_async(&mut conn)
            .await
            .with_context(|| "failed to check unhealthy key in redis")?;
        Ok(exists > 0)
    }

    async fn clear_unhealthy(&self, account_id: Uuid) -> anyhow::Result<()> {
        let mut conn = self.connection().await?;
        let _: i64 = redis::cmd("DEL")
            .arg(self.unhealthy_key(account_id))
            .query_async(&mut conn)
            .await
            .with_context(|| "failed to clear unhealthy key in redis")?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct HybridRoutingCache {
    local: Arc<InMemoryRoutingCache>,
    #[cfg(feature = "redis-backend")]
    shared: Option<Arc<RedisRoutingCache>>,
    local_sticky_hit_total: Arc<AtomicU64>,
    local_sticky_miss_total: Arc<AtomicU64>,
    shared_sticky_hit_total: Arc<AtomicU64>,
    shared_sticky_miss_total: Arc<AtomicU64>,
    shared_sticky_error_total: Arc<AtomicU64>,
    local_unhealthy_hit_total: Arc<AtomicU64>,
    local_unhealthy_miss_total: Arc<AtomicU64>,
    shared_unhealthy_hit_total: Arc<AtomicU64>,
    shared_unhealthy_miss_total: Arc<AtomicU64>,
    shared_unhealthy_error_total: Arc<AtomicU64>,
    shared_write_error_total: Arc<AtomicU64>,
}

impl HybridRoutingCache {
    pub fn local_only(local: Arc<InMemoryRoutingCache>) -> Self {
        Self {
            local,
            #[cfg(feature = "redis-backend")]
            shared: None,
            local_sticky_hit_total: Arc::new(AtomicU64::new(0)),
            local_sticky_miss_total: Arc::new(AtomicU64::new(0)),
            shared_sticky_hit_total: Arc::new(AtomicU64::new(0)),
            shared_sticky_miss_total: Arc::new(AtomicU64::new(0)),
            shared_sticky_error_total: Arc::new(AtomicU64::new(0)),
            local_unhealthy_hit_total: Arc::new(AtomicU64::new(0)),
            local_unhealthy_miss_total: Arc::new(AtomicU64::new(0)),
            shared_unhealthy_hit_total: Arc::new(AtomicU64::new(0)),
            shared_unhealthy_miss_total: Arc::new(AtomicU64::new(0)),
            shared_unhealthy_error_total: Arc::new(AtomicU64::new(0)),
            shared_write_error_total: Arc::new(AtomicU64::new(0)),
        }
    }

    #[cfg(feature = "redis-backend")]
    pub fn with_shared(local: Arc<InMemoryRoutingCache>, shared: Arc<RedisRoutingCache>) -> Self {
        Self {
            local,
            shared: Some(shared),
            local_sticky_hit_total: Arc::new(AtomicU64::new(0)),
            local_sticky_miss_total: Arc::new(AtomicU64::new(0)),
            shared_sticky_hit_total: Arc::new(AtomicU64::new(0)),
            shared_sticky_miss_total: Arc::new(AtomicU64::new(0)),
            shared_sticky_error_total: Arc::new(AtomicU64::new(0)),
            local_unhealthy_hit_total: Arc::new(AtomicU64::new(0)),
            local_unhealthy_miss_total: Arc::new(AtomicU64::new(0)),
            shared_unhealthy_hit_total: Arc::new(AtomicU64::new(0)),
            shared_unhealthy_miss_total: Arc::new(AtomicU64::new(0)),
            shared_unhealthy_error_total: Arc::new(AtomicU64::new(0)),
            shared_write_error_total: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[async_trait]
impl RoutingCache for HybridRoutingCache {
    async fn get_sticky_account_id(&self, sticky_key: &str) -> anyhow::Result<Option<Uuid>> {
        if let Some(account_id) = self.local.get_sticky_account_id(sticky_key).await? {
            self.local_sticky_hit_total.fetch_add(1, Ordering::Relaxed);
            return Ok(Some(account_id));
        }
        self.local_sticky_miss_total.fetch_add(1, Ordering::Relaxed);
        #[cfg(not(feature = "redis-backend"))]
        {
            return Ok(None);
        }
        #[cfg(feature = "redis-backend")]
        {
            let Some(shared) = self.shared.as_ref() else {
                return Ok(None);
            };
            let shared_hit = shared.get_sticky_account_id(sticky_key).await;
            match shared_hit {
                Ok(Some(account_id)) => {
                    self.shared_sticky_hit_total.fetch_add(1, Ordering::Relaxed);
                    let _ = self
                        .local
                        .set_sticky_account_id(
                            sticky_key,
                            account_id,
                            DEFAULT_LOCAL_STICKY_REHYDRATE_TTL,
                        )
                        .await;
                    Ok(Some(account_id))
                }
                Ok(None) => {
                    self.shared_sticky_miss_total
                        .fetch_add(1, Ordering::Relaxed);
                    Ok(None)
                }
                Err(_) => {
                    self.shared_sticky_error_total
                        .fetch_add(1, Ordering::Relaxed);
                    Ok(None)
                }
            }
        }
    }

    async fn set_sticky_account_id(
        &self,
        sticky_key: &str,
        account_id: Uuid,
        ttl: Duration,
    ) -> anyhow::Result<()> {
        self.local
            .set_sticky_account_id(sticky_key, account_id, ttl)
            .await?;
        #[cfg(feature = "redis-backend")]
        if let Some(shared) = self.shared.as_ref() {
            if shared
                .set_sticky_account_id(sticky_key, account_id, ttl)
                .await
                .is_err()
            {
                self.shared_write_error_total
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    async fn delete_sticky_account_id(&self, sticky_key: &str) -> anyhow::Result<()> {
        self.local.delete_sticky_account_id(sticky_key).await?;
        #[cfg(feature = "redis-backend")]
        if let Some(shared) = self.shared.as_ref() {
            if shared.delete_sticky_account_id(sticky_key).await.is_err() {
                self.shared_write_error_total
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    async fn set_unhealthy(&self, account_id: Uuid, ttl: Duration) -> anyhow::Result<()> {
        self.local.set_unhealthy(account_id, ttl).await?;
        #[cfg(feature = "redis-backend")]
        if let Some(shared) = self.shared.as_ref() {
            if shared.set_unhealthy(account_id, ttl).await.is_err() {
                self.shared_write_error_total
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    async fn is_unhealthy(&self, account_id: Uuid) -> anyhow::Result<bool> {
        if self.local.is_unhealthy(account_id).await? {
            self.local_unhealthy_hit_total
                .fetch_add(1, Ordering::Relaxed);
            return Ok(true);
        }
        self.local_unhealthy_miss_total
            .fetch_add(1, Ordering::Relaxed);
        #[cfg(not(feature = "redis-backend"))]
        {
            return Ok(false);
        }
        #[cfg(feature = "redis-backend")]
        {
            let Some(shared) = self.shared.as_ref() else {
                return Ok(false);
            };
            let shared_hit = shared.is_unhealthy(account_id).await;
            match shared_hit {
                Ok(true) => {
                    self.shared_unhealthy_hit_total
                        .fetch_add(1, Ordering::Relaxed);
                    let _ = self
                        .local
                        .set_unhealthy(account_id, DEFAULT_LOCAL_UNHEALTHY_REHYDRATE_TTL)
                        .await;
                    Ok(true)
                }
                Ok(false) => {
                    self.shared_unhealthy_miss_total
                        .fetch_add(1, Ordering::Relaxed);
                    Ok(false)
                }
                Err(_) => {
                    self.shared_unhealthy_error_total
                        .fetch_add(1, Ordering::Relaxed);
                    Ok(false)
                }
            }
        }
    }

    async fn clear_unhealthy(&self, account_id: Uuid) -> anyhow::Result<()> {
        self.local.clear_unhealthy(account_id).await?;
        #[cfg(feature = "redis-backend")]
        if let Some(shared) = self.shared.as_ref() {
            if shared.clear_unhealthy(account_id).await.is_err() {
                self.shared_write_error_total
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    fn stats_snapshot(&self) -> RoutingCacheStats {
        RoutingCacheStats {
            local_sticky_hit_total: self.local_sticky_hit_total.load(Ordering::Relaxed),
            local_sticky_miss_total: self.local_sticky_miss_total.load(Ordering::Relaxed),
            shared_sticky_hit_total: self.shared_sticky_hit_total.load(Ordering::Relaxed),
            shared_sticky_miss_total: self.shared_sticky_miss_total.load(Ordering::Relaxed),
            shared_sticky_error_total: self.shared_sticky_error_total.load(Ordering::Relaxed),
            local_unhealthy_hit_total: self.local_unhealthy_hit_total.load(Ordering::Relaxed),
            local_unhealthy_miss_total: self.local_unhealthy_miss_total.load(Ordering::Relaxed),
            shared_unhealthy_hit_total: self.shared_unhealthy_hit_total.load(Ordering::Relaxed),
            shared_unhealthy_miss_total: self.shared_unhealthy_miss_total.load(Ordering::Relaxed),
            shared_unhealthy_error_total: self.shared_unhealthy_error_total.load(Ordering::Relaxed),
            shared_write_error_total: self.shared_write_error_total.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HybridRoutingCache, InMemoryRoutingCache, RoutingCache};
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    #[tokio::test]
    async fn in_memory_sticky_roundtrip_and_expire() {
        let cache = InMemoryRoutingCache::new();
        let sticky_key = "s1";
        let account_id = Uuid::new_v4();
        cache
            .set_sticky_account_id(sticky_key, account_id, Duration::from_millis(10))
            .await
            .unwrap();
        assert_eq!(
            cache.get_sticky_account_id(sticky_key).await.unwrap(),
            Some(account_id)
        );

        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(cache.get_sticky_account_id(sticky_key).await.unwrap(), None);
    }

    #[tokio::test]
    async fn in_memory_unhealthy_roundtrip_and_expire() {
        let cache = InMemoryRoutingCache::new();
        let account_id = Uuid::new_v4();
        cache
            .set_unhealthy(account_id, Duration::from_millis(10))
            .await
            .unwrap();
        assert!(cache.is_unhealthy(account_id).await.unwrap());
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!cache.is_unhealthy(account_id).await.unwrap());
    }

    #[tokio::test]
    async fn hybrid_local_only_works_without_shared_cache() {
        let local = Arc::new(InMemoryRoutingCache::new());
        let cache = HybridRoutingCache::local_only(local);
        let sticky_key = "s2";
        let account_id = Uuid::new_v4();
        cache
            .set_sticky_account_id(sticky_key, account_id, Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(
            cache.get_sticky_account_id(sticky_key).await.unwrap(),
            Some(account_id)
        );
    }

    #[tokio::test]
    async fn hybrid_local_only_exposes_local_hit_miss_stats() {
        let local = Arc::new(InMemoryRoutingCache::new());
        let cache = HybridRoutingCache::local_only(local);
        let sticky_key = "stats-key";
        let account_id = Uuid::new_v4();

        assert_eq!(cache.get_sticky_account_id(sticky_key).await.unwrap(), None);
        cache
            .set_sticky_account_id(sticky_key, account_id, Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(
            cache.get_sticky_account_id(sticky_key).await.unwrap(),
            Some(account_id)
        );

        let stats = cache.stats_snapshot();
        assert_eq!(stats.local_sticky_miss_total, 1);
        assert_eq!(stats.local_sticky_hit_total, 1);
        assert_eq!(stats.shared_sticky_hit_total, 0);
        assert_eq!(stats.shared_write_error_total, 0);
    }
}
