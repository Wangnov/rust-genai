//! FileSearchStores API surface.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderName, HeaderValue};
use rust_genai_types::file_search_stores::{
    CreateFileSearchStoreConfig, DeleteFileSearchStoreConfig, FileSearchStore,
    GetFileSearchStoreConfig, ImportFileConfig, ListFileSearchStoresConfig,
    ListFileSearchStoresResponse, UploadToFileSearchStoreConfig,
};
use rust_genai_types::operations::Operation;
use serde_json::Value;
use tokio::io::AsyncReadExt;

use crate::client::{Backend, ClientInner};
use crate::documents::Documents;
use crate::error::{Error, Result};

const CHUNK_SIZE: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub struct FileSearchStores {
    pub(crate) inner: Arc<ClientInner>,
}

impl FileSearchStores {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 访问 Documents 子服务。
    pub fn documents(&self) -> Documents {
        Documents::new(self.inner.clone())
    }

    /// 创建 FileSearchStore。
    pub async fn create(&self, mut config: CreateFileSearchStoreConfig) -> Result<FileSearchStore> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let mut body = serde_json::to_value(&config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let url = build_file_search_stores_url(&self.inner, http_options.as_ref())?;
        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<FileSearchStore>().await?)
    }

    /// 获取 FileSearchStore。
    pub async fn get(&self, name: impl AsRef<str>) -> Result<FileSearchStore> {
        self.get_with_config(name, GetFileSearchStoreConfig::default())
            .await
    }

    /// 获取 FileSearchStore（带配置）。
    pub async fn get_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: GetFileSearchStoreConfig,
    ) -> Result<FileSearchStore> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_file_search_store_name(name.as_ref());
        let url = build_file_search_store_url(&self.inner, &name, http_options.as_ref())?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<FileSearchStore>().await?)
    }

    /// 删除 FileSearchStore。
    pub async fn delete(&self, name: impl AsRef<str>) -> Result<()> {
        self.delete_with_config(name, DeleteFileSearchStoreConfig::default())
            .await
    }

    /// 删除 FileSearchStore（带配置）。
    pub async fn delete_with_config(
        &self,
        name: impl AsRef<str>,
        mut config: DeleteFileSearchStoreConfig,
    ) -> Result<()> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let name = normalize_file_search_store_name(name.as_ref());
        let url = build_file_search_store_url(&self.inner, &name, http_options.as_ref())?;
        let url = add_delete_query_params(url, &config)?;
        let mut request = self.inner.http.delete(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(())
    }

    /// 列出 FileSearchStore。
    pub async fn list(&self) -> Result<ListFileSearchStoresResponse> {
        self.list_with_config(ListFileSearchStoresConfig::default())
            .await
    }

    /// 列出 FileSearchStore（带配置）。
    pub async fn list_with_config(
        &self,
        mut config: ListFileSearchStoresConfig,
    ) -> Result<ListFileSearchStoresResponse> {
        ensure_gemini_backend(&self.inner)?;
        let http_options = config.http_options.take();
        let url = build_file_search_stores_url(&self.inner, http_options.as_ref())?;
        let url = add_list_query_params(url, &config)?;
        let mut request = self.inner.http.get(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<ListFileSearchStoresResponse>().await?)
    }

    /// 列出所有 FileSearchStore（自动翻页）。
    pub async fn all(&self) -> Result<Vec<FileSearchStore>> {
        self.all_with_config(ListFileSearchStoresConfig::default())
            .await
    }

    /// 列出所有 FileSearchStore（带配置，自动翻页）。
    pub async fn all_with_config(
        &self,
        mut config: ListFileSearchStoresConfig,
    ) -> Result<Vec<FileSearchStore>> {
        let mut stores = Vec::new();
        let http_options = config.http_options.clone();
        loop {
            let mut page_config = config.clone();
            page_config.http_options = http_options.clone();
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

    /// 上传文件内容到 FileSearchStore（直接上传字节数据）。
    pub async fn upload_to_file_search_store(
        &self,
        file_search_store_name: impl AsRef<str>,
        data: Vec<u8>,
        mut config: UploadToFileSearchStoreConfig,
    ) -> Result<Operation> {
        ensure_gemini_backend(&self.inner)?;
        let mime_type = config
            .mime_type
            .clone()
            .ok_or_else(|| Error::InvalidConfig {
                message: "mime_type is required when uploading raw bytes".into(),
            })?;

        let http_options = config.http_options.take();
        let size_bytes = data.len() as u64;
        let upload_url = self
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

    /// 上传文件内容到 FileSearchStore（从文件路径）。
    pub async fn upload_to_file_search_store_from_path(
        &self,
        file_search_store_name: impl AsRef<str>,
        path: impl AsRef<Path>,
        mut config: UploadToFileSearchStoreConfig,
    ) -> Result<Operation> {
        ensure_gemini_backend(&self.inner)?;
        let path = path.as_ref();
        let metadata = tokio::fs::metadata(path).await?;
        if !metadata.is_file() {
            return Err(Error::InvalidConfig {
                message: format!("{} is not a valid file path", path.display()),
            });
        }

        let size_bytes = metadata.len();
        let mime_type = if let Some(value) = config.mime_type.take() {
            value
        } else {
            mime_guess::from_path(path)
                .first_or_octet_stream()
                .essence_str()
                .to_string()
        };
        config.mime_type = Some(mime_type.clone());

        let file_name = path.file_name().and_then(|name| name.to_str());
        let http_options = config.http_options.take();
        let upload_url = self
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

    /// 导入 File API 文件到 FileSearchStore。
    pub async fn import_file(
        &self,
        file_search_store_name: impl AsRef<str>,
        file_name: impl AsRef<str>,
        mut config: ImportFileConfig,
    ) -> Result<Operation> {
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
            build_file_search_store_import_url(&self.inner, &store_name, http_options.as_ref())?;
        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<Operation>().await?)
    }

    async fn start_resumable_upload(
        &self,
        file_search_store_name: &str,
        config: &UploadToFileSearchStoreConfig,
        http_options: Option<&rust_genai_types::http::HttpOptions>,
        mime_type: &str,
        size_bytes: Option<u64>,
        file_name: Option<&str>,
    ) -> Result<String> {
        let store_name = normalize_file_search_store_name(file_search_store_name);
        let url = build_file_search_store_upload_url(&self.inner, &store_name, http_options)?;
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

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let upload_url =
            response
                .headers()
                .get("x-goog-upload-url")
                .ok_or_else(|| Error::Parse {
                    message: "Missing x-goog-upload-url header".into(),
                })?;
        let upload_url = upload_url.to_str().map_err(|_| Error::Parse {
            message: "Invalid x-goog-upload-url header".into(),
        })?;
        Ok(upload_url.to_string())
    }

    async fn upload_bytes(
        &self,
        upload_url: &str,
        data: &[u8],
        http_options: Option<&rust_genai_types::http::HttpOptions>,
    ) -> Result<Operation> {
        let mut offset: usize = 0;
        if data.is_empty() {
            let (status, operation) = self
                .send_upload_chunk(upload_url, &[], 0, true, http_options)
                .await?;
            return finalize_upload(status, operation);
        }

        while offset < data.len() {
            let end = std::cmp::min(offset + CHUNK_SIZE, data.len());
            let finalize = end >= data.len();
            let (status, operation) = self
                .send_upload_chunk(
                    upload_url,
                    &data[offset..end],
                    offset as u64,
                    finalize,
                    http_options,
                )
                .await?;
            if finalize {
                return finalize_upload(status, operation);
            }
            offset = end;
        }

        Err(Error::Parse {
            message: "Upload ended unexpectedly".into(),
        })
    }

    async fn upload_reader(
        &self,
        upload_url: &str,
        reader: &mut tokio::fs::File,
        size_bytes: u64,
        http_options: Option<&rust_genai_types::http::HttpOptions>,
    ) -> Result<Operation> {
        let mut offset: u64 = 0;
        if size_bytes == 0 {
            let (status, operation) = self
                .send_upload_chunk(upload_url, &[], 0, true, http_options)
                .await?;
            return finalize_upload(status, operation);
        }

        let mut buffer = vec![0u8; CHUNK_SIZE];
        loop {
            let read = reader.read(&mut buffer).await?;
            if read == 0 {
                return Err(Error::Parse {
                    message: "Unexpected EOF while uploading file".into(),
                });
            }
            let finalize = offset + read as u64 >= size_bytes;
            let (status, operation) = self
                .send_upload_chunk(upload_url, &buffer[..read], offset, finalize, http_options)
                .await?;
            offset += read as u64;
            if finalize {
                return finalize_upload(status, operation);
            }
        }
    }

    async fn send_upload_chunk(
        &self,
        upload_url: &str,
        data: &[u8],
        offset: u64,
        finalize: bool,
        http_options: Option<&rust_genai_types::http::HttpOptions>,
    ) -> Result<(String, Option<Operation>)> {
        let command = if finalize {
            "upload, finalize"
        } else {
            "upload"
        };
        let mut request = self.inner.http.post(upload_url);
        request = apply_http_options(request, http_options)?;
        request = request
            .header("Content-Type", "application/json")
            .header("X-Goog-Upload-Command", command)
            .header("X-Goog-Upload-Offset", offset.to_string())
            .header("Content-Length", data.len().to_string())
            .body(data.to_vec());

        let response = self.inner.send(request).await?;
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

        let operation = response.json::<Operation>().await?;
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
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!("{base}{version}/fileSearchStores"))
}

fn build_file_search_store_url(
    inner: &ClientInner,
    name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!("{base}{version}/{name}"))
}

fn build_file_search_store_import_url(
    inner: &ClientInner,
    store_name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!("{base}{version}/{store_name}:importFile"))
}

fn build_file_search_store_upload_url(
    inner: &ClientInner,
    store_name: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    Ok(format!(
        "{base}upload/{version}/{store_name}:uploadToFileSearchStore"
    ))
}

fn add_list_query_params(url: String, config: &ListFileSearchStoresConfig) -> Result<String> {
    let mut url = reqwest::Url::parse(&url).map_err(|err| Error::InvalidConfig {
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

fn add_delete_query_params(url: String, config: &DeleteFileSearchStoreConfig) -> Result<String> {
    let mut url = reqwest::Url::parse(&url).map_err(|err| Error::InvalidConfig {
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

fn finalize_upload(status: String, operation: Option<Operation>) -> Result<Operation> {
    if status != "final" {
        return Err(Error::Parse {
            message: format!("Upload finalize failed: {status}"),
        });
    }
    operation.ok_or_else(|| Error::Parse {
        message: "Upload completed but response body was empty".into(),
    })
}
