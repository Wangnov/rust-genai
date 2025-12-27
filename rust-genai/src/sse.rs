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

            match self.parse_lines(&event_bytes) {
                Ok(Some(event)) => events.push(Ok(event)),
                Ok(None) => {}
                Err(err) => events.push(Err(err)),
            }
        }

        events
    }

    fn find_delimiter(&self, buf: &[u8]) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize)> = None;

        if let Some(pos) = self.finder_crlf.find(buf) {
            best = Some((pos, 4));
        }
        if let Some(pos) = self.finder_lf.find(buf) {
            best = pick_min(best, pos, 2);
        }
        if let Some(pos) = self.finder_cr.find(buf) {
            best = pick_min(best, pos, 2);
        }

        best
    }

    fn parse_lines(&self, data: &[u8]) -> Result<Option<ServerSentEvent>> {
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

fn pick_min(best: Option<(usize, usize)>, pos: usize, len: usize) -> Option<(usize, usize)> {
    match best {
        None => Some((pos, len)),
        Some((best_pos, best_len)) => {
            if pos < best_pos {
                Some((pos, len))
            } else {
                Some((best_pos, best_len))
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
pub fn parse_sse_stream_with<T>(response: reqwest::Response) -> SseJsonStream<T>
where
    T: DeserializeOwned,
{
    SseJsonStream::new(response)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
