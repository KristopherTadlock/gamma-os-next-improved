//! Thimsy universal installer engine.
//!
//! Everything the app does besides drawing windows lives here: the device and
//! profile catalog, plan resolution, downloads, firmware container editing
//! (Unisoc PAC, Android `super`), device-pack deltas, the job journal, USB
//! detection and running the bundled flashing tools.

pub mod archive;
pub mod catalog;
pub mod delta;
pub mod detect;
pub mod drivers;
pub mod error;
pub mod events;
pub mod fetch;
pub mod flash;
pub mod host;
pub mod i18n;
pub mod job;
pub mod lp;
pub mod manifest;
pub mod pac;
pub mod pack;
pub mod resolve;
pub mod tools;
pub mod util;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod testutil;

pub use error::{Code, EngineError};
pub use events::{Event, Reporter};

/// Engine version, shown in `catalog` output and job locks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
