use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use codex_pool_core::model::{
    AccountRoutingHealthFreshness, AccountRoutingTraits, CompiledRoutingPlan, UpstreamAccount,
    UpstreamMode,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AccountDiagnostics {
    pub id: Uuid,
    pub label: String,
    pub mode: UpstreamMode,
    pub enabled: bool,
    pub priority: i32,
    pub base_url: String,
    pub chatgpt_account_id: Option<String>,
    pub temporarily_unhealthy: bool,
}

const DEFAULT_STICKY_SESSION_TTL: Duration = Duration::from_secs(30 * 60);
const DEFAULT_STICKY_SESSION_MAX_ENTRIES: usize = 10_000;

#[derive(Debug, Clone, Copy)]
struct StickySessionEntry {
    account_id: Uuid,
    expires_at: Instant,
    last_used_at: Instant,
}

#[derive(Debug, Clone)]
pub struct RoundRobinRouter {
    accounts: Arc<RwLock<Vec<UpstreamAccount>>>,
    account_traits: Arc<RwLock<HashMap<Uuid, AccountRoutingTraits>>>,
    compiled_routing_plan: Arc<RwLock<Option<CompiledRoutingPlan>>>,
    cursor: Arc<AtomicUsize>,
    health: Arc<RwLock<HashMap<Uuid, Instant>>>,
    recent_success: Arc<RwLock<HashMap<Uuid, Instant>>>,
    sticky_sessions: Arc<RwLock<HashMap<String, StickySessionEntry>>>,
    sticky_session_ttl: Duration,
    sticky_session_max_entries: usize,
    sticky_session_total: Arc<AtomicU64>,
    sticky_hit_count: Arc<AtomicU64>,
    sticky_miss_count: Arc<AtomicU64>,
    sticky_rebind_count: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickyRoutingStats {
    pub sticky_session_total: u64,
    pub sticky_hit_count: u64,
    pub sticky_miss_count: u64,
    pub sticky_rebind_count: u64,
    pub sticky_mapping_total: usize,
}

impl RoundRobinRouter {
    pub fn new(accounts: Vec<UpstreamAccount>) -> Self {
        Self::new_with_sticky_limits(
            accounts,
            DEFAULT_STICKY_SESSION_TTL,
            DEFAULT_STICKY_SESSION_MAX_ENTRIES,
        )
    }

    pub fn new_with_sticky_limits(
        accounts: Vec<UpstreamAccount>,
        sticky_session_ttl: Duration,
        sticky_session_max_entries: usize,
    ) -> Self {
        Self {
            accounts: Arc::new(RwLock::new(accounts)),
            account_traits: Arc::new(RwLock::new(HashMap::new())),
            compiled_routing_plan: Arc::new(RwLock::new(None)),
            cursor: Arc::new(AtomicUsize::new(0)),
            health: Arc::new(RwLock::new(HashMap::new())),
            recent_success: Arc::new(RwLock::new(HashMap::new())),
            sticky_sessions: Arc::new(RwLock::new(HashMap::new())),
            sticky_session_ttl: sticky_session_ttl.max(Duration::from_millis(1)),
            sticky_session_max_entries: sticky_session_max_entries.max(1),
            sticky_session_total: Arc::new(AtomicU64::new(0)),
            sticky_hit_count: Arc::new(AtomicU64::new(0)),
            sticky_miss_count: Arc::new(AtomicU64::new(0)),
            sticky_rebind_count: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn pick(&self) -> Option<UpstreamAccount> {
        let excluded = HashSet::new();
        self.pick_excluding(&excluded)
    }

    pub fn pick_excluding(&self, excluded_account_ids: &HashSet<Uuid>) -> Option<UpstreamAccount> {
        let accounts = self.accounts.read().unwrap();

        if accounts.is_empty() {
            return None;
        }

        if let Some(account_id) =
            self.preferred_recent_success_account_id(&accounts, excluded_account_ids)
        {
            if let Some(account) = accounts.iter().find(|account| account.id == account_id) {
                return Some(account.clone());
            }
        }

        if let Some(account_id) =
            self.preferred_recent_probe_account_id(&accounts, excluded_account_ids)
        {
            if let Some(account) = accounts.iter().find(|account| account.id == account_id) {
                return Some(account.clone());
            }
        }

        for _ in 0..accounts.len() {
            let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % accounts.len();
            let account = accounts.get(idx)?;
            if excluded_account_ids.contains(&account.id) {
                continue;
            }
            if account.enabled && self.is_healthy(account.id) {
                return Some(account.clone());
            }
        }

        None
    }

    pub fn pick_with_sticky(&self, sticky_key: Option<&str>) -> Option<UpstreamAccount> {
        let excluded = HashSet::new();
        self.pick_for_model(None, sticky_key, &excluded, false)
    }

    pub fn pick_with_policy(
        &self,
        sticky_key: Option<&str>,
        excluded_account_ids: &HashSet<Uuid>,
        prefer_non_conflicting: bool,
    ) -> Option<UpstreamAccount> {
        self.pick_for_model(
            None,
            sticky_key,
            excluded_account_ids,
            prefer_non_conflicting,
        )
    }

    pub fn pick_for_model(
        &self,
        model: Option<&str>,
        sticky_key: Option<&str>,
        excluded_account_ids: &HashSet<Uuid>,
        prefer_non_conflicting: bool,
    ) -> Option<UpstreamAccount> {
        if let Some(candidate_ids) = self.candidate_account_ids_for_model(model) {
            return self.pick_from_ordered_candidates(
                &candidate_ids,
                sticky_key,
                excluded_account_ids,
                prefer_non_conflicting,
            );
        }
        self.pick_with_round_robin_policy(sticky_key, excluded_account_ids, prefer_non_conflicting)
    }

    pub fn account_matches_model_route(&self, account_id: Uuid, model: Option<&str>) -> bool {
        self.candidate_account_ids_for_model(model)
            .is_none_or(|candidate_ids| candidate_ids.contains(&account_id))
    }

    pub fn replace_compiled_routing_plan(
        &self,
        compiled_routing_plan: Option<CompiledRoutingPlan>,
    ) {
        *self.compiled_routing_plan.write().unwrap() = compiled_routing_plan;
    }

    pub fn compiled_routing_plan(&self) -> Option<CompiledRoutingPlan> {
        self.compiled_routing_plan.read().unwrap().clone()
    }

    fn pick_with_round_robin_policy(
        &self,
        sticky_key: Option<&str>,
        excluded_account_ids: &HashSet<Uuid>,
        prefer_non_conflicting: bool,
    ) -> Option<UpstreamAccount> {
        let Some(sticky_key) = normalize_sticky_key(sticky_key) else {
            return self.pick_excluding(excluded_account_ids);
        };
        self.sticky_session_total.fetch_add(1, Ordering::Relaxed);

        if let Some(sticky_account_id) = self.get_sticky_account_id(&sticky_key) {
            if excluded_account_ids.contains(&sticky_account_id) {
                self.remove_sticky_mapping(&sticky_key);
            } else if let Some(account) = self.pick_specific(sticky_account_id) {
                self.sticky_hit_count.fetch_add(1, Ordering::Relaxed);
                return Some(account);
            } else {
                self.remove_sticky_mapping(&sticky_key);
            }
        }

        self.sticky_miss_count.fetch_add(1, Ordering::Relaxed);
        let account = if prefer_non_conflicting {
            self.pick_avoiding_sticky_conflicts(&sticky_key, excluded_account_ids)
                .or_else(|| self.pick_excluding(excluded_account_ids))
        } else {
            self.pick_excluding(excluded_account_ids)
        }?;
        let rebind = self.insert_sticky_mapping(sticky_key, account.id);
        if rebind {
            self.sticky_rebind_count.fetch_add(1, Ordering::Relaxed);
        }
        Some(account)
    }

    pub fn pick_account(&self, account_id: Uuid) -> Option<UpstreamAccount> {
        self.pick_specific(account_id)
    }

    pub fn get_account_any_state(&self, account_id: Uuid) -> Option<UpstreamAccount> {
        let accounts = self.accounts.read().unwrap();
        accounts.iter().find(|item| item.id == account_id).cloned()
    }

    pub fn sticky_stats(&self) -> StickyRoutingStats {
        StickyRoutingStats {
            sticky_session_total: self.sticky_session_total.load(Ordering::Relaxed),
            sticky_hit_count: self.sticky_hit_count.load(Ordering::Relaxed),
            sticky_miss_count: self.sticky_miss_count.load(Ordering::Relaxed),
            sticky_rebind_count: self.sticky_rebind_count.load(Ordering::Relaxed),
            sticky_mapping_total: self.sticky_mapping_total(),
        }
    }

    pub fn total(&self) -> usize {
        self.accounts.read().unwrap().len()
    }

    pub fn enabled_total(&self) -> usize {
        self.accounts
            .read()
            .unwrap()
            .iter()
            .filter(|a| a.enabled)
            .count()
    }

    pub fn list_account_diagnostics(&self) -> Vec<AccountDiagnostics> {
        let now = Instant::now();
        let accounts = self.accounts.read().unwrap();
        let health = self.health.read().unwrap();

        accounts
            .iter()
            .map(|account| account_diagnostics_for(account, &health, now))
            .collect()
    }

    pub fn account_diagnostics(&self, account_id: Uuid) -> Option<AccountDiagnostics> {
        let now = Instant::now();
        let accounts = self.accounts.read().unwrap();
        let health = self.health.read().unwrap();

        let account = accounts.iter().find(|item| item.id == account_id)?;
        Some(account_diagnostics_for(account, &health, now))
    }

    pub fn replace_accounts(&self, accounts: Vec<UpstreamAccount>) {
        let valid_ids: HashSet<Uuid> = accounts.iter().map(|account| account.id).collect();
        *self.accounts.write().unwrap() = accounts;
        self.cursor.store(0, Ordering::Relaxed);
        if let Ok(mut account_traits) = self.account_traits.write() {
            account_traits.retain(|account_id, _| valid_ids.contains(account_id));
        }
        if let Ok(mut health) = self.health.write() {
            health.retain(|account_id, _| valid_ids.contains(account_id));
        }
        if let Ok(mut recent_success) = self.recent_success.write() {
            recent_success.retain(|account_id, _| valid_ids.contains(account_id));
        }
        if let Ok(mut sticky_sessions) = self.sticky_sessions.write() {
            sticky_sessions.retain(|_, entry| valid_ids.contains(&entry.account_id));
        }
    }

    pub fn replace_account_traits(&self, account_traits: Vec<AccountRoutingTraits>) {
        let valid_ids = self
            .accounts
            .read()
            .unwrap()
            .iter()
            .map(|account| account.id)
            .collect::<HashSet<_>>();
        let traits = account_traits
            .into_iter()
            .filter(|item| valid_ids.contains(&item.account_id))
            .map(|item| (item.account_id, item))
            .collect::<HashMap<_, _>>();
        *self.account_traits.write().unwrap() = traits;
    }

    fn candidate_account_ids_for_model(&self, model: Option<&str>) -> Option<Vec<Uuid>> {
        let model = model?.trim();
        if model.is_empty() {
            return None;
        }

        let plan = self.compiled_routing_plan.read().unwrap();
        let plan = plan.as_ref()?;

        let exact_match = plan
            .policies
            .iter()
            .find(|policy| policy.exact_models.iter().any(|item| item == model));
        let prefix_match = plan.policies.iter().find(|policy| {
            policy
                .model_prefixes
                .iter()
                .any(|prefix| model.starts_with(prefix))
        });

        let segments = exact_match
            .or(prefix_match)
            .map(|policy| &policy.fallback_segments)
            .unwrap_or(&plan.default_route);

        if segments.is_empty() {
            return None;
        }

        let mut account_ids = Vec::new();
        for segment in segments {
            account_ids.extend(segment.account_ids.iter().copied());
        }
        (!account_ids.is_empty()).then_some(account_ids)
    }

    pub fn upsert_account(&self, account: UpstreamAccount) {
        let account_id = account.id;
        let account_enabled = account.enabled;
        let mut replaced = false;

        if let Ok(mut accounts) = self.accounts.write() {
            if let Some(existing) = accounts.iter_mut().find(|item| item.id == account_id) {
                *existing = account;
                replaced = true;
            } else {
                accounts.push(account);
            }
        }

        if !replaced {
            self.cursor.store(0, Ordering::Relaxed);
        }

        if !account_enabled {
            if let Ok(mut account_traits) = self.account_traits.write() {
                account_traits.remove(&account_id);
            }
            if let Ok(mut health) = self.health.write() {
                health.remove(&account_id);
            }
            if let Ok(mut recent_success) = self.recent_success.write() {
                recent_success.remove(&account_id);
            }
            if let Ok(mut sticky_sessions) = self.sticky_sessions.write() {
                sticky_sessions.retain(|_, entry| entry.account_id != account_id);
            }
        }
    }

    pub fn delete_account(&self, account_id: Uuid) -> bool {
        let mut removed = false;
        if let Ok(mut accounts) = self.accounts.write() {
            let before = accounts.len();
            accounts.retain(|account| account.id != account_id);
            removed = accounts.len() != before;
        }

        if !removed {
            return false;
        }

        if let Ok(mut health) = self.health.write() {
            health.remove(&account_id);
        }
        if let Ok(mut account_traits) = self.account_traits.write() {
            account_traits.remove(&account_id);
        }
        if let Ok(mut recent_success) = self.recent_success.write() {
            recent_success.remove(&account_id);
        }
        if let Ok(mut sticky_sessions) = self.sticky_sessions.write() {
            sticky_sessions.retain(|_, entry| entry.account_id != account_id);
        }
        true
    }

    pub fn mark_unhealthy(&self, account_id: Uuid, ttl: Duration) {
        if ttl.is_zero() {
            return;
        }

        let until = Instant::now() + ttl;
        if let Ok(mut health) = self.health.write() {
            health.insert(account_id, until);
        }
    }

    pub fn clear_unhealthy(&self, account_id: Uuid) -> bool {
        let exists = self
            .accounts
            .read()
            .unwrap()
            .iter()
            .any(|account| account.id == account_id);
        if !exists {
            return false;
        }

        if let Ok(mut health) = self.health.write() {
            health.remove(&account_id);
        }

        true
    }

    pub fn clear_all_unhealthy(&self) -> usize {
        let Ok(mut health) = self.health.write() else {
            return 0;
        };

        let cleared = health.len();
        health.clear();
        cleared
    }

    pub fn record_success(&self, account_id: Uuid) {
        let exists = self
            .accounts
            .read()
            .unwrap()
            .iter()
            .any(|account| account.id == account_id && account.enabled);
        if !exists {
            return;
        }

        if let Ok(mut recent_success) = self.recent_success.write() {
            recent_success.insert(account_id, Instant::now());
        }
    }

    pub fn bind_sticky(&self, sticky_key: &str, account_id: Uuid) -> bool {
        let Some(sticky_key) = normalize_sticky_key(Some(sticky_key)) else {
            return false;
        };
        if self.pick_specific(account_id).is_none() {
            return false;
        }
        let rebind = self.insert_sticky_mapping(sticky_key, account_id);
        if rebind {
            self.sticky_rebind_count.fetch_add(1, Ordering::Relaxed);
        }
        true
    }

    pub fn unbind_sticky(&self, sticky_key: &str) -> bool {
        let Some(sticky_key) = normalize_sticky_key(Some(sticky_key)) else {
            return false;
        };
        let now = Instant::now();
        let Ok(mut items) = self.sticky_sessions.write() else {
            return false;
        };
        prune_expired_sticky_sessions(&mut items, now);
        items.remove(&sticky_key).is_some()
    }

    fn is_healthy(&self, account_id: Uuid) -> bool {
        let now = Instant::now();
        let until = {
            let Ok(health) = self.health.read() else {
                return true;
            };
            health.get(&account_id).copied()
        };

        let Some(until) = until else {
            return true;
        };
        if until > now {
            return false;
        }

        if let Ok(mut health) = self.health.write() {
            if health
                .get(&account_id)
                .is_some_and(|deadline| *deadline <= now)
            {
                health.remove(&account_id);
            }
        }
        true
    }

    fn pick_specific(&self, account_id: Uuid) -> Option<UpstreamAccount> {
        let accounts = self.accounts.read().unwrap();
        let account = accounts.iter().find(|item| item.id == account_id)?;
        if !account.enabled || !self.is_healthy(account.id) {
            return None;
        }
        Some(account.clone())
    }

    fn preferred_recent_success_account_id(
        &self,
        accounts: &[UpstreamAccount],
        excluded_account_ids: &HashSet<Uuid>,
    ) -> Option<Uuid> {
        let recent_success = self.recent_success.read().unwrap();
        accounts
            .iter()
            .filter(|account| {
                !excluded_account_ids.contains(&account.id)
                    && account.enabled
                    && self.is_healthy(account.id)
            })
            .filter_map(|account| {
                recent_success
                    .get(&account.id)
                    .copied()
                    .map(|seen_at| (account.id, seen_at))
            })
            .max_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)))
            .map(|item| item.0)
    }

    fn preferred_recent_probe_account_id(
        &self,
        accounts: &[UpstreamAccount],
        excluded_account_ids: &HashSet<Uuid>,
    ) -> Option<Uuid> {
        let account_traits = self.account_traits.read().unwrap();
        accounts
            .iter()
            .filter(|account| {
                !excluded_account_ids.contains(&account.id)
                    && account.enabled
                    && self.is_healthy(account.id)
            })
            .filter_map(|account| {
                let traits = account_traits.get(&account.id)?;
                let freshness_rank = routing_health_freshness_rank(traits.health_freshness);
                (freshness_rank > 0).then_some((account.id, freshness_rank, traits.last_probe_at))
            })
            .max_by(|left, right| {
                left.1
                    .cmp(&right.1)
                    .then_with(|| left.2.cmp(&right.2))
                    .then_with(|| left.0.cmp(&right.0))
            })
            .map(|item| item.0)
    }

    fn pick_from_ordered_candidates(
        &self,
        candidate_ids: &[Uuid],
        sticky_key: Option<&str>,
        excluded_account_ids: &HashSet<Uuid>,
        prefer_non_conflicting: bool,
    ) -> Option<UpstreamAccount> {
        let candidate_id_set = candidate_ids.iter().copied().collect::<HashSet<_>>();
        let Some(sticky_key) = normalize_sticky_key(sticky_key) else {
            return self.pick_candidate_account(candidate_ids, excluded_account_ids);
        };
        self.sticky_session_total.fetch_add(1, Ordering::Relaxed);

        if let Some(sticky_account_id) = self.get_sticky_account_id(&sticky_key) {
            if excluded_account_ids.contains(&sticky_account_id)
                || !candidate_id_set.contains(&sticky_account_id)
            {
                self.remove_sticky_mapping(&sticky_key);
            } else if let Some(account) = self.pick_specific(sticky_account_id) {
                self.sticky_hit_count.fetch_add(1, Ordering::Relaxed);
                return Some(account);
            } else {
                self.remove_sticky_mapping(&sticky_key);
            }
        }

        self.sticky_miss_count.fetch_add(1, Ordering::Relaxed);
        let account = if prefer_non_conflicting {
            self.pick_candidate_account_avoiding_conflicts(
                &sticky_key,
                candidate_ids,
                excluded_account_ids,
            )
            .or_else(|| self.pick_candidate_account(candidate_ids, excluded_account_ids))
        } else {
            self.pick_candidate_account(candidate_ids, excluded_account_ids)
        }?;
        let rebind = self.insert_sticky_mapping(sticky_key, account.id);
        if rebind {
            self.sticky_rebind_count.fetch_add(1, Ordering::Relaxed);
        }
        Some(account)
    }

    fn pick_candidate_account(
        &self,
        candidate_ids: &[Uuid],
        excluded_account_ids: &HashSet<Uuid>,
    ) -> Option<UpstreamAccount> {
        let recent_success = self.recent_success.read().unwrap();
        if let Some(account_id) = candidate_ids
            .iter()
            .filter(|account_id| !excluded_account_ids.contains(account_id))
            .filter_map(|account_id| {
                recent_success
                    .get(account_id)
                    .copied()
                    .map(|seen_at| (*account_id, seen_at))
            })
            .max_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)))
            .map(|item| item.0)
        {
            drop(recent_success);
            if let Some(account) = self.pick_specific(account_id) {
                return Some(account);
            }
        } else {
            drop(recent_success);
        }

        if let Some(account_id) =
            self.preferred_candidate_probe_account_id(candidate_ids, excluded_account_ids)
        {
            if let Some(account) = self.pick_specific(account_id) {
                return Some(account);
            }
        }

        for account_id in candidate_ids {
            if excluded_account_ids.contains(account_id) {
                continue;
            }
            if let Some(account) = self.pick_specific(*account_id) {
                return Some(account);
            }
        }
        None
    }

    fn preferred_candidate_probe_account_id(
        &self,
        candidate_ids: &[Uuid],
        excluded_account_ids: &HashSet<Uuid>,
    ) -> Option<Uuid> {
        let account_traits = self.account_traits.read().unwrap();
        candidate_ids
            .iter()
            .filter(|account_id| !excluded_account_ids.contains(account_id))
            .filter_map(|account_id| {
                let traits = account_traits.get(account_id)?;
                let freshness_rank = routing_health_freshness_rank(traits.health_freshness);
                (freshness_rank > 0).then_some((*account_id, freshness_rank, traits.last_probe_at))
            })
            .max_by(|left, right| {
                left.1
                    .cmp(&right.1)
                    .then_with(|| left.2.cmp(&right.2))
                    .then_with(|| left.0.cmp(&right.0))
            })
            .map(|item| item.0)
    }

    fn pick_candidate_account_avoiding_conflicts(
        &self,
        sticky_key: &str,
        candidate_ids: &[Uuid],
        excluded_account_ids: &HashSet<Uuid>,
    ) -> Option<UpstreamAccount> {
        let conflict_ids = self.collect_conflict_account_ids(sticky_key);
        for account_id in candidate_ids {
            if excluded_account_ids.contains(account_id) || conflict_ids.contains(account_id) {
                continue;
            }
            if let Some(account) = self.pick_specific(*account_id) {
                return Some(account);
            }
        }
        None
    }

    fn pick_avoiding_sticky_conflicts(
        &self,
        sticky_key: &str,
        excluded_account_ids: &HashSet<Uuid>,
    ) -> Option<UpstreamAccount> {
        let conflict_ids = self.collect_conflict_account_ids(sticky_key);
        let accounts = self.accounts.read().unwrap();

        if accounts.is_empty() {
            return None;
        }

        for _ in 0..accounts.len() {
            let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % accounts.len();
            let account = accounts.get(idx)?;
            if excluded_account_ids.contains(&account.id) {
                continue;
            }
            if conflict_ids.contains(&account.id) {
                continue;
            }
            if account.enabled && self.is_healthy(account.id) {
                return Some(account.clone());
            }
        }

        None
    }

    fn collect_conflict_account_ids(&self, sticky_key: &str) -> HashSet<Uuid> {
        let now = Instant::now();
        let Ok(mut items) = self.sticky_sessions.write() else {
            return HashSet::new();
        };
        prune_expired_sticky_sessions(&mut items, now);
        items
            .iter()
            .filter(|(key, _)| key.as_str() != sticky_key)
            .map(|(_, entry)| entry.account_id)
            .collect()
    }

    fn get_sticky_account_id(&self, sticky_key: &str) -> Option<Uuid> {
        let now = Instant::now();
        let Ok(mut items) = self.sticky_sessions.write() else {
            return None;
        };
        prune_expired_sticky_sessions(&mut items, now);
        let entry = items.get_mut(sticky_key)?;
        entry.last_used_at = now;
        entry.expires_at = now + self.sticky_session_ttl;
        Some(entry.account_id)
    }

    fn remove_sticky_mapping(&self, sticky_key: &str) {
        if let Ok(mut items) = self.sticky_sessions.write() {
            items.remove(sticky_key);
        }
    }

    fn insert_sticky_mapping(&self, sticky_key: String, account_id: Uuid) -> bool {
        let now = Instant::now();
        let Ok(mut items) = self.sticky_sessions.write() else {
            return false;
        };
        prune_expired_sticky_sessions(&mut items, now);
        enforce_sticky_capacity(&mut items, self.sticky_session_max_entries);
        let entry = StickySessionEntry {
            account_id,
            expires_at: now + self.sticky_session_ttl,
            last_used_at: now,
        };
        items
            .insert(sticky_key, entry)
            .is_some_and(|previous| previous.account_id != account_id)
    }

    fn sticky_mapping_total(&self) -> usize {
        let now = Instant::now();
        let Ok(mut items) = self.sticky_sessions.write() else {
            return 0;
        };
        prune_expired_sticky_sessions(&mut items, now);
        items.len()
    }
}

