use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::contracts::{OAuthRefreshStatus, SessionCredentialKind};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use codex_pool_core::api::ProductEdition;
use codex_pool_core::model::{
    AiErrorLearningSettings, ApiKey, BuiltinErrorTemplateOverrideRecord, ClaudeCodeRoutingSettings,
    ModelRoutingPolicy, ModelRoutingSettings, ModelRoutingTriggerMode, OutboundProxyNode,
    OutboundProxyPoolSettings, RoutingPlanVersion, RoutingPolicy, RoutingProfile, Tenant,
    UpstreamAccount, UpstreamAuthProvider, UpstreamErrorTemplateRecord,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[cfg(feature = "postgres-backend")]
use crate::store::postgres::PostgresStore;
use crate::store::SqliteBackedStore;
#[cfg(feature = "postgres-backend")]
use crate::usage::migration::{export_postgres_usage_bundle, import_postgres_usage_bundle};
use crate::usage::migration::{
    export_sqlite_usage_bundle, import_sqlite_usage_bundle, UsageMigrationBundle,
};

pub const EDITION_MIGRATION_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyTokenMigrationRecord {
    pub token: String,
    pub api_key_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountAuthProviderMigrationRecord {
    pub account_id: Uuid,
    pub auth_provider: UpstreamAuthProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCredentialMigrationRecord {
    pub account_id: Uuid,
    pub access_token_enc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_access_token_enc: Option<String>,
    pub refresh_token_enc: String,
    pub refresh_token_sha256: String,
    pub token_family_id: String,
    pub token_version: u64,
    pub token_expires_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_token_expires_at: Option<DateTime<Utc>>,
    pub last_refresh_at: Option<DateTime<Utc>>,
    pub last_refresh_status: OAuthRefreshStatus,
    pub refresh_reused_detected: bool,
    pub last_refresh_error_code: Option<String>,
    pub last_refresh_error: Option<String>,
    pub refresh_failure_count: u32,
    pub refresh_backoff_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionProfileMigrationRecord {
    pub account_id: Uuid,
    pub credential_kind: SessionCredentialKind,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub email: Option<String>,
    pub oauth_subject: Option<String>,
    pub oauth_identity_provider: Option<String>,
    pub email_verified: Option<bool>,
    pub chatgpt_plan_type: Option<String>,
    pub chatgpt_user_id: Option<String>,
    pub chatgpt_subscription_active_start: Option<DateTime<Utc>>,
    pub chatgpt_subscription_active_until: Option<DateTime<Utc>>,
    pub chatgpt_subscription_last_checked: Option<DateTime<Utc>>,
    pub chatgpt_account_user_id: Option<String>,
    pub chatgpt_compute_residency: Option<String>,
    pub workspace_name: Option<String>,
    pub organizations: Option<Vec<Value>>,
    pub groups: Option<Vec<Value>>,
    pub source_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamAccountHealthStateMigrationRecord {
    pub account_id: Uuid,
    pub seen_ok_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountModelSupportMigrationRecord {
    pub account_id: Uuid,
    pub supported_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ControlPlaneMigrationBundle {
    pub tenants: Vec<Tenant>,
    pub api_keys: Vec<ApiKey>,
    pub api_key_tokens: Vec<ApiKeyTokenMigrationRecord>,
    pub accounts: Vec<UpstreamAccount>,
    pub account_auth_providers: Vec<AccountAuthProviderMigrationRecord>,
    pub oauth_credentials: Vec<OAuthCredentialMigrationRecord>,
    pub session_profiles: Vec<SessionProfileMigrationRecord>,
    pub account_health_states: Vec<UpstreamAccountHealthStateMigrationRecord>,
    pub account_model_support: Vec<AccountModelSupportMigrationRecord>,
    pub routing_policies: Vec<RoutingPolicy>,
    pub routing_profiles: Vec<RoutingProfile>,
    pub model_routing_policies: Vec<ModelRoutingPolicy>,
    pub model_routing_settings: Option<ModelRoutingSettings>,
    pub claude_code_routing_settings: Option<ClaudeCodeRoutingSettings>,
    pub outbound_proxy_pool_settings: Option<OutboundProxyPoolSettings>,
    pub outbound_proxy_nodes: Vec<OutboundProxyNode>,
    pub upstream_error_learning_settings: Option<AiErrorLearningSettings>,
    pub upstream_error_templates: Vec<UpstreamErrorTemplateRecord>,
    pub builtin_error_template_overrides: Vec<BuiltinErrorTemplateOverrideRecord>,
    pub routing_plan_versions: Vec<RoutingPlanVersion>,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EditionMigrationArchiveKind {
    TenantUsers,
    TenantCreditAccounts,
    TenantCreditLedger,
    TenantCreditAuthorizations,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditionMigrationArchiveItem {
    pub kind: EditionMigrationArchiveKind,
    pub count: u64,
    pub description: String,
    #[serde(default)]
    pub rows: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct EditionMigrationArchiveManifest {
    pub items: Vec<EditionMigrationArchiveItem>,
}

impl EditionMigrationArchiveManifest {
    pub fn non_empty_items(&self) -> Vec<&EditionMigrationArchiveItem> {
        self.items
            .iter()
            .filter(|item| item.count > 0 || !item.rows.is_empty())
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditionMigrationArchiveInspectionItem {
    pub kind: EditionMigrationArchiveKind,
    pub count: u64,
    pub description: String,
    pub sample_rows: Vec<Value>,
    pub omitted_row_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditionMigrationPackage {
    pub schema_version: u32,
    pub source_edition: ProductEdition,
    pub exported_at: DateTime<Utc>,
    pub control_plane: ControlPlaneMigrationBundle,
    pub usage: UsageMigrationBundle,
    pub archive: EditionMigrationArchiveManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditionMigrationIssue {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditionMigrationSummary {
    pub tenant_count: usize,
    pub api_key_count: usize,
    pub account_count: usize,
    pub request_log_count: usize,
    pub archive_item_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditionMigrationPreflightReport {
    pub source_edition: ProductEdition,
    pub target_edition: ProductEdition,
    pub allowed: bool,
    pub blockers: Vec<EditionMigrationIssue>,
    pub warnings: Vec<EditionMigrationIssue>,
    pub summary: EditionMigrationSummary,
    pub archive: EditionMigrationArchiveManifest,
}

fn push_blocker(
    blockers: &mut Vec<EditionMigrationIssue>,
    code: impl Into<String>,
    message: impl Into<String>,
) {
    blockers.push(EditionMigrationIssue {
        code: code.into(),
        message: message.into(),
    });
}

fn push_warning(
    warnings: &mut Vec<EditionMigrationIssue>,
    code: impl Into<String>,
    message: impl Into<String>,
) {
    warnings.push(EditionMigrationIssue {
        code: code.into(),
        message: message.into(),
    });
}

pub(crate) fn default_model_routing_settings() -> ModelRoutingSettings {
    ModelRoutingSettings {
        enabled: true,
        auto_publish: true,
        planner_model_chain: Vec::new(),
        trigger_mode: ModelRoutingTriggerMode::Hybrid,
        kill_switch: false,
        updated_at: Utc::now(),
    }
}

pub(crate) fn default_claude_code_routing_settings() -> ClaudeCodeRoutingSettings {
    ClaudeCodeRoutingSettings {
        enabled: false,
        opus_target_model: None,
        sonnet_target_model: None,
        haiku_target_model: None,
        updated_at: Utc::now(),
    }
}

pub fn preflight_package(
    package: &EditionMigrationPackage,
    target_edition: ProductEdition,
) -> EditionMigrationPreflightReport {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let non_empty_archives = package.archive.non_empty_items();

    if package.schema_version != EDITION_MIGRATION_SCHEMA_VERSION {
        push_blocker(
            &mut blockers,
            "unsupported_schema_version",
            format!(
                "迁移包 schema_version={}，当前工具仅支持 {}",
                package.schema_version, EDITION_MIGRATION_SCHEMA_VERSION
            ),
        );
    }

    match target_edition {
        ProductEdition::Personal => {
            if package.control_plane.tenants.len() > 1 {
                push_blocker(
                    &mut blockers,
                    "multi_tenant_not_supported",
                    "personal 版只支持单 workspace，当前迁移包包含多个 tenant。",
                );
            }

            if package.source_edition == ProductEdition::Business
                && package.control_plane.tenants.len() > 1
            {
                push_blocker(
                    &mut blockers,
                    "business_to_personal_requires_staged_migration",
                    "business -> personal 在多 tenant 场景下只支持分阶段迁移；请先收缩到单 tenant 并归档 business 专属数据。",
                );
            }
        }
        ProductEdition::Team => {}
        ProductEdition::Business => {}
    }

    for item in non_empty_archives {
        let message = format!(
            "{} 条 {}会被归档，不会导入 {:?} 版运行态。",
            item.count, item.description, target_edition
        );
        push_warning(&mut warnings, "archive_item_present", message);
    }

    EditionMigrationPreflightReport {
        source_edition: package.source_edition,
        target_edition,
        allowed: blockers.is_empty(),
        blockers,
        warnings,
        summary: EditionMigrationSummary {
            tenant_count: package.control_plane.tenants.len(),
            api_key_count: package.control_plane.api_keys.len(),
            account_count: package.control_plane.accounts.len(),
            request_log_count: package.usage.request_logs.len(),
            archive_item_count: package.archive.non_empty_items().len(),
        },
        archive: package.archive.clone(),
    }
}

pub async fn build_sqlite_package(
    source_edition: ProductEdition,
    store: &SqliteBackedStore,
) -> Result<EditionMigrationPackage> {
    let control_plane = store.export_migration_bundle().await?;
    let usage = export_sqlite_usage_bundle(&store.clone_pool()).await?;

    Ok(EditionMigrationPackage {
        schema_version: EDITION_MIGRATION_SCHEMA_VERSION,
        source_edition,
        exported_at: Utc::now(),
        control_plane,
        usage,
        archive: EditionMigrationArchiveManifest::default(),
    })
}

#[cfg(feature = "postgres-backend")]
pub async fn build_postgres_package(
    source_edition: ProductEdition,
    database_url: &str,
) -> Result<EditionMigrationPackage> {
    let store = PostgresStore::connect(database_url).await?;
    let control_plane = store.export_migration_bundle().await?;
    let usage = export_postgres_usage_bundle(database_url).await?;
    let archive = store.export_archive_manifest(source_edition).await?;

    Ok(EditionMigrationPackage {
        schema_version: EDITION_MIGRATION_SCHEMA_VERSION,
        source_edition,
        exported_at: Utc::now(),
        control_plane,
        usage,
        archive,
    })
}

pub async fn import_package_into_sqlite(
    database_url: &str,
    package: &EditionMigrationPackage,
) -> Result<()> {
    let preflight = preflight_package(package, ProductEdition::Personal);
    if !preflight.allowed {
        bail!(
            "迁移预检失败: {}",
            preflight
                .blockers
                .iter()
                .map(|item| item.message.as_str())
                .collect::<Vec<_>>()
                .join("；")
        );
    }

    SqliteBackedStore::import_migration_bundle(database_url, &package.control_plane).await?;
    import_sqlite_usage_bundle(database_url, &package.usage).await?;
    Ok(())
}

#[cfg(feature = "postgres-backend")]
pub async fn import_package_into_postgres(
    target_edition: ProductEdition,
    database_url: &str,
    package: &EditionMigrationPackage,
) -> Result<()> {
    if matches!(target_edition, ProductEdition::Personal) {
        bail!("请使用 SQLite 导入 personal 版迁移包");
    }

    let preflight = preflight_package(package, target_edition);
    if !preflight.allowed {
        bail!(
            "迁移预检失败: {}",
            preflight
                .blockers
                .iter()
                .map(|item| item.message.as_str())
                .collect::<Vec<_>>()
                .join("；")
        );
    }

    PostgresStore::import_migration_bundle(database_url, &package.control_plane).await?;
    import_postgres_usage_bundle(database_url, &package.usage).await?;
    Ok(())
}

pub fn write_package_to_file(path: &Path, package: &EditionMigrationPackage) -> Result<()> {
    let encoded =
        serde_json::to_vec_pretty(package).context("failed to encode edition migration package")?;
    fs::write(path, encoded)
        .with_context(|| format!("failed to write migration package to {}", path.display()))
}

pub fn read_package_from_file(path: &Path) -> Result<EditionMigrationPackage> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read migration package from {}", path.display()))?;
    serde_json::from_slice(&bytes).context("failed to decode edition migration package")
}

pub fn write_archive_manifest_to_file(
    path: &Path,
    archive: &EditionMigrationArchiveManifest,
) -> Result<()> {
    let encoded =
        serde_json::to_vec_pretty(archive).context("failed to encode archive manifest")?;
    fs::write(path, encoded)
        .with_context(|| format!("failed to write archive manifest to {}", path.display()))
}

pub fn read_archive_manifest_from_file(path: &Path) -> Result<EditionMigrationArchiveManifest> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read archive manifest from {}", path.display()))?;
    serde_json::from_slice(&bytes).context("failed to decode archive manifest")
}

pub fn inspect_archive_manifest(
    archive: &EditionMigrationArchiveManifest,
) -> Vec<EditionMigrationArchiveInspectionItem> {
    archive
        .items
        .iter()
        .map(|item| {
            let sample_rows = item.rows.iter().take(3).cloned().collect::<Vec<_>>();
            EditionMigrationArchiveInspectionItem {
                kind: item.kind,
                count: item.count,
                description: item.description.clone(),
                omitted_row_count: item.count.saturating_sub(sample_rows.len() as u64),
                sample_rows,
            }
        })
        .collect()
}

fn archive_row_matches_tenant(row: &Value, tenant_id: Uuid) -> bool {
    row.get("tenant_id")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .map(|row_tenant_id| row_tenant_id == tenant_id)
        .unwrap_or(true)
}

pub fn shrink_package_to_tenant(
    package: &EditionMigrationPackage,
    tenant_id: Uuid,
) -> Result<EditionMigrationPackage> {
    let tenant = package
        .control_plane
        .tenants
        .iter()
        .find(|item| item.id == tenant_id)
        .cloned()
        .with_context(|| format!("tenant {tenant_id} not found in migration package"))?;

    let retained_api_keys = package
        .control_plane
        .api_keys
        .iter()
        .filter(|item| item.tenant_id == tenant_id)
        .cloned()
        .collect::<Vec<_>>();
    let retained_api_key_ids = retained_api_keys
        .iter()
        .map(|item| item.id)
        .collect::<HashSet<_>>();

    let mut shrunk = package.clone();
    shrunk.exported_at = Utc::now();
    shrunk.control_plane.tenants = vec![tenant];
    shrunk.control_plane.api_keys = retained_api_keys;
    shrunk.control_plane.api_key_tokens = package
        .control_plane
        .api_key_tokens
        .iter()
        .filter(|item| retained_api_key_ids.contains(&item.api_key_id))
        .cloned()
        .collect();
    shrunk.control_plane.routing_policies = package
        .control_plane
        .routing_policies
        .iter()
        .filter(|item| item.tenant_id == tenant_id)
        .cloned()
        .collect();
    shrunk.usage.request_logs = package
        .usage
        .request_logs
        .iter()
        .filter(|item| {
            item.tenant_id == Some(tenant_id)
                || item
                    .api_key_id
                    .map(|api_key_id| retained_api_key_ids.contains(&api_key_id))
                    .unwrap_or(false)
        })
        .cloned()
        .collect();
    shrunk.archive.items = package
        .archive
        .items
        .iter()
        .cloned()
        .map(|mut item| {
            item.rows
                .retain(|row| archive_row_matches_tenant(row, tenant_id));
            item.count = item.rows.len() as u64;
            item
        })
        .collect();

    Ok(shrunk)
}

pub fn query_window() -> (i64, i64) {
    let start = 0;
    let end = (Utc::now() + Duration::days(3650)).timestamp();
    (start, end)
}

#[cfg(test)]
mod tests {
    use crate::contracts::{
        CreateApiKeyRequest, CreateTenantRequest, CreateUpstreamAccountRequest,
        UpsertModelRoutingPolicyRequest, UpsertRoutingProfileRequest,
    };
    use crate::store::{ControlPlaneStore, SqliteBackedStore};
    use crate::usage::clickhouse_repo::UsageQueryRepository;
    use crate::usage::sqlite_repo::SqliteUsageRepo;
    use crate::usage::UsageIngestRepository;
    use chrono::Utc;
    use codex_pool_core::api::ProductEdition;
    use codex_pool_core::events::RequestLogEvent;
    use codex_pool_core::model::{RoutingProfileSelector, UpstreamMode};
    use uuid::Uuid;

    fn sqlite_path(name: &str) -> String {
        let path = std::env::temp_dir().join(format!("{name}-{}.sqlite3", Uuid::new_v4()));
        path.display().to_string()
    }

    #[tokio::test]
    async fn sqlite_package_roundtrip_preserves_core_state_and_usage() {
        let source_url = sqlite_path("edition-migrate-source");
        let source_store = SqliteBackedStore::connect(&source_url)
            .await
            .expect("connect source sqlite store");
        let source_usage = SqliteUsageRepo::new(source_store.clone_pool())
            .await
            .expect("connect source sqlite usage repo");

        let tenant = source_store
            .create_tenant(CreateTenantRequest {
                name: "Personal Workspace".to_string(),
            })
            .await
            .expect("create tenant");
        let api_key = source_store
            .create_api_key(CreateApiKeyRequest {
                tenant_id: tenant.id,
                name: "default".to_string(),
            })
            .await
            .expect("create api key")
            .record;
        let account = source_store
            .create_upstream_account(CreateUpstreamAccountRequest {
                label: "OpenAI".to_string(),
                mode: UpstreamMode::OpenAiApiKey,
                base_url: "https://api.openai.com".to_string(),
                bearer_token: "sk-test".to_string(),
                chatgpt_account_id: None,
                auth_provider: None,
                enabled: Some(true),
                priority: Some(100),
            })
            .await
            .expect("create upstream account");

        let profile = source_store
            .upsert_routing_profile(UpsertRoutingProfileRequest {
                id: None,
                name: "default-profile".to_string(),
                description: Some("default profile".to_string()),
                enabled: true,
                priority: 100,
                selector: RoutingProfileSelector {
                    include_account_ids: vec![account.id],
                    ..RoutingProfileSelector::default()
                },
            })
            .await
            .expect("upsert routing profile");
        let policy = source_store
            .upsert_model_routing_policy(UpsertModelRoutingPolicyRequest {
                id: None,
                name: "default-policy".to_string(),
                family: "openai".to_string(),
                exact_models: vec!["gpt-5-mini".to_string()],
                model_prefixes: Vec::new(),
                fallback_profile_ids: vec![profile.id],
                enabled: true,
                priority: 100,
            })
            .await
            .expect("upsert model routing policy");

        let request_id = format!("req-{}", Uuid::new_v4());
        source_usage
            .ingest_request_log(RequestLogEvent {
                id: Uuid::new_v4(),
                account_id: account.id,
                tenant_id: Some(tenant.id),
                api_key_id: Some(api_key.id),
                event_version: 2,
                path: "/v1/responses".to_string(),
                method: "POST".to_string(),
                status_code: 200,
                latency_ms: 180,
                is_stream: false,
                error_code: None,
                request_id: Some(request_id.clone()),
                model: Some("gpt-5-mini".to_string()),
                service_tier: None,
                input_tokens: Some(200),
                cached_input_tokens: Some(20),
                output_tokens: Some(60),
                reasoning_tokens: Some(10),
                first_token_latency_ms: Some(50),
                billing_phase: Some("captured".to_string()),
                authorization_id: None,
                capture_status: Some("captured".to_string()),
                created_at: Utc::now(),
            })
            .await
            .expect("ingest request log");

        let package = super::build_sqlite_package(ProductEdition::Personal, &source_store)
            .await
            .expect("export sqlite edition package");

        let target_url = sqlite_path("edition-migrate-target");
        super::import_package_into_sqlite(&target_url, &package)
            .await
            .expect("import package into sqlite");

        let imported_store = SqliteBackedStore::connect(&target_url)
            .await
            .expect("connect imported sqlite store");
        let imported_usage = SqliteUsageRepo::new(imported_store.clone_pool())
            .await
            .expect("connect imported sqlite usage repo");

        let imported_tenants = imported_store.list_tenants().await.expect("list tenants");
        let imported_api_keys = imported_store.list_api_keys().await.expect("list api keys");
        let imported_accounts = imported_store
            .list_upstream_accounts()
            .await
            .expect("list upstream accounts");
        let imported_profiles = imported_store
            .list_routing_profiles()
            .await
            .expect("list routing profiles");
        let imported_policies = imported_store
            .list_model_routing_policies()
            .await
            .expect("list model routing policies");
        let logs = imported_usage
            .query_request_logs(crate::usage::RequestLogQuery {
                start_ts: Utc::now().timestamp() - 3600,
                end_ts: Utc::now().timestamp() + 3600,
                limit: 20,
                tenant_id: Some(tenant.id),
                api_key_id: Some(api_key.id),
                status_code: None,
                request_id: Some(request_id),
                keyword: None,
            })
            .await
            .expect("query imported request logs");

        assert_eq!(imported_tenants.len(), 1);
        assert_eq!(imported_tenants[0].id, tenant.id);
        assert_eq!(imported_api_keys.len(), 1);
        assert_eq!(imported_api_keys[0].id, api_key.id);
        assert_eq!(imported_accounts.len(), 1);
        assert_eq!(imported_accounts[0].id, account.id);
        assert_eq!(imported_profiles.len(), 1);
        assert_eq!(imported_profiles[0].id, profile.id);
        assert_eq!(imported_policies.len(), 1);
        assert_eq!(imported_policies[0].id, policy.id);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].tenant_id, Some(tenant.id));
        assert_eq!(logs[0].api_key_id, Some(api_key.id));
    }

    #[test]
    fn preflight_blocks_team_to_personal_when_multiple_tenants_exist() {
        let package = super::EditionMigrationPackage {
            schema_version: super::EDITION_MIGRATION_SCHEMA_VERSION,
            source_edition: ProductEdition::Team,
            exported_at: Utc::now(),
            control_plane: super::ControlPlaneMigrationBundle {
                tenants: vec![
                    codex_pool_core::model::Tenant {
                        id: Uuid::new_v4(),
                        name: "A".to_string(),
                        created_at: Utc::now(),
                    },
                    codex_pool_core::model::Tenant {
                        id: Uuid::new_v4(),
                        name: "B".to_string(),
                        created_at: Utc::now(),
                    },
                ],
                ..Default::default()
            },
            usage: super::UsageMigrationBundle::default(),
            archive: super::EditionMigrationArchiveManifest::default(),
        };

        let report = super::preflight_package(&package, ProductEdition::Personal);

        assert!(!report.allowed);
        assert!(report
            .blockers
            .iter()
            .any(|item| item.code == "multi_tenant_not_supported"));
    }

    #[test]
    fn preflight_allows_team_single_tenant_to_personal_with_archive_warning() {
        let tenant_id = Uuid::new_v4();
        let package = super::EditionMigrationPackage {
            schema_version: super::EDITION_MIGRATION_SCHEMA_VERSION,
            source_edition: ProductEdition::Team,
            exported_at: Utc::now(),
            control_plane: super::ControlPlaneMigrationBundle {
                tenants: vec![codex_pool_core::model::Tenant {
                    id: tenant_id,
                    name: "Solo Team".to_string(),
                    created_at: Utc::now(),
                }],
                ..Default::default()
            },
            usage: super::UsageMigrationBundle::default(),
            archive: super::EditionMigrationArchiveManifest {
                items: vec![super::EditionMigrationArchiveItem {
                    kind: super::EditionMigrationArchiveKind::TenantUsers,
                    count: 1,
                    description: "租户登录/会话相关数据".to_string(),
                    rows: vec![serde_json::json!({
                        "id": Uuid::new_v4(),
                        "email": "user@example.com"
                    })],
                }],
            },
        };

        let report = super::preflight_package(&package, ProductEdition::Personal);

        assert!(report.allowed);
        assert!(report.blockers.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|item| item.code == "archive_item_present"));
    }

    #[tokio::test]
    async fn import_team_package_into_personal_sqlite_preserves_core_state() {
        let tenant_id = Uuid::new_v4();
        let package = super::EditionMigrationPackage {
            schema_version: super::EDITION_MIGRATION_SCHEMA_VERSION,
            source_edition: ProductEdition::Team,
            exported_at: Utc::now(),
            control_plane: super::ControlPlaneMigrationBundle {
                tenants: vec![codex_pool_core::model::Tenant {
                    id: tenant_id,
                    name: "Solo Team".to_string(),
                    created_at: Utc::now(),
                }],
                revision: 7,
                ..Default::default()
            },
            usage: super::UsageMigrationBundle::default(),
            archive: super::EditionMigrationArchiveManifest {
                items: vec![super::EditionMigrationArchiveItem {
                    kind: super::EditionMigrationArchiveKind::TenantUsers,
                    count: 1,
                    description: "租户登录/会话相关数据".to_string(),
                    rows: vec![serde_json::json!({
                        "id": Uuid::new_v4(),
                        "tenant_id": tenant_id,
                        "email": "user@example.com"
                    })],
                }],
            },
        };

        let target_url = sqlite_path("edition-migrate-team-to-personal");
        super::import_package_into_sqlite(&target_url, &package)
            .await
            .expect("import team package into personal sqlite");

        let imported_store = SqliteBackedStore::connect(&target_url)
            .await
            .expect("connect imported sqlite store");
        let tenants = imported_store.list_tenants().await.expect("list tenants");

        assert_eq!(tenants.len(), 1);
        assert_eq!(tenants[0].id, tenant_id);
    }

    #[test]
    fn preflight_allows_business_to_team_with_archive_warning() {
        let package = super::EditionMigrationPackage {
            schema_version: super::EDITION_MIGRATION_SCHEMA_VERSION,
            source_edition: ProductEdition::Business,
            exported_at: Utc::now(),
            control_plane: super::ControlPlaneMigrationBundle {
                tenants: vec![codex_pool_core::model::Tenant {
                    id: Uuid::new_v4(),
                    name: "Business".to_string(),
                    created_at: Utc::now(),
                }],
                ..Default::default()
            },
            usage: super::UsageMigrationBundle::default(),
            archive: super::EditionMigrationArchiveManifest {
                items: vec![super::EditionMigrationArchiveItem {
                    kind: super::EditionMigrationArchiveKind::TenantCreditLedger,
                    count: 2,
                    description: "信用账本流水".to_string(),
                    rows: vec![
                        serde_json::json!({"id": Uuid::new_v4(), "request_id": "req-1"}),
                        serde_json::json!({"id": Uuid::new_v4(), "request_id": "req-2"}),
                    ],
                }],
            },
        };

        let report = super::preflight_package(&package, ProductEdition::Team);

        assert!(report.allowed);
        assert!(report.blockers.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|item| item.code == "archive_item_present"));
    }

    #[test]
    fn preflight_allows_single_tenant_business_to_personal_with_archive_warning() {
        let tenant_id = Uuid::new_v4();
        let package = super::EditionMigrationPackage {
            schema_version: super::EDITION_MIGRATION_SCHEMA_VERSION,
            source_edition: ProductEdition::Business,
            exported_at: Utc::now(),
            control_plane: super::ControlPlaneMigrationBundle {
                tenants: vec![codex_pool_core::model::Tenant {
                    id: tenant_id,
                    name: "Business Solo".to_string(),
                    created_at: Utc::now(),
                }],
                ..Default::default()
            },
            usage: super::UsageMigrationBundle::default(),
            archive: super::EditionMigrationArchiveManifest {
                items: vec![
                    super::EditionMigrationArchiveItem {
                        kind: super::EditionMigrationArchiveKind::TenantCreditAccounts,
                        count: 1,
                        description: "信用账户主表".to_string(),
                        rows: vec![serde_json::json!({
                            "tenant_id": tenant_id,
                            "balance_microcredits": 42
                        })],
                    },
                    super::EditionMigrationArchiveItem {
                        kind: super::EditionMigrationArchiveKind::TenantCreditLedger,
                        count: 1,
                        description: "信用账本流水".to_string(),
                        rows: vec![serde_json::json!({
                            "id": Uuid::new_v4(),
                            "tenant_id": tenant_id,
                            "request_id": "req-1"
                        })],
                    },
                ],
            },
        };

        let report = super::preflight_package(&package, ProductEdition::Personal);

        assert!(report.allowed);
        assert!(report.blockers.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|item| item.code == "archive_item_present"));
    }

    #[tokio::test]
    async fn import_single_tenant_business_package_into_personal_sqlite_preserves_core_state() {
        let tenant_id = Uuid::new_v4();
        let package = super::EditionMigrationPackage {
            schema_version: super::EDITION_MIGRATION_SCHEMA_VERSION,
            source_edition: ProductEdition::Business,
            exported_at: Utc::now(),
            control_plane: super::ControlPlaneMigrationBundle {
                tenants: vec![codex_pool_core::model::Tenant {
                    id: tenant_id,
                    name: "Business Solo".to_string(),
                    created_at: Utc::now(),
                }],
                revision: 11,
                ..Default::default()
            },
            usage: super::UsageMigrationBundle::default(),
            archive: super::EditionMigrationArchiveManifest {
                items: vec![super::EditionMigrationArchiveItem {
                    kind: super::EditionMigrationArchiveKind::TenantCreditAccounts,
                    count: 1,
                    description: "信用账户主表".to_string(),
                    rows: vec![serde_json::json!({
                        "tenant_id": tenant_id,
                        "balance_microcredits": 42
                    })],
                }],
            },
        };

        let target_url = sqlite_path("edition-migrate-business-to-personal");
        super::import_package_into_sqlite(&target_url, &package)
            .await
            .expect("import business package into personal sqlite");

        let imported_store = SqliteBackedStore::connect(&target_url)
            .await
            .expect("connect imported sqlite store");
        let tenants = imported_store.list_tenants().await.expect("list tenants");

        assert_eq!(tenants.len(), 1);
        assert_eq!(tenants[0].id, tenant_id);
    }

    #[test]
    fn archive_inspection_limits_rows_to_samples() {
        let archive = super::EditionMigrationArchiveManifest {
            items: vec![super::EditionMigrationArchiveItem {
                kind: super::EditionMigrationArchiveKind::TenantUsers,
                count: 4,
                description: "租户登录/会话相关数据".to_string(),
                rows: vec![
                    serde_json::json!({"email": "a@example.com"}),
                    serde_json::json!({"email": "b@example.com"}),
                    serde_json::json!({"email": "c@example.com"}),
                    serde_json::json!({"email": "d@example.com"}),
                ],
            }],
        };

        let inspection = super::inspect_archive_manifest(&archive);

        assert_eq!(inspection.len(), 1);
        assert_eq!(inspection[0].sample_rows.len(), 3);
        assert_eq!(inspection[0].omitted_row_count, 1);
    }

    #[test]
    fn shrink_package_to_tenant_filters_core_and_archive_state() {
        let retained_tenant_id = Uuid::new_v4();
        let dropped_tenant_id = Uuid::new_v4();
        let retained_api_key_id = Uuid::new_v4();
        let dropped_api_key_id = Uuid::new_v4();
        let package = super::EditionMigrationPackage {
            schema_version: super::EDITION_MIGRATION_SCHEMA_VERSION,
            source_edition: ProductEdition::Business,
            exported_at: Utc::now(),
            control_plane: super::ControlPlaneMigrationBundle {
                tenants: vec![
                    codex_pool_core::model::Tenant {
                        id: retained_tenant_id,
                        name: "Keep".to_string(),
                        created_at: Utc::now(),
                    },
                    codex_pool_core::model::Tenant {
                        id: dropped_tenant_id,
                        name: "Drop".to_string(),
                        created_at: Utc::now(),
                    },
                ],
                api_keys: vec![
                    codex_pool_core::model::ApiKey {
                        id: retained_api_key_id,
                        tenant_id: retained_tenant_id,
                        name: "keep-key".to_string(),
                        key_prefix: "cpk_keep".to_string(),
                        key_hash: "hash-keep".to_string(),
                        enabled: true,
                        created_at: Utc::now(),
                    },
                    codex_pool_core::model::ApiKey {
                        id: dropped_api_key_id,
                        tenant_id: dropped_tenant_id,
                        name: "drop-key".to_string(),
                        key_prefix: "cpk_drop".to_string(),
                        key_hash: "hash-drop".to_string(),
                        enabled: true,
                        created_at: Utc::now(),
                    },
                ],
                api_key_tokens: vec![
                    super::ApiKeyTokenMigrationRecord {
                        token: "tok-keep".to_string(),
                        api_key_id: retained_api_key_id,
                    },
                    super::ApiKeyTokenMigrationRecord {
                        token: "tok-drop".to_string(),
                        api_key_id: dropped_api_key_id,
                    },
                ],
                routing_policies: vec![
                    codex_pool_core::model::RoutingPolicy {
                        tenant_id: retained_tenant_id,
                        strategy: codex_pool_core::model::RoutingStrategy::RoundRobin,
                        max_retries: 1,
                        stream_max_retries: 1,
                        updated_at: Utc::now(),
                    },
                    codex_pool_core::model::RoutingPolicy {
                        tenant_id: dropped_tenant_id,
                        strategy: codex_pool_core::model::RoutingStrategy::FillFirst,
                        max_retries: 2,
                        stream_max_retries: 2,
                        updated_at: Utc::now(),
                    },
                ],
                ..Default::default()
            },
            usage: super::UsageMigrationBundle {
                request_logs: vec![
                    crate::usage::RequestLogRow {
                        id: Uuid::new_v4(),
                        account_id: Uuid::new_v4(),
                        tenant_id: Some(retained_tenant_id),
                        api_key_id: Some(retained_api_key_id),
                        request_id: Some("req-keep".to_string()),
                        path: "/v1/responses".to_string(),
                        method: "POST".to_string(),
                        model: Some("gpt-5-mini".to_string()),
                        service_tier: None,
                        input_tokens: Some(1),
                        cached_input_tokens: None,
                        output_tokens: Some(1),
                        reasoning_tokens: None,
                        first_token_latency_ms: None,
                        status_code: 200,
                        latency_ms: 100,
                        is_stream: false,
                        error_code: None,
                        billing_phase: None,
                        authorization_id: None,
                        capture_status: None,
                        estimated_cost_microusd: Some(1),
                        created_at: Utc::now(),
                        event_version: 2,
                    },
                    crate::usage::RequestLogRow {
                        id: Uuid::new_v4(),
                        account_id: Uuid::new_v4(),
                        tenant_id: Some(dropped_tenant_id),
                        api_key_id: Some(dropped_api_key_id),
                        request_id: Some("req-drop".to_string()),
                        path: "/v1/responses".to_string(),
                        method: "POST".to_string(),
                        model: Some("gpt-5-mini".to_string()),
                        service_tier: None,
                        input_tokens: Some(1),
                        cached_input_tokens: None,
                        output_tokens: Some(1),
                        reasoning_tokens: None,
                        first_token_latency_ms: None,
                        status_code: 200,
                        latency_ms: 100,
                        is_stream: false,
                        error_code: None,
                        billing_phase: None,
                        authorization_id: None,
                        capture_status: None,
                        estimated_cost_microusd: Some(1),
                        created_at: Utc::now(),
                        event_version: 2,
                    },
                ],
            },
            archive: super::EditionMigrationArchiveManifest {
                items: vec![super::EditionMigrationArchiveItem {
                    kind: super::EditionMigrationArchiveKind::TenantUsers,
                    count: 2,
                    description: "租户登录/会话相关数据".to_string(),
                    rows: vec![
                        serde_json::json!({
                            "tenant_id": retained_tenant_id,
                            "email": "keep@example.com"
                        }),
                        serde_json::json!({
                            "tenant_id": dropped_tenant_id,
                            "email": "drop@example.com"
                        }),
                    ],
                }],
            },
        };

        let shrunk = super::shrink_package_to_tenant(&package, retained_tenant_id)
            .expect("shrink package to tenant");

        assert_eq!(shrunk.control_plane.tenants.len(), 1);
        assert_eq!(shrunk.control_plane.tenants[0].id, retained_tenant_id);
        assert_eq!(shrunk.control_plane.api_keys.len(), 1);
        assert_eq!(shrunk.control_plane.api_keys[0].id, retained_api_key_id);
        assert_eq!(shrunk.control_plane.api_key_tokens.len(), 1);
        assert_eq!(
            shrunk.control_plane.api_key_tokens[0].api_key_id,
            retained_api_key_id
        );
        assert_eq!(shrunk.control_plane.routing_policies.len(), 1);
        assert_eq!(
            shrunk.control_plane.routing_policies[0].tenant_id,
            retained_tenant_id
        );
        assert_eq!(shrunk.usage.request_logs.len(), 1);
        assert_eq!(
            shrunk.usage.request_logs[0].tenant_id,
            Some(retained_tenant_id)
        );
        assert_eq!(shrunk.archive.items.len(), 1);
        assert_eq!(shrunk.archive.items[0].count, 1);
        assert_eq!(shrunk.archive.items[0].rows.len(), 1);
        let retained_tenant_id_string = retained_tenant_id.to_string();
        assert_eq!(
            shrunk.archive.items[0].rows[0]
                .get("tenant_id")
                .and_then(serde_json::Value::as_str),
            Some(retained_tenant_id_string.as_str())
        );

        let preflight = super::preflight_package(&shrunk, ProductEdition::Personal);
        assert!(preflight.allowed);
    }
}
