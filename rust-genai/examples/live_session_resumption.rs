use rust_genai::types::enums::Modality;
use rust_genai::types::live_types::{
    AudioTranscriptionConfig, LiveConnectConfig, LiveSendRealtimeInputParameters,
};
use rust_genai::{Client, Result};
use std::io::Write;

/// 演示 Live API 的 Session Resumption 功能。
/// 注意：Session resumption 功能可能在某些 API 版本或模型上还未完全启用。
/// 如果 resumable 字段返回 None，说明当前不支持恢复功能。
#[tokio::main]
async fn main() -> Result<()> {
    run().await
}

async fn run() -> Result<()> {
    let client = Client::from_env()?;
    let model = std::env::var("GENAI_LIVE_MODEL")
        .unwrap_or_else(|_| "gemini-2.5-flash-native-audio-preview-12-2025".to_string());

    let native_audio = model.contains("native-audio");

    let config = build_live_config(native_audio);

    println!("连接 Live API 中... (model={model})");
    let mut session = client
        .live()
        .builder(model.clone())
        .with_config(config.clone())
        .with_session_resumption()
        .connect()
        .await?;

    println!("连接成功。发送多条消息以获取 resumption handle...");

    send_text_or_audio(&session, native_audio, "你好，我叫小明。").await?;

    let mut handle = None;
    let mut state = TurnState::new();

    println!("等待响应并收集 resumption handle...");

    receive_until_turn_complete(&mut session, native_audio, &mut state, &mut handle).await?;

    // 如果还没获取到 handle，再发送一条消息
    if handle.is_none() {
        println!("\n发送第二条消息...");
        send_text_or_audio(&session, native_audio, "今天天气怎么样？").await?;
        receive_until_turn_complete(&mut session, native_audio, &mut state, &mut handle).await?;
    }

    if let Some(handle) = handle {
        println!("\n=== 测试会话恢复 ===");
        session.close().await?;

        println!("重新连接会话...");
        let mut resumed = client
            .live()
            .builder(model.clone())
            .with_config(config)
            .with_session_resumption_handle(handle)
            .connect()
            .await?;

        println!("重连成功。继续对话...");

        // 发送恢复后的消息
        send_text_or_audio(&resumed, native_audio, "我叫什么名字？").await?;

        let mut resume_state = TurnState::new();
        let mut resume_handle = None;
        receive_until_turn_complete(
            &mut resumed,
            native_audio,
            &mut resume_state,
            &mut resume_handle,
        )
        .await?;

        resumed.close().await?;
        println!("\n✓ 会话恢复成功！模型记住了你叫\"小明\"。");
    } else {
        println!("\n未获取到可用的 resumption handle。");
        println!("注意：Session resumption 功能可能在当前模型或 API 版本上还未完全启用。");
        session.close().await?;
    }

    Ok(())
}

fn build_live_config(native_audio: bool) -> LiveConnectConfig {
    if native_audio {
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
    }
}

async fn send_text_or_audio(
    session: &rust_genai::live::LiveSession,
    native_audio: bool,
    text: &str,
) -> Result<()> {
    if native_audio {
        session
            .send_realtime_input(LiveSendRealtimeInputParameters {
                media: None,
                audio: None,
                audio_stream_end: None,
                video: None,
                text: Some(text.to_string()),
                activity_start: None,
                activity_end: None,
            })
            .await
    } else {
        session.send_text(text).await
    }
}

async fn receive_until_turn_complete(
    session: &mut rust_genai::live::LiveSession,
    native_audio: bool,
    state: &mut TurnState,
    handle: &mut Option<String>,
) -> Result<()> {
    loop {
        let message = session.receive().await;
        let Some(message) = message else { break };
        let message = message?;

        update_resumption_handle(&message, handle);
        render_message(&message, native_audio, state);

        if message
            .server_content
            .as_ref()
            .and_then(|content| content.turn_complete)
            .unwrap_or(false)
        {
            if state.text_started {
                println!();
            }
            state.reset();
            break;
        }
    }
    Ok(())
}

fn update_resumption_handle(
    message: &rust_genai::types::live_types::LiveServerMessage,
    handle: &mut Option<String>,
) {
    if let Some(update) = message.session_resumption_update.as_ref() {
        if update.resumable.unwrap_or(false) && update.new_handle.is_some() {
            handle.clone_from(&update.new_handle);
            println!("[收到可用的 resumption handle]");
        }
    }
}

fn render_message(
    message: &rust_genai::types::live_types::LiveServerMessage,
    native_audio: bool,
    state: &mut TurnState,
) {
    if native_audio {
        if let Some(text) = message
            .server_content
            .as_ref()
            .and_then(|content| content.output_transcription.as_ref())
            .and_then(|transcription| transcription.text.as_deref())
        {
            emit_transcription(text, state);
        }
    } else if let Some(text) = message
        .server_content
        .as_ref()
        .and_then(|content| content.model_turn.as_ref())
        .and_then(|content| content.first_text())
    {
        emit_text(text, state);
    }
}

fn emit_transcription(text: &str, state: &mut TurnState) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
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

fn emit_text(text: &str, state: &mut TurnState) {
    if !state.text_started {
        print!("assistant: ");
        state.text_started = true;
    }
    print!("{text}");
    std::io::stdout().flush().ok();
}

struct TurnState {
    text_started: bool,
    last_char: Option<char>,
}

impl TurnState {
    const fn new() -> Self {
        Self {
            text_started: false,
            last_char: None,
        }
    }

    const fn reset(&mut self) {
        self.text_started = false;
        self.last_char = None;
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
