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
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 连接到 Live API。
    pub async fn connect(
        &self,
        model: impl Into<String>,
        config: LiveConnectConfig,
    ) -> Result<LiveSession> {
        LiveSessionBuilder::new(self.inner.clone(), model.into())
            .with_config(config)
            .connect()
            .await
    }

    /// 创建 LiveSessionBuilder。
    pub fn builder(&self, model: impl Into<String>) -> LiveSessionBuilder {
        LiveSessionBuilder::new(self.inner.clone(), model.into())
    }

    /// 访问 Live Music API。
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
    pub fn with_config(mut self, config: LiveConnectConfig) -> Self {
        self.config = config;
        self
    }

    /// 设置系统指令。
    pub fn with_system_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.config.system_instruction = Some(Content::text(instruction));
        self
    }

    /// 设置工具列表。
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.config.tools = Some(tools);
        self
    }

    /// 设置生成配置。
    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.config.generation_config = Some(config);
        self
    }

    /// 启用会话恢复（自动获取 resumption handle）。
    pub fn with_session_resumption(mut self) -> Self {
        self.config.session_resumption = Some(SessionResumptionConfig {
            handle: None,
            transparent: None,
        });
        self
    }

    /// 使用指定的 resumption handle 恢复会话。
    pub fn with_session_resumption_handle(mut self, handle: impl Into<String>) -> Self {
        self.config.session_resumption = Some(SessionResumptionConfig {
            handle: Some(handle.into()),
            transparent: None,
        });
        self
    }

    /// 配置上下文窗口压缩。
    pub fn with_context_window_compression(
        mut self,
        config: ContextWindowCompressionConfig,
    ) -> Self {
        self.config.context_window_compression = Some(config);
        self
    }

    /// 配置输入音频转录。
    pub fn with_input_audio_transcription(mut self, config: AudioTranscriptionConfig) -> Self {
        self.config.input_audio_transcription = Some(config);
        self
    }

    /// 配置输出音频转录。
    pub fn with_output_audio_transcription(mut self, config: AudioTranscriptionConfig) -> Self {
        self.config.output_audio_transcription = Some(config);
        self
    }

    /// 连接并创建会话。
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
        self.send(message)
    }

    /// 发送音频（realtime）。
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
        self.send(message)
    }

    /// 发送 client content。
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
        self.send(message)
    }

    /// 发送 realtime input。
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
        self.send(message)
    }

    /// 发送工具响应。
    pub async fn send_tool_response(&self, params: LiveSendToolResponseParameters) -> Result<()> {
        let message = LiveClientMessage {
            setup: None,
            client_content: None,
            realtime_input: None,
            tool_response: Some(rust_genai_types::live_types::LiveClientToolResponse {
                function_responses: params.function_responses,
            }),
        };
        self.send(message)
    }

    /// 接收服务器消息。
    pub async fn receive(&mut self) -> Option<Result<LiveServerMessage>> {
        self.incoming_rx.recv().await
    }

    /// 关闭会话。
    pub async fn close(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        Ok(())
    }

    /// 获取最新的会话恢复状态。
    pub fn resumption_state(&self) -> LiveSessionResumptionState {
        self.resumption_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 获取最新的 resumption handle。
    pub fn resumption_handle(&self) -> Option<String> {
        self.resumption_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .handle
            .clone()
    }

    /// 获取最近一次 GoAway 的 time_left。
    pub fn last_go_away_time_left(&self) -> Option<String> {
        self.go_away_time_left
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn send(&self, message: LiveClientMessage) -> Result<()> {
        self.outgoing_tx
            .send(message)
            .map_err(|_| Error::ChannelClosed)?;
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
    let request = build_ws_request(url, headers)?;
    let (ws_stream, _) = timeout(
        Duration::from_millis(setup_timeout_ms),
        connect_async(request),
    )
    .await
    .map_err(|_| Error::Timeout {
        message: format!("Timed out connecting to Live API after {setup_timeout_ms}ms"),
    })??;
    let (mut write, mut read) = ws_stream.split();

    let setup = build_live_setup(model, &config)?;
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

fn build_live_setup(model: String, config: &LiveConnectConfig) -> Result<LiveClientSetup> {
    let model = normalize_model_name(&model);
    let generation_config = merge_generation_config(config);

    Ok(LiveClientSetup {
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
    })
}

fn merge_generation_config(config: &LiveConnectConfig) -> Option<GenerationConfig> {
    let mut generation_config = config.generation_config.clone().unwrap_or_default();
    let mut updated = config.generation_config.is_some();

    if let Some(value) = config.response_modalities.clone() {
        generation_config.response_modalities = Some(value);
        updated = true;
    }
    if let Some(value) = config.temperature {
        generation_config.temperature = Some(value);
        updated = true;
    }
    if let Some(value) = config.top_p {
        generation_config.top_p = Some(value);
        updated = true;
    }
    if let Some(value) = config.top_k {
        generation_config.top_k = Some(value as f32);
        updated = true;
    }
    if let Some(value) = config.max_output_tokens {
        generation_config.max_output_tokens = Some(value);
        updated = true;
    }
    if let Some(value) = config.media_resolution {
        generation_config.media_resolution = Some(value);
        updated = true;
    }
    if let Some(value) = config.seed {
        generation_config.seed = Some(value);
        updated = true;
    }
    if let Some(value) = config.speech_config.clone() {
        generation_config.speech_config = Some(value);
        updated = true;
    }
    if let Some(value) = config.thinking_config.clone() {
        generation_config.thinking_config = Some(value);
        updated = true;
    }
    if let Some(value) = config.enable_affective_dialog {
        generation_config.enable_affective_dialog = Some(value);
        updated = true;
    }

    if updated {
        Some(generation_config)
    } else {
        None
    }
}

fn build_ws_request(
    url: Url,
    headers: HeaderMap,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>> {
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|err| Error::Parse {
            message: err.to_string(),
        })?;
    {
        let request_headers = request.headers_mut();
        for (key, value) in headers.iter() {
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
        "https" => "wss",
        "http" => "ws",
        "wss" => "wss",
        "ws" => "ws",
        _ => "wss",
    };
    url.set_scheme(scheme).map_err(|_| Error::InvalidConfig {
        message: "Invalid base_url scheme".into(),
    })?;

    let base_path = url.path().trim_end_matches('/');
    let method = if api_key.starts_with("auth_tokens/") {
        "BidiGenerateContentConstrained"
    } else {
        "BidiGenerateContent"
    };
    let path = format!(
        "{}/ws/google.ai.generativelanguage.{}.GenerativeService.{}",
        base_path, api_version, method
    );
    url.set_path(&path);

    let mut headers = HeaderMap::new();
    if api_key.starts_with("auth_tokens/") {
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Token {}", api_key)).map_err(|_| {
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
        Message::Ping(_) | Message::Pong(_) => Ok(None),
        Message::Close(_) => Ok(None),
        _ => Ok(None),
    }
}

fn update_resumption_state(
    state: &Arc<Mutex<LiveSessionResumptionState>>,
    message: &LiveServerMessage,
) {
    if let Some(update) = message.session_resumption_update.as_ref() {
        let mut guard = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if update.new_handle.is_some() || update.resumable.is_some() {
            guard.handle = update.new_handle.clone();
        }
        if update.resumable.is_some() {
            guard.resumable = update.resumable;
        }
        if update.last_consumed_client_message_index.is_some() {
            guard.last_consumed_client_message_index =
                update.last_consumed_client_message_index.clone();
        }
    }
}

fn update_go_away(state: &Arc<Mutex<Option<String>>>, message: &LiveServerMessage) {
    if let Some(go_away) = message.go_away.as_ref() {
        let mut guard = state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = go_away.time_left.clone();
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
    use rust_genai_types::enums::Modality;

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
}
