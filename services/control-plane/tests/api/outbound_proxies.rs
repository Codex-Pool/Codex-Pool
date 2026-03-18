#[tokio::test]
async fn admin_proxy_pool_crud_and_settings_flow_masks_proxy_url() {
    let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
    let app = build_app_with_store(store);
    let admin_token = login_admin_token(&app).await;

    let initial_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/proxies")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(initial_response.status(), StatusCode::OK);
    let initial_body = to_bytes(initial_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let initial_json: Value = serde_json::from_slice(&initial_body).unwrap();
    assert_eq!(initial_json["settings"]["enabled"], false);
    assert_eq!(initial_json["settings"]["fail_mode"], "strict_proxy");
    assert_eq!(initial_json["nodes"], json!([]));

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/proxies")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "label": "Surge local",
                        "proxy_url": "http://alice:secret@127.0.0.1:6152",
                        "enabled": true,
                        "weight": 3,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_response.status(), StatusCode::OK);
    let create_body = to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let create_json: Value = serde_json::from_slice(&create_body).unwrap();
    let proxy_id = create_json["node"]["id"]
        .as_str()
        .expect("proxy id")
        .to_string();
    assert_eq!(create_json["node"]["scheme"], "http");
    assert_eq!(create_json["node"]["has_auth"], true);
    assert_eq!(
        create_json["node"]["proxy_url_masked"],
        "http://alice:***@127.0.0.1:6152"
    );

    let update_settings_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/admin/proxies/settings")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enabled": true,
                        "fail_mode": "allow_direct_fallback"
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
    assert_eq!(update_settings_json["settings"]["enabled"], true);
    assert_eq!(
        update_settings_json["settings"]["fail_mode"],
        "allow_direct_fallback"
    );

    let update_node_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/admin/proxies/{proxy_id}"))
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "label": "Surge backup",
                        "enabled": false,
                        "weight": 7
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_node_response.status(), StatusCode::OK);
    let update_node_body = to_bytes(update_node_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let update_node_json: Value = serde_json::from_slice(&update_node_body).unwrap();
    assert_eq!(update_node_json["node"]["label"], "Surge backup");
    assert_eq!(update_node_json["node"]["enabled"], false);
    assert_eq!(update_node_json["node"]["weight"], 7);
    assert_eq!(
        update_node_json["node"]["proxy_url_masked"],
        "http://alice:***@127.0.0.1:6152"
    );

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/proxies")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_response.status(), StatusCode::OK);
    let list_body = to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let list_json: Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_json["settings"]["enabled"], true);
    assert_eq!(
        list_json["settings"]["fail_mode"],
        "allow_direct_fallback"
    );
    assert_eq!(list_json["nodes"][0]["id"], proxy_id);
    assert_eq!(list_json["nodes"][0]["label"], "Surge backup");
    assert_eq!(list_json["nodes"][0]["enabled"], false);
    assert_eq!(list_json["nodes"][0]["weight"], 7);
    assert_eq!(
        list_json["nodes"][0]["proxy_url_masked"],
        "http://alice:***@127.0.0.1:6152"
    );

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/admin/proxies/{proxy_id}"))
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let final_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/proxies")
                .header("authorization", format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(final_response.status(), StatusCode::OK);
    let final_body = to_bytes(final_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let final_json: Value = serde_json::from_slice(&final_body).unwrap();
    assert_eq!(final_json["nodes"], json!([]));
}

#[tokio::test]
async fn admin_proxy_pool_rejects_invalid_proxy_scheme() {
    let store: Arc<dyn ControlPlaneStore> = Arc::new(InMemoryStore::default());
    let app = build_app_with_store(store);
    let admin_token = login_admin_token(&app).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/admin/proxies")
                .header("authorization", format!("Bearer {admin_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "label": "Bad proxy",
                        "proxy_url": "ftp://127.0.0.1:21"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["error"]["code"], "invalid_proxy_url");
}
