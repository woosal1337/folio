use std::time::Duration;

use folio_core::server::{sync_session, RemoteClient, RemoteStatus};
use folio_core::transcription::SessionTranscript;

#[tokio::test]
#[ignore]
async fn remote_round_trip_against_live_server() {
    let endpoint = std::env::var("FOLIO_TEST_ENDPOINT")
        .expect("set FOLIO_TEST_ENDPOINT (e.g. https://folio-api.example.com)");
    let wav = std::env::var("FOLIO_TEST_WAV").expect("set FOLIO_TEST_WAV (path to a speech wav)");

    let email = format!("rusttest-{}@example.com", uuid::Uuid::new_v4().simple());
    let password = "supersecret123";

    let anon = RemoteClient::new(&endpoint).unwrap();
    let caps = anon.capabilities().await.expect("capabilities");
    println!(
        "CAPABILITIES engine={} model={} gpu={}",
        caps.engine, caps.model, caps.gpu
    );

    let tokens = anon.register(&email, password).await.expect("register");
    let client = RemoteClient::new(&endpoint)
        .unwrap()
        .with_token(tokens.access_token);

    let dir = tempfile::tempdir().unwrap();
    let session = dir.path().join("2026-07-22-11-57-07");
    std::fs::create_dir_all(&session).unwrap();
    std::fs::copy(&wav, session.join("mic.wav")).expect("copy test wav");

    let mut outcome = sync_session(&client, &session, Some("en"))
        .await
        .expect("first sync pass");
    let mut guard = 0;
    while !matches!(
        outcome.state.remote_status,
        RemoteStatus::Succeeded | RemoteStatus::Failed
    ) && guard < 120
    {
        tokio::time::sleep(Duration::from_secs(2)).await;
        outcome = sync_session(&client, &session, Some("en"))
            .await
            .expect("sync pass");
        guard += 1;
    }

    assert!(
        matches!(outcome.state.remote_status, RemoteStatus::Succeeded),
        "remote status={:?} error={:?}",
        outcome.state.remote_status,
        outcome.state.error
    );

    let transcript = SessionTranscript::read_json(&session.join("transcript.json"))
        .expect("transcript written to disk");
    let text: String = transcript
        .channels
        .iter()
        .flat_map(|c| c.segments.iter())
        .map(|s| s.text.trim())
        .collect::<Vec<_>>()
        .join(" ");
    println!("TRANSCRIPT: {text}");
    assert!(!transcript.channels.is_empty(), "no channels in transcript");
    assert!(!text.trim().is_empty(), "transcript text was empty");
}
