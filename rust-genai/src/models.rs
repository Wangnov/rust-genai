//! Models API surface.

use std::collections::{HashMap, VecDeque};
use std::hash::BuildHasher;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::{Stream, StreamExt};
use rust_genai_types::content::{Content, FunctionCall, Part, PartKind, Role};
use rust_genai_types::converters;
use rust_genai_types::models::{
    ComputeTokensConfig, ComputeTokensRequest, ComputeTokensResponse, CountTokensConfig,
    CountTokensRequest, CountTokensResponse, DeleteModelConfig, DeleteModelResponse,
    EditImageConfig, EditImageResponse, EmbedContentConfig, EmbedContentResponse,
    GenerateContentConfig, GenerateContentRequest, GenerateImagesConfig, GenerateImagesResponse,
    GenerateVideosConfig, GenerateVideosOperation, GenerateVideosSource, Image, ListModelsConfig,
    ListModelsResponse, Model, RecontextImageConfig, RecontextImageResponse, RecontextImageSource,
    ReferenceImage, SegmentImageConfig, SegmentImageResponse, SegmentImageSource,
    UpdateModelConfig,
};
use rust_genai_types::response::{GenerateContentResponse, GenerateContentResponseUsageMetadata};
use serde::de::DeserializeOwned;

use crate::afc::{
    call_callable_tools, max_remote_calls, resolve_callable_tools, should_append_history,
    should_disable_afc, validate_afc_config, validate_afc_tools, CallableTool,
};
use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};
use crate::http_response::{
    sdk_http_response_from_headers, sdk_http_response_from_headers_and_body,
};
use crate::model_capabilities::{
    validate_code_execution_image_inputs, validate_function_response_media,
};
use crate::sse::{parse_sse_stream, parse_sse_stream_with_done_signal};
use crate::thinking::{validate_temperature, ThoughtSignatureValidator};
use crate::tokenizer::TokenEstimator;
use serde_json::Value;

mod builders;
mod http;
mod media;
pub(crate) mod parsers;

use builders::{
    build_edit_image_body, build_embed_body_gemini, build_embed_body_vertex,
    build_function_call_content, build_generate_images_body, build_generate_videos_body,
    build_recontext_image_body, build_segment_image_body, build_upscale_image_body,
};
use http::{
    apply_http_options, build_model_get_url, build_model_get_url_with_options,
    build_model_method_url, build_models_list_url, merge_extra_body,
};
use parsers::{
    convert_vertex_embed_response, parse_edit_image_response, parse_generate_images_response,
    parse_generate_videos_operation, parse_recontext_image_response, parse_segment_image_response,
    parse_upscale_image_response,
};

#[derive(Clone)]
pub struct Models {
    pub(crate) inner: Arc<ClientInner>,
}

#[derive(Debug, Clone)]
pub enum GenerateContentStreamEvent {
    /// Text delta extracted from a streaming chunk.
    Text(String),
    /// Function call surfaced by a streaming chunk.
    FunctionCall(FunctionCall),
    /// Usage metadata emitted by a streaming chunk.
    Usage(GenerateContentResponseUsageMetadata),
    /// Raw response chunk.
    Response(GenerateContentResponse),
    /// Aggregated final response assembled from the full stream.
    Done(GenerateContentResponse),
}

pub struct GenerateContentEventStream {
    inner: Pin<Box<dyn Stream<Item = Result<GenerateContentResponse>> + Send>>,
    pending: VecDeque<GenerateContentStreamEvent>,
    aggregate_response: Option<GenerateContentResponse>,
    saw_done: Arc<AtomicBool>,
    finished: bool,
}

impl GenerateContentEventStream {
    fn new(
        inner: Pin<Box<dyn Stream<Item = Result<GenerateContentResponse>> + Send>>,
        saw_done: Arc<AtomicBool>,
    ) -> Self {
        Self {
            inner,
            pending: VecDeque::new(),
            aggregate_response: None,
            saw_done,
            finished: false,
        }
    }

    pub async fn next_event(&mut self) -> Result<Option<GenerateContentStreamEvent>> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }

            if self.finished {
                return Ok(None);
            }

            match self.inner.next().await {
                Some(Ok(response)) => {
                    merge_stream_response(&mut self.aggregate_response, &response);
                    enqueue_stream_events(&mut self.pending, response);
                }
                Some(Err(err)) => {
                    self.finished = true;
                    return Err(err);
                }
                None => {
                    self.finished = true;
                    if self.saw_done.load(Ordering::Relaxed) {
                        if let Some(response) = self.aggregate_response.take() {
                            return Ok(Some(GenerateContentStreamEvent::Done(response)));
                        }
                    }
                    self.aggregate_response.take();
                    return Ok(None);
                }
            }
        }
    }
}

fn enqueue_stream_events(
    pending: &mut VecDeque<GenerateContentStreamEvent>,
    response: GenerateContentResponse,
) {
    for candidate in &response.candidates {
        if let Some(content) = &candidate.content {
            for part in &content.parts {
                if let Some(text) = part.text_value() {
                    pending.push_back(GenerateContentStreamEvent::Text(text.to_string()));
                }
                if let Some(call) = part.function_call_ref() {
                    pending.push_back(GenerateContentStreamEvent::FunctionCall(call.clone()));
                }
            }
        }
    }

    if let Some(usage) = response.usage_metadata.clone() {
        pending.push_back(GenerateContentStreamEvent::Usage(usage));
    }

    pending.push_back(GenerateContentStreamEvent::Response(response));
}

