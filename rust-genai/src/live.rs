//! Live API (WebSocket) support.

use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Duration};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderMap, HeaderValue};
use tokio_tungstenite::tungstenite::Message;

use rust_genai_types::config::GenerationConfig;
use rust_genai_types::content::{Blob, Content};
use rust_genai_types::live_types::{
    AudioTranscriptionConfig, ContextWindowCompressionConfig, LiveClientContent, LiveClientMessage,
    LiveClientRealtimeInput, LiveClientSetup, LiveConnectConfig, LiveSendClientContentParameters,
    LiveSendRealtimeInputParameters, LiveSendToolResponseParameters, LiveServerMessage,
    SessionResumptionConfig,
};
use rust_genai_types::tool::Tool;

use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};
use crate::live_music::LiveMusic;

#[derive(Clone)]
pub struct Live {
    pub(crate) inner: Arc<ClientInner>,
}

impl Live {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 连接到 Live API。
    ///
    /// # Errors
    /// 当连接失败或配置无效时返回错误。
    pub async fn connect(
        &self,
        model: impl Into<String>,
        config: LiveConnectConfig,
    ) -> Result<LiveSession> {
        Box::pin(
            LiveSessionBuilder::new(self.inner.clone(), model.into())
                .with_config(config)
                .connect(),
        )
        .await
    }

    /// 创建 `LiveSessionBuilder`。
    #[must_use]
    pub fn builder(&self, model: impl Into<String>) -> LiveSessionBuilder {
        LiveSessionBuilder::new(self.inner.clone(), model.into())
    }

    /// 访问 Live Music API。
    #[must_use]
    pub fn music(&self) -> LiveMusic {
        LiveMusic::new(self.inner.clone())
    }
}

pub struct LiveSessionBuilder {
    inner: Arc<ClientInner>,
    model: String,
    config: LiveConnectConfig,
}

impl LiveSessionBuilder {
    pub(crate) fn new(inner: Arc<ClientInner>, model: String) -> Self {
        Self {
            inner,
            model,
            config: LiveConnectConfig::default(),
        }
    }

    /// 设置连接配置。
    #[must_use]
    pub fn with_config(mut self, config: LiveConnectConfig) -> Self {
        self.config = config;
        self
    }

    /// 设置系统指令。
    #[must_use]
    pub fn with_system_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.config.system_instruction = Some(Content::text(instruction));
        self
    }

    /// 设置工具列表。
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.config.tools = Some(tools);
        self
    }

    /// 设置生成配置。
    #[must_use]
    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.config.generation_config = Some(config);
        self
    }

    /// 启用会话恢复（自动获取 resumption handle）。
    #[must_use]
    pub fn with_session_resumption(mut self) -> Self {
        self.config.session_resumption = Some(SessionResumptionConfig {
            handle: None,
            transparent: None,
        });
        self
    }

    /// 使用指定的 resumption handle 恢复会话。
    #[must_use]
    pub fn with_session_resumption_handle(mut self, handle: impl Into<String>) -> Self {
        self.config.session_resumption = Some(SessionResumptionConfig {
            handle: Some(handle.into()),
            transparent: None,
        });
        self
    }

    /// 配置上下文窗口压缩。
    #[must_use]
    pub fn with_context_window_compression(
        mut self,
        config: ContextWindowCompressionConfig,
    ) -> Self {
        self.config.context_window_compression = Some(config);
        self
    }

    /// 配置输入音频转录。
    #[must_use]
    pub const fn with_input_audio_transcription(
        mut self,
        config: AudioTranscriptionConfig,
    ) -> Self {
        self.config.input_audio_transcription = Some(config);
        self
    }

    /// 配置输出音频转录。
    #[must_use]
    pub const fn with_output_audio_transcription(
        mut self,
        config: AudioTranscriptionConfig,
    ) -> Self {
        self.config.output_audio_transcription = Some(config);
        self
    }

    /// 连接并创建会话。
    ///
    /// # Errors
    /// 当连接失败或配置无效时返回错误。
    pub async fn connect(self) -> Result<LiveSession> {
        connect_live_session(self.inner, self.model, self.config).await
    }
}

