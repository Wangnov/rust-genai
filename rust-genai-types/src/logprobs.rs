use serde::{Deserialize, Serialize};

/// Logprobs 候选项。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogprobCandidate {
    pub token: String,
    pub token_id: i32,
    pub log_probability: f64,
}

/// Top candidates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopCandidates {
    #[serde(default)]
    pub candidates: Vec<LogprobCandidate>,
}

/// Logprobs 结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogprobsResult {
    #[serde(default)]
    pub top_candidates: Vec<TopCandidates>,
    #[serde(default)]
    pub chosen_candidates: Vec<LogprobCandidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_probability_sum: Option<f64>,
}

// 兼容旧名称（避免外部依赖受影响）。
pub type LogprobsResultCandidate = LogprobCandidate;
pub type LogprobsResultTopCandidates = TopCandidates;
