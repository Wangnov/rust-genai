//! Local token estimation utilities (optional).

use rust_genai_types::config::GenerationConfig;
use rust_genai_types::content::{Content, FunctionCall, FunctionResponse, PartKind};
use rust_genai_types::models::CountTokensConfig;
use rust_genai_types::tool::{FunctionDeclaration, Schema, Tool};
use serde_json::Value;

/// Token estimator trait.
pub trait TokenEstimator {
    fn estimate_tokens(&self, contents: &[Content]) -> usize;
}

/// A very rough heuristic estimator (bytes / 4).
#[derive(Debug, Clone, Default)]
pub struct SimpleTokenEstimator;

impl TokenEstimator for SimpleTokenEstimator {
    fn estimate_tokens(&self, contents: &[Content]) -> usize {
        let mut bytes = 0usize;
        for content in contents {
            for part in &content.parts {
                match &part.kind {
                    PartKind::Text { text } => {
                        bytes += text.len();
                    }
                    PartKind::InlineData { inline_data } => {
                        bytes += inline_data.data.len();
                    }
                    PartKind::FileData { file_data } => {
                        bytes += file_data.file_uri.len();
                    }
                    PartKind::FunctionCall { function_call } => {
                        if let Some(name) = &function_call.name {
                            bytes += name.len();
                        }
                    }
                    PartKind::FunctionResponse { function_response } => {
                        if let Some(name) = &function_response.name {
                            bytes += name.len();
                        }
                    }
                    PartKind::ExecutableCode { executable_code } => {
                        bytes += executable_code.code.len();
                    }
                    PartKind::CodeExecutionResult {
                        code_execution_result,
                    } => {
                        if let Some(output) = &code_execution_result.output {
                            bytes += output.len();
                        }
                    }
                }
            }
        }
        // Rough heuristic: 1 token ~ 4 bytes.
        bytes.div_ceil(4)
    }
}

pub(crate) fn build_estimation_contents(
    contents: &[Content],
    config: &CountTokensConfig,
) -> Vec<Content> {
    let mut combined = Vec::with_capacity(contents.len() + 1);
    combined.extend_from_slice(contents);
    if let Some(system_instruction) = &config.system_instruction {
        combined.push(system_instruction.clone());
    }

    let mut accumulator = TextAccumulator::default();
    accumulator.add_function_texts_from_contents(&combined);
    if let Some(tools) = &config.tools {
        accumulator.add_tools(tools);
    }
    if let Some(generation_config) = &config.generation_config {
        accumulator.add_generation_config(generation_config);
    }
    combined.extend(accumulator.into_contents());
    combined
}

#[derive(Debug, Default)]
struct TextAccumulator {
    texts: Vec<String>,
}

impl TextAccumulator {
    fn add_function_texts_from_contents(&mut self, contents: &[Content]) {
        for content in contents {
            self.add_function_texts_from_content(content);
        }
    }

    fn add_function_texts_from_content(&mut self, content: &Content) {
        for part in &content.parts {
            match &part.kind {
                PartKind::FunctionCall { function_call } => {
                    self.add_function_call(function_call);
                }
                PartKind::FunctionResponse { function_response } => {
                    self.add_function_response(function_response);
                }
                _ => {}
            }
        }
    }

    fn add_function_call(&mut self, function_call: &FunctionCall) {
        if let Some(name) = &function_call.name {
            self.push_text(name);
        }
        if let Some(args) = &function_call.args {
            self.add_json(args);
        }
    }

    fn add_function_response(&mut self, function_response: &FunctionResponse) {
        if let Some(name) = &function_response.name {
            self.push_text(name);
        }
        if let Some(response) = &function_response.response {
            self.add_json(response);
        }
    }

    fn add_tools(&mut self, tools: &[Tool]) {
        for tool in tools {
            if let Some(functions) = &tool.function_declarations {
                for function in functions {
                    self.add_function_declaration(function);
                }
            }
        }
    }

    fn add_function_declaration(&mut self, declaration: &FunctionDeclaration) {
        self.push_text(&declaration.name);
        if let Some(description) = &declaration.description {
            self.push_text(description);
        }
        if let Some(parameters) = &declaration.parameters {
            self.add_schema(parameters);
        }
        if let Some(response) = &declaration.response {
            self.add_schema(response);
        }
        if let Some(parameters_json) = &declaration.parameters_json_schema {
            self.add_json(parameters_json);
        }
        if let Some(response_json) = &declaration.response_json_schema {
            self.add_json(response_json);
        }
    }