/// Live 会话。
pub struct LiveSession {
    outgoing_tx: mpsc::UnboundedSender<LiveClientMessage>,
    incoming_rx: mpsc::UnboundedReceiver<Result<LiveServerMessage>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    pub session_id: Option<String>,
    resumption_state: Arc<Mutex<LiveSessionResumptionState>>,
    go_away_time_left: Arc<Mutex<Option<String>>>,
}

#[derive(Debug, Clone, Default)]
pub struct LiveSessionResumptionState {
    pub handle: Option<String>,
    pub resumable: Option<bool>,
    pub last_consumed_client_message_index: Option<String>,
}

impl LiveSession {
    /// 发送文本（turn-based）。
    ///
    /// # Errors
    /// 当发送失败或连接中断时返回错误。
    pub async fn send_text(&self, text: impl Into<String>) -> Result<()> {
        let message = LiveClientMessage {
            setup: None,
            client_content: Some(LiveClientContent {
                turns: Some(vec![Content::text(text)]),
                turn_complete: Some(true),
            }),
            realtime_input: None,
            tool_response: None,
        };
        self.send_async(message).await
    }

    /// 发送音频（realtime）。
    ///
    /// # Errors
    /// 当发送失败或连接中断时返回错误。
    pub async fn send_audio(&self, data: Vec<u8>, mime_type: impl Into<String>) -> Result<()> {
        let message = LiveClientMessage {
            setup: None,
            client_content: None,
            realtime_input: Some(LiveClientRealtimeInput {
                media_chunks: None,
                audio: Some(Blob {
                    mime_type: mime_type.into(),
                    data,
                    display_name: None,
                }),
                audio_stream_end: None,
                video: None,
                text: None,
                activity_start: None,
                activity_end: None,
            }),
            tool_response: None,
        };
        self.send_async(message).await
    }

    /// 发送 client content。
    ///
    /// # Errors
    /// 当发送失败或连接中断时返回错误。
    pub async fn send_client_content(&self, params: LiveSendClientContentParameters) -> Result<()> {
        let message = LiveClientMessage {
            setup: None,
            client_content: Some(LiveClientContent {
                turns: params.turns,
                turn_complete: params.turn_complete,
            }),
            realtime_input: None,
            tool_response: None,
        };
        self.send_async(message).await
    }

    /// 发送 realtime input。
    ///
    /// # Errors
    /// 当发送失败或连接中断时返回错误。
    pub async fn send_realtime_input(&self, params: LiveSendRealtimeInputParameters) -> Result<()> {
        let message = LiveClientMessage {
            setup: None,
            client_content: None,
            realtime_input: Some(LiveClientRealtimeInput {
                media_chunks: params.media.map(|media| vec![media]),
                audio: params.audio,
                audio_stream_end: params.audio_stream_end,
                video: params.video,
                text: params.text,
                activity_start: params.activity_start,
                activity_end: params.activity_end,
            }),
            tool_response: None,
        };
        self.send_async(message).await
    }

    /// 发送工具响应。
    ///
    /// # Errors
    /// 当发送失败或连接中断时返回错误。
    pub async fn send_tool_response(&self, params: LiveSendToolResponseParameters) -> Result<()> {
        let message = LiveClientMessage {
            setup: None,
            client_content: None,
            realtime_input: None,
            tool_response: Some(rust_genai_types::live_types::LiveClientToolResponse {
                function_responses: params.function_responses,
            }),
        };
        self.send_async(message).await
    }

