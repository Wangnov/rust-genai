//! Models API surface.

use std::pin::Pin;
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use futures_util::{Stream, StreamExt};
use rust_genai_types::content::{Content, FunctionCall, Part, Role};
use rust_genai_types::converters;
use rust_genai_types::models::{
    ComputeTokensConfig, ComputeTokensRequest, ComputeTokensResponse, ContentEmbedding,
    CountTokensConfig, CountTokensRequest, CountTokensResponse, DeleteModelConfig,
    DeleteModelResponse, EditImageConfig, EditImageResponse, EmbedContentConfig,
    EmbedContentMetadata, EmbedContentResponse, EntityLabel, GenerateContentConfig,
    GenerateContentRequest, GenerateImagesConfig, GenerateImagesResponse, GenerateVideosConfig,
    GenerateVideosSource, GeneratedImage, GeneratedImageMask, Image, ListModelsConfig,
    ListModelsResponse, Model, RecontextImageConfig, RecontextImageResponse, RecontextImageSource,
    ReferenceImage, SafetyAttributes, SegmentImageConfig, SegmentImageResponse, SegmentImageSource,
    UpdateModelConfig, Video, VideoGenerationMask, VideoGenerationReferenceImage,
};
use rust_genai_types::response::GenerateContentResponse;

use crate::afc::{
    call_callable_tools, max_remote_calls, resolve_callable_tools, should_append_history,
    should_disable_afc, validate_afc_config, validate_afc_tools, CallableTool,
};
use crate::client::{Backend, ClientInner};
use crate::error::{Error, Result};
use crate::model_capabilities::{
    validate_code_execution_image_inputs, validate_function_response_media,
};
use crate::sse::parse_sse_stream;
use crate::thinking::{validate_temperature, ThoughtSignatureValidator};
use crate::tokenizer::TokenEstimator;
use serde_json::{Map, Number, Value};

#[derive(Clone)]
pub struct Models {
    pub(crate) inner: Arc<ClientInner>,
}

impl Models {
    pub(crate) fn new(inner: Arc<ClientInner>) -> Self {
        Self { inner }
    }

    /// 生成内容（默认配置）。
    pub async fn generate_content(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<GenerateContentResponse> {
        self.generate_content_with_config(model, contents, GenerateContentConfig::default())
            .await
    }

    /// 生成内容（自定义配置）。
    pub async fn generate_content_with_config(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
    ) -> Result<GenerateContentResponse> {
        let model = model.into();
        validate_temperature(&model, &config)?;
        ThoughtSignatureValidator::new(&model).validate(&contents)?;
        validate_function_response_media(&model, &contents)?;
        validate_code_execution_image_inputs(&model, &contents, config.tools.as_deref())?;

        let request = GenerateContentRequest {
            contents,
            system_instruction: config.system_instruction,
            generation_config: config.generation_config,
            safety_settings: config.safety_settings,
            tools: config.tools,
            tool_config: config.tool_config,
            cached_content: config.cached_content,
            labels: config.labels,
        };

        let backend = self.inner.config.backend;
        let url = build_model_method_url(&self.inner, &model, "generateContent")?;
        let body = match backend {
            Backend::GeminiApi => converters::generate_content_request_to_mldev(&request)?,
            Backend::VertexAi => converters::generate_content_request_to_vertex(&request)?,
        };

        let request = self.inner.http.post(url).json(&body);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let value = response.json::<Value>().await?;
        let result = match backend {
            Backend::GeminiApi => converters::generate_content_response_from_mldev(value)?,
            Backend::VertexAi => converters::generate_content_response_from_vertex(value)?,
        };
        Ok(result)
    }

    /// 生成内容（自动函数调用 + callable tools）。
    pub async fn generate_content_with_callable_tools(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
        mut callable_tools: Vec<Box<dyn CallableTool>>,
    ) -> Result<GenerateContentResponse> {
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
    pub async fn generate_content_stream_with_callable_tools(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
        mut callable_tools: Vec<Box<dyn CallableTool>>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GenerateContentResponse>> + Send>>> {
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

        tokio::spawn(async move {
            let mut conversation = contents;
            let mut history: Vec<Content> = Vec::new();
            let mut remaining_calls = max_calls;
            let mut callable_tools = callable_tools;
            let request_config = request_config;

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

                let mut stream = stream;
                let mut function_calls: Vec<FunctionCall> = Vec::new();
                let mut response_contents: Vec<Content> = Vec::new();

                while let Some(item) = stream.next().await {
                    if let Ok(response) = &item {
                        if let Some(content) =
                            response.candidates.first().and_then(|c| c.content.clone())
                        {
                            for part in &content.parts {
                                if let Some(call) = part.function_call_ref() {
                                    function_calls.push(call.clone());
                                }
                            }
                            response_contents.push(content);
                        }
                    }

                    if tx.send(item).await.is_err() {
                        return;
                    }
                }

                if function_calls.is_empty() {
                    break;
                }

                let response_parts =
                    match call_callable_tools(&mut callable_tools, &function_map, &function_calls)
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

                let mut synthetic = GenerateContentResponse {
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

                if append_history && !history.is_empty() {
                    synthetic.automatic_function_calling_history = Some(history.clone());
                }

                if tx.send(Ok(synthetic)).await.is_err() {
                    return;
                }
            }
        });

        let output = futures_util::stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        });

        Ok(Box::pin(output))
    }

    /// 生成内容（流式）。
    pub async fn generate_content_stream(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: GenerateContentConfig,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GenerateContentResponse>> + Send>>> {
        let model = model.into();
        validate_temperature(&model, &config)?;
        ThoughtSignatureValidator::new(&model).validate(&contents)?;
        validate_function_response_media(&model, &contents)?;
        validate_code_execution_image_inputs(&model, &contents, config.tools.as_deref())?;

        let request = GenerateContentRequest {
            contents,
            system_instruction: config.system_instruction,
            generation_config: config.generation_config,
            safety_settings: config.safety_settings,
            tools: config.tools,
            tool_config: config.tool_config,
            cached_content: config.cached_content,
            labels: config.labels,
        };

        let mut url = build_model_method_url(&self.inner, &model, "streamGenerateContent")?;
        url.push_str("?alt=sse");

        let request = self.inner.http.post(url).json(&request);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        Ok(Box::pin(parse_sse_stream(response)))
    }

    /// 生成嵌入向量（默认配置）。
    pub async fn embed_content(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<EmbedContentResponse> {
        self.embed_content_with_config(model, contents, EmbedContentConfig::default())
            .await
    }

    /// 生成嵌入向量（自定义配置）。
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
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        match self.inner.config.backend {
            Backend::GeminiApi => Ok(response.json::<EmbedContentResponse>().await?),
            Backend::VertexAi => {
                let value = response.json::<Value>().await?;
                Ok(convert_vertex_embed_response(value)?)
            }
        }
    }

    /// 计数 tokens（默认配置）。
    pub async fn count_tokens(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<CountTokensResponse> {
        self.count_tokens_with_config(model, contents, CountTokensConfig::default())
            .await
    }

    /// 计数 tokens（自定义配置）。
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
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let value = response.json::<Value>().await?;
        let result = match backend {
            Backend::GeminiApi => converters::count_tokens_response_from_mldev(value)?,
            Backend::VertexAi => converters::count_tokens_response_from_vertex(value)?,
        };
        Ok(result)
    }

    /// 计算 tokens（默认配置，仅 Vertex AI）。
    pub async fn compute_tokens(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
    ) -> Result<ComputeTokensResponse> {
        self.compute_tokens_with_config(model, contents, ComputeTokensConfig::default())
            .await
    }

    /// 计算 tokens（自定义配置，仅 Vertex AI）。
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

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let value = response.json::<Value>().await?;
        let result = converters::compute_tokens_response_from_vertex(value)?;
        Ok(result)
    }

    /// 本地估算 tokens（离线估算器）。
    pub fn estimate_tokens_local(
        &self,
        contents: &[Content],
        estimator: &dyn TokenEstimator,
    ) -> CountTokensResponse {
        let total = estimator.estimate_tokens(contents) as i32;
        CountTokensResponse {
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
        let total = estimator.estimate_tokens(&estimation_contents) as i32;
        CountTokensResponse {
            total_tokens: Some(total),
            cached_content_token_count: None,
        }
    }

    /// 计数 tokens（优先使用本地估算器）。
    pub async fn count_tokens_or_estimate(
        &self,
        model: impl Into<String>,
        contents: Vec<Content>,
        config: CountTokensConfig,
        estimator: Option<&dyn TokenEstimator>,
    ) -> Result<CountTokensResponse> {
        if let Some(estimator) = estimator {
            return Ok(self.estimate_tokens_local_with_config(&contents, &config, estimator));
        }
        self.count_tokens_with_config(model, contents, config).await
    }

    /// 生成图像（Imagen）。
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

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let value = response.json::<Value>().await?;
        parse_generate_images_response(value, self.inner.config.backend)
    }

    /// 编辑图像（仅 Vertex AI）。
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

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let value = response.json::<Value>().await?;
        parse_edit_image_response(value)
    }

