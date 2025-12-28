//! SSE (Server-Sent Events) stream decoding utilities.

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Buf, Bytes, BytesMut};
use futures_util::Stream;
use memchr::memmem::Finder;
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};
use rust_genai_types::response::GenerateContentResponse;

/// SSE 事件。
#[derive(Debug, Clone)]
pub struct ServerSentEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

/// SSE 解码器。
pub struct SseDecoder {
    buffer: BytesMut,
    finder_lf: Finder<'static>,
    finder_cr: Finder<'static>,
    finder_crlf: Finder<'static>,
}

impl SseDecoder {
    /// 创建新的 SSE 解码器。
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(8192),
            finder_lf: Finder::new(b"\n\n"),
            finder_cr: Finder::new(b"\r\r"),
            finder_crlf: Finder::new(b"\r\n\r\n"),
        }
    }

    /// 解码一个 chunk，返回完整的 SSE 事件。
    pub fn decode(&mut self, chunk: &[u8]) -> Vec<Result<ServerSentEvent>> {
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::with_capacity(4);

        while let Some((pos, len)) = self.find_delimiter(&self.buffer) {
            let event_bytes = self.buffer.split_to(pos);
            self.buffer.advance(len);

            match Self::parse_lines(&event_bytes) {
                Ok(Some(event)) => events.push(Ok(event)),
                Ok(None) => {}
                Err(err) => events.push(Err(err)),
            }
        }

        events
    }

    fn find_delimiter(&self, buf: &[u8]) -> Option<(usize, usize)> {
        let best = self.finder_crlf.find(buf).map(|pos| (pos, 4));
        let best = self
            .finder_lf
            .find(buf)
            .map_or(best, |pos| Some(pick_min(best, pos, 2)));
        self.finder_cr
            .find(buf)
            .map_or(best, |pos| Some(pick_min(best, pos, 2)))
    }

    fn parse_lines(data: &[u8]) -> Result<Option<ServerSentEvent>> {
        if data.is_empty() {
            return Ok(None);
        }

        let text = std::str::from_utf8(data).map_err(|err| Error::Parse {
            message: err.to_string(),
        })?;

        let mut event: Option<String> = None;
        let mut id: Option<String> = None;
        let mut data_lines: Vec<String> = Vec::with_capacity(4);
        let mut has_field = false;

        for line in text.split('\n') {
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            if line.starts_with(':') {
                continue;
            }

            let (field, value) = match line.split_once(':') {
                Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
                None => (line, ""),
            };

            match field {
                "event" => {
                    has_field = true;
                    if !value.is_empty() {
                        event = Some(value.to_string());
                    }
                }
                "data" => {
                    has_field = true;
                    data_lines.push(value.to_string());
                }
                "id" => {
                    has_field = true;
                    if !value.is_empty() {
                        id = Some(value.to_string());
                    }
                }
                _ => {}
            }
        }

        if !has_field {
            return Ok(None);
        }

        Ok(Some(ServerSentEvent {
            event,
            data: data_lines.join("\n"),
            id,
        }))
    }
}

impl Default for SseDecoder {
    fn default() -> Self {
        Self::new()
    }
}

const fn pick_min(best: Option<(usize, usize)>, pos: usize, len: usize) -> (usize, usize) {
    match best {
        None => (pos, len),
        Some((best_pos, best_len)) => {
            if pos < best_pos {
                (pos, len)
            } else {
                (best_pos, best_len)
            }
        }
    }
}

/// SSE JSON Stream 包装器（泛型）。
pub struct SseJsonStream<T> {
    stream: Pin<Box<dyn Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send>>,
    decoder: SseDecoder,
    pending: VecDeque<Result<ServerSentEvent>>,
    done: bool,
    _marker: PhantomData<T>,
}

impl<T> Unpin for SseJsonStream<T> {}

impl<T> SseJsonStream<T> {
    /// 从 HTTP 响应创建 SSE 流。
    #[must_use]
    pub fn new(response: reqwest::Response) -> Self {
        Self {
            stream: Box::pin(response.bytes_stream()),
            decoder: SseDecoder::new(),
            pending: VecDeque::new(),
            done: false,
            _marker: PhantomData,
        }
    }
}

impl<T> Stream for SseJsonStream<T>
where
    T: DeserializeOwned,
{
    type Item = Result<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let Some(item) = this.pending.pop_front() {
                match item {
                    Err(err) => return Poll::Ready(Some(Err(err))),
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            this.done = true;
                            continue;
                        }

                        let parsed = serde_json::from_str::<T>(&event.data).map_err(Error::from)?;
                        return Poll::Ready(Some(Ok(parsed)));
                    }
                }
            }

            if this.done {
                return Poll::Ready(None);
            }

            match this.stream.as_mut().poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Ready(Some(Err(err))) => return Poll::Ready(Some(Err(err.into()))),
                Poll::Ready(Some(Ok(bytes))) => {
                    let events = this.decoder.decode(&bytes);
                    for event in events {
                        this.pending.push_back(event);
                    }
                }
            }
        }
    }
}

