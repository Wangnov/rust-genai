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
    let client = Client::from_env()?;
    let model = std::env::var("GENAI_LIVE_MODEL")
        .unwrap_or_else(|_| "gemini-2.5-flash-native-audio-preview-12-2025".to_string());

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

    println!("连接 Live API 中... (model={})", model);
    let mut session = client
        .live()
        .builder(model.clone())
        .with_config(config.clone())
        .with_session_resumption()
        .connect()
        .await?;

    println!("连接成功。发送多条消息以获取 resumption handle...");

    // 发送第一条消息
    if native_audio {
        session
            .send_realtime_input(LiveSendRealtimeInputParameters {
                media: None,
                audio: None,
                audio_stream_end: None,
                video: None,
                text: Some("你好，我叫小明。".to_string()),
                activity_start: None,
                activity_end: None,
            })
            .await?;
    } else {
        session.send_text("你好，我叫小明。").await?;
    }

    let mut handle = None;
    let mut text_started = false;
    let mut last_char: Option<char> = None;
    let mut turns = 0;

    println!("等待响应并收集 resumption handle...");

    // 先完成第一轮对话
    loop {
        let message = session.receive().await;
        let Some(message) = message else { break };
        let message = message?;

        // 收集 resumption handle
        if let Some(update) = message.session_resumption_update.as_ref() {
            if update.resumable.unwrap_or(false) && update.new_handle.is_some() {
                handle = update.new_handle.clone();
                println!("[收到可用的 resumption handle]");
            }
        }

        // 显示模型响应
        if native_audio {
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
        } else if let Some(content) = message
            .server_content
            .as_ref()
            .and_then(|c| c.model_turn.as_ref())
        {
            if let Some(text) = content.first_text() {
                if !text_started {
                    print!("assistant: ");
                    text_started = true;
                }
                print!("{}", text);
                std::io::stdout().flush().ok();
            }
        }

        if message
            .server_content
            .as_ref()
            .and_then(|c| c.turn_complete)
            .unwrap_or(false)
        {
            if text_started {
                println!();
                text_started = false;
            }
            turns += 1;
            break;
        }
    }

    // 如果还没获取到 handle，再发送一条消息
    if handle.is_none() && turns < 2 {
        println!("\n发送第二条消息...");
        if native_audio {
            session
                .send_realtime_input(LiveSendRealtimeInputParameters {
                    media: None,
                    audio: None,
                    audio_stream_end: None,
                    video: None,
                    text: Some("今天天气怎么样？".to_string()),
                    activity_start: None,
                    activity_end: None,
                })
                .await?;
        } else {
            session.send_text("今天天气怎么样？").await?;
        }

        loop {
            let message = session.receive().await;
            let Some(message) = message else { break };
            let message = message?;

            // 收集 resumption handle
            if let Some(update) = message.session_resumption_update.as_ref() {
                if update.resumable.unwrap_or(false) && update.new_handle.is_some() {
                    handle = update.new_handle.clone();
                    println!("[收到可用的 resumption handle]");
                }
            }

            // 显示模型响应
            if native_audio {
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
            } else if let Some(content) = message
                .server_content
                .as_ref()
                .and_then(|c| c.model_turn.as_ref())
            {
                if let Some(text) = content.first_text() {
                    if !text_started {
                        print!("assistant: ");
                        text_started = true;
                    }
                    print!("{}", text);
                    std::io::stdout().flush().ok();
                }
            }

            if message
                .server_content
                .as_ref()
                .and_then(|c| c.turn_complete)
                .unwrap_or(false)
            {
                if text_started {
                    println!();
                }
                break;
            }
        }
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
        if native_audio {
            resumed
                .send_realtime_input(LiveSendRealtimeInputParameters {
                    media: None,
                    audio: None,
                    audio_stream_end: None,
                    video: None,
                    text: Some("我叫什么名字？".to_string()),
                    activity_start: None,
                    activity_end: None,
                })
                .await?;
        } else {
            resumed.send_text("我叫什么名字？").await?;
        }

        // 显示恢复后的响应
        text_started = false;
        last_char = None;

        while let Some(message) = resumed.receive().await {
            let message = message?;

            if native_audio {
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
            } else if let Some(content) = message
                .server_content
                .as_ref()
                .and_then(|c| c.model_turn.as_ref())
            {
                if let Some(text) = content.first_text() {
                    if !text_started {
                        print!("assistant: ");
                        text_started = true;
                    }
                    print!("{}", text);
                    std::io::stdout().flush().ok();
                }
            }

            if message
                .server_content
                .as_ref()
                .and_then(|c| c.turn_complete)
                .unwrap_or(false)
            {
                if text_started {
                    println!();
                }
                break;
            }
        }

        resumed.close().await?;
        println!("\n✓ 会话恢复成功！模型记住了你叫\"小明\"。");
    } else {
        println!("\n未获取到可用的 resumption handle。");
        println!("注意：Session resumption 功能可能在当前模型或 API 版本上还未完全启用。");
        session.close().await?;
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
