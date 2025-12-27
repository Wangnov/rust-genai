//! Chats API surface.

use std::sync::Arc;

use futures_util::Stream;
use futures_util::StreamExt;
use tokio::sync::RwLock;

use rust_genai_types::content::Content;
use rust_genai_types::models::GenerateContentConfig;
use rust_genai_types::response::GenerateContentResponse;

use crate::afc::CallableTool;
use crate::client::ClientInner;
use crate::error::Result;
use crate::models::Models;

#[derive(Clone)]
pub struct Chats {
    pub(crate) inner: Arc<ClientInner>,
}

impl Chats {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 创建新会话。
    pub fn create(&self, model: impl Into<String>) -> ChatSession {
        ChatSession::new(self.inner.clone(), model.into())
    }

    /// 带配置创建会话。
    pub fn create_with_config(
        &self,
        model: impl Into<String>,
        config: GenerateContentConfig,
    ) -> ChatSession {
        ChatSession::with_config(self.inner.clone(), model.into(), config)
    }
}

/// Chat 会话。
#[derive(Clone)]
pub struct ChatSession {
    client: Arc<ClientInner>,
    model: String,
    history: Arc<RwLock<Vec<Content>>>,
    config: GenerateContentConfig,
}

impl ChatSession {
    fn new(client: Arc<ClientInner>, model: String) -> Self {
        Self {
            client,
            model,
            history: Arc::new(RwLock::new(Vec::new())),
            config: GenerateContentConfig::default(),
        }
    }

    fn with_config(client: Arc<ClientInner>, model: String, config: GenerateContentConfig) -> Self {
        Self {
            client,
            model,
            history: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// 发送消息。
    pub async fn send_message(
        &self,
        message: impl Into<String>,
    ) -> Result<GenerateContentResponse> {
        let user_content = Content::text(message);

        {
            let mut history = self.history.write().await;
            history.push(user_content.clone());
        }

        let models = Models::new(self.client.clone());
        let history = self.history.read().await.clone();

        let response = models
            .generate_content_with_config(&self.model, history, self.config.clone())
            .await?;

        if let Some(candidate) = response.candidates.first() {
            if let Some(content) = &candidate.content {
                let mut history = self.history.write().await;
                history.push(content.clone());
            }
        }

        Ok(response)
    }

    /// 发送消息（兼容别名）。
    pub async fn send(&self, message: impl Into<String>) -> Result<GenerateContentResponse> {
        self.send_message(message).await
    }

    /// 流式发送消息。
    pub async fn send_message_stream(
        &self,
        message: impl Into<String>,
    ) -> Result<impl Stream<Item = Result<GenerateContentResponse>>> {
        let user_content = Content::text(message);

        {
            let mut history = self.history.write().await;
            history.push(user_content.clone());
        }

        let models = Models::new(self.client.clone());
        let history = self.history.read().await.clone();

        let stream = models
            .generate_content_stream(&self.model, history, self.config.clone())
            .await?;

        let history_ref = self.history.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(8);

        tokio::spawn(async move {
            let mut stream = stream;
            let mut last_content: Option<Content> = None;

            while let Some(item) = stream.next().await {
                if let Ok(response) = &item {
                    if let Some(candidate) = response.candidates.first() {
                        if let Some(content) = &candidate.content {
                            last_content = Some(content.clone());
                        }
                    }
                }

                if tx.send(item).await.is_err() {
                    break;
                }
            }

            if let Some(content) = last_content {
                let mut history = history_ref.write().await;
                history.push(content);
            }
        });

        let output = futures_util::stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        });

        Ok(output)
    }

    /// 流式发送消息（兼容别名）。
    pub async fn send_stream(
        &self,
        message: impl Into<String>,
    ) -> Result<impl Stream<Item = Result<GenerateContentResponse>>> {
        self.send_message_stream(message).await
    }

    /// 发送消息（自动函数调用 + callable tools）。
    pub async fn send_message_with_callable_tools(
        &self,
        message: impl Into<String>,
        callable_tools: Vec<Box<dyn CallableTool>>,
    ) -> Result<GenerateContentResponse> {
        let user_content = Content::text(message);

        {
            let mut history = self.history.write().await;
            history.push(user_content.clone());
        }

        let models = Models::new(self.client.clone());
        let history = self.history.read().await.clone();

        let response = models
            .generate_content_with_callable_tools(
                &self.model,
                history,
                self.config.clone(),
                callable_tools,
            )
            .await?;

        if let Some(afc_history) = response.automatic_function_calling_history.clone() {
            let mut history = self.history.write().await;
            *history = afc_history;
        }

        if let Some(candidate) = response.candidates.first() {
            if let Some(content) = &candidate.content {
                let mut history = self.history.write().await;
                history.push(content.clone());
            }
        }

        Ok(response)
    }

    /// 流式发送消息（自动函数调用 + callable tools）。
    pub async fn send_message_stream_with_callable_tools(
        &self,
        message: impl Into<String>,
        callable_tools: Vec<Box<dyn CallableTool>>,
    ) -> Result<impl Stream<Item = Result<GenerateContentResponse>>> {
        let user_content = Content::text(message);

        {
            let mut history = self.history.write().await;
            history.push(user_content.clone());
        }

        let models = Models::new(self.client.clone());
        let history = self.history.read().await.clone();

        let stream = models
            .generate_content_stream_with_callable_tools(
                &self.model,
                history,
                self.config.clone(),
                callable_tools,
            )
            .await?;

        let history_ref = self.history.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(8);

        tokio::spawn(async move {
            let mut stream = stream;
            let mut last_content: Option<Content> = None;
            let mut last_afc_history: Option<Vec<Content>> = None;

            while let Some(item) = stream.next().await {
                if let Ok(response) = &item {
                    if let Some(content) = response
                        .candidates
                        .first()
                        .and_then(|candidate| candidate.content.clone())
                    {
                        last_content = Some(content);
                    }

                    if let Some(history) = response.automatic_function_calling_history.clone() {
                        last_afc_history = Some(history);
                    }
                }

                if tx.send(item).await.is_err() {
                    break;
                }
            }

            if let Some(history) = last_afc_history {
                let mut history_ref = history_ref.write().await;
                *history_ref = history;
            }

            if let Some(content) = last_content {
                let mut history = history_ref.write().await;
                history.push(content);
            }
        });

        let output = futures_util::stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        });

        Ok(output)
    }

    /// 获取历史。
    pub async fn history(&self) -> Vec<Content> {
        self.history.read().await.clone()
    }

    /// 清空历史。
    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }
}