/// 便捷函数：从 reqwest Response 创建 SSE 流。
pub fn parse_sse_stream(
    response: reqwest::Response,
) -> impl Stream<Item = Result<GenerateContentResponse>> {
    parse_sse_stream_with::<GenerateContentResponse>(response)
}

/// 泛型 SSE JSON 流解析器。
#[must_use]
pub fn parse_sse_stream_with<T>(response: reqwest::Response) -> SseJsonStream<T>
where
    T: DeserializeOwned,
{
    SseJsonStream::new(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use serde_json::Value;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_sse_decoder_basic() {
        let mut decoder = SseDecoder::new();
        let chunk = b"data: {\"text\":\"Hello\"}\n\ndata: {\"text\":\"World\"}\n\n";
        let events = decoder.decode(chunk);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].as_ref().unwrap().data, r#"{"text":"Hello"}"#);
        assert_eq!(events[1].as_ref().unwrap().data, r#"{"text":"World"}"#);
    }

    #[test]
    fn test_sse_decoder_crlf() {
        let mut decoder = SseDecoder::new();
        let chunk = b"data: {\"text\":\"Hello\"}\r\n\r\n";
        let events = decoder.decode(chunk);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].as_ref().unwrap().data, r#"{"text":"Hello"}"#);
    }

    #[test]
    fn test_sse_decoder_default_works() {
        let mut decoder = SseDecoder::default();
        let chunk = b"data: {\"text\":\"Hello\"}\n\n";
        let events = decoder.decode(chunk);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_sse_decoder_line_without_colon_and_empty_lines() {
        let mut decoder = SseDecoder::new();
        let chunk = b"data\n\n\n";
        let events = decoder.decode(chunk);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].as_ref().unwrap().data, "");
    }

    #[test]
    fn test_sse_decoder_only_comments_returns_empty() {
        let mut decoder = SseDecoder::new();
        let chunk = b":comment\n\n";
        let events = decoder.decode(chunk);
        assert!(events.is_empty());
    }

    #[test]
    fn test_sse_done_signal() {
        let mut decoder = SseDecoder::new();
        let chunk = b"data: [DONE]\n\n";
        let events = decoder.decode(chunk);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].as_ref().unwrap().data, "[DONE]");
    }

    #[test]
    fn test_sse_double_cr() {
        let mut decoder = SseDecoder::new();
        let chunk = b"data: {\"text\":\"Hello\"}\r\r";
        let events = decoder.decode(chunk);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].as_ref().unwrap().data, r#"{"text":"Hello"}"#);
    }

    #[test]
    fn test_sse_decoder_event_and_id() {
        let mut decoder = SseDecoder::new();
        let chunk = b":comment\nid: 7\nevent: update\ndata: line1\ndata: line2\n\n";
        let events = decoder.decode(chunk);
        assert_eq!(events.len(), 1);
        let event = events[0].as_ref().unwrap();
        assert_eq!(event.event.as_deref(), Some("update"));
        assert_eq!(event.id.as_deref(), Some("7"));
        assert_eq!(event.data, "line1\nline2");
    }

    #[test]
    fn test_sse_decoder_invalid_utf8_and_empty() {
        let mut decoder = SseDecoder::new();
        let chunk = b"data: \xFF\xFF\n\n";
        let events = decoder.decode(chunk);
        assert_eq!(events.len(), 1);
        assert!(events[0].as_ref().is_err());

        let events = decoder.decode(b"\n\n");
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_sse_json_stream_invalid_utf8() {
        let server = MockServer::start().await;
        let body = vec![0xFF, 0xFF, b'\n', b'\n'];
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_bytes(body),
            )
            .mount(&server)
            .await;

        let response = reqwest::Client::new()
            .get(server.uri())
            .send()
            .await
            .unwrap();
        let mut stream = parse_sse_stream_with::<Value>(response);
        let err = stream.next().await.unwrap().unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[test]
    fn test_pick_min_prefers_smaller_position() {
        assert_eq!(pick_min(Some((5, 2)), 2, 4), (2, 4));
        assert_eq!(pick_min(Some((2, 2)), 5, 4), (2, 2));
    }

    #[tokio::test]
    async fn test_sse_json_stream_parses_and_done() {
        let server = MockServer::start().await;
        let body = "data: {\"value\":1}\n\ndata: [DONE]\n\n";
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(&server)
            .await;

        let response = reqwest::Client::new()
            .get(server.uri())
            .send()
            .await
            .unwrap();
        let mut stream = parse_sse_stream_with::<Value>(response);
        let first = stream.next().await.unwrap().unwrap();
        assert_eq!(first["value"], 1);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_sse_json_stream_invalid_json() {
        let server = MockServer::start().await;
        let body = "data: {bad json}\n\n";
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(&server)
            .await;

        let response = reqwest::Client::new()
            .get(server.uri())
            .send()
            .await
            .unwrap();
        let mut stream = parse_sse_stream_with::<Value>(response);
        let err = stream.next().await.unwrap().unwrap_err();
        assert!(matches!(err, Error::Serialization { .. }));
    }
}