    fn add_generation_config(&mut self, generation_config: &GenerationConfig) {
        if let Some(response_schema) = &generation_config.response_schema {
            self.add_schema(response_schema);
        }
        if let Some(response_json_schema) = &generation_config.response_json_schema {
            self.add_json(response_json_schema);
        }
    }

    fn add_schema(&mut self, schema: &Schema) {
        if let Some(title) = &schema.title {
            self.push_text(title);
        }
        if let Some(format) = &schema.format {
            self.push_text(format);
        }
        if let Some(description) = &schema.description {
            self.push_text(description);
        }
        if let Some(enum_values) = &schema.enum_values {
            for value in enum_values {
                self.push_text(value);
            }
        }
        if let Some(required) = &schema.required {
            for value in required {
                self.push_text(value);
            }
        }
        if let Some(properties) = &schema.properties {
            for (key, value) in properties {
                self.push_text(key);
                self.add_schema(value);
            }
        }
        if let Some(items) = &schema.items {
            self.add_schema(items);
        }
        if let Some(any_of) = &schema.any_of {
            for schema in any_of {
                self.add_schema(schema);
            }
        }
        if let Some(example) = &schema.example {
            self.add_json(example);
        }
        if let Some(default) = &schema.default {
            self.add_json(default);
        }
    }

    fn add_json(&mut self, value: &Value) {
        match value {
            Value::String(value) => self.push_text(value),
            Value::Array(values) => {
                for item in values {
                    self.add_json(item);
                }
            }
            Value::Object(map) => {
                for (key, value) in map {
                    self.push_text(key);
                    self.add_json(value);
                }
            }
            _ => {}
        }
    }

    fn push_text(&mut self, value: &str) {
        if !value.is_empty() {
            self.texts.push(value.to_string());
        }
    }

