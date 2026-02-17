//! Files API surface.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::header::{HeaderName, HeaderValue};

use crate::client::Credentials;
use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};
use crate::http_response::{sdk_http_response_from_headers, sdk_http_response_from_headers_and_body};
use crate::upload;
#[cfg(test)]
use crate::upload::CHUNK_SIZE;
use rust_genai_types::enums::FileState;
use rust_genai_types::files::{
    DeleteFileResponse, DownloadFileConfig, File, ListFilesConfig, ListFilesResponse,
    RegisterFilesConfig, RegisterFilesResponse, UploadFileConfig,
};
use serde_json::Value;

#[derive(Clone)]
pub struct Files {
    pub(crate) inner: Arc<ClientInner>,
}

impl Files {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 上传文件（直接上传字节数据）。
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
    pub async fn upload(&self, data: Vec<u8>, mime_type: impl Into<String>) -> Result<File> {
        let config = UploadFileConfig {
            mime_type: Some(mime_type.into()),
            ..UploadFileConfig::default()
        };
        self.upload_with_config(data, config).await
    }

    /// 上传文件（自定义配置）。
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
    pub async fn upload_with_config(
        &self,
        data: Vec<u8>,
        config: UploadFileConfig,
    ) -> Result<File> {
        ensure_gemini_backend(&self.inner)?;

        let mime_type = config
            .mime_type
            .clone()
            .ok_or_else(|| Error::InvalidConfig {
                message: "mime_type is required when uploading raw bytes".into(),
            })?;
        let size_bytes = data.len() as u64;
        let file = build_upload_file(config, size_bytes, &mime_type);
        let upload_url = self
            .start_resumable_upload(file, size_bytes, &mime_type, None)
            .await?;
        self.upload_bytes(&upload_url, &data).await
    }

    /// 从文件路径上传。
    ///
    /// # Errors
    /// 当文件无效、请求失败或响应解析失败时返回错误。
    pub async fn upload_from_path(&self, path: impl AsRef<Path>) -> Result<File> {
        self.upload_from_path_with_config(path, UploadFileConfig::default())
            .await
    }

    /// 从文件路径上传（自定义配置）。
    ///
    /// # Errors
    /// 当文件无效、请求失败或响应解析失败时返回错误。
    pub async fn upload_from_path_with_config(
        &self,
        path: impl AsRef<Path>,
        mut config: UploadFileConfig,
    ) -> Result<File> {
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

        let file_name = path.file_name().and_then(|name| name.to_str());
        let file = build_upload_file(config, size_bytes, &mime_type);
        let upload_url = self
            .start_resumable_upload(file, size_bytes, &mime_type, file_name)
            .await?;
        let mut file_handle = tokio::fs::File::open(path).await?;
        self.upload_reader(&upload_url, &mut file_handle, size_bytes)
            .await
    }

    /// 下载文件（返回字节内容）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn download(&self, name_or_uri: impl AsRef<str>) -> Result<Vec<u8>> {
        ensure_gemini_backend(&self.inner)?;

