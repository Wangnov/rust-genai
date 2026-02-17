//! Deep Research convenience wrapper (Preview).

use std::pin::Pin;
use std::sync::Arc;

use futures_util::Stream;
use rust_genai_types::interactions::{
    AgentConfig, CreateInteractionConfig, DeepResearchAgentConfig, Interaction, InteractionInput,
    InteractionSseEvent, InteractionThinkingSummaries, Tool,
};

use crate::client::ClientInner;
use crate::error::Result;
use crate::interactions::Interactions;

const DEEP_RESEARCH_AGENT: &str = "deep-research-pro-preview-12-2025";

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
    pub async fn start(&self, input: impl Into<InteractionInput>) -> Result<Interaction> {
        let mut config = CreateInteractionConfig::new_agent(DEEP_RESEARCH_AGENT, input);
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
    ) -> Result<Pin<Box<dyn Stream<Item = Result<InteractionSseEvent>> + Send>>> {
        apply_deep_research_defaults(&mut config);
        Interactions::new(self.inner.clone())
            .create_stream(config)
            .await
    }
}

fn apply_deep_research_defaults(config: &mut CreateInteractionConfig) {
    // Ensure this wrapper always targets the Deep Research agent.
    if config.agent.is_none() {
        config.agent = Some(DEEP_RESEARCH_AGENT.to_string());
    }
    config.model = None;
    config.generation_config = None;

    if config.background.is_none() {
        config.background = Some(true);
    }
    if config.store.is_none() {
        config.store = Some(true);
    }

    // Agent-specific config defaults.
    match &mut config.agent_config {
        None => {
            config.agent_config = Some(AgentConfig::DeepResearch(DeepResearchAgentConfig {
                thinking_summaries: Some(InteractionThinkingSummaries::Auto),
                ..Default::default()
            }));
        }
        Some(AgentConfig::DeepResearch(cfg)) => {
            if cfg.thinking_summaries.is_none() {
                cfg.thinking_summaries = Some(InteractionThinkingSummaries::Auto);
            }
        }
        Some(AgentConfig::Dynamic(_)) => {}
    }

    if config.tools.is_none() {
        config.tools = Some(vec![Tool::GoogleSearch]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_defaults_sets_missing_fields() {
        let mut config = CreateInteractionConfig::new_agent(
            DEEP_RESEARCH_AGENT,
            InteractionInput::Text("hi".into()),
        );
        apply_deep_research_defaults(&mut config);
        assert_eq!(config.agent.as_deref(), Some(DEEP_RESEARCH_AGENT));
        assert!(config.model.is_none());
        assert_eq!(config.background, Some(true));
        assert_eq!(config.store, Some(true));
        assert!(config.tools.is_some());

        match config.agent_config {
            Some(AgentConfig::DeepResearch(cfg)) => {
                assert_eq!(
                    cfg.thinking_summaries,
                    Some(InteractionThinkingSummaries::Auto)
                );
            }
            _ => panic!("expected deep research agent config"),
        }
    }

    #[test]
    fn apply_defaults_respects_existing_fields() {
        let mut config = CreateInteractionConfig::new_agent(
            DEEP_RESEARCH_AGENT,
            InteractionInput::Text("hi".into()),
        );
        config.background = Some(false);
        config.store = Some(false);
        config.agent_config = Some(AgentConfig::DeepResearch(DeepResearchAgentConfig {
            thinking_summaries: Some(InteractionThinkingSummaries::NoneValue),
            ..Default::default()
        }));
        config.tools = Some(Vec::new());
        apply_deep_research_defaults(&mut config);
        assert_eq!(config.agent.as_deref(), Some(DEEP_RESEARCH_AGENT));
        assert!(config.model.is_none());
        assert_eq!(config.background, Some(false));
        assert_eq!(config.store, Some(false));
        match config.agent_config {
            Some(AgentConfig::DeepResearch(cfg)) => {
                assert_eq!(
                    cfg.thinking_summaries,
                    Some(InteractionThinkingSummaries::NoneValue)
                );
            }
            _ => panic!("expected deep research agent config"),
        }
        assert!(config.tools.as_ref().unwrap().is_empty());
    }
}
