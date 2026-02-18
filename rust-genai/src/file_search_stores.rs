//! `FileSearchStores` API surface.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::client::{Backend, ClientInner};
use crate::documents::Documents;
use crate::error::{Error, Result};
use crate::http_response::{
    sdk_http_response_from_headers, sdk_http_response_from_headers_and_body,
};
use crate::upload;
#[cfg(test)]
use crate::upload::CHUNK_SIZE;
use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::file_search_stores::{
    CreateFileSearchStoreConfig, DeleteFileSearchStoreConfig, FileSearchStore,
    GetFileSearchStoreConfig, ImportFileConfig, ImportFileOperation, ListFileSearchStoresConfig,
    ListFileSearchStoresResponse, UploadToFileSearchStoreConfig, UploadToFileSearchStoreOperation,
    UploadToFileSearchStoreResumableResponse,
};
use serde_json::Value;

#[derive(Clone)]
pub struct FileSearchStores {
    pub(crate) inner: Arc<ClientInner>,
}

impl FileSearchStores {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 访问 Documents 子服务。
    #[must_use]
    pub fn documents(&self) -> Documents {
        Documents::new(self.inner.clone())
    }

    /// 创建 `FileSearchStore`。
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
    pub async fn create(&self, mut config: CreateFileSearchStoreConfig) -> Result<FileSearchStore> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let mut body = serde_json::to_value(&config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let url = build_file_search_stores_url(&self.inner, http_options.as_ref());
        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<FileSearchStore>().await?)
    }

    /// 获取 `FileSearchStore`。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<FileSearchStore> {
        self.get_with_config(name, GetFileSearchStoreConfig::default())
            .await
    }

    /// 获取 `FileSearchStore`（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetFileSearchStoreConfig,
    ) -> Result<FileSearchStore> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_file_search_store_name(name.as_ref());
        let url = build_file_search_store_url(&self.inner, &name, http_options.as_ref());
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<FileSearchStore>().await?)
    }

    /// 删除 `FileSearchStore`。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn delete(&self, name: impl AsRef<str>) -> Result<()> {
        self.delete_with_config(name, DeleteFileSearchStoreConfig::default())
            .await
    }

    /// 删除 `FileSearchStore`（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn delete_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: DeleteFileSearchStoreConfig,
    ) -> Result<()> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_file_search_store_name(name.as_ref());
        let url = build_file_search_store_url(&self.inner, &name, http_options.as_ref());
        let url = add_delete_query_params(&url, &config)?;
        let mut request = self.inner.http.delete(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(())
    }

    /// 列出 `FileSearchStore`。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list(&self) -> Result<ListFileSearchStoresResponse> {
        self.list_with_config(ListFileSearchStoresConfig::default())
            .await
    }

    /// 列出 `FileSearchStore`（带配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list_with_config(
        &self,
        mut config: ListFileSearchStoresConfig,
    ) -> Result<ListFileSearchStoresResponse> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let url = build_file_search_stores_url(&self.inner, http_options.as_ref());
        let url = add_list_query_params(&url, &config)?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let headers = response.headers().clone();
        let mut result = response.json::<ListFileSearchStoresResponse>().await?;
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 列出所有 `FileSearchStore`（自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all(&self) -> Result<Vec<FileSearchStore>> {
        self.all_with_config(ListFileSearchStoresConfig::default())
            .await
    }

    /// 列出所有 `FileSearchStore`（带配置，自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all_with_config(
        &self,
        mut config: ListFileSearchStoresConfig,
    ) -> Result<Vec<FileSearchStore>> {
        let mut stores = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options.clone_from(&http_options);
            let response = self.list_with_config(page_config).await?;
            if let Some(items) = response.file_search_stores {
                stores.extend(items);
            }
            match response.next_page_token {
                Some(token) if !token.is_empty() => {
                    config.page_token = Some(token);
                }
                _ => break,
            }
        }
        Ok(stores)
    }

    /// 上传文件内容到 `FileSearchStore`（直接上传字节数据）。
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
    pub async fn upload_to_file_search_store(
        &self,
        file_search_store_name: impl AsRef<str>,
        data: Vec<u8>,
        mut config: UploadToFileSearchStoreConfig,
    ) -> Result<UploadToFileSearchStoreOperation> {
        ensure_gemini_backend(&self.inner)?;
        let mime_type = config
            .mime_type
            .clone()
            .ok_or_else(|| Error::InvalidConfig {
                message: "mime_type is required when uploading raw bytes".into(),
            })?;

        let http_options = config.http_options.take();
        let size_bytes = data.len() as u64;
        let (upload_url, _, _) = self
            .start_resumable_upload(
                file_search_store_name.as_ref(),
                &config,
                http_options.as_ref(),
                &mime_type,
                Some(size_bytes),
                None,
            )
            .await?;
        self.upload_bytes(&upload_url, &data, http_options.as_ref())
            .await
    }

    /// 上传文件内容到 `FileSearchStore`（从文件路径）。
    ///
    /// # Errors
    /// 当文件无效、请求失败或响应解析失败时返回错误。
    pub async fn upload_to_file_search_store_from_path(
        &self,
        file_search_store_name: impl AsRef<str>,
        path: impl AsRef<Path>,
        mut config: UploadToFileSearchStoreConfig,
    ) -> Result<UploadToFileSearchStoreOperation> {
        ensure_gemini_backend(&self.inner)?;
        let path = path.as_ref();
        let metadata = tokio::fs::metadata(path).await?;
        if !metadata.is_file() {
            return Err(Error::InvalidConfig {
                message: format!("{} is not a valid file path", path.display()),
            });
        }

        let size_bytes = metadata.len();
        let mime_type = config.mime_type.take().unwrap_or_else(|| {
            mime_guess::from_path(path)
                .first_or_octet_stream()
                .essence_str()
                .to_string()
        });
        config.mime_type = Some(mime_type.clone());

        let file_name = path.file_name().and_then(|name| name.to_str());
        let http_options = config.http_options.take();
        let (upload_url, _, _) = self
            .start_resumable_upload(
                file_search_store_name.as_ref(),
                &config,
                http_options.as_ref(),
                &mime_type,
                Some(size_bytes),
                file_name,
            )
            .await?;
        let mut file_handle = tokio::fs::File::open(path).await?;
        self.upload_reader(
            &upload_url,
            &mut file_handle,
            size_bytes,
            http_options.as_ref(),
        )
        .await
    }

    /// 初始化一个 `FileSearchStore` 的 resumable upload，并返回原始 HTTP headers（含 `x-goog-upload-url`）。
    ///
    /// 该方法只执行 `start` 请求，不会上传文件内容。
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
    pub async fn upload_to_file_search_store_resumable(
        &self,
        file_search_store_name: impl AsRef<str>,
        mut config: UploadToFileSearchStoreConfig,
    ) -> Result<UploadToFileSearchStoreResumableResponse> {
        ensure_gemini_backend(&self.inner)?;

        let should_return_http_response = config.should_return_http_response.unwrap_or(false);
        let http_options = config.http_options.take();
        let mime_type = config
            .mime_type
            .clone()
            .ok_or_else(|| Error::InvalidConfig {
                message: "mime_type is required when starting a resumable upload".into(),
            })?;

        let (_, headers, text) = self
            .start_resumable_upload(
                file_search_store_name.as_ref(),
                &config,
                http_options.as_ref(),
                &mime_type,
                None,
                None,
            )
            .await?;

        let response = UploadToFileSearchStoreResumableResponse {
            sdk_http_response: Some(if should_return_http_response {
                sdk_http_response_from_headers_and_body(&headers, text)
            } else {
                sdk_http_response_from_headers(&headers)
            }),
        };
        Ok(response)
    }

    /// 导入 `File` API 文件到 `FileSearchStore`。
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
    pub async fn import_file(
        &self,
        file_search_store_name: impl AsRef<str>,
        file_name: impl AsRef<str>,
        mut config: ImportFileConfig,
    ) -> Result<ImportFileOperation> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let store_name = normalize_file_search_store_name(file_search_store_name.as_ref());
        let file_name = normalize_file_name(file_name.as_ref())?;

        let mut body = serde_json::to_value(&config)?;
        let body_map = body.as_object_mut().ok_or_else(|| Error::Parse {
            message: "ImportFileConfig must be object".into(),
        })?;
        body_map.insert("fileName".to_string(), Value::String(file_name));

        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }

        let url =
            build_file_search_store_import_url(&self.inner, &store_name, http_options.as_ref());
        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<ImportFileOperation>().await?)
    }

    async fn start_resumable_upload(
        &self,
        file_search_store_name: &str,
        config: &UploadToFileSearchStoreConfig,
        http_options: Option<&rust_genai_types::http::HttpOptions>,
        mime_type: &str,
        size_bytes: Option<u64>,
        file_name: Option<&str>,
    ) -> Result<(String, reqwest::header::HeaderMap, String)> {
        let store_name = normalize_file_search_store_name(file_search_store_name);
        let url = build_file_search_store_upload_url(&self.inner, &store_name, http_options);
        let mut body = serde_json::to_value(config)?;
        if let Some(options) = http_options {
            merge_extra_body(&mut body, options)?;
        }

        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options)?;
        request = request
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header("X-Goog-Upload-Header-Content-Type", mime_type);
        if let Some(size_bytes) = size_bytes {
            request = request.header(
                "X-Goog-Upload-Header-Content-Length",
                size_bytes.to_string(),
            );
        }
        if let Some(file_name) = file_name {
            request = request.header("X-Goog-Upload-File-Name", file_name);
        }

        let response = self
            .inner
            .send_with_http_options(request, http_options)
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let headers = response.headers().clone();
        let upload_url = headers
            .get("x-goog-upload-url")
            .ok_or_else(|| Error::Parse {
                message: "Missing x-goog-upload-url header".into(),
            })?
            .to_str()
            .map_err(|_| Error::Parse {
                message: "Invalid x-goog-upload-url header".into(),
            })?
            .to_string();
        let text = response.text().await.unwrap_or_default();
        Ok((upload_url, headers, text))
    }

    async fn upload_bytes(
        &self,
        upload_url: &str,
        data: &[u8],
        http_options: Option<&rust_genai_types::http::HttpOptions>,
    ) -> Result<UploadToFileSearchStoreOperation> {
        let validate_status = |_status: &str| Ok(());
        upload::upload_bytes_with(
            data,
            |chunk, offset, finalize| {
                self.send_upload_chunk(upload_url, chunk, offset, finalize, http_options)
            },
            validate_status,
            "Upload ended unexpectedly",
        )
        .await
    }

    async fn upload_reader(
        &self,
        upload_url: &str,
        reader: &mut tokio::fs::File,
        size_bytes: u64,
        http_options: Option<&rust_genai_types::http::HttpOptions>,
    ) -> Result<UploadToFileSearchStoreOperation> {
        let validate_status = |_status: &str| Ok(());
        upload::upload_reader_with(
            reader,
            size_bytes,
            |chunk, offset, finalize| {
                self.send_upload_chunk(upload_url, chunk, offset, finalize, http_options)
            },
            validate_status,
            "Upload ended unexpectedly",
        )
        .await
    }

    async fn send_upload_chunk(
        &self,
        upload_url: &str,
        data: Vec<u8>,
        offset: u64,
        finalize: bool,
        http_options: Option<&rust_genai_types::http::HttpOptions>,
    ) -> Result<(String, Option<UploadToFileSearchStoreOperation>)> {
        let command = if finalize {
            "upload, finalize"
        } else {
            "upload"
        };
        let data_len = data.len();
        let mut request = self.inner.http.post(upload_url);
        request = apply_http_options(request, http_options)?;
        request = request
            .header("Content-Type", "application/json")
            .header("X-Goog-Upload-Command", command)
            .header("X-Goog-Upload-Offset", offset.to_string())
            .header("Content-Length", data_len.to_string())
            .body(data);

        let response = self
            .inner
            .send_with_http_options(request, http_options)
            .await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let status = response
            .headers()
            .get("x-goog-upload-status")
            .ok_or_else(|| Error::Parse {
                message: "Missing x-goog-upload-status header".into(),
            })?
            .to_str()
            .map_err(|_| Error::Parse {
                message: "Invalid x-goog-upload-status header".into(),
            })?
            .to_string();

        if status != "final" {
            return Ok((status, None));
        }

        let operation = response.json::<UploadToFileSearchStoreOperation>().await?;
        Ok((status, Some(operation)))
    }
}

