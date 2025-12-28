//! Live Music API surface.

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderMap, HeaderValue};
use tokio_tungstenite::tungstenite::Message;

use rust_genai_types::live_music_types::{
    LiveMusicClientContent, LiveMusicClientMessage, LiveMusicClientSetup,
    LiveMusicGenerationConfig, LiveMusicPlaybackControl, LiveMusicServerMessage, WeightedPrompt,
};

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};

#[derive(Clone)]
pub struct LiveMusic {
    pub(crate) inner: Arc<ClientInner>,
}

impl LiveMusic {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 连接到 Live Music API。
    ///
    /// # Errors
    /// 当连接失败或配置无效时返回错误。
    pub async fn connect(&self, model: impl Into<String>) -> Result<LiveMusicSession> {
        connect_live_music_session(self.inner.clone(), model.into()).await
    }
}

pub struct LiveMusicSession {
    outgoing_tx: mpsc::UnboundedSender<LiveMusicClientMessage>,
    incoming_rx: mpsc::UnboundedReceiver<Result<LiveMusicServerMessage>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LiveMusicSession {
    /// 设置加权提示词。
    ///
    /// # Errors
    /// 当配置无效或发送失败时返回错误。
    pub async fn set_weighted_prompts(&self, prompts: Vec<WeightedPrompt>) -> Result<()> {
        if prompts.is_empty() {
            return Err(Error::InvalidConfig {
                message: "weighted_prompts must contain at least one entry".into(),
            });
        }
        let message = LiveMusicClientMessage {
            setup: None,
            client_content: Some(LiveMusicClientContent {
                weighted_prompts: Some(prompts),
            }),
            music_generation_config: None,
            playback_control: None,
        };
        self.send_async(message).await
    }

    /// 设置音乐生成配置。
    ///
    /// # Errors
    /// 当发送失败时返回错误。
    pub async fn set_music_generation_config(
        &self,
        config: Option<LiveMusicGenerationConfig>,
    ) -> Result<()> {
        let message = LiveMusicClientMessage {
            setup: None,
            client_content: None,
            music_generation_config: Some(config.unwrap_or_default()),
            playback_control: None,
        };
        self.send_async(message).await
    }

    /// 播放。
    ///
    /// # Errors
    /// 当发送失败时返回错误。
    pub async fn play(&self) -> Result<()> {
        self.send_playback(LiveMusicPlaybackControl::Play).await
    }

    /// 暂停。
    ///
    /// # Errors
    /// 当发送失败时返回错误。
    pub async fn pause(&self) -> Result<()> {
        self.send_playback(LiveMusicPlaybackControl::Pause).await
    }

    /// 停止。
    ///
    /// # Errors
    /// 当发送失败时返回错误。
    pub async fn stop(&self) -> Result<()> {
        self.send_playback(LiveMusicPlaybackControl::Stop).await
    }

    /// 重置上下文。
    ///
    /// # Errors
    /// 当发送失败时返回错误。
    pub async fn reset_context(&self) -> Result<()> {
        self.send_playback(LiveMusicPlaybackControl::ResetContext)
            .await
    }

    /// 接收服务器消息。
    pub async fn receive(&mut self) -> Option<Result<LiveMusicServerMessage>> {
        self.incoming_rx.recv().await
    }

    /// 关闭会话。
    ///
    /// # Errors
    /// 当发送关闭信号失败时返回错误。
    pub async fn close(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        tokio::task::yield_now().await;
        Ok(())
    }

    async fn send_playback(&self, control: LiveMusicPlaybackControl) -> Result<()> {
        let message = LiveMusicClientMessage {
            setup: None,
            client_content: None,
            music_generation_config: None,
            playback_control: Some(control),
        };
        self.send_async(message).await
    }

    fn send(&self, message: LiveMusicClientMessage) -> Result<()> {
        self.outgoing_tx
            .send(message)
            .map_err(|_| Error::ChannelClosed)?;
        Ok(())
    }

    async fn send_async(&self, message: LiveMusicClientMessage) -> Result<()> {
        self.send(message)?;
        tokio::task::yield_now().await;
        Ok(())
    }
}

async fn connect_live_music_session(
    inner: Arc<ClientInner>,
    model: String,
) -> Result<LiveMusicSession> {
    if inner.config.backend == Backend::VertexAi {
        return Err(Error::InvalidConfig {
            message: "Live music is not supported for Vertex AI".into(),
        });
    }

    let api_key = inner
        .config
        .api_key
        .as_ref()
        .ok_or_else(|| Error::InvalidConfig {
            message: "API key required for Live Music API".into(),
        })?;

    if api_key.starts_with("auth_tokens/") {
        return Err(Error::InvalidConfig {
            message: "Live music does not support ephemeral tokens".into(),
        });
    }

    let (url, headers) = build_live_music_ws_url(
        &inner.api_client.base_url,
        &inner.api_client.api_version,
        api_key,
    )?;

    let request = build_ws_request(&url, &headers)?;
    let (ws_stream, _) = connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();

    let setup = LiveMusicClientMessage {
        setup: Some(LiveMusicClientSetup {
            model: Some(normalize_model_name(&model)),
        }),
        client_content: None,
        music_generation_config: None,
        playback_control: None,
    };
    let payload = serde_json::to_string(&setup)?;
    write.send(Message::Text(payload.into())).await?;

    loop {
        match read.next().await {
            Some(Ok(message)) => match message {
                Message::Close(frame) => {
                    return Err(Error::Parse {
                        message: format!("WebSocket closed before setup_complete: {frame:?}"),
                    })
                }
                _ => {
                    if let Some(parsed) = parse_server_message(message)? {
                        if parsed.setup_complete.is_some() {
                            break;
                        }
                    }
                }
            },
            Some(Err(err)) => return Err(Error::WebSocket { source: err }),
            None => {
                return Err(Error::Parse {
                    message: "WebSocket closed before setup_complete".into(),
                })
            }
        }
    }

    let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
    let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    tokio::spawn(message_loop(
        write,
        read,
        outgoing_rx,
        incoming_tx,
        shutdown_rx,
    ));

    Ok(LiveMusicSession {
        outgoing_tx,
        incoming_rx,
        shutdown_tx: Some(shutdown_tx),
    })
}

fn normalize_model_name(model: &str) -> String {
    if model.starts_with("models/") {
        model.to_string()
    } else {
        format!("models/{model}")
    }
}

fn build_ws_request(
    url: &Url,
    headers: &HeaderMap,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>> {
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|err| Error::Parse {
            message: err.to_string(),
        })?;
    {
        let request_headers = request.headers_mut();
        for (key, value) in headers {
            request_headers.insert(key, value.clone());
        }
    }
    Ok(request)
}

fn build_live_music_ws_url(
    base_url: &str,
    api_version: &str,
    api_key: &str,
) -> Result<(Url, HeaderMap)> {
    let mut url = Url::parse(base_url).map_err(|err| Error::InvalidConfig {
        message: err.to_string(),
    })?;

    let scheme = match url.scheme() {
        "http" | "ws" => "ws",
        _ => "wss",
    };
    url.set_scheme(scheme).map_err(|()| Error::InvalidConfig {
        message: "Invalid base_url scheme".into(),
    })?;

    let base_path = url.path().trim_end_matches('/');
    let path = format!(
        "{base_path}/ws/google.ai.generativelanguage.{api_version}.GenerativeService.BidiGenerateMusic"
    );
    url.set_path(&path);
    url.set_query(Some(&format!("key={api_key}")));

    let mut headers = HeaderMap::new();
    headers.insert(
        "x-goog-api-key",
        HeaderValue::from_str(api_key).map_err(|_| Error::InvalidConfig {
            message: "Invalid API key".into(),
        })?,
    );

    Ok((url, headers))
}

fn parse_server_message(message: Message) -> Result<Option<LiveMusicServerMessage>> {
    match message {
        Message::Text(text) => {
            let msg = serde_json::from_str::<LiveMusicServerMessage>(&text)?;
            Ok(Some(msg))
        }
        Message::Binary(data) => {
            let msg = serde_json::from_slice::<LiveMusicServerMessage>(&data)?;
            Ok(Some(msg))
        }
        Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => Ok(None),
    }
}

async fn message_loop(
    mut write: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    mut read: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    mut outgoing_rx: mpsc::UnboundedReceiver<LiveMusicClientMessage>,
    incoming_tx: mpsc::UnboundedSender<Result<LiveMusicServerMessage>>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            Some(message) = outgoing_rx.recv() => {
                match serde_json::to_string(&message) {
                    Ok(payload) => {
                        if write.send(Message::Text(payload.into())).await.is_err() {
                            let _ = incoming_tx.send(Err(Error::ChannelClosed));
                            break;
                        }
                    }
                    Err(err) => {
                        let _ = incoming_tx.send(Err(Error::Serialization { source: err }));
                    }
                }
            }
            message = read.next() => {
                match message {
                    Some(Ok(message)) => {
                        match message {
                            Message::Ping(payload) => {
                                let _ = write.send(Message::Pong(payload)).await;
                            }
                            Message::Close(_) => break,
                            other => match parse_server_message(other) {
                                Ok(Some(parsed)) => {
                                    let _ = incoming_tx.send(Ok(parsed));
                                }
                                Ok(None) => {}
                                Err(err) => {
                                    let _ = incoming_tx.send(Err(err));
                                }
                            },
                        }
                    }
                    Some(Err(err)) => {
                        let _ = incoming_tx.send(Err(Error::WebSocket { source: err }));
                        break;
                    }
                    None => break,
                }
            }
            _ = &mut shutdown_rx => {
                let _ = write.send(Message::Close(None)).await;
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_client_inner_with_api_key;
    use rust_genai_types::live_music_types::LiveMusicServerSetupComplete;
    use tokio::sync::{mpsc, oneshot};

    #[test]
    fn test_build_live_music_ws_url() {
        let (url, headers) = build_live_music_ws_url(
            "https://generativelanguage.googleapis.com/",
            "v1beta",
            "test-key",
        )
        .unwrap();
        assert!(url.as_str().starts_with("wss://"));
        assert!(
            url.as_str().contains("BidiGenerateMusic"),
            "missing music endpoint"
        );
        assert!(url.as_str().contains("key=test-key"));
        assert!(headers.contains_key("x-goog-api-key"));
    }

    #[tokio::test]
    async fn test_live_music_session_send_and_close() {
        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel();
        let (_incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let session = LiveMusicSession {
            outgoing_tx,
            incoming_rx,
            shutdown_tx: Some(shutdown_tx),
        };

        let err = session.set_weighted_prompts(vec![]).await.unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        session
            .set_weighted_prompts(vec![WeightedPrompt {
                text: Some("hi".to_string()),
                weight: Some(1.0),
            }])
            .await
            .unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(msg.client_content.is_some());

        session
            .set_music_generation_config(Some(LiveMusicGenerationConfig {
                temperature: Some(0.2),
                ..Default::default()
            }))
            .await
            .unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(msg.music_generation_config.is_some());

        session.play().await.unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(matches!(
            msg.playback_control,
            Some(LiveMusicPlaybackControl::Play)
        ));

        session.pause().await.unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(matches!(
            msg.playback_control,
            Some(LiveMusicPlaybackControl::Pause)
        ));

        session.stop().await.unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(matches!(
            msg.playback_control,
            Some(LiveMusicPlaybackControl::Stop)
        ));

        session.reset_context().await.unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(matches!(
            msg.playback_control,
            Some(LiveMusicPlaybackControl::ResetContext)
        ));

        session.close().await.unwrap();
        assert!(shutdown_rx.await.is_ok());
    }

    #[tokio::test]
    async fn test_live_music_session_send_channel_closed() {
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        drop(outgoing_rx);
        let (_incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let session = LiveMusicSession {
            outgoing_tx,
            incoming_rx,
            shutdown_tx: None,
        };
        let err = session.play().await.unwrap_err();
        assert!(matches!(err, Error::ChannelClosed));
    }

    #[tokio::test]
    async fn test_connect_live_music_session_errors() {
        let inner = Arc::new(test_client_inner_with_api_key(
            Backend::VertexAi,
            Some("key"),
        ));
        let err = connect_live_music_session(inner, "model".to_string())
            .await
            .err()
            .unwrap();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let inner = Arc::new(test_client_inner_with_api_key(Backend::GeminiApi, None));
        let err = connect_live_music_session(inner, "model".to_string())
            .await
            .err()
            .unwrap();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let inner = Arc::new(test_client_inner_with_api_key(
            Backend::GeminiApi,
            Some("auth_tokens/abc"),
        ));
        let err = connect_live_music_session(inner, "model".to_string())
            .await
            .err()
            .unwrap();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_live_music_ws_url_invalid_inputs() {
        let err = build_live_music_ws_url("://bad", "v1beta", "test-key").unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let err =
            build_live_music_ws_url("https://example.com/", "v1beta", "bad\nkey").unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_parse_live_music_message_variants() {
        let text_message = Message::Text(
            serde_json::to_string(&LiveMusicServerMessage {
                setup_complete: Some(LiveMusicServerSetupComplete::default()),
                ..Default::default()
            })
            .unwrap()
            .into(),
        );
        assert!(parse_server_message(text_message).unwrap().is_some());

        let bin_message = Message::Binary(
            serde_json::to_vec(&LiveMusicServerMessage {
                setup_complete: Some(LiveMusicServerSetupComplete::default()),
                ..Default::default()
            })
            .unwrap()
            .into(),
        );
        assert!(parse_server_message(bin_message).unwrap().is_some());

        assert!(parse_server_message(Message::Ping(vec![1].into()))
            .unwrap()
            .is_none());
        assert!(parse_server_message(Message::Close(None))
            .unwrap()
            .is_none());
        assert!(parse_server_message(Message::Text("bad-json".into())).is_err());
    }
}
