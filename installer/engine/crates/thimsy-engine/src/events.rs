//! Progress and status events, written as JSON Lines for the app or as text
//! for a terminal.

use serde::Serialize;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Stage {
        stage: String,
        title: String,
    },
    Progress {
        stage: String,
        label: String,
        done: u64,
        total: u64,
    },
    Log {
        level: String,
        message: String,
    },
    Prompt {
        id: String,
        title: String,
        body: String,
        image: Option<String>,
    },
    Error {
        code: String,
        message: String,
        hint: Option<String>,
    },
}

pub trait Sink: Send + Sync {
    fn emit(&self, event: &Event);
}

/// JSON Lines on stdout.
pub struct JsonSink;
impl Sink for JsonSink {
    fn emit(&self, event: &Event) {
        let line = serde_json::to_string(event).expect("event serializes");
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

/// Human-readable text on stderr.
pub struct TextSink;
impl Sink for TextSink {
    fn emit(&self, event: &Event) {
        let mut err = std::io::stderr().lock();
        let _ = match event {
            Event::Stage { title, .. } => writeln!(err, "== {title}"),
            Event::Progress {
                label, done, total, ..
            } => {
                if *total > 0 {
                    writeln!(
                        err,
                        "   {label}: {:.1}%",
                        *done as f64 * 100.0 / *total as f64
                    )
                } else {
                    writeln!(err, "   {label}: {done}")
                }
            }
            Event::Log { level, message } => writeln!(err, "   [{level}] {message}"),
            Event::Prompt { title, body, .. } => writeln!(err, ">> {title}\n   {body}"),
            Event::Error {
                code,
                message,
                hint,
            } => match hint {
                Some(h) => writeln!(err, "error ({code}): {message}\n   hint: {h}"),
                None => writeln!(err, "error ({code}): {message}"),
            },
        };
    }
}

/// Collects events in memory (tests).
#[derive(Default)]
pub struct MemorySink(pub Mutex<Vec<Event>>);
impl Sink for MemorySink {
    fn emit(&self, event: &Event) {
        self.0.lock().unwrap().push(event.clone());
    }
}

/// Cheap, cloneable handle used throughout the engine.
#[derive(Clone)]
pub struct Reporter {
    sink: Arc<dyn Sink>,
    stage: Arc<Mutex<String>>,
    last_progress: Arc<Mutex<Option<Instant>>>,
}

impl Reporter {
    pub fn new(sink: Arc<dyn Sink>) -> Self {
        Reporter {
            sink,
            stage: Arc::new(Mutex::new(String::new())),
            last_progress: Arc::new(Mutex::new(None)),
        }
    }
    pub fn json() -> Self {
        Self::new(Arc::new(JsonSink))
    }
    pub fn text() -> Self {
        Self::new(Arc::new(TextSink))
    }
    pub fn emit(&self, event: Event) {
        self.sink.emit(&event);
    }
    pub fn stage(&self, stage: &str, title: &str) {
        *self.stage.lock().unwrap() = stage.to_string();
        *self.last_progress.lock().unwrap() = None;
        self.emit(Event::Stage {
            stage: stage.into(),
            title: crate::i18n::tr(title),
        });
    }
    /// Progress, rate-limited to ~5 events a second except the final one.
    pub fn progress(&self, label: &str, done: u64, total: u64) {
        let now = Instant::now();
        {
            let mut last = self.last_progress.lock().unwrap();
            let finished = total > 0 && done >= total;
            if let Some(t) = *last {
                if !finished && now.duration_since(t) < Duration::from_millis(200) {
                    return;
                }
            }
            *last = Some(now);
        }
        let stage = self.stage.lock().unwrap().clone();
        self.emit(Event::Progress {
            stage,
            label: label.into(),
            done,
            total,
        });
    }
    pub fn info(&self, message: impl Into<String>) {
        self.emit(Event::Log {
            level: "info".into(),
            message: message.into(),
        });
    }
    pub fn warn(&self, message: impl Into<String>) {
        self.emit(Event::Log {
            level: "warn".into(),
            message: message.into(),
        });
    }
    pub fn prompt(&self, id: &str, title: &str, body: &str, image: Option<&str>) {
        self.emit(Event::Prompt {
            id: id.into(),
            title: crate::i18n::tr(title),
            body: crate::i18n::tr(body),
            image: image.map(str::to_string),
        });
    }
    pub fn error(&self, err: &anyhow::Error) {
        let (code, hint) = crate::error::code_of(err);
        let message = err
            .chain()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(": ");
        self.emit(Event::Error {
            code: code.as_str().into(),
            message,
            hint: hint.map(|h| crate::i18n::tr(&h)),
        });
    }
}
