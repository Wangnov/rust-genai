//! Files API surface.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rust_genai_types::enums::FileState;
use rust_genai_types::files::{
    DownloadFileConfig, File, ListFilesConfig, ListFilesResponse, UploadFileConfig,
};
use serde_json::Value;
use tokio::io::AsyncReadExt;

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

const CHUNK_SIZE: usize = 8 * 1024 * 1024;

#[derive(Clone)]
pub struct Files {
    pub(crate) inner: Arc<ClientInner>,
}

impl Files {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 上传文件（直接上传字节数据）。
    pub async fn upload(&self, data: Vec<u8>, mime_type: impl Into<String>) -> Result<File> {
        let config = UploadFileConfig {
            mime_type: Some(mime_type.into()),
            ..UploadFileConfig::default()
        };
        self.upload_with_config(data, config).await
    }

    /// 上传文件（自定义配置）。
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
        let file = build_upload_file(config, size_bytes, &mime_type)?;
        let upload_url = self
            .start_resumable_upload(file, size_bytes, &mime_type, None)
            .await?;
        self.upload_bytes(&upload_url, &data).await
    }

    /// 从文件路径上传。
    pub async fn upload_from_path(&self, path: impl AsRef<Path>) -> Result<File> {
        self.upload_from_path_with_config(path, UploadFileConfig::default())
            .await
    }

    /// 从文件路径上传（自定义配置）。
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
        let mime_type = if let Some(value) = config.mime_type.take() {
            value
        } else {
            mime_guess::from_path(path)
                .first_or_octet_stream()
                .essence_str()
                .to_string()
        };

        let file_name = path.file_name().and_then(|name| name.to_str());
        let file = build_upload_file(config, size_bytes, &mime_type)?;
        let upload_url = self
            .start_resumable_upload(file, size_bytes, &mime_type, file_name)
            .await?;
        let mut file_handle = tokio::fs::File::open(path).await?;
        self.upload_reader(&upload_url, &mut file_handle, size_bytes)
            .await
    }

    /// 下载文件（返回字节内容）。
    pub async fn download(&self, name_or_uri: impl AsRef<str>) -> Result<Vec<u8>> {
        ensure_gemini_backend(&self.inner)?;

        let file_name = normalize_file_name(name_or_uri.as_ref())?;
        let url = build_file_download_url(&self.inner, &file_name)?;
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
    pub async fn download_with_config(
        &self,
        name_or_uri: impl AsRef<str>,
        _config: DownloadFileConfig,
    ) -> Result<Vec<u8>> {
        self.download(name_or_uri).await
    }

    /// 列出文件。
    pub async fn list(&self) -> Result<ListFilesResponse> {
        self.list_with_config(ListFilesConfig::default()).await
    }

    /// 列出文件（自定义配置）。
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
        Ok(response.json::<ListFilesResponse>().await?)
    }

    /// 列出所有文件（自动翻页）。
    pub async fn all(&self) -> Result<Vec<File>> {
        self.all_with_config(ListFilesConfig::default()).await
    }

    /// 列出所有文件（带配置，自动翻页）。
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
    pub async fn get(&self, name_or_uri: impl AsRef<str>) -> Result<File> {
        ensure_gemini_backend(&self.inner)?;

        let file_name = normalize_file_name(name_or_uri.as_ref())?;
        let url = build_file_url(&self.inner, &file_name)?;
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
    pub async fn delete(&self, name_or_uri: impl AsRef<str>) -> Result<()> {
        ensure_gemini_backend(&self.inner)?;

        let file_name = normalize_file_name(name_or_uri.as_ref())?;
        let url = build_file_url(&self.inner, &file_name)?;
        let request = self.inner.http.delete(url);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(())
    }

    /// 轮询直到文件状态变为 ACTIVE。
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
        let url = build_files_upload_url(&self.inner)?;
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
        if data.is_empty() {
            let (status, file) = self.send_upload_chunk(upload_url, &[], 0, true).await?;
            return finalize_upload(status, file);
        }

        let mut offset: usize = 0;
        while offset < data.len() {
            let end = (offset + CHUNK_SIZE).min(data.len());
            let finalize = end == data.len();
            let (status, file) = self
                .send_upload_chunk(upload_url, &data[offset..end], offset as u64, finalize)
                .await?;

            if finalize {
                return finalize_upload(status, file);
            }

            if status != "active" {
                return Err(Error::Parse {
                    message: format!("Unexpected upload status: {status}"),
                });
            }

            offset = end;
        }

        Err(Error::Parse {
            message: "Upload finished without final response".into(),
        })
    }

    async fn upload_reader(
        &self,
        upload_url: &str,
        reader: &mut tokio::fs::File,
        total_size: u64,
    ) -> Result<File> {
        if total_size == 0 {
            let (status, file) = self.send_upload_chunk(upload_url, &[], 0, true).await?;
            return finalize_upload(status, file);
        }

        let mut offset: u64 = 0;
        let mut buffer = vec![0u8; CHUNK_SIZE];
        while offset < total_size {
            let read_bytes = reader.read(&mut buffer).await?;
            if read_bytes == 0 {
                return Err(Error::Parse {
                    message: "Unexpected EOF while uploading file".into(),
                });
            }

            let finalize = offset + read_bytes as u64 >= total_size;
            let (status, file) = self
                .send_upload_chunk(upload_url, &buffer[..read_bytes], offset, finalize)
                .await?;

            if finalize {
                return finalize_upload(status, file);
            }

            if status != "active" {
                return Err(Error::Parse {
                    message: format!("Unexpected upload status: {status}"),
                });
            }

            offset += read_bytes as u64;
        }

        Err(Error::Parse {
            message: "Upload finished without final response".into(),
        })
    }

    async fn send_upload_chunk(
        &self,
        upload_url: &str,
        chunk: &[u8],
        offset: u64,
        finalize: bool,
    ) -> Result<(String, Option<File>)> {
        let command = if finalize {
            "upload, finalize"
        } else {
            "upload"
        };
        let response = self
            .inner
            .http
            .post(upload_url)
            .header("X-Goog-Upload-Command", command)
            .header("X-Goog-Upload-Offset", offset.to_string())
            .header("Content-Length", chunk.len().to_string())
            .body(chunk.to_vec())
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

fn finalize_upload(status: String, file: Option<File>) -> Result<File> {
    if status != "final" {
        return Err(Error::Parse {
            message: format!("Upload finalize failed: {status}"),
        });
    }
    file.ok_or_else(|| Error::Parse {
        message: "Upload completed but response body was empty".into(),
    })
}

fn ensure_gemini_backend(inner: &ClientInner) -> Result<()> {
    if inner.config.backend == Backend::VertexAi {
        return Err(Error::InvalidConfig {
            message: "Files API is only supported in Gemini API".into(),
        });
    }
    Ok(())
}

fn build_upload_file(config: UploadFileConfig, size_bytes: u64, mime_type: &str) -> Result<File> {
    let mut file = File::default();
    if let Some(name) = config.name {
        file.name = Some(normalize_upload_name(&name));
    }
    file.display_name = config.display_name;
    file.mime_type = Some(mime_type.to_string());
    file.size_bytes = Some(size_bytes.to_string());
    Ok(file)
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

fn build_files_upload_url(inner: &ClientInner) -> Result<String> {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    Ok(format!("{base}upload/{version}/files"))
}

fn build_files_list_url(inner: &ClientInner, config: &ListFilesConfig) -> Result<String> {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    let url = format!("{base}{version}/files");
    add_list_query_params(url, config)
}

fn build_file_url(inner: &ClientInner, name: &str) -> Result<String> {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    Ok(format!("{base}{version}/files/{name}"))
}

fn build_file_download_url(inner: &ClientInner, name: &str) -> Result<String> {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    Ok(format!("{base}{version}/files/{name}:download?alt=media"))
}

fn add_list_query_params(url: String, config: &ListFilesConfig) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Client;

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
        let url = build_files_upload_url(&files.inner).unwrap();
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/upload/v1beta/files"
        );
    }
}