        let file_name = normalize_file_name(name_or_uri.as_ref())?;
        let url = build_file_download_url(&self.inner, &file_name);
        let request = self.inner.http.get(url);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    #[allow(unused_variables)]
    /// 下载文件（自定义配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn download_with_config(
        &self,
        name_or_uri: impl AsRef<str>,
        _config: DownloadFileConfig,
    ) -> Result<Vec<u8>> {
        self.download(name_or_uri).await
    }

    /// 列出文件。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list(&self) -> Result<ListFilesResponse> {
        self.list_with_config(ListFilesConfig::default()).await
    }

    /// 列出文件（自定义配置）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list_with_config(&self, config: ListFilesConfig) -> Result<ListFilesResponse> {
        ensure_gemini_backend(&self.inner)?;
        let url = build_files_list_url(&self.inner, &config)?;
        let request = self.inner.http.get(url);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let headers = response.headers().clone();
        let mut result = response.json::<ListFilesResponse>().await?;
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 列出所有文件（自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all(&self) -> Result<Vec<File>> {
        self.all_with_config(ListFilesConfig::default()).await
    }

    /// 列出所有文件（带配置，自动翻页）。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all_with_config(&self, mut config: ListFilesConfig) -> Result<Vec<File>> {
        let mut files = Vec::new();
        loop {
            let response = self.list_with_config(config.clone()).await?;
            if let Some(items) = response.files {
                files.extend(items);
            }
            match response.next_page_token {
                Some(token) if !token.is_empty() => {
                    config.page_token = Some(token);
                }
                _ => break,
            }
        }
        Ok(files)
    }

    /// 获取文件元数据。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn get(&self, name_or_uri: impl AsRef<str>) -> Result<File> {
        ensure_gemini_backend(&self.inner)?;

        let file_name = normalize_file_name(name_or_uri.as_ref())?;
        let url = build_file_url(&self.inner, &file_name);
        let request = self.inner.http.get(url);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<File>().await?)
    }

    /// 删除文件。
    ///
    /// # Errors
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn delete(&self, name_or_uri: impl AsRef<str>) -> Result<DeleteFileResponse> {
        ensure_gemini_backend(&self.inner)?;

        let file_name = normalize_file_name(name_or_uri.as_ref())?;
        let url = build_file_url(&self.inner, &file_name);
        let request = self.inner.http.delete(url);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let headers = response.headers().clone();
        let text = response.text().await.unwrap_or_default();
        let mut result = if text.trim().is_empty() {
            DeleteFileResponse::default()
        } else {
            serde_json::from_str::<DeleteFileResponse>(&text)?
        };
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 注册 Google Cloud Storage 文件（使其可用于 Gemini Developer API）。
    ///
    /// 该方法要求客户端使用 OAuth/ADC（即 `Credentials::OAuth` 或 `Credentials::ApplicationDefault`），
    /// **不支持** API key 认证。
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
    pub async fn register_files(&self, uris: Vec<String>) -> Result<RegisterFilesResponse> {
        self.register_files_with_config(uris, RegisterFilesConfig::default())
            .await
    }

    /// 注册 Google Cloud Storage 文件（自定义配置）。
    ///
    /// # Errors
    /// 当配置无效、请求失败或响应解析失败时返回错误。
    pub async fn register_files_with_config(
        &self,
        uris: Vec<String>,
        mut config: RegisterFilesConfig,
    ) -> Result<RegisterFilesResponse> {
        ensure_gemini_backend(&self.inner)?;
        if matches!(self.inner.config.credentials, Credentials::ApiKey(_)) {
            return Err(Error::InvalidConfig {
                message: "register_files requires OAuth/ADC credentials, API key is not supported"
                    .into(),
            });
        }

        let should_return_http_response = config.should_return_http_response.unwrap_or(false);
        let http_options = config.http_options.take();
        let url = build_files_register_url(&self.inner, http_options.as_ref());
        let mut request = self
            .inner
            .http
            .post(url)
            .json(&serde_json::json!({ "uris": uris }));
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
        let text = response.text().await.unwrap_or_default();
        if should_return_http_response {
            let mut result = RegisterFilesResponse::default();
            result.sdk_http_response =
                Some(sdk_http_response_from_headers_and_body(&headers, text));
            return Ok(result);
        }
        if text.trim().is_empty() {
            let mut result = RegisterFilesResponse::default();
            result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
            return Ok(result);
        }
        let mut result: RegisterFilesResponse = serde_json::from_str(&text)?;
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 轮询直到文件状态变为 ACTIVE。
    ///
    /// # Errors
    /// 当请求失败、文件失败或超时返回错误。
    pub async fn wait_for_active(
        &self,
        name_or_uri: impl AsRef<str>,
        config: WaitForFileConfig,
    ) -> Result<File> {
        ensure_gemini_backend(&self.inner)?;

        let start = Instant::now();
        loop {
            let file = self.get(name_or_uri.as_ref()).await?;
            match file.state {
                Some(FileState::Active) => return Ok(file),
                Some(FileState::Failed) => {
                    return Err(Error::ApiError {
                        status: 500,
                        message: "File processing failed".into(),
                    })
                }
                _ => {}
            }

            if let Some(timeout) = config.timeout {
                if start.elapsed() >= timeout {
                    return Err(Error::Timeout {
                        message: "Timed out waiting for file to become ACTIVE".into(),
                    });
                }
            }

            tokio::time::sleep(config.poll_interval).await;
        }
    }

    async fn start_resumable_upload(
        &self,
        file: File,
        size_bytes: u64,
        mime_type: &str,
        file_name: Option<&str>,
    ) -> Result<String> {
        let url = build_files_upload_url(&self.inner);
        let mut request = self
            .inner
            .http
            .post(url)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header(
                "X-Goog-Upload-Header-Content-Length",
                size_bytes.to_string(),
            )
            .header("X-Goog-Upload-Header-Content-Type", mime_type);

        if let Some(file_name) = file_name {
            request = request.header("X-Goog-Upload-File-Name", file_name);
        }

        let body = serde_json::json!({ "file": file });
        let request = request.json(&body);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let upload_url = response
            .headers()
            .get("x-goog-upload-url")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| Error::Parse {
                message: "Missing x-goog-upload-url header".into(),
            })?;

        Ok(upload_url.to_string())
    }

    async fn upload_bytes(&self, upload_url: &str, data: &[u8]) -> Result<File> {
        let validate_status = |status: &str| {
            if status != "active" {
                return Err(Error::Parse {
                    message: format!("Unexpected upload status: {status}"),
                });
            }
            Ok(())
        };

        upload::upload_bytes_with(
            data,
            |chunk, offset, finalize| self.send_upload_chunk(upload_url, chunk, offset, finalize),
            validate_status,
            "Upload finished without final response",
        )
        .await
    }

    async fn upload_reader(
        &self,
        upload_url: &str,
        reader: &mut tokio::fs::File,
        total_size: u64,
    ) -> Result<File> {
        let validate_status = |status: &str| {
            if status != "active" {
                return Err(Error::Parse {
                    message: format!("Unexpected upload status: {status}"),
                });
            }
            Ok(())
        };

        upload::upload_reader_with(
            reader,
            total_size,
            |chunk, offset, finalize| self.send_upload_chunk(upload_url, chunk, offset, finalize),
            validate_status,
            "Upload finished without final response",
        )
        .await
    }

    async fn send_upload_chunk(
        &self,
        upload_url: &str,
        chunk: Vec<u8>,
        offset: u64,
        finalize: bool,
    ) -> Result<(String, Option<File>)> {
        let command = if finalize {
            "upload, finalize"
        } else {
            "upload"
        };
        let chunk_len = chunk.len();
        let response = self
            .inner
            .http
            .post(upload_url)
            .header("X-Goog-Upload-Command", command)
            .header("X-Goog-Upload-Offset", offset.to_string())
            .header("Content-Length", chunk_len.to_string())
            .body(chunk)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let upload_status = response
            .headers()
            .get("x-goog-upload-status")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| Error::Parse {
                message: "Missing x-goog-upload-status header".into(),
            })?
            .to_string();

        let body = response.bytes().await?;
        if body.is_empty() {
            return Ok((upload_status, None));
        }

        let value: Value = serde_json::from_slice(&body)?;
        let file_value = value.get("file").cloned().unwrap_or(value);
        let file: File = serde_json::from_value(file_value)?;

        Ok((upload_status, Some(file)))
    }
}

