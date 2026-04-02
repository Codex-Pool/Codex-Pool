use http_body_util::BodyExt;

pub async fn anthropic_messages_handler(
    State(state): State<std::sync::Arc<AppState>>,
    request: Request<Body>,
) -> Response {
    let principal = request.extensions().get::<ApiPrincipal>().cloned();
    let (mut parts, body) = request.into_parts();
    let header_locale = detect_request_locale(&parts.headers, &bytes::Bytes::new());
    let max_request_body_bytes =
        max_request_body_bytes_for_path(state.max_request_body_bytes, "/v1/messages");
    let body_bytes = match axum::body::to_bytes(body, max_request_body_bytes).await {
        Ok(body) => body,
        Err(_) => {
            return anthropic_json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "failed to read request body",
            );
        }
    };
    let request_value = match serde_json::from_slice::<Value>(&body_bytes) {
        Ok(value) => value,
        Err(_) => {
            return anthropic_json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "request body must be valid JSON",
            );
        }
    };
    let requested_model = match request_value
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(model) => model.to_string(),
        None => {
            return anthropic_json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "model is required",
            );
        }
    };
    let target_model = match resolve_claude_code_target_model(state.as_ref(), &requested_model) {
        Ok(model) => model,
        Err(response) => return response,
    };
    let translated_request =
        match translate_anthropic_messages_request(&request_value, target_model.as_str()) {
            Ok(value) => value,
            Err(response) => return response,
        };

    parts.method = axum::http::Method::POST;
    parts.uri = axum::http::Uri::from_static("/v1/responses");
    parts.headers.remove("x-api-key");
    parts.headers.remove("anthropic-version");
    parts.headers.remove("anthropic-beta");
    parts.headers.remove(axum::http::header::CONTENT_LENGTH);
    parts.headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let mut internal_request = Request::from_parts(parts, Body::from(translated_request.to_string()));
    if let Some(principal) = principal {
        internal_request.extensions_mut().insert(principal);
    }

    let response = proxy_handler(State(state.clone()), internal_request).await;
    translate_proxy_response_to_anthropic(response, requested_model, header_locale.as_str()).await
}

pub async fn anthropic_count_tokens_handler(
    State(state): State<std::sync::Arc<AppState>>,
    request: Request<Body>,
) -> Response {
    let (_, body) = request.into_parts();
    let max_request_body_bytes =
        max_request_body_bytes_for_path(state.max_request_body_bytes, "/v1/messages/count_tokens");
    let body_bytes = match axum::body::to_bytes(body, max_request_body_bytes).await {
        Ok(body) => body,
        Err(_) => {
            return anthropic_json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "failed to read request body",
            );
        }
    };
    let request_value = match serde_json::from_slice::<Value>(&body_bytes) {
        Ok(value) => value,
        Err(_) => {
            return anthropic_json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "request body must be valid JSON",
            );
        }
    };
    let requested_model = match request_value
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(model) => model.to_string(),
        None => {
            return anthropic_json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "model is required",
            );
        }
    };
    let target_model = match resolve_claude_code_target_model(state.as_ref(), &requested_model) {
        Ok(model) => model,
        Err(response) => return response,
    };
    let translated_request =
        match translate_anthropic_messages_request(&request_value, target_model.as_str()) {
            Ok(value) => value,
            Err(response) => return response,
        };
    let input_tokens = estimate_request_input_tokens(&translated_request)
        .unwrap_or(0)
        .max(0);

    anthropic_json_response(
        StatusCode::OK,
        &serde_json::json!({
            "input_tokens": input_tokens
        }),
        None,
    )
}

async fn translate_proxy_response_to_anthropic(
    response: Response,
    requested_model: String,
    _locale: &str,
) -> Response {
    let status = response.status();
    let headers = response.headers().clone();
    let is_event_stream = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("text/event-stream"));
    let request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok().map(|_| value.clone()));
    let (_, body) = response.into_parts();

    if !status.is_success() {
        let body_bytes = match body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => bytes::Bytes::new(),
        };
        return anthropic_error_from_proxy(status, &body_bytes, request_id);
    }

    if is_event_stream {
        return translate_proxy_stream_to_anthropic(body, requested_model, request_id);
    }

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            return anthropic_json_error(
                StatusCode::BAD_GATEWAY,
                "api_error",
                "failed to read upstream response body",
            );
        }
    };
    let body_value = match serde_json::from_slice::<Value>(&body_bytes) {
        Ok(value) => value,
        Err(_) => {
            return anthropic_json_error(
                StatusCode::BAD_GATEWAY,
                "api_error",
                "upstream response body was not valid JSON",
            );
        }
    };
    let translated = translate_responses_json_to_anthropic_message(&body_value, &requested_model);
    anthropic_json_response(StatusCode::OK, &translated, request_id)
}

fn translate_proxy_stream_to_anthropic(
    body: Body,
    requested_model: String,
    request_id: Option<HeaderValue>,
) -> Response {
    let (tx, rx) = mpsc::channel::<Result<Bytes, Infallible>>(16);
    tokio::spawn(async move {
        let mut translator = AnthropicSseTranslator::new(requested_model);
        let mut buffer = Vec::new();
        let mut stream = body.into_data_stream();

        while let Some(item) = stream.next().await {
            let chunk = match item {
                Ok(chunk) => chunk,
                Err(_) => return,
            };
            buffer.extend_from_slice(&chunk);
            while let Some(frame_end) = find_sse_frame_end(&buffer) {
                let frame = buffer.drain(..frame_end).collect::<Vec<_>>();
                let Some((_, payload)) = parse_sse_frame(&frame) else {
                    continue;
                };
                for translated_frame in translator.translate_frame(&payload) {
                    if tx.send(Ok(translated_frame)).await.is_err() {
                        return;
                    }
                }
            }
        }
    });

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "text/event-stream")
        .header(axum::http::header::CACHE_CONTROL, "no-cache");
    if let Some(request_id) = request_id.as_ref() {
        builder = builder.header("x-request-id", request_id);
    }
    builder
        .body(Body::from_stream(ReceiverStream::new(rx)))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn anthropic_error_from_proxy(
    status: StatusCode,
    body_bytes: &Bytes,
    request_id: Option<HeaderValue>,
) -> Response {
    let message = serde_json::from_slice::<Value>(body_bytes)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "request failed".to_string());
    anthropic_json_response(
        status,
        &serde_json::json!({
            "type": "error",
            "error": {
                "type": anthropic_error_type_for_status(status),
                "message": message,
            }
        }),
        request_id,
    )
}

fn anthropic_json_error(
    status: StatusCode,
    error_type: &'static str,
    message: &'static str,
) -> Response {
    anthropic_json_response(
        status,
        &serde_json::json!({
            "type": "error",
            "error": {
                "type": error_type,
                "message": message,
            }
        }),
        None,
    )
}

fn anthropic_json_response(
    status: StatusCode,
    payload: &Value,
    request_id: Option<HeaderValue>,
) -> Response {
    let mut builder = Response::builder()
        .status(status)
        .header(axum::http::header::CONTENT_TYPE, "application/json");
    if let Some(request_id) = request_id.as_ref() {
        builder = builder.header("x-request-id", request_id);
    }
    builder
        .body(Body::from(payload.to_string()))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn anthropic_error_type_for_status(status: StatusCode) -> &'static str {
    match status {
        StatusCode::BAD_REQUEST => "invalid_request_error",
        StatusCode::UNAUTHORIZED => "authentication_error",
        StatusCode::FORBIDDEN => "permission_error",
        StatusCode::TOO_MANY_REQUESTS => "rate_limit_error",
        _ => "api_error",
    }
}
