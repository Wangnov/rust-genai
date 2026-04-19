# Examples

运行示例前请设置 `GEMINI_API_KEY`（或 `GOOGLE_API_KEY`）：

```bash
export GEMINI_API_KEY="YOUR_API_KEY"
```

执行示例（任选一种方式）：

```bash
# 在工作区根目录执行
cargo run -p rust-genai --example generate_content_basic

# 或进入 rust-genai crate 目录执行
cd rust-genai
cargo run --example generate_content_basic
```

低成本端到端 smoke probe：

```bash
cargo run -p rust-genai --example live_smoke

# 加上已知边界探针
GENAI_SMOKE_INCLUDE_EDGE_PROBES=1 cargo run -p rust-genai --example live_smoke
```

示例输入文件位于 `examples/files/input`，示例输出默认保存到 `examples/files/output`，
可通过环境变量 `GENAI_EXAMPLE_FILES_DIR` 覆盖输出目录。

## 示例列表

- generate_content_basic
- generate_content_stream
- list_models
- live_smoke
- embed_content
- count_tokens
- compute_tokens
- function_calling_auto
- mcp_basic（需要 `--features mcp`）
- generate_images
- generate_content_image
- recontext_image（仅 Vertex AI）
- segment_image（仅 Vertex AI）
- generate_videos
- chat_basic
- chat_stream
- chat_history
- files_upload_from_path
- files_list
- files_download
- caches_create
- caches_list
- batches_create
- batches_list
- operations_wait
- file_search_store_create
- file_search_store_upload
- documents_list
- auth_tokens_create
- live_session_basic
- live_native_audio
- live_music_generation
- interactions_basic
- deep_research_basic
- computer_use_tools
- function_response_media
- code_execution_with_image
- error_handling
- timeout_and_proxy
- grounding_citations
- tts_multispeaker
