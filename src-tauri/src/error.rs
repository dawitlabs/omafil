use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DirectoryError {
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
pub(crate) struct DriveError {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct RecentFilesError {
    code: &'static str,
    message: &'static str,
}

impl DirectoryError {
    pub(crate) fn cancelled() -> Self {
        Self {
            code: "operation_cancelled",
            message: "Operation cancelled. Completed items were kept.".into(),
        }
    }

    pub(crate) fn detail(message: impl Into<String>) -> Self {
        Self {
            code: "operation_failed",
            message: message.into(),
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.code == "operation_cancelled"
    }
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn unavailable() -> Self {
        Self {
            code: "directory_unavailable",
            message: "This folder is unavailable on this device.".into(),
        }
    }

    pub(crate) fn read_failed() -> Self {
        Self {
            code: "directory_read_failed",
            message: "Unable to read this folder.".into(),
        }
    }

    pub(crate) fn open_failed() -> Self {
        Self {
            code: "open_failed",
            message: "Unable to open this item.".into(),
        }
    }

    pub(crate) fn invalid_name() -> Self {
        Self {
            code: "invalid_name",
            message: "That name contains characters that are not allowed.".into(),
        }
    }

    pub(crate) fn already_exists() -> Self {
        Self {
            code: "already_exists",
            message: "An item with that name already exists here.".into(),
        }
    }

    pub(crate) fn invalid_destination() -> Self {
        Self {
            code: "invalid_destination",
            message: "A folder cannot be moved into itself.".into(),
        }
    }

    pub(crate) fn operation_failed() -> Self {
        Self {
            code: "operation_failed",
            message: "Unable to complete that operation.".into(),
        }
    }
}

impl RecentFilesError {
    pub(crate) const fn unavailable() -> Self {
        Self {
            code: "recent_files_unavailable",
            message: "Unable to read recent files from this desktop.",
        }
    }

    pub(crate) const fn clear_failed() -> Self {
        Self {
            code: "recent_files_clear_failed",
            message: "Unable to clear the list of recently used files.",
        }
    }
}

impl DriveError {
    pub(crate) const fn discovery_failed() -> Self {
        Self {
            code: "drive_discovery_failed",
            message: "Unable to discover mounted drives.",
        }
    }
}

impl From<std::io::Error> for DirectoryError {
    fn from(error: std::io::Error) -> Self {
        let (code, message) = match error.kind() {
            std::io::ErrorKind::PermissionDenied => ("permission_denied", "You do not have permission to access or change this item."),
            std::io::ErrorKind::NotFound => ("location_missing", "This location no longer exists or its drive is not mounted."),
            std::io::ErrorKind::NotConnected | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe =>
                ("location_disconnected", "The connection to this location was lost. Reconnect and try again."),
            _ if error.raw_os_error() == Some(107) =>
                ("location_disconnected", "The connection to this location was lost. Reconnect and try again."),
            _ => return Self::detail(format!("Unable to complete this operation: {error}")),
        };
        Self { code, message: message.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn io_errors_keep_actionable_categories() {
        for (kind, code) in [
            (std::io::ErrorKind::PermissionDenied, "permission_denied"),
            (std::io::ErrorKind::NotFound, "location_missing"),
            (std::io::ErrorKind::NotConnected, "location_disconnected"),
        ] {
            assert_eq!(DirectoryError::from(std::io::Error::from(kind)).code, code);
        }
    }
}