    /// 放大图像（仅 Vertex AI）。
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

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let value = response.json::<Value>().await?;
        parse_upscale_image_response(value)
    }

    /// Recontext 图像（Vertex AI）。
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

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let value = response.json::<Value>().await?;
        parse_recontext_image_response(value)
    }

    /// Segment 图像（Vertex AI）。
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

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let value = response.json::<Value>().await?;
        parse_segment_image_response(value)
    }

    /// 生成视频（Veo）。
    pub async fn generate_videos(
        &self,
        model: impl Into<String>,
        source: GenerateVideosSource,
        mut config: GenerateVideosConfig,
    ) -> Result<rust_genai_types::operations::Operation> {
        let http_options = config.http_options.take();
        let model = model.into();
        let mut body = build_generate_videos_body(self.inner.config.backend, &source, &config)?;
        if let Some(options) = http_options.as_ref() {
            merge_extra_body(&mut body, options)?;
        }
        let url = build_model_method_url(&self.inner, &model, "predictLongRunning")?;

        let mut request = self.inner.http.post(url).json(&body);
        request = apply_http_options(request, http_options.as_ref())?;

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let value = response.json::<Value>().await?;
        parse_generate_videos_operation(value, self.inner.config.backend)
    }

    /// 生成视频（仅文本提示）。
    pub async fn generate_videos_with_prompt(
        &self,
        model: impl Into<String>,
        prompt: impl Into<String>,
        config: GenerateVideosConfig,
    ) -> Result<rust_genai_types::operations::Operation> {
        let source = GenerateVideosSource {
            prompt: Some(prompt.into()),
            ..GenerateVideosSource::default()
        };
        self.generate_videos(model, source, config).await
    }

    /// 列出模型（基础列表）。
    pub async fn list(&self) -> Result<ListModelsResponse> {
        self.list_with_config(ListModelsConfig::default()).await
    }

    /// 列出模型（带配置）。
    pub async fn list_with_config(&self, config: ListModelsConfig) -> Result<ListModelsResponse> {
        let url = build_models_list_url(&self.inner, &config)?;
        let request = self.inner.http.get(url);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let result = response.json::<ListModelsResponse>().await?;
        Ok(result)
    }

    /// 列出所有模型（自动翻页）。
    pub async fn all(&self) -> Result<Vec<Model>> {
        self.all_with_config(ListModelsConfig::default()).await
    }

    /// 列出所有模型（带配置，自动翻页）。
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
    pub async fn get(&self, model: impl Into<String>) -> Result<Model> {
        let url = build_model_get_url(&self.inner, &model.into())?;
        let request = self.inner.http.get(url);
        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        let result = response.json::<Model>().await?;
        Ok(result)
    }

    /// 更新模型信息。
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

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        Ok(response.json::<Model>().await?)
    }

    /// 删除模型。
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

        let response = self.inner.send(request).await?;
        if !response.status().is_success() {
            return Err(Error::ApiError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }
        if response.content_length().unwrap_or(0) == 0 {
            return Ok(DeleteModelResponse::default());
        }
        Ok(response
            .json::<DeleteModelResponse>()
            .await
            .unwrap_or_default())
    }
}

