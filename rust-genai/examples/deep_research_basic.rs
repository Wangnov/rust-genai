use futures_util::StreamExt;
use rust_genai::types::interactions::{
    AgentConfig, CreateInteractionConfig, DeepResearchAgentConfig, InteractionThinkingSummaries,
};
use rust_genai::Client;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let deep_research = client.deep_research();

    // Deep Research 需要使用专用 agent（docs 中为 deep-research-pro-preview-12-2025）。
    let mut config = CreateInteractionConfig::new_agent(
        "deep-research-pro-preview-12-2025",
        "调研 2025 年电池技术的三大趋势，并给出引用摘要。",
    );
    config.background = Some(true);
    config.agent_config = Some(AgentConfig::DeepResearch(DeepResearchAgentConfig {
        thinking_summaries: Some(InteractionThinkingSummaries::Auto),
        ..Default::default()
    }));

    let mut stream = deep_research.stream_with_config(config).await?;
    while let Some(event) = stream.next().await {
        let event = event?;
        println!("{:?}", event.event_type);
        if let Some(interaction) = &event.interaction {
            println!("{:?}", interaction.status);
        }
        io::stdout().flush().ok();
    }

    Ok(())
}
