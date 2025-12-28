use rust_genai::live::LiveSession;
use rust_genai::types::content::Blob;
use rust_genai::types::enums::Modality;
use rust_genai::types::live_types::LiveServerMessage;
use rust_genai::types::live_types::{
    AudioTranscriptionConfig, LiveConnectConfig, LiveSendRealtimeInputParameters,
};
use rust_genai::{Client, Result};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn example_input_audio() -> PathBuf {
    if let Ok(path) = std::env::var("GENAI_AUDIO_PATH") {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("files")
        .join("input")
        .join("audio")
        .join("voice_chat_input.wav")
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::from_env()?;
    let model = "gemini-2.5-flash-native-audio-preview-12-2025";

    let config = build_live_config();

    println!("连接 Live API 中... (model={model})");
    let mut session = client.live().connect(model, config).await?;

    println!("连接成功。发送音频文件...");

    let audio_path = example_input_audio();
    println!("使用音频文件: {path}", path = audio_path.display());
    let pcm_data = read_pcm_audio(&audio_path)?;
    let bytes = pcm_data.len();
    println!("发送 PCM 数据 ({bytes} bytes)...");
    send_audio(&session, pcm_data).await?;

    println!("请求已发送，等待响应...");

    let audio_out_path = std::env::var("GENAI_AUDIO_OUT_PATH").ok();
    process_live_responses(&mut session, audio_out_path).await?;

    session.close().await?;
    Ok(())
}

fn build_live_config() -> LiveConnectConfig {
    LiveConnectConfig {
        response_modalities: Some(vec![Modality::Audio]),
        output_audio_transcription: Some(AudioTranscriptionConfig::default()),
        input_audio_transcription: Some(AudioTranscriptionConfig::default()),
        ..Default::default()
    }
}

fn read_pcm_audio(audio_path: &Path) -> Result<Vec<u8>> {
    let audio_data = std::fs::read(audio_path)?;
    if audio_data.len() > 44 && &audio_data[0..4] == b"RIFF" {
        println!("检测到 WAV 格式，跳过文件头");
        Ok(audio_data[44..].to_vec())
    } else {
        Ok(audio_data)
    }
}

async fn send_audio(session: &LiveSession, pcm_data: Vec<u8>) -> Result<()> {
    session
        .send_realtime_input(LiveSendRealtimeInputParameters {
            media: None,
            audio: Some(Blob {
                mime_type: "audio/pcm;rate=24000".to_string(),
                data: pcm_data,
                display_name: None,
            }),
            audio_stream_end: None,
            video: None,
            text: None,
            activity_start: None,
            activity_end: None,
        })
        .await?;

    session
        .send_realtime_input(LiveSendRealtimeInputParameters {
            media: None,
            audio: None,
            audio_stream_end: Some(true),
            video: None,
            text: None,
            activity_start: None,
            activity_end: None,
        })
        .await?;

    Ok(())
}

#[derive(Default)]
struct TranscriptionState {
    user_text_started: bool,
    user_last_char: Option<char>,
    text_started: bool,
    last_char: Option<char>,
}

fn handle_input_transcription(state: &mut TranscriptionState, message: &LiveServerMessage) {
    let Some(transcription) = message
        .server_content
        .as_ref()
        .and_then(|c| c.input_transcription.as_ref())
    else {
        return;
    };
    let Some(text) = transcription.text.as_deref() else {
        return;
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    if !state.user_text_started {
        print!("user: ");
        state.user_text_started = true;
    } else if let Some(first_char) = trimmed.chars().next() {
        if text.starts_with(char::is_whitespace)
            && needs_space_before(state.user_last_char, first_char)
        {
            print!(" ");
        }
    }

    print!("{trimmed}");
    std::io::stdout().flush().ok();
    state.user_last_char = trimmed.chars().last();
}

fn handle_output_transcription(state: &mut TranscriptionState, message: &LiveServerMessage) {
    let Some(transcription) = message
        .server_content
        .as_ref()
        .and_then(|c| c.output_transcription.as_ref())
    else {
        return;
    };
    let Some(text) = transcription.text.as_deref() else {
        return;
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }

    if state.user_text_started && !state.text_started {
        println!();
        state.user_text_started = false;
    }

    if !state.text_started {
        print!("assistant: ");
        state.text_started = true;
    } else if let Some(first_char) = trimmed.chars().next() {
        if text.starts_with(char::is_whitespace) && needs_space_before(state.last_char, first_char)
        {
            print!(" ");
        }
    }

    print!("{trimmed}");
    std::io::stdout().flush().ok();
    state.last_char = trimmed.chars().last();
}

fn handle_audio_output(
    message: &LiveServerMessage,
    audio_out_path: Option<&str>,
    wav_writer: &mut Option<WavWriter>,
) -> Result<()> {
    let Some(path) = audio_out_path else {
        return Ok(());
    };

    let Some(content) = message
        .server_content
        .as_ref()
        .and_then(|c| c.model_turn.as_ref())
    else {
        return Ok(());
    };

    for part in &content.parts {
        if part.thought.unwrap_or(false) {
            continue;
        }
        if let rust_genai::types::content::PartKind::InlineData { inline_data } = &part.kind {
            if inline_data.mime_type.starts_with("audio/") {
                let rate = parse_sample_rate(&inline_data.mime_type).unwrap_or(24_000);
                if wav_writer.is_none() {
                    let writer = WavWriter::create(path, rate)?;
                    *wav_writer = Some(writer);
                }
                if let Some(writer) = wav_writer.as_mut() {
                    writer.write_chunk(&inline_data.data)?;
                }
            }
        }
    }

    Ok(())
}

fn finalize_audio_output(
    wav_writer: &mut Option<WavWriter>,
    audio_out_path: Option<&str>,
) -> Result<()> {
    if let Some(writer) = wav_writer.as_mut() {
        writer.update_header()?;
        if let Some(path) = audio_out_path {
            let rate = writer.sample_rate;
            println!("[audio] 已保存到 {path} (rate={rate}Hz)");
        }
    }
    Ok(())
}

fn is_turn_complete(message: &LiveServerMessage) -> bool {
    message
        .server_content
        .as_ref()
        .and_then(|c| c.turn_complete)
        .is_some_and(|value| value)
}

async fn process_live_responses(
    session: &mut LiveSession,
    audio_out_path: Option<String>,
) -> Result<()> {
    let mut state = TranscriptionState::default();
    let mut wav_writer: Option<WavWriter> = None;
    let deadline = Duration::from_secs(30);

    loop {
        let receive_result = tokio::time::timeout(deadline, session.receive()).await;

        let message = match receive_result {
            Ok(Some(msg)) => msg?,
            Ok(None) => {
                if state.user_text_started {
                    println!();
                }
                println!("\n会话结束。");
                break;
            }
            Err(_) => {
                if state.user_text_started {
                    println!();
                }
                println!("\n等待响应超时。");
                break;
            }
        };

        handle_input_transcription(&mut state, &message);
        handle_output_transcription(&mut state, &message);
        handle_audio_output(&message, audio_out_path.as_deref(), &mut wav_writer)?;

        if is_turn_complete(&message) {
            if state.text_started {
                println!();
            }
            finalize_audio_output(&mut wav_writer, audio_out_path.as_deref())?;
            break;
        }
    }

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
        let chunk_len = u32::try_from(data.len()).unwrap_or(u32::MAX);
        self.data_len = self.data_len.saturating_add(chunk_len);
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