fn merge_stream_response(
    aggregate: &mut Option<GenerateContentResponse>,
    response: &GenerateContentResponse,
) {
    let aggregate = aggregate.get_or_insert_with(|| GenerateContentResponse {
        sdk_http_response: response.sdk_http_response.clone(),
        candidates: Vec::new(),
        create_time: response.create_time.clone(),
        automatic_function_calling_history: response.automatic_function_calling_history.clone(),
        prompt_feedback: response.prompt_feedback.clone(),
        usage_metadata: response.usage_metadata.clone(),
        model_version: response.model_version.clone(),
        response_id: response.response_id.clone(),
    });

    if response.sdk_http_response.is_some() {
        aggregate.sdk_http_response = response.sdk_http_response.clone();
    }
    if response.create_time.is_some() {
        aggregate.create_time = response.create_time.clone();
    }
    if response.automatic_function_calling_history.is_some() {
        aggregate.automatic_function_calling_history =
            response.automatic_function_calling_history.clone();
    }
    if response.prompt_feedback.is_some() {
        aggregate.prompt_feedback = response.prompt_feedback.clone();
    }
    if response.usage_metadata.is_some() {
        aggregate.usage_metadata = response.usage_metadata.clone();
    }
    if response.model_version.is_some() {
        aggregate.model_version = response.model_version.clone();
    }
    if response.response_id.is_some() {
        aggregate.response_id = response.response_id.clone();
    }

    for (position, candidate) in response.candidates.iter().enumerate() {
        let existing_position = if let Some(index) = candidate.index {
            aggregate
                .candidates
                .iter()
                .position(|item| item.index == Some(index))
                .or_else(|| {
                    single_candidate_stream_position(
                        &aggregate.candidates,
                        response.candidates.len(),
                        position,
                    )
                })
        } else if response.candidates.len() == 1 && aggregate.candidates.len() == 1 {
            Some(position)
        } else {
            None
        };

        if let Some(existing_position) = existing_position {
            merge_candidate(&mut aggregate.candidates[existing_position], candidate);
        } else {
            aggregate.candidates.push(candidate.clone());
        }
    }
}

fn single_candidate_stream_position(
    aggregate_candidates: &[rust_genai_types::response::Candidate],
    response_candidate_count: usize,
    position: usize,
) -> Option<usize> {
    if response_candidate_count == 1
        && aggregate_candidates.len() == 1
        && aggregate_candidates[0].index.is_none()
    {
        Some(position)
    } else {
        None
    }
}

fn merge_candidate(
    existing: &mut rust_genai_types::response::Candidate,
    next: &rust_genai_types::response::Candidate,
) {
    if let Some(content) = &next.content {
        match &mut existing.content {
            Some(existing_content) => {
                if existing_content.role.is_none() {
                    existing_content.role = content.role;
                }
                merge_content_parts(&mut existing_content.parts, &content.parts);
            }
            None => existing.content = Some(content.clone()),
        }
    }

    if next.citation_metadata.is_some() {
        existing.citation_metadata = next.citation_metadata.clone();
    }
    if next.finish_message.is_some() {
        existing.finish_message = next.finish_message.clone();
    }
    if next.token_count.is_some() {
        existing.token_count = next.token_count;
    }
    if next.finish_reason.is_some() {
        existing.finish_reason = next.finish_reason;
    }
    if next.avg_logprobs.is_some() {
        existing.avg_logprobs = next.avg_logprobs;
    }
    if next.grounding_metadata.is_some() {
        existing.grounding_metadata = next.grounding_metadata.clone();
    }
    if next.index.is_some() {
        existing.index = next.index;
    }
    if next.logprobs_result.is_some() {
        existing.logprobs_result = next.logprobs_result.clone();
    }
    if !next.safety_ratings.is_empty() {
        existing.safety_ratings = next.safety_ratings.clone();
    }
    if next.url_context_metadata.is_some() {
        existing.url_context_metadata = next.url_context_metadata.clone();
    }
}

fn merge_content_parts(existing_parts: &mut Vec<Part>, next_parts: &[Part]) {
    for (position, part) in next_parts.iter().enumerate() {
        if existing_parts
            .get_mut(position)
            .is_some_and(|existing_part| merge_stream_part(Some(existing_part), part))
        {
            continue;
        }
        existing_parts.push(part.clone());
    }
}

fn merge_stream_part(last_part: Option<&mut Part>, next_part: &Part) -> bool {
    let Some(last_part) = last_part else {
        return false;
    };
    if !parts_share_merge_context(last_part, next_part) {
        return false;
    }

    match (&mut last_part.kind, &next_part.kind) {
        (PartKind::Text { text: existing }, PartKind::Text { text }) => {
            existing.push_str(text.as_str());
            true
        }
        (
            PartKind::FunctionCall {
                function_call: existing_call,
            },
            PartKind::FunctionCall {
                function_call: next_call,
            },
        ) if function_calls_share_target(existing_call, next_call) => {
            merge_function_call(existing_call, next_call);
            true
        }
        _ => false,
    }
}

fn parts_share_merge_context(last_part: &Part, next_part: &Part) -> bool {
    last_part.thought == next_part.thought
        && last_part.thought_signature == next_part.thought_signature
        && media_resolution_matches(&last_part.media_resolution, &next_part.media_resolution)
        && video_metadata_matches(&last_part.video_metadata, &next_part.video_metadata)
}

fn media_resolution_matches(
    left: &Option<rust_genai_types::content::PartMediaResolution>,
    right: &Option<rust_genai_types::content::PartMediaResolution>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.level == right.level && left.num_tokens == right.num_tokens
        }
        _ => false,
    }
}

fn video_metadata_matches(
    left: &Option<rust_genai_types::content::VideoMetadata>,
    right: &Option<rust_genai_types::content::VideoMetadata>,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.start_offset == right.start_offset
                && left.end_offset == right.end_offset
                && left.fps == right.fps
        }
        _ => false,
    }
}

