#[tokio::test]
async fn admin_model_routing_management_endpoints_work() {
    let app = build_app();
    let admin_token = login_admin_token(&app).await;

    let initial_settings_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/model-routing/settings")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(initial_settings_response.status(), StatusCode::OK);
    let initial_settings_body = to_bytes(initial_settings_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let initial_settings_json: Value = serde_json::from_slice(&initial_settings_body).unwrap();
    assert_eq!(
        initial_settings_json["settings"]["planner_model_chain"],
        json!([])
    );

    let list_profiles_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/model-routing/profiles")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_profiles_response.status(), StatusCode::OK);
    let list_profiles_body = to_bytes(list_profiles_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_profiles_json: Value = serde_json::from_slice(&list_profiles_body).unwrap();
    assert!(
        list_profiles_json
            .get("profiles")
            .map(Value::is_array)
            .unwrap_or(true)
    );

    let upsert_profile_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/model-routing/profiles")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "free-first",
                        "description": "Prefer free accounts for supported models.",
                        "enabled": true,
                        "priority": 100,
                        "selector": {
                            "plan_types": ["free"],
                            "modes": ["codex_oauth"]
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upsert_profile_response.status(), StatusCode::OK);
    let upsert_profile_body = to_bytes(upsert_profile_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let upsert_profile_json: Value = serde_json::from_slice(&upsert_profile_body).unwrap();
    let profile_id = upsert_profile_json["id"]
        .as_str()
        .expect("routing profile id");
    assert_eq!(upsert_profile_json["name"], "free-first");

    let upsert_policy_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/model-routing/model-policies")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "gpt-5 default",
                        "family": "gpt-5",
                        "exact_models": ["gpt-5.2-codex"],
                        "model_prefixes": ["gpt-5"],
                        "fallback_profile_ids": [profile_id],
                        "enabled": true,
                        "priority": 80
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upsert_policy_response.status(), StatusCode::OK);
    let upsert_policy_body = to_bytes(upsert_policy_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let upsert_policy_json: Value = serde_json::from_slice(&upsert_policy_body).unwrap();
    let policy_id = upsert_policy_json["id"].as_str().expect("policy id");
    assert_eq!(upsert_policy_json["family"], "gpt-5");

    let update_settings_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/admin/model-routing/settings")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enabled": true,
                        "auto_publish": true,
                        "planner_model_chain": ["gpt-5.2-codex", "gpt-4.1-mini"],
                        "trigger_mode": "hybrid",
                        "kill_switch": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_settings_response.status(), StatusCode::OK);
    let update_settings_body = to_bytes(update_settings_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let update_settings_json: Value = serde_json::from_slice(&update_settings_body).unwrap();
    assert_eq!(
        update_settings_json["settings"]["planner_model_chain"],
        json!(["gpt-5.2-codex", "gpt-4.1-mini"])
    );

    let list_policies_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/model-routing/model-policies")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_policies_response.status(), StatusCode::OK);
    let list_policies_body = to_bytes(list_policies_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_policies_json: Value = serde_json::from_slice(&list_policies_body).unwrap();
    assert_eq!(list_policies_json["policies"][0]["id"], policy_id);

    let list_versions_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/model-routing/versions")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_versions_response.status(), StatusCode::OK);
    let list_versions_body = to_bytes(list_versions_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_versions_json: Value = serde_json::from_slice(&list_versions_body).unwrap();
    assert!(
        list_versions_json
            .get("versions")
            .map(Value::is_array)
            .unwrap_or(true)
    );

    let delete_policy_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/admin/model-routing/model-policies/{policy_id}"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_policy_response.status(), StatusCode::NO_CONTENT);

    let delete_profile_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/v1/admin/model-routing/profiles/{profile_id}"
                ))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_profile_response.status(), StatusCode::NO_CONTENT);
}
