use rust_genai::types::content::{Content, FunctionResponse, Part, Role};
use rust_genai::types::enums::Modality;
use rust_genai::types::live_types::{
    AudioTranscriptionConfig, LiveConnectConfig, LiveSendClientContentParameters,
    LiveSendToolResponseParameters,
};
use rust_genai::{Client, Result};
use serde_json::json;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?;
    let model = "gemini-2.5-flash-native-audio-preview-12-2025";

    // 定义天气查询函数
    let get_weather = json!({
        "name": "get_weather",
        "description": "Get the current weather for a city",
        "parameters": {
            "type": "OBJECT",
            "properties": {
                "city": {
                    "type": "STRING",
                    "description": "The city name"
                }
            },
            "required": ["city"]
        }
    });

    // 注意：必须使用 camelCase - functionDeclarations 而不是 function_declarations
    let tools = json!([{
        "functionDeclarations": [get_weather]
    }]);

    // 使用 native audio 模型配置
    let config = LiveConnectConfig {
        response_modalities: Some(vec![Modality::Audio]),
        output_audio_transcription: Some(AudioTranscriptionConfig::default()),
        tools: Some(serde_json::from_value(tools)?),
        ..Default::default()
    };

    println!("连接 Live API 中... (model={})", model);
    let mut session = client.live().connect(model, config).await?;

    println!("连接成功。发送请求...");

    //发送天气查询请求
    let prompt = "What's the weather like in Beijing?";
    session
        .send_client_content(LiveSendClientContentParameters {
            turns: Some(vec![Content {
                role: Some(Role::User),
                parts: vec![Part::text(prompt)],
            }]),
            turn_complete: Some(true),
        })
        .await?;

    println!("请求已发送，等待响应...");

    let mut text_started = false;
    let mut last_char: Option<char> = None;
    let audio_out_path = std::env::var("GENAI_AUDIO_OUT_PATH").ok();
    let mut wav_writer: Option<WavWriter> = None;
    let deadline = std::time::Duration::from_secs(30);

    loop {
        let receive_result = tokio::time::timeout(deadline, session.receive()).await;

        let message = match receive_result {
            Ok(Some(msg)) => {
                let msg = msg?;

                // 只在有 tool_call 时打印提示
                if msg.tool_call.is_some() {
                    println!(); // 换行让输出更清晰
                }

                msg
            }
            Ok(None) => {
                println!("\n\n会话结束。");
                break;
            }
            Err(_) => {
                println!("\n等待响应超时。");
                break;
            }
        };

        // 处理 function call
        if let Some(tool_call) = message.tool_call.as_ref() {
            let responses = tool_call
                .function_calls
                .as_ref()
                .unwrap_or(&vec![])
                .iter()
                .map(|call| {
                    let func_name = call.name.as_deref().unwrap_or("unknown");
                    let args_str = serde_json::to_string(&call.args).unwrap_or_default();
                    println!("[function_call] {}({})", func_name, args_str);
                    FunctionResponse {
                        will_continue: None,
                        scheduling: None,
                        parts: None,
                        id: call.id.clone(),
                        name: call.name.clone(),
                        response: Some(json!({"temperature": "20C", "condition": "sunny"})),
                    }
                })
                .collect::<Vec<_>>();

            session
                .send_tool_response(LiveSendToolResponseParameters {
                    function_responses: Some(responses),
                })
                .await?;
        }

        // 显示输出转写
        if let Some(transcription) = message
            .server_content
            .as_ref()
            .and_then(|c| c.output_transcription.as_ref())
        {
            if let Some(text) = transcription.text.as_deref() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if !text_started {
                        print!("assistant: ");
                        text_started = true;
                    } else if let Some(first_char) = trimmed.chars().next() {
                        if text.starts_with(char::is_whitespace)
                            && needs_space_before(last_char, first_char)
                        {
                            print!(" ");
                        }
                    }
                    print!("{}", trimmed);
                    std::io::stdout().flush().ok();
                    last_char = trimmed.chars().last();
                }
            }
        }

        // 处理模型响应 - 流式输出文本和音频
        if let Some(content) = message
            .server_content
            .as_ref()
            .and_then(|c| c.model_turn.as_ref())
        {
            for part in &content.parts {
                if part.thought.unwrap_or(false) {
                    continue;
                }
                match &part.kind {
                    rust_genai::types::content::PartKind::Text { text } => {
                        // 如果已经有转写输出，就不显示文本了（避免重复）
                        if message
                            .server_content
                            .as_ref()
                            .and_then(|c| c.output_transcription.as_ref())
                            .is_none()
                        {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                if !text_started {
                                    print!("assistant: ");
                                    text_started = true;
                                } else if let Some(first_char) = trimmed.chars().next() {
                                    if text.starts_with(char::is_whitespace)
                                        && needs_space_before(last_char, first_char)
                                    {
                                        print!(" ");
                                    }
                                }
                                print!("{}", trimmed);
                                std::io::stdout().flush().ok();
                                last_char = trimmed.chars().last();
                            }
                        }
                    }
                    rust_genai::types::content::PartKind::InlineData { inline_data } => {
                        // 保存音频（使用 WavWriter）
                        if inline_data.mime_type.starts_with("audio/") {
                            if let Some(path) = audio_out_path.as_ref() {
                                let rate =
                                    parse_sample_rate(&inline_data.mime_type).unwrap_or(24_000);
                                if wav_writer.is_none() {
                                    let writer = WavWriter::create(path, rate)?;
                                    wav_writer = Some(writer);
                                }
                                if let Some(writer) = wav_writer.as_mut() {
                                    writer.write_chunk(&inline_data.data)?;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // 检查是否完成
        if message
            .server_content
            .as_ref()
            .and_then(|c| c.turn_complete)
            .unwrap_or(false)
        {
            if text_started {
                println!();
            }
            if let Some(writer) = wav_writer.as_mut() {
                writer.update_header()?;
                if let Some(path) = audio_out_path.as_ref() {
                    println!("[audio] 已保存到 {} (rate={}Hz)", path, writer.sample_rate);
                }
            }
            break;
        }
    }

    session.close().await?;
    Ok(())
}

fn parse_sample_rate(mime_type: &str) -> Option<u32> {
    mime_type
        .split(';')
        .find_map(|part| part.trim().strip_prefix("rate="))
        .and_then(|value| value.parse::<u32>().ok())
}

struct WavWriter {
    file: File,
    data_len: u32,
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
}

impl WavWriter {
    fn create(path: &str, sample_rate: u32) -> rust_genai::Result<Self> {
        let file = File::create(path)?;
        let mut writer = Self {
            file,
            data_len: 0,
            sample_rate,
            channels: 1,
            bits_per_sample: 16,
        };
        writer.write_header()?;
        Ok(writer)
    }

    fn write_chunk(&mut self, data: &[u8]) -> rust_genai::Result<()> {
        self.file.write_all(data)?;
        self.data_len = self.data_len.saturating_add(data.len() as u32);
        Ok(())
    }

    fn write_header(&mut self) -> rust_genai::Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        let byte_rate =
            self.sample_rate * u32::from(self.channels) * u32::from(self.bits_per_sample) / 8;
        let block_align = self.channels * (self.bits_per_sample / 8);
        let chunk_size = 36u32.saturating_add(self.data_len);

        self.file.write_all(b"RIFF")?;
        self.file.write_all(&chunk_size.to_le_bytes())?;
        self.file.write_all(b"WAVE")?;
        self.file.write_all(b"fmt ")?;
        self.file.write_all(&16u32.to_le_bytes())?;
        self.file.write_all(&1u16.to_le_bytes())?;
        self.file.write_all(&self.channels.to_le_bytes())?;
        self.file.write_all(&self.sample_rate.to_le_bytes())?;
        self.file.write_all(&byte_rate.to_le_bytes())?;
        self.file.write_all(&block_align.to_le_bytes())?;
        self.file.write_all(&self.bits_per_sample.to_le_bytes())?;
        self.file.write_all(b"data")?;
        self.file.write_all(&self.data_len.to_le_bytes())?;

        self.file.seek(SeekFrom::End(0))?;
        Ok(())
    }

    fn update_header(&mut self) -> rust_genai::Result<()> {
        self.write_header()
    }
}

fn needs_space_before(last: Option<char>, current_first: char) -> bool {
    let Some(last_char) = last else {
        return false;
    };

    // CJK 字符判断
    let is_cjk = |c: char| -> bool {
        matches!(c,
            '\u{4E00}'..='\u{9FFF}' |  // CJK 统一表意文字
            '\u{3400}'..='\u{4DBF}' |  // CJK 扩展 A
            '\u{20000}'..='\u{2A6DF}' | // CJK 扩展 B
            '\u{2A700}'..='\u{2B73F}' | // CJK 扩展 C
            '\u{2B740}'..='\u{2B81F}' | // CJK 扩展 D
            '\u{2B820}'..='\u{2CEAF}' | // CJK 扩展 E
            '\u{3000}'..='\u{303F}' |  // CJK 符号和标点
            '\u{FF00}'..='\u{FFEF}' |  // 全角 ASCII
            '\u{3040}'..='\u{309F}' |  // 平假名
            '\u{30A0}'..='\u{30FF}'    // 片假名
        )
    };

    // 常见标点符号
    let is_punctuation = |c: char| -> bool {
        matches!(
            c,
            '.' | ','
                | '!'
                | '?'
                | ';'
                | ':'
                | ')'
                | ']'
                | '}'
                | '\''
                | '。'
                | '，'
                | '！'
                | '？'
                | '；'
                | '：'
                | '）'
                | '】'
                | '』'
                | '"'
                | '\u{2019}'
        )
    };

    // 如果当前字符是 CJK，检查上一个字符
    if is_cjk(current_first) {
        // 如果上一个也是 CJK，不需要空格
        if is_cjk(last_char) {
            return false;
        }
        // 如果上一个是字母或数字，需要空格
        if last_char.is_alphanumeric() {
            return true;
        }
    }

    // 如果当前字符是标点，通常不需要前面的空格
    if is_punctuation(current_first) {
        return false;
    }

    // 如果上一个字符是标点，当前是字母数字，需要空格
    if is_punctuation(last_char) && current_first.is_alphanumeric() {
        return true;
    }

    // 如果当前是字母数字，上一个也是字母数字，需要空格
    if current_first.is_alphanumeric() && last_char.is_alphanumeric() {
        return true;
    }

    // 其他情况不需要空格
    false
}