    fn into_contents(self) -> Vec<Content> {
        self.texts.into_iter().map(Content::text).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_genai_types::config::GenerationConfig;
    use rust_genai_types::content::{FunctionCall, FunctionResponse, Part, Role};
    use rust_genai_types::models::CountTokensConfig;
    use rust_genai_types::tool::{FunctionDeclaration, Schema, Tool};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn simple_token_estimator_counts_various_parts() {
        let call = FunctionCall {
            id: Some("call-1".into()),
            name: Some("lookup".into()),
            args: Some(json!({"q": "rust"})),
            partial_args: None,
            will_continue: None,
        };
        let response = FunctionResponse {
            will_continue: None,
            scheduling: None,
            parts: None,
            id: Some("resp-1".into()),
            name: Some("lookup".into()),
            response: Some(json!({"ok": true})),
        };
        let content = Content::from_parts(
            vec![
                Part::text("hello"),
                Part::inline_data(vec![0, 1, 2, 3], "image/png"),
                Part::file_data("files/abc", "application/pdf"),
                Part::function_call(call),
                Part::function_response(response),
                Part::executable_code("print('hi')", rust_genai_types::enums::Language::Python),
                Part::code_execution_result(rust_genai_types::enums::Outcome::OutcomeOk, "ok"),
            ],
            Role::User,
        );

        let estimator = SimpleTokenEstimator;
        let tokens = estimator.estimate_tokens(&[content]);
        assert!(tokens > 0);
    }

    #[test]
    fn build_estimation_contents_includes_tools_and_config() {
        let declaration = FunctionDeclaration {
            name: "search".to_string(),
            description: Some("desc".to_string()),
            parameters: Some(
                Schema::object()
                    .property("q", Schema::string())
                    .required("q")
                    .build(),
            ),
            parameters_json_schema: Some(
                json!({"type": "object", "properties": {"q": {"type": "string"}}}),
            ),
            response: Some(Schema::string()),
            response_json_schema: Some(json!({"type": "string"})),
            behavior: None,
        };
        let tool = Tool {
            function_declarations: Some(vec![declaration]),
            ..Default::default()
        };
        let generation_config = GenerationConfig {
            response_schema: Some(Schema::object().property("r", Schema::string()).build()),
            response_json_schema: Some(
                json!({"type": "object", "properties": {"r": {"type": "string"}}}),
            ),
            ..Default::default()
        };

        let config = CountTokensConfig {
            system_instruction: Some(Content::text("sys")),
            tools: Some(vec![tool]),
            generation_config: Some(generation_config),
        };

        let contents = vec![Content::text("user")];
        let combined = build_estimation_contents(&contents, &config);
        // 原始内容 + 系统指令 + 追加文本内容
        assert!(combined.len() >= 2);
    }

    #[test]
    fn text_accumulator_collects_schema_and_json_fields() {
        let mut properties = HashMap::new();
        properties.insert("prop".to_string(), Box::new(Schema::string()));
        let schema = Schema {
            title: Some("Title".into()),
            format: Some("Fmt".into()),
            description: Some("Desc".into()),
            enum_values: Some(vec!["A".into(), "B".into()]),
            required: Some(vec!["req".into()]),
            properties: Some(properties),
            items: Some(Box::new(Schema::number())),
            any_of: Some(vec![Schema::boolean()]),
            example: Some(json!({"ex_key": "ex_val"})),
            default: Some(json!(["d"])),
            ..Default::default()
        };

        let mut accumulator = TextAccumulator::default();
        accumulator.add_schema(&schema);
        accumulator.add_json(&json!(["a", {"k": "v"}, 1]));
        let texts = accumulator.texts;

        assert!(texts.contains(&"Title".to_string()));
        assert!(texts.contains(&"Fmt".to_string()));
        assert!(texts.contains(&"Desc".to_string()));
        assert!(texts.contains(&"A".to_string()));
        assert!(texts.contains(&"B".to_string()));
        assert!(texts.contains(&"req".to_string()));
        assert!(texts.contains(&"prop".to_string()));
        assert!(texts.contains(&"ex_key".to_string()));
        assert!(texts.contains(&"ex_val".to_string()));
        assert!(texts.contains(&"k".to_string()));
        assert!(texts.contains(&"v".to_string()));
        assert!(texts.contains(&"a".to_string()));
    }

    #[test]
    fn text_accumulator_collects_function_parts() {
        let call = FunctionCall {
            id: None,
            name: None,
            args: Some(json!({"q": "rust"})),
            partial_args: None,
            will_continue: None,
        };
        let response = FunctionResponse {
            will_continue: None,
            scheduling: None,
            parts: None,
            id: None,
            name: None,
            response: Some(json!({"answer": "ok"})),
        };
        let content = Content::from_parts(
            vec![Part::function_call(call), Part::function_response(response)],
            Role::User,
        );

        let mut accumulator = TextAccumulator::default();
        accumulator.add_function_texts_from_content(&content);
        let texts = accumulator.texts;

        assert!(texts.contains(&"q".to_string()));
        assert!(texts.contains(&"rust".to_string()));
        assert!(texts.contains(&"answer".to_string()));
        assert!(texts.contains(&"ok".to_string()));
    }

    #[test]
    fn text_accumulator_collects_named_parts_and_declarations() {
        let call = FunctionCall {
            id: None,
            name: Some("lookup".into()),
            args: Some(json!({"k": "v"})),
            partial_args: None,
            will_continue: None,
        };
        let response = FunctionResponse {
            will_continue: None,
            scheduling: None,
            parts: None,
            id: None,
            name: Some("lookup_result".into()),
            response: Some(json!({"out": "done"})),
        };
        let content = Content::from_parts(
            vec![Part::function_call(call), Part::function_response(response)],
            Role::User,
        );

        let declaration = FunctionDeclaration {
            name: "search".to_string(),
            description: Some("desc".to_string()),
            parameters: Some(Schema::object().property("q", Schema::string()).build()),
            parameters_json_schema: Some(
                json!({"type": "object", "properties": {"q": {"type": "string"}}}),
            ),
            response: Some(Schema::string()),
            response_json_schema: Some(json!({"type": "string"})),
            behavior: None,
        };

        let generation_config = GenerationConfig {
            response_schema: Some(Schema::string()),
            response_json_schema: Some(json!({"type": "string"})),
            ..Default::default()
        };

        let mut accumulator = TextAccumulator::default();
        accumulator.add_function_texts_from_content(&content);
        accumulator.add_function_declaration(&declaration);
        accumulator.add_generation_config(&generation_config);
        let texts = accumulator.texts;

        assert!(texts.contains(&"lookup".to_string()));
        assert!(texts.contains(&"lookup_result".to_string()));
        assert!(texts.contains(&"k".to_string()));
        assert!(texts.contains(&"v".to_string()));
        assert!(texts.contains(&"out".to_string()));
        assert!(texts.contains(&"done".to_string()));
        assert!(texts.contains(&"search".to_string()));
        assert!(texts.contains(&"desc".to_string()));
        assert!(texts.contains(&"q".to_string()));
    }

    #[test]
    fn simple_token_estimator_counts_function_names() {
        let call = FunctionCall {
            id: None,
            name: Some("ping".into()),
            args: None,
            partial_args: None,
            will_continue: None,
        };
        let response = FunctionResponse {
            will_continue: None,
            scheduling: None,
            parts: None,
            id: None,
            name: Some("pong".into()),
            response: None,
        };
        let content = Content::from_parts(
            vec![Part::function_call(call), Part::function_response(response)],
            Role::User,
        );

        let estimator = SimpleTokenEstimator;
        let tokens = estimator.estimate_tokens(&[content]);
        assert_eq!(tokens, 2);
    }

    #[test]
    fn simple_token_estimator_empty_is_zero() {
        let estimator = SimpleTokenEstimator;
        let tokens = estimator.estimate_tokens(&[]);
        assert_eq!(tokens, 0);
    }
}

#[cfg(feature = "kitoken")]
pub mod kitoken {
    use super::TokenEstimator;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    use kitoken::convert::ConversionError;
    use kitoken::EncodeError;
    use kitoken::Kitoken;
    use rust_genai_types::content::{
        Content, FunctionCall, FunctionResponse, Part, PartKind, Role,
    };
    use rust_genai_types::models::{ComputeTokensResponse, TokensInfo};
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::fmt::Write;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    const CACHE_DIR: &str = "vertexai_tokenizer_model";