#[derive(Debug, Clone)]
pub struct WaitForFileConfig {
    pub poll_interval: Duration,
    pub timeout: Option<Duration>,
}

impl Default for WaitForFileConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(2),
            timeout: Some(Duration::from_secs(300)),
        }
    }
}

#[cfg(test)]
fn finalize_upload(status: &str, file: Option<File>) -> Result<File> {
    upload::finalize_upload(status, file)
}

fn ensure_gemini_backend(inner: &ClientInner) -> Result<()> {
    if inner.config.backend == Backend::VertexAi {
        return Err(Error::InvalidConfig {
            message: "Files API is only supported in Gemini API".into(),
        });
    }
    Ok(())
}

fn build_upload_file(config: UploadFileConfig, size_bytes: u64, mime_type: &str) -> File {
    let mut file = File::default();
    if let Some(name) = config.name {
        file.name = Some(normalize_upload_name(&name));
    }
    file.display_name = config.display_name;
    file.mime_type = Some(mime_type.to_string());
    file.size_bytes = Some(size_bytes.to_string());
    file
}

fn normalize_upload_name(name: &str) -> String {
    if name.starts_with("files/") {
        name.to_string()
    } else {
        format!("files/{name}")
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
        Ok(name)
    } else if value.starts_with("files/") {
        Ok(value.trim_start_matches("files/").to_string())
    } else {
        Ok(value.to_string())
    }
}

fn build_files_upload_url(inner: &ClientInner) -> String {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    format!("{base}upload/{version}/files")
}

fn build_files_list_url(inner: &ClientInner, config: &ListFilesConfig) -> Result<String> {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    let url = format!("{base}{version}/files");
    add_list_query_params(&url, config)
}