    /// 接收服务器消息。
    pub async fn receive(&mut self) -> Option<Result<LiveServerMessage>> {
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

    /// 获取最新的会话恢复状态。
    pub fn resumption_state(&self) -> LiveSessionResumptionState {
        self.resumption_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// 获取最新的 resumption handle。
    pub fn resumption_handle(&self) -> Option<String> {
        self.resumption_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .handle
            .clone()
    }

    /// 获取最近一次 `GoAway` 的 `time_left`。
    pub fn last_go_away_time_left(&self) -> Option<String> {
        self.go_away_time_left
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn send(&self, message: LiveClientMessage) -> Result<()> {
        self.outgoing_tx
            .send(message)
            .map_err(|_| Error::ChannelClosed)?;
        Ok(())
    }

    async fn send_async(&self, message: LiveClientMessage) -> Result<()> {
        self.send(message)?;
        tokio::task::yield_now().await;
        Ok(())
    }
}

async fn connect_live_session(
    inner: Arc<ClientInner>,
    model: String,
    config: LiveConnectConfig,
) -> Result<LiveSession> {
    if config.http_options.is_some() {
        return Err(Error::InvalidConfig {
            message: "LiveConnectConfig.http_options is not supported yet".into(),
        });
    }

    if inner.config.backend == Backend::VertexAi {
        return Err(Error::InvalidConfig {
            message: "Live API for Vertex AI is not supported yet".into(),
        });
    }

    let api_key = inner
        .config
        .api_key
        .as_ref()
        .ok_or_else(|| Error::InvalidConfig {
            message: "API key required for Live API".into(),
        })?;

    let (url, headers) = build_live_ws_url(
        &inner.api_client.base_url,
        &inner.api_client.api_version,
        api_key,
    )?;

    let setup_timeout_ms = inner.config.http_options.timeout.unwrap_or(30_000);
    let request = build_ws_request(&url, &headers)?;
    let (ws_stream, _) = timeout(
        Duration::from_millis(setup_timeout_ms),
        connect_async(request),
    )
    .await
    .map_err(|_| Error::Timeout {
        message: format!("Timed out connecting to Live API after {setup_timeout_ms}ms"),
    })??;
    let (mut write, mut read) = ws_stream.split();

    let setup = build_live_setup(&model, &config);
    let setup_message = LiveClientMessage {
        setup: Some(setup),
        client_content: None,
        realtime_input: None,
        tool_response: None,
    };
    let payload = serde_json::to_string(&setup_message)?;
    write.send(Message::Text(payload.into())).await?;

    let (incoming_tx, incoming_rx) = mpsc::unbounded_channel();
    let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let resumption_state = Arc::new(Mutex::new(LiveSessionResumptionState::default()));
    let go_away_time_left = Arc::new(Mutex::new(None));

    let session_id = timeout(Duration::from_millis(setup_timeout_ms), async {
        loop {
            match read.next().await {
                Some(Ok(message)) => match message {
                    Message::Close(frame) => {
                        return Err(Error::Parse {
                            message: format!("WebSocket closed before setup_complete: {frame:?}"),
                        })
                    }
                    _ => {
                        if let Some(msg) = parse_server_message(message)? {
                            if let Some(setup) = msg.setup_complete.as_ref() {
                                return Ok(setup.session_id.clone());
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
    })
    .await
    .map_err(|_| Error::Timeout {
        message: format!(
            "Timed out waiting for Live API setup_complete after {setup_timeout_ms}ms"
        ),
    })??;

    tokio::spawn(message_loop(
        write,
        read,
        outgoing_rx,
        incoming_tx,
        shutdown_rx,
        resumption_state.clone(),
        go_away_time_left.clone(),
    ));

    Ok(LiveSession {
        outgoing_tx,
        incoming_rx,
        shutdown_tx: Some(shutdown_tx),
        session_id,
        resumption_state,
        go_away_time_left,
    })
}

fn build_live_setup(model: &str, config: &LiveConnectConfig) -> LiveClientSetup {
    let model = normalize_model_name(model);
    let generation_config = merge_generation_config(config);

    LiveClientSetup {
        model: Some(model),
        generation_config,
        system_instruction: config.system_instruction.clone(),
        tools: config.tools.clone(),
        realtime_input_config: config.realtime_input_config.clone(),
        session_resumption: config.session_resumption.clone(),
        context_window_compression: config.context_window_compression.clone(),
        input_audio_transcription: config.input_audio_transcription.clone(),
        output_audio_transcription: config.output_audio_transcription.clone(),
        proactivity: config.proactivity.clone(),
        explicit_vad_signal: config.explicit_vad_signal,
    }
}

fn merge_generation_config(config: &LiveConnectConfig) -> Option<GenerationConfig> {
    let mut generation_config = config.generation_config.clone().unwrap_or_default();
    let updated = config.generation_config.is_some()
        || config.response_modalities.is_some()
        || config.temperature.is_some()
        || config.top_p.is_some()
        || config.top_k.is_some()
        || config.max_output_tokens.is_some()
        || config.media_resolution.is_some()
        || config.seed.is_some()
        || config.speech_config.is_some()
        || config.thinking_config.is_some()
        || config.enable_affective_dialog.is_some();

    if let Some(value) = config.response_modalities.clone() {
        generation_config.response_modalities = Some(value);
    }
    if let Some(value) = config.temperature {
        generation_config.temperature = Some(value);
    }
    if let Some(value) = config.top_p {
        generation_config.top_p = Some(value);
    }
    if let Some(value) = config.top_k {
        let top_k_value = i16::try_from(value).unwrap_or_else(|_| {
            if value > i32::from(i16::MAX) {
                i16::MAX
            } else {
                i16::MIN
            }
        });
        generation_config.top_k = Some(f32::from(top_k_value));
    }
    if let Some(value) = config.max_output_tokens {
        generation_config.max_output_tokens = Some(value);
    }
    if let Some(value) = config.media_resolution {
        generation_config.media_resolution = Some(value);
    }
    if let Some(value) = config.seed {
        generation_config.seed = Some(value);
    }
    if let Some(value) = config.speech_config.clone() {
        generation_config.speech_config = Some(value);
    }
    if let Some(value) = config.thinking_config.clone() {
        generation_config.thinking_config = Some(value);
    }
    if let Some(value) = config.enable_affective_dialog {
        generation_config.enable_affective_dialog = Some(value);
    }

    updated.then_some(generation_config)
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

fn build_live_ws_url(base_url: &str, api_version: &str, api_key: &str) -> Result<(Url, HeaderMap)> {
    if api_key.starts_with("auth_tokens/") && api_version != "v1alpha" {
        return Err(Error::InvalidConfig {
            message: "Ephemeral tokens require v1alpha for Live API".into(),
        });
    }
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
    let method = if api_key.starts_with("auth_tokens/") {
        "BidiGenerateContentConstrained"
    } else {
        "BidiGenerateContent"
    };
    let path = format!(
        "{base_path}/ws/google.ai.generativelanguage.{api_version}.GenerativeService.{method}"
    );
    url.set_path(&path);

    let mut headers = HeaderMap::new();
    if api_key.starts_with("auth_tokens/") {
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Token {api_key}")).map_err(|_| {
                Error::InvalidConfig {
                    message: "Invalid ephemeral token".into(),
                }
            })?,
        );
    } else {
        headers.insert(
            "x-goog-api-key",
            HeaderValue::from_str(api_key).map_err(|_| Error::InvalidConfig {
                message: "Invalid API key".into(),
            })?,
        );
    }

    Ok((url, headers))
}

fn normalize_model_name(model: &str) -> String {
    if model.starts_with("models/") {
        model.to_string()
    } else {
        format!("models/{model}")
    }
}

fn parse_server_message(message: Message) -> Result<Option<LiveServerMessage>> {
    match message {
        Message::Text(text) => {
            let msg = serde_json::from_str::<LiveServerMessage>(&text)?;
            Ok(Some(msg))
        }
        Message::Binary(data) => {
            let msg = serde_json::from_slice::<LiveServerMessage>(&data)?;
            Ok(Some(msg))
        }
        Message::Ping(_) | Message::Pong(_) | Message::Close(_) | Message::Frame(_) => Ok(None),
    }
}

fn update_resumption_state(
    state: &Arc<Mutex<LiveSessionResumptionState>>,
    message: &LiveServerMessage,
) {
    if let Some(update) = message.session_resumption_update.as_ref() {
        let mut guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if update.new_handle.is_some() || update.resumable.is_some() {
            guard.handle.clone_from(&update.new_handle);
        }
        if update.resumable.is_some() {
            guard.resumable = update.resumable;
        }
        if update.last_consumed_client_message_index.is_some() {
            guard
                .last_consumed_client_message_index
                .clone_from(&update.last_consumed_client_message_index);
        }
    }
}

fn update_go_away(state: &Arc<Mutex<Option<String>>>, message: &LiveServerMessage) {
    if let Some(go_away) = message.go_away.as_ref() {
        let mut guard = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.clone_from(&go_away.time_left);
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
    mut outgoing_rx: mpsc::UnboundedReceiver<LiveClientMessage>,
    incoming_tx: mpsc::UnboundedSender<Result<LiveServerMessage>>,
    mut shutdown_rx: oneshot::Receiver<()>,
    resumption_state: Arc<Mutex<LiveSessionResumptionState>>,
    go_away_time_left: Arc<Mutex<Option<String>>>,
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
                                    update_resumption_state(&resumption_state, &parsed);
                                    update_go_away(&go_away_time_left, &parsed);
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
    use rust_genai_types::config::{SpeechConfig, ThinkingConfig};
    use rust_genai_types::enums::{MediaResolution, Modality};
    use rust_genai_types::live_types::{
        LiveServerGoAway, LiveServerMessage, LiveServerSessionResumptionUpdate,
    };
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn test_build_live_ws_url() {
        let (url, headers) = build_live_ws_url(
            "https://generativelanguage.googleapis.com/",
            "v1beta",
            "test-key",
        )
        .unwrap();
        assert!(url.as_str().starts_with("wss://"));
        assert_eq!(
            url.as_str(),
            "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent"
        );
        assert!(headers.contains_key("x-goog-api-key"));
    }

    #[test]
    fn test_build_live_ws_url_with_ephemeral_token() {
        let (_url, headers) = build_live_ws_url(
            "https://generativelanguage.googleapis.com/",
            "v1alpha",
            "auth_tokens/abc",
        )
        .unwrap();
        assert!(headers.contains_key("authorization"));
        assert!(!headers.contains_key("x-goog-api-key"));
    }

    #[test]
    fn test_build_live_ws_url_invalid_key() {
        let err = build_live_ws_url(
            "https://generativelanguage.googleapis.com/",
            "v1beta",
            "bad\nkey",
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_merge_generation_config() {
        let config = LiveConnectConfig {
            response_modalities: Some(vec![Modality::Text]),
            temperature: Some(0.7),
            ..LiveConnectConfig::default()
        };
        let generation = merge_generation_config(&config).unwrap();
        assert_eq!(generation.response_modalities.unwrap().len(), 1);
        assert_eq!(generation.temperature, Some(0.7));
    }

    #[test]
    fn test_build_live_setup_and_ws_request() {
        let config = LiveConnectConfig {
            response_modalities: Some(vec![Modality::Text]),
            temperature: Some(0.5),
            ..LiveConnectConfig::default()
        };
        let setup = build_live_setup("gemini-2.0-flash", &config);
        assert_eq!(setup.model.as_deref(), Some("models/gemini-2.0-flash"));
        assert!(setup.generation_config.is_some());

        let (url, headers) =
            build_live_ws_url("https://example.com/", "v1beta", "test-key").unwrap();
        let request = build_ws_request(&url, &headers).unwrap();
        assert!(request.headers().contains_key("x-goog-api-key"));
    }

    #[test]
    fn test_live_builder_and_music_accessors() {
        let inner = Arc::new(test_client_inner_with_api_key(
            Backend::GeminiApi,
            Some("key"),
        ));
        let live = Live::new(inner);
        let builder = live.builder("gemini-2.0-flash");
        assert_eq!(builder.model, "gemini-2.0-flash");
        let _music = live.music();
    }

    #[test]
    fn test_merge_generation_config_all_fields() {
        let config = LiveConnectConfig {
            response_modalities: Some(vec![Modality::Text]),
            temperature: Some(0.7),
            top_p: Some(0.9),
            top_k: Some(32),
            max_output_tokens: Some(256),
            media_resolution: Some(MediaResolution::MediaResolutionHigh),
            seed: Some(42),
            speech_config: Some(SpeechConfig::default()),
            thinking_config: Some(ThinkingConfig::default()),
            enable_affective_dialog: Some(true),
            ..LiveConnectConfig::default()
        };
        let generation = merge_generation_config(&config).unwrap();
        assert_eq!(generation.top_p, Some(0.9));
        assert_eq!(generation.top_k, Some(32.0));
        assert_eq!(generation.max_output_tokens, Some(256));
        assert_eq!(generation.seed, Some(42));
        assert!(generation.speech_config.is_some());
        assert!(generation.thinking_config.is_some());
        assert_eq!(generation.enable_affective_dialog, Some(true));
    }

    #[test]
    fn test_build_ws_request_invalid_scheme() {
        let url = Url::parse("file:///tmp/socket").unwrap();
        let err = build_ws_request(&url, &HeaderMap::new()).unwrap_err();
        assert!(matches!(err, Error::Parse { .. }));
    }

    #[test]
    fn test_build_live_ws_url_scheme_variants_and_invalid_token() {
        let (url, _) = build_live_ws_url("ws://example.com/", "v1beta", "test-key").unwrap();
        assert!(url.as_str().starts_with("ws://"));
        let (url, _) = build_live_ws_url("wss://example.com/", "v1beta", "test-key").unwrap();
        assert!(url.as_str().starts_with("wss://"));

        let err =
            build_live_ws_url("https://example.com/", "v1alpha", "auth_tokens/bad\n").unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_normalize_model_name_with_prefix() {
        assert_eq!(
            normalize_model_name("models/gemini-2.0-flash"),
            "models/gemini-2.0-flash"
        );
    }

    #[test]
    fn test_poisoned_mutex_accessors() {
        let state = Arc::new(Mutex::new(LiveSessionResumptionState {
            handle: Some("handle".into()),
            resumable: Some(true),
            last_consumed_client_message_index: Some("idx".into()),
        }));
        let go_away = Arc::new(Mutex::new(Some("5s".into())));
        let state_clone = Arc::clone(&state);
        let go_away_clone = Arc::clone(&go_away);
        let _ = std::thread::spawn(move || {
            let _guard = state_clone.lock().unwrap();
            let _guard2 = go_away_clone.lock().unwrap();
            panic!("poison");
        })
        .join();

        let (outgoing_tx, _outgoing_rx) = mpsc::unbounded_channel();
        let (_incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let session = LiveSession {
            outgoing_tx,
            incoming_rx,
            shutdown_tx: None,
            session_id: None,
            resumption_state: state,
            go_away_time_left: go_away,
        };
        assert_eq!(session.resumption_handle().as_deref(), Some("handle"));
        assert_eq!(session.last_go_away_time_left().as_deref(), Some("5s"));
        let state = session.resumption_state();
        assert_eq!(
            state.last_consumed_client_message_index.as_deref(),
            Some("idx")
        );
    }

    #[test]
    fn test_parse_message_and_state_updates() {
        let message = Message::Text(
            serde_json::to_string(&LiveServerMessage {
                session_resumption_update: Some(LiveServerSessionResumptionUpdate {
                    new_handle: Some("handle".to_string()),
                    resumable: Some(true),
                    last_consumed_client_message_index: Some("1".to_string()),
                }),
                go_away: Some(LiveServerGoAway {
                    time_left: Some("5s".to_string()),
                }),
                ..LiveServerMessage {
                    setup_complete: None,
                    server_content: None,
                    tool_call: None,
                    tool_call_cancellation: None,
                    usage_metadata: None,
                    voice_activity_detection_signal: None,
                    session_resumption_update: None,
                    go_away: None,
                }
            })
            .unwrap()
            .into(),
        );

        let parsed = parse_server_message(message).unwrap().unwrap();
        let state = Arc::new(Mutex::new(LiveSessionResumptionState::default()));
        update_resumption_state(&state, &parsed);
        let guard = state.lock().unwrap();
        assert_eq!(guard.handle.as_deref(), Some("handle"));
        assert_eq!(guard.resumable, Some(true));
        drop(guard);

        let go_away = Arc::new(Mutex::new(None));
        update_go_away(&go_away, &parsed);
        assert_eq!(*go_away.lock().unwrap(), Some("5s".to_string()));

        let bin_message = Message::Binary(
            serde_json::to_vec(&LiveServerMessage {
                setup_complete: None,
                server_content: None,
                tool_call: None,
                tool_call_cancellation: None,
                usage_metadata: None,
                go_away: None,
                session_resumption_update: None,
                voice_activity_detection_signal: None,
            })
            .unwrap()
            .into(),
        );
        assert!(parse_server_message(bin_message).unwrap().is_some());
    }

    #[test]
    fn test_parse_server_message_variants() {
        assert!(parse_server_message(Message::Ping(vec![1].into()))
            .unwrap()
            .is_none());
        assert!(parse_server_message(Message::Close(None))
            .unwrap()
            .is_none());
        assert!(parse_server_message(Message::Text("not-json".into())).is_err());
    }

    #[test]
    fn test_update_state_with_partial_resumption_update() {
        let message = LiveServerMessage {
            session_resumption_update: Some(LiveServerSessionResumptionUpdate {
                new_handle: None,
                resumable: None,
                last_consumed_client_message_index: Some("2".to_string()),
            }),
            setup_complete: None,
            server_content: None,
            tool_call: None,
            tool_call_cancellation: None,
            usage_metadata: None,
            voice_activity_detection_signal: None,
            go_away: None,
        };
        let state = Arc::new(Mutex::new(LiveSessionResumptionState {
            handle: Some("keep".into()),
            resumable: Some(false),
            last_consumed_client_message_index: None,
        }));
        update_resumption_state(&state, &message);
        let guard = state.lock().unwrap();
        assert_eq!(guard.handle.as_deref(), Some("keep"));
        assert_eq!(guard.resumable, Some(false));
        assert_eq!(
            guard.last_consumed_client_message_index.as_deref(),
            Some("2")
        );
        drop(guard);

        let go_away = Arc::new(Mutex::new(Some("stay".to_string())));
        update_go_away(&go_away, &message);
        assert_eq!(*go_away.lock().unwrap(), Some("stay".to_string()));
    }

    #[test]
    fn test_live_builder_config_chain() {
        let inner = Arc::new(test_client_inner_with_api_key(
            Backend::GeminiApi,
            Some("key"),
        ));
        let builder = LiveSessionBuilder::new(inner, "gemini-2.0-flash".to_string())
            .with_system_instruction("sys")
            .with_tools(vec![Tool::default()])
            .with_generation_config(GenerationConfig::default())
            .with_session_resumption()
            .with_context_window_compression(ContextWindowCompressionConfig {
                trigger_tokens: None,
                sliding_window: None,
            })
            .with_input_audio_transcription(AudioTranscriptionConfig::default())
            .with_output_audio_transcription(AudioTranscriptionConfig::default());

        assert_eq!(builder.model, "gemini-2.0-flash");
        assert!(builder.config.system_instruction.is_some());
        assert!(builder.config.tools.is_some());
        assert!(builder.config.generation_config.is_some());
        assert!(builder.config.session_resumption.is_some());
        assert!(builder.config.context_window_compression.is_some());
        assert!(builder.config.input_audio_transcription.is_some());
        assert!(builder.config.output_audio_transcription.is_some());

        let builder = builder.with_session_resumption_handle("handle");
        assert_eq!(
            builder
                .config
                .session_resumption
                .as_ref()
                .and_then(|cfg| cfg.handle.as_deref()),
            Some("handle")
        );
    }

    #[tokio::test]
    async fn test_live_session_send_and_close() {
        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel();
        let (_incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let session = LiveSession {
            outgoing_tx,
            incoming_rx,
            shutdown_tx: Some(shutdown_tx),
            session_id: Some("session".to_string()),
            resumption_state: Arc::new(Mutex::new(LiveSessionResumptionState::default())),
            go_away_time_left: Arc::new(Mutex::new(None)),
        };

        session.send_text("hi").await.unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(msg.client_content.is_some());
        assert!(msg.realtime_input.is_none());

        session
            .send_audio(vec![1, 2, 3], "audio/pcm")
            .await
            .unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(msg.realtime_input.as_ref().unwrap().audio.is_some());

        session
            .send_client_content(LiveSendClientContentParameters {
                turns: Some(vec![Content::text("turn")]),
                turn_complete: Some(false),
            })
            .await
            .unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(msg.client_content.is_some());

        session
            .send_realtime_input(LiveSendRealtimeInputParameters {
                media: Some(Blob {
                    mime_type: "audio/pcm".to_string(),
                    data: vec![9],
                    display_name: None,
                }),
                audio: None,
                audio_stream_end: Some(true),
                video: None,
                text: Some("rt".to_string()),
                activity_start: None,
                activity_end: None,
            })
            .await
            .unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(msg.realtime_input.is_some());

        session
            .send_tool_response(LiveSendToolResponseParameters {
                function_responses: None,
            })
            .await
            .unwrap();
        let msg = outgoing_rx.recv().await.unwrap();
        assert!(msg.tool_response.is_some());

        session.close().await.unwrap();
        assert!(shutdown_rx.await.is_ok());
    }

    #[tokio::test]
    async fn test_live_session_send_channel_closed() {
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        drop(outgoing_rx);
        let (_incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let session = LiveSession {
            outgoing_tx,
            incoming_rx,
            shutdown_tx: None,
            session_id: None,
            resumption_state: Arc::new(Mutex::new(LiveSessionResumptionState::default())),
            go_away_time_left: Arc::new(Mutex::new(None)),
        };
        let err = session.send_text("hi").await.unwrap_err();
        assert!(matches!(err, Error::ChannelClosed));
    }

    #[test]
    fn test_live_session_state_accessors() {
        let (outgoing_tx, _outgoing_rx) = mpsc::unbounded_channel();
        let (_incoming_tx, incoming_rx) = mpsc::unbounded_channel();
        let state = Arc::new(Mutex::new(LiveSessionResumptionState {
            handle: Some("h".to_string()),
            resumable: Some(true),
            last_consumed_client_message_index: Some("7".to_string()),
        }));
        let go_away = Arc::new(Mutex::new(Some("10s".to_string())));
        let session = LiveSession {
            outgoing_tx,
            incoming_rx,
            shutdown_tx: None,
            session_id: None,
            resumption_state: state,
            go_away_time_left: go_away,
        };
        assert_eq!(session.resumption_handle().as_deref(), Some("h"));
        assert_eq!(session.last_go_away_time_left().as_deref(), Some("10s"));
        let state = session.resumption_state();
        assert_eq!(
            state.last_consumed_client_message_index.as_deref(),
            Some("7")
        );
    }

    #[tokio::test]
    async fn test_connect_live_session_errors() {
        let inner = Arc::new(test_client_inner_with_api_key(
            Backend::GeminiApi,
            Some("key"),
        ));
        let config = LiveConnectConfig {
            http_options: Some(rust_genai_types::http::HttpOptions::default()),
            ..Default::default()
        };
        let err = connect_live_session(inner, "model".to_string(), config)
            .await
            .err()
            .unwrap();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let inner = Arc::new(test_client_inner_with_api_key(
            Backend::VertexAi,
            Some("key"),
        ));
        let err = connect_live_session(inner, "model".to_string(), LiveConnectConfig::default())
            .await
            .err()
            .unwrap();
        assert!(matches!(err, Error::InvalidConfig { .. }));

        let inner = Arc::new(test_client_inner_with_api_key(Backend::GeminiApi, None));
        let err = connect_live_session(inner, "model".to_string(), LiveConnectConfig::default())
            .await
            .err()
            .unwrap();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_live_ws_url_ephemeral_requires_v1alpha() {
        let err = build_live_ws_url(
            "https://generativelanguage.googleapis.com/",
            "v1beta",
            "auth_tokens/abc",
        )
        .unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }

    #[test]
    fn test_build_live_ws_url_invalid_base_url() {
        let err = build_live_ws_url("://bad-url", "v1beta", "test-key").unwrap_err();
        assert!(matches!(err, Error::InvalidConfig { .. }));
    }
}
