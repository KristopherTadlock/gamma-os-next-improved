//! The computer the installer runs on.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Host {
    Windows,
    Macos,
    Linux,
    Android,
}

impl Host {
    pub fn current() -> Host {
        if cfg!(target_os = "windows") {
            Host::Windows
        } else if cfg!(target_os = "macos") {
            Host::Macos
        } else if cfg!(target_os = "android") {
            Host::Android
        } else {
            Host::Linux
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Host::Windows => "windows",
            Host::Macos => "macos",
            Host::Linux => "linux",
            Host::Android => "android",
        }
    }
    pub fn display(self) -> &'static str {
        match self {
            Host::Windows => "Windows",
            Host::Macos => "macOS",
            Host::Linux => "Linux",
            Host::Android => "Android",
        }
    }
    pub fn parse(s: &str) -> Option<Host> {
        match s.to_ascii_lowercase().as_str() {
            "windows" => Some(Host::Windows),
            "macos" | "mac" | "darwin" => Some(Host::Macos),
            "linux" => Some(Host::Linux),
            "android" => Some(Host::Android),
            _ => None,
        }
    }
    pub fn exe_suffix(self) -> &'static str {
        if self == Host::Windows {
            ".exe"
        } else {
            ""
        }
    }
}
