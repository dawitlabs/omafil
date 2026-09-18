//! Application primitives only. No native AirDrop transport is connected.
pub mod commands;
pub mod error;
pub mod session;
#[cfg(target_os = "linux")]
pub mod storage;

use error::{AirDropError, ErrorCode};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Availability {
    pub available: bool,
    pub reason: AirDropError,
}

pub fn availability() -> Availability {
    Availability {
        available: false,
        reason: ErrorCode::TransportUnavailable.into(),
    }
}

// A shared guard keeps every entry point fail-closed until a verified transport exists.
fn require_transport<T>() -> Result<T, AirDropError> {
    Err(ErrorCode::TransportUnavailable.into())
}
