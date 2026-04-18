use serde_json::json;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use rust_genai::types::webhooks::{
    CreateWebhookConfig, ListWebhooksConfig, RotateWebhookSigningSecretConfig, UpdateWebhookConfig,
};

mod support;
use support::build_gemini_client_with_version;

#[tokio::test]
async fn webhooks_api_flow() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1beta/webhooks"))
        .and(query_param("webhook_id", "hook-1"))
        .and(body_json(json!({
            "subscribed_events": ["batch.succeeded", "interaction.completed"],
            "uri": "https://example.com/webhook",
            "state": "enabled"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "webhooks/hook-1",
            "uri": "https://example.com/webhook",
            "subscribed_events": ["batch.succeeded", "interaction.completed"],
            "state": "enabled",
            "new_signing_secret": "secret-1"
        })))
        .mount(&server)
        .await;

    Mock::given(method("PATCH"))
        .and(path("/v1beta/webhooks/hook-1"))
        .and(query_param("update_mask", "uri,subscribed_events,state"))
        .and(body_json(json!({
            "subscribed_events": ["interaction.completed"],
            "uri": "https://example.com/webhook-updated",
            "state": "disabled"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "webhooks/hook-1",
            "uri": "https://example.com/webhook-updated",
            "subscribed_events": ["interaction.completed"],
            "state": "disabled"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/webhooks"))
        .and(query_param("page_size", "5"))
        .and(query_param("page_token", "page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "webhooks": [
                {
                    "name": "webhooks/hook-1",
                    "uri": "https://example.com/webhook-updated",
                    "subscribed_events": ["interaction.completed"],
                    "state": "disabled"
                }
            ],
            "next_page_token": "page-3"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1beta/webhooks/hook-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "webhooks/hook-1",
            "uri": "https://example.com/webhook-updated",
            "subscribed_events": ["interaction.completed"],
            "state": "disabled"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/webhooks/hook-1:ping"))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1beta/webhooks/hook-1:rotateSigningSecret"))
        .and(body_json(json!({
            "revocation_behavior": "revoke_previous_secrets_immediately"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "secret": "secret-2"
        })))
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1beta/webhooks/hook-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;

    let client = build_gemini_client_with_version(&server.uri(), "v1beta");
    let webhooks = client.webhooks();

    let created = webhooks
        .create(CreateWebhookConfig {
            webhook_id: Some("hook-1".to_string()),
            state: Some("enabled".to_string()),
            ..CreateWebhookConfig::new(
                "https://example.com/webhook",
                vec![
                    "batch.succeeded".to_string(),
                    "interaction.completed".to_string(),
                ],
            )
        })
        .await
        .unwrap();
    assert_eq!(created.name.as_deref(), Some("webhooks/hook-1"));
    assert_eq!(created.new_signing_secret.as_deref(), Some("secret-1"));

    let updated = webhooks
        .update(
            "hook-1",
            UpdateWebhookConfig {
                update_mask: Some("uri,subscribed_events,state".to_string()),
                state: Some("disabled".to_string()),
                ..UpdateWebhookConfig::new(
                    "https://example.com/webhook-updated",
                    vec!["interaction.completed".to_string()],
                )
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.state.as_deref(), Some("disabled"));

    let listed = webhooks
        .list_with_config(ListWebhooksConfig {
            page_size: Some(5),
            page_token: Some("page-2".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(listed.webhooks.as_ref().map(Vec::len), Some(1));
    assert_eq!(listed.next_page_token.as_deref(), Some("page-3"));

    let got = webhooks.get("hook-1").await.unwrap();
    assert_eq!(
        got.uri.as_deref(),
        Some("https://example.com/webhook-updated")
    );

    webhooks.ping("webhooks/hook-1").await.unwrap();

    let rotated = webhooks
        .rotate_signing_secret_with_config(
            "hook-1",
            RotateWebhookSigningSecretConfig {
                revocation_behavior: Some("revoke_previous_secrets_immediately".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(rotated.secret.as_deref(), Some("secret-2"));

    webhooks.delete("hook-1").await.unwrap();
}
