use futures_util::StreamExt;

pub(crate) const OPENAI_MODELS_INDEX_URL: &str = "https://developers.openai.com/api/docs/models";

fn strip_html_to_text(raw_html: &str) -> String {
    let mut text = raw_html.to_string();
    for (start, end) in [("<script", "</script>"), ("<style", "</style>")] {
        loop {
            let Some(start_idx) = text.find(start) else { break; };
            let Some(rel_end_idx) = text[start_idx..].find(end) else {
                text.truncate(start_idx);
                break;
            };
            let end_idx = start_idx + rel_end_idx + end.len();
            text.replace_range(start_idx..end_idx, " ");
        }
    }
    let mut stripped = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                stripped.push(' ');
            }
            _ if !in_tag => stripped.push(ch),
            _ => {}
        }
    }
    stripped
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_meta_description(raw_html: &str) -> Option<String> {
    let marker = "name=\"description\" content=\"";
    let start = raw_html.find(marker)? + marker.len();
    let rest = &raw_html[start..];
    let end = rest.find('"')?;
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn extract_title(raw_html: &str, fallback_model_id: &str) -> String {
    if let Some(start) = raw_html.find("<title>") {
        let rest = &raw_html[start + "<title>".len()..];
        if let Some(end) = rest.find("</title>") {
            let title = rest[..end]
                .trim()
                .trim_end_matches("| OpenAI API")
                .trim()
                .to_string();
            if !title.is_empty() {
                return title;
            }
        }
    }
    fallback_model_id.to_string()
}

fn parse_money_to_microcredits(raw: &str) -> Option<i64> {
    let numeric = raw.trim().trim_start_matches('$').replace(',', "");
    if numeric.is_empty() {
        return None;
    }
    let value: f64 = numeric.parse().ok()?;
    Some((value * 1_000_000.0).round() as i64)
}

fn parse_optional_i64_from_capture(raw: Option<&str>) -> Option<i64> {
    raw.map(|value| value.replace(',', "")).and_then(|value| value.parse::<i64>().ok())
}

fn parse_modalities(section: &str) -> (Vec<String>, Vec<String>) {
    let mut input = Vec::new();
    let mut output = Vec::new();
    for (label, key) in [("Text", "text"), ("Image", "image"), ("Audio", "audio"), ("Video", "video")] {
        let lower = section.to_ascii_lowercase();
        let label_pos = lower.find(&label.to_ascii_lowercase());
        let Some(label_pos) = label_pos else { continue; };
        let tail = &section[label_pos + label.len()..];
        let status = if tail.starts_with(" Input and output") {
            "input_output"
        } else if tail.starts_with(" Input only") {
            "input_only"
        } else if tail.starts_with(" Output only") {
            "output_only"
        } else {
            "unsupported"
        };
        match status {
            "input_output" => {
                input.push(key.to_string());
                output.push(key.to_string());
            }
            "input_only" => input.push(key.to_string()),
            "output_only" => output.push(key.to_string()),
            _ => {}
        }
    }
    (input, output)
}

fn parse_endpoints(text: &str) -> Vec<String> {
    let Some(start) = text.find("Endpoints ") else { return Vec::new(); };
    let tail = &text[start + "Endpoints ".len()..];
    let end = ["Pricing tier", "Snapshots", "Latest model alias", "Use cases", "Strengths", "Limitations"]
        .iter()
        .filter_map(|marker| tail.find(marker))
        .min()
        .unwrap_or(tail.len());
    let section = &tail[..end];
    let regex = regex::Regex::new(r"v1/[A-Za-z0-9_./-]+").expect("valid endpoint regex");
    let mut endpoints = regex
        .find_iter(section)
        .map(|m| m.as_str().to_string())
        .collect::<Vec<_>>();
    endpoints.sort();
    endpoints.dedup();
    endpoints
}

pub(crate) fn build_catalog_item_from_model_page(
    model_id: &str,
    raw_html: &str,
    synced_at: DateTime<Utc>,
) -> Result<OpenAiModelCatalogItem> {
    let text = strip_html_to_text(raw_html);
    let price_regex = regex::Regex::new(
        r"Input \$([0-9][0-9.,]*) Cached input \$([0-9][0-9.,]*) Output \$([0-9][0-9.,]*)"
    )
    .expect("valid price regex");
    let context_regex =
        regex::Regex::new(r"([0-9,]+) context window").expect("valid context regex");
    let max_output_regex =
        regex::Regex::new(r"([0-9,]+) max output tokens").expect("valid max output regex");
    let cutoff_regex = regex::Regex::new(r"([A-Z][a-z]{2} \d{1,2}, \d{4}) knowledge cutoff")
        .expect("valid cutoff regex");
    let price_match = price_regex.captures(&text);
    let modalities_section = text
        .split("Modalities ")
        .nth(1)
        .and_then(|tail| tail.split(" Endpoints ").next())
        .unwrap_or_default();
    let (input_modalities, output_modalities) = parse_modalities(modalities_section);
    let reasoning_token_support = if text.contains("Reasoning token support") {
        Some(true)
    } else {
        None
    };

    Ok(OpenAiModelCatalogItem {
        model_id: model_id.to_string(),
        owned_by: "openai".to_string(),
        title: extract_title(raw_html, model_id),
        description: extract_meta_description(raw_html),
        context_window_tokens: parse_optional_i64_from_capture(
            context_regex.captures(&text).and_then(|caps| caps.get(1)).map(|m| m.as_str()),
        ),
        max_output_tokens: parse_optional_i64_from_capture(
            max_output_regex.captures(&text).and_then(|caps| caps.get(1)).map(|m| m.as_str()),
        ),
        knowledge_cutoff: cutoff_regex
            .captures(&text)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string()),
        reasoning_token_support,
        input_price_microcredits: price_match
            .as_ref()
            .and_then(|caps| caps.get(1))
            .and_then(|m| parse_money_to_microcredits(m.as_str())),
        cached_input_price_microcredits: price_match
            .as_ref()
            .and_then(|caps| caps.get(2))
            .and_then(|m| parse_money_to_microcredits(m.as_str())),
        output_price_microcredits: price_match
            .as_ref()
            .and_then(|caps| caps.get(3))
            .and_then(|m| parse_money_to_microcredits(m.as_str())),
        pricing_notes: text
            .split("Pricing ")
            .nth(1)
            .and_then(|tail| tail.split(" Text tokens ").next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        input_modalities,
        output_modalities,
        endpoints: parse_endpoints(&text),
        source_url: format!("{OPENAI_MODELS_INDEX_URL}/{model_id}"),
        raw_text: Some(text),
        synced_at,
    })
}

pub(crate) fn parse_model_ids_from_models_index(raw_html: &str) -> Vec<String> {
    let regex = regex::Regex::new(r#"/api/docs/models/([A-Za-z0-9_.-]+)"#)
        .expect("valid model link regex");
    let mut ids = regex
        .captures_iter(raw_html)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

async fn fetch_openai_docs_html(client: &reqwest::Client, url: &str) -> Result<String> {
    client
        .get(url)
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0 Codex-Pool/1.0")
        .send()
        .await
        .with_context(|| format!("failed to fetch {url}"))?
        .error_for_status()
        .with_context(|| format!("upstream returned non-success for {url}"))?
        .text()
        .await
        .with_context(|| format!("failed to decode html from {url}"))
}

pub(crate) async fn fetch_openai_model_catalog_items_with_client(
    client: reqwest::Client,
) -> Result<(DateTime<Utc>, Vec<OpenAiModelCatalogItem>)> {
    let synced_at = Utc::now();
    let index_html = fetch_openai_docs_html(&client, OPENAI_MODELS_INDEX_URL).await?;
    let model_ids = parse_model_ids_from_models_index(&index_html);
    if model_ids.is_empty() {
        return Err(anyhow!("official models index returned no model ids"));
    }

    let pages = futures_util::stream::iter(model_ids.into_iter().map(|model_id| {
        let client = client.clone();
        async move {
            let url = format!("{OPENAI_MODELS_INDEX_URL}/{model_id}");
            let html = fetch_openai_docs_html(&client, &url).await?;
            build_catalog_item_from_model_page(&model_id, &html, synced_at)
        }
    }))
    .buffer_unordered(8)
    .collect::<Vec<_>>()
    .await;

    let mut items = Vec::new();
    for page in pages {
        items.push(page?);
    }
    items.sort_by(|left, right| left.model_id.cmp(&right.model_id));
    Ok((synced_at, items))
}

pub(crate) async fn fetch_openai_model_catalog_items(
) -> Result<(DateTime<Utc>, Vec<OpenAiModelCatalogItem>)> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .context("failed to build openai catalog sync client")?;
    fetch_openai_model_catalog_items_with_client(client).await
}

impl TenantAuthService {
    async fn apply_openai_model_catalog_items(
        &self,
        synced_at: DateTime<Utc>,
        items: &[OpenAiModelCatalogItem],
    ) -> Result<OpenAiModelsSyncResponse> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start openai catalog sync transaction")?;

        for item in items {
            sqlx::query(
                r#"
                INSERT INTO openai_models_catalog (
                    model_id,
                    owned_by,
                    title,
                    description,
                    context_window_tokens,
                    max_output_tokens,
                    knowledge_cutoff,
                    reasoning_token_support,
                    input_price_microcredits,
                    cached_input_price_microcredits,
                    output_price_microcredits,
                    pricing_notes,
                    input_modalities_json,
                    output_modalities_json,
                    endpoints_json,
                    source_url,
                    raw_text,
                    synced_at
                )
                VALUES (
                    $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18
                )
                ON CONFLICT (model_id) DO UPDATE SET
                    owned_by = EXCLUDED.owned_by,
                    title = EXCLUDED.title,
                    description = EXCLUDED.description,
                    context_window_tokens = EXCLUDED.context_window_tokens,
                    max_output_tokens = EXCLUDED.max_output_tokens,
                    knowledge_cutoff = EXCLUDED.knowledge_cutoff,
                    reasoning_token_support = EXCLUDED.reasoning_token_support,
                    input_price_microcredits = EXCLUDED.input_price_microcredits,
                    cached_input_price_microcredits = EXCLUDED.cached_input_price_microcredits,
                    output_price_microcredits = EXCLUDED.output_price_microcredits,
                    pricing_notes = EXCLUDED.pricing_notes,
                    input_modalities_json = EXCLUDED.input_modalities_json,
                    output_modalities_json = EXCLUDED.output_modalities_json,
                    endpoints_json = EXCLUDED.endpoints_json,
                    source_url = EXCLUDED.source_url,
                    raw_text = EXCLUDED.raw_text,
                    synced_at = EXCLUDED.synced_at
                "#,
            )
            .bind(&item.model_id)
            .bind(&item.owned_by)
            .bind(&item.title)
            .bind(&item.description)
            .bind(item.context_window_tokens)
            .bind(item.max_output_tokens)
            .bind(&item.knowledge_cutoff)
            .bind(item.reasoning_token_support)
            .bind(item.input_price_microcredits)
            .bind(item.cached_input_price_microcredits)
            .bind(item.output_price_microcredits)
            .bind(&item.pricing_notes)
            .bind(sqlx_core::types::Json(&item.input_modalities))
            .bind(sqlx_core::types::Json(&item.output_modalities))
            .bind(sqlx_core::types::Json(&item.endpoints))
            .bind(&item.source_url)
            .bind(&item.raw_text)
            .bind(item.synced_at)
            .execute(tx.as_mut())
            .await
            .with_context(|| format!("failed to upsert official model {}", item.model_id))?;
        }

        let model_ids_array = items.iter().map(|item| item.model_id.clone()).collect::<Vec<_>>();
        let deleted_catalog_rows = sqlx::query(
            r#"DELETE FROM openai_models_catalog WHERE NOT (model_id = ANY($1))"#,
        )
        .bind(&model_ids_array)
        .execute(tx.as_mut())
        .await
        .context("failed to delete removed official catalog rows")?
        .rows_affected() as usize;

        let cleared_custom_entities = sqlx::query("DELETE FROM admin_model_entities")
            .execute(tx.as_mut())
            .await
            .context("failed to clear admin_model_entities during official sync")?
            .rows_affected();

        let cleared_billing_rules = sqlx::query("DELETE FROM billing_pricing_rules")
            .execute(tx.as_mut())
            .await
            .context("failed to clear billing_pricing_rules during official sync")?
            .rows_affected();

        let deleted_legacy_pricing_rows = sqlx::query(
            r#"
            DELETE FROM model_pricing
            WHERE model LIKE '%*'
               OR NOT (model = ANY($1))
            "#,
        )
        .bind(&model_ids_array)
        .execute(tx.as_mut())
        .await
        .context("failed to clean legacy non-official pricing rows")?
        .rows_affected();

        tx.commit()
            .await
            .context("failed to commit openai catalog sync transaction")?;

        Ok(OpenAiModelsSyncResponse {
            models_total: items.len(),
            created_or_updated: items.len(),
            deleted_catalog_rows,
            cleared_custom_entities,
            cleared_billing_rules,
            deleted_legacy_pricing_rows,
            synced_at,
        })
    }

    pub async fn admin_list_openai_model_catalog(&self) -> Result<Vec<OpenAiModelCatalogItem>> {
        let rows = sqlx::query(
            r#"
            SELECT
                model_id,
                owned_by,
                title,
                description,
                context_window_tokens,
                max_output_tokens,
                knowledge_cutoff,
                reasoning_token_support,
                input_price_microcredits,
                cached_input_price_microcredits,
                output_price_microcredits,
                pricing_notes,
                input_modalities_json,
                output_modalities_json,
                endpoints_json,
                source_url,
                raw_text,
                synced_at
            FROM openai_models_catalog
            ORDER BY model_id ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("failed to list openai models catalog")?;

        rows.into_iter()
            .map(|row| -> Result<OpenAiModelCatalogItem> {
                Ok(OpenAiModelCatalogItem {
                    model_id: row.try_get("model_id")?,
                    owned_by: row.try_get("owned_by")?,
                    title: row.try_get("title")?,
                    description: row.try_get("description")?,
                    context_window_tokens: row.try_get("context_window_tokens")?,
                    max_output_tokens: row.try_get("max_output_tokens")?,
                    knowledge_cutoff: row.try_get("knowledge_cutoff")?,
                    reasoning_token_support: row.try_get("reasoning_token_support")?,
                    input_price_microcredits: row.try_get("input_price_microcredits")?,
                    cached_input_price_microcredits: row.try_get("cached_input_price_microcredits")?,
                    output_price_microcredits: row.try_get("output_price_microcredits")?,
                    pricing_notes: row.try_get("pricing_notes")?,
                    input_modalities: row
                        .try_get::<sqlx_core::types::Json<Vec<String>>, _>("input_modalities_json")?
                        .0,
                    output_modalities: row
                        .try_get::<sqlx_core::types::Json<Vec<String>>, _>("output_modalities_json")?
                        .0,
                    endpoints: row
                        .try_get::<sqlx_core::types::Json<Vec<String>>, _>("endpoints_json")?
                        .0,
                    source_url: row.try_get("source_url")?,
                    raw_text: row.try_get("raw_text")?,
                    synced_at: row.try_get("synced_at")?,
                })
            })
            .collect()
    }

    pub async fn admin_sync_openai_models_catalog(&self) -> Result<OpenAiModelsSyncResponse> {
        self.admin_sync_openai_models_catalog_with_client(None).await
    }

    pub async fn admin_sync_openai_models_catalog_with_client(
        &self,
        client: Option<reqwest::Client>,
    ) -> Result<OpenAiModelsSyncResponse> {
        let (synced_at, items) = match client {
            Some(client) => fetch_openai_model_catalog_items_with_client(client).await?,
            None => fetch_openai_model_catalog_items().await?,
        };
        self.apply_openai_model_catalog_items(synced_at, &items).await
    }
}

#[cfg(test)]
mod openai_catalog_sync_tests {
    use super::{build_catalog_item_from_model_page, parse_model_ids_from_models_index};
    use chrono::Utc;

    #[test]
    fn parses_model_ids_from_index_html() {
        let html = r#"<a href="/api/docs/models/gpt-5.1">GPT-5.1</a><a href="/api/docs/models/gpt-4.1">GPT-4.1</a>"#;
        let ids = parse_model_ids_from_models_index(html);
        assert_eq!(ids, vec!["gpt-4.1".to_string(), "gpt-5.1".to_string()]);
    }

    #[test]
    fn parses_model_page_visible_text_fields() {
        let html = r#"
        <html>
            <head>
                <title>GPT-5.1 Model | OpenAI API</title>
                <meta name="description" content="Great for coding.">
            </head>
            <body>
                <div>400,000 context window</div>
                <div>128,000 max output tokens</div>
                <div>Sep 30, 2024 knowledge cutoff</div>
                <div>Reasoning token support</div>
                <div>Pricing Pricing is based on tokens. Text tokens Per 1M tokens Input $1.25 Cached input $0.125 Output $10.00</div>
                <div>Modalities Text Input and output Image Input only Audio Not supported Video Not supported Endpoints Chat Completions v1/chat/completions Responses v1/responses</div>
            </body>
        </html>
        "#;
        let item = build_catalog_item_from_model_page("gpt-5.1", html, Utc::now()).expect("parse model page");
        assert_eq!(item.model_id, "gpt-5.1");
        assert_eq!(item.title, "GPT-5.1 Model");
        assert_eq!(item.description.as_deref(), Some("Great for coding."));
        assert_eq!(item.context_window_tokens, Some(400_000));
        assert_eq!(item.max_output_tokens, Some(128_000));
        assert_eq!(item.knowledge_cutoff.as_deref(), Some("Sep 30, 2024"));
        assert_eq!(item.reasoning_token_support, Some(true));
        assert_eq!(item.input_price_microcredits, Some(1_250_000));
        assert_eq!(item.cached_input_price_microcredits, Some(125_000));
        assert_eq!(item.output_price_microcredits, Some(10_000_000));
        assert_eq!(item.input_modalities, vec!["text".to_string(), "image".to_string()]);
        assert_eq!(item.output_modalities, vec!["text".to_string()]);
        assert_eq!(item.endpoints, vec!["v1/chat/completions".to_string(), "v1/responses".to_string()]);
    }
}
