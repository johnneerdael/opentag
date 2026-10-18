use super::*;
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    fs,
    io::Write,
    rc::Rc,
};

// Real filesystem-backed provider; only SAF and its failure modes are substituted.
#[derive(Clone)]
struct Files(Rc<Backend>);
struct Backend {
    root: PathBuf,
    docs: RefCell<BTreeMap<String, DocumentInfo>>,
    failure: Cell<u8>,
    cancel: Cell<bool>,
    unavailable: Cell<bool>,
    workspace: PathBuf,
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn info(uri: &str, name: &str, writable: bool) -> DocumentInfo {
    DocumentInfo {
        uri: uri.into(),
        name: name.into(),
        mime_type: Some("audio/wav".into()),
        size: None,
        modified_ms: Some(1_700_000_000_000),
        writable,
        persisted: true,
    }
}
impl Files {
    fn path(&self, uri: &str) -> PathBuf {
        self.0
            .root
            .join(uri.strip_prefix("content://test/").unwrap())
    }
    fn result(&self, uri: &str) -> DocumentSnapshot {
        let bytes = fs::read(self.path(uri)).unwrap();
        DocumentSnapshot {
            document: self.0.docs.borrow()[uri].clone(),
            sha256: sha(&bytes),
            bytes: bytes.len() as u64,
        }
    }
}
impl Provider for Files {
    fn snapshot(&self, uri: &str, out: &Path) -> Result<DocumentSnapshot, String> {
        if self.0.unavailable.get() && uri == "content://test/source" {
            return Err("grant revoked".into());
        }
        let bytes = fs::read(self.path(uri)).map_err(|e| e.to_string())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes).unwrap();
        Ok(self.result(uri))
    }
    fn publish(&self, uri: &str, input: &Path, expected: &str) -> Result<DocumentSnapshot, String> {
        if self.0.unavailable.get() && uri == "content://test/source" {
            return Err("grant revoked".into());
        }
        if !self.0.docs.borrow()[uri].writable {
            return Err("read only".into());
        }
        if sha(&fs::read(self.path(uri)).unwrap()) != expected {
            return Err("source changed".into());
        }
        // At publication, a durable pending manifest must already exist.
        let pending = fs::read_dir(&self.0.workspace)
            .unwrap()
            .filter_map(Result::ok)
            .any(|dir| {
                fs::read(dir.path().join("manifest.json"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                    .is_some_and(|m| m["document"]["uri"] == uri && m["state"] == "pending")
            });
        assert!(pending, "provider contacted without pending manifest");
        let bytes = fs::read(input).unwrap();
        if self.0.failure.get() == 1 {
            fs::write(self.path(uri), &bytes[..40]).unwrap();
            return Err("partial provider write".into());
        }
        fs::write(self.path(uri), bytes).unwrap();
        if self.0.failure.get() == 2 {
            return Err("fresh read failed".into());
        }
        #[cfg(unix)]
        if self.0.failure.get() == 4 {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                input.parent().unwrap().parent().unwrap(),
                fs::Permissions::from_mode(0o555),
            )
            .unwrap();
        }
        let mut result = self.result(uri);
        if self.0.failure.get() == 3 {
            result.sha256 = "0".repeat(64);
        }
        Ok(result)
    }
    fn create(&self, name: &str, _mime: Option<&str>) -> Result<Option<DocumentInfo>, String> {
        if self.0.cancel.get() {
            return Ok(None);
        }
        let uri = format!("content://test/copy{}", self.0.docs.borrow().len());
        let d = info(&uri, name, true);
        fs::write(self.path(&uri), b"provider initial contents").unwrap();
        self.0.docs.borrow_mut().insert(uri, d.clone());
        Ok(Some(d))
    }
}
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    files: Files,
    store: Store<Files>,
    source: Vec<u8>,
}
impl Fixture {
    fn new(writable: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("private");
        fs::create_dir(&root).unwrap();
        let provider = dir.path().join("provider");
        fs::create_dir(&provider).unwrap();
        let source = wav();
        fs::write(provider.join("source"), &source).unwrap();
        let files = Files(Rc::new(Backend {
            root: provider,
            docs: RefCell::new(BTreeMap::from([(
                "content://test/source".into(),
                info("content://test/source", "音楽 – test.wav", writable),
            )])),
            failure: Cell::new(0),
            cancel: Cell::new(false),
            unavailable: Cell::new(false),
            workspace: root.clone(),
        }));
        let store = Store::open(&root, files.clone()).unwrap();
        Self {
            _dir: dir,
            root,
            files,
            store,
            source,
        }
    }
    fn import(&mut self) -> AudioEntry {
        self.store
            .import(self.files.0.docs.borrow()["content://test/source"].clone())
            .unwrap()
    }
    fn restart(&mut self) {
        self.store = Store::open(&self.root, self.files.clone()).unwrap();
    }
    fn backups_contain(&self, bytes: &[u8]) -> bool {
        fn visit(dir: &Path, bytes: &[u8]) -> bool {
            fs::read_dir(dir)
                .unwrap()
                .filter_map(Result::ok)
                .any(|item| {
                    if item.file_type().unwrap().is_dir() {
                        visit(&item.path(), bytes)
                    } else {
                        fs::read(item.path()).unwrap() == bytes
                    }
                })
        }
        visit(&self.root, bytes)
    }
}
fn wav() -> Vec<u8> {
    let samples: Vec<u8> = (0..8820).map(|i| (i % 251) as u8).collect();
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + samples.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(44100u32.to_le_bytes());
    bytes.extend(88200u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((samples.len() as u32).to_le_bytes());
    bytes.extend(samples);
    bytes
}
fn audio(bytes: &[u8]) -> &[u8] {
    let mut cursor = 12;
    while cursor + 8 <= bytes.len() {
        let len = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        if &bytes[cursor..cursor + 4] == b"data" {
            return &bytes[cursor + 8..cursor + 8 + len];
        }
        cursor += 8 + len + (len % 2);
    }
    panic!("missing WAV audio")
}
fn updates() -> Fields {
    BTreeMap::from([("TITLE".into(), vec!["Edited title".into()])])
}
fn edit(path: &Path) -> Result<AudioEntry, String> {
    tag_core::write_fields(path, &updates())
}

#[test]
fn successful_publication_preserves_real_wav_audio_and_projects_provider_context() {
    let mut f = Fixture::new(true);
    let entry = f.import();
    let edited = f.store.mutate(Path::new(&entry.path), edit).unwrap();
    assert_eq!(edited.fields["TITLE"], ["Edited title"]);
    assert_eq!(edited.filename, "音楽 – test.wav");
    assert_eq!(edited.technical["_PATH"], ["content://test/source"]);
    assert_eq!(edited.technical["_FILE_MOD_DATETIME_RAW"], ["1700000000"]);
    assert!(!edited.technical.contains_key("_FILE_CREATE_DATE"));
    assert!(!edited.technical.contains_key("_FOLDERPATH"));
    assert!(!edited.fields.contains_key("_PATH"));
    let provider = fs::read(f.files.path("content://test/source")).unwrap();
    assert_eq!(audio(&provider), audio(&f.source));
    assert_eq!(
        tag_core::read_entry(&f.files.path("content://test/source"))
            .unwrap()
            .fields["TITLE"],
        ["Edited title"]
    );
    assert_eq!(f.store.states()[&entry.path].state, "ready");
    f.restart();
    assert_eq!(f.store.states()[&entry.path].state, "ready");
    assert!(
        f.backups_contain(&f.source),
        "old verified baseline removed too soon"
    );
}
#[test]
fn same_size_same_time_source_change_is_rejected_and_both_versions_survive() {
    let mut f = Fixture::new(true);
    let entry = f.import();
    let source_path = f.files.path("content://test/source");
    let time = fs::metadata(&source_path).unwrap().modified().unwrap();
    let mut changed = f.source.clone();
    changed[50] ^= 1;
    fs::write(&source_path, &changed).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&source_path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(time))
        .unwrap();
    assert!(f
        .store
        .mutate(Path::new(&entry.path), edit)
        .unwrap_err()
        .contains("changed"));
    assert_eq!(fs::read(source_path).unwrap(), changed);
    assert!(f.backups_contain(&f.source));
    assert_eq!(
        tag_core::read_entry(Path::new(&entry.path)).unwrap().fields["TITLE"],
        ["Edited title"]
    );
    f.restart();
    assert_eq!(f.store.states()[&entry.path].state, "recovery_required");
}
#[test]
fn provider_failures_keep_original_and_edited_without_rollback_or_reselection_loss() {
    for failure in [1, 2, 3] {
        let mut f = Fixture::new(true);
        let entry = f.import();
        f.files.0.failure.set(failure);
        assert!(f.store.mutate(Path::new(&entry.path), edit).is_err());
        let provider = fs::read(f.files.path("content://test/source")).unwrap();
        assert_ne!(provider, f.source, "unknown provider was rolled back");
        let edited = fs::read(&entry.path).unwrap();
        assert_ne!(edited, f.source);
        assert!(f.backups_contain(&f.source));
        assert!(f.store.reload(Path::new(&entry.path)).is_err());
        let duplicate = f.import();
        assert_eq!(duplicate.path, entry.path);
        assert_eq!(fs::read(&entry.path).unwrap(), edited);
        f.restart();
        assert_eq!(f.store.states()[&entry.path].state, "recovery_required");
    }
}
#[test]
fn read_only_does_not_call_local_writer_but_edited_copy_applies_drafts() {
    let mut f = Fixture::new(false);
    let entry = f.import();
    assert!(f
        .store
        .mutate(Path::new(&entry.path), |_| panic!(
            "read-only writer called"
        ))
        .is_err());
    let result = f
        .store
        .save_copy(Path::new(&entry.path), &updates(), None, "edited")
        .unwrap();
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].fields["TITLE"], ["Edited title"]);
    assert_ne!(result.entries[0].path, entry.path);
    assert_eq!(fs::read(&entry.path).unwrap(), f.source);
    assert_eq!(
        fs::read(f.files.path("content://test/source")).unwrap(),
        f.source
    );
}
#[test]
fn original_copy_ignores_draft_and_export_does_not_clear_recovery() {
    let mut f = Fixture::new(true);
    let entry = f.import();
    f.files.0.failure.set(1);
    assert!(f.store.mutate(Path::new(&entry.path), edit).is_err());
    f.files.0.failure.set(0);
    let result = f
        .store
        .save_copy(Path::new(&entry.path), &updates(), None, "original")
        .unwrap();
    assert_eq!(fs::read(&result.entries[0].path).unwrap(), f.source);
    assert_eq!(f.store.states()[&entry.path].state, "recovery_required");
    f.files.0.cancel.set(true);
    let count = f.store.states().len();
    let cancelled = f
        .store
        .save_copy(Path::new(&entry.path), &updates(), None, "edited")
        .unwrap();
    assert!(cancelled.entries.is_empty() && cancelled.errors.is_empty());
    assert_eq!(f.store.states().len(), count);
}
#[test]
fn failed_copy_keeps_pending_artwork_and_fields_through_restart_and_fresh_copy() {
    let mut image = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(3, 2, image::Rgb([0, 255, 0])))
        .write_to(&mut image, image::ImageFormat::Png)
        .unwrap();
    let image = image.into_inner();
    let replacement = vec![tag_core::CoverChange {
        image: Some(image.clone()),
        picture_type: "CoverBack".into(),
        description: "Pending back".into(),
        all_files: true,
        ..Default::default()
    }];
    for remove_all in [false, true] {
        for failure in [1, 2, 3] {
            let mut f = Fixture::new(true);
            let source = f.files.path("content://test/source");
            tag_core::add_cover_bytes_with_options(
                &source,
                image.clone(),
                "Original",
                Some("CoverFront"),
                &tag_core::TagOptions::default(),
            )
            .unwrap();
            f.source = fs::read(&source).unwrap();
            let entry = f.import();
            let covers = if remove_all {
                &[][..]
            } else {
                &replacement[..]
            };
            f.files.0.failure.set(failure);
            assert!(f
                .store
                .save_copy(Path::new(&entry.path), &updates(), Some(covers), "edited")
                .is_err());
            assert_eq!(fs::read(&source).unwrap(), f.source);
            assert_eq!(fs::read(&entry.path).unwrap(), f.source);
            let state = f.store.states();
            let copy_path = state
                .keys()
                .find(|path| *path != &entry.path)
                .unwrap()
                .clone();
            assert_eq!(state[&copy_path].state, "recovery_required");
            let edited_bytes = fs::read(&copy_path).unwrap();
            assert_eq!(audio(&edited_bytes), audio(&f.source));
            f.files.0.unavailable.set(true);
            f.restart();
            assert_eq!(f.store.states()[&copy_path].state, "recovery_required");
            let recovered = f.store.read_working(Path::new(&copy_path)).unwrap();
            assert_eq!(recovered.fields["TITLE"], ["Edited title"]);
            assert_eq!(recovered.covers.len(), usize::from(!remove_all));
            assert_eq!(fs::read(&copy_path).unwrap(), edited_bytes);
            f.files.0.failure.set(0);
            let retry = f
                .store
                .save_copy(Path::new(&copy_path), &Fields::new(), None, "edited")
                .unwrap();
            let copied = tag_core::read_entry(Path::new(&retry.entries[0].path)).unwrap();
            assert_eq!(copied.fields["TITLE"], ["Edited title"]);
            assert_eq!(copied.covers.len(), usize::from(!remove_all));
            if !remove_all {
                assert_eq!(copied.covers[0].description, "Pending back");
                assert_eq!(copied.covers[0].picture_type, "CoverBack");
                use base64::Engine;
                assert_eq!(
                    base64::engine::general_purpose::STANDARD
                        .decode(&copied.covers[0].data_base64)
                        .unwrap(),
                    image
                );
            }
            assert_eq!(fs::read(&entry.path).unwrap(), f.source);
            assert_eq!(fs::read(&copy_path).unwrap(), edited_bytes);
            assert!(f.backups_contain(&f.source));
            assert_eq!(f.store.states()[&copy_path].state, "recovery_required");
        }
    }
}
#[test]
fn restart_detects_unrecorded_local_changes_and_restores_without_provider_access() {
    let mut f = Fixture::new(true);
    let entry = f.import();
    fs::write(&entry.path, b"interrupted local write").unwrap();
    f.files.0.unavailable.set(true);
    f.restart();
    assert_eq!(f.store.states()[&entry.path].state, "recovery_required");
    let result = f.store.restore_recovery();
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].path, entry.path);
    assert_eq!(fs::read(&entry.path).unwrap(), b"interrupted local write");
    assert!(!result.entries[0].fields.contains_key("TITLE"));
    assert!(f.store.states()[&entry.path].error.is_some());
}
#[test]
fn ready_reload_refreshes_source_and_duplicate_ready_reuses_handle() {
    let mut f = Fixture::new(true);
    let entry = f.import();
    let duplicate = f.import();
    assert_eq!(entry.path, duplicate.path);
    edit(&f.files.path("content://test/source")).unwrap();
    let refreshed = f.store.reload(Path::new(&entry.path)).unwrap();
    assert_eq!(refreshed.fields["TITLE"], ["Edited title"]);
    f.restart();
    assert_eq!(f.store.states()[&entry.path].state, "ready");
    assert!(f.store.restore_recovery().entries.is_empty());
}
#[test]
fn unsafe_provider_names_and_forged_manifest_paths_are_rejected() {
    for name in [
        "../escape.wav",
        "/tmp/escape.wav",
        "a\\b.wav",
        ".",
        "..",
        "",
        "manifest.json/evil",
    ] {
        let mut f = Fixture::new(true);
        assert!(
            f.store
                .import(info("content://test/source", name, true))
                .is_err(),
            "{name}"
        );
    }
    let mut f = Fixture::new(true);
    let entry = f.import();
    let manifest = Path::new(&entry.path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("manifest.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    json["baseline_leaf"] = serde_json::json!("../../provider/source");
    fs::write(manifest, serde_json::to_vec(&json).unwrap()).unwrap();
    assert!(Store::open(&f.root, f.files.clone()).is_err());
}
#[cfg(unix)]
#[test]
fn symlinked_working_file_never_mutates_provider_or_external_target() {
    let mut f = Fixture::new(true);
    let entry = f.import();
    fs::remove_file(&entry.path).unwrap();
    std::os::unix::fs::symlink(f.files.path("content://test/source"), &entry.path).unwrap();
    assert!(f
        .store
        .mutate(Path::new(&entry.path), |_| panic!("unsafe writer called"))
        .is_err());
    assert_eq!(
        fs::read(f.files.path("content://test/source")).unwrap(),
        f.source
    );
    assert!(Store::open(&f.root, f.files.clone()).is_err());
}

#[test]
fn pending_restart_is_recovery_even_when_working_bytes_match_baseline() {
    let mut f = Fixture::new(true);
    let entry = f.import();
    let manifest = Path::new(&entry.path)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("manifest.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    json["state"] = serde_json::json!("pending");
    fs::write(manifest, serde_json::to_vec(&json).unwrap()).unwrap();
    f.restart();
    assert_eq!(f.store.states()[&entry.path].state, "recovery_required");
    let duplicate = f.import();
    assert_eq!(duplicate.path, entry.path);
    assert!(f
        .store
        .mutate(Path::new(&entry.path), |_| panic!("pending mutation"))
        .is_err());
}
#[test]
fn duplicate_import_detects_modified_working_bytes_before_returning_ready() {
    let mut f = Fixture::new(true);
    let entry = f.import();
    edit(Path::new(&entry.path)).unwrap();
    let duplicate = f.import();
    assert_eq!(duplicate.fields["TITLE"], ["Edited title"]);
    assert_eq!(f.store.states()[&entry.path].state, "recovery_required");
}
#[test]
fn provider_unavailable_recovery_can_export_verified_backup_to_new_provider_document() {
    let mut f = Fixture::new(true);
    let entry = f.import();
    fs::write(&entry.path, b"invalid edited audio").unwrap();
    f.files.0.unavailable.set(true);
    f.restart();
    let restored = f.store.restore_recovery();
    assert_eq!(restored.entries[0].path, entry.path);
    let copied = f
        .store
        .save_copy(Path::new(&entry.path), &updates(), None, "original")
        .unwrap();
    assert_eq!(fs::read(&copied.entries[0].path).unwrap(), f.source);
    assert_eq!(fs::read(&entry.path).unwrap(), b"invalid edited audio");
    assert!(f
        .store
        .save_copy(Path::new(&entry.path), &updates(), None, "edited")
        .is_err());
}
#[cfg(unix)]
#[test]
fn failed_ready_manifest_after_provider_success_keeps_old_backup_until_restart() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = Fixture::new(true);
    let entry = f.import();
    f.files.0.failure.set(4);
    let result = f.store.mutate(Path::new(&entry.path), edit);
    let directory = Path::new(&entry.path).parent().unwrap().parent().unwrap();
    fs::set_permissions(directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err());
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["state"], "pending");
    assert_eq!(
        fs::read(
            directory
                .join("baselines")
                .join(manifest["baseline_leaf"].as_str().unwrap())
        )
        .unwrap(),
        f.source
    );
    assert_eq!(
        tag_core::read_entry(&f.files.path("content://test/source"))
            .unwrap()
            .fields["TITLE"],
        ["Edited title"]
    );
    f.restart();
    assert_eq!(f.store.states()[&entry.path].state, "recovery_required");
    f.files.0.failure.set(0);
    let copy = f
        .store
        .save_copy(Path::new(&entry.path), &Fields::new(), None, "original")
        .unwrap();
    assert_eq!(fs::read(&copy.entries[0].path).unwrap(), f.source);
}

#[test]
fn image_snapshot_is_private_exclusive_and_cannot_escape_via_input_symlink() {
    let f = Fixture::new(true);
    let png = vec![137, 80, 78, 71, 13, 10, 26, 10];
    let image = info("content://test/image", "art.png", false);
    f.files.0.docs.borrow_mut().insert(image.uri.clone(), image);
    fs::write(f.files.path("content://test/image"), &png).unwrap();
    let first = f.store.snapshot_input("content://test/image").unwrap();
    let second = f.store.snapshot_input("content://test/image").unwrap();
    assert_ne!(first, second);
    assert!(first.starts_with(dunce::canonicalize(&f.root).unwrap()));
    assert_eq!(fs::read(first).unwrap(), png);
    assert_eq!(fs::read(second).unwrap(), png);
    assert!(f.store.snapshot_input("file:///outside").is_err());
    #[cfg(unix)]
    {
        fs::rename(f.root.join("inputs"), f.root.join("saved-inputs")).unwrap();
        std::os::unix::fs::symlink(&f.files.0.root, f.root.join("inputs")).unwrap();
        assert!(f.store.snapshot_input("content://test/image").is_err());
        fs::remove_file(f.root.join("inputs")).unwrap();
    }
}
#[test]
fn projection_clears_unknown_provider_times_and_leaves_raw_fields_intact() {
    let mut f = Fixture::new(true);
    let mut entry = f.import();
    entry
        .fields
        .insert("_PATH".into(), vec!["literal stored custom tag".into()]);
    let raw = entry.fields.clone();
    let mut document = info("content://test/source", "new.name.wav", true);
    document.modified_ms = None;
    project_entry(&mut entry, &document);
    assert_eq!(entry.fields, raw);
    assert_eq!(entry.technical["_FILENAME"], ["new.name"]);
    assert!(!entry
        .technical
        .keys()
        .any(|key| key.starts_with("_FILE_MOD") || key.starts_with("_FILE_CREATE")));
    assert!(!entry
        .technical
        .values()
        .flatten()
        .any(|value| value.contains("private")));
}

#[test]
fn out_of_range_provider_timestamp_is_omitted_without_losing_audio() {
    let mut f = Fixture::new(true);
    f.files
        .0
        .docs
        .borrow_mut()
        .get_mut("content://test/source")
        .unwrap()
        .modified_ms = Some(i64::MAX);
    let entry = f.import();
    assert!(!entry
        .technical
        .keys()
        .any(|key| key.starts_with("_FILE_MOD")));
    assert_eq!(fs::read(entry.path).unwrap(), f.source);
}
