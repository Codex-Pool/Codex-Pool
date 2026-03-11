impl InMemoryStore {
    fn set_oauth_family_enabled_inner(
        &self,
        account_id: Uuid,
        enabled: bool,
    ) -> Result<OAuthFamilyActionResponse> {
        let provider = self.account_auth_provider(account_id);
        if provider != UpstreamAuthProvider::OAuthRefreshToken {
            return Err(anyhow!("account is not an oauth account"));
        }

        let credentials = self.oauth_credentials.read().unwrap();
        let target = credentials
            .get(&account_id)
            .ok_or_else(|| anyhow!("oauth credential not found"))?;
        let family_id = target.token_family_id.clone();
        drop(credentials);

        let affected = self.disable_or_enable_oauth_family(&family_id, enabled);
        if affected > 0 {
            self.revision.fetch_add(1, Ordering::Relaxed);
        }

        Ok(OAuthFamilyActionResponse {
            account_id,
            token_family_id: Some(family_id),
            enabled,
            affected_accounts: affected as u64,
        })
    }

    fn disable_oauth_family_inner(&self, family_id: &str) {
        let affected = self.disable_or_enable_oauth_family(family_id, false);
        if affected > 0 {
            self.revision.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn disable_or_enable_oauth_family(&self, family_id: &str, enabled: bool) -> usize {
        let account_ids = {
            let credentials = self.oauth_credentials.read().unwrap();
            credentials
                .iter()
                .filter_map(|(account_id, credential)| {
                    if credential.token_family_id == family_id {
                        Some(*account_id)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };

        if account_ids.is_empty() {
            return 0;
        }

        if enabled {
            let mut credentials = self.oauth_credentials.write().unwrap();
            for account_id in &account_ids {
                if let Some(credential) = credentials.get_mut(account_id) {
                    credential.refresh_reused_detected = false;
                    credential.refresh_backoff_until = None;
                }
            }
        }

        let mut affected = 0_usize;
        let mut accounts = self.accounts.write().unwrap();
        for account_id in account_ids {
            if let Some(account) = accounts.get_mut(&account_id) {
                if account.enabled != enabled {
                    account.enabled = enabled;
                    affected = affected.saturating_add(1);
                }
            }
        }
        affected
    }

    fn build_account_routing_traits(
        &self,
        accounts: &[UpstreamAccount],
    ) -> HashMap<Uuid, AccountRoutingTraits> {
        let session_profiles = self.session_profiles.read().unwrap().clone();
        let providers = self.account_auth_providers.read().unwrap().clone();
        let model_support = self.account_model_support.read().unwrap().clone();

        accounts
            .iter()
            .map(|account| {
                let provider = providers
                    .get(&account.id)
                    .cloned()
                    .unwrap_or(UpstreamAuthProvider::LegacyBearer);
                let support = model_support.get(&account.id).cloned().unwrap_or_default();
                (
                    account.id,
                    AccountRoutingTraits {
                        account_id: account.id,
                        plan_type: session_profiles
                            .get(&account.id)
                            .and_then(|profile| profile.chatgpt_plan_type.clone()),
                        auth_provider: Some(provider),
                        supported_models: support.supported_models,
                        blocked_until: None,
                        hard_block_reason: None,
                    },
                )
            })
            .collect()
    }

    fn compile_routing_plan_from_state(
        &self,
        accounts: &[UpstreamAccount],
        account_traits: &HashMap<Uuid, AccountRoutingTraits>,
    ) -> Option<CompiledRoutingPlan> {
        let mut profiles = self
            .routing_profiles
            .read()
            .unwrap()
            .values()
            .filter(|profile| profile.enabled)
            .cloned()
            .collect::<Vec<_>>();
        profiles.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.created_at.cmp(&right.created_at))
        });

        let compiled_profiles = profiles
            .iter()
            .map(|profile| {
                let mut matched = accounts
                    .iter()
                    .filter(|account| {
                        profile_matches_account(profile, account, account_traits)
                            && !account_traits
                                .get(&account.id)
                                .and_then(|traits| traits.blocked_until)
                                .is_some_and(|blocked_until| blocked_until > Utc::now())
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                matched.sort_by(|left, right| {
                    right
                        .priority
                        .cmp(&left.priority)
                        .then_with(|| left.created_at.cmp(&right.created_at))
                });
                (
                    profile.id,
                    CompiledRoutingProfile {
                        id: profile.id,
                        name: profile.name.clone(),
                        account_ids: matched.into_iter().map(|account| account.id).collect(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let mut source_policies = self
            .model_routing_policies
            .read()
            .unwrap()
            .values()
            .filter(|policy| policy.enabled)
            .cloned()
            .collect::<Vec<_>>();
        source_policies.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.created_at.cmp(&right.created_at))
        });

        let mut known_models = account_traits
            .values()
            .flat_map(|traits| traits.supported_models.iter().cloned())
            .collect::<Vec<_>>();
        for policy in &source_policies {
            known_models.extend(policy.exact_models.iter().cloned());
        }
        known_models.sort();
        known_models.dedup();

        let default_policy = source_policies
            .iter()
            .find(|policy| policy.exact_models.is_empty() && policy.model_prefixes.is_empty())
            .cloned();
        let mut policies = Vec::new();
        let mut routed_models = std::collections::HashSet::new();
        for policy in source_policies
            .into_iter()
            .filter(|policy| !(policy.exact_models.is_empty() && policy.model_prefixes.is_empty()))
        {
            let mut exact_models = policy.exact_models.clone();
            exact_models.extend(
                known_models
                    .iter()
                    .filter(|model: &&String| {
                        policy
                            .model_prefixes
                            .iter()
                            .any(|prefix| model.starts_with(prefix))
                    })
                    .cloned(),
            );
            exact_models.sort();
            exact_models.dedup();

            for model in exact_models {
                if routed_models.contains(&model) {
                    continue;
                }
                let fallback_segments = policy
                    .fallback_profile_ids
                    .iter()
                    .filter_map(|profile_id| compiled_profiles.get(profile_id).cloned())
                    .map(|profile| CompiledRoutingProfile {
                        account_ids: profile
                            .account_ids
                            .into_iter()
                            .filter(|account_id| account_supports_model(account_traits, *account_id, &model))
                            .collect(),
                        ..profile
                    })
                    .filter(|profile| !profile.account_ids.is_empty())
                    .collect::<Vec<_>>();

                if fallback_segments.is_empty() {
                    continue;
                }
                routed_models.insert(model.clone());

                policies.push(CompiledModelRoutingPolicy {
                    id: policy.id,
                    name: policy.name.clone(),
                    family: policy.family.clone(),
                    exact_models: vec![model],
                    model_prefixes: Vec::new(),
                    fallback_segments,
                });
            }
        }

        let default_route = default_policy
            .as_ref()
            .map(|policy| {
                policy
                    .fallback_profile_ids
                    .iter()
                    .filter_map(|profile_id| compiled_profiles.get(profile_id).cloned())
                    .filter(|profile| !profile.account_ids.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if let Some(default_policy) = default_policy {
            for model in known_models {
                if routed_models.contains(&model) {
                    continue;
                }
                let fallback_segments = default_policy
                    .fallback_profile_ids
                    .iter()
                    .filter_map(|profile_id| compiled_profiles.get(profile_id).cloned())
                    .map(|profile| CompiledRoutingProfile {
                        account_ids: profile
                            .account_ids
                            .into_iter()
                            .filter(|account_id| {
                                account_supports_model(account_traits, *account_id, &model)
                            })
                            .collect(),
                        ..profile
                    })
                    .filter(|profile| !profile.account_ids.is_empty())
                    .collect::<Vec<_>>();

                if fallback_segments.is_empty() {
                    continue;
                }

                policies.push(CompiledModelRoutingPolicy {
                    id: default_policy.id,
                    name: default_policy.name.clone(),
                    family: default_policy.family.clone(),
                    exact_models: vec![model],
                    model_prefixes: Vec::new(),
                    fallback_segments,
                });
            }
        }

        if default_route.is_empty() && policies.is_empty() {
            return None;
        }

        Some(CompiledRoutingPlan {
            version_id: Uuid::new_v4(),
            published_at: Utc::now(),
            trigger_reason: Some("in_memory_snapshot".to_string()),
            default_route,
            policies,
        })
    }

    fn snapshot_inner(&self) -> Result<DataPlaneSnapshot> {
        self.purge_expired_one_time_accounts_inner();
        let providers = self.account_auth_providers.read().unwrap().clone();
        let oauth_credentials = self.oauth_credentials.read().unwrap().clone();
        let mut accounts = self.list_upstream_accounts_inner();

        for account in &mut accounts {
            let provider = providers
                .get(&account.id)
                .cloned()
                .unwrap_or(UpstreamAuthProvider::LegacyBearer);
            if provider != UpstreamAuthProvider::OAuthRefreshToken {
                continue;
            }

            let Some(credential) = oauth_credentials.get(&account.id) else {
                account.enabled = false;
                account.bearer_token.clear();
                continue;
            };

            if credential.token_expires_at <= Utc::now() + Duration::seconds(OAUTH_MIN_VALID_SEC) {
                account.enabled = false;
            }

            if let Some(cipher) = &self.credential_cipher {
                match cipher.decrypt(&credential.access_token_enc) {
                    Ok(access_token) => account.bearer_token = access_token,
                    Err(_) => {
                        account.enabled = false;
                        account.bearer_token.clear();
                    }
                }
            } else {
                account.enabled = false;
                account.bearer_token.clear();
            }
        }

        let account_traits_map = self.build_account_routing_traits(&accounts);
        let compiled_routing_plan = self.compile_routing_plan_from_state(&accounts, &account_traits_map);

        Ok(DataPlaneSnapshot {
            revision: self.revision.load(Ordering::Relaxed),
            cursor: 0,
            accounts,
            account_traits: account_traits_map.into_values().collect(),
            compiled_routing_plan,
            issued_at: Utc::now(),
        })
    }
}

fn profile_matches_account(
    profile: &RoutingProfile,
    account: &UpstreamAccount,
    account_traits: &HashMap<Uuid, AccountRoutingTraits>,
) -> bool {
    if profile.selector.exclude_account_ids.contains(&account.id) {
        return false;
    }
    if profile.selector.include_account_ids.contains(&account.id) {
        return true;
    }

    let Some(traits) = account_traits.get(&account.id) else {
        return false;
    };

    if !profile.selector.plan_types.is_empty()
        && !traits.plan_type.as_ref().is_some_and(|plan_type| {
            profile
                .selector
                .plan_types
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(plan_type))
        })
    {
        return false;
    }

    if !profile.selector.modes.is_empty() && !profile.selector.modes.contains(&account.mode) {
        return false;
    }

    if !profile.selector.auth_providers.is_empty()
        && !traits
            .auth_provider
            .as_ref()
            .is_some_and(|provider| profile.selector.auth_providers.contains(provider))
    {
        return false;
    }

    true
}

fn account_supports_model(
    account_traits: &HashMap<Uuid, AccountRoutingTraits>,
    account_id: Uuid,
    model: &str,
) -> bool {
    account_traits
        .get(&account_id)
        .map(|traits| {
            traits.supported_models.is_empty()
                || traits.supported_models.iter().any(|candidate| candidate == model)
        })
        .unwrap_or(false)
}

include!("trait_impl.rs");