    struct TokenizerConfig {
        model_url: &'static str,
        model_hash: &'static str,
    }

    const GEMINI_MODELS_TO_TOKENIZER_NAMES: &[(&str, &str)] = &[
        ("gemini-1.0-pro", "gemma2"),
        ("gemini-1.5-pro", "gemma2"),
        ("gemini-1.5-flash", "gemma2"),
        ("gemini-2.5-pro", "gemma3"),
        ("gemini-2.5-flash", "gemma3"),
        ("gemini-2.5-flash-lite", "gemma3"),
        ("gemini-2.0-flash", "gemma3"),
        ("gemini-2.0-flash-lite", "gemma3"),
    ];

    const GEMINI_STABLE_MODELS_TO_TOKENIZER_NAMES: &[(&str, &str)] = &[
        ("gemini-1.0-pro-001", "gemma2"),
        ("gemini-1.0-pro-002", "gemma2"),
        ("gemini-1.5-pro-001", "gemma2"),
        ("gemini-1.5-pro-002", "gemma2"),
        ("gemini-1.5-flash-001", "gemma2"),
        ("gemini-1.5-flash-002", "gemma2"),
        ("gemini-2.5-pro-preview-06-05", "gemma3"),
        ("gemini-2.5-pro-preview-05-06", "gemma3"),
        ("gemini-2.5-pro-exp-03-25", "gemma3"),
        ("gemini-live-2.5-flash", "gemma3"),
        ("gemini-2.5-flash-native-audio-preview-12-2025", "gemma3"),
        ("gemini-2.5-flash-native-audio-preview-09-2025", "gemma3"),
        ("gemini-2.5-flash-preview-05-20", "gemma3"),
        ("gemini-2.5-flash-preview-04-17", "gemma3"),
        ("gemini-2.5-flash-lite-preview-06-17", "gemma3"),
        ("gemini-2.0-flash-001", "gemma3"),
        ("gemini-2.0-flash-lite-001", "gemma3"),
        ("gemini-3-pro-preview", "gemma3"),
    ];