fn routing_health_freshness_rank(freshness: Option<AccountRoutingHealthFreshness>) -> u8 {
    match freshness {
        Some(AccountRoutingHealthFreshness::Fresh) => 2,
        Some(AccountRoutingHealthFreshness::Stale) => 1,
        Some(AccountRoutingHealthFreshness::Unknown) | None => 0,
    }
}

fn prune_expired_sticky_sessions(items: &mut HashMap<String, StickySessionEntry>, now: Instant) {
    items.retain(|_, entry| entry.expires_at > now);
}

fn enforce_sticky_capacity(items: &mut HashMap<String, StickySessionEntry>, max_entries: usize) {
    while items.len() >= max_entries {
        let Some(oldest_key) = items
            .iter()
            .min_by_key(|(_, entry)| entry.last_used_at)
            .map(|(key, _)| key.clone())
        else {
            break;
        };
        items.remove(&oldest_key);
    }
}

fn account_diagnostics_for(
    account: &UpstreamAccount,
    health: &HashMap<Uuid, Instant>,
    now: Instant,
) -> AccountDiagnostics {
    AccountDiagnostics {
        id: account.id,
        label: account.label.clone(),
        mode: account.mode.clone(),
        enabled: account.enabled,
        priority: account.priority,
        base_url: account.base_url.clone(),
        chatgpt_account_id: account.chatgpt_account_id.clone(),
        temporarily_unhealthy: health.get(&account.id).is_some_and(|until| *until > now),
    }
}

