//! Flat-file receive staging. This does not parse or authenticate AirDrop traffic.
use super::error::{AirDropError, ErrorCode};
use rustix::fs::{open, renameat_with, Mode, OFlags, RenameFlags};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    io::Write,
    os::fd::AsRawFd,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use tempfile::TempDir;

const MAX_FILES: usize = 256;
const MAX_BYTES: u64 = 10 * 1024 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileOffer {
    pub name: String,
    pub size: u64,
    /// An adapter may supply a digest only when the actual protocol provides it.
    pub expected_sha256: Option<[u8; 32]>,
}

pub fn validate_name(name: &str) -> Result<(), AirDropError> {
    if name.is_empty()
        || name.len() > 255
        || matches!(name, "." | "..")
        || name.chars().any(|c| {
            c.is_control()
                || matches!(c, '/' | '\\' | ':' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
    {
        return Err(ErrorCode::InvalidMetadata.into());
    }
    Ok(())
}

fn validate_offer(offers: &[FileOffer]) -> Result<(), AirDropError> {
    if offers.is_empty() || offers.len() > MAX_FILES {
        return Err(ErrorCode::InvalidMetadata.into());
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for offer in offers {
        validate_name(&offer.name)?;
        if !names.insert(&offer.name) {
            return Err(ErrorCode::InvalidMetadata.into());
        }
        total = total
            .checked_add(offer.size)
            .filter(|n| *n <= MAX_BYTES)
            .ok_or(ErrorCode::InvalidMetadata)?;
    }
    Ok(())
}

struct ActiveFile {
    file: File,
    bytes: u64,
    hash: Sha256,
}

pub struct ReceiveBatch {
    // Drop staging before the directory fd: its /proc path depends on that fd.
    staging: TempDir,
    directory: File,
    offers: Vec<FileOffer>,
    completed: usize,
    active: Option<ActiveFile>,
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    poisoned: bool,
}

impl ReceiveBatch {
    /// `destination` is selected locally, never supplied by the remote peer.
    pub fn new(
        destination: &Path,
        offers: Vec<FileOffer>,
        cancelled: Arc<AtomicBool>,
        deadline: Instant,
    ) -> Result<Self, AirDropError> {
        validate_offer(&offers)?;
        if cancelled.load(Ordering::Acquire) {
            return Err(ErrorCode::Cancelled.into());
        }
        if Instant::now() >= deadline {
            return Err(ErrorCode::TimedOut.into());
        }
        let directory = File::from(
            open(
                destination,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| ErrorCode::StorageFailed)?,
        );
        // Pin the destination across renames and symlink substitution during a transfer.
        let anchored = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
        let staging = tempfile::Builder::new()
            .prefix(".omafil-airdrop-")
            .tempdir_in(anchored)?;
        Ok(Self {
            staging,
            directory,
            offers,
            completed: 0,
            active: None,
            cancelled,
            deadline,
            poisoned: false,
        })
    }

    fn check(&self) -> Result<(), AirDropError> {
        if self.poisoned {
            return Err(ErrorCode::InvalidState.into());
        }
        if self.cancelled.load(Ordering::Acquire) {
            return Err(ErrorCode::Cancelled.into());
        }
        if Instant::now() >= self.deadline {
            return Err(ErrorCode::TimedOut.into());
        }
        Ok(())
    }

    /// Files arrive in the validated manifest order. No remote path is accepted here.
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), AirDropError> {
        let result = self.write_inner(bytes);
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    fn write_inner(&mut self, bytes: &[u8]) -> Result<(), AirDropError> {
        self.check()?;
        let offer = self
            .offers
            .get(self.completed)
            .ok_or(ErrorCode::InvalidState)?;
        let written = self.active.as_ref().map_or(0, |file| file.bytes);
        let next = written
            .checked_add(bytes.len() as u64)
            .filter(|n| *n <= offer.size)
            .ok_or(ErrorCode::InvalidMetadata)?;
        if self.active.is_none() {
            let file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(self.staging.path().join(&offer.name))?;
            self.active = Some(ActiveFile {
                file,
                bytes: 0,
                hash: Sha256::new(),
            });
        }
        let active = self.active.as_mut().ok_or(ErrorCode::InvalidState)?;
        active.file.write_all(bytes)?;
        active.hash.update(bytes);
        active.bytes = next;
        Ok(())
    }

    /// Returns a local checksum; without an authenticated expected digest it is not proof of sender identity.
    pub fn finish_file(&mut self) -> Result<[u8; 32], AirDropError> {
        let result = self.finish_inner();
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }

    fn finish_inner(&mut self) -> Result<[u8; 32], AirDropError> {
        self.write_inner(&[])?;
        let active = self.active.take().ok_or(ErrorCode::InvalidState)?;
        let offer = &self.offers[self.completed];
        if active.bytes != offer.size {
            return Err(ErrorCode::Incomplete.into());
        }
        let digest: [u8; 32] = active.hash.finalize().into();
        if offer
            .expected_sha256
            .is_some_and(|expected| expected != digest)
        {
            return Err(ErrorCode::IntegrityMismatch.into());
        }
        active.file.sync_all()?;
        self.completed += 1;
        Ok(digest)
    }

    /// Atomically publishes the batch as a new directory; existing names are never replaced.
    /// The caller must first validate the transport/archive completion, then choose a local name.
    pub fn publish(self, local_name: &str) -> Result<(), AirDropError> {
        self.check()?;
        validate_name(local_name)?;
        if self.completed != self.offers.len() || self.active.is_some() {
            return Err(ErrorCode::Incomplete.into());
        }
        File::open(self.staging.path())?.sync_all()?;
        self.check()?;
        let source = self
            .staging
            .path()
            .file_name()
            .ok_or(ErrorCode::StorageFailed)?;
        renameat_with(
            &self.directory,
            source,
            &self.directory,
            local_name,
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| {
            if error == rustix::io::Errno::EXIST {
                AirDropError::from(ErrorCode::DestinationExists)
            } else {
                ErrorCode::StorageFailed.into()
            }
        })?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
