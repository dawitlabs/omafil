use crate::paths::current_user_home_directory;
use serde::Serialize;
use std::path::{Path, PathBuf};
use sysinfo::Disks;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DriveInfo {
    name: String,
    mount_point: String,
    path: String,
    total_bytes: u64,
    available_bytes: u64,
    is_removable: bool,
    is_read_only: bool,
}

pub(crate) fn is_user_visible_drive_mount(mount_point: &Path) -> bool {
    if mount_point == Path::new("/") {
        return true;
    }

    mount_point.starts_with("/media")
        || mount_point.starts_with("/mnt")
        || mount_point.starts_with("/run/media")
        // GNOME/KDE mount SMB, SFTP and other GVFS locations beneath the
        // current user's runtime directory instead of /media.
        || mount_point.to_string_lossy().contains("/gvfs/")
}

pub(crate) fn drive_navigation_path(mount_point: &Path) -> PathBuf {
    if mount_point == Path::new("/") {
        if let Ok(home_directory) = current_user_home_directory() {
            return home_directory;
        }
    }

    mount_point.to_path_buf()
}

pub(crate) fn read_drives() -> Vec<DriveInfo> {
    let disks = Disks::new_with_refreshed_list();
    let home_directory_name = current_user_home_directory().ok().and_then(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    });
    let mut drives = disks
        .list()
        .iter()
        .filter(|disk| disk.total_space() > 0 && is_user_visible_drive_mount(disk.mount_point()))
        .map(|disk| DriveInfo {
            name: if disk.mount_point() == Path::new("/") {
                home_directory_name
                    .clone()
                    .unwrap_or_else(|| "Home".to_owned())
            } else {
                disk.name().to_string_lossy().into_owned()
            },
            mount_point: disk.mount_point().to_string_lossy().into_owned(),
            path: drive_navigation_path(disk.mount_point())
                .to_string_lossy()
                .into_owned(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
            is_removable: disk.is_removable(),
            is_read_only: disk.is_read_only(),
        })
        .collect::<Vec<_>>();

    drives.sort_by(|left, right| left.mount_point.cmp(&right.mount_point));
    drives
}

#[cfg(test)]
mod tests {
    use super::is_user_visible_drive_mount;
    use std::path::Path;

    #[test]
    fn drive_usage_percentage_is_bounded_by_total_space() {
        let total_bytes = 100_u64;
        let available_bytes = 40_u64;
        let used_percentage = (total_bytes.saturating_sub(available_bytes) * 100) / total_bytes;

        assert_eq!(used_percentage, 60);
    }

    #[test]
    fn only_user_facing_linux_mounts_appear_as_drives() {
        assert!(is_user_visible_drive_mount(Path::new("/")));
        assert!(is_user_visible_drive_mount(Path::new("/run/media/dave/USB")));
        assert!(is_user_visible_drive_mount(Path::new("/media/USB")));
        assert!(is_user_visible_drive_mount(Path::new("/run/user/1000/gvfs/smb-share:server=nas,share=files")));
        assert!(!is_user_visible_drive_mount(Path::new("/proc")));
        assert!(!is_user_visible_drive_mount(Path::new("/var/lib/docker")));
        assert!(!is_user_visible_drive_mount(Path::new("/home")));
    }
}