fn ensure_gemini_backend(inner: &ClientInner) -> Result<()> {
    if inner.config.backend == Backend::VertexAi {
        return Err(Error::InvalidConfig {
            message: "FileSearchStores API is only supported in Gemini API".into(),
        });
    }
    Ok(())
}

fn normalize_file_search_store_name(name: &str) -> String {
    if name.starts_with("fileSearchStores/") {
        name.to_string()
    } else {
        format!("fileSearchStores/{name}")
    }
}

fn normalize_file_name(value: &str) -> Result<String> {
    if value.starts_with("http://") || value.starts_with("https://") {
        let marker = "files/";
        let start = value.find(marker).ok_or_else(|| Error::InvalidConfig {
            message: format!("Could not find 'files/' in URI: {value}"),
        })?;
        let suffix = &value[start + marker.len()..];
        let name: String = suffix
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        if name.is_empty() {
            return Err(Error::InvalidConfig {
                message: format!("Could not extract file name from URI: {value}"),
            });
        }
        Ok(format!("files/{name}"))
    } else if value.starts_with("files/") {
        Ok(value.to_string())
    } else {
        Ok(format!("files/{value}"))
    }
}

fn build_file_search_stores_url(
    inner: &ClientInner,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/fileSearchStores")
}

fn build_file_search_store_url(
    inner: &ClientInner,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/{name}")
}

