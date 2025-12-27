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

    /// Kitoken-based estimator (SentencePiece compatible).
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

        /// Load a SentencePiece model from file.
        pub fn from_sentencepiece_file(
            path: impl AsRef<Path>,
        ) -> Result<Self, LocalTokenizerError> {
            let encoder = Kitoken::from_sentencepiece_file(path)?;
            Ok(Self::from_encoder(encoder))
        }

        /// Load a known Gemini model tokenizer by model name (downloads & caches).
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
                    .map(|value| value.to_string());

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
                            token_ids.push(id as i64);
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
        match std::str::from_utf8(bytes) {
            Ok(text) => text.replace('▁', " ").into_bytes(),
            Err(_) => bytes.to_vec(),
        }
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
            let _ = write!(output, "{:02x}", byte);
        }
        output
    }
}