fn function_calls_share_target(existing: &FunctionCall, next: &FunctionCall) -> bool {
    if let (Some(existing_id), Some(next_id)) = (&existing.id, &next.id) {
        if existing_id != next_id {
            return false;
        }
    }
    if let (Some(existing_name), Some(next_name)) = (&existing.name, &next.name) {
        if existing_name != next_name {
            return false;
        }
    }

    existing.id.is_some() || next.id.is_some() || existing.name.is_some() || next.name.is_some()
}

fn merge_function_call(existing: &mut FunctionCall, next: &FunctionCall) {
    if next.id.is_some() {
        existing.id = next.id.clone();
    }
    if next.name.is_some() {
        existing.name = next.name.clone();
    }
    if next.args.is_some() {
        existing.args = next.args.clone();
        if next.partial_args.is_none() {
            existing.partial_args = None;
        }
    }
    if let Some(partial_args) = &next.partial_args {
        existing
            .partial_args
            .get_or_insert_with(Vec::new)
            .extend(partial_args.clone());
    }
    if next.will_continue.is_some() {
        existing.will_continue = next.will_continue;
    }
}

fn prepare_json_generation_config(
    mut config: GenerateContentConfig,
    schema: Option<Value>,
) -> Result<GenerateContentConfig> {
    let generation_config = config
        .generation_config
        .get_or_insert_with(Default::default);
    match generation_config.response_mime_type.as_deref() {
        Some("application/json") => {}
        Some(other) => {
            return Err(Error::InvalidConfig {
                message: format!(
                    "generate_json_with_config requires response_mime_type = application/json, got {other}"
                ),
            });
        }
        None => {
            generation_config.response_mime_type = Some("application/json".into());
        }
    }

    if let Some(schema) = schema {
        if generation_config.response_schema.is_some()
            || generation_config.response_json_schema.is_some()
        {
            return Err(Error::InvalidConfig {
                message:
                    "generate_json_with_schema_with_config requires an empty response schema configuration"
                        .into(),
            });
        }
        generation_config.response_json_schema = Some(schema);
    }

    Ok(config)
}

struct CallableStreamContext<S> {
    models: Models,
    model: String,
    contents: Vec<Content>,
    request_config: GenerateContentConfig,
    callable_tools: Vec<Box<dyn CallableTool>>,
    function_map: HashMap<String, usize, S>,
    max_calls: usize,
    append_history: bool,
}

fn build_synthetic_afc_response(
    response_content: Content,
    history: &[Content],
) -> GenerateContentResponse {
    let mut response = GenerateContentResponse {
        sdk_http_response: None,
        candidates: vec![rust_genai_types::response::Candidate {
            content: Some(response_content),
            citation_metadata: None,
            finish_message: None,
            token_count: None,
            finish_reason: None,
            avg_logprobs: None,
            grounding_metadata: None,
            index: None,
            logprobs_result: None,
            safety_ratings: Vec::new(),
            url_context_metadata: None,
        }],
        create_time: None,
        automatic_function_calling_history: None,
        prompt_feedback: None,
        usage_metadata: None,
        model_version: None,
        response_id: None,
    };

    if !history.is_empty() {
        response.automatic_function_calling_history = Some(history.to_vec());
    }

    response
}

async fn forward_stream_items(
    mut stream: Pin<Box<dyn Stream<Item = Result<GenerateContentResponse>> + Send>>,
    tx: &tokio::sync::mpsc::Sender<Result<GenerateContentResponse>>,
) -> Option<(Vec<FunctionCall>, Vec<Content>)> {
    let mut function_calls: Vec<FunctionCall> = Vec::new();
    let mut response_contents: Vec<Content> = Vec::new();

    while let Some(item) = stream.next().await {
        if let Ok(response) = &item {
            if let Some(content) = response.candidates.first().and_then(|c| c.content.clone()) {
                for part in &content.parts {
                    if let Some(call) = part.function_call_ref() {
                        function_calls.push(call.clone());
                    }
                }
                response_contents.push(content);
            }
        }

        if tx.send(item).await.is_err() {
            return None;
        }
    }

    Some((function_calls, response_contents))
}

fn spawn_callable_stream_loop<S>(
    ctx: CallableStreamContext<S>,
    tx: tokio::sync::mpsc::Sender<Result<GenerateContentResponse>>,
) where
    S: BuildHasher + Sync + Send + 'static,
{
    let CallableStreamContext {
        models,
        model,
        contents,
        request_config,
        mut callable_tools,
        function_map,
        max_calls,
        append_history,
    } = ctx;
    tokio::spawn(async move {
        let mut conversation = contents;
        let mut history: Vec<Content> = Vec::new();
        let mut remaining_calls = max_calls;

        loop {
            if remaining_calls == 0 {
                break;
            }

            let stream = match models
                .generate_content_stream(&model, conversation.clone(), request_config.clone())
                .await
            {
                Ok(stream) => stream,
                Err(err) => {
                    let _ = tx.send(Err(err)).await;
                    break;
                }
            };

            let Some((function_calls, response_contents)) = forward_stream_items(stream, &tx).await
            else {
                return;
            };

            if function_calls.is_empty() {
                break;
            }

            let response_parts = match call_callable_tools(
                &mut callable_tools,
                &function_map,
                &function_calls,
            )
            .await
            {
                Ok(parts) => parts,
                Err(err) => {
                    let _ = tx.send(Err(err)).await;
                    break;
                }
            };

            if response_parts.is_empty() {
                break;
            }

            let call_content = build_function_call_content(&function_calls);
            let response_content = Content::from_parts(response_parts.clone(), Role::Function);

            if append_history {
                if history.is_empty() {
                    history.extend(conversation.clone());
                }
                history.push(call_content.clone());
                history.push(response_content.clone());
            }

            conversation.extend(response_contents);
            conversation.push(call_content);
            conversation.push(response_content.clone());
            remaining_calls = remaining_calls.saturating_sub(1);

            let synthetic = build_synthetic_afc_response(response_content, &history);
            if tx.send(Ok(synthetic)).await.is_err() {
                return;
            }
        }
    });
}

