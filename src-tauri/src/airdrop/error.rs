use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    TransportUnavailable,
    InvalidMetadata,
    InvalidState,
    Cancelled,
    TimedOut,
    IntegrityMismatch,
    Incomplete,
    DestinationExists,
    StorageFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AirDropError {
    pub code: ErrorCode,
    pub message: &'static str,
}

impl From<ErrorCode> for AirDropError {
    fn from(code: ErrorCode) -> Self {
        let message = match code {
            ErrorCode::TransportUnavailable => "AirDrop is not available in this version of Omafil. Native iPhone discovery and transfers are not supported yet.",
            ErrorCode::InvalidMetadata => "The transfer contains invalid or unsupported file information.",
            ErrorCode::InvalidState => "This transfer action is no longer available.",
            ErrorCode::Cancelled => "The transfer was cancelled.",
            ErrorCode::TimedOut => "The transfer timed out.",
            ErrorCode::IntegrityMismatch => "The received file did not match its expected checksum.",
            ErrorCode::Incomplete => "The transfer is incomplete.",
            ErrorCode::DestinationExists => "The destination already exists. Choose another name.",
            ErrorCode::StorageFailed => "The received files could not be saved.",
        };
        Self { code, message }
    }
}

impl From<std::io::Error> for AirDropError {
    fn from(_: std::io::Error) -> Self {
        ErrorCode::StorageFailed.into()
    }
}
