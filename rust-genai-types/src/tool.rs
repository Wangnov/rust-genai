use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::enums::{
    ApiSpec, AuthType, Behavior, DynamicRetrievalConfigMode, Environment, FunctionCallingMode,
    HttpElementLocation, PhishBlockThreshold, Type,
};

/// 工具定义。每个 Tool 通常仅设置一种工具字段。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval: Option<Retrieval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computer_use: Option<ComputerUse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_search: Option<FileSearch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_execution: Option<CodeExecution>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enterprise_web_search: Option<EnterpriseWebSearch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<FunctionDeclaration>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_maps: Option<GoogleMaps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_search: Option<GoogleSearch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_search_retrieval: Option<GoogleSearchRetrieval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_context: Option<UrlContext>,
}

/// 函数声明。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionDeclaration {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters_json_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_json_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<Behavior>,
}

/// Google Search 工具。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking_confidence: Option<PhishBlockThreshold>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range_filter: Option<Interval>,
}

/// Enterprise Web Search 工具（Vertex AI Search）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseWebSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking_confidence: Option<PhishBlockThreshold>,
}

/// Code execution 工具。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeExecution {}

/// URL Context 工具。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UrlContext {}

/// Computer Use 工具。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ComputerUse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_predefined_functions: Option<Vec<String>>,
}

/// Google Maps 工具。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GoogleMaps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_config: Option<AuthConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_widget: Option<bool>,
}

/// Google Search Retrieval 工具。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSearchRetrieval {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_retrieval_config: Option<DynamicRetrievalConfig>,
}

/// File Search 工具。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_search_store_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_filter: Option<String>,
}

/// 时间区间。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Interval {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
}

/// API key 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_string: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_element_location: Option<HttpElementLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Deprecated: API auth key config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiAuthApiKeyConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_secret_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_string: Option<String>,
}

/// Deprecated: API auth config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiAuth {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_config: Option<ApiAuthApiKeyConfig>,
}

/// Google Service Account 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfigGoogleServiceAccountConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account: Option<String>,
}

/// HTTP Basic 认证配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfigHttpBasicAuthConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_secret: Option<String>,
}

/// OAuth 认证配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfigOauthConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account: Option<String>,
}

/// OIDC 认证配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfigOidcConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account: Option<String>,
}

/// Auth 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_config: Option<ApiKeyConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<AuthType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_service_account_config: Option<AuthConfigGoogleServiceAccountConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_basic_auth_config: Option<AuthConfigHttpBasicAuthConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_config: Option<AuthConfigOauthConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oidc_config: Option<AuthConfigOidcConfig>,
}

/// ElasticSearch 参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExternalApiElasticSearchParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_hits: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_template: Option<String>,
}

/// Simple Search 参数。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExternalApiSimpleSearchParams {}

/// External API 检索配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExternalApi {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_auth: Option<ApiAuth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_spec: Option<ApiSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_config: Option<AuthConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elastic_search_params: Option<ExternalApiElasticSearchParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simple_search_params: Option<ExternalApiSimpleSearchParams>,
}

/// Vertex AI Search 数据源配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VertexAiSearchDataStoreSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_store: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

/// Vertex AI Search 检索配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VertexAiSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_store_specs: Option<Vec<VertexAiSearchDataStoreSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datastore: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<i32>,
}

/// Vertex RAG 资源配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VertexRagStoreRagResource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rag_corpus: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rag_file_ids: Option<Vec<String>>,
}

/// RAG 过滤配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalConfigFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_distance_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_similarity_threshold: Option<f64>,
}

/// RAG Hybrid Search 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalConfigHybridSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f32>,
}

/// RAG Ranker 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalConfigRankingLlmRanker {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
}

/// RAG Rank Service 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalConfigRankingRankService {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
}

/// RAG Ranking 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalConfigRanking {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_ranker: Option<RagRetrievalConfigRankingLlmRanker>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank_service: Option<RagRetrievalConfigRankingRankService>,
}

