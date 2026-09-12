use crate::error::DirectoryError;
use crate::paths::current_user_home_directory;
use notify::RecommendedWatcher;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use sysinfo::Disks;

pub(crate) struct DriveWatcher(#[allow(dead_code)] pub(crate) RecommendedWatcher);

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
    is_mounted: bool,
    device: Option<String>,
}

#[derive(Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<BlockDevice>,
}

#[derive(Deserialize)]
struct BlockDevice {
    path: String,
    pkname: Option<String>,
    mountpoint: Option<String>,
    #[serde(default)]
    rm: bool,
    #[serde(default)]
    hotplug: bool,
    tran: Option<String>,
    #[serde(rename = "type")]
    kind: String,
    fstype: Option<String>,
    label: Option<String>,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    children: Vec<BlockDevice>,
}

impl BlockDevice {
    fn is_removable(&self) -> bool {
        self.rm || self.hotplug || self.tran.as_deref() == Some("usb")
    }

    fn is_mountable_filesystem(&self) -> bool {
        matches!(self.kind.as_str(), "part" | "disk")
            && self
                .fstype
                .as_deref()
                .is_some_and(|fs| !matches!(fs, "swap" | "crypto_LUKS" | "LVM2_member"))
    }

    fn display_name(&self) -> String {
        self.label
            .clone()
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| self.path.rsplit('/').next().unwrap_or(&self.path).to_owned())
    }
}

fn flatten(devices: Vec<BlockDevice>) -> Vec<BlockDevice> {
    devices
        .into_iter()
        .flat_map(|mut device| {
            let children = std::mem::take(&mut device.children);
            std::iter::once(device).chain(flatten(children))
        })
        .collect()
}

fn parse_block_devices(json: &[u8]) -> Vec<BlockDevice> {
    serde_json::from_slice::<LsblkOutput>(json)
        .map(|output| flatten(output.blockdevices))
        .unwrap_or_default()
}

fn read_block_devices() -> Vec<BlockDevice> {
    Command::new("lsblk")
        .args(["-J", "-b", "-o", "NAME,PATH,PKNAME,MOUNTPOINT,RM,HOTPLUG,TRAN,TYPE,FSTYPE,LABEL,SIZE"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| parse_block_devices(&output.stdout))
        .unwrap_or_default()
}

fn known_device(device: &str) -> Result<BlockDevice, DirectoryError> {
    read_block_devices()
        .into_iter()
        .find(|candidate| candidate.path == device)
        .ok_or_else(|| DirectoryError::detail("That drive is no longer connected."))
}

/// udisksctl reports failures as "Error mounting /dev/x: GDBus.Error:...: Not authorized",
/// and only the last clause is worth showing.
fn udisks(args: &[&str]) -> Result<String, DirectoryError> {
    let output = Command::new("udisksctl")
        .args(args)
        .output()
        .map_err(|_| DirectoryError::detail("udisksctl is not installed on this system."))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let reason = stderr.trim().rsplit(": ").next().unwrap_or("").trim_end_matches('.').to_owned();

    Err(DirectoryError::detail(if reason.is_empty() { "The drive command failed.".to_owned() } else { reason }))
}

fn mount_point_from(output: &str) -> Option<String> {
    output
        .rsplit(" at ")
        .next()
        .map(|point| point.trim_end_matches('.').to_owned())
        .filter(|point| point.starts_with('/'))
}

pub(crate) fn mount_drive(device: &str) -> Result<String, DirectoryError> {
    let device = known_device(device)?;
    let output = udisks(&["mount", "-b", &device.path])?;

    mount_point_from(&output).ok_or_else(|| DirectoryError::detail("Mounted, but the mount point could not be read."))
}

pub(crate) fn unmount_drive(device: &str) -> Result<(), DirectoryError> {
    let device = known_device(device)?;

    udisks(&["unmount", "-b", &device.path]).map(drop)
}

pub(crate) fn eject_drive(device: &str) -> Result<(), DirectoryError> {
    let device = known_device(device)?;

    if device.mountpoint.is_some() {
        udisks(&["unmount", "-b", &device.path])?;
    }
    let disk = device.pkname.as_ref().map(|name| format!("/dev/{name}")).unwrap_or_else(|| device.path.clone());

    udisks(&["power-off", "-b", &disk]).map(drop)
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
    let devices = read_block_devices();
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
            is_mounted: true,
            device: devices
                .iter()
                .find(|device| device.mountpoint.as_deref() == Some(&*disk.mount_point().to_string_lossy()))
                .map(|device| device.path.clone()),
        })
        .collect::<Vec<_>>();

    drives.extend(
        devices
            .iter()
            .filter(|device| device.mountpoint.is_none() && device.is_removable() && device.is_mountable_filesystem())
            .map(|device| DriveInfo {
                name: device.display_name(),
                mount_point: String::new(),
                path: String::new(),
                total_bytes: device.size,
                available_bytes: 0,
                is_removable: true,
                is_read_only: false,
                is_mounted: false,
                device: Some(device.path.clone()),
            }),
    );

    drives.sort_by(|left, right| right.is_mounted.cmp(&left.is_mounted).then_with(|| left.mount_point.cmp(&right.mount_point)));
    drives
}

#[cfg(test)]
mod tests {
    use super::{is_user_visible_drive_mount, mount_point_from, parse_block_devices};
    use std::path::Path;

    #[test]
    fn finds_unmounted_removable_filesystems_in_lsblk_output() {
        let json = br#"{"blockdevices": [
            {"name":"nvme0n1","path":"/dev/nvme0n1","pkname":null,"mountpoint":null,"rm":false,"hotplug":false,"tran":"nvme","type":"disk","fstype":null,"label":null,"size":10,
             "children":[{"name":"nvme0n1p1","path":"/dev/nvme0n1p1","pkname":"nvme0n1","mountpoint":"/boot","rm":false,"hotplug":false,"tran":"nvme","type":"part","fstype":"vfat","label":null,"size":5}]},
            {"name":"sdb","path":"/dev/sdb","pkname":null,"mountpoint":null,"rm":true,"hotplug":true,"tran":"usb","type":"disk","fstype":null,"label":null,"size":8,
             "children":[{"name":"sdb1","path":"/dev/sdb1","pkname":"sdb","mountpoint":null,"rm":true,"hotplug":true,"tran":"usb","type":"part","fstype":"exfat","label":"STICK","size":8}]}
        ]}"#;
        let devices = parse_block_devices(json);
        let unmounted: Vec<_> = devices.iter().filter(|d| d.mountpoint.is_none() && d.is_removable() && d.is_mountable_filesystem()).collect();

        assert_eq!(devices.len(), 4);
        assert_eq!(unmounted.len(), 1);
        assert_eq!(unmounted[0].display_name(), "STICK");
        assert_eq!(mount_point_from("Mounted /dev/sdb1 at /run/media/dave/STICK").as_deref(), Some("/run/media/dave/STICK"));
        assert_eq!(mount_point_from("nonsense"), None);
    }

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
