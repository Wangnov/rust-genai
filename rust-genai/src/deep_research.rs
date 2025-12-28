//! Deep Research convenience wrapper (Preview).

use std::pin::Pin;
use std::sync::Arc;

use futures_util::Stream;
use rust_genai_types::interactions::{
    CreateInteractionConfig, Interaction, InteractionEvent, InteractionInput,
    InteractionThinkingSummaries,
};
use rust_genai_types::tool::{GoogleSearch, Tool};

use crate::client::ClientInner;
use crate::error::Result;
use crate::interactions::Interactions;

#[derive(Clone)]
pub struct DeepResearch {
    inner: Arc<ClientInner>,
}

impl DeepResearch {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 启动 Deep Research（默认配置）。
    ///
    /// # Errors
    ///
    /// 当创建交互请求失败或服务端返回错误时返回错误。
    pub async fn start(
        &self,
        model: impl Into<String>,
        input: impl Into<InteractionInput>,
    ) -> Result<Interaction> {
        let mut config = CreateInteractionConfig::new(model, input);
        apply_deep_research_defaults(&mut config);
        Interactions::new(self.inner.clone()).create(config).await
    }

    /// 启动 Deep Research（自定义配置）。
    ///
    /// # Errors
    ///
    /// 当创建交互请求失败或服务端返回错误时返回错误。
    pub async fn start_with_config(
        &self,
        mut config: CreateInteractionConfig,
    ) -> Result<Interaction> {
        apply_deep_research_defaults(&mut config);
        Interactions::new(self.inner.clone()).create(config).await
    }

    /// 流式启动 Deep Research（自定义配置）。
    ///
    /// # Errors
    ///
    /// 当创建流式交互请求失败或服务端返回错误时返回错误。
    pub async fn stream_with_config(
        &self,
        mut config: CreateInteractionConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<InteractionEvent>> + Send>>> {
        apply_deep_research_defaults(&mut config);
        Interactions::new(self.inner.clone())
            .create_stream(config)
            .await
    }
}

fn apply_deep_research_defaults(config: &mut CreateInteractionConfig) {
    if config.background.is_none() {
        config.background = Some(true);
    }
    if config.store.is_none() {
        config.store = Some(true);
    }
    if config.thinking_summaries.is_none() {
        config.thinking_summaries = Some(InteractionThinkingSummaries::Auto);
    }
    if config.tools.is_none() {
        config.tools = Some(vec![Tool {
            google_search: Some(GoogleSearch::default()),
            ..Default::default()
        }]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_genai_types::interactions::InteractionThinkingSummaries;

    #[test]
    fn apply_defaults_sets_missing_fields() {
        let mut config = CreateInteractionConfig::new("model", InteractionInput::Text("hi".into()));
        apply_deep_research_defaults(&mut config);
        assert_eq!(config.background, Some(true));
        assert_eq!(config.store, Some(true));
        assert_eq!(
            config.thinking_summaries,
            Some(InteractionThinkingSummaries::Auto)
        );
        assert!(config.tools.is_some());
    }

    #[test]
    fn apply_defaults_respects_existing_fields() {
        let mut config = CreateInteractionConfig::new("model", InteractionInput::Text("hi".into()));
        config.background = Some(false);
        config.store = Some(false);
        config.thinking_summaries = Some(InteractionThinkingSummaries::NoneValue);
        config.tools = Some(Vec::new());
        apply_deep_research_defaults(&mut config);
        assert_eq!(config.background, Some(false));
        assert_eq!(config.store, Some(false));
        assert_eq!(
            config.thinking_summaries,
            Some(InteractionThinkingSummaries::NoneValue)
        );
        assert!(config.tools.as_ref().unwrap().is_empty());
    }
}
