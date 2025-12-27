use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::sync::RwLock;

use crate::error::{Error, Result};

const DEFAULT_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_TOKEN_CACHE_FILE: &str = "token.json";
const EXPIRY_SKEW_SECONDS: u64 = 20;

#[derive(Debug, Clone)]
pub(crate) struct OAuthTokenProvider {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    token_uri: String,
    token_cache_path: PathBuf,
    http: HttpClient,
    token: Arc<RwLock<Option<CachedToken>>>,
}

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
    expires_in: u64,
}

impl CachedToken {
    fn is_expired(&self) -> bool {
        self.expires_at <= Instant::now() + Duration::from_secs(EXPIRY_SKEW_SECONDS)
    }
}

impl OAuthTokenProvider {
    pub(crate) fn from_paths(
        client_secret_path: impl AsRef<Path>,
        token_cache_path: Option<PathBuf>,
    ) -> Result<Self> {
        let client_secret = load_client_secret(client_secret_path.as_ref())?;
        let cache_path =
            token_cache_path.unwrap_or_else(|| PathBuf::from(DEFAULT_TOKEN_CACHE_FILE));
        let token_cache = load_token_cache(&cache_path)?;

        let refresh_token = token_cache
            .refresh_token
            .ok_or_else(|| Error::InvalidConfig {
                message: format!("token cache {} missing refresh_token", cache_path.display()),
            })?;

        if let Some(client_id) = token_cache.client_id.as_ref() {
            if client_id != &client_secret.client_id {
                return Err(Error::InvalidConfig {
                    message: "client_id in token.json does not match client_secret.json".into(),
                });
            }
        }
        if let Some(client_secret_value) = token_cache.client_secret.as_ref() {
            if client_secret_value != &client_secret.client_secret {
                return Err(Error::InvalidConfig {
                    message: "client_secret in token.json does not match client_secret.json".into(),
                });
            }
        }

        let client_id = token_cache
            .client_id
            .unwrap_or_else(|| client_secret.client_id.clone());
        let client_secret_value = token_cache
            .client_secret
            .unwrap_or_else(|| client_secret.client_secret.clone());
        let token_uri = token_cache
            .token_uri
            .or_else(|| client_secret.token_uri.clone())
            .unwrap_or_else(|| DEFAULT_TOKEN_URI.to_string());

        Ok(Self {
            client_id,
            client_secret: client_secret_value,
            refresh_token,
            token_uri,
            token_cache_path: cache_path,
            http: HttpClient::new(),
            token: Arc::new(RwLock::new(None)),
        })
    }

    pub(crate) async fn token(&self) -> Result<String> {
        if let Some(token) = self.token.read().await.as_ref() {
            if !token.is_expired() {
                return Ok(token.access_token.clone());
            }
        }

        let mut guard = self.token.write().await;
        if let Some(token) = guard.as_ref() {
            if !token.is_expired() {
                return Ok(token.access_token.clone());
            }
        }

        let refreshed = self.refresh_token().await?;
        *guard = Some(refreshed.clone());
        Ok(refreshed.access_token)
    }

    async fn refresh_token(&self) -> Result<CachedToken> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("refresh_token", self.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ];

        let response = self.http.post(&self.token_uri).form(&params).send().await?;

        if !response.status().is_success() {
            return Err(Error::Auth {
                message: format!(
                    "OAuth token refresh failed (status {}): {}",
                    response.status().as_u16(),
                    response.text().await.unwrap_or_default()
                ),
            });
        }

        let payload = response.json::<RefreshResponse>().await?;
        let token = CachedToken {
            access_token: payload.access_token,
            expires_at: Instant::now() + Duration::from_secs(payload.expires_in),
            expires_in: payload.expires_in,
        };
        self.update_token_cache(&token).await?;
        Ok(token)
    }

    async fn update_token_cache(&self, token: &CachedToken) -> Result<()> {
        let existing = tokio::fs::read_to_string(&self.token_cache_path).await;
        let mut value = match existing {
            Ok(content) => serde_json::from_str::<Value>(&content).unwrap_or(Value::Null),
            Err(_) => Value::Null,
        };

        let map = match &mut value {
            Value::Object(map) => map,
            _ => {
                value = Value::Object(Map::new());
                match &mut value {
                    Value::Object(map) => map,
                    _ => unreachable!("value just initialized to object"),
                }
            }
        };

        map.insert(
            "access_token".to_string(),
            Value::String(token.access_token.clone()),
        );
        map.insert(
            "token".to_string(),
            Value::String(token.access_token.clone()),
        );
        map.insert(
            "expires_in".to_string(),
            Value::Number(token.expires_in.into()),
        );
        map.entry("token_type".to_string())
            .or_insert_with(|| Value::String("Bearer".to_string()));
        map.entry("client_id".to_string())
            .or_insert_with(|| Value::String(self.client_id.clone()));
        map.entry("client_secret".to_string())
            .or_insert_with(|| Value::String(self.client_secret.clone()));
        map.entry("refresh_token".to_string())
            .or_insert_with(|| Value::String(self.refresh_token.clone()));
        map.entry("token_uri".to_string())
            .or_insert_with(|| Value::String(self.token_uri.clone()));

        let payload = serde_json::to_string_pretty(&value)?;
        tokio::fs::write(&self.token_cache_path, payload).await?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ClientSecretFile {
    installed: Option<ClientSecretInner>,
    web: Option<ClientSecretInner>,
}

#[derive(Debug, Deserialize)]
struct ClientSecretInner {
    client_id: String,
    client_secret: String,
    token_uri: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct TokenCacheFile {
    refresh_token: Option<String>,
    client_id: Option<String>,
    client_secret: Option<String>,
    token_uri: Option<String>,
    #[serde(default)]
    quota_project_id: Option<String>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    #[serde(default)]
    expires_in: u64,
}

fn load_client_secret(path: &Path) -> Result<ClientSecretInner> {
    let content = std::fs::read_to_string(path).map_err(|err| Error::InvalidConfig {
        message: format!(
            "Failed to read client_secret.json {}: {err}",
            path.display()
        ),
    })?;
    let parsed: ClientSecretFile =
        serde_json::from_str(&content).map_err(|err| Error::InvalidConfig {
            message: format!(
                "Failed to parse client_secret.json {}: {err}",
                path.display()
            ),
        })?;
    if let Some(installed) = parsed.installed {
        Ok(installed)
    } else if let Some(web) = parsed.web {
        Ok(web)
    } else {
        Err(Error::InvalidConfig {
            message: "client_secret.json must include `installed` or `web` section".into(),
        })
    }
}

fn load_token_cache(path: &Path) -> Result<TokenCacheFile> {
    let content = std::fs::read_to_string(path).map_err(|err| Error::InvalidConfig {
        message: format!(
            "Failed to read token cache {}: {err}. Please generate token.json first.",
            path.display()
        ),
    })?;
    serde_json::from_str(&content).map_err(|err| Error::InvalidConfig {
        message: format!("Failed to parse token cache {}: {err}", path.display()),
    })
}