fn build_files_register_url(
    inner: &ClientInner,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> String {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    format!("{base}{version}/files:register")
}

fn build_file_url(inner: &ClientInner, name: &str) -> String {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    format!("{base}{version}/files/{name}")
}

fn build_file_download_url(inner: &ClientInner, name: &str) -> String {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    format!("{base}{version}/files/{name}:download?alt=media")
}

fn add_list_query_params(url: &str, config: &ListFilesConfig) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Client;
    use crate::test_support::test_client_inner;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    #[test]
    fn test_normalize_file_name() {
        assert_eq!(normalize_file_name("files/abc-123").unwrap(), "abc-123");
        assert_eq!(normalize_file_name("abc-123").unwrap(), "abc-123");
        assert_eq!(
            normalize_file_name("https://example.com/files/abc-123?foo=bar").unwrap(),
            "abc-123"
        );
    }

    #[test]
    fn test_build_urls() {
        let client = Client::new("test-key").unwrap();
        let files = client.files();
        let url = build_files_upload_url(&files.inner);
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/upload/v1beta/files"
        );
        let url = build_files_register_url(&files.inner, None);
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/files:register"
        );
    }

    #[test]
    fn test_normalize_upload_and_list_params() {
        assert_eq!(normalize_upload_name("files/abc"), "files/abc");
        assert_eq!(normalize_upload_name("abc"), "files/abc");
        assert!(normalize_file_name("https://example.com/no-files").is_err());
        assert!(normalize_file_name("https://example.com/files/?x").is_err());

        let url = add_list_query_params(
            "https://example.com/files",
            &ListFilesConfig {
                page_size: Some(3),
                page_token: Some("t".to_string()),
            },
        )
        .unwrap();
        assert!(url.contains("pageSize=3"));
        assert!(url.contains("pageToken=t"));
    }

    #[test]
    fn test_build_upload_file_and_finalize_errors() {
        let file = build_upload_file(
            UploadFileConfig {
                name: Some("abc".to_string()),
                display_name: Some("d".to_string()),
                ..Default::default()
            },
            5,
            "text/plain",
        );
        assert_eq!(file.name.as_deref(), Some("files/abc"));
        assert_eq!(file.size_bytes.as_deref(), Some("5"));

        let err = finalize_upload("active", None).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
        let err = finalize_upload("final", None).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[test]
    fn test_ensure_gemini_backend_error() {
        let vertex = test_client_inner(Backend::VertexAi);
        let err = ensure_gemini_backend(&vertex).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[tokio::test]
    async fn test_start_resumable_upload_and_send_chunk_errors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/upload/v1beta/files"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = Client::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()
            .unwrap();
        let files = client.files();
        let file = build_upload_file(UploadFileConfig::default(), 1, "text/plain");
        let err = files
            .start_resumable_upload(file, 1, "text/plain", None)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));

        Mock::given(method("POST"))
            .and(path("/upload-chunk"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
        let err = files
            .send_upload_chunk(
                &format!("{}/upload-chunk", server.uri()),
                Vec::new(),
                0,
                true,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));

        Mock::given(method("POST"))
            .and(path("/upload-fail"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
            .mount(&server)
            .await;
        let err = files
            .send_upload_chunk(
                &format!("{}/upload-fail", server.uri()),
                Vec::new(),
                0,
                true,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::ApiError { .. }));
    }

    #[tokio::test]
    async fn test_files_upload_errors() {
        let client = Client::new("test-key").unwrap();
        let files = client.files();

        let err = files
            .upload_with_config(vec![1, 2, 3], UploadFileConfig::default())
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let temp_dir = std::env::temp_dir().join("rust_genai_files_test_dir");
        let _ = tokio::fs::create_dir_all(&temp_dir).await;
        let err = files
            .upload_from_path_with_config(&temp_dir, UploadFileConfig::default())
            .await
            .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn test_start_resumable_upload_error_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/upload/v1beta/files"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let client = Client::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()
            .unwrap();
        let files = client.files();
        let file = build_upload_file(UploadFileConfig::default(), 1, "text/plain");
        let err = files
            .start_resumable_upload(file, 1, "text/plain", None)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::ApiError { .. }));
    }

    #[tokio::test]
    async fn test_upload_bytes_empty_and_status_errors() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/upload-empty"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-status", "final")
                    .set_body_json(json!({
                        "file": {"name": "files/empty", "state": "ACTIVE"}
                    })),
            )
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/upload-bad"))
            .respond_with(
                ResponseTemplate::new(200).insert_header("x-goog-upload-status", "paused"),
            )
            .mount(&server)
            .await;

        let client = Client::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()
            .unwrap();
        let files = client.files();

        let file = files
            .upload_bytes(&format!("{}/upload-empty", server.uri()), &[])
            .await
            .unwrap();
        assert_eq!(file.name.as_deref(), Some("files/empty"));

        let data = vec![0u8; CHUNK_SIZE + 1];
        let err = files
            .upload_bytes(&format!("{}/upload-bad", server.uri()), &data)
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[tokio::test]
    async fn test_upload_reader_empty_file() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/upload-empty-file"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-status", "final")
                    .set_body_json(json!({
                        "file": {"name": "files/empty-file", "state": "ACTIVE"}
                    })),
            )
            .mount(&server)
            .await;

        let client = Client::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()
            .unwrap();
        let files = client.files();
        let temp_path = std::env::temp_dir().join("rust_genai_empty_upload_file");
        let _ = tokio::fs::write(&temp_path, &[]).await;
        let mut handle = tokio::fs::File::open(&temp_path).await.unwrap();

        let file = files
            .upload_reader(
                &format!("{}/upload-empty-file", server.uri()),
                &mut handle,
                0,
            )
            .await
            .unwrap();
        assert_eq!(file.name.as_deref(), Some("files/empty-file"));
        let _ = tokio::fs::remove_file(&temp_path).await;
    }

    #[tokio::test]
    async fn test_upload_bytes_and_reader_active_then_final() {
        #[derive(Clone)]
        struct UploadResponder;

        impl Respond for UploadResponder {
            fn respond(&self, request: &Request) -> ResponseTemplate {
                let finalize = request
                    .headers
                    .get("x-goog-upload-command")
                    .and_then(|value| value.to_str().ok())
                    .is_some_and(|value| value.contains("finalize"));
                if finalize {
                    ResponseTemplate::new(200)
                        .insert_header("x-goog-upload-status", "final")
                        .set_body_json(json!({
                            "file": {"name": "files/final", "state": "ACTIVE"}
                        }))
                } else {
                    ResponseTemplate::new(200).insert_header("x-goog-upload-status", "active")
                }
            }
        }

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/upload-active"))
            .respond_with(UploadResponder)
            .mount(&server)
            .await;

        let client = Client::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()
            .unwrap();
        let files = client.files();
        let data = vec![0u8; CHUNK_SIZE + 1];
        let file = files
            .upload_bytes(&format!("{}/upload-active", server.uri()), &data)
            .await
            .unwrap();
        assert_eq!(file.name.as_deref(), Some("files/final"));

        Mock::given(method("POST"))
            .and(path("/upload-reader"))
            .respond_with(UploadResponder)
            .mount(&server)
            .await;
        let temp_path = std::env::temp_dir().join("rust_genai_reader_active");
        let _ = tokio::fs::write(&temp_path, vec![0u8; CHUNK_SIZE + 1]).await;
        let mut handle = tokio::fs::File::open(&temp_path).await.unwrap();
        let file = files
            .upload_reader(
                &format!("{}/upload-reader", server.uri()),
                &mut handle,
                (CHUNK_SIZE + 1) as u64,
            )
            .await
            .unwrap();
        assert_eq!(file.name.as_deref(), Some("files/final"));
        let _ = tokio::fs::remove_file(&temp_path).await;
    }

    #[tokio::test]
    async fn test_upload_with_config_and_mime_guess() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/upload/v1beta/files"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-url", format!("{}/upload-ok", server.uri())),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/upload-ok"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-goog-upload-status", "final")
                    .set_body_json(json!({
                        "file": {"name": "files/ok", "state": "ACTIVE"}
                    })),
            )
            .mount(&server)
            .await;

        let client = Client::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()
            .unwrap();
        let files = client.files();
        let file = files.upload(vec![1, 2, 3], "text/plain").await.unwrap();
        assert_eq!(file.name.as_deref(), Some("files/ok"));

        let temp_path = std::env::temp_dir().join("rust_genai_upload_guess.txt");
        let _ = tokio::fs::write(&temp_path, b"hello").await;
        let file = files
            .upload_from_path_with_config(&temp_path, UploadFileConfig::default())
            .await
            .unwrap();
        assert_eq!(file.name.as_deref(), Some("files/ok"));
        let _ = tokio::fs::remove_file(&temp_path).await;
    }

    #[tokio::test]
    async fn test_wait_for_active_timeout_after_sleep() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1beta/files/slow"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "files/slow",
                "state": "PROCESSING"
            })))
            .mount(&server)
            .await;

        let client = Client::builder()
            .api_key("test-key")
            .base_url(server.uri())
            .build()
            .unwrap();
        let files = client.files();
        let err = files
            .wait_for_active(
                "slow",
                WaitForFileConfig {
                    poll_interval: Duration::from_millis(1),
                    timeout: Some(Duration::from_millis(2)),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Timeout { .. }));
    }

    #[test]
    fn test_add_list_query_params_invalid_url() {
        let err = add_list_query_params("http://[::1", &ListFilesConfig::default()).unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }
}
