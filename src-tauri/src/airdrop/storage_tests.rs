use super::*;
use std::{fs, time::Duration};

fn offer(name: &str, size: u64) -> FileOffer {
    FileOffer {
        name: name.into(),
        size,
        expected_sha256: None,
    }
}
fn batch(root: &Path, offers: Vec<FileOffer>) -> ReceiveBatch {
    ReceiveBatch::new(
        root,
        offers,
        Arc::new(AtomicBool::new(false)),
        Instant::now() + Duration::from_secs(30),
    )
    .unwrap()
}
fn empty(root: &Path) {
    assert_eq!(fs::read_dir(root).unwrap().count(), 0);
}

#[test]
fn rejects_traversal_special_names_and_malformed_metadata() {
    for name in [
        "",
        ".",
        "..",
        "../file",
        "/etc/passwd",
        "a/b",
        "a\\b",
        "a\0b",
        "C:file",
        "a\nb",
        "\u{202e}txt",
    ] {
        assert!(validate_name(name).is_err(), "{name:?}");
    }
    assert!(validate_name(&"x".repeat(256)).is_err());
    assert!(serde_json::from_str::<FileOffer>(r#"{"name":"x","size":-1}"#).is_err());
    assert!(serde_json::from_str::<FileOffer>(r#"{"name":"x","size":1,"path":"/etc"}"#).is_err());
    assert!(validate_offer(&[offer("same", 0), offer("same", 0)]).is_err());
    assert!(validate_offer(&[offer("x", MAX_BYTES + 1)]).is_err());
    assert!(validate_offer(&vec![offer("x", 0); MAX_FILES + 1]).is_err());
}

#[test]
fn publishes_only_complete_batches_and_checks_known_digest() {
    let root = tempfile::tempdir().unwrap();
    let mut incoming = batch(root.path(), vec![offer("hello.txt", 3), offer("empty", 0)]);
    incoming.write(b"a").unwrap();
    incoming.write(b"bc").unwrap();
    let hash = incoming.finish_file().unwrap();
    assert_eq!(
        hash,
        [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad
        ]
    );
    incoming.finish_file().unwrap();
    incoming.publish("AirDrop files").unwrap();
    assert_eq!(
        fs::read(root.path().join("AirDrop files/hello.txt")).unwrap(),
        b"abc"
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn mismatch_partial_and_excess_data_are_never_published() {
    let root = tempfile::tempdir().unwrap();
    let mut wrong = offer("x", 1);
    wrong.expected_sha256 = Some([0; 32]);
    let mut incoming = batch(root.path(), vec![wrong]);
    incoming.write(b"x").unwrap();
    assert_eq!(
        incoming.finish_file().unwrap_err().code,
        ErrorCode::IntegrityMismatch
    );
    assert!(incoming.publish("failed").is_err());
    empty(root.path());
    let mut incoming = batch(root.path(), vec![offer("x", 2)]);
    incoming.write(b"x").unwrap();
    assert_eq!(
        incoming.finish_file().unwrap_err().code,
        ErrorCode::Incomplete
    );
    drop(incoming);
    empty(root.path());
    let mut incoming = batch(root.path(), vec![offer("x", 1)]);
    assert!(incoming.write(b"too much").is_err());
    assert!(incoming.publish("failed").is_err());
    empty(root.path());
}

#[test]
fn cancellation_and_timeout_clean_up_partial_output() {
    let root = tempfile::tempdir().unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let mut incoming = ReceiveBatch::new(
        root.path(),
        vec![offer("x", 2)],
        cancel.clone(),
        Instant::now() + Duration::from_secs(30),
    )
    .unwrap();
    incoming.write(b"x").unwrap();
    cancel.store(true, Ordering::Release);
    assert_eq!(incoming.write(b"x").unwrap_err().code, ErrorCode::Cancelled);
    drop(incoming);
    empty(root.path());
    let mut incoming = batch(root.path(), vec![offer("x", 0)]);
    incoming.deadline = Instant::now();
    assert_eq!(
        incoming.finish_file().unwrap_err().code,
        ErrorCode::TimedOut
    );
    drop(incoming);
    empty(root.path());
}

#[test]
fn concurrent_batches_never_overwrite_existing_destinations() {
    let root = tempfile::tempdir().unwrap();
    std::thread::scope(|scope| {
        let jobs: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let mut incoming = batch(root.path(), vec![offer("x", 1)]);
                    incoming.write(b"x").unwrap();
                    incoming.finish_file().unwrap();
                    incoming.publish("same")
                })
            })
            .collect();
        let results: Vec<_> = jobs.into_iter().map(|job| job.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert!(results
            .iter()
            .filter_map(|r| r.as_ref().err())
            .all(|e| e.code == ErrorCode::DestinationExists));
    });
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    assert_eq!(fs::read(root.path().join("same/x")).unwrap(), b"x");
}

#[test]
fn destination_symlink_swap_cannot_redirect_publication() {
    let root = tempfile::tempdir().unwrap();
    let downloads = root.path().join("Downloads");
    let moved = root.path().join("Moved");
    let outside = root.path().join("Outside");
    fs::create_dir(&downloads).unwrap();
    fs::create_dir(&outside).unwrap();
    let mut incoming = batch(&downloads, vec![offer("x", 0)]);
    fs::rename(&downloads, &moved).unwrap();
    std::os::unix::fs::symlink(&outside, &downloads).unwrap();
    incoming.finish_file().unwrap();
    incoming.publish("received").unwrap();
    empty(&outside);
    assert!(moved.join("received/x").is_file());
}

#[test]
fn destination_collision_with_symlink_is_not_followed() {
    let root = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink("/etc", root.path().join("received")).unwrap();
    let mut incoming = batch(root.path(), vec![offer("x", 0)]);
    incoming.finish_file().unwrap();
    assert_eq!(
        incoming.publish("received").unwrap_err().code,
        ErrorCode::DestinationExists
    );
    assert_eq!(
        fs::read_link(root.path().join("received")).unwrap(),
        PathBuf::from("/etc")
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}