    fn tokenizer_config(name: &str) -> Option<TokenizerConfig> {
        match name {
            "gemma2" => Some(TokenizerConfig {
                model_url: "https://raw.githubusercontent.com/google/gemma_pytorch/33b652c465537c6158f9a472ea5700e5e770ad3f/tokenizer/tokenizer.model",
                model_hash: "61a7b147390c64585d6c3543dd6fc636906c9af3865a5548f27f31aee1d4c8e2",
            }),
            "gemma3" => Some(TokenizerConfig {
                model_url: "https://raw.githubusercontent.com/google/gemma_pytorch/014acb7ac4563a5f77c76d7ff98f31b568c16508/tokenizer/gemma3_cleaned_262144_v2.spiece.model",
                model_hash: "1299c11d7cf632ef3b4e11937501358ada021bbdf7c47638d13c0ee982f2e79c",
            }),
            _ => None,
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum LocalTokenizerError {
        #[error("Model {model} is not supported. Supported models: {supported}")]
        UnsupportedModel { model: String, supported: String },
        #[error("Tokenizer {name} is not supported")]
        UnsupportedTokenizer { name: String },
        #[error("Failed to download tokenizer model from {url}: {source}")]
        Download {
            url: String,
            #[source]
            source: reqwest::Error,
        },
        #[error("Tokenizer model download returned non-success status {status} for {url}")]
        DownloadStatus { url: String, status: u16 },
        #[error("Tokenizer model hash mismatch. expected {expected}, got {actual}")]
        HashMismatch { expected: String, actual: String },
        #[error("IO error: {source}")]
        Io {
            #[from]
            source: std::io::Error,
        },
        #[error("Tokenizer encode error: {source}")]
        Encode {
            #[from]
            source: EncodeError,
        },
        #[error("Local tokenizer does not support non-text content: {kind}")]
        UnsupportedContent { kind: &'static str },
        #[error("Tokenizer token id {id} not found in vocabulary")]
        MissingToken { id: u32 },
        #[error("Tokenizer conversion error: {source}")]
        Conversion {
            #[from]
            source: ConversionError,
        },
    }

    /// Kitoken-based estimator (`SentencePiece` compatible).
    #[derive(Debug, Clone)]
    pub struct KitokenEstimator {
        encoder: Arc<Kitoken>,
        token_bytes: Arc<HashMap<u32, Vec<u8>>>,
    }

    impl KitokenEstimator {
        fn from_encoder(encoder: Kitoken) -> Self {
            let token_bytes = Arc::new(build_token_bytes_map(&encoder));
            Self {
                encoder: Arc::new(encoder),
                token_bytes,
            }
        }

        /// Load a `SentencePiece` model from file.
        ///
        /// # Errors
        /// 当模型加载失败或文件无效时返回错误。
        pub fn from_sentencepiece_file(
            path: impl AsRef<Path>,
        ) -> Result<Self, LocalTokenizerError> {
            let encoder = Kitoken::from_sentencepiece_file(path)?;
            Ok(Self::from_encoder(encoder))
        }

        /// Load a known Gemini model tokenizer by model name (downloads & caches).
        ///
        /// # Errors
        /// 当模型名未知、下载失败或解析失败时返回错误。
        pub async fn from_model_name(model_name: &str) -> Result<Self, LocalTokenizerError> {
            let tokenizer_name = get_tokenizer_name(model_name)?;
            let config = tokenizer_config(tokenizer_name).ok_or_else(|| {
                LocalTokenizerError::UnsupportedTokenizer {
                    name: tokenizer_name.to_string(),
                }
            })?;
            let model_bytes = load_model_bytes(config.model_url, config.model_hash).await?;
            let encoder = Kitoken::from_sentencepiece_slice(&model_bytes)?;
            Ok(Self::from_encoder(encoder))
        }

        /// Compute token ids and token bytes for text contents.
        ///
        /// # Errors
        /// 当内容不受支持或编码失败时返回错误。
        pub fn compute_tokens(
            &self,
            contents: &[Content],
        ) -> Result<ComputeTokensResponse, LocalTokenizerError> {
            let mut tokens_info: Vec<TokensInfo> = Vec::new();
            for content in contents {
                let role = content
                    .role
                    .map(|role| match role {
                        Role::User => "user",
                        Role::Model => "model",
                        Role::Function => "function",
                    })
                    .map(ToString::to_string);

                for part in &content.parts {
                    let texts = collect_part_texts(part)?;
                    if texts.is_empty() {
                        continue;
                    }
                    let mut token_ids = Vec::new();
                    let mut tokens = Vec::new();
                    for text in texts {
                        if text.is_empty() {
                            continue;
                        }
                        let ids = self.encoder.encode(&text, true)?;
                        for id in ids {
                            let bytes = self
                                .token_bytes
                                .get(&id)
                                .ok_or(LocalTokenizerError::MissingToken { id })?;
                            tokens.push(STANDARD.encode(bytes));
                            token_ids.push(i64::from(id));
                        }
                    }
                    if token_ids.is_empty() {
                        continue;
                    }
                    tokens_info.push(TokensInfo {
                        role: role.clone(),
                        token_ids: Some(token_ids),
                        tokens: Some(tokens),
                    });
                }
            }

            Ok(ComputeTokensResponse {
                tokens_info: Some(tokens_info),
            })
        }
    }

    impl TokenEstimator for KitokenEstimator {
        fn estimate_tokens(&self, contents: &[Content]) -> usize {
            let mut total = 0usize;
            for content in contents {
                for part in &content.parts {
                    if let Some(text) = part.text_value() {
                        if let Ok(tokens) = self.encoder.encode(text, true) {
                            total += tokens.len();
                        }
                    }
                }
            }
            total
        }
    }

    fn build_token_bytes_map(encoder: &Kitoken) -> HashMap<u32, Vec<u8>> {
        let definition = encoder.to_definition();
        let mut map = HashMap::new();
        for token in definition.model.vocab() {
            map.insert(token.id, normalize_token_bytes(&token.bytes));
        }
        for special in definition.specials {
            map.insert(special.id, normalize_token_bytes(&special.bytes));
        }
        map
    }

    fn normalize_token_bytes(bytes: &[u8]) -> Vec<u8> {
        std::str::from_utf8(bytes).map_or_else(
            |_| bytes.to_vec(),
            |text| text.replace('▁', " ").into_bytes(),
        )
    }

    fn collect_part_texts(part: &Part) -> Result<Vec<String>, LocalTokenizerError> {
        let mut texts = Vec::new();
        match &part.kind {
            PartKind::Text { text } => {
                if !text.is_empty() {
                    texts.push(text.clone());
                }
            }
            PartKind::FunctionCall { function_call } => {
                add_function_call_texts(function_call, &mut texts);
            }
            PartKind::FunctionResponse { function_response } => {
                add_function_response_texts(function_response, &mut texts);
            }
            PartKind::ExecutableCode { executable_code } => {
                if !executable_code.code.is_empty() {
                    texts.push(executable_code.code.clone());
                }
            }
            PartKind::CodeExecutionResult {
                code_execution_result,
            } => {
                if let Some(output) = &code_execution_result.output {
                    if !output.is_empty() {
                        texts.push(output.clone());
                    }
                }
            }
            PartKind::InlineData { .. } => {
                return Err(LocalTokenizerError::UnsupportedContent {
                    kind: "inline_data",
                });
            }
            PartKind::FileData { .. } => {
                return Err(LocalTokenizerError::UnsupportedContent { kind: "file_data" });
            }
        }
        Ok(texts)
    }

    fn add_function_call_texts(function_call: &FunctionCall, texts: &mut Vec<String>) {
        if let Some(name) = &function_call.name {
            if !name.is_empty() {
                texts.push(name.clone());
            }
        }
        if let Some(args) = &function_call.args {
            add_json_texts(args, texts);
        }
    }

    fn add_function_response_texts(function_response: &FunctionResponse, texts: &mut Vec<String>) {
        if let Some(name) = &function_response.name {
            if !name.is_empty() {
                texts.push(name.clone());
            }
        }
        if let Some(response) = &function_response.response {
            add_json_texts(response, texts);
        }
    }

    fn add_json_texts(value: &serde_json::Value, texts: &mut Vec<String>) {
        match value {
            serde_json::Value::String(value) => {
                if !value.is_empty() {
                    texts.push(value.clone());
                }
            }
            serde_json::Value::Array(values) => {
                for item in values {
                    add_json_texts(item, texts);
                }
            }
            serde_json::Value::Object(map) => {
                for (key, value) in map {
                    if !key.is_empty() {
                        texts.push(key.clone());
                    }
                    add_json_texts(value, texts);
                }
            }
            _ => {}
        }
    }

    fn get_tokenizer_name(model_name: &str) -> Result<&'static str, LocalTokenizerError> {
        for (name, tokenizer) in GEMINI_MODELS_TO_TOKENIZER_NAMES {
            if *name == model_name {
                return Ok(*tokenizer);
            }
        }
        for (name, tokenizer) in GEMINI_STABLE_MODELS_TO_TOKENIZER_NAMES {
            if *name == model_name {
                return Ok(*tokenizer);
            }
        }
        let mut supported: Vec<String> = GEMINI_MODELS_TO_TOKENIZER_NAMES
            .iter()
            .map(|(name, _)| (*name).to_string())
            .collect();
        supported.extend(
            GEMINI_STABLE_MODELS_TO_TOKENIZER_NAMES
                .iter()
                .map(|(name, _)| (*name).to_string()),
        );
        supported.sort();
        supported.dedup();
        Err(LocalTokenizerError::UnsupportedModel {
            model: model_name.to_string(),
            supported: supported.join(", "),
        })
    }

    async fn load_model_bytes(
        url: &str,
        expected_hash: &str,
    ) -> Result<Vec<u8>, LocalTokenizerError> {
        let cache_path = cache_path_for(url);
        if let Some(bytes) = read_cache(&cache_path, expected_hash)? {
            return Ok(bytes);
        }
        let bytes = download_model(url).await?;
        let actual_hash = sha256_hex(&bytes);
        if actual_hash != expected_hash {
            return Err(LocalTokenizerError::HashMismatch {
                expected: expected_hash.to_string(),
                actual: actual_hash,
            });
        }
        let _ = write_cache(&cache_path, &bytes);
        Ok(bytes)
    }

    fn cache_path_for(url: &str) -> PathBuf {
        let filename = sha256_hex(url.as_bytes());
        std::env::temp_dir().join(CACHE_DIR).join(filename)
    }

    fn read_cache(
        path: &Path,
        expected_hash: &str,
    ) -> Result<Option<Vec<u8>>, LocalTokenizerError> {
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path)?;
        if sha256_hex(&bytes) == expected_hash {
            return Ok(Some(bytes));
        }
        let _ = fs::remove_file(path);
        Ok(None)
    }

    fn write_cache(path: &Path, bytes: &[u8]) -> Result<(), LocalTokenizerError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = path.with_extension("tmp");
        fs::write(&tmp_path, bytes)?;
        fs::rename(tmp_path, path)?;
        Ok(())
    }

