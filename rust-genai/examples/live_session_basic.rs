use rust_genai::types::content::Blob;
use rust_genai::types::enums::Modality;
use rust_genai::types::live_types::{
    AudioTranscriptionConfig, LiveConnectConfig, LiveSendRealtimeInputParameters,
};
use rust_genai::Client;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use tokio::io::{self, AsyncBufReadExt};

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("运行失败: {err}");
        eprintln!(
            "排查建议:\n- Live API 走 WebSocket，若你的网络/防火墙阻断 wss 连接会导致超时。\n- 若使用 native-audio 模型，请提供音频输入（设置 GENAI_AUDIO_PATH）或改用支持文本的 Live 模型。\n- 如需切换模型，可设置 GENAI_LIVE_MODEL。\n- 如果 REST 示例可用但 Live 失败，优先排查网络层或模型与输入类型是否匹配。"
        );
        std::process::exit(1);
    }
}

async fn run() -> rust_genai::Result<()> {
    let client = Client::from_env()?;
    let model = std::env::var("GENAI_LIVE_MODEL")
        .unwrap_or_else(|_| "gemini-2.5-flash-native-audio-preview-12-2025".to_string());
    let response_timeout_secs: u64 = std::env::var("GENAI_LIVE_RESPONSE_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(20);
    let native_audio = model.contains("native-audio");
    let config = if native_audio {
        LiveConnectConfig {
            response_modalities: Some(vec![Modality::Audio]),
            output_audio_transcription: Some(AudioTranscriptionConfig::default()),
            ..Default::default()
        }
    } else {
        LiveConnectConfig {
            response_modalities: Some(vec![Modality::Text]),
            ..Default::default()
        }
    };
    println!("连接 Live API 中... (model={model})");
    let mut session = client.live().connect(model, config).await?;

    println!("连接成功。示例将先发送一句话，然后进入交互模式。");
    send_text_or_audio(&mut session, native_audio, "你好，Live API").await?;
    receive_turn(&mut session, response_timeout_secs, native_audio).await?;

    println!("进入交互模式：输入内容回车发送，输入 /exit 退出。");
    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();
    loop {
        print!("> ");
        std::io::stdout().flush().ok();
        let line = lines.next_line().await?;
        let Some(line) = line else { break };
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "/exit" {
            break;
        }
        send_text_or_audio(&mut session, native_audio, input).await?;
        receive_turn(&mut session, response_timeout_secs, native_audio).await?;
    }

    session.close().await?;
    Ok(())
}

async fn send_text_or_audio(
    session: &mut rust_genai::live::LiveSession,
    native_audio: bool,
    text: &str,
) -> rust_genai::Result<()> {
    if native_audio {
        if let Ok(path) = std::env::var("GENAI_AUDIO_PATH") {
            if !path.trim().is_empty() {
                let data = fs::read(path)?;
                let mime_type = std::env::var("GENAI_AUDIO_MIME")
                    .unwrap_or_else(|_| "audio/pcm;rate=16000".to_string());
                return session
                    .send_realtime_input(LiveSendRealtimeInputParameters {
                        media: None,
                        audio: Some(Blob {
                            mime_type,
                            data,
                            display_name: None,
                        }),
                        audio_stream_end: None,
                        video: None,
                        text: None,
                        activity_start: None,
                        activity_end: None,
                    })
                    .await;
            }
        }
        return session
            .send_realtime_input(LiveSendRealtimeInputParameters {
                media: None,
                audio: None,
                audio_stream_end: None,
                video: None,
                text: Some(text.to_string()),
                activity_start: None,
                activity_end: None,
            })
            .await;
    }
    session.send_text(text).await
}

async fn receive_turn(
    session: &mut rust_genai::live::LiveSession,
    timeout_secs: u64,
    native_audio: bool,
) -> rust_genai::Result<()> {
    let audio_out_path = std::env::var("GENAI_AUDIO_OUT_PATH").ok();
    let audio_verbose = env_flag("GENAI_AUDIO_VERBOSE");
    let mut wav_writer: Option<WavWriter> = None;
    let mut audio_saved_path: Option<(String, u32)> = None;
    let mut text_started = false;
    let mut last_char: Option<char> = None;
    let deadline = std::time::Duration::from_secs(timeout_secs);
    loop {
        let response = tokio::time::timeout(deadline, session.receive())
            .await
            .map_err(|_| rust_genai::Error::Timeout {
                message: format!(
                    "Timed out waiting for Live response ({timeout_secs}s). Try setting GENAI_LIVE_MODEL or check your network."
                ),
            })?;
        let Some(message) = response else { break };
        let message = message?;
        let server_content = message.server_content.as_ref();
        if let Some(transcription) = server_content.and_then(|c| c.output_transcription.as_ref()) {
            if let Some(text) = transcription.text.as_deref() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    if !text_started {
                        print!("assistant: ");
                        text_started = true;
                    } else if let Some(first_char) = trimmed.chars().next() {
                        // 智能判断是否需要空格
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
        if let Some(content) = server_content.and_then(|c| c.model_turn.as_ref()) {
            for part in &content.parts {
                if part.thought.unwrap_or(false) {
                    continue;
                }
                match &part.kind {
                    rust_genai::types::content::PartKind::Text { text } => {
                        if !native_audio {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                if !text_started {
                                    print!("assistant: ");
                                    text_started = true;
                                } else if let Some(first_char) = trimmed.chars().next() {
                                    // 智能判断是否需要空格
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
                        } else if audio_verbose {
                            println!("assistant: {}", text);
                        }
                    }
                    rust_genai::types::content::PartKind::InlineData { inline_data } => {
                        if inline_data.mime_type.starts_with("audio/") {
                            if let Some(path) = audio_out_path.as_ref() {
                                let rate =
                                    parse_sample_rate(&inline_data.mime_type).unwrap_or(24_000);
                                if wav_writer.is_none() {
                                    let writer = WavWriter::create(path, rate)?;
                                    wav_writer = Some(writer);
                                }
                                if let Some(writer) = wav_writer.as_mut() {
                                    if audio_saved_path.is_none() {
                                        audio_saved_path = Some((path.clone(), writer.sample_rate));
                                    }
                                    writer.write_chunk(&inline_data.data)?;
                                }
                            }
                            if audio_verbose {
                                println!(
                                    "assistant: [audio chunk] mime={} bytes={}",
                                    inline_data.mime_type,
                                    inline_data.data.len()
                                );
                            }
                        } else if audio_verbose {
                            println!(
                                "assistant: [inline data] mime={} bytes={}",
                                inline_data.mime_type,
                                inline_data.data.len()
                            );
                        }
                    }
                    rust_genai::types::content::PartKind::FileData { file_data } => {
                        if audio_verbose {
                            println!(
                                "assistant: [file] uri={} mime={}",
                                file_data.file_uri, file_data.mime_type
                            );
                        }
                    }
                    rust_genai::types::content::PartKind::FunctionCall { function_call } => {
                        if audio_verbose {
                            println!("assistant: [function_call] {:?}", function_call);
                        }
                    }
                    rust_genai::types::content::PartKind::FunctionResponse {
                        function_response,
                    } => {
                        if audio_verbose {
                            println!("assistant: [function_response] {:?}", function_response);
                        }
                    }
                    rust_genai::types::content::PartKind::ExecutableCode { executable_code } => {
                        if audio_verbose {
                            println!("assistant: [code] {:?}", executable_code);
                        }
                    }
                    rust_genai::types::content::PartKind::CodeExecutionResult {
                        code_execution_result,
                    } => {
                        if audio_verbose {
                            println!("assistant: [code_result] {:?}", code_execution_result);
                        }
                    }
                }
            }
        }
        if server_content
            .and_then(|c| c.turn_complete)
            .unwrap_or(false)
        {
            if text_started {
                println!();
            }
            if let Some((path, rate)) = audio_saved_path.as_ref() {
                println!("[audio] 已保存到 {} (rate={}Hz)", path, rate);
            }
            if let Some(writer) = wav_writer.as_mut() {
                writer.update_header()?;
            }
            break;
        }
    }
    Ok(())
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

    // 如果当前字符是 CJK 或标点，检查上一个字符
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

    // 如果上一个字符是标点，当前是字母数字，需要空格（例如：", trained"）
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

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            let value = value.trim().to_lowercase();
            value == "1" || value == "true" || value == "yes"
        })
        .unwrap_or(false)
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
