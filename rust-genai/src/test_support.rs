use crate::client::{
    ApiClient, Backend, ClientConfig, ClientInner, Credentials, HttpOptions, VertexConfig,
};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce()) {
    let _guard = ENV_LOCK.lock().unwrap();
    let backup: Vec<(String, Option<String>)> = vars
        .iter()
        .map(|(key, _)| ((*key).to_string(), std::env::var(key).ok()))
        .collect();
    for (key, value) in vars {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    f();
    for (key, value) in backup {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
}

pub fn test_client_inner(backend: Backend) -> ClientInner {
    test_client_inner_with_api_key(backend, Some("test-key"))
}

pub fn test_client_inner_with_api_key(backend: Backend, api_key: Option<&str>) -> ClientInner {
    let vertex_config = if backend == Backend::VertexAi {
        Some(VertexConfig {
            project: "proj".to_string(),
            location: "loc".to_string(),
            credentials: None,
        })
    } else {
        None
    };
    let config = ClientConfig {
        api_key: api_key.map(ToString::to_string),
        backend,
        vertex_config,
        http_options: HttpOptions::default(),
        credentials: Credentials::ApiKey("test-key".into()),
        auth_scopes: Vec::new(),
    };
    let api_client = ApiClient::new(&config);
    ClientInner {
        http: reqwest::Client::new(),
        config,
        api_client,
        auth_provider: None,
    }
}

pub fn test_client_inner_with_base(
    backend: Backend,
    base_url: &str,
    api_version: &str,
) -> ClientInner {
    let vertex_config = if backend == Backend::VertexAi {
        Some(VertexConfig {
            project: "proj".to_string(),
            location: "loc".to_string(),
            credentials: None,
        })
    } else {
        None
    };
    let http_options = HttpOptions {
        base_url: Some(base_url.to_string()),
        api_version: Some(api_version.to_string()),
        ..Default::default()
    };
    let config = ClientConfig {
        api_key: Some("test-key".into()),
        backend,
        vertex_config,
        http_options,
        credentials: Credentials::ApiKey("test-key".into()),
        auth_scopes: Vec::new(),
    };
    let api_client = ApiClient::new(&config);
    ClientInner {
        http: reqwest::Client::new(),
        config,
        api_client,
        auth_provider: None,
    }
}

pub fn test_vertex_inner_missing_config() -> ClientInner {
    let config = ClientConfig {
        api_key: None,
        backend: Backend::VertexAi,
        vertex_config: None,
        http_options: HttpOptions::default(),
        credentials: Credentials::ApplicationDefault,
        auth_scopes: Vec::new(),
    };
    let api_client = ApiClient::new(&config);
    ClientInner {
        http: reqwest::Client::new(),
        config,
        api_client,
        auth_provider: None,
    }
}
