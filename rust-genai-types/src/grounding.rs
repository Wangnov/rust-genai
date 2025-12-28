use crate::base64_serde;
use serde::{Deserialize, Serialize};

/// 引用日期。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTypeDate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
}

/// 引用信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publication_date: Option<GoogleTypeDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// 引用元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citations: Option<Vec<Citation>>,
}

/// Author attribution for Maps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceAnswerSourcesAuthorAttribution {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Review snippet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceAnswerSourcesReviewSnippet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_attribution: Option<PlaceAnswerSourcesAuthorAttribution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_content_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_maps_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relative_publish_time_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Sources used to generate the place answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceAnswerSources {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_content_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_snippets: Option<Vec<PlaceAnswerSourcesReviewSnippet>>,
}

/// Grounding chunk from Google Maps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapsChunk {
    pub uri: String,
    pub title: String,
    pub text: String,
    pub place_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place_answer_sources: Option<PlaceAnswerSources>,
}

/// Rag chunk page span.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagChunkPageSpan {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_page: Option<i32>,
}

/// RAG chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagChunk {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_span: Option<RagChunkPageSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Grounding chunk from retrieved context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievedContextChunk {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rag_chunk: Option<RagChunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

/// Grounding chunk from web.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebChunk {
    pub uri: String,
    pub title: String,
}

/// Grounding chunk union.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum GroundingChunk {
    Web {
        web: WebChunk,
    },
    RetrievedContext {
        retrieved_context: RetrievedContextChunk,
    },
    Maps {
        maps: MapsChunk,
    },
}

impl GroundingChunk {
    /// 获取来源 URI（如果存在）。
    #[must_use]
    pub fn uri(&self) -> Option<&str> {
        match self {
            Self::Web { web } => Some(web.uri.as_str()),
            Self::Maps { maps } => Some(maps.uri.as_str()),
            Self::RetrievedContext { retrieved_context } => retrieved_context.uri.as_deref(),
        }
    }

    /// 获取标题（如果存在）。
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        match self {
            Self::Web { web } => Some(web.title.as_str()),
            Self::Maps { maps } => Some(maps.title.as_str()),
            Self::RetrievedContext { retrieved_context } => retrieved_context.title.as_deref(),
        }
    }
}

/// Segment of the content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub part_index: i32,
    pub start_index: i32,
    pub end_index: i32,
    pub text: String,
}

/// Grounding support.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingSupport {
    #[serde(default)]
    pub grounding_chunk_indices: Vec<i32>,
    #[serde(default)]
    pub confidence_scores: Vec<f64>,
    pub segment: Segment,
}

/// Retrieval metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_search_dynamic_retrieval_score: Option<f32>,
}

/// Google search entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchEntryPoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_content: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "base64_serde::option"
    )]
    pub sdk_blob: Option<Vec<u8>>,
}

/// Source flagging URI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroundingMetadataSourceFlaggingUri {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_content_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// Grounding 元数据。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GroundingMetadata {
    #[serde(default)]
    pub grounding_chunks: Vec<GroundingChunk>,
    #[serde(default)]
    pub grounding_supports: Vec<GroundingSupport>,
    #[serde(default)]
    pub web_search_queries: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_entry_point: Option<SearchEntryPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_metadata: Option<RetrievalMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_maps_widget_context_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_queries: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_flagging_uris: Option<Vec<GroundingMetadataSourceFlaggingUri>>,
}

impl GroundingMetadata {
    /// 生成带内联引用的文本（使用 `grounding_supports` 的 `segment.end_index` 位置插入引用序号）。
    #[must_use]
    pub fn add_citations(&self, text: &str) -> String {
        if self.grounding_supports.is_empty() {
            return text.to_string();
        }

        let mut positions =
            std::collections::BTreeMap::<usize, std::collections::BTreeSet<i32>>::new();

        for support in &self.grounding_supports {
            let Ok(end_index) = usize::try_from(support.segment.end_index) else {
                continue;
            };
            let Some(byte_end) = char_index_to_byte(text, end_index) else {
                continue;
            };

            let entry = positions.entry(byte_end).or_default();
            for idx in &support.grounding_chunk_indices {
                if let Some(one_based) = idx.checked_add(1) {
                    if one_based > 0 {
                        entry.insert(one_based);
                    }
                }
            }
        }

        if positions.is_empty() {
            return text.to_string();
        }

        let mut output = text.to_string();
        for (pos, indices) in positions.into_iter().rev() {
            if pos > output.len() {
                continue;
            }
            let label = indices
                .into_iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(",");
            output.insert_str(pos, &format!(" [{label}]"));
        }

        output
    }

