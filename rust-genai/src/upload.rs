use crate::error::{Error, Result};
use std::future::Future;
use tokio::io::AsyncReadExt;

pub const CHUNK_SIZE: usize = 8 * 1024 * 1024;

pub fn finalize_upload<T>(status: &str, value: Option<T>) -> Result<T> {
    if status != "final" {
        return Err(Error::Parse {
            message: format!("Upload finalize failed: {status}"),
        });
    }
    value.ok_or_else(|| Error::Parse {
        message: "Upload completed but response body was empty".into(),
    })
}

pub async fn upload_bytes_with<P, F, Fut, H>(
    data: &[u8],
    mut send_chunk: F,
    mut validate_status: H,
    finished_error: &'static str,
) -> Result<P>
where
    F: FnMut(Vec<u8>, u64, bool) -> Fut,
    Fut: Future<Output = Result<(String, Option<P>)>>,
    H: FnMut(&str) -> Result<()>,
{
    if data.is_empty() {
        let (status, payload) = send_chunk(Vec::new(), 0, true).await?;
        return finalize_upload(&status, payload);
    }

    let mut offset: usize = 0;
    while offset < data.len() {
        let end = (offset + CHUNK_SIZE).min(data.len());
        let finalize = end == data.len();
        let (status, payload) =
            send_chunk(data[offset..end].to_vec(), offset as u64, finalize).await?;

        if finalize {
            return finalize_upload(&status, payload);
        }

        validate_status(&status)?;
        offset = end;
    }

    Err(Error::Parse {
        message: finished_error.into(),
    })
}

pub async fn upload_reader_with<P, F, Fut, H>(
    reader: &mut tokio::fs::File,
    total_size: u64,
    mut send_chunk: F,
    mut validate_status: H,
    finished_error: &'static str,
) -> Result<P>
where
    F: FnMut(Vec<u8>, u64, bool) -> Fut,
    Fut: Future<Output = Result<(String, Option<P>)>>,
    H: FnMut(&str) -> Result<()>,
{
    if total_size == 0 {
        let (status, payload) = send_chunk(Vec::new(), 0, true).await?;
        return finalize_upload(&status, payload);
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
        let (status, payload) = send_chunk(buffer[..read_bytes].to_vec(), offset, finalize).await?;
        if finalize {
            return finalize_upload(&status, payload);
        }

        validate_status(&status)?;
        offset += read_bytes as u64;
    }

    Err(Error::Parse {
        message: finished_error.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::io::AsyncWriteExt;

    #[test]
    fn finalize_upload_rejects_invalid_status_and_empty_body() {
        let err = finalize_upload::<usize>("in_progress", Some(1))
            .err()
            .unwrap();
        assert!(matches!(err, Error::Parse { .. }));

        let err = finalize_upload::<usize>("final", None).err().unwrap();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[tokio::test]
    async fn upload_bytes_with_empty_payload_finishes() {
        let called = Arc::new(AtomicBool::new(false));
        let called_inner = called.clone();
        let result = upload_bytes_with::<usize, _, _, _>(
            &[],
            move |chunk, offset, finalize| {
                let called_inner = called_inner.clone();
                async move {
                    called_inner.store(true, Ordering::SeqCst);
                    assert!(chunk.is_empty());
                    assert_eq!(offset, 0);
                    assert!(finalize);
                    Ok(("final".to_string(), Some(5)))
                }
            },
            |_| Ok(()),
            "finished_error",
        )
        .await
        .unwrap();
        assert_eq!(result, 5);
        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn upload_bytes_with_multi_chunk_validates_status() {
        let data = vec![7u8; CHUNK_SIZE + 1];
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_inner = calls.clone();
        let result = upload_bytes_with::<usize, _, _, _>(
            &data,
            move |chunk, offset, finalize| {
                let calls_inner = calls_inner.clone();
                async move {
                    calls_inner.fetch_add(1, Ordering::SeqCst);
                    assert!(!chunk.is_empty());
                    if finalize {
                        Ok(("final".to_string(), Some(7)))
                    } else {
                        assert_eq!(offset, 0);
                        Ok(("in_progress".to_string(), None))
                    }
                }
            },
            |status| {
                assert_eq!(status, "in_progress");
                Ok(())
            },
            "finished_error",
        )
        .await
        .unwrap();
        assert_eq!(result, 7);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn upload_reader_with_unexpected_eof_errors() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("data.bin");
        let mut file = tokio::fs::File::create(&path).await.unwrap();
        file.write_all(&[1]).await.unwrap();
        file.flush().await.unwrap();

        let mut file = tokio::fs::File::open(&path).await.unwrap();
        let err = upload_reader_with::<usize, _, _, _>(
            &mut file,
            2,
            |_chunk, _offset, finalize| async move {
                if finalize {
                    Ok(("final".to_string(), Some(1)))
                } else {
                    Ok(("in_progress".to_string(), None))
                }
            },
            |status| {
                assert_eq!(status, "in_progress");
                Ok(())
            },
            "finished_error",
        )
        .await
        .err()
        .unwrap();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[tokio::test]
    async fn upload_reader_with_empty_file_finishes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.bin");
        tokio::fs::write(&path, &[]).await.unwrap();

        let mut file = tokio::fs::File::open(&path).await.unwrap();
        let result = upload_reader_with::<usize, _, _, _>(
            &mut file,
            0,
            |_chunk, _offset, _finalize| async move { Ok(("final".to_string(), Some(9))) },
            |_| Ok(()),
            "finished_error",
        )
        .await
        .unwrap();
        assert_eq!(result, 9);
    }
}
