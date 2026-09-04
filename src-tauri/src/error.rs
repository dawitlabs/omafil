use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct DirectoryError {
    code: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
pub(crate) struct DriveError {
    code: &'static str,
    message: &'static str,
}

#[derive(Serialize)]
pub(crate) struct RecentFilesError {
    code: &'static str,
    message: &'static str,
}

impl DirectoryError {
    pub(crate) const fn unavailable() -> Self {
        Self {
            code: "directory_unavailable",
            message: "This folder is unavailable on this device.",
        }
    }

    pub(crate) const fn read_failed() -> Self {
        Self {
            code: "directory_read_failed",
            message: "Unable to read this folder.",
        }
    }

    pub(crate) const fn not_allowed() -> Self {
        Self {
            code: "directory_not_allowed",
            message: "This location is outside your files and drives.",
        }
    }

    pub(crate) const fn open_failed() -> Self {
        Self {
            code: "open_failed",
            message: "Unable to open this item.",
        }
    }

    pub(crate) const fn invalid_name() -> Self {
        Self {
            code: "invalid_name",
            message: "That name contains characters that are not allowed.",
        }
    }

    pub(crate) const fn already_exists() -> Self {
        Self {
            code: "already_exists",
            message: "An item with that name already exists here.",
        }
    }

    pub(crate) const fn invalid_destination() -> Self {
        Self {
            code: "invalid_destination",
            message: "A folder cannot be moved into itself.",
        }
    }

    pub(crate) const fn operation_failed() -> Self {
        Self {
            code: "operation_failed",
            message: "Unable to complete that operation.",
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
}

impl DriveError {
    pub(crate) const fn discovery_failed() -> Self {
        Self {
            code: "drive_discovery_failed",
            message: "Unable to discover mounted drives.",
        }
    }
}
