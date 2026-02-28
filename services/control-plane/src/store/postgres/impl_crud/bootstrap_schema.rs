impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(postgres_max_connections_from_env())
            .connect(database_url)
            .await
            .context("failed to connect to postgres")?;
        Self::ensure_schema(&pool).await?;
        Ok(Self {
            pool,
            oauth_client: std::sync::Arc::new(OpenAiOAuthClient::from_env()),
            credential_cipher: CredentialCipher::from_env().unwrap_or(None),
        })
    }

    pub async fn connect_with_oauth(
        database_url: &str,
        oauth_client: std::sync::Arc<dyn OAuthTokenClient>,
        credential_cipher: Option<CredentialCipher>,
    ) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(postgres_max_connections_from_env())
            .connect(database_url)
            .await
            .context("failed to connect to postgres")?;
        Self::ensure_schema(&pool).await?;
        Ok(Self {
            pool,
            oauth_client,
            credential_cipher,
        })
    }

    pub fn clone_pool(&self) -> PgPool {
        self.pool.clone()
    }

    pub async fn migrate_legacy_plaintext_api_key_tokens(&self, batch_size: usize) -> Result<u64> {
        let batch_size = batch_size.clamp(1, 10_000);
        let mut migrated = 0_u64;

        loop {
            let rows = sqlx::query(
                r#"
                SELECT api_key_id, token
                FROM api_key_tokens
                WHERE token LIKE 'cp_%'
                ORDER BY api_key_id ASC
                LIMIT $1
                "#,
            )
            .bind(batch_size as i64)
            .fetch_all(&self.pool)
            .await
            .context("failed to load legacy plaintext api key tokens")?;

            if rows.is_empty() {
                break;
            }

            let mut batch_migrated = 0_u64;
            for row in rows {
                let api_key_id: Uuid = row.try_get("api_key_id")?;
                let plaintext_token: String = row.try_get("token")?;
                let target_hash = hash_api_key_token(&plaintext_token);
                if self
                    .upgrade_api_key_token_hash(
                        api_key_id,
                        &plaintext_token,
                        &plaintext_token,
                        &target_hash,
                    )
                    .await?
                {
                    batch_migrated += 1;
                }
            }

            if batch_migrated == 0 {
                return Err(anyhow!(
                    "api key hash migration made no progress; manual inspection required"
                ));
            }

            migrated += batch_migrated;
        }

        Ok(migrated)
    }

    async fn ensure_schema(pool: &PgPool) -> Result<()> {
        let mut tx = pool
            .begin()
            .await
            .context("failed to start schema migration transaction")?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(SCHEMA_MIGRATION_LOCK_ID)
            .execute(tx.as_mut())
            .await
            .context("failed to lock schema migration")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS admin_users (
                id UUID PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create admin_users table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tenants (
                id UUID PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenants table")?;

        sqlx::query(
            r#"
            ALTER TABLE tenants
            ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active'
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add tenants.status column")?;

        sqlx::query(
            r#"
            ALTER TABLE tenants
            ADD COLUMN IF NOT EXISTS plan TEXT NOT NULL DEFAULT 'credit'
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add tenants.plan column")?;

        sqlx::query(
            r#"
            ALTER TABLE tenants
            ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ NULL
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add tenants.expires_at column")?;

        sqlx::query(
            r#"
            ALTER TABLE tenants
            ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add tenants.updated_at column")?;

        sqlx::query(
            r#"
            UPDATE tenants
            SET updated_at = COALESCE(updated_at, created_at, now())
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to backfill tenants.updated_at")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_keys (
                id UUID PRIMARY KEY,
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                key_prefix TEXT NOT NULL,
                key_hash TEXT NOT NULL,
                enabled BOOLEAN NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create api_keys table")?;

        sqlx::query(
            r#"
            ALTER TABLE api_keys
            ADD COLUMN IF NOT EXISTS ip_allowlist JSONB NOT NULL DEFAULT '[]'::jsonb
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add api_keys.ip_allowlist column")?;

        sqlx::query(
            r#"
            ALTER TABLE api_keys
            ADD COLUMN IF NOT EXISTS model_allowlist JSONB NOT NULL DEFAULT '[]'::jsonb
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add api_keys.model_allowlist column")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tenant_users (
                id UUID PRIMARY KEY,
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                email TEXT NOT NULL,
                password_hash TEXT NOT NULL,
                email_verified BOOLEAN NOT NULL DEFAULT false,
                enabled BOOLEAN NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                last_login_at TIMESTAMPTZ NULL,
                UNIQUE (tenant_id, email)
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_users table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_tenant_users_email
            ON tenant_users (email)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_users email index")?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_tenant_users_email_unique
            ON tenant_users (email)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_users email unique index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tenant_email_verification_codes (
                id UUID PRIMARY KEY,
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                tenant_user_id UUID NOT NULL REFERENCES tenant_users(id) ON DELETE CASCADE,
                purpose TEXT NOT NULL,
                code_hash TEXT NOT NULL,
                expires_at TIMESTAMPTZ NOT NULL,
                consumed_at TIMESTAMPTZ NULL,
                attempt_count INT NOT NULL DEFAULT 0,
                max_attempts INT NOT NULL DEFAULT 8,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_email_verification_codes table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_tenant_email_verification_codes_lookup
            ON tenant_email_verification_codes (tenant_user_id, purpose, consumed_at, expires_at DESC)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant email verification lookup index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tenant_credit_accounts (
                tenant_id UUID PRIMARY KEY REFERENCES tenants(id) ON DELETE CASCADE,
                balance_microcredits BIGINT NOT NULL DEFAULT 0,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_credit_accounts table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tenant_credit_ledger (
                id UUID PRIMARY KEY,
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                api_key_id UUID NULL REFERENCES api_keys(id) ON DELETE SET NULL,
                request_id TEXT NULL,
                event_type TEXT NOT NULL,
                delta_microcredits BIGINT NOT NULL,
                balance_after_microcredits BIGINT NOT NULL,
                unit_price_microcredits BIGINT NULL,
                input_tokens BIGINT NULL,
                output_tokens BIGINT NULL,
                model TEXT NULL,
                meta_json JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_credit_ledger table")?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_tenant_credit_ledger_request_id
            ON tenant_credit_ledger (tenant_id, request_id)
            WHERE request_id IS NOT NULL
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_credit_ledger request id unique index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_tenant_credit_ledger_tenant_created_at
            ON tenant_credit_ledger (tenant_id, created_at DESC)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_credit_ledger list index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tenant_credit_authorizations (
                id UUID PRIMARY KEY,
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                api_key_id UUID NULL REFERENCES api_keys(id) ON DELETE SET NULL,
                request_id TEXT NOT NULL,
                model TEXT NULL,
                reserved_microcredits BIGINT NOT NULL,
                captured_microcredits BIGINT NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                expires_at TIMESTAMPTZ NOT NULL,
                meta_json JSONB NOT NULL DEFAULT '{}'::jsonb,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_credit_authorizations table")?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_tenant_credit_authorizations_tenant_request
            ON tenant_credit_authorizations (tenant_id, request_id)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_credit_authorizations tenant request unique index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_tenant_credit_authorizations_status_expires
            ON tenant_credit_authorizations (status, expires_at)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_credit_authorizations status expires index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tenant_daily_checkins (
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                local_date DATE NOT NULL,
                reward_microcredits BIGINT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                PRIMARY KEY (tenant_id, local_date)
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create tenant_daily_checkins table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS model_pricing (
                id UUID PRIMARY KEY,
                model TEXT NOT NULL UNIQUE,
                input_price_microcredits BIGINT NOT NULL,
                cached_input_price_microcredits BIGINT NOT NULL DEFAULT 0,
                output_price_microcredits BIGINT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create model_pricing table")?;

        sqlx::query(
            r#"
            ALTER TABLE model_pricing
            ADD COLUMN IF NOT EXISTS cached_input_price_microcredits BIGINT NOT NULL DEFAULT 0
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add model_pricing cached_input_price_microcredits column")?;

        // Seed OpenAI Codex defaults (per 1M tokens, microcredits unit) without overriding custom pricing.
        let now = Utc::now();
        for (model, input_price, cached_input_price, output_price) in [
            ("gpt-5.3-codex", 1_500_000_i64, 150_000_i64, 6_000_000_i64),
            ("gpt-5-codex", 1_250_000_i64, 125_000_i64, 10_000_000_i64),
            ("gpt-5-codex-mini", 300_000_i64, 30_000_i64, 1_500_000_i64),
            ("gpt-5-codex-nano", 50_000_i64, 5_000_i64, 400_000_i64),
        ] {
            sqlx::query(
                r#"
                INSERT INTO model_pricing (
                    id,
                    model,
                    input_price_microcredits,
                    cached_input_price_microcredits,
                    output_price_microcredits,
                    enabled,
                    created_at,
                    updated_at
                )
                VALUES ($1, $2, $3, $4, $5, true, $6, $6)
                ON CONFLICT (model) DO NOTHING
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(model)
            .bind(input_price)
            .bind(cached_input_price)
            .bind(output_price)
            .bind(now)
            .execute(tx.as_mut())
            .await
            .with_context(|| format!("failed to seed default model pricing for {model}"))?;
        }

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS admin_model_entities (
                id UUID PRIMARY KEY,
                model_id TEXT NOT NULL UNIQUE,
                provider TEXT NOT NULL,
                visibility TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create admin_model_entities table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_admin_model_entities_model_id
            ON admin_model_entities (model_id)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create admin_model_entities model_id index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS admin_impersonation_sessions (
                id UUID PRIMARY KEY,
                admin_user_id UUID NOT NULL REFERENCES admin_users(id) ON DELETE CASCADE,
                tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
                reason TEXT NOT NULL,
                expires_at TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create admin_impersonation_sessions table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS audit_logs (
                id UUID PRIMARY KEY,
                actor_type TEXT NOT NULL,
                actor_id UUID NULL,
                tenant_id UUID NULL REFERENCES tenants(id) ON DELETE SET NULL,
                action TEXT NOT NULL,
                reason TEXT NULL,
                request_ip TEXT NULL,
                user_agent TEXT NULL,
                target_type TEXT NULL,
                target_id TEXT NULL,
                payload_json JSONB NOT NULL DEFAULT '{}'::jsonb,
                result_status TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create audit_logs table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant_created_at
            ON audit_logs (tenant_id, created_at DESC)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create audit_logs tenant index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS api_key_tokens (
                token TEXT PRIMARY KEY,
                api_key_id UUID NOT NULL UNIQUE REFERENCES api_keys(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create api_key_tokens table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS upstream_accounts (
                id UUID PRIMARY KEY,
                label TEXT NOT NULL,
                mode TEXT NOT NULL,
                base_url TEXT NOT NULL,
                bearer_token TEXT NOT NULL,
                chatgpt_account_id TEXT NULL,
                enabled BOOLEAN NOT NULL,
                pool_state TEXT NOT NULL DEFAULT 'active',
                priority INT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream_accounts table")?;

        sqlx::query(
            r#"
            ALTER TABLE upstream_accounts
            ADD COLUMN IF NOT EXISTS auth_provider TEXT NOT NULL DEFAULT 'legacy_bearer'
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add upstream_accounts.auth_provider column")?;

        sqlx::query(
            r#"
            ALTER TABLE upstream_accounts
            ADD COLUMN IF NOT EXISTS pool_state TEXT NOT NULL DEFAULT 'active'
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add upstream_accounts.pool_state column")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_accounts_pool_state_created_at
            ON upstream_accounts (pool_state, created_at)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream_accounts pool_state index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_refresh_token_vault (
                id UUID PRIMARY KEY,
                refresh_token_enc TEXT NOT NULL,
                refresh_token_sha256 TEXT NOT NULL DEFAULT '',
                base_url TEXT NOT NULL,
                label TEXT NOT NULL,
                email TEXT NULL,
                chatgpt_account_id TEXT NULL,
                chatgpt_plan_type TEXT NULL,
                source_type TEXT NULL,
                desired_mode TEXT NOT NULL DEFAULT 'chat_gpt_session',
                desired_enabled BOOLEAN NOT NULL DEFAULT true,
                desired_priority INT NOT NULL DEFAULT 100,
                status TEXT NOT NULL DEFAULT 'queued',
                failure_count INT NOT NULL DEFAULT 0,
                backoff_until TIMESTAMPTZ NULL,
                next_attempt_at TIMESTAMPTZ NULL,
                last_error_code TEXT NULL,
                last_error_message TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create oauth_refresh_token_vault table")?;

        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS idx_oauth_refresh_token_vault_hash
            ON oauth_refresh_token_vault (refresh_token_sha256)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create oauth_refresh_token_vault hash index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_oauth_refresh_token_vault_status_next_attempt
            ON oauth_refresh_token_vault (status, next_attempt_at, id)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create oauth_refresh_token_vault status-next index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_oauth_refresh_token_vault_chatgpt_account_id
            ON oauth_refresh_token_vault (chatgpt_account_id)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create oauth_refresh_token_vault chatgpt_account index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS upstream_account_oauth_credentials (
                account_id UUID PRIMARY KEY REFERENCES upstream_accounts(id) ON DELETE CASCADE,
                access_token_enc TEXT NOT NULL,
                refresh_token_enc TEXT NOT NULL,
                refresh_token_sha256 TEXT NOT NULL DEFAULT '',
                token_expires_at TIMESTAMPTZ NOT NULL,
                last_refresh_at TIMESTAMPTZ NULL,
                last_refresh_status TEXT NOT NULL DEFAULT 'never',
                last_refresh_error TEXT NULL,
                refresh_failure_count INT NOT NULL DEFAULT 0,
                refresh_backoff_until TIMESTAMPTZ NULL,
                next_refresh_at TIMESTAMPTZ NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream_account_oauth_credentials table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS upstream_account_session_profiles (
                account_id UUID PRIMARY KEY REFERENCES upstream_accounts(id) ON DELETE CASCADE,
                credential_kind TEXT NOT NULL DEFAULT 'refresh_rotatable',
                token_expires_at TIMESTAMPTZ NULL,
                chatgpt_plan_type TEXT NULL,
                source_type TEXT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream_account_session_profiles table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_session_profiles_kind_expiry
            ON upstream_account_session_profiles (credential_kind, token_expires_at)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create session profile kind-expiry index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS upstream_account_rate_limit_snapshots (
                account_id UUID PRIMARY KEY REFERENCES upstream_accounts(id) ON DELETE CASCADE,
                rate_limits_json JSONB NOT NULL DEFAULT '[]'::jsonb,
                fetched_at TIMESTAMPTZ NULL,
                expires_at TIMESTAMPTZ NULL,
                last_error_code TEXT NULL,
                last_error_message TEXT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream_account_rate_limit_snapshots table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_rate_limit_snapshot_expiry
            ON upstream_account_rate_limit_snapshots (expires_at)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create rate-limit snapshot expiry index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS upstream_account_health_state (
                account_id UUID PRIMARY KEY REFERENCES upstream_accounts(id) ON DELETE CASCADE,
                seen_ok_at TIMESTAMPTZ NULL,
                last_probe_at TIMESTAMPTZ NULL,
                last_probe_status TEXT NULL,
                last_probe_http_status INT NULL,
                last_probe_error_code TEXT NULL,
                last_probe_error_message TEXT NULL,
                failure_count INT NOT NULL DEFAULT 0,
                next_probe_at TIMESTAMPTZ NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream_account_health_state table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_account_health_state_next_probe
            ON upstream_account_health_state (next_probe_at)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream health next_probe index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_account_health_state_seen_ok
            ON upstream_account_health_state (seen_ok_at)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream health seen_ok index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS upstream_account_ops_locks (
                account_id UUID NOT NULL REFERENCES upstream_accounts(id) ON DELETE CASCADE,
                op_type TEXT NOT NULL,
                inflight_until TIMESTAMPTZ NOT NULL,
                claimed_at TIMESTAMPTZ NOT NULL,
                claimed_by TEXT NOT NULL,
                PRIMARY KEY (account_id, op_type)
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream_account_ops_locks table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_account_ops_locks_inflight
            ON upstream_account_ops_locks (op_type, inflight_until)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create upstream ops lock inflight index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_rate_limit_refresh_jobs (
                id UUID PRIMARY KEY,
                status TEXT NOT NULL,
                total BIGINT NOT NULL DEFAULT 0,
                processed BIGINT NOT NULL DEFAULT 0,
                success_count BIGINT NOT NULL DEFAULT 0,
                failed_count BIGINT NOT NULL DEFAULT 0,
                started_at TIMESTAMPTZ NULL,
                finished_at TIMESTAMPTZ NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create oauth_rate_limit_refresh_jobs table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_rate_limit_refresh_jobs_status_created_at
            ON oauth_rate_limit_refresh_jobs (status, created_at DESC)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create rate-limit refresh jobs status index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_rate_limit_refresh_job_errors (
                job_id UUID NOT NULL REFERENCES oauth_rate_limit_refresh_jobs(id) ON DELETE CASCADE,
                error_code TEXT NOT NULL,
                count BIGINT NOT NULL DEFAULT 0,
                updated_at TIMESTAMPTZ NOT NULL,
                PRIMARY KEY (job_id, error_code)
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create oauth_rate_limit_refresh_job_errors table")?;

        sqlx::query(
            r#"
            ALTER TABLE upstream_account_oauth_credentials
            ADD COLUMN IF NOT EXISTS refresh_token_sha256 TEXT NOT NULL DEFAULT ''
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add refresh_token_sha256 column")?;

        sqlx::query(
            r#"
            ALTER TABLE upstream_account_oauth_credentials
            ADD COLUMN IF NOT EXISTS token_family_id TEXT NULL
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add token_family_id column")?;

        sqlx::query(
            r#"
            ALTER TABLE upstream_account_oauth_credentials
            ADD COLUMN IF NOT EXISTS token_version BIGINT NOT NULL DEFAULT 0
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add token_version column")?;

        sqlx::query(
            r#"
            ALTER TABLE upstream_account_oauth_credentials
            ADD COLUMN IF NOT EXISTS last_refresh_error_code TEXT NULL
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add last_refresh_error_code column")?;

        sqlx::query(
            r#"
            ALTER TABLE upstream_account_oauth_credentials
            ADD COLUMN IF NOT EXISTS refresh_reused_detected BOOLEAN NOT NULL DEFAULT false
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add refresh_reused_detected column")?;

        sqlx::query(
            r#"
            ALTER TABLE upstream_account_oauth_credentials
            ADD COLUMN IF NOT EXISTS refresh_inflight_until TIMESTAMPTZ NULL
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add refresh_inflight_until column")?;

        sqlx::query(
            r#"
            ALTER TABLE upstream_account_oauth_credentials
            ADD COLUMN IF NOT EXISTS next_refresh_at TIMESTAMPTZ NULL
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to add next_refresh_at column")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_oauth_refresh_hash
            ON upstream_account_oauth_credentials (refresh_token_sha256)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create refresh hash index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_oauth_token_family
            ON upstream_account_oauth_credentials (token_family_id)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create token family index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_oauth_next_refresh_at
            ON upstream_account_oauth_credentials (next_refresh_at)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create oauth next_refresh_at index")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_upstream_oauth_token_expires_at
            ON upstream_account_oauth_credentials (token_expires_at)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create oauth token_expires_at index")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS routing_policies (
                tenant_id UUID PRIMARY KEY REFERENCES tenants(id) ON DELETE CASCADE,
                strategy TEXT NOT NULL,
                max_retries BIGINT NOT NULL,
                stream_max_retries BIGINT NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create routing_policies table")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS snapshot_state (
                singleton BOOLEAN PRIMARY KEY,
                revision BIGINT NOT NULL,
                dirty BOOLEAN NOT NULL DEFAULT FALSE
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create snapshot_state table")?;

        sqlx::query(
            r#"
            ALTER TABLE snapshot_state
            ADD COLUMN IF NOT EXISTS dirty BOOLEAN NOT NULL DEFAULT FALSE
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to alter snapshot_state table")?;

        sqlx::query(
            r#"
            INSERT INTO snapshot_state (singleton, revision, dirty)
            VALUES ($1, 1, false)
            ON CONFLICT (singleton) DO NOTHING
            "#,
        )
        .bind(SNAPSHOT_SINGLETON_ROW)
        .execute(tx.as_mut())
        .await
        .context("failed to initialize snapshot_state row")?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS data_plane_outbox (
                id BIGSERIAL PRIMARY KEY,
                event_type TEXT NOT NULL,
                account_id UUID NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create data_plane_outbox table")?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_data_plane_outbox_created_at
            ON data_plane_outbox (created_at)
            "#,
        )
        .execute(tx.as_mut())
        .await
        .context("failed to create data_plane_outbox created_at index")?;
        tx.commit()
            .await
            .context("failed to commit schema migration transaction")?;

        Ok(())
    }
}