impl Models {
    pub(crate) const fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 生成内容（默认配置）。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置校验失败或响应解析失败时返回错误。
    pub async fn generate_content(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<GenerateContentResponse> {
        self.generate_content_with_config(model, contents, GenerateContentConfig::default())
            .await
    }

    /// 生成并解析 JSON 响应。
    ///
    /// # Errors
    ///
    /// 当请求失败、响应没有文本内容或 JSON 解析失败时返回错误。
    pub async fn generate_json<T: DeserializeOwned>(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<T> {
        self.generate_json_with_config(model, contents, GenerateContentConfig::default())
            .await
    }

    /// 生成并解析 JSON 响应（自定义配置）。
    ///
    /// # Errors
    ///
    /// 当请求失败、响应没有文本内容或 JSON 解析失败时返回错误。
    pub async fn generate_json_with_config<T: DeserializeOwned>(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
    ) -> Result<T> {
        let config = prepare_json_generation_config(config, None)?;

        let response = self
            .generate_content_with_config(model, contents, config)
            .await?;
        let text = first_candidate_text(&response).ok_or_else(|| Error::Parse {
            message: "Expected text response containing JSON".into(),
        })?;

        Ok(serde_json::from_str(&text)?)
    }

    /// 生成并解析 JSON 响应，同时自动附加 JSON Schema。
    ///
    /// # Errors
    ///
    /// 当请求失败、响应没有文本内容、schema 构建失败或 JSON 解析失败时返回错误。
    ///
    /// 需要启用 `schemars` feature。
    #[cfg(feature = "schemars")]
    pub async fn generate_json_with_schema<T>(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<T>
    where
        T: DeserializeOwned + schemars::JsonSchema,
    {
        self.generate_json_with_schema_with_config(
            model,
            contents,
            GenerateContentConfig::default(),
        )
        .await
    }

    /// 生成并解析 JSON 响应，同时自动附加 JSON Schema（自定义配置）。
    ///
    /// # Errors
    ///
    /// 当请求失败、响应没有文本内容、schema 构建失败或 JSON 解析失败时返回错误。
    ///
    /// 需要启用 `schemars` feature。
    #[cfg(feature = "schemars")]
    pub async fn generate_json_with_schema_with_config<T>(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
    ) -> Result<T>
    where
        T: DeserializeOwned + schemars::JsonSchema,
    {
        let schema = serde_json::to_value(schemars::schema_for!(T))?;
        let config = prepare_json_generation_config(config, Some(schema))?;
        let response = self
            .generate_content_with_config(model, contents, config)
            .await?;
        let text = first_candidate_text(&response).ok_or_else(|| Error::Parse {
            message: "Expected text response containing JSON".into(),
        })?;

        Ok(serde_json::from_str(&text)?)
    }

    /// 生成内容（自定义配置）。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置校验失败或响应解析失败时返回错误。
    pub async fn generate_content_with_config(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
    ) -> Result<GenerateContentResponse> {
        let should_return_http_response = config.should_return_http_response.unwrap_or(false);
        let model = model.into();
        validate_temperature(&model, &config)?;
        ThoughtSignatureValidator::new(&model).validate(&contents)?;
        validate_function_response_media(&model, &contents)?;
        validate_code_execution_image_inputs(&model, &contents, config.tools.as_deref())?;

        let backend = self.inner.config.backend;
        if backend == Backend::GeminiApi && config.model_armor_config.is_some() {
            return Err(Error::InvalidConfig {
                message: "model_armor_config is not supported in Gemini API".into(),
            });
        }
        if config.model_armor_config.is_some() && config.safety_settings.is_some() {
            return Err(Error::InvalidConfig {
                message: "model_armor_config cannot be combined with safety_settings".into(),
            });
        }

        let request = GenerateContentRequest {
            contents,
            system_instruction: config.system_instruction,
            generation_config: config.generation_config,
            safety_settings: config.safety_settings,
            model_armor_config: config.model_armor_config,
            tools: config.tools,
            tool_config: config.tool_config,
            cached_content: config.cached_content,
            labels: config.labels,
        };

        let url = build_model_method_url(&self.inner, &model, "generateContent")?;
        let body = match backend {
            Backend::GeminiApi => converters::generate_content_request_to_mldev(&request)?,
            Backend::VertexAi => converters::generate_content_request_to_vertex(&request)?,
        };

        let request = self.inner.http.post(url).json(&body);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        let headers = response.headers().clone();
        if should_return_http_response {
            let body = response.text().await.unwrap_or_default();
            return Ok(GenerateContentResponse {
                sdk_http_response: Some(sdk_http_response_from_headers_and_body(&headers, body)),
                candidates: Vec::new(),
                create_time: None,
                automatic_function_calling_history: None,
                prompt_feedback: None,
                usage_metadata: None,
                model_version: None,
                response_id: None,
            });
        }
        let value = response.json::<Value>().await?;
        let mut result = match backend {
            Backend::GeminiApi => converters::generate_content_response_from_mldev(value)?,
            Backend::VertexAi => converters::generate_content_response_from_vertex(value)?,
        };
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 生成内容（自动函数调用 + callable tools）。
    ///
    /// # Errors
    ///
    /// 当配置校验失败、自动函数调用执行失败或请求失败时返回错误。
    pub async fn generate_content_with_callable_tools(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
        mut callable_tools: Vec<Box<dyn CallableTool>>,
    ) -> Result<GenerateContentResponse> {
        if config.should_return_http_response.unwrap_or(false) {
            return Err(Error::InvalidConfig {
                message: "should_return_http_response is not supported in callable tools methods"
                    .into(),
            });
        }
        let model = model.into();
        if callable_tools.is_empty() {
            return self
                .generate_content_with_config(model, contents, config)
                .await;
        }

        validate_afc_config(&config)?;

        let mut callable_info = resolve_callable_tools(&mut callable_tools).await?;
        let has_callable = !callable_info.function_map.is_empty();
        let mut merged_tools = config.tools.clone().unwrap_or_default();
        merged_tools.append(&mut callable_info.tools);

        let mut request_config = config.clone();
        request_config.tools = Some(merged_tools);

        if should_disable_afc(&config, has_callable) {
            return self
                .generate_content_with_config(model, contents, request_config)
                .await;
        }

        validate_afc_tools(&callable_info.function_map, config.tools.as_deref())?;

        let max_calls = max_remote_calls(&config);
        let append_history = should_append_history(&config);
        let mut history: Vec<Content> = Vec::new();
        let mut conversation = contents.clone();
        let mut remaining_calls = max_calls;
        let mut response = self
            .generate_content_with_config(&model, conversation.clone(), request_config.clone())
            .await?;

        loop {
            let function_calls: Vec<FunctionCall> =
                response.function_calls().into_iter().cloned().collect();

            if function_calls.is_empty() {
                if append_history && !history.is_empty() {
                    response.automatic_function_calling_history = Some(history);
                }
                return Ok(response);
            }

            if remaining_calls == 0 {
                break;
            }

            let response_parts = call_callable_tools(
                &mut callable_tools,
                &callable_info.function_map,
                &function_calls,
            )
            .await?;
            if response_parts.is_empty() {
                break;
            }

            let call_content = build_function_call_content(&function_calls);
            let response_content = Content::from_parts(response_parts.clone(), Role::Function);

            if append_history {
                if history.is_empty() {
                    history.extend(conversation.clone());
                }
                history.push(call_content.clone());
                history.push(response_content.clone());
            }

            conversation.push(call_content);
            conversation.push(response_content);
            remaining_calls = remaining_calls.saturating_sub(1);

            response = self
                .generate_content_with_config(&model, conversation.clone(), request_config.clone())
                .await?;
        }

        if append_history && !history.is_empty() {
            response.automatic_function_calling_history = Some(history);
        }
        Ok(response)
    }

    /// 生成内容（流式 + 自动函数调用）。
    ///
    /// # Errors
    ///
    /// 当配置校验失败、自动函数调用执行失败或请求失败时返回错误。
    pub async fn generate_content_stream_with_callable_tools(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
        mut callable_tools: Vec<Box<dyn CallableTool>>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GenerateContentResponse>> + Send>>> {
        if config.should_return_http_response.unwrap_or(false) {
            return Err(Error::InvalidConfig {
                message: "should_return_http_response is not supported in callable tools methods"
                    .into(),
            });
        }
        let model = model.into();
        if callable_tools.is_empty() {
            return self.generate_content_stream(model, contents, config).await;
        }

        validate_afc_config(&config)?;

        let callable_info = resolve_callable_tools(&mut callable_tools).await?;
        let function_map = callable_info.function_map;
        let has_callable = !function_map.is_empty();
        let mut merged_tools = config.tools.clone().unwrap_or_default();
        merged_tools.extend(callable_info.tools);

        let mut request_config = config.clone();
        request_config.tools = Some(merged_tools);

        if should_disable_afc(&config, has_callable) {
            return self
                .generate_content_stream(model, contents, request_config)
                .await;
        }

        validate_afc_tools(&function_map, config.tools.as_deref())?;

        let max_calls = max_remote_calls(&config);
        let append_history = should_append_history(&config);
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        let models = self.clone();
        let ctx = CallableStreamContext {
            models,
            model,
            contents,
            request_config,
            callable_tools,
            function_map,
            max_calls,
            append_history,
        };
        spawn_callable_stream_loop(ctx, tx);

        let output = futures_util::stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        });

        Ok(Box::pin(output))
    }

    /// 生成内容（流式）。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置校验失败或响应解析失败时返回错误。
    pub async fn generate_content_stream(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GenerateContentResponse>> + Send>>> {
        if config.should_return_http_response.unwrap_or(false) {
            return Err(Error::InvalidConfig {
                message: "should_return_http_response is not supported in streaming methods".into(),
            });
        }
        let model = model.into();
        validate_temperature(&model, &config)?;
        ThoughtSignatureValidator::new(&model).validate(&contents)?;
        validate_function_response_media(&model, &contents)?;
        validate_code_execution_image_inputs(&model, &contents, config.tools.as_deref())?;

        let backend = self.inner.config.backend;
        if backend == Backend::GeminiApi && config.model_armor_config.is_some() {
            return Err(Error::InvalidConfig {
                message: "model_armor_config is not supported in Gemini API".into(),
            });
        }
        if config.model_armor_config.is_some() && config.safety_settings.is_some() {
            return Err(Error::InvalidConfig {
                message: "model_armor_config cannot be combined with safety_settings".into(),
            });
        }

        let request = GenerateContentRequest {
            contents,
            system_instruction: config.system_instruction,
            generation_config: config.generation_config,
            safety_settings: config.safety_settings,
            model_armor_config: config.model_armor_config,
            tools: config.tools,
            tool_config: config.tool_config,
            cached_content: config.cached_content,
            labels: config.labels,
        };

        let mut url = build_model_method_url(&self.inner, &model, "streamGenerateContent")?;
        url.push_str("?alt=sse");

        let body = match backend {
            Backend::GeminiApi => converters::generate_content_request_to_mldev(&request)?,
            Backend::VertexAi => converters::generate_content_request_to_vertex(&request)?,
        };

        let request = self.inner.http.post(url).json(&body);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }

        let headers = response.headers().clone();
        let sdk_http_response = sdk_http_response_from_headers(&headers);
        let stream = parse_sse_stream(response).map(move |item| {
            item.map(|mut resp| {
                resp.sdk_http_response = Some(sdk_http_response.clone());
                resp
            })
        });
        Ok(Box::pin(stream))
    }

    /// 生成内容事件流。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置校验失败或响应解析失败时返回错误。
    pub async fn generate_content_event_stream(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
    ) -> Result<GenerateContentEventStream> {
        let should_return_http_response = config.should_return_http_response.unwrap_or(false);
        if should_return_http_response {
            return Err(Error::InvalidConfig {
                message: "should_return_http_response is not supported in streaming".into(),
            });
        }

        let model = model.into();
        validate_temperature(&model, &config)?;
        ThoughtSignatureValidator::new(&model).validate(&contents)?;
        validate_function_response_media(&model, &contents)?;
        validate_code_execution_image_inputs(&model, &contents, config.tools.as_deref())?;

        let backend = self.inner.config.backend;
        if backend == Backend::GeminiApi && config.model_armor_config.is_some() {
            return Err(Error::InvalidConfig {
                message: "model_armor_config is not supported in Gemini API".into(),
            });
        }
        if config.model_armor_config.is_some() && config.safety_settings.is_some() {
            return Err(Error::InvalidConfig {
                message: "model_armor_config cannot be combined with safety_settings".into(),
            });
        }

        let request = GenerateContentRequest {
            contents,
            system_instruction: config.system_instruction,
            generation_config: config.generation_config,
            safety_settings: config.safety_settings,
            model_armor_config: config.model_armor_config,
            tools: config.tools,
            tool_config: config.tool_config,
            cached_content: config.cached_content,
            labels: config.labels,
        };

        let url = build_model_method_url(&self.inner, &model, "streamGenerateContent")?;
        let body = match backend {
            Backend::GeminiApi => converters::generate_content_request_to_mldev(&request)?,
            Backend::VertexAi => converters::generate_content_request_to_vertex(&request)?,
        };

        let request = self
            .inner
            .http
            .post(format!("{url}?alt=sse"))
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .json(&body);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }

        let headers = response.headers().clone();
        let sdk_http_response = sdk_http_response_from_headers(&headers);
        let saw_done = Arc::new(AtomicBool::new(false));
        let stream =
            parse_sse_stream_with_done_signal(response, saw_done.clone()).map(move |item| {
                item.map(|mut resp: GenerateContentResponse| {
                    resp.sdk_http_response = Some(sdk_http_response.clone());
                    resp
                })
            });

        Ok(GenerateContentEventStream::new(Box::pin(stream), saw_done))
    }

    /// 生成嵌入向量（默认配置）。
    ///
    /// # Errors
    ///
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn embed_content(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<EmbedContentResponse> {
        self.embed_content_with_config(model, contents, EmbedContentConfig::default())
            .await
    }

    /// 生成嵌入向量（自定义配置）。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置不合法或响应解析失败时返回错误。
    pub async fn embed_content_with_config(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: EmbedContentConfig,
    ) -> Result<EmbedContentResponse> {
        let model = model.into();
        let url = match self.inner.config.backend {
            Backend::GeminiApi => {
                build_model_method_url(&self.inner, &model, "batchEmbedContents")?
            }
            Backend::VertexAi => build_model_method_url(&self.inner, &model, "predict")?,
        };

        let body = match self.inner.config.backend {
            Backend::GeminiApi => build_embed_body_gemini(&model, &contents, &config)?,
            Backend::VertexAi => build_embed_body_vertex(&contents, &config)?,
        };

        let request = self.inner.http.post(url).json(&body);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }

        let headers = response.headers().clone();
        match self.inner.config.backend {
            Backend::GeminiApi => {
                let mut result = response.json::<EmbedContentResponse>().await?;
                result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
                Ok(result)
            }
            Backend::VertexAi => {
                let value = response.json::<Value>().await?;
                let mut result = convert_vertex_embed_response(&value)?;
                result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
                Ok(result)
            }
        }
    }

    /// 计数 tokens（默认配置）。
    ///
    /// # Errors
    ///
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn count_tokens(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<CountTokensResponse> {
        self.count_tokens_with_config(model, contents, CountTokensConfig::default())
            .await
    }

    /// 计数 tokens（自定义配置）。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置不合法或响应解析失败时返回错误。
    pub async fn count_tokens_with_config(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: CountTokensConfig,
    ) -> Result<CountTokensResponse> {
        let request = CountTokensRequest {
            contents,
            system_instruction: config.system_instruction,
            tools: config.tools,
            generation_config: config.generation_config,
        };

        let backend = self.inner.config.backend;
        let url = build_model_method_url(&self.inner, &model.into(), "countTokens")?;
        let body = match backend {
            Backend::GeminiApi => converters::count_tokens_request_to_mldev(&request)?,
            Backend::VertexAi => converters::count_tokens_request_to_vertex(&request)?,
        };
        let request = self.inner.http.post(url).json(&body);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        let headers = response.headers().clone();
        let value = response.json::<Value>().await?;
        let mut result = match backend {
            Backend::GeminiApi => converters::count_tokens_response_from_mldev(value)?,
            Backend::VertexAi => converters::count_tokens_response_from_vertex(value)?,
        };
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 计算 tokens（默认配置，仅 Vertex AI）。
    ///
    /// # Errors
    ///
    /// 当后端不支持或请求失败时返回错误。
    pub async fn compute_tokens(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<ComputeTokensResponse> {
        self.compute_tokens_with_config(model, contents, ComputeTokensConfig::default())
            .await
    }

    /// 计算 tokens（自定义配置，仅 Vertex AI）。
    ///
    /// # Errors
    ///
    /// 当后端不支持、配置不合法或请求失败时返回错误。
    pub async fn compute_tokens_with_config(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: ComputeTokensConfig,
    ) -> Result<ComputeTokensResponse> {
        if self.inner.config.backend != Backend::VertexAi {
            return Err(Error::InvalidConfig {
                message: "Compute tokens is only supported in Vertex AI backend".into(),
            });
        }

        let request = ComputeTokensRequest { contents };
        let url = build_model_method_url(&self.inner, &model.into(), "computeTokens")?;
        let mut body = converters::compute_tokens_request_to_vertex(&request)?;
        if let Some(options) = config.http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }

        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, config.http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, config.http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        let headers = response.headers().clone();
        let value = response.json::<Value>().await?;
        let mut result = converters::compute_tokens_response_from_vertex(value)?;
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 本地估算 tokens（离线估算器）。
    pub fn estimate_tokens_local(
        &self,
        contents: &[Content],
        estimator: &dyn TokenEstimator,
    ) -> CountTokensResponse {
        let total = i32::try_from(estimator.estimate_tokens(contents)).unwrap_or(i32::MAX);
        CountTokensResponse {
            sdk_http_response: None,
            total_tokens: Some(total),
            cached_content_token_count: None,
        }
    }

    /// 本地估算 tokens（包含 tools / system instruction / response schema）。
    pub fn estimate_tokens_local_with_config(
        &self,
        contents: &[Content],
        config: &CountTokensConfig,
        estimator: &dyn TokenEstimator,
    ) -> CountTokensResponse {
        let estimation_contents = crate::tokenizer::build_estimation_contents(contents, config);
        let total =
            i32::try_from(estimator.estimate_tokens(&estimation_contents)).unwrap_or(i32::MAX);
        CountTokensResponse {
            sdk_http_response: None,
            total_tokens: Some(total),
            cached_content_token_count: None,
        }
    }

    /// 计数 tokens（优先使用本地估算器）。
    ///
    /// # Errors
    ///
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn count_tokens_or_estimate(
        &self,
        model: impl Into<String> + Send,
        contents: Vec<Content>,
        config: CountTokensConfig,
        estimator: Option<&(dyn TokenEstimator + Sync)>,
    ) -> Result<CountTokensResponse> {
        if let Some(estimator) = estimator {
            return Ok(self.estimate_tokens_local_with_config(&contents, &config, estimator));
        }
        self.count_tokens_with_config(model, contents, config).await
    }

    /// 生成图像（Imagen）。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置不合法或响应解析失败时返回错误。
    pub async fn generate_images(
        &self,
        model: impl Into<String>,
        prompt: impl Into<String>,
        mut config: GenerateImagesConfig,
    ) -> Result<GenerateImagesResponse> {
        let http_options = config.http_options.take();
        let model = model.into();
        let prompt = prompt.into();
        let mut body = build_generate_images_body(self.inner.config.backend, &prompt, &config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let url = build_model_method_url(&self.inner, &model, "predict")?;

        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }

        let headers = response.headers().clone();
        let value = response.json::<Value>().await?;
        let mut result = parse_generate_images_response(&value);
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 编辑图像（仅 Vertex AI）。
    ///
    /// # Errors
    ///
    /// 当后端不支持、请求失败或响应解析失败时返回错误。
    pub async fn edit_image(
        &self,
        model: impl Into<String>,
        prompt: impl Into<String>,
        reference_images: Vec<ReferenceImage>,
        mut config: EditImageConfig,
    ) -> Result<EditImageResponse> {
        if self.inner.config.backend != Backend::VertexAi {
            return Err(Error::InvalidConfig {
                message: "Edit image is only supported in Vertex AI backend".into(),
            });
        }

        let http_options = config.http_options.take();
        let model = model.into();
        let prompt = prompt.into();
        let mut body = build_edit_image_body(&prompt, &reference_images, &config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let url = build_model_method_url(&self.inner, &model, "predict")?;

        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }

        let headers = response.headers().clone();
        let value = response.json::<Value>().await?;
        let mut result = parse_edit_image_response(&value);
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 放大图像（仅 Vertex AI）。
    ///
    /// # Errors
    ///
    /// 当后端不支持、请求失败或响应解析失败时返回错误。
    pub async fn upscale_image(
        &self,
        model: impl Into<String>,
        image: Image,
        upscale_factor: impl Into<String>,
        mut config: rust_genai_types::models::UpscaleImageConfig,
    ) -> Result<rust_genai_types::models::UpscaleImageResponse> {
        if self.inner.config.backend != Backend::VertexAi {
            return Err(Error::InvalidConfig {
                message: "Upscale image is only supported in Vertex AI backend".into(),
            });
        }

        let http_options = config.http_options.take();
        let model = model.into();
        let upscale_factor = upscale_factor.into();
        let mut body = build_upscale_image_body(&image, &upscale_factor, &config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let url = build_model_method_url(&self.inner, &model, "predict")?;

        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }

        let headers = response.headers().clone();
        let value = response.json::<Value>().await?;
        let mut result = parse_upscale_image_response(&value);
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// Recontext 图像（Vertex AI）。
    ///
    /// # Errors
    ///
    /// 当后端不支持、请求失败或响应解析失败时返回错误。
    pub async fn recontext_image(
        &self,
        model: impl Into<String>,
        source: RecontextImageSource,
        mut config: RecontextImageConfig,
    ) -> Result<RecontextImageResponse> {
        if self.inner.config.backend != Backend::VertexAi {
            return Err(Error::InvalidConfig {
                message: "Recontext image is only supported in Vertex AI backend".into(),
            });
        }

        let http_options = config.http_options.take();
        let model = model.into();
        let mut body = build_recontext_image_body(&source, &config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let url = build_model_method_url(&self.inner, &model, "predict")?;

        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }

        let value = response.json::<Value>().await?;
        Ok(parse_recontext_image_response(&value))
    }

    /// Segment 图像（Vertex AI）。
    ///
    /// # Errors
    ///
    /// 当后端不支持、请求失败或响应解析失败时返回错误。
    pub async fn segment_image(
        &self,
        model: impl Into<String>,
        source: SegmentImageSource,
        mut config: SegmentImageConfig,
    ) -> Result<SegmentImageResponse> {
        if self.inner.config.backend != Backend::VertexAi {
            return Err(Error::InvalidConfig {
                message: "Segment image is only supported in Vertex AI backend".into(),
            });
        }

        let http_options = config.http_options.take();
        let model = model.into();
        let mut body = build_segment_image_body(&source, &config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let url = build_model_method_url(&self.inner, &model, "predict")?;

        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }

        let value = response.json::<Value>().await?;
        Ok(parse_segment_image_response(&value))
    }

    /// 生成视频（Veo）。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置不合法或响应解析失败时返回错误。
    pub async fn generate_videos(
        &self,
        model: impl Into<String>,
        source: GenerateVideosSource,
        mut config: GenerateVideosConfig,
    ) -> Result<GenerateVideosOperation> {
        let http_options = config.http_options.take();
        let model = model.into();
        let mut body = build_generate_videos_body(self.inner.config.backend, &source, &config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let url = build_model_method_url(&self.inner, &model, "predictLongRunning")?;

        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }

        let value = response.json::<Value>().await?;
        parse_generate_videos_operation(value, self.inner.config.backend)
    }

    /// 生成视频（仅文本提示）。
    ///
    /// # Errors
    ///
    /// 当请求失败或配置不合法时返回错误。
    pub async fn generate_videos_with_prompt(
        &self,
        model: impl Into<String>,
        prompt: impl Into<String>,
        config: GenerateVideosConfig,
    ) -> Result<GenerateVideosOperation> {
        let source = GenerateVideosSource {
            prompt: Some(prompt.into()),
            ..GenerateVideosSource::default()
        };
        self.generate_videos(model, source, config).await
    }

    /// 列出模型（基础列表）。
    ///
    /// # Errors
    ///
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn list(&self) -> Result<ListModelsResponse> {
        self.list_with_config(ListModelsConfig::default()).await
    }

    /// 列出模型（带配置）。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置不合法或响应解析失败时返回错误。
    pub async fn list_with_config(&self, config: ListModelsConfig) -> Result<ListModelsResponse> {
        let url = build_models_list_url(&self.inner, &config)?;
        let request = self.inner.http.get(url);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        let headers = response.headers().clone();
        let mut result = response.json::<ListModelsResponse>().await?;
        result.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(result)
    }

    /// 列出所有模型（自动翻页）。
    ///
    /// # Errors
    ///
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn all(&self) -> Result<Vec<Model>> {
        self.all_with_config(ListModelsConfig::default()).await
    }

    /// 列出所有模型（带配置，自动翻页）。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置不合法或响应解析失败时返回错误。
    pub async fn all_with_config(&self, mut config: ListModelsConfig) -> Result<Vec<Model>> {
        let mut models = Vec::new();
        loop {
            let response = self.list_with_config(config.clone()).await?;
            if let Some(items) = response.models {
                models.extend(items);
            }
            match response.next_page_token {
                Some(token) if !token.is_empty() => {
                    config.page_token = Some(token);
                }
                _ => break,
            }
        }
        Ok(models)
    }

    /// 获取单个模型信息。
    ///
    /// # Errors
    ///
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn get(&self, model: impl Into<String>) -> Result<Model> {
        let url = build_model_get_url(&self.inner, &model.into())?;
        let request = self.inner.http.get(url);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        let result = response.json::<Model>().await?;
        Ok(result)
    }

    /// 更新模型信息。
    ///
    /// # Errors
    ///
    /// 当请求失败、配置不合法或响应解析失败时返回错误。
    pub async fn update(
        &self,
        model: impl Into<String>,
        mut config: UpdateModelConfig,
    ) -> Result<Model> {
        let http_options = config.http_options.take();
        let url =
            build_model_get_url_with_options(&self.inner, &model.into(), http_options.as_ref())?;

        let mut body = serde_json::to_value(&config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let mut request = self.inner.http.patch(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        Ok(response.json::<Model>().await?)
    }

    /// 删除模型。
    ///
    /// # Errors
    ///
    /// 当请求失败或响应解析失败时返回错误。
    pub async fn delete(
        &self,
        model: impl Into<String>,
        mut config: DeleteModelConfig,
    ) -> Result<DeleteModelResponse> {
        let http_options = config.http_options.take();
        let url =
            build_model_get_url_with_options(&self.inner, &model.into(), http_options.as_ref())?;

        let mut request = self.inner.http.delete(url);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self
            .inner
            .send_with_http_options(request, http_options.as_ref())
            .await?;
        if !response.status().is_success() {
            return Err(Error::api_error_from_response(response, None).await);
        }
        let headers = response.headers().clone();
        if response.content_length().unwrap_or(0) == 0 {
            let resp = DeleteModelResponse {
                sdk_http_response: Some(sdk_http_response_from_headers(&headers)),
            };
            return Ok(resp);
        }
        let mut resp = response
            .json::<DeleteModelResponse>()
            .await
            .unwrap_or_default();
        resp.sdk_http_response = Some(sdk_http_response_from_headers(&headers));
        Ok(resp)
    }
}

fn first_candidate_text(response: &GenerateContentResponse) -> Option<String> {
    let mut text = String::new();
    let content = response.candidates.first()?.content.as_ref()?;
    for part in &content.parts {
        if let Some(part_text) = part.text_value() {
            text.push_str(part_text);
        }
    }
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests;