    async fn download_model(url: &str) -> Result<Vec<u8>, LocalTokenizerError> {
        let response = reqwest::get(url)
            .await
            .map_err(|source| LocalTokenizerError::Download {
                url: url.to_string(),
                source,
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(LocalTokenizerError::DownloadStatus {
                url: url.to_string(),
                status: status.as_u16(),
            });
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|source| LocalTokenizerError::Download {
                url: url.to_string(),
                source,
            })?;
        Ok(bytes.to_vec())
    }

    fn sha256_hex(data: &[u8]) -> String {
        let digest = Sha256::digest(data);
        let mut output = String::with_capacity(digest.len() * 2);
        for byte in digest {
            let _ = write!(output, "{byte:02x}");
        }
        output
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rust_genai_types::content::{Content, FunctionCall, FunctionResponse, Part, Role};
        use rust_genai_types::enums::{Language, Outcome};
        use serde_json::json;
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        fn build_test_encoder() -> Kitoken {
            let vocab = vec![
                kitoken::Token {
                    id: 0,
                    bytes: b"hi".to_vec(),
                },
                kitoken::Token {
                    id: 1,
                    bytes: b"lookup".to_vec(),
                },
                kitoken::Token {
                    id: 2,
                    bytes: b"q".to_vec(),
                },
                kitoken::Token {
                    id: 3,
                    bytes: b"rust".to_vec(),
                },
                kitoken::Token {
                    id: 4,
                    bytes: b"resp".to_vec(),
                },
                kitoken::Token {
                    id: 5,
                    bytes: b"ok".to_vec(),
                },
                kitoken::Token {
                    id: 6,
                    bytes: b"code".to_vec(),
                },
                kitoken::Token {
                    id: 7,
                    bytes: b"out".to_vec(),
                },
                kitoken::Token {
                    id: 8,
                    bytes: "\u{2581}".as_bytes().to_vec(),
                },
            ];
            let specials = vec![kitoken::SpecialToken {
                id: 99,
                bytes: b"[UNK]".to_vec(),
                kind: kitoken::SpecialTokenKind::Unknown,
                ident: None,
                score: 0.0,
                extract: false,
            }];
            let model = kitoken::Model::WordPiece {
                vocab,
                max_word_chars: 0,
            };
            let config = kitoken::Configuration::default();
            let meta = kitoken::Metadata::default();
            Kitoken::new(model, specials, config, meta).unwrap()
        }

        fn unique_cache_key(tag: &str) -> String {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            format!("test://{tag}-{nanos}")
        }

        #[test]
        fn get_tokenizer_name_known_and_unknown() {
            assert_eq!(get_tokenizer_name("gemini-1.5-pro").unwrap(), "gemma2");
            let err = get_tokenizer_name("unknown-model").unwrap_err();
            match err {
                LocalTokenizerError::UnsupportedModel { supported, .. } => {
                    assert!(supported.contains("gemini-1.0-pro"));
                }
                _ => panic!("expected UnsupportedModel error"),
            }
        }

        #[test]
        fn normalize_token_bytes_replaces_separator_and_handles_invalid_utf8() {
            let replaced = normalize_token_bytes("\u{2581}hi".as_bytes());
            assert_eq!(replaced, b" hi".to_vec());

            let invalid = normalize_token_bytes(&[0xff, 0xfe]);
            assert_eq!(invalid, vec![0xff, 0xfe]);
        }

        #[test]
        fn cache_roundtrip_and_mismatch_evicts() {
            let key = unique_cache_key("cache-roundtrip");
            let path = cache_path_for(&key);
            let _ = fs::remove_file(&path);

            let bytes = b"cached".to_vec();
            write_cache(&path, &bytes).unwrap();
            let hash = sha256_hex(&bytes);
            let cached = read_cache(&path, &hash).unwrap().unwrap();
            assert_eq!(cached, bytes);

            let wrong_hash = sha256_hex(b"other");
            let result = read_cache(&path, &wrong_hash).unwrap();
            assert!(result.is_none());
            assert!(!path.exists());
        }

        #[tokio::test]
        async fn load_model_bytes_uses_cache() {
            let key = unique_cache_key("load-cache");
            let path = cache_path_for(&key);
            let _ = fs::remove_file(&path);

            let bytes = b"model-bytes".to_vec();
            write_cache(&path, &bytes).unwrap();
            let hash = sha256_hex(&bytes);

            let loaded = load_model_bytes(&key, &hash).await.unwrap();
            assert_eq!(loaded, bytes);
        }

        #[test]
        fn collect_part_texts_rejects_binary_parts() {
            let inline = Part::inline_data(vec![1, 2, 3], "image/png");
            let err = collect_part_texts(&inline).unwrap_err();
            assert!(matches!(
                err,
                LocalTokenizerError::UnsupportedContent {
                    kind: "inline_data"
                }
            ));

            let file = Part::file_data("files/1", "application/pdf");
            let err = collect_part_texts(&file).unwrap_err();
            assert!(matches!(
                err,
                LocalTokenizerError::UnsupportedContent { kind: "file_data" }
            ));
        }

        #[test]
        fn kitoken_estimator_compute_tokens_and_map_normalization() {
            let encoder = build_test_encoder();
            let estimator = KitokenEstimator::from_encoder(encoder);

            let call = FunctionCall {
                id: None,
                name: Some("lookup".into()),
                args: Some(json!({"q": "rust"})),
                partial_args: None,
                will_continue: None,
            };
            let response = FunctionResponse {
                will_continue: None,
                scheduling: None,
                parts: None,
                id: None,
                name: Some("resp".into()),
                response: Some(json!({"ok": "ok"})),
            };
            let content = Content::from_parts(
                vec![
                    Part::text("hi"),
                    Part::function_call(call),
                    Part::function_response(response),
                    Part::executable_code("code", Language::Python),
                    Part::code_execution_result(Outcome::OutcomeOk, "out"),
                ],
                Role::User,
            );

            let result = estimator.compute_tokens(&[content]).unwrap();
            assert!(!result.tokens_info.as_ref().unwrap().is_empty());

            let estimated = estimator.estimate_tokens(&[Content::text("hi")]);
            assert!(estimated > 0);

            let normalized = estimator.token_bytes.get(&8).unwrap();
            assert_eq!(normalized.as_slice(), b" ");
        }
    }
}
