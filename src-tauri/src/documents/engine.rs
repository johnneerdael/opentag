//! Local persistence is separate from SAF transport so failure recovery is testable.
use super::DocumentState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
use tag_core::{AudioEntry, Fields, FileError, ScanResult};
use tauri_plugin_document_access::{DocumentInfo, DocumentSnapshot};
use uuid::Uuid;

type Result<T, E = String> = std::result::Result<T, E>;
pub(super) trait Provider {
    fn snapshot(&self, uri: &str, output: &Path) -> Result<DocumentSnapshot>;
    fn publish(&self, uri: &str, input: &Path, expected: &str) -> Result<DocumentSnapshot>;
    fn create(&self, name: &str, mime: Option<&str>) -> Result<Option<DocumentInfo>>;
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Ready,
    Pending,
    RecoveryRequired,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    id: String,
    working_name: String,
    document: DocumentInfo,
    baseline_leaf: String,
    baseline_sha256: String,
    state: Phase,
    error: Option<String>,
}
pub(super) struct Store<P> {
    root: PathBuf,
    provider: P,
    records: BTreeMap<String, Record>,
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn leaf(name: &str) -> Result<()> {
    let path = Path::new(name);
    if name.is_empty()
        || name.ends_with([' ', '.'])
        || name.chars().any(|c| c.is_control())
        || name.contains(['/', '\\', ':'])
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err("Document name must be a safe, nonempty leaf filename".into());
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix)
                .is_some_and(|n| matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        })
    {
        return Err("Document name is reserved on this platform".into());
    }
    Ok(())
}
fn identity(id: &str) -> Result<()> {
    if Uuid::parse_str(id).map_err(error)?.to_string() != id {
        return Err("Noncanonical document identity".into());
    }
    Ok(())
}
fn fingerprint(path: &Path) -> Result<(String, u64)> {
    let mut file = File::open(path).map_err(error)?;
    let mut hash = Sha256::new();
    let mut count = 0;
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(error)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
        count += n as u64;
    }
    Ok((format!("{:x}", hash.finalize()), count))
}
fn sync_dir(path: &Path) -> Result<()> {
    // Android and Unix require directory fsync after rename/create. Desktop
    // Windows only exercises the portable engine tests; SAF is Android-only.
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(error)?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
fn sync_file(path: &Path) -> Result<()> {
    OpenOptions::new()
        .write(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(error)
}
fn copy_new(source: &Path, target: &Path) -> Result<()> {
    let mut input = File::open(source).map_err(error)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(error)?;
    std::io::copy(&mut input, &mut output).map_err(error)?;
    output.sync_all().map_err(error)?;
    sync_dir(target.parent().ok_or("Missing private parent")?)
}
fn replace_from(source: &Path, target: &Path) -> Result<()> {
    let parent = target.parent().ok_or("Missing private parent")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    std::io::copy(&mut File::open(source).map_err(error)?, &mut temporary).map_err(error)?;
    temporary.as_file().sync_all().map_err(error)?;
    temporary.persist(target).map_err(error)?;
    sync_dir(parent)
}
impl<P: Provider> Store<P> {
    pub(super) fn open(root: &Path, provider: P) -> Result<Self> {
        fs::create_dir_all(root).map_err(error)?;
        if fs::symlink_metadata(root)
            .map_err(error)?
            .file_type()
            .is_symlink()
        {
            return Err("Private workspace root is a symlink".into());
        }
        let root = dunce::canonicalize(root).map_err(error)?;
        sync_dir(root.parent().ok_or("Private workspace has no parent")?)?;
        let mut store = Self {
            root,
            provider,
            records: BTreeMap::new(),
        };
        for item in fs::read_dir(&store.root).map_err(error)? {
            let item = item.map_err(error)?;
            let path = item.path();
            store.checked(&path, false)?;
            if !item.file_type().map_err(error)?.is_dir() {
                return Err("Unexpected private workspace file".into());
            }
            if item.file_name() == "inputs" {
                continue;
            }
            let id = item
                .file_name()
                .into_string()
                .map_err(|_| "Invalid record identity")?;
            identity(&id)?;
            let manifest = path.join("manifest.json");
            store.checked(&manifest, true)?;
            if !manifest.exists() {
                continue;
            } // incomplete import: no provider mutation occurred
            let mut record: Record =
                serde_json::from_slice(&fs::read(&manifest).map_err(error)?).map_err(error)?;
            if record.version != 1 || record.id != id {
                return Err("Invalid document manifest identity/version".into());
            }
            leaf(&record.working_name)?;
            leaf(&record.document.name)?;
            identity(&record.baseline_leaf)?;
            store.checked(&store.baseline(&record), true)?;
            let work = store.working(&record);
            store.checked(&work, true)?;
            if store
                .records
                .values()
                .any(|r| r.document.uri == record.document.uri)
            {
                return Err("Duplicate document URI in manifests".into());
            }
            let baseline_ok = store.valid_baseline(&record).is_ok();
            let working_ok =
                fingerprint(&work).is_ok_and(|(hash, _)| hash == record.baseline_sha256);
            if !matches!(record.state, Phase::Ready) || !baseline_ok || !working_ok {
                record.state = Phase::RecoveryRequired;
                record.error.get_or_insert_with(|| {
                    "Interrupted operation or changed working copy; save a copy to recover".into()
                });
                store.persist(&record)?;
            }
            store
                .records
                .insert(work.to_string_lossy().into_owned(), record);
        }
        Ok(store)
    }
    /// Validate every existing component below the canonical private root.
    /// The caller owns this tree; no adversarial concurrent ancestor renames are claimed.
    fn checked(&self, path: &Path, missing_leaf: bool) -> Result<()> {
        let relative = path
            .strip_prefix(&self.root)
            .map_err(|_| "Path escapes private workspace")?;
        let mut current = self.root.clone();
        let count = relative.components().count();
        for (index, component) in relative.components().enumerate() {
            if !matches!(component, Component::Normal(_)) {
                return Err("Unsafe private path component".into());
            }
            current.push(component);
            match fs::symlink_metadata(&current) {
                Ok(meta) => {
                    if meta.file_type().is_symlink() || (!meta.is_dir() && !meta.is_file()) {
                        return Err("Unsafe private path (symlink or special file)".into());
                    }
                    if index + 1 < count && !meta.is_dir() {
                        return Err("Private parent is not a directory".into());
                    }
                    if dunce::canonicalize(&current).map_err(error)? != current {
                        return Err("Noncanonical private path".into());
                    }
                }
                Err(e)
                    if missing_leaf
                        && index + 1 == count
                        && e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(error(e)),
            }
        }
        Ok(())
    }
    fn directory(&self, record: &Record) -> PathBuf {
        self.root.join(&record.id)
    }
    fn working(&self, record: &Record) -> PathBuf {
        self.directory(record)
            .join("working")
            .join(&record.working_name)
    }
    fn baseline(&self, record: &Record) -> PathBuf {
        self.directory(record)
            .join("baselines")
            .join(&record.baseline_leaf)
    }
    fn valid_baseline(&self, record: &Record) -> Result<PathBuf> {
        let path = self.baseline(record);
        self.checked(&path, false)?;
        if fingerprint(&path)?.0 != record.baseline_sha256 {
            return Err("Verified backup is missing or changed".into());
        }
        Ok(path)
    }
    fn persist(&self, record: &Record) -> Result<()> {
        let directory = self.directory(record);
        self.checked(&directory, false)?;
        let manifest = directory.join("manifest.json");
        self.checked(&manifest, true)?;
        let mut temporary = tempfile::NamedTempFile::new_in(&directory).map_err(error)?;
        serde_json::to_writer(&mut temporary, record).map_err(error)?;
        temporary.flush().map_err(error)?;
        temporary.as_file().sync_all().map_err(error)?;
        temporary.persist(manifest).map_err(error)?;
        sync_dir(&directory)
    }
    fn commit(&mut self, record: Record) -> Result<()> {
        self.persist(&record)?;
        self.records
            .insert(self.working(&record).to_string_lossy().into_owned(), record);
        Ok(())
    }
    fn record(&self, path: &Path) -> Result<Record> {
        let record = self
            .records
            .get(path.to_str().ok_or("Invalid working path")?)
            .ok_or("Not a managed document")?;
        self.checked(&self.working(record), true)?;
        Ok(record.clone())
    }
    fn recover<T>(&mut self, mut record: Record, reason: String) -> Result<T> {
        record.state = Phase::RecoveryRequired;
        record.error = Some(reason.clone());
        // Keep the recovery state in memory even if storage is failing. A prior
        // pending manifest or working/baseline hash mismatch also detects restart.
        self.records.insert(
            self.working(&record).to_string_lossy().into_owned(),
            record.clone(),
        );
        match self.persist(&record) {
            Ok(()) => Err(reason),
            Err(e) => Err(format!("{reason}; could not persist recovery state: {e}")),
        }
    }
    fn ready(&mut self, path: &Path, writable: bool) -> Result<Record> {
        let record = self.record(path)?;
        if !matches!(record.state, Phase::Ready) {
            return Err("Document requires recovery; save a copy before further editing".into());
        }
        if writable && !record.document.writable {
            return Err("Read-only document; use Save a copy".into());
        }
        if self.valid_baseline(&record).is_err()
            || !fingerprint(path).is_ok_and(|(hash, _)| hash == record.baseline_sha256)
        {
            return self.recover(
                record,
                "Working copy or backup changed; recovery required".into(),
            );
        }
        Ok(record)
    }
    fn verify(&self, result: &DocumentSnapshot, uri: &str, path: &Path) -> Result<()> {
        self.checked(path, false)?;
        leaf(&result.document.name)?;
        let (hash, bytes) = fingerprint(path)?;
        if result.document.uri != uri || result.sha256 != hash || result.bytes != bytes {
            return Err("Provider snapshot/publication verification mismatch".into());
        }
        Ok(())
    }
    fn allocate(&self, document: DocumentInfo) -> Result<Record> {
        leaf(&document.name)?;
        if !document.uri.starts_with("content://") {
            return Err("Expected a provider content URI".into());
        }
        let id = Uuid::new_v4().to_string();
        let directory = self.root.join(&id);
        fs::create_dir(&directory).map_err(error)?;
        for child in ["working", "baselines"] {
            fs::create_dir(directory.join(child)).map_err(error)?;
        }
        sync_dir(&directory)?;
        sync_dir(&self.root)?;
        let mut record = Record {
            version: 1,
            id,
            working_name: document.name.clone(),
            document,
            baseline_leaf: Uuid::new_v4().to_string(),
            baseline_sha256: String::new(),
            state: Phase::Ready,
            error: None,
        };
        let baseline = self.baseline(&record);
        let snapshot = self.provider.snapshot(&record.document.uri, &baseline)?;
        self.verify(&snapshot, &record.document.uri, &baseline)?;
        sync_file(&baseline)?;
        sync_dir(baseline.parent().ok_or("Missing baseline parent")?)?;
        record.baseline_sha256 = snapshot.sha256;
        record.document = snapshot.document;
        copy_new(&baseline, &self.working(&record))?;
        Ok(record)
    }
    fn entry(&self, record: &Record) -> Result<AudioEntry> {
        let working = self.working(record);
        self.checked(&working, true)?;
        let mut entry = tag_core::read_entry(&working).or_else(|working_error| {
            if matches!(record.state, Phase::Ready) {
                return Err(working_error);
            }
            tag_core::read_entry(&self.valid_baseline(record)?).map_err(|e| {
                format!("Edited copy could not be read ({working_error}); backup: {e}")
            })
        })?;
        entry.path = working.to_string_lossy().into_owned();
        project_entry(&mut entry, &record.document);
        Ok(entry)
    }
    pub(super) fn import(&mut self, document: DocumentInfo) -> Result<AudioEntry> {
        if let Some(record) = self
            .records
            .values()
            .find(|r| r.document.uri == document.uri)
            .cloned()
        {
            let path = self.working(&record);
            if matches!(record.state, Phase::Ready) {
                let _ = self.ready(&path, false);
            }
            return self.entry(&self.record(&path)?);
        }
        let record = self.allocate(document)?;
        let entry = self.entry(&record)?;
        self.commit(record)?;
        Ok(entry)
    }
    fn publish(&mut self, mut record: Record) -> Result<AudioEntry> {
        let working = self.working(&record);
        let result = (|| {
            self.checked(&working, false)?;
            sync_file(&working)?;
            sync_dir(working.parent().ok_or("Missing working parent")?)?;
            // Pending was committed before mutation; commit again after edited
            // bytes are durable, before any provider output stream is opened.
            self.persist(&record)?;
            let snapshot =
                self.provider
                    .publish(&record.document.uri, &working, &record.baseline_sha256)?;
            self.verify(&snapshot, &record.document.uri, &working)?;
            // Never overwrite the old baseline. If this copy or the final ready
            // manifest fails, the previous manifest still names the old backup.
            let mut ready = record.clone();
            ready.baseline_leaf = Uuid::new_v4().to_string();
            let next = self.baseline(&ready);
            self.checked(&next, true)?;
            copy_new(&working, &next)?;
            ready.baseline_sha256 = snapshot.sha256;
            ready.document = snapshot.document;
            ready.state = Phase::Ready;
            ready.error = None;
            let entry = self.entry(&ready)?;
            self.commit(ready)?;
            Ok(entry)
        })();
        match result {
            Ok(entry) => Ok(entry),
            Err(e) => {
                record.state = Phase::RecoveryRequired;
                self.recover(record, e)
            }
        }
    }
    pub(super) fn mutate(
        &mut self,
        path: &Path,
        work: impl FnOnce(&Path) -> Result<AudioEntry>,
    ) -> Result<AudioEntry> {
        let mut record = self.ready(path, true)?;
        record.state = Phase::Pending;
        self.commit(record.clone())?;
        if let Err(e) = work(path) {
            return self.recover(record, e);
        }
        self.publish(record)
    }
    // Read only the retained edited bytes. Recovery presentation must never
    // substitute a baseline or contact the provider to describe an edited copy.
    pub(super) fn read_working(&self, path: &Path) -> Result<AudioEntry> {
        let record = self.record(path)?;
        let working = self.working(&record);
        self.checked(&working, false)?;
        let mut entry = tag_core::read_entry_summary(&working)?;
        project_entry(&mut entry, &record.document);
        Ok(entry)
    }
    pub(super) fn reload(&mut self, path: &Path) -> Result<AudioEntry> {
        let mut record = self.ready(path, false)?;
        // A failed read leaves the current ready working copy intact.
        let snapshot_path = self
            .directory(&record)
            .join("baselines")
            .join(Uuid::new_v4().to_string());
        self.checked(&snapshot_path, true)?;
        let snapshot = self
            .provider
            .snapshot(&record.document.uri, &snapshot_path)?;
        self.verify(&snapshot, &record.document.uri, &snapshot_path)?;
        tag_core::read_entry(&snapshot_path)?;
        sync_file(&snapshot_path)?;
        sync_dir(snapshot_path.parent().ok_or("Missing baseline parent")?)?;
        record.state = Phase::Pending;
        self.commit(record.clone())?;
        let result = (|| {
            self.checked(path, false)?;
            replace_from(&snapshot_path, path)?;
            let mut ready = record.clone();
            ready.baseline_leaf = snapshot_path
                .file_name()
                .ok_or("Missing snapshot name")?
                .to_string_lossy()
                .into_owned();
            ready.baseline_sha256 = snapshot.sha256;
            ready.document = snapshot.document;
            ready.state = Phase::Ready;
            ready.error = None;
            let entry = self.entry(&ready)?;
            self.commit(ready)?;
            Ok(entry)
        })();
        match result {
            Ok(entry) => Ok(entry),
            Err(e) => self.recover(record, e),
        }
    }
    pub(super) fn save_copy(
        &mut self,
        path: &Path,
        updates: &Fields,
        covers: Option<&[tag_core::CoverChange]>,
        version: &str,
    ) -> Result<ScanResult> {
        let source = self.record(path)?;
        let input = match version {
            "edited" => {
                let path = self.working(&source);
                self.checked(&path, false)?;
                tag_core::read_entry(&path)?;
                path
            }
            "original" => self.valid_baseline(&source)?,
            _ => return Err("Copy version must be edited or original".into()),
        };
        let Some(document) = self
            .provider
            .create(&source.document.name, source.document.mime_type.as_deref())?
        else {
            return Ok(ScanResult::default());
        };
        if self
            .records
            .values()
            .any(|r| r.document.uri == document.uri)
        {
            return Err("Copy destination is an existing managed document".into());
        }
        if !document.writable {
            return Err("Copy destination is not writable".into());
        }
        // CREATE_DOCUMENT contents are provider-defined: snapshot and hash them
        // before publishing, even when the freshly created file is nonempty.
        let mut record = self.allocate(document)?;
        record.state = Phase::Pending;
        self.commit(record.clone())?;
        let working = self.working(&record);
        if let Err(e) = (|| {
            self.checked(&input, false)?;
            self.checked(&working, false)?;
            replace_from(&input, &working)?;
            if version == "edited" {
                if let Some(covers) = covers {
                    tag_core::apply_cover_changes_with_options(
                        &working,
                        covers,
                        &tag_core::TagOptions::default(),
                    )?;
                }
                if !updates.is_empty() {
                    tag_core::write_fields(&working, updates)?;
                }
            }
            Ok(())
        })() {
            return self.recover(record, e);
        }
        let entry = self.publish(record)?;
        Ok(ScanResult {
            entries: vec![entry],
            errors: vec![],
        })
    }
    pub(super) fn restore_recovery(&self) -> ScanResult {
        let mut result = ScanResult::default();
        for record in self
            .records
            .values()
            .filter(|r| !matches!(r.state, Phase::Ready))
        {
            match self.entry(record) {
                Ok(entry) => result.entries.push(entry),
                Err(error) => result.errors.push(FileError {
                    path: self.working(record).to_string_lossy().into_owned(),
                    error,
                }),
            }
        }
        result
    }
    pub(super) fn states(&self) -> BTreeMap<String, DocumentState> {
        self.records
            .iter()
            .map(|(path, r)| {
                (
                    path.clone(),
                    DocumentState {
                        uri: r.document.uri.clone(),
                        name: r.document.name.clone(),
                        writable: r.document.writable,
                        persisted: r.document.persisted,
                        state: if matches!(r.state, Phase::Ready) {
                            "ready"
                        } else {
                            "recovery_required"
                        }
                        .into(),
                        error: r.error.clone(),
                    },
                )
            })
            .collect()
    }
    pub(super) fn is_managed(&self, path: &Path) -> bool {
        path.to_str().is_some_and(|p| self.records.contains_key(p))
            || dunce::canonicalize(path)
                .ok()
                .and_then(|p| p.to_str().map(str::to_owned))
                .is_some_and(|p| self.records.contains_key(&p))
    }
    pub(super) fn project(&self, entry: &mut AudioEntry) {
        if let Some(record) = self.records.get(&entry.path) {
            project_entry(entry, &record.document);
        }
    }
    pub(super) fn snapshot_input(&self, uri: &str) -> Result<PathBuf> {
        self.snapshot_input_with_name(uri).map(|(path, _)| path)
    }
    pub(super) fn snapshot_input_with_name(&self, uri: &str) -> Result<(PathBuf, String)> {
        if !uri.starts_with("content://") {
            return Err("Mobile artwork input must be a provider content URI".into());
        }
        let inputs = self.root.join("inputs");
        self.checked(&inputs, true)?;
        if !inputs.exists() {
            fs::create_dir(&inputs).map_err(error)?;
            sync_dir(&self.root)?;
        }
        let output = inputs.join(Uuid::new_v4().to_string());
        self.checked(&output, true)?;
        let snapshot = self.provider.snapshot(uri, &output)?;
        self.verify(&snapshot, uri, &output)?;
        Ok((output, snapshot.document.name))
    }
}
fn project_entry(entry: &mut AudioEntry, document: &DocumentInfo) {
    entry.filename = document.name.clone();
    entry.technical.retain(|key, _| {
        !matches!(
            key.as_str(),
            "_PATH"
                | "_FILENAME"
                | "_FILENAME_EXT"
                | "_EXTENSION"
                | "_FOLDERPATH"
                | "_FOLDERPATH_REL"
                | "_DIRECTORY"
                | "_PARENT_DIRECTORY"
        ) && !key.starts_with("_FILE_MOD")
            && !key.starts_with("_FILE_CREATE")
    });
    let path = Path::new(&document.name);
    for (key, value) in [
        ("_PATH", document.uri.clone()),
        ("_FILENAME_EXT", document.name.clone()),
        (
            "_FILENAME",
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "_EXTENSION",
            path.extension()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        ),
    ] {
        entry.technical.insert(key.into(), vec![value]);
    }
    let modified = document.modified_ms.and_then(|millis| {
        let duration = std::time::Duration::from_millis(millis.unsigned_abs());
        if millis >= 0 {
            std::time::UNIX_EPOCH.checked_add(duration)
        } else {
            std::time::UNIX_EPOCH.checked_sub(duration)
        }
    });
    entry
        .technical
        .extend(tag_core::file_time_fields(None, modified));
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
