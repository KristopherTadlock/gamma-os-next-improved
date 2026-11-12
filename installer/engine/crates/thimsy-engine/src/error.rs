use std::fmt;

/// Stable error codes the app maps to messages (see docs/engine-protocol.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    DeviceNotFound,
    WrongDevice,
    DownloadFailed,
    HashMismatch,
    NotEnoughSpace,
    ToolMissing,
    ToolFailed,
    UnsupportedHost,
    ProfileUnavailable,
    UnlockGuided,
    Cancelled,
    BadInput,
    Internal,
}

impl Code {
    pub fn as_str(self) -> &'static str {
        match self {
            Code::DeviceNotFound => "device_not_found",
            Code::WrongDevice => "wrong_device",
            Code::DownloadFailed => "download_failed",
            Code::HashMismatch => "hash_mismatch",
            Code::NotEnoughSpace => "not_enough_space",
            Code::ToolMissing => "tool_missing",
            Code::ToolFailed => "tool_failed",
            Code::UnsupportedHost => "unsupported_host",
            Code::ProfileUnavailable => "profile_unavailable",
            Code::UnlockGuided => "unlock_guided",
            Code::Cancelled => "cancelled",
            Code::BadInput => "bad_input",
            Code::Internal => "internal",
        }
    }
}

/// An error with a code and an optional hint for the user.
#[derive(Debug)]
pub struct EngineError {
    pub code: Code,
    pub message: String,
    pub hint: Option<String>,
}

impl EngineError {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        EngineError {
            code,
            message: message.into(),
            hint: None,
        }
    }
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for EngineError {}

/// Shorthand: `bail_code!(Code::X, "message {}", arg)`.
#[macro_export]
macro_rules! bail_code {
    ($code:expr, $($arg:tt)*) => {
        return Err($crate::error::EngineError::new($code, format!($($arg)*)).into())
    };
}

/// Finds the code of an `anyhow::Error`, or `Internal`.
pub fn code_of(err: &anyhow::Error) -> (Code, Option<String>) {
    for cause in err.chain() {
        if let Some(e) = cause.downcast_ref::<EngineError>() {
            return (e.code, e.hint.clone());
        }
    }
    (Code::Internal, None)
}