fn build_file_search_store_import_url(
    inner: &ClientInner,
    store_name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/{store_name}:importFile")
}

fn build_file_search_store_upload_url(
    inner: &ClientInner,
    store_name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}upload/{version}/{store_name}:uploadToFileSearchStore")
}

fn add_list_query_params(url: &str, config: &ListFileSearchStoresConfig) -> Result<String> {
    let mut url = reqwest::Url::parse(url).map_err(|err| Error::InvalidConfig {
        message: err.to_string(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(page_size) = config.page_size {
            pairs.append_pair("pageSize", &page_size.to_string());
        }
        if let Some(page_token) = &config.page_token {
            pairs.append_pair("pageToken", page_token);
        }
    }
    Ok(url.to_string())
}

fn add_delete_query_params(url: &str, config: &DeleteFileSearchStoreConfig) -> Result<String> {
    let mut url = reqwest::Url::parse(url).map_err(|err| Error::InvalidConfig {
        message: err.to_string(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(force) = config.force {
            pairs.append_pair("force", &force.to_string());
        }
    }
    Ok(url.to_string())
}

fn apply_http_options(
    mut request: reqwest::RequestBuilder,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<reqwest::RequestBuilder> {
    if let Some(options) = http_options {
        if let Some(timeout) = options.timeout {
            request = request.timeout(Duration::from_millis(timeout));
        }
        if let Some(headers) = &options.headers {
            for (key, value) in headers {
                let name =
                    HeaderName::from_bytes(key.as_bytes()).map_err(|_| Error::InvalidConfig {
                        message: format!("Invalid header name: {key}"),
                    })?;
                let value = HeaderValue::from_str(value).map_err(|_| Error::InvalidConfig {
                    message: format!("Invalid header value for {key}"),
                })?;
                request = request.header(name, value);
            }
        }
    }
    Ok(request)
}

fn merge_extra_body(
    body: &mut Value,
    http_options: &rust_genai_types::http::HttpOptions,
) -> Result<()> {
    if let Some(extra) = &http_options.extra_body {
        match (body, extra) {
            (Value::Object(body_map), Value::Object(extra_map)) => {
                for (key, value) in extra_map {
                    body_map.insert(key.clone(), value.clone());
                }
            }
            (_, _) => {
                return Err(Error::InvalidConfig {
                    message: "HttpOptions.extra_body must be an object".into(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
fn finalize_upload(
    status: &str,
    operation: Option<UploadToFileSearchStoreOperation>,
) -> Result<UploadToFileSearchStoreOperation> {
    upload::finalize_upload(status, operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_client_inner;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    #[derive(Clone)]
    struct UploadMultiResponder {
        calls: Arc<AtomicUsize>,
    }

    impl Respond for UploadMultiResponder {
        fn respond(&self, request: &Request) -> ResponseTemplate {
            let idx = self.calls.fetch_add(1, Ordering::SeqCst);
            let command = request
                .headers
                .get("x-goog-upload-command")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            let should_finalize = command.contains("finalize") || idx > 0;
            if should_finalize {
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-status", "final")
                    .set_body_json(serde_json::json!({"name": "operations/final"}))
            } else {
                ResponseTemplate::new(200).insert_header("x-goog-upload-status", "active")
            }
        }
    }

    #[test]
    fn test_normalize_names_and_urls() {
        assert_eq!(
            normalize_file_search_store_name("store"),
            "fileSearchStores/store"
        );
        assert_eq!(normalize_file_name("files/abc").unwrap(), "files/abc");
        assert_eq!(normalize_file_name("abc").unwrap(), "files/abc");
        assert!(normalize_file_name("https://example.com/no-files").is_err());

        let gemini = test_client_inner(Backend::GeminiApi);
        let url = build_file_search_stores_url(&gemini, None);
        assert!(url.ends_with("/v1beta/fileSearchStores"));
        let url = build_file_search_store_upload_url(&gemini, "fileSearchStores/1", None);
        assert!(url.contains("/upload/"));
    }

    #[test]
    fn test_query_params_and_backend_check() {
        let url = add_list_query_params(
            "https://example.com/fileSearchStores",
            &ListFileSearchStoresConfig {
                page_size: Some(2),
                page_token: Some("t".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(url.contains("pageSize=2"));
        assert!(url.contains("pageToken=t"));

        let url = add_delete_query_params(
            "https://example.com/fileSearchStores/1",
            &DeleteFileSearchStoreConfig {
                force: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(url.contains("force=true"));

        let vertex = test_client_inner(Backend::VertexAi);
        let err = ensure_gemini_backend(&vertex).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_finalize_upload_errors() {
        let err = finalize_upload("active", None).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
        let err = finalize_upload("final", None).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[tokio::test]
    async fn test_start_resumable_upload_missing_header() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/upload/v1beta/fileSearchStores/store:uploadToFileSearchStore",
            ))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);
        let config = UploadToFileSearchStoreConfig {
            http_options: Some(rust_genai_types::http::HttpOptions {
                base_url: Some(format!("{}/", server.uri())),
                api_version: Some("v1beta".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = stores
            .start_resumable_upload(
                "fileSearchStores/store",
                &config,
                config.http_options.as_ref(),
                "text/plain",
                Some(3),
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[tokio::test]
    async fn test_create_merges_extra_body_and_import_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1beta/fileSearchStores"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "fileSearchStores/1"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1beta/fileSearchStores/store:importFile"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);
        let config = CreateFileSearchStoreConfig {
            http_options: Some(rust_genai_types::http::HttpOptions {
                base_url: Some(format!("{}/", server.uri())),
                api_version: Some("v1beta".to_string()),
                extra_body: Some(json!({"extra": "value"})),
                ..Default::default()
            }),
            ..Default::default()
        };
        let created = stores.create(config).await.unwrap();
        assert_eq!(created.name.as_deref(), Some("fileSearchStores/1"));

        let import_config = ImportFileConfig {
            http_options: Some(rust_genai_types::http::HttpOptions {
                base_url: Some(format!("{}/", server.uri())),
                api_version: Some("v1beta".to_string()),
                extra_body: Some(json!({"extra": "value"})),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = stores
            .import_file("fileSearchStores/store", "files/123", import_config)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::ApiError { .. }));

        let received = server.received_requests().await.unwrap();
        let body0 = String::from_utf8_lossy(&received[0].body);
        let body1 = String::from_utf8_lossy(&received[1].body);
        assert!(body0.contains(r#""extra":"value""#));
        assert!(body1.contains(r#""extra":"value""#));
    }

    #[tokio::test]
    async fn test_upload_bytes_and_mime_guess_success() {
        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/upload/v1beta/fileSearchStores/store:uploadToFileSearchStore",
            ))
            .respond_with(ResponseTemplate::new(200).insert_header(
                "x-goog-upload-url",
                format!("{}/upload-bytes", server.uri()),
            ))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/upload-bytes"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-status", "final")
                    .set_body_json(json!({"name": "operations/bytes"})),
            )
            .mount(&server)
            .await;

        let config = UploadToFileSearchStoreConfig {
            mime_type: Some("text/plain".to_string()),
            http_options: Some(rust_genai_types::http::HttpOptions {
                base_url: Some(format!("{}/", server.uri())),
                api_version: Some("v1beta".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let op = stores
            .upload_to_file_search_store("fileSearchStores/store", vec![1, 2, 3], config)
            .await
            .unwrap();
        assert_eq!(op.name.as_deref(), Some("operations/bytes"));

        let received = server.received_requests().await.unwrap();
        assert_eq!(received.len(), 2);
        assert!(received[0]
            .headers
            .get("x-goog-upload-header-content-length")
            .is_some());

        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("file.bin");
        tokio::fs::write(&file_path, b"data").await.unwrap();

        Mock::given(method("POST"))
            .and(path(
                "/upload/v1beta/fileSearchStores/store:uploadToFileSearchStore",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-url", format!("{}/upload-path", server.uri())),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/upload-path"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-status", "final")
                    .set_body_json(json!({"name": "operations/path"})),
            )
            .mount(&server)
            .await;

        let config = UploadToFileSearchStoreConfig {
            http_options: Some(rust_genai_types::http::HttpOptions {
                base_url: Some(format!("{}/", server.uri())),
                api_version: Some("v1beta".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let op = stores
            .upload_to_file_search_store_from_path("fileSearchStores/store", &file_path, config)
            .await
            .unwrap();
        assert_eq!(op.name.as_deref(), Some("operations/path"));
    }

    #[tokio::test]
    async fn test_upload_to_file_search_store_resumable_sets_http_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/upload/v1beta/fileSearchStores/store:uploadToFileSearchStore",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header(
                        "x-goog-upload-url",
                        format!("{}/upload-resumable", server.uri()),
                    )
                    .set_body_string("raw-body"),
            )
            .mount(&server)
            .await;

        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);
        let config = UploadToFileSearchStoreConfig {
            mime_type: Some("text/plain".to_string()),
            should_return_http_response: Some(true),
            http_options: Some(rust_genai_types::http::HttpOptions {
                base_url: Some(format!("{}/", server.uri())),
                api_version: Some("v1beta".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resp = stores
            .upload_to_file_search_store_resumable("fileSearchStores/store", config)
            .await
            .unwrap();
        let sdk = resp.sdk_http_response.unwrap();
        let headers = sdk.headers.unwrap();
        assert!(headers.contains_key("x-goog-upload-url"));
        assert_eq!(sdk.body.as_deref(), Some("raw-body"));
    }

    #[tokio::test]
    async fn test_start_resumable_upload_error_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(
                "/upload/v1beta/fileSearchStores/store:uploadToFileSearchStore",
            ))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);
        let config = UploadToFileSearchStoreConfig {
            http_options: Some(rust_genai_types::http::HttpOptions {
                base_url: Some(format!("{}/", server.uri())),
                api_version: Some("v1beta".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let err = stores
            .start_resumable_upload(
                "fileSearchStores/store",
                &config,
                config.http_options.as_ref(),
                "text/plain",
                Some(3),
                Some("file.txt"),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::ApiError { .. }));
    }

    #[tokio::test]
    async fn test_upload_bytes_and_send_chunk_errors() {
        let server = MockServer::start().await;
        let upload_url = format!("{}/upload-final", server.uri());

        Mock::given(method("POST"))
            .and(path("/upload-final"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-status", "final")
                    .set_body_json(serde_json::json!({"name": "operations/1"})),
            )
            .mount(&server)
            .await;

        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);
        let op = stores.upload_bytes(&upload_url, &[], None).await.unwrap();
        assert_eq!(op.name.as_deref(), Some("operations/1"));

        Mock::given(method("POST"))
            .and(path("/upload-missing"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let err = stores
            .send_upload_chunk(
                &format!("{}/upload-missing", server.uri()),
                Vec::new(),
                0,
                true,
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));

        Mock::given(method("POST"))
            .and(path("/upload-fail"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
            .mount(&server)
            .await;
        let err = stores
            .send_upload_chunk(
                &format!("{}/upload-fail", server.uri()),
                Vec::new(),
                0,
                true,
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::ApiError { .. }));
    }

    #[test]
    fn test_documents_accessor() {
        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);
        let _documents = stores.documents();
    }

    #[tokio::test]
    async fn test_upload_missing_mime_type_and_bad_path() {
        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);

        let err = stores
            .upload_to_file_search_store(
                "fileSearchStores/store",
                vec![1, 2, 3],
                UploadToFileSearchStoreConfig::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let temp_dir = std::env::temp_dir().join("rust_genai_fs_store_test");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;
        let err = stores
            .upload_to_file_search_store_from_path(
                "fileSearchStores/store",
                &temp_dir,
                UploadToFileSearchStoreConfig::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_upload_reader_unexpected_eof() {
        let server = MockServer::start().await;
        let upload_url = format!("{}/upload-eof", server.uri());
        Mock::given(method("POST"))
            .and(path("/upload-eof"))
            .respond_with(ResponseTemplate::new(200).insert_header("x-goog-upload-status", "final"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");
        tokio::fs::write(&file_path, b"").await.unwrap();
        let mut file = tokio::fs::File::open(&file_path).await.unwrap();

        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);
        let err = stores
            .upload_reader(&upload_url, &mut file, 10, None)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[tokio::test]
    async fn test_upload_bytes_multi_chunk_and_reader_empty() {
        let server = MockServer::start().await;
        let upload_url = format!("{}/upload-multi", server.uri());

        let calls = Arc::new(AtomicUsize::new(0));
        Mock::given(method("POST"))
            .and(path("/upload-multi"))
            .respond_with(UploadMultiResponder {
                calls: Arc::clone(&calls),
            })
            .mount(&server)
            .await;

        let inner = Arc::new(test_client_inner(Backend::GeminiApi));
        let stores = FileSearchStores::new(inner);
        let data = vec![0u8; CHUNK_SIZE + 1];
        let op = stores.upload_bytes(&upload_url, &data, None).await.unwrap();
        assert_eq!(op.name.as_deref(), Some("operations/final"));
        let received = server.received_requests().await.unwrap();
        assert_eq!(received.len(), 2);
        assert_eq!(
            received[0]
                .headers
                .get("x-goog-upload-command")
                .unwrap()
                .to_str()
                .unwrap(),
            "upload"
        );
        assert_eq!(
            received[1]
                .headers
                .get("x-goog-upload-command")
                .unwrap()
                .to_str()
                .unwrap(),
            "upload, finalize"
        );

        let upload_url = format!("{}/upload-empty", server.uri());
        Mock::given(method("POST"))
            .and(path("/upload-empty"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-status", "final")
                    .set_body_json(serde_json::json!({"name": "operations/empty"})),
            )
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("empty2.txt");
        tokio::fs::write(&file_path, b"").await.unwrap();
        let mut file = tokio::fs::File::open(&file_path).await.unwrap();
        let op = stores
            .upload_reader(&upload_url, &mut file, 0, None)
            .await
            .unwrap();
        assert_eq!(op.name.as_deref(), Some("operations/empty"));
    }

    #[test]
    fn test_apply_http_options_invalid_header_value() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = rust_genai_types::http::HttpOptions {
            headers: Some([("x-test".to_string(), "bad\nvalue".to_string())].into()),
            ..Default::default()
        };
        let err = apply_http_options(request, Some(&options)).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_merge_extra_body_invalid() {
        let mut body = json!({});
        let options = rust_genai_types::http::HttpOptions {
            extra_body: Some(json!("bad")),
            ..Default::default()
        };
        let err = merge_extra_body(&mut body, &options).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_normalize_file_name_from_uri_and_query_param_errors() {
        let name = normalize_file_name("https://example.com/files/abc-123?alt=media").unwrap();
        assert_eq!(name, "files/abc-123");

        let err =
            add_list_query_params("://bad", &ListFileSearchStoresConfig::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
        let err =
            add_delete_query_params("://bad", &DeleteFileSearchStoreConfig::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_apply_http_options_invalid_header_name_and_timeout() {
        let client = reqwest::Client::new();
        let request = client.get("https://example.com");
        let options = rust_genai_types::http::HttpOptions {
            timeout: Some(1000),
            headers: Some([("bad header".to_string(), "ok".to_string())].into()),
            ..Default::default()
        };
        let err = apply_http_options(request, Some(&options)).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_merge_extra_body_success() {
        let mut body = json!({"a": 1});
        let options = rust_genai_types::http::HttpOptions {
            extra_body: Some(json!({"b": 2})),
            ..Default::default()
        };
        merge_extra_body(&mut body, &options).unwrap();
        assert_eq!(body["b"], 2);
    }
}