fn transform_model_name(backend: Backend, model: &str) -> String {
    match backend {
        Backend::GeminiApi => {
            if model.starts_with("models/") {
                model.to_string()
            } else {
                format!("models/{model}")
            }
        }
        Backend::VertexAi => {
            if model.starts_with("projects/") || model.starts_with("publishers/") {
                model.to_string()
            } else {
                format!("publishers/google/models/{model}")
            }
        }
    }
}

fn build_model_method_url(inner: &ClientInner, model: &str, method: &str) -> Result<String> {
    let model = transform_model_name(inner.config.backend, model);
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/{model}:{method}"),
        Backend::VertexAi => {
            let vertex =
                inner
                    .config
                    .vertex_config
                    .as_ref()
                    .ok_or_else(|| Error::InvalidConfig {
                        message: "Vertex config missing".into(),
                    })?;
            format!(
                "{base}{version}/projects/{}/locations/{}/{}:{method}",
                vertex.project, vertex.location, model
            )
        }
    };
    Ok(url)
}

fn build_model_get_url(inner: &ClientInner, model: &str) -> Result<String> {
    let model = transform_model_name(inner.config.backend, model);
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/{model}"),
        Backend::VertexAi => {
            let vertex =
                inner
                    .config
                    .vertex_config
                    .as_ref()
                    .ok_or_else(|| Error::InvalidConfig {
                        message: "Vertex config missing".into(),
                    })?;
            format!(
                "{base}{version}/projects/{}/locations/{}/{}",
                vertex.project, vertex.location, model
            )
        }
    };
    Ok(url)
}

fn build_model_get_url_with_options(
    inner: &ClientInner,
    model: &str,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<String> {
    let model = transform_model_name(inner.config.backend, model);
    let base = http_options
        .and_then(|opts| opts.base_url.as_deref())
        .unwrap_or(&inner.api_client.base_url);
    let version = http_options
        .and_then(|opts| opts.api_version.as_deref())
        .unwrap_or(&inner.api_client.api_version);
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/{model}"),
        Backend::VertexAi => {
            let vertex =
                inner
                    .config
                    .vertex_config
                    .as_ref()
                    .ok_or_else(|| Error::InvalidConfig {
                        message: "Vertex config missing".into(),
                    })?;
            format!(
                "{base}{version}/projects/{}/locations/{}/{}",
                vertex.project, vertex.location, model
            )
        }
    };
    Ok(url)
}

fn build_models_list_url(inner: &ClientInner, config: &ListModelsConfig) -> Result<String> {
    let base = &inner.api_client.base_url;
    let version = &inner.api_client.api_version;
    let url = match inner.config.backend {
        Backend::GeminiApi => format!("{base}{version}/models"),
        Backend::VertexAi => {
            let vertex =
                inner
                    .config
                    .vertex_config
                    .as_ref()
                    .ok_or_else(|| Error::InvalidConfig {
                        message: "Vertex config missing".into(),
                    })?;
            format!(
                "{base}{version}/projects/{}/locations/{}/publishers/google/models",
                vertex.project, vertex.location
            )
        }
    };
    add_list_query_params(url, config)
}

