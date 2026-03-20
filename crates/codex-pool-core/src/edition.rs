use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProductEdition {
    Personal,
    Team,
    Business,
}

impl ProductEdition {
    pub fn from_env_value(value: Option<&str>) -> Self {
        value
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .and_then(|normalized| match normalized.as_str() {
                "personal" => Some(Self::Personal),
                "team" => Some(Self::Team),
                "business" => Some(Self::Business),
                _ => None,
            })
            .unwrap_or(Self::Business)
    }

    pub fn from_env_var(env_var: &str) -> Self {
        let value = std::env::var(env_var).ok();
        Self::from_env_value(value.as_deref())
    }

    pub fn infer_from_binary_name(binary_name: Option<&str>) -> Option<Self> {
        let file_name = binary_name
            .map(Path::new)
            .and_then(Path::file_name)
            .and_then(|value| value.to_str())?;
        match file_name {
            "codex-pool-personal" => Some(Self::Personal),
            "codex-pool-team" => Some(Self::Team),
            "codex-pool-business" => Some(Self::Business),
            _ => None,
        }
    }

    pub fn resolve_runtime_edition(env_value: Option<&str>, binary_name: Option<&str>) -> Self {
        if env_value.is_some() {
            return Self::from_env_value(env_value);
        }

        Self::infer_from_binary_name(binary_name).unwrap_or(Self::Business)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BillingMode {
    CostReportOnly,
    CreditEnforced,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditionFeatures {
    pub multi_tenant: bool,
    pub tenant_portal: bool,
    pub tenant_self_service: bool,
    pub tenant_recharge: bool,
    pub credit_billing: bool,
    pub cost_reports: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemCapabilitiesResponse {
    pub edition: ProductEdition,
    pub billing_mode: BillingMode,
    pub features: EditionFeatures,
}

impl SystemCapabilitiesResponse {
    pub fn for_edition(edition: ProductEdition) -> Self {
        match edition {
            ProductEdition::Personal => Self {
                edition,
                billing_mode: BillingMode::CostReportOnly,
                features: EditionFeatures {
                    multi_tenant: false,
                    tenant_portal: false,
                    tenant_self_service: false,
                    tenant_recharge: false,
                    credit_billing: false,
                    cost_reports: true,
                },
            },
            ProductEdition::Team => Self {
                edition,
                billing_mode: BillingMode::CostReportOnly,
                features: EditionFeatures {
                    multi_tenant: true,
                    tenant_portal: true,
                    tenant_self_service: false,
                    tenant_recharge: false,
                    credit_billing: false,
                    cost_reports: true,
                },
            },
            ProductEdition::Business => Self {
                edition,
                billing_mode: BillingMode::CreditEnforced,
                features: EditionFeatures {
                    multi_tenant: true,
                    tenant_portal: true,
                    tenant_self_service: true,
                    tenant_recharge: true,
                    credit_billing: true,
                    cost_reports: true,
                },
            },
        }
    }

    pub fn allows_multi_tenant(&self) -> bool {
        self.features.multi_tenant
    }

    pub fn allows_tenant_portal(&self) -> bool {
        self.features.tenant_portal
    }

    pub fn allows_tenant_self_service(&self) -> bool {
        self.features.tenant_self_service
    }

    pub fn allows_tenant_recharge(&self) -> bool {
        self.features.tenant_recharge
    }

    pub fn allows_credit_billing(&self) -> bool {
        self.features.credit_billing
    }

    pub fn visible_balance_microcredits(&self, balance_microcredits: Option<i64>) -> Option<i64> {
        if self.allows_credit_billing() {
            balance_microcredits
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ProductEdition, SystemCapabilitiesResponse};

    #[test]
    fn system_capabilities_follow_product_edition_defaults() {
        let personal = SystemCapabilitiesResponse::for_edition(ProductEdition::Personal);
        assert_eq!(personal.edition, ProductEdition::Personal);
        assert!(!personal.features.multi_tenant);
        assert!(!personal.features.credit_billing);

        let team = SystemCapabilitiesResponse::for_edition(ProductEdition::Team);
        assert_eq!(team.edition, ProductEdition::Team);
        assert!(team.features.multi_tenant);
        assert!(!team.features.tenant_self_service);
        assert!(!team.features.credit_billing);

        let business = SystemCapabilitiesResponse::for_edition(ProductEdition::Business);
        assert_eq!(business.edition, ProductEdition::Business);
        assert!(business.features.multi_tenant);
        assert!(business.features.credit_billing);
    }

    #[test]
    fn non_business_editions_hide_credit_balances() {
        let personal = SystemCapabilitiesResponse::for_edition(ProductEdition::Personal);
        assert_eq!(personal.visible_balance_microcredits(Some(42)), None);

        let team = SystemCapabilitiesResponse::for_edition(ProductEdition::Team);
        assert_eq!(team.visible_balance_microcredits(Some(42)), None);

        let business = SystemCapabilitiesResponse::for_edition(ProductEdition::Business);
        assert_eq!(business.visible_balance_microcredits(Some(42)), Some(42));
    }

    #[test]
    fn product_edition_parses_known_values_and_defaults_to_business() {
        assert_eq!(
            ProductEdition::from_env_value(Some("personal")),
            ProductEdition::Personal
        );
        assert_eq!(
            ProductEdition::from_env_value(Some("TEAM")),
            ProductEdition::Team
        );
        assert_eq!(
            ProductEdition::from_env_value(Some("business")),
            ProductEdition::Business
        );
        assert_eq!(
            ProductEdition::from_env_value(Some("unknown")),
            ProductEdition::Business
        );
        assert_eq!(
            ProductEdition::from_env_value(None),
            ProductEdition::Business
        );
    }

    #[test]
    fn product_edition_infers_from_binary_name() {
        assert_eq!(
            ProductEdition::infer_from_binary_name(Some("codex-pool-personal")),
            Some(ProductEdition::Personal)
        );
        assert_eq!(
            ProductEdition::infer_from_binary_name(Some("/tmp/bin/codex-pool-team")),
            Some(ProductEdition::Team)
        );
        assert_eq!(
            ProductEdition::infer_from_binary_name(Some("codex-pool-business")),
            Some(ProductEdition::Business)
        );
        assert_eq!(ProductEdition::infer_from_binary_name(Some("gateway")), None);
        assert_eq!(ProductEdition::infer_from_binary_name(None), None);
    }

    #[test]
    fn product_edition_resolves_env_before_binary_name() {
        assert_eq!(
            ProductEdition::resolve_runtime_edition(
                Some("team"),
                Some("codex-pool-personal")
            ),
            ProductEdition::Team
        );
        assert_eq!(
            ProductEdition::resolve_runtime_edition(
                Some("unknown"),
                Some("codex-pool-personal")
            ),
            ProductEdition::Business
        );
        assert_eq!(
            ProductEdition::resolve_runtime_edition(None, Some("/tmp/codex-pool-team")),
            ProductEdition::Team
        );
        assert_eq!(
            ProductEdition::resolve_runtime_edition(None, Some("control-plane")),
            ProductEdition::Business
        );
    }
}
