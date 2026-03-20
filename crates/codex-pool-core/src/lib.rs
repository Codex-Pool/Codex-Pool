pub mod api;
pub mod edition;
pub mod error;
pub mod events;
pub mod logging;
pub mod model;
pub mod runtime_contract;
pub mod snapshot;

pub use edition::{BillingMode, EditionFeatures, ProductEdition, SystemCapabilitiesResponse};
pub use error::{ErrorBody, ErrorEnvelope};
pub use runtime_contract::{
    ApiKeyGroupStatus, ApiKeyPolicy, ValidateApiKeyRequest, ValidateApiKeyResponse,
};
pub use snapshot::{
    DataPlaneSnapshot, DataPlaneSnapshotEvent, DataPlaneSnapshotEventType,
    DataPlaneSnapshotEventsResponse,
};

#[cfg(test)]
mod tests {
    use crate::{
        DataPlaneSnapshotEventType, ErrorEnvelope, ProductEdition, SystemCapabilitiesResponse,
        ValidateApiKeyRequest,
    };

    #[test]
    fn root_re_exports_core_contracts() {
        let caps = SystemCapabilitiesResponse::for_edition(ProductEdition::Personal);
        assert_eq!(caps.edition, ProductEdition::Personal);

        let envelope = ErrorEnvelope::new("invalid_request", "bad request");
        assert_eq!(envelope.error.code, "invalid_request");

        let _event_type = DataPlaneSnapshotEventType::RoutingPlanRefresh;
        let req = ValidateApiKeyRequest {
            token: "token".to_string(),
        };
        assert_eq!(req.token, "token");
    }
}
