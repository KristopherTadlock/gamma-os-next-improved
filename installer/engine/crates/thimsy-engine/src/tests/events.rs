use crate::events::{Event, MemorySink, Reporter};
use std::sync::Arc;

#[test]
fn progress_is_rate_limited_but_final_progress_always_arrives() {
    let sink = Arc::new(MemorySink::default());
    let rep = Reporter::new(sink.clone());
    rep.stage("fetch", "Downloading");
    for i in 0..10_000u64 {
        rep.progress("file", i, 10_000);
    }
    rep.progress("file", 10_000, 10_000);
    let ev = sink.0.lock().unwrap();
    let progress: Vec<_> = ev
        .iter()
        .filter(|e| matches!(e, Event::Progress { .. }))
        .collect();
    assert!(progress.len() < 20, "{} progress events", progress.len());
    assert!(
        matches!(progress.last().unwrap(), Event::Progress { done: 10_000, total: 10_000, stage, .. } if stage == "fetch")
    );
}

#[test]
fn events_serialize_to_the_protocol_shape() {
    let e = Event::Error {
        code: "hash_mismatch".into(),
        message: "m".into(),
        hint: None,
    };
    assert_eq!(
        serde_json::to_string(&e).unwrap(),
        r#"{"type":"error","code":"hash_mismatch","message":"m","hint":null}"#
    );
    let e = Event::Prompt {
        id: "x".into(),
        title: "t".into(),
        body: "b".into(),
        image: None,
    };
    assert!(serde_json::to_string(&e)
        .unwrap()
        .starts_with(r#"{"type":"prompt","id":"x""#));
}

#[test]
fn errors_carry_code_hint_and_full_chain() {
    let sink = Arc::new(MemorySink::default());
    let rep = Reporter::new(sink.clone());
    let err: anyhow::Error = crate::EngineError::new(crate::Code::DeviceNotFound, "no device")
        .hint("plug it in")
        .into();
    let err = err.context("flashing");
    rep.error(&err);
    let ev = sink.0.lock().unwrap();
    match &ev[0] {
        Event::Error {
            code,
            message,
            hint,
        } => {
            assert_eq!(code, "device_not_found");
            assert_eq!(message, "flashing: no device");
            assert_eq!(hint.as_deref(), Some("plug it in"));
        }
        e => panic!("{e:?}"),
    }
    // Plain errors map to internal.
    assert_eq!(
        crate::error::code_of(&anyhow::anyhow!("x")).0,
        crate::Code::Internal
    );
}