fn normalize_sticky_key(raw: Option<&str>) -> Option<String> {
    let key = raw?.trim();
    if key.is_empty() {
        return None;
    }
    Some(key.to_string())
}

#[cfg(test)]
mod tests {
    use super::RoundRobinRouter;
    use chrono::Utc;
    use codex_pool_core::model::{
        CompiledModelRoutingPolicy, CompiledRoutingPlan, CompiledRoutingProfile, UpstreamAccount,
        UpstreamMode,
    };
    use std::collections::HashSet;
    use std::time::Duration;
    use uuid::Uuid;

    fn account(label: &str) -> UpstreamAccount {
        UpstreamAccount {
            id: Uuid::new_v4(),
            label: label.to_string(),
            mode: UpstreamMode::ChatGptSession,
            base_url: "https://chatgpt.com/backend-api/codex".to_string(),
            bearer_token: format!("tok-{label}"),
            chatgpt_account_id: Some(format!("acct-{label}")),
            enabled: true,
            priority: 100,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn sticky_session_reuses_same_account() {
        let a = account("a");
        let b = account("b");
        let router = RoundRobinRouter::new(vec![a.clone(), b.clone()]);

        let first = router.pick_with_sticky(Some("session-1")).unwrap();
        let second = router.pick_with_sticky(Some("session-1")).unwrap();
        let third = router.pick_with_sticky(Some("session-2")).unwrap();

        assert_eq!(first.id, a.id);
        assert_eq!(second.id, a.id);
        assert_eq!(third.id, b.id);

        let stats = router.sticky_stats();
        assert_eq!(stats.sticky_session_total, 3);
        assert_eq!(stats.sticky_hit_count, 1);
        assert_eq!(stats.sticky_miss_count, 2);
        assert_eq!(stats.sticky_mapping_total, 2);
    }

    #[tokio::test]
    async fn sticky_session_expires_after_ttl_and_rebinds() {
        let a = account("a");
        let b = account("b");
        let router = RoundRobinRouter::new_with_sticky_limits(
            vec![a.clone(), b.clone()],
            Duration::from_millis(10),
            64,
        );

        let first = router.pick_with_sticky(Some("session-expire")).unwrap();
        let second = router.pick_with_sticky(Some("session-expire")).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        let third = router.pick_with_sticky(Some("session-expire")).unwrap();

        assert_eq!(first.id, a.id);
        assert_eq!(second.id, a.id);
        assert_eq!(third.id, b.id);
    }

    #[test]
    fn sticky_session_capacity_evicts_oldest_mapping() {
        let a = account("a");
        let b = account("b");
        let router = RoundRobinRouter::new_with_sticky_limits(
            vec![a.clone(), b.clone()],
            Duration::from_secs(300),
            2,
        );

        let first = router.pick_with_sticky(Some("s1")).unwrap();
        let _ = router.pick_with_sticky(Some("s2")).unwrap();
        let _ = router.pick_with_sticky(Some("s3")).unwrap();
        let replay = router.pick_with_sticky(Some("s1")).unwrap();
        let stats = router.sticky_stats();

        assert_eq!(first.id, a.id);
        assert_eq!(replay.id, b.id);
        assert_eq!(stats.sticky_mapping_total, 2);
    }

    #[test]
    fn pick_excluding_skips_already_tried_accounts() {
        let a = account("a");
        let b = account("b");
        let c = account("c");
        let router = RoundRobinRouter::new(vec![a.clone(), b.clone(), c.clone()]);
        let excluded = HashSet::from([a.id, b.id]);

        let picked = router.pick_excluding(&excluded).expect("must pick account");

        assert_eq!(picked.id, c.id);
    }

    #[test]
    fn sticky_policy_prefers_non_conflicting_account_when_available() {
        let a = account("a");
        let b = account("b");
        let router = RoundRobinRouter::new(vec![a.clone(), b.clone()]);
        assert!(router.bind_sticky("busy-session", a.id));

        let picked = router
            .pick_with_policy(Some("new-session"), &HashSet::new(), true)
            .expect("must pick account");

        assert_eq!(picked.id, b.id);
    }

    fn compiled_route(
        exact_models: &[&str],
        fallback_segments: Vec<Vec<Uuid>>,
    ) -> CompiledRoutingPlan {
        CompiledRoutingPlan {
            version_id: Uuid::new_v4(),
            published_at: Utc::now(),
            trigger_reason: Some("test".to_string()),
            default_route: Vec::new(),
            policies: vec![CompiledModelRoutingPolicy {
                id: Uuid::new_v4(),
                name: "test-policy".to_string(),
                family: "test-family".to_string(),
                exact_models: exact_models
                    .iter()
                    .map(|item| (*item).to_string())
                    .collect(),
                model_prefixes: Vec::new(),
                fallback_segments: fallback_segments
                    .into_iter()
                    .enumerate()
                    .map(|(index, account_ids)| CompiledRoutingProfile {
                        id: Uuid::new_v4(),
                        name: format!("segment-{index}"),
                        account_ids,
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn compiled_model_route_prefers_configured_fallback_chain_before_round_robin() {
        let free = account("free");
        let paid = account("paid");
        let router = RoundRobinRouter::new(vec![free.clone(), paid.clone()]);
        router.replace_compiled_routing_plan(Some(compiled_route(
            &["gpt-5.4"],
            vec![vec![paid.id], vec![free.id]],
        )));

        let picked = router
            .pick_for_model(Some("gpt-5.4"), None, &HashSet::new(), false)
            .expect("route should pick account");

        assert_eq!(picked.id, paid.id);
    }

    #[test]
    fn sticky_binding_rebinds_when_compiled_route_no_longer_allows_previous_account() {
        let free = account("free");
        let paid = account("paid");
        let router = RoundRobinRouter::new(vec![free.clone(), paid.clone()]);
        router.replace_compiled_routing_plan(Some(compiled_route(
            &["gpt-5.2-codex"],
            vec![vec![free.id], vec![paid.id]],
        )));

        let first = router
            .pick_for_model(
                Some("gpt-5.2-codex"),
                Some("sticky-1"),
                &HashSet::new(),
                false,
            )
            .expect("initial route should pick account");
        assert_eq!(first.id, free.id);

        router.replace_compiled_routing_plan(Some(compiled_route(
            &["gpt-5.2-codex"],
            vec![vec![paid.id]],
        )));

        let rebound = router
            .pick_for_model(
                Some("gpt-5.2-codex"),
                Some("sticky-1"),
                &HashSet::new(),
                false,
            )
            .expect("route should rebind after compiled route update");

        assert_eq!(rebound.id, paid.id);
    }

    #[test]
    fn compiled_model_route_reports_account_eligibility() {
        let free = account("free");
        let paid = account("paid");
        let router = RoundRobinRouter::new(vec![free.clone(), paid.clone()]);
        router
            .replace_compiled_routing_plan(Some(compiled_route(&["gpt-5.4"], vec![vec![paid.id]])));

        assert!(!router.account_matches_model_route(free.id, Some("gpt-5.4")));
        assert!(router.account_matches_model_route(paid.id, Some("gpt-5.4")));
        assert!(router.account_matches_model_route(free.id, Some("gpt-5.2-codex")));
    }
}