/// RAG Retrieval 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RagRetrievalConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<RagRetrievalConfigFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_search: Option<RagRetrievalConfigHybridSearch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking: Option<RagRetrievalConfigRanking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
}

/// Vertex RAG Store 配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VertexRagStore {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rag_corpora: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rag_resources: Option<Vec<VertexRagStoreRagResource>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rag_retrieval_config: Option<RagRetrievalConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_context: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_distance_threshold: Option<f64>,
}

/// Retrieval 工具配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Retrieval {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_attribution: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_api: Option<ExternalApi>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_ai_search: Option<VertexAiSearch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertex_rag_store: Option<VertexRagStore>,
}

/// 动态检索配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DynamicRetrievalConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamic_threshold: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<DynamicRetrievalConfigMode>,
}

/// Tool config（共享配置）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_calling_config: Option<FunctionCallingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_config: Option<RetrievalConfig>,
}

/// Function calling config。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FunctionCallingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_function_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<FunctionCallingMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_function_call_arguments: Option<bool>,
}

/// 经纬度位置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LatLng {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
}

/// Retrieval config。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lat_lng: Option<LatLng>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
}

/// OpenAPI Schema（精简实现）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<Schema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Schema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Box<Schema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_ordering: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub ty: Option<Type>,
}

#[cfg(test)]
mod schema_builder_tests {
    use super::*;

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
            google_maps: Some(GoogleMaps {
                enable_widget: Some(true),
                ..GoogleMaps::default()
            }),
            ..Tool::default()
        };

        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["googleMaps"]["enableWidget"].as_bool(), Some(true));
    }
}

impl Schema {
    /// 创建对象 Schema builder。
    pub fn object() -> SchemaBuilder {
        SchemaBuilder::new(Type::Object)
    }

    /// 创建数组 Schema builder。
    pub fn array() -> SchemaBuilder {
        SchemaBuilder::new(Type::Array)
    }

    /// 创建字符串 Schema。
    pub fn string() -> Self {
        Schema {
            ty: Some(Type::String),
            ..Default::default()
        }
    }

    /// 创建整数 Schema。
    pub fn integer() -> Self {
        Schema {
            ty: Some(Type::Integer),
            ..Default::default()
        }
    }

    /// 创建数字 Schema。
    pub fn number() -> Self {
        Schema {
            ty: Some(Type::Number),
            ..Default::default()
        }
    }

    /// 创建布尔 Schema。
    pub fn boolean() -> Self {
        Schema {
            ty: Some(Type::Boolean),
            ..Default::default()
        }
    }
}

pub struct SchemaBuilder {
    schema: Schema,
}

impl SchemaBuilder {
    /// 创建 Schema builder。
    pub fn new(ty: Type) -> Self {
        Self {
            schema: Schema {
                ty: Some(ty),
                ..Default::default()
            },
        }
    }

    /// 设置描述。
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.schema.description = Some(description.into());
        self
    }

    /// 添加字段。
    pub fn property(mut self, name: impl Into<String>, schema: Schema) -> Self {
        let properties = self.schema.properties.get_or_insert_with(HashMap::new);
        properties.insert(name.into(), Box::new(schema));
        self
    }

    /// 标记必填字段。
    pub fn required(mut self, name: impl Into<String>) -> Self {
        let required = self.schema.required.get_or_insert_with(Vec::new);
        required.push(name.into());
        self
    }

    /// 设置数组元素 Schema。
    pub fn items(mut self, schema: Schema) -> Self {
        self.schema.items = Some(Box::new(schema));
        self
    }

    /// 设置枚举值。
    pub fn enum_values(mut self, values: Vec<String>) -> Self {
        self.schema.enum_values = Some(values);
        self
    }

    /// 构建 Schema。
    pub fn build(self) -> Schema {
        self.schema
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_builder_object() {
        let schema = Schema::object()
            .property("name", Schema::string())
            .required("name")
            .build();
        assert_eq!(schema.ty, Some(Type::Object));
        assert!(schema.properties.unwrap().contains_key("name"));
    }
}
