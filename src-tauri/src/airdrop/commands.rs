use super::{availability, error::AirDropError, require_transport, Availability};

#[tauri::command]
pub fn airdrop_availability() -> Availability {
    availability()
}

#[tauri::command]
pub fn discover_airdrop_devices() -> Result<(), AirDropError> {
    require_transport()
}

#[tauri::command]
pub fn send_airdrop_file(device_id: String, paths: Vec<String>) -> Result<(), AirDropError> {
    let _ = (device_id, paths);
    require_transport()
}

#[tauri::command]
pub fn accept_airdrop_transfer(transfer_id: String) -> Result<(), AirDropError> {
    let _ = transfer_id;
    require_transport()
}

#[tauri::command]
pub fn reject_airdrop_transfer(transfer_id: String) -> Result<(), AirDropError> {
    let _ = transfer_id;
    require_transport()
}

#[tauri::command]
pub fn cancel_airdrop_transfer(transfer_id: String) -> Result<(), AirDropError> {
    let _ = transfer_id;
    require_transport()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::airdrop::error::ErrorCode;

    #[test]
    fn every_network_entry_point_fails_closed() {
        assert!(!airdrop_availability().available);
        for result in [
            discover_airdrop_devices(),
            send_airdrop_file("untrusted".into(), vec!["/etc/passwd".into()]),
            accept_airdrop_transfer("unknown".into()),
            reject_airdrop_transfer("unknown".into()),
            cancel_airdrop_transfer("unknown".into()),
        ] {
            assert_eq!(result.unwrap_err().code, ErrorCode::TransportUnavailable);
        }
    }
}
