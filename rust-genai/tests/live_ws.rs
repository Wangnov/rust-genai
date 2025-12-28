use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

use rust_genai::types;
use rust_genai::Client;

const fn live_server_message(
    setup_complete: Option<types::live_types::LiveServerSetupComplete>,
    server_content: Option<types::live_types::LiveServerContent>,
    go_away: Option<types::live_types::LiveServerGoAway>,
    session_resumption_update: Option<types::live_types::LiveServerSessionResumptionUpdate>,
) -> types::live_types::LiveServerMessage {
    types::live_types::LiveServerMessage {
        setup_complete,
        server_content,
        tool_call: None,
        tool_call_cancellation: None,
        usage_metadata: None,
        go_away,
        session_resumption_update,
        voice_activity_detection_signal: None,
    }
}

#[tokio::test]
async fn live_session_websocket_flow() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(run_live_session_server(listener));

    let base_url = format!("http://{addr}");
    let client = Client::builder()
        .api_key("test-key")
        .base_url(base_url)
        .build()
        .unwrap();

    let mut session = client
        .live()
        .connect(
            "gemini-2.0-flash",
            types::live_types::LiveConnectConfig::default(),
        )
        .await
        .unwrap();

    send_live_session_messages(&session).await.unwrap();

    let _ = session.receive().await.unwrap().unwrap();
    let _ = session.receive().await.unwrap().unwrap();

    assert_eq!(session.resumption_handle(), Some("handle-1".to_string()));
    assert_eq!(session.last_go_away_time_left(), Some("10s".to_string()));

    session.close().await.unwrap();
    let _ = server.await;
}

async fn run_live_session_server(listener: TcpListener) {
    let (stream, _) = listener.accept().await.unwrap();
    let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Consume setup message from client.
    let _ = read.next().await;

    let setup_complete = live_server_message(
        Some(types::live_types::LiveServerSetupComplete {
            session_id: Some("session-1".to_string()),
        }),
        None,
        None,
        None,
    );
    write
        .send(Message::Text(
            serde_json::to_string(&setup_complete).unwrap().into(),
        ))
        .await
        .unwrap();

    let update = live_server_message(
        None,
        None,
        Some(types::live_types::LiveServerGoAway {
            time_left: Some("10s".to_string()),
        }),
        Some(types::live_types::LiveServerSessionResumptionUpdate {
            new_handle: Some("handle-1".to_string()),
            resumable: Some(true),
            last_consumed_client_message_index: Some("idx-1".to_string()),
        }),
    );
    write
        .send(Message::Text(
            serde_json::to_string(&update).unwrap().into(),
        ))
        .await
        .unwrap();

    let binary = live_server_message(
        None,
        Some(types::live_types::LiveServerContent {
            model_turn: Some(types::content::Content::from_parts(
                vec![types::content::Part::text("hi")],
                types::content::Role::Model,
            )),
            turn_complete: Some(true),
            interrupted: None,
            grounding_metadata: None,
            generation_complete: Some(true),
            input_transcription: None,
            output_transcription: None,
            url_context_metadata: None,
            turn_complete_reason: None,
            waiting_for_input: None,
        }),
        None,
        None,
    );
    write
        .send(Message::Binary(serde_json::to_vec(&binary).unwrap().into()))
        .await
        .unwrap();

    write
        .send(Message::Ping(vec![1, 2, 3].into()))
        .await
        .unwrap();

    // Drain client messages then close.
    for _ in 0..5 {
        let _ = read.next().await;
    }
    let _ = write.send(Message::Close(None)).await;
}

async fn send_live_session_messages(
    session: &rust_genai::live::LiveSession,
) -> rust_genai::Result<()> {
    session.send_text("hello").await?;
    session.send_audio(vec![1, 2, 3], "audio/wav").await?;
    session
        .send_client_content(types::live_types::LiveSendClientContentParameters {
            turns: Some(vec![types::content::Content::text("turn")]),
            turn_complete: Some(true),
        })
        .await?;
    session
        .send_realtime_input(types::live_types::LiveSendRealtimeInputParameters {
            media: None,
            audio: None,
            audio_stream_end: Some(true),
            video: None,
            text: Some("rt".into()),
            activity_start: None,
            activity_end: None,
        })
        .await?;
    session
        .send_tool_response(types::live_types::LiveSendToolResponseParameters {
            function_responses: Some(vec![types::content::FunctionResponse {
                will_continue: None,
                scheduling: None,
                parts: None,
                id: Some("id".into()),
                name: Some("tool".into()),
                response: Some(json!({"ok": true})),
            }]),
        })
        .await?;
    Ok(())
}
#[tokio::test]
async fn live_music_websocket_flow() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws_stream = tokio_tungstenite::accept_async(stream).await.unwrap();
        let (mut write, mut read) = ws_stream.split();

        // Consume setup message from client.
        let _ = read.next().await;

        let setup_complete = types::live_music_types::LiveMusicServerMessage {
            setup_complete: Some(types::live_music_types::LiveMusicServerSetupComplete {}),
            ..Default::default()
        };
        write
            .send(Message::Text(
                serde_json::to_string(&setup_complete).unwrap().into(),
            ))
            .await
            .unwrap();

        let server_content = types::live_music_types::LiveMusicServerMessage {
            server_content: Some(types::live_music_types::LiveMusicServerContent {
                audio_chunks: Some(vec![types::live_music_types::AudioChunk {
                    data: Some(vec![1, 2, 3]),
                    mime_type: Some("audio/wav".to_string()),
                    source_metadata: None,
                }]),
            }),
            ..Default::default()
        };
        write
            .send(Message::Binary(
                serde_json::to_vec(&server_content).unwrap().into(),
            ))
            .await
            .unwrap();
        write
            .send(Message::Ping(vec![1, 2, 3].into()))
            .await
            .unwrap();

        for _ in 0..6 {
            let _ = read.next().await;
        }
        let _ = write.send(Message::Close(None)).await;
    });

    let base_url = format!("http://{addr}");
    let client = Client::builder()
        .api_key("test-key")
        .base_url(base_url)
        .build()
        .unwrap();

    let mut session = client.live_music().connect("music-model").await.unwrap();

    let err = session.set_weighted_prompts(vec![]).await;
    assert!(err.is_err());

    session
        .set_weighted_prompts(vec![types::live_music_types::WeightedPrompt {
            text: Some("calm".to_string()),
            weight: Some(1.0),
        }])
        .await
        .unwrap();
    session
        .set_music_generation_config(Some(types::live_music_types::LiveMusicGenerationConfig {
            temperature: Some(0.2),
            ..Default::default()
        }))
        .await
        .unwrap();
    session.play().await.unwrap();
    session.pause().await.unwrap();
    session.stop().await.unwrap();
    session.reset_context().await.unwrap();

    let msg = session.receive().await.unwrap().unwrap();
    assert!(msg.server_content.is_some());

    session.close().await.unwrap();
    let _ = server.await;
}