fn add_list_query_params(url: String, config: &ListModelsConfig) -> Result<String> {
    let mut url = reqwest::Url::parse(&url).map_err(|err| Error::InvalidConfig {
        message: err.to_string(),
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        if let Some(page_size) = config.page_size {
            pairs.append_pair("pageSize", &page_size.to_string());
        }
        if let Some(page_token) = &config.page_token {
            pairs.append_pair("pageToken", page_token);
        }
        if let Some(filter) = &config.filter {
            pairs.append_pair("filter", filter);
        }
        if let Some(query_base) = config.query_base {
            pairs.append_pair("queryBase", if query_base { "true" } else { "false" });
        }
    }
    Ok(url.to_string())
}

fn build_embed_body_gemini(
    model: &str,
    contents: &[Content],
    config: &EmbedContentConfig,
) -> Result<Value> {
    if config.mime_type.is_some() || config.auto_truncate.is_some() {
        return Err(Error::InvalidConfig {
            message: "mime_type/auto_truncate not supported in Gemini API".into(),
        });
    }

    let mut requests: Vec<Value> = Vec::new();
    for content in contents {
        let mut obj = Map::new();
        obj.insert(
            "model".to_string(),
            Value::String(transform_model_name(Backend::GeminiApi, model)),
        );
        obj.insert("content".to_string(), serde_json::to_value(content)?);
        if let Some(task_type) = &config.task_type {
            obj.insert("taskType".to_string(), Value::String(task_type.clone()));
        }
        if let Some(title) = &config.title {
            obj.insert("title".to_string(), Value::String(title.clone()));
        }
        if let Some(output_dimensionality) = config.output_dimensionality {
            obj.insert(
                "outputDimensionality".to_string(),
                Value::Number(Number::from(output_dimensionality as i64)),
            );
        }
        requests.push(Value::Object(obj));
    }

    Ok(Value::Object({
        let mut root = Map::new();
        root.insert("requests".to_string(), Value::Array(requests));
        root
    }))
}

fn build_embed_body_vertex(contents: &[Content], config: &EmbedContentConfig) -> Result<Value> {
    let mut instances: Vec<Value> = Vec::new();
    for content in contents {
        let mut obj = Map::new();
        obj.insert("content".to_string(), serde_json::to_value(content)?);
        if let Some(task_type) = &config.task_type {
            obj.insert("task_type".to_string(), Value::String(task_type.clone()));
        }
        if let Some(title) = &config.title {
            obj.insert("title".to_string(), Value::String(title.clone()));
        }
        if let Some(mime_type) = &config.mime_type {
            obj.insert("mimeType".to_string(), Value::String(mime_type.clone()));
        }
        instances.push(Value::Object(obj));
    }

    let mut root = Map::new();
    root.insert("instances".to_string(), Value::Array(instances));

    let mut parameters = Map::new();
    if let Some(output_dimensionality) = config.output_dimensionality {
        parameters.insert(
            "outputDimensionality".to_string(),
            Value::Number(Number::from(output_dimensionality as i64)),
        );
    }
    if let Some(auto_truncate) = config.auto_truncate {
        parameters.insert("autoTruncate".to_string(), Value::Bool(auto_truncate));
    }
    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

fn convert_vertex_embed_response(value: Value) -> Result<EmbedContentResponse> {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut embeddings: Vec<ContentEmbedding> = Vec::new();
    for item in predictions {
        if let Some(embedding_value) = item.get("embeddings") {
            let embedding: ContentEmbedding = serde_json::from_value(embedding_value.clone())?;
            embeddings.push(embedding);
        }
    }

    let metadata: Option<EmbedContentMetadata> = value
        .get("metadata")
        .map(|meta| serde_json::from_value(meta.clone()))
        .transpose()?;

    Ok(EmbedContentResponse {
        embeddings: Some(embeddings),
        metadata,
    })
}

fn build_generate_images_body(
    backend: Backend,
    prompt: &str,
    config: &GenerateImagesConfig,
) -> Result<Value> {
    let mut instances = Vec::new();
    let mut instance = Map::new();
    instance.insert("prompt".to_string(), Value::String(prompt.to_string()));
    instances.push(Value::Object(instance));

    let mut root = Map::new();
    root.insert("instances".to_string(), Value::Array(instances));

    let mut parameters = Map::new();
    let mut output_options = Map::new();

    if let Some(value) = &config.output_gcs_uri {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "output_gcs_uri is not supported in Gemini API".into(),
            });
        }
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.negative_prompt {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "negative_prompt is not supported in Gemini API".into(),
            });
        }
        parameters.insert("negativePrompt".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.number_of_images {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = &config.aspect_ratio {
        parameters.insert("aspectRatio".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.guidance_scale {
        parameters.insert(
            "guidanceScale".to_string(),
            Value::Number(Number::from_f64(value as f64).unwrap_or_else(|| Number::from(0))),
        );
    }
    if let Some(value) = config.seed {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "seed is not supported in Gemini API".into(),
            });
        }
        parameters.insert("seed".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = config.safety_filter_level {
        parameters.insert("safetySetting".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.person_generation {
        parameters.insert("personGeneration".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.include_safety_attributes {
        parameters.insert("includeSafetyAttributes".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.include_rai_reason {
        parameters.insert("includeRaiReason".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.language {
        parameters.insert("language".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = &config.output_mime_type {
        output_options.insert("mimeType".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.output_compression_quality {
        output_options.insert(
            "compressionQuality".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if !output_options.is_empty() {
        parameters.insert("outputOptions".to_string(), Value::Object(output_options));
    }
    if let Some(value) = config.add_watermark {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "add_watermark is not supported in Gemini API".into(),
            });
        }
        parameters.insert("addWatermark".to_string(), Value::Bool(value));
    }
    if let Some(labels) = &config.labels {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "labels is not supported in Gemini API".into(),
            });
        }
        root.insert("labels".to_string(), serde_json::to_value(labels)?);
    }
    if let Some(value) = &config.image_size {
        parameters.insert("sampleImageSize".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.enhance_prompt {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "enhance_prompt is not supported in Gemini API".into(),
            });
        }
        parameters.insert("enhancePrompt".to_string(), Value::Bool(value));
    }

    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

fn build_edit_image_body(
    prompt: &str,
    reference_images: &[ReferenceImage],
    config: &EditImageConfig,
) -> Result<Value> {
    let mut instances = Vec::new();
    let mut instance = Map::new();
    instance.insert("prompt".to_string(), Value::String(prompt.to_string()));
    if !reference_images.is_empty() {
        let mut refs = Vec::new();
        for image in reference_images {
            refs.push(reference_image_to_vertex(image)?);
        }
        instance.insert("referenceImages".to_string(), Value::Array(refs));
    }
    instances.push(Value::Object(instance));

    let mut root = Map::new();
    root.insert("instances".to_string(), Value::Array(instances));

    let mut parameters = Map::new();
    let mut output_options = Map::new();
    let mut edit_config = Map::new();

    if let Some(value) = &config.output_gcs_uri {
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.negative_prompt {
        parameters.insert("negativePrompt".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.number_of_images {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = &config.aspect_ratio {
        parameters.insert("aspectRatio".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.guidance_scale {
        parameters.insert(
            "guidanceScale".to_string(),
            Value::Number(Number::from_f64(value as f64).unwrap_or_else(|| Number::from(0))),
        );
    }
    if let Some(value) = config.seed {
        parameters.insert("seed".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = config.safety_filter_level {
        parameters.insert("safetySetting".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.person_generation {
        parameters.insert("personGeneration".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.include_safety_attributes {
        parameters.insert("includeSafetyAttributes".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.include_rai_reason {
        parameters.insert("includeRaiReason".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.language {
        parameters.insert("language".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = &config.output_mime_type {
        output_options.insert("mimeType".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.output_compression_quality {
        output_options.insert(
            "compressionQuality".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if !output_options.is_empty() {
        parameters.insert("outputOptions".to_string(), Value::Object(output_options));
    }
    if let Some(value) = config.add_watermark {
        parameters.insert("addWatermark".to_string(), Value::Bool(value));
    }
    if let Some(labels) = &config.labels {
        root.insert("labels".to_string(), serde_json::to_value(labels)?);
    }
    if let Some(value) = config.edit_mode {
        parameters.insert("editMode".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.base_steps {
        edit_config.insert("baseSteps".to_string(), Value::Number(Number::from(value)));
    }
    if !edit_config.is_empty() {
        parameters.insert("editConfig".to_string(), Value::Object(edit_config));
    }

    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

fn build_upscale_image_body(
    image: &Image,
    upscale_factor: &str,
    config: &rust_genai_types::models::UpscaleImageConfig,
) -> Result<Value> {
    let mut instances = Vec::new();
    let mut instance = Map::new();
    instance.insert("image".to_string(), image_to_vertex(image)?);
    instances.push(Value::Object(instance));

    let mut root = Map::new();
    root.insert("instances".to_string(), Value::Array(instances));

    let mut parameters = Map::new();
    let mut output_options = Map::new();
    let mut upscale_config = Map::new();

    parameters.insert(
        "mode".to_string(),
        Value::String(config.mode.clone().unwrap_or_else(|| "upscale".to_string())),
    );

    if let Some(value) = config.number_of_images {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    } else {
        parameters.insert("sampleCount".to_string(), Value::Number(Number::from(1)));
    }

    if let Some(value) = &config.output_gcs_uri {
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.safety_filter_level {
        parameters.insert("safetySetting".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.person_generation {
        parameters.insert("personGeneration".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.include_rai_reason {
        parameters.insert("includeRaiReason".to_string(), Value::Bool(value));
    }
    if let Some(value) = &config.output_mime_type {
        output_options.insert("mimeType".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.output_compression_quality {
        output_options.insert(
            "compressionQuality".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if !output_options.is_empty() {
        parameters.insert("outputOptions".to_string(), Value::Object(output_options));
    }
    if let Some(value) = config.enhance_input_image {
        upscale_config.insert("enhanceInputImage".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.image_preservation_factor {
        upscale_config.insert(
            "imagePreservationFactor".to_string(),
            Value::Number(Number::from_f64(value as f64).unwrap_or_else(|| Number::from(0))),
        );
    }
    upscale_config.insert(
        "upscaleFactor".to_string(),
        Value::String(upscale_factor.to_string()),
    );
    parameters.insert("upscaleConfig".to_string(), Value::Object(upscale_config));

    if let Some(labels) = &config.labels {
        root.insert("labels".to_string(), serde_json::to_value(labels)?);
    }

    root.insert("parameters".to_string(), Value::Object(parameters));

    Ok(Value::Object(root))
}

fn build_recontext_image_body(
    source: &RecontextImageSource,
    config: &RecontextImageConfig,
) -> Result<Value> {
    let mut instance = Map::new();
    if let Some(prompt) = &source.prompt {
        instance.insert("prompt".to_string(), Value::String(prompt.clone()));
    }
    if let Some(person_image) = &source.person_image {
        let mut person = Map::new();
        person.insert("image".to_string(), image_to_vertex(person_image)?);
        instance.insert("personImage".to_string(), Value::Object(person));
    }
    if let Some(product_images) = &source.product_images {
        let mut products = Vec::new();
        for item in product_images {
            if let Some(image) = &item.product_image {
                let mut product = Map::new();
                product.insert("image".to_string(), image_to_vertex(image)?);
                products.push(Value::Object(product));
            }
        }
        if !products.is_empty() {
            instance.insert("productImages".to_string(), Value::Array(products));
        }
    }

    let mut root = Map::new();
    root.insert(
        "instances".to_string(),
        Value::Array(vec![Value::Object(instance)]),
    );

    let mut parameters = Map::new();
    let mut output_options = Map::new();

    if let Some(value) = config.number_of_images {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = config.base_steps {
        parameters.insert("baseSteps".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = &config.output_gcs_uri {
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.seed {
        parameters.insert("seed".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = config.safety_filter_level {
        parameters.insert("safetySetting".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.person_generation {
        parameters.insert("personGeneration".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.add_watermark {
        parameters.insert("addWatermark".to_string(), Value::Bool(value));
    }
    if let Some(value) = &config.output_mime_type {
        output_options.insert("mimeType".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.output_compression_quality {
        output_options.insert(
            "compressionQuality".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if !output_options.is_empty() {
        parameters.insert("outputOptions".to_string(), Value::Object(output_options));
    }
    if let Some(value) = config.enhance_prompt {
        parameters.insert("enhancePrompt".to_string(), Value::Bool(value));
    }
    if let Some(labels) = &config.labels {
        root.insert("labels".to_string(), serde_json::to_value(labels)?);
    }

    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

fn build_segment_image_body(
    source: &SegmentImageSource,
    config: &SegmentImageConfig,
) -> Result<Value> {
    let mut instance = Map::new();
    if let Some(prompt) = &source.prompt {
        instance.insert("prompt".to_string(), Value::String(prompt.clone()));
    }
    if let Some(image) = &source.image {
        instance.insert("image".to_string(), image_to_vertex(image)?);
    }
    if let Some(scribble) = &source.scribble_image {
        if let Some(image) = &scribble.image {
            let mut scribble_map = Map::new();
            scribble_map.insert("image".to_string(), image_to_vertex(image)?);
            instance.insert("scribble".to_string(), Value::Object(scribble_map));
        }
    }

    let mut root = Map::new();
    root.insert(
        "instances".to_string(),
        Value::Array(vec![Value::Object(instance)]),
    );

    let mut parameters = Map::new();
    if let Some(value) = config.mode {
        parameters.insert("mode".to_string(), serde_json::to_value(value)?);
    }
    if let Some(value) = config.max_predictions {
        parameters.insert(
            "maxPredictions".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = config.confidence_threshold {
        parameters.insert(
            "confidenceThreshold".to_string(),
            Value::Number(Number::from_f64(value as f64).unwrap_or_else(|| Number::from(0))),
        );
    }
    if let Some(value) = config.mask_dilation {
        parameters.insert(
            "maskDilation".to_string(),
            Value::Number(Number::from_f64(value as f64).unwrap_or_else(|| Number::from(0))),
        );
    }
    if let Some(value) = config.binary_color_threshold {
        parameters.insert(
            "binaryColorThreshold".to_string(),
            Value::Number(Number::from_f64(value as f64).unwrap_or_else(|| Number::from(0))),
        );
    }
    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    if let Some(labels) = &config.labels {
        root.insert("labels".to_string(), serde_json::to_value(labels)?);
    }

    Ok(Value::Object(root))
}

fn build_generate_videos_body(
    backend: Backend,
    source: &GenerateVideosSource,
    config: &GenerateVideosConfig,
) -> Result<Value> {
    let mut instance = Map::new();
    if let Some(prompt) = &source.prompt {
        instance.insert("prompt".to_string(), Value::String(prompt.clone()));
    }
    if let Some(image) = &source.image {
        let value = match backend {
            Backend::GeminiApi => image_to_mldev(image)?,
            Backend::VertexAi => image_to_vertex(image)?,
        };
        instance.insert("image".to_string(), value);
    }
    if let Some(video) = &source.video {
        let value = match backend {
            Backend::GeminiApi => video_to_mldev(video)?,
            Backend::VertexAi => video_to_vertex(video)?,
        };
        instance.insert("video".to_string(), value);
    }

    if let Some(last_frame) = &config.last_frame {
        let value = match backend {
            Backend::GeminiApi => image_to_mldev(last_frame)?,
            Backend::VertexAi => image_to_vertex(last_frame)?,
        };
        instance.insert("lastFrame".to_string(), value);
    }

    if let Some(reference_images) = &config.reference_images {
        let mut refs = Vec::new();
        for item in reference_images {
            refs.push(video_reference_image_to_value(backend, item)?);
        }
        instance.insert("referenceImages".to_string(), Value::Array(refs));
    }

    if let Some(mask) = &config.mask {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "mask is not supported in Gemini API".into(),
            });
        }
        instance.insert("mask".to_string(), video_mask_to_vertex(mask)?);
    }

    let mut root = Map::new();
    root.insert(
        "instances".to_string(),
        Value::Array(vec![Value::Object(instance)]),
    );

    let mut parameters = Map::new();

    if let Some(value) = config.number_of_videos {
        parameters.insert(
            "sampleCount".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = &config.output_gcs_uri {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "output_gcs_uri is not supported in Gemini API".into(),
            });
        }
        parameters.insert("storageUri".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.fps {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "fps is not supported in Gemini API".into(),
            });
        }
        parameters.insert("fps".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = config.duration_seconds {
        parameters.insert(
            "durationSeconds".to_string(),
            Value::Number(Number::from(value)),
        );
    }
    if let Some(value) = config.seed {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "seed is not supported in Gemini API".into(),
            });
        }
        parameters.insert("seed".to_string(), Value::Number(Number::from(value)));
    }
    if let Some(value) = &config.aspect_ratio {
        parameters.insert("aspectRatio".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.resolution {
        parameters.insert("resolution".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.person_generation {
        parameters.insert("personGeneration".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.pubsub_topic {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "pubsub_topic is not supported in Gemini API".into(),
            });
        }
        parameters.insert("pubsubTopic".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &config.negative_prompt {
        parameters.insert("negativePrompt".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = config.enhance_prompt {
        parameters.insert("enhancePrompt".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.generate_audio {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "generate_audio is not supported in Gemini API".into(),
            });
        }
        parameters.insert("generateAudio".to_string(), Value::Bool(value));
    }
    if let Some(value) = config.compression_quality {
        if backend == Backend::GeminiApi {
            return Err(Error::InvalidConfig {
                message: "compression_quality is not supported in Gemini API".into(),
            });
        }
        parameters.insert(
            "compressionQuality".to_string(),
            serde_json::to_value(value)?,
        );
    }

    if !parameters.is_empty() {
        root.insert("parameters".to_string(), Value::Object(parameters));
    }

    Ok(Value::Object(root))
}

fn parse_generate_images_response(
    value: Value,
    backend: Backend,
) -> Result<GenerateImagesResponse> {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_images = Vec::new();
    for item in predictions {
        generated_images.push(parse_generated_image(&item, backend)?);
    }

    let positive_prompt_safety_attributes = value
        .get("positivePromptSafetyAttributes")
        .and_then(parse_safety_attributes);

    Ok(GenerateImagesResponse {
        generated_images,
        positive_prompt_safety_attributes,
    })
}

fn parse_edit_image_response(value: Value) -> Result<EditImageResponse> {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_images = Vec::new();
    for item in predictions {
        generated_images.push(parse_generated_image(&item, Backend::VertexAi)?);
    }

    Ok(EditImageResponse { generated_images })
}

fn parse_upscale_image_response(
    value: Value,
) -> Result<rust_genai_types::models::UpscaleImageResponse> {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_images = Vec::new();
    for item in predictions {
        generated_images.push(parse_generated_image(&item, Backend::VertexAi)?);
    }

    Ok(rust_genai_types::models::UpscaleImageResponse { generated_images })
}

fn parse_recontext_image_response(value: Value) -> Result<RecontextImageResponse> {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_images = Vec::new();
    for item in predictions {
        generated_images.push(parse_generated_image(&item, Backend::VertexAi)?);
    }

    Ok(RecontextImageResponse { generated_images })
}

fn parse_segment_image_response(value: Value) -> Result<SegmentImageResponse> {
    let predictions = value
        .get("predictions")
        .and_then(|pred| pred.as_array())
        .cloned()
        .unwrap_or_default();

    let mut generated_masks = Vec::new();
    for item in predictions {
        generated_masks.push(parse_generated_image_mask(&item)?);
    }

    Ok(SegmentImageResponse { generated_masks })
}

fn parse_generate_videos_operation(
    value: Value,
    backend: Backend,
) -> Result<rust_genai_types::operations::Operation> {
    let mut operation: rust_genai_types::operations::Operation = serde_json::from_value(value)?;
    if backend == Backend::GeminiApi {
        if let Some(response) = operation.response.take() {
            if let Some(inner) = response.get("generateVideoResponse") {
                operation.response = Some(inner.clone());
            } else {
                operation.response = Some(response);
            }
        }
    }
    Ok(operation)
}

fn parse_generated_image(value: &Value, backend: Backend) -> Result<GeneratedImage> {
    let image = match backend {
        Backend::GeminiApi => serde_json::from_value::<Image>(value.clone()).ok(),
        Backend::VertexAi => serde_json::from_value::<Image>(value.clone()).ok(),
    };

    let rai_filtered_reason = value
        .get("raiFilteredReason")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());
    let enhanced_prompt = value
        .get("enhancedPrompt")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    let safety_attributes = parse_safety_attributes(value);

    Ok(GeneratedImage {
        image,
        rai_filtered_reason,
        safety_attributes,
        enhanced_prompt,
    })
}

fn parse_generated_image_mask(value: &Value) -> Result<GeneratedImageMask> {
    let mask = serde_json::from_value::<Image>(value.clone()).ok();
    let labels = value
        .get("labels")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(parse_entity_label)
                .collect::<Vec<EntityLabel>>()
        });

    Ok(GeneratedImageMask { mask, labels })
}

fn parse_entity_label(value: &Value) -> Option<EntityLabel> {
    let obj = value.as_object()?;
    let label = obj
        .get("label")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    let score = obj.get("score").and_then(|value| match value {
        Value::Number(num) => num.as_f64().map(|num| num as f32),
        Value::String(text) => text.parse::<f32>().ok(),
        _ => None,
    });

    Some(EntityLabel { label, score })
}

fn parse_safety_attributes(value: &Value) -> Option<SafetyAttributes> {
    let obj = value.as_object()?;
    let safety = obj.get("safetyAttributes").and_then(|v| v.as_object());

    let categories = obj
        .get("categories")
        .or_else(|| safety.and_then(|s| s.get("categories")))
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        });

    let scores = obj
        .get("scores")
        .or_else(|| safety.and_then(|s| s.get("scores")))
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_f64().map(|score| score as f32))
                .collect::<Vec<_>>()
        });

    let content_type = obj
        .get("contentType")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    if categories.is_none() && scores.is_none() && content_type.is_none() {
        None
    } else {
        Some(SafetyAttributes {
            categories,
            scores,
            content_type,
        })
    }
}

fn image_to_mldev(image: &Image) -> Result<Value> {
    if image.gcs_uri.is_some() {
        return Err(Error::InvalidConfig {
            message: "gcs_uri is not supported in Gemini API".into(),
        });
    }
    let mut map = Map::new();
    if let Some(bytes) = &image.image_bytes {
        map.insert(
            "bytesBase64Encoded".to_string(),
            Value::String(STANDARD.encode(bytes)),
        );
    }
    if let Some(mime) = &image.mime_type {
        map.insert("mimeType".to_string(), Value::String(mime.clone()));
    }
    Ok(Value::Object(map))
}

fn image_to_vertex(image: &Image) -> Result<Value> {
    let mut map = Map::new();
    if let Some(gcs_uri) = &image.gcs_uri {
        map.insert("gcsUri".to_string(), Value::String(gcs_uri.clone()));
    }
    if let Some(bytes) = &image.image_bytes {
        map.insert(
            "bytesBase64Encoded".to_string(),
            Value::String(STANDARD.encode(bytes)),
        );
    }
    if let Some(mime) = &image.mime_type {
        map.insert("mimeType".to_string(), Value::String(mime.clone()));
    }
    Ok(Value::Object(map))
}

fn video_to_mldev(video: &Video) -> Result<Value> {
    if let Some(uri) = &video.uri {
        let mut map = Map::new();
        map.insert("uri".to_string(), Value::String(uri.clone()));
        if let Some(bytes) = &video.video_bytes {
            map.insert(
                "encodedVideo".to_string(),
                Value::String(STANDARD.encode(bytes)),
            );
        }
        if let Some(mime) = &video.mime_type {
            map.insert("encoding".to_string(), Value::String(mime.clone()));
        }
        return Ok(Value::Object(map));
    }

    let mut map = Map::new();
    if let Some(bytes) = &video.video_bytes {
        map.insert(
            "encodedVideo".to_string(),
            Value::String(STANDARD.encode(bytes)),
        );
    }
    if let Some(mime) = &video.mime_type {
        map.insert("encoding".to_string(), Value::String(mime.clone()));
    }
    Ok(Value::Object(map))
}

fn video_to_vertex(video: &Video) -> Result<Value> {
    let mut map = Map::new();
    if let Some(uri) = &video.uri {
        map.insert("gcsUri".to_string(), Value::String(uri.clone()));
    }
    if let Some(bytes) = &video.video_bytes {
        map.insert(
            "bytesBase64Encoded".to_string(),
            Value::String(STANDARD.encode(bytes)),
        );
    }
    if let Some(mime) = &video.mime_type {
        map.insert("mimeType".to_string(), Value::String(mime.clone()));
    }
    Ok(Value::Object(map))
}

fn reference_image_to_vertex(image: &ReferenceImage) -> Result<Value> {
    let mut map = Map::new();
    if let Some(reference_image) = &image.reference_image {
        map.insert(
            "referenceImage".to_string(),
            image_to_vertex(reference_image)?,
        );
    }
    if let Some(reference_id) = image.reference_id {
        map.insert(
            "referenceId".to_string(),
            Value::Number(Number::from(reference_id)),
        );
    }
    if let Some(reference_type) = image.reference_type {
        map.insert(
            "referenceType".to_string(),
            serde_json::to_value(reference_type)?,
        );
    }
    if let Some(config) = &image.mask_image_config {
        map.insert("maskImageConfig".to_string(), serde_json::to_value(config)?);
    }
    if let Some(config) = &image.control_image_config {
        map.insert(
            "controlImageConfig".to_string(),
            serde_json::to_value(config)?,
        );
    }
    if let Some(config) = &image.style_image_config {
        map.insert(
            "styleImageConfig".to_string(),
            serde_json::to_value(config)?,
        );
    }
    if let Some(config) = &image.subject_image_config {
        map.insert(
            "subjectImageConfig".to_string(),
            serde_json::to_value(config)?,
        );
    }
    Ok(Value::Object(map))
}

fn video_reference_image_to_value(
    backend: Backend,
    reference: &VideoGenerationReferenceImage,
) -> Result<Value> {
    let mut map = Map::new();
    if let Some(image) = &reference.image {
        let value = match backend {
            Backend::GeminiApi => image_to_mldev(image)?,
            Backend::VertexAi => image_to_vertex(image)?,
        };
        map.insert("image".to_string(), value);
    }
    if let Some(reference_type) = reference.reference_type {
        map.insert(
            "referenceType".to_string(),
            serde_json::to_value(reference_type)?,
        );
    }
    Ok(Value::Object(map))
}

fn video_mask_to_vertex(mask: &VideoGenerationMask) -> Result<Value> {
    let mut map = Map::new();
    if let Some(image) = &mask.image {
        map.insert("image".to_string(), image_to_vertex(image)?);
    }
    if let Some(mode) = mask.mask_mode {
        map.insert("maskMode".to_string(), serde_json::to_value(mode)?);
    }
    Ok(Value::Object(map))
}

fn apply_http_options(
    mut request: reqwest::RequestBuilder,
    http_options: Option<&rust_genai_types::http::HttpOptions>,
) -> Result<reqwest::RequestBuilder> {
    if let Some(options) = http_options {
        if let Some(timeout) = options.timeout {
            request = request.timeout(std::time::Duration::from_millis(timeout));
        }
        if let Some(headers) = &options.headers {
            for (key, value) in headers {
                let name =
                    reqwest::header::HeaderName::from_bytes(key.as_bytes()).map_err(|_| {
                        Error::InvalidConfig {
                            message: format!("Invalid header name: {key}"),
                        }
                    })?;
                let value = reqwest::header::HeaderValue::from_str(value).map_err(|_| {
                    Error::InvalidConfig {
                        message: format!("Invalid header value for {key}"),
                    }
                })?;
                request = request.header(name, value);
            }
        }
    }
    Ok(request)
}

fn build_function_call_content(function_calls: &[FunctionCall]) -> Content {
    let parts = function_calls
        .iter()
        .cloned()
        .map(Part::function_call)
        .collect();
    Content::from_parts(parts, Role::Model)
}

fn merge_extra_body(
    body: &mut Value,
    http_options: &rust_genai_types::http::HttpOptions,
) -> Result<()> {
    if let Some(extra) = &http_options.extra_body {
        match (body, extra) {
            (Value::Object(body_map), Value::Object(extra_map)) => {
                for (key, value) in extra_map {
                    body_map.insert(key.clone(), value.clone());
                }
            }
            (_, _) => {
                return Err(Error::InvalidConfig {
                    message: "HttpOptions.extra_body must be an object".into(),
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::Client;

    #[test]
    fn test_transform_model_name() {
        assert_eq!(
            transform_model_name(Backend::GeminiApi, "gemini-1.5-pro"),
            "models/gemini-1.5-pro"
        );
        assert_eq!(
            transform_model_name(Backend::VertexAi, "gemini-1.5-pro"),
            "publishers/google/models/gemini-1.5-pro"
        );
    }

    #[test]
    fn test_build_model_urls() {
        let client = Client::new("test-key").unwrap();
        let models = client.models();
        let url =
            build_model_method_url(&models.inner, "gemini-1.5-pro", "generateContent").unwrap();
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent"
        );
    }

    #[test]
    fn test_build_recontext_image_body() {
        let source = RecontextImageSource {
            prompt: Some("test".to_string()),
            person_image: Some(Image {
                gcs_uri: Some("gs://person.png".to_string()),
                ..Default::default()
            }),
            product_images: Some(vec![rust_genai_types::models::ProductImage {
                product_image: Some(Image {
                    gcs_uri: Some("gs://product.png".to_string()),
                    ..Default::default()
                }),
            }]),
        };
        let config = RecontextImageConfig {
            number_of_images: Some(2),
            ..Default::default()
        };

        let body = build_recontext_image_body(&source, &config).unwrap();
        let instances = body.get("instances").and_then(|v| v.as_array()).unwrap();
        let instance = instances[0].as_object().unwrap();
        assert!(instance.get("prompt").is_some());
        assert!(instance.get("personImage").is_some());
        assert!(instance.get("productImages").is_some());
    }

    #[test]
    fn test_build_segment_image_body() {
        let source = SegmentImageSource {
            prompt: Some("foreground".to_string()),
            image: Some(Image {
                gcs_uri: Some("gs://input.png".to_string()),
                ..Default::default()
            }),
            scribble_image: None,
        };
        let config = SegmentImageConfig {
            mode: Some(rust_genai_types::enums::SegmentMode::Foreground),
            ..Default::default()
        };

        let body = build_segment_image_body(&source, &config).unwrap();
        let instances = body.get("instances").and_then(|v| v.as_array()).unwrap();
        let instance = instances[0].as_object().unwrap();
        assert!(instance.get("image").is_some());
        assert!(body.get("parameters").is_some());
    }
}