    /// 提取引用链接（按照 `grounding_chunks` 顺序去重）。
    #[must_use]
    pub fn citation_uris(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut uris = Vec::new();

        for chunk in &self.grounding_chunks {
            if let Some(uri) = chunk.uri() {
                if seen.insert(uri.to_string()) {
                    uris.push(uri.to_string());
                }
            }
        }

        uris
    }
}

fn char_index_to_byte(text: &str, index: usize) -> Option<usize> {
    if index == 0 {
        return Some(0);
    }
    let mut count = 0usize;
    for (byte_idx, _) in text.char_indices() {
        if count == index {
            return Some(byte_idx);
        }
        count += 1;
    }
    if count == index {
        Some(text.len())
    } else {
        None
    }
}

// 兼容旧名称（避免外部依赖受影响）。
pub type GroundingChunkMapsPlaceAnswerSources = PlaceAnswerSources;
pub type GroundingChunkMapsPlaceAnswerSourcesReviewSnippet = PlaceAnswerSourcesReviewSnippet;
pub type GroundingChunkMapsPlaceAnswerSourcesAuthorAttribution =
    PlaceAnswerSourcesAuthorAttribution;
pub type GroundingChunkMaps = MapsChunk;
pub type GroundingChunkRetrievedContext = RetrievedContextChunk;
pub type GroundingChunkWeb = WebChunk;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn grounding_chunk_uri_and_title() {
        let web = GroundingChunk::Web {
            web: WebChunk {
                uri: "https://example.com".to_string(),
                title: "Example".to_string(),
            },
        };
        let maps = GroundingChunk::Maps {
            maps: MapsChunk {
                uri: "https://maps.example.com".to_string(),
                title: "Map".to_string(),
                text: "info".to_string(),
                place_id: "place-1".to_string(),
                place_answer_sources: None,
            },
        };
        let retrieved = GroundingChunk::RetrievedContext {
            retrieved_context: RetrievedContextChunk {
                document_name: None,
                rag_chunk: None,
                text: None,
                title: Some("Doc".to_string()),
                uri: Some("https://doc.example.com".to_string()),
            },
        };

        assert_eq!(web.uri(), Some("https://example.com"));
        assert_eq!(maps.title(), Some("Map"));
        assert_eq!(retrieved.uri(), Some("https://doc.example.com"));
        assert_eq!(retrieved.title(), Some("Doc"));
    }

    #[test]
    fn search_entry_point_base64_roundtrip() {
        let entry = SearchEntryPoint {
            rendered_content: Some("rendered".to_string()),
            sdk_blob: Some(vec![1, 2, 3]),
        };
        let value = serde_json::to_value(&entry).unwrap();
        assert_eq!(
            value,
            json!({
                "renderedContent": "rendered",
                "sdkBlob": "AQID"
            })
        );

        let decoded: SearchEntryPoint = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.sdk_blob, Some(vec![1, 2, 3]));
    }

    #[test]
    fn grounding_metadata_add_citations_and_uris() {
        let metadata = GroundingMetadata {
            grounding_chunks: vec![
                GroundingChunk::Web {
                    web: WebChunk {
                        uri: "https://a.example".to_string(),
                        title: "A".to_string(),
                    },
                },
                GroundingChunk::Web {
                    web: WebChunk {
                        uri: "https://b.example".to_string(),
                        title: "B".to_string(),
                    },
                },
            ],
            grounding_supports: vec![GroundingSupport {
                grounding_chunk_indices: vec![0, 1],
                confidence_scores: vec![0.9],
                segment: Segment {
                    part_index: 0,
                    start_index: 0,
                    end_index: 2,
                    text: "hi".to_string(),
                },
            }],
            ..Default::default()
        };

        let cited = metadata.add_citations("hi!");
        assert_eq!(cited, "hi [1,2]!");
        let uris = metadata.citation_uris();
        assert_eq!(uris, vec!["https://a.example", "https://b.example"]);
    }
}
