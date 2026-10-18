//! Explicit, source-bound setup import. Audio and credential files are never written.
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const LIMIT: u64 = 128 * 1024 * 1024;
const ENTRIES: usize = 4096;
const DEPTH: usize = 64;
const STORE: &str = ".profile-migration";
const NATIVE: [&str; 7] = [
    "settings.json",
    "actions.json",
    "exports.json",
    "columns.json",
    "tag-panel.json",
    "directories.json",
    "lists.json",
];
type Files = BTreeMap<String, Vec<u8>>;
type Encoded = BTreeMap<String, String>;

pub const PREFERENCE_KEYS: &[&str] = &[
    "opentag.language",
    "opentag.appearance",
    "opentag.main-layout",
    "opentag.panel-width",
    "opentag.filter-auto-apply",
    "opentag.filter-shown",
    "opentag.numbering",
    "opentag.actions.quick",
    "opentag.cover.aspect",
    "opentag.cover.extractFormat",
    "opentag.tagSources.lastSource",
    "opentag.tagSources.last",
    "opentag.tagSources.saveCover",
    "opentag.tagSources.suggestSorting",
    "opentag.last-version",
    "opentag.tagSources.disabled.musicbrainz",
    "opentag.tagSources.disabled.discogs",
    "opentag.tagSources.disabled.itunes",
    "opentag.tagSources.disabled.deezer",
    "opentag.tagSources.disabled.lastfm",
    "opentag.tagSources.disabled.fanart",
    "opentag.tagSources.disabled.lrclib",
    "opentag.tagSources.disabled.acoustid",
    "opentag.tagSources.disabled.freedb",
    "opentag.tagSources.disabled.local_freedb",
    "opentag.tagSources.disabled.web",
];

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub paths: Vec<String>,
    pub selected: Vec<String>,
    pub roots: Vec<String>,
    pub location: String,
    pub location_is_data: bool,
    pub filter: String,
    pub filter_draft: Option<String>,
    pub anchor: Option<String>,
    pub playlist: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WindowState {
    pub version: u32,
    pub preferences: BTreeMap<String, String>,
    pub workspace: Workspace,
}

fn validate_window(window: &WindowState) -> Result<(), String> {
    if window.version != 1
        || window
            .preferences
            .keys()
            .any(|key| !PREFERENCE_KEYS.contains(&key.as_str()))
    {
        return Err("Unsupported setup window-state version or preference key.".into());
    }
    let workspace = &window.workspace;
    if workspace.paths.len() > 10000 || workspace.roots.len() > 10000 {
        return Err("Setup workspace recovery supports at most 10000 listed files and 10000 roots. Save the larger file list as a playlist, clear it, then inspect the setup again; no files are truncated.".into());
    }
    let listed = workspace.paths.iter().collect::<BTreeSet<_>>();
    if workspace.selected.len() > workspace.paths.len()
        || listed.len() != workspace.paths.len()
        || workspace.selected.iter().collect::<BTreeSet<_>>().len() != workspace.selected.len()
        || workspace.selected.iter().any(|path| !listed.contains(path))
        || workspace
            .anchor
            .as_ref()
            .is_some_and(|path| !listed.contains(path))
    {
        return Err("The setup workspace has invalid file identities.".into());
    }
    for path in workspace
        .paths
        .iter()
        .chain(&workspace.roots)
        .chain(workspace.playlist.iter())
    {
        if path.is_empty()
            || path.contains('\0')
            || path.contains("://")
            || !Path::new(path).is_absolute()
        {
            return Err("Setup workspace recovery requires absolute local file paths.".into());
        }
    }
    if serde_json::to_vec(window).map_err(|e| e.to_string())?.len() > 2 * 1024 * 1024 {
        return Err("The setup window state exceeds 2 MB.".into());
    }
    Ok(())
}

fn relative(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.split('/').count() > DEPTH
        || name.contains(['\\', ':', '\0'])
        || name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!("Unsafe setup path: {name}"));
    }
    Ok(())
}
fn managed(name: &str) -> bool {
    NATIVE.contains(&name) || name.starts_with("export/")
}
fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(format!(
            "{} must be a bounded regular file without symbolic links.",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("The setup file exceeded its read limit.".into());
    }
    Ok(bytes)
}
fn bounded(files: &Files) -> Result<(), String> {
    if files.len() > ENTRIES || files.values().map(|bytes| bytes.len() as u64).sum::<u64>() > LIMIT
    {
        return Err("Setup files exceed 128 MB or 4096 entries.".into());
    }
    Ok(())
}
fn walk(
    root: &Path,
    prefix: &str,
    depth: usize,
    files: &mut Files,
    count: &mut usize,
) -> Result<(), String> {
    if depth > DEPTH {
        return Err("Setup folders exceed 64 levels.".into());
    }
    for entry in std::fs::read_dir(root).map_err(|e| format!("{}: {e}", root.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        *count += 1;
        if *count > ENTRIES {
            return Err("Setup files exceed 128 MB or 4096 entries.".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Setup file names must be Unicode.")?;
        let name = format!("{prefix}{name}");
        relative(&name)?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            walk(&entry.path(), &format!("{name}/"), depth + 1, files, count)?;
        } else {
            files.insert(name, read(&entry.path(), LIMIT)?);
            bounded(files)?;
        }
    }
    Ok(())
}
fn input(path: &Path) -> Result<Files, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    let mut files = Files::new();
    if metadata.is_dir() {
        walk(path, "", 0, &mut files, &mut 0)?;
    } else {
        let bytes = read(path, LIMIT)?;
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
            .map_err(|e| format!("Choose a setup folder or ZIP: {e}"))?;
        if zip.len() > ENTRIES {
            return Err("Setup files exceed 128 MB or 4096 entries.".into());
        }
        let mut seen = BTreeSet::new();
        for index in 0..zip.len() {
            let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
            let name = entry.name().replace('\\', "/");
            let name = name.trim_end_matches('/');
            relative(name)?;
            if !seen.insert(name.to_lowercase()) {
                return Err(format!("Duplicate setup ZIP path: {name}"));
            }
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(format!("Setup ZIP contains a symbolic link: {name}"));
            }
            if entry.is_dir() {
                continue;
            }
            if entry.size() > LIMIT {
                return Err("Setup files exceed 128 MB or 4096 entries.".into());
            }
            let mut bytes = Vec::new();
            (&mut entry)
                .take(LIMIT + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            files.insert(name.into(), bytes);
            bounded(&files)?;
        }
    }
    let mut names = BTreeSet::new();
    for name in files.keys() {
        if !names.insert(name.to_lowercase()) {
            return Err(format!("Duplicate setup path ignoring case: {name}"));
        }
    }
    if files.is_empty() {
        return Err("The setup contains no files.".into());
    }
    Ok(files)
}
fn native(directory: &Path) -> Result<Files, String> {
    let mut files = Files::new();
    for name in NATIVE {
        match std::fs::symlink_metadata(directory.join(name)) {
            Ok(_) => {
                files.insert(name.into(), read(&directory.join(name), LIMIT)?);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    let export = directory.join("export");
    match std::fs::symlink_metadata(&export) {
        Ok(metadata) => {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err("The export folder must be a real directory.".into());
            }
            walk(&export, "export/", 0, &mut files, &mut 0)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    bounded(&files)?;
    Ok(files)
}
fn hash(files: &Files, window: Option<&WindowState>) -> Result<String, String> {
    let mut digest = Sha256::new();
    for (name, bytes) in files {
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    if let Some(window) = window {
        digest.update(serde_json::to_vec(window).map_err(|e| e.to_string())?);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn encode(files: &Files) -> Encoded {
    files
        .iter()
        .map(|(name, bytes)| {
            (
                name.clone(),
                base64::engine::general_purpose::STANDARD.encode(bytes),
            )
        })
        .collect()
}
fn decode(files: &Encoded) -> Result<Files, String> {
    let mut decoded = Files::new();
    for (name, encoded) in files {
        relative(name)?;
        if !managed(name) {
            return Err(format!("The setup snapshot cannot write {name}."));
        }
        decoded.insert(
            name.clone(),
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| e.to_string())?,
        );
        bounded(&decoded)?;
    }
    Ok(decoded)
}
fn write_stage(root: &Path, files: &Files) -> Result<(), String> {
    for (name, bytes) in files {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Clone)]
enum Candidate {
    Columns(crate::columns::Layout),
    Panel(crate::tag_panel::Layout),
    Actions(tag_core::actions::ActionGroup),
    Export(crate::exports::Configuration),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub file: String,
    pub kind: String,
    pub name: String,
    pub warnings: Vec<String>,
    pub collision: bool,
    pub supported: bool,
}
#[derive(Clone, PartialEq, Eq)]
struct SourceIdentity {
    canonical: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}
// Resolve future configuration directories without creating them. Existing ancestors may
// contain symlinks, so canonicalize before appending the missing path components.
fn canonical_target(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    let mut canonical = loop {
        match std::fs::canonicalize(ancestor) {
            Ok(path) => break path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(
                    ancestor
                        .file_name()
                        .ok_or("Cannot resolve the setup target location.")?
                        .to_os_string(),
                );
                ancestor = ancestor
                    .parent()
                    .ok_or("Cannot resolve the setup target location.")?;
            }
            Err(error) => return Err(error.to_string()),
        }
    };
    for component in missing.into_iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}
fn source_identity(directory: &Path, path: &Path) -> Result<SourceIdentity, String> {
    let canonical = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
    let target = canonical_target(directory)?;
    if canonical.starts_with(&target) || target.starts_with(&canonical) {
        return Err("Choose a profile outside the TagGroove configuration folder. Copy the profile folder or ZIP to a separate location that neither contains nor is inside the configuration folder, then inspect that copy.".into());
    }
    #[cfg(unix)]
    let metadata = std::fs::metadata(&canonical).map_err(|error| error.to_string())?;
    Ok(SourceIdentity {
        canonical,
        #[cfg(unix)]
        device: {
            use std::os::unix::fs::MetadataExt;
            metadata.dev()
        },
        #[cfg(unix)]
        inode: {
            use std::os::unix::fs::MetadataExt;
            metadata.ino()
        },
    })
}
#[derive(Clone)]
struct Plan {
    path: String,
    source_identity: SourceIdentity,
    source_hash: String,
    target_hash: String,
    legacy_encoding: Option<String>,
    items: Vec<Item>,
    candidates: BTreeMap<String, Candidate>,
    window: WindowState,
}
#[derive(Serialize)]
pub struct Preview {
    pub token: String,
    pub items: Vec<Item>,
    pub source_hash: String,
}
static PLANS: OnceLock<Mutex<BTreeMap<String, Plan>>> = OnceLock::new();

fn inspect(
    directory: &Path,
    source: &Files,
    legacy_encoding: Option<&str>,
) -> Result<(Vec<Item>, BTreeMap<String, Candidate>), String> {
    let extracted = tempfile::tempdir().map_err(|e| e.to_string())?;
    write_stage(extracted.path(), source)?;
    let actions = crate::action_host::load_library(&directory.join("actions.json"))?;
    let exports = crate::exports::load(&directory.join("exports.json"))?;
    let mut items = Vec::new();
    let mut candidates = BTreeMap::new();
    for (file, bytes) in source {
        let lower = file.to_lowercase();
        let name = Path::new(file)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut item = Item {
            file: file.clone(),
            kind: "unsupported".into(),
            name,
            warnings: Vec::new(),
            collision: false,
            supported: false,
        };
        let result: Result<Option<Candidate>, String> = (|| {
            if matches!(lower.rsplit('.').next(), Some("ini" | "mta" | "mte"))
                && bytes.len() > 4 * 1024 * 1024
            {
                return Err("Supported setup files must be at most 4 MB each.".into());
            }
            if lower.rsplit('/').next() == Some("columns.ini")
                || (lower.split('/').any(|part| part == "columns") && lower.ends_with(".ini"))
            {
                item.kind = "columns".into();
                item.collision = directory.join("columns.json").exists();
                let parsed =
                    crate::mp3tag_columns::parse_diagnostic(&crate::mp3tag_columns::decode(bytes))
                        .map_err(|e| e.to_string())?;
                let defined =
                    crate::mp3tag_columns::sections(&crate::mp3tag_columns::decode(bytes))
                        .into_iter()
                        .filter(|(_, keys)| {
                            crate::mp3tag_columns::lookup(
                                keys,
                                &["value", "format", "fmt", "display format"],
                            )
                            .is_some_and(|value| !value.is_empty())
                        })
                        .count();
                let omitted = defined.saturating_sub(
                    parsed.result.skipped.len() + parsed.result.layout.columns.len(),
                );
                item.warnings = parsed.result.skipped;
                if omitted > 0 {
                    item.warnings.push(format!(
                        "Additional supported columns omitted after the limit of 200: {omitted}."
                    ));
                }
                Ok(Some(Candidate::Columns(parsed.result.layout)))
            } else if lower.rsplit('/').next() == Some("usrfields.ini")
                || (lower.split('/').any(|part| part == "panels") && lower.ends_with(".ini"))
            {
                item.kind = "panel".into();
                item.collision = directory.join("tag-panel.json").exists();
                let parsed = crate::mp3tag_tag_panel::parse_diagnostic(
                    &crate::mp3tag_columns::decode(bytes),
                )
                .map_err(|e| e.to_string())?;
                item.warnings = parsed.skipped;
                Ok(Some(Candidate::Panel(parsed.panel)))
            } else if lower.ends_with(".mta") {
                item.kind = "actions".into();
                let mut parsed = crate::action_host::import_group_diagnostic(
                    &extracted.path().join(file),
                    legacy_encoding,
                )
                .map_err(|e| e.to_string())?;
                item.warnings = parsed
                    .result
                    .issues
                    .iter()
                    .map(|issue| format!("Action {}: {}", issue.index + 1, issue.reason))
                    .collect();
                parsed.result.group.enabled = false;
                item.collision = actions
                    .groups
                    .iter()
                    .any(|group| group.name.eq_ignore_ascii_case(&parsed.result.group.name));
                Ok(Some(Candidate::Actions(parsed.result.group)))
            } else if lower.ends_with(".mte") {
                item.kind = "export".into();
                let parsed = crate::exports::import(&extracted.path().join(file), legacy_encoding)?;
                item.collision = exports
                    .configurations
                    .iter()
                    .any(|configuration| configuration.name.eq_ignore_ascii_case(&parsed.name));
                Ok(Some(Candidate::Export(parsed)))
            } else {
                item.warnings.push(
                    if lower.ends_with(".src")
                        || lower.ends_with(".inc")
                        || lower.ends_with(".settings")
                    {
                        "Install complete source collections separately with Install tag sources."
                    } else {
                        "No supported setup mapping; this file is left untouched."
                    }
                    .into(),
                );
                Ok(None)
            }
        })();
        match result {
            Ok(Some(candidate)) => {
                item.supported = true;
                candidates.insert(file.clone(), candidate);
            }
            Ok(None) => {}
            Err(error) => {
                item.warnings.push(error);
            }
        }
        items.push(item);
    }
    Ok((items, candidates))
}

fn preview(
    directory: &Path,
    path: &str,
    window: WindowState,
    legacy_encoding: Option<String>,
) -> Result<Preview, String> {
    validate_window(&window)?;
    let source_identity = source_identity(directory, Path::new(path))?;
    if status(directory, None, None, None)?.pending.is_some() {
        return Err(
            "Complete the pending workspace recovery before importing another setup.".into(),
        );
    }
    let files = input(Path::new(path))?;
    let target = native(directory)?;
    let legacy_encoding = legacy_encoding.filter(|encoding| !encoding.trim().is_empty());
    if legacy_encoding
        .as_ref()
        .is_some_and(|encoding| encoding.len() > 64)
    {
        return Err("The legacy codepage name is too long.".into());
    }
    let (items, candidates) = inspect(directory, &files, legacy_encoding.as_deref())?;
    let source_hash = hash(&files, None)?;
    let token = uuid::Uuid::new_v4().to_string();
    let plan = Plan {
        path: path.into(),
        source_identity,
        source_hash: source_hash.clone(),
        target_hash: hash(&target, Some(&window))?,
        legacy_encoding: legacy_encoding.clone(),
        items: items.clone(),
        candidates,
        window,
    };
    let mut plans = PLANS
        .get_or_init(Default::default)
        .lock()
        .map_err(|e| e.to_string())?;
    plans.clear();
    plans.insert(token.clone(), plan);
    Ok(Preview {
        token,
        items,
        source_hash,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Report {
    pub legacy_encoding: Option<String>,
    pub items: Vec<Item>,
    pub imported: Vec<String>,
    pub kept: Vec<String>,
    pub source_hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pending {
    id: String,
    attempted: bool,
    window: WindowState,
}
#[derive(Clone, Serialize, Deserialize)]
struct Journal {
    checksum: String,
    version: u32,
    phase: String,
    before: Encoded,
    after: Encoded,
    window: WindowState,
    report: Report,
    pending: Option<Pending>,
    rollback: Option<Encoded>,
    rollback_window: Option<WindowState>,
    previous: bool,
}
#[derive(Debug, Serialize)]
pub struct Status {
    pub desktop: bool,
    pub phase: String,
    pub report: Option<Report>,
    pub restorable: bool,
    pub pending: Option<Pending>,
    pub preference_keys: Vec<String>,
}

fn store(directory: &Path) -> Result<std::path::PathBuf, String> {
    let path = directory.join(STORE);
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("The setup recovery location must be a real directory.".into());
        }
    } else {
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    Ok(path)
}
fn persist(directory: &Path, journal: &Journal) -> Result<(), String> {
    let mut journal = journal.clone();
    journal.checksum.clear();
    let signed = serde_json::to_vec(&journal).map_err(|e| e.to_string())?;
    journal.checksum = format!("{:x}", Sha256::digest(&signed));
    let bytes = serde_json::to_vec(&journal).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT * 5 {
        return Err("The setup recovery journal exceeds its bounded storage limit.".into());
    }
    let path = store(directory)?.join("current.json");
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("The setup journal must be a regular file.".into());
        }
    }
    crate::action_host::atomic_output(&path, &bytes, false)
}
fn journal(directory: &Path) -> Result<Option<Journal>, String> {
    let recovery = directory.join(STORE);
    match std::fs::symlink_metadata(&recovery) {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            return Err("The setup recovery location must be a real directory.".into())
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    }
    let path = recovery.join("current.json");
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    }
    checked_journal(&read(&path, LIMIT * 5)?).map(Some)
}
fn checked_journal(bytes: &[u8]) -> Result<Journal, String> {
    let journal: Journal =
        serde_json::from_slice(bytes).map_err(|e| format!("Setup recovery journal: {e}"))?;
    let mut checked = journal.clone();
    checked.checksum.clear();
    let checksum = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&checked).map_err(|e| e.to_string())?)
    );
    if journal.checksum != checksum {
        return Err(
            "The setup recovery journal checksum does not match; its files were retained.".into(),
        );
    }
    if !["applying", "restoring", "applied", "restored", "failed"].contains(&journal.phase.as_str())
    {
        return Err("Unsupported setup recovery phase.".into());
    }
    if journal.version != 1 {
        return Err("Unsupported setup recovery journal version.".into());
    }
    decode(&journal.before)?;
    decode(&journal.after)?;
    validate_window(&journal.window)?;
    if let Some(pending) = &journal.pending {
        validate_window(&pending.window)?;
    }
    if let Some(rollback) = &journal.rollback {
        decode(rollback)?;
    }
    if let Some(window) = &journal.rollback_window {
        validate_window(window)?;
    }
    Ok(journal)
}
fn current_file(directory: &Path, name: &str) -> Result<Option<Vec<u8>>, String> {
    relative(name)?;
    let mut path = directory.to_path_buf();
    let parts = name.split('/').collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        path.push(part);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!("Setup path is a symbolic link: {name}"))
            }
            Ok(metadata) if index + 1 < parts.len() && !metadata.is_dir() => {
                return Err(format!("Setup path has a non-directory parent: {name}"))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        }
    }
    read(&path, LIMIT).map(Some)
}
fn replace(directory: &Path, expected: &Files, desired: &Files) -> Result<(), String> {
    replace_with(directory, expected, desired, |_| Ok(()))
}
fn replace_with(
    directory: &Path,
    expected: &Files,
    desired: &Files,
    mut before_write: impl FnMut(&str) -> Result<(), String>,
) -> Result<(), String> {
    if native(directory)? != *expected {
        return Err(
            "Setup changed while its replacement was prepared; nothing was overwritten.".into(),
        );
    }
    let names: BTreeSet<_> = expected.keys().chain(desired.keys()).cloned().collect();
    for name in names {
        let path = directory.join(&name);
        // Recheck each path immediately before mutation, including any export parents.
        let current = current_file(directory, &name)?;
        if current.as_ref() != expected.get(&name) {
            return Err(format!("Setup changed during replacement: {name}"));
        }
        if desired.get(&name) != expected.get(&name) {
            before_write(&name)?;
        }
        match desired.get(&name) {
            Some(bytes) if expected.get(&name) != Some(bytes) => {
                crate::action_host::atomic_output(&path, bytes, false)?
            }
            None if expected.contains_key(&name) => {
                std::fs::remove_file(path).map_err(|e| e.to_string())?
            }
            _ => {}
        }
    }
    if native(directory)? != *desired {
        return Err("Setup changed before replacement completed.".into());
    }
    Ok(())
}
fn rollback(directory: &Path, before: &Files, desired: &Files) -> Result<(), String> {
    let current = native(directory)?;
    for name in current.keys().chain(before.keys()).chain(desired.keys()) {
        if current.get(name) != before.get(name) && current.get(name) != desired.get(name) {
            return Err(format!("Recovery retained its snapshot because {name} changed outside the setup operation."));
        }
    }
    replace(directory, &current, before)
}
fn recovered(directory: &Path, state: Journal) -> Result<Journal, String> {
    recovered_with(directory, state, || Ok(()))
}
fn recovered_with(
    directory: &Path,
    mut state: Journal,
    before_publish: impl FnOnce() -> Result<(), String>,
) -> Result<Journal, String> {
    if state.phase == "applying" || state.phase == "restoring" {
        let before = decode(
            state
                .rollback
                .as_ref()
                .ok_or("Setup recovery has no rollback snapshot.")?,
        )?;
        let desired = if state.phase == "applying" {
            decode(&state.after)?
        } else {
            decode(&state.before)?
        };
        // Validate and prepare the complete next journal in memory. Publishing raw
        // previous bytes would both discard this operation's receipt on a crash and
        // destroy the valid current journal when the previous snapshot is corrupt.
        let previous = if state.phase == "applying" && state.previous {
            let mut previous = checked_journal(&read(
                &directory.join(STORE).join("previous.json"),
                LIMIT * 5,
            )?)?;
            let window = state
                .rollback_window
                .as_ref()
                .ok_or("Setup recovery has no window snapshot.")?
                .clone();
            validate_window(&window)?;
            previous.pending = Some(Pending {
                id: uuid::Uuid::new_v4().to_string(),
                attempted: false,
                window,
            });
            Some(previous)
        } else {
            None
        };
        rollback(directory, &before, &desired).map_err(|error| {
            format!(
                "{error} Recovery snapshot retained in {}.",
                directory.join(STORE).display()
            )
        })?;
        if let Some(previous) = previous {
            before_publish()?;
            persist(directory, &previous)?;
            return Ok(previous);
        }
        state.pending = Some(Pending {
            id: uuid::Uuid::new_v4().to_string(),
            attempted: false,
            window: state
                .rollback_window
                .take()
                .ok_or("Setup recovery has no window snapshot.")?,
        });
        if state.phase == "applying" {
            state.report.imported.clear();
        }
        state.phase = if state.phase == "restoring" {
            "applied"
        } else {
            "failed"
        }
        .into();
        state.rollback = None;
        before_publish()?;
        persist(directory, &state)?;
    }
    Ok(state)
}
fn status(
    directory: &Path,
    start: Option<&str>,
    ack: Option<&str>,
    retry: Option<&str>,
) -> Result<Status, String> {
    let state = journal(directory)?
        .map(|state| recovered(directory, state))
        .transpose()?;
    if let Some(mut state) = state {
        if let Some(id) = start {
            let pending = state
                .pending
                .as_mut()
                .filter(|pending| pending.id == id)
                .ok_or("The setup workspace receipt expired.")?;
            pending.attempted = true;
            persist(directory, &state)?;
        }
        if let Some(id) = retry {
            let pending = state
                .pending
                .as_mut()
                .filter(|pending| pending.id == id)
                .ok_or("The setup workspace receipt expired.")?;
            pending.attempted = false;
            persist(directory, &state)?;
        }
        if let Some(id) = ack {
            if state
                .pending
                .as_ref()
                .is_none_or(|pending| pending.id != id || !pending.attempted)
            {
                return Err("The setup workspace receipt expired.".into());
            }
            state.pending = None;
            persist(directory, &state)?;
        }
        Ok(Status {
            desktop: true,
            phase: state.phase.clone(),
            report: Some(state.report),
            restorable: state.phase == "applied",
            pending: state.pending,
            preference_keys: PREFERENCE_KEYS.iter().map(|key| (*key).into()).collect(),
        })
    } else {
        Ok(Status {
            desktop: true,
            phase: "none".into(),
            report: None,
            restorable: false,
            pending: None,
            preference_keys: PREFERENCE_KEYS.iter().map(|key| (*key).into()).collect(),
        })
    }
}

fn apply(
    directory: &Path,
    token: &str,
    selected: &[String],
    replace_names: bool,
    window: WindowState,
) -> Result<Status, String> {
    validate_window(&window)?;
    let plan = PLANS
        .get_or_init(Default::default)
        .lock()
        .map_err(|e| e.to_string())?
        .remove(token)
        .ok_or("Setup preview expired; inspect the profile again.")?;
    if source_identity(directory, Path::new(&plan.path))? != plan.source_identity {
        return Err("The source profile location changed after preview; inspect it again.".into());
    }
    let _ = status(directory, None, None, None)?;
    let before = native(directory)?;
    if hash(&input(Path::new(&plan.path))?, None)? != plan.source_hash
        || hash(&before, Some(&window))? != plan.target_hash
        || window != plan.window
    {
        return Err(
            "The source profile or current setup changed after preview; inspect it again.".into(),
        );
    }
    let names: BTreeSet<_> = selected.iter().collect();
    if names.is_empty()
        || names.len() != selected.len()
        || names
            .iter()
            .any(|name| !plan.candidates.contains_key(*name))
    {
        return Err("Select supported setup items from the current preview.".into());
    }
    for kind in ["columns", "panel"] {
        if plan
            .items
            .iter()
            .filter(|item| item.kind == kind && names.contains(&item.file))
            .count()
            > 1
        {
            return Err("Select at most one column layout and one tag-panel layout.".into());
        }
    }
    // Target keep/replace policy cannot disambiguate two incoming definitions.
    let mut incoming = BTreeMap::new();
    for file in selected {
        let identity = match &plan.candidates[file] {
            Candidate::Actions(group) => ("actions", group.name.to_ascii_lowercase()),
            Candidate::Export(configuration) => ("export", configuration.name.to_ascii_lowercase()),
            _ => continue,
        };
        if let Some(previous) = incoming.insert(identity, file) {
            return Err(format!("Selected setup files have the same destination name: {previous} and {file}. Select only one of these files, then inspect the profile again."));
        }
    }
    let stage = tempfile::tempdir().map_err(|e| e.to_string())?;
    write_stage(stage.path(), &before)?;
    let mut actions = crate::action_host::load_library(&stage.path().join("actions.json"))?;
    let mut exports = crate::exports::load(&stage.path().join("exports.json"))?;
    let mut report = Report {
        legacy_encoding: plan.legacy_encoding,
        items: plan.items,
        imported: Vec::new(),
        kept: Vec::new(),
        source_hash: plan.source_hash,
    };
    let mut actions_changed = false;
    let mut exports_changed = false;
    for name in selected {
        match &plan.candidates[name] {
            Candidate::Columns(layout) => {
                crate::columns::save(&stage.path().join("columns.json"), layout)?
            }
            Candidate::Panel(layout) => {
                let path = stage.path().join("tag-panel.json");
                let mut panel = crate::tag_panel::load(&path)?;
                panel.panel = layout.clone();
                crate::tag_panel::save(&path, &panel)?;
            }
            Candidate::Actions(group) => {
                if let Some(index) = actions
                    .groups
                    .iter()
                    .position(|old| old.name.eq_ignore_ascii_case(&group.name))
                {
                    if !replace_names {
                        report.kept.push(name.clone());
                        continue;
                    }
                    actions.groups[index] = group.clone();
                } else {
                    actions.groups.push(group.clone());
                }
                actions_changed = true;
            }
            Candidate::Export(configuration) => {
                let mut configuration = configuration.clone();
                if let Some(index) = exports
                    .configurations
                    .iter()
                    .position(|old| old.name.eq_ignore_ascii_case(&configuration.name))
                {
                    if !replace_names {
                        report.kept.push(name.clone());
                        continue;
                    }
                    configuration.id = exports.configurations[index].id.clone();
                    configuration.file = exports.configurations[index].file.clone();
                    exports.configurations[index] = configuration;
                } else {
                    exports.configurations.push(configuration);
                }
                exports_changed = true;
            }
        }
        report.imported.push(name.clone());
    }
    if report.imported.is_empty() {
        return Err("No setup items would change; review the collision choices.".into());
    }
    if actions_changed {
        crate::action_host::save_library(&stage.path().join("actions.json"), &actions)?;
    }
    if exports_changed {
        crate::exports::save(&stage.path().join("exports.json"), &exports)?;
    }
    let after = native(stage.path())?;
    // Persist the complete rollback journal before replacing a single managed file.
    let old = journal(directory)?;
    if let Some(old) = &old {
        crate::action_host::atomic_output(
            &store(directory)?.join("previous.json"),
            &serde_json::to_vec(old).map_err(|e| e.to_string())?,
            false,
        )?;
    }
    let mut state = Journal {
        checksum: String::new(),
        version: 1,
        phase: "applying".into(),
        before: encode(&before),
        after: encode(&after),
        window: window.clone(),
        report,
        pending: None,
        rollback: Some(encode(&before)),
        rollback_window: Some(window.clone()),
        previous: old.is_some(),
    };
    persist(directory, &state)?;
    if let Err(error) = replace(directory, &before, &after) {
        recovered(directory, state)?;
        return Err(format!(
            "Setup import failed and its previous configuration was restored: {error}"
        ));
    }
    state.phase = "applied".into();
    state.rollback = None;
    state.rollback_window = None;
    state.pending = Some(Pending {
        id: uuid::Uuid::new_v4().to_string(),
        attempted: false,
        window,
    });
    persist(directory, &state)?;
    status(directory, None, None, None)
}

fn restore(directory: &Path, window: WindowState) -> Result<Status, String> {
    validate_window(&window)?;
    let state = journal(directory)?.ok_or("No setup migration snapshot is available.")?;
    let mut state = recovered(directory, state)?;
    if state.phase != "applied" {
        return Err("No completed setup import is available to restore.".into());
    }
    let current = native(directory)?;
    let before = decode(&state.before)?;
    state.phase = "restoring".into();
    state.rollback = Some(encode(&current));
    state.rollback_window = Some(window);
    persist(directory, &state)?;
    if let Err(error) = replace(directory, &current, &before) {
        recovered(directory, state)?;
        return Err(format!(
            "Setup restore failed; its previous current configuration was retained: {error}"
        ));
    }
    state.phase = "restored".into();
    state.rollback = None;
    state.rollback_window = None;
    state.pending = Some(Pending {
        id: uuid::Uuid::new_v4().to_string(),
        attempted: false,
        window: state.window.clone(),
    });
    persist(directory, &state)?;
    status(directory, None, None, None)
}

fn require_desktop() -> Result<(), String> {
    if cfg!(mobile) {
        return Err("Setup migration is available in the desktop app.".into());
    }
    Ok(())
}
#[tauri::command]
pub async fn profile_migration_preview(
    app: tauri::AppHandle,
    path: String,
    window: WindowState,
    legacy_encoding: Option<String>,
) -> Result<Preview, String> {
    require_desktop()?;
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || preview(&directory, &path, window, legacy_encoding)).await
}
#[tauri::command]
pub async fn profile_migration_apply(
    app: tauri::AppHandle,
    token: String,
    selected: Vec<String>,
    replace_names: bool,
    window: WindowState,
) -> Result<Status, String> {
    require_desktop()?;
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || apply(&directory, &token, &selected, replace_names, window)).await
}
#[tauri::command]
pub async fn profile_migration_status(
    app: tauri::AppHandle,
    start: Option<String>,
    ack: Option<String>,
    retry: Option<String>,
) -> Result<Status, String> {
    if cfg!(mobile) {
        return Ok(Status {
            desktop: false,
            phase: "unsupported".into(),
            report: None,
            restorable: false,
            pending: None,
            preference_keys: Vec::new(),
        });
    }
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || {
        let result = status(
            &directory,
            start.as_deref(),
            ack.as_deref(),
            retry.as_deref(),
        )?;
        crate::settings::reload(&app)?;
        Ok(result)
    })
    .await
}
#[tauri::command]
pub async fn profile_migration_restore(
    app: tauri::AppHandle,
    window: WindowState,
) -> Result<Status, String> {
    require_desktop()?;
    let directory = crate::utilities::configuration_directory(&app)?;
    crate::io(move || {
        let result = restore(&directory, window)?;
        crate::settings::reload(&app)?;
        Ok(result)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    static TESTS: Mutex<()> = Mutex::new(());
    fn window() -> WindowState {
        let root = std::env::temp_dir()
            .join("profile-music")
            .to_string_lossy()
            .into_owned();
        let path = Path::new(&root)
            .join("Practice.flac")
            .to_string_lossy()
            .into_owned();
        WindowState {
            version: 1,
            preferences: BTreeMap::from([
                ("opentag.language".into(), "nl".into()),
                ("opentag.filter-shown".into(), "false".into()),
            ]),
            workspace: Workspace {
                paths: vec![path.clone()],
                selected: vec![path.clone()],
                roots: vec![root.clone()],
                location: root,
                location_is_data: true,
                filter: "ARTIST HAS Practice".into(),
                filter_draft: Some("draft expression".into()),
                anchor: Some(path),
                playlist: None,
            },
        }
    }
    fn make_preview(directory: &Path, path: &str, window: WindowState) -> Result<Preview, String> {
        preview(directory, path, window, None)
    }
    fn fixture() -> Files {
        input(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/profile-migration"))
            .unwrap()
    }
    fn config(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(
            path.join("settings.json"),
            serde_json::to_vec(&crate::settings::Settings::default()).unwrap(),
        )
        .unwrap();
        std::fs::write(path.join("lists.json"), br#"{"version":1,"lists":{"opentag.history.filter":[{"value":"Before","favorite":true}]}}"#).unwrap();
        std::fs::write(
            path.join("directories.json"),
            br#"{"version":1,"favorites":["/Before"]}"#,
        )
        .unwrap();
        let parsed = crate::mp3tag_columns::parse_diagnostic(
            "[#0]\nName=Before\nValue=%title%\nField=%title%\nWidth=120\n",
        )
        .unwrap();
        crate::columns::save(&path.join("columns.json"), &parsed.result.layout).unwrap();
        let mut panel = crate::tag_panel::Settings::default();
        panel
            .field_values
            .lists
            .insert("GENRE".into(), vec!["Before genre".into()]);
        crate::tag_panel::save(&path.join("tag-panel.json"), &panel).unwrap();
        let actions = tag_core::actions::ActionLibrary {
            groups: vec![tag_core::actions::ActionGroup {
                name: "Normalize".into(),
                enabled: true,
                actions: Vec::new(),
            }],
            presets: Vec::new(),
        };
        crate::action_host::save_library(&path.join("actions.json"), &actions).unwrap();
        let mut exports = crate::exports::Library::default();
        exports.configurations.push(crate::exports::Configuration {
            id: "before-practice".into(),
            name: "Practice".into(),
            template: "Before export".into(),
            file: None,
        });
        crate::exports::save(&path.join("exports.json"), &exports).unwrap();
        std::fs::create_dir_all(path.join("export/.metadata/deep")).unwrap();
        std::fs::write(
            path.join("export/.metadata/deep/notes"),
            b"untouched metadata",
        )
        .unwrap();
        std::fs::write(
            path.join("tag-sources.json"),
            b"fixture credentials remain untouched",
        )
        .unwrap();
    }
    fn selected() -> Vec<String> {
        [
            "columns.ini",
            "usrfields.ini",
            "data/actions/Normalize.mta",
            "data/actions/Needs review.mta",
            "export/Practice.mte",
            "export/Other.mte",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }
    fn make_source(root: &Path) -> String {
        write_stage(root, &fixture()).unwrap();
        root.to_string_lossy().into_owned()
    }
    fn pending_state(before: &Files, after: &Files, phase: &str) -> Journal {
        Journal {
            checksum: String::new(),
            version: 1,
            phase: phase.into(),
            before: encode(before),
            after: encode(after),
            window: window(),
            report: Report {
                legacy_encoding: None,
                items: Vec::new(),
                imported: vec!["fixture".into()],
                kept: Vec::new(),
                source_hash: hash(after, None).unwrap(),
            },
            pending: None,
            rollback: Some(encode(before)),
            rollback_window: Some(window()),
            previous: false,
        }
    }
    #[test]
    fn review_source_target_overlap_is_rejected_before_any_recovery_write() {
        let _guard = TESTS.lock().unwrap();
        for relation in ["equal", "contains", "inside", "not-created"] {
            let root = tempfile::tempdir().unwrap();
            let directory = root.path().join("config");
            config(&directory);
            let source = match relation {
                "equal" => directory.clone(),
                "contains" | "not-created" => root.path().to_path_buf(),
                _ => directory.join("profile"),
            };
            std::fs::create_dir_all(&source).unwrap();
            std::fs::write(source.join("columns.ini"), fixture()["columns.ini"].clone()).unwrap();
            let target = if relation == "not-created" {
                directory.join("future/nested")
            } else {
                directory.clone()
            };
            if relation != "not-created" {
                let before = native(&target).unwrap();
                let mut after = before.clone();
                after.insert("columns.json".into(), b"pending operation".to_vec());
                persist(&target, &pending_state(&before, &after, "applying")).unwrap();
                std::fs::write(target.join("columns.json"), &after["columns.json"]).unwrap();
            }
            let had_store = target.join(STORE).exists();
            let before = input(root.path()).unwrap();
            let result = make_preview(&target, source.to_str().unwrap(), window());
            assert!(
                result.is_err(),
                "{relation}: overlapping source was accepted"
            );
            assert!(result.err().unwrap().contains("outside"));
            assert_eq!(input(root.path()).unwrap(), before);
            assert_eq!(target.join(STORE).exists(), had_store);
        }
    }
    #[cfg(unix)]
    #[test]
    fn review_source_retarget_is_rejected_before_apply_recovery() {
        let _guard = TESTS.lock().unwrap();
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("config");
        config(&directory);
        let original = root.path().join("first/profile");
        let other = root.path().join("second/profile");
        write_stage(&original, &fixture()).unwrap();
        write_stage(&other, &fixture()).unwrap();
        let alias = root.path().join("selected");
        std::os::unix::fs::symlink(original.parent().unwrap(), &alias).unwrap();
        let path = alias.join("profile");
        let preview = make_preview(&directory, path.to_str().unwrap(), window()).unwrap();
        std::fs::remove_file(&alias).unwrap();
        std::os::unix::fs::symlink(other.parent().unwrap(), &alias).unwrap();
        let before = native(&directory).unwrap();
        // A pending operation would mutate settings if status were called before
        // checking the source location; rejection must retain it byte-for-byte.
        let mut after = before.clone();
        after.insert("columns.json".into(), b"interrupted bytes".to_vec());
        persist(&directory, &pending_state(&before, &after, "applying")).unwrap();
        std::fs::write(directory.join("columns.json"), &after["columns.json"]).unwrap();
        let current = std::fs::read(directory.join(STORE).join("current.json")).unwrap();
        let error = apply(&directory, &preview.token, &selected(), true, window()).unwrap_err();
        assert!(error.contains("location changed"), "{error}");
        assert_eq!(native(&directory).unwrap(), after);
        assert_eq!(
            std::fs::read(directory.join(STORE).join("current.json")).unwrap(),
            current
        );
        // Retargeting the parent into the configuration tree must fail disjointness
        // before status as well, even when the profile bytes are identical.
        std::fs::remove_file(&alias).unwrap();
        std::os::unix::fs::symlink(original.parent().unwrap(), &alias).unwrap();
        // Obtain a clean preview in a separate destination.
        let clean = root.path().join("clean");
        let preview = make_preview(&clean, path.to_str().unwrap(), window()).unwrap();
        std::fs::remove_file(&alias).unwrap();
        std::os::unix::fs::symlink(&clean, &alias).unwrap();
        write_stage(&clean.join("profile"), &fixture()).unwrap();
        let error = apply(&clean, &preview.token, &selected(), true, window()).unwrap_err();
        assert!(error.contains("outside"), "{error}");
        assert!(!clean.join(STORE).exists());
    }
    #[test]
    fn review_corrupt_previous_journal_never_replaces_valid_current() {
        let dir = tempfile::tempdir().unwrap();
        config(dir.path());
        let before = native(dir.path()).unwrap();
        let mut state = pending_state(&before, &before, "applying");
        state.previous = true;
        persist(dir.path(), &state).unwrap();
        let path = dir.path().join(STORE).join("current.json");
        let current = std::fs::read(&path).unwrap();
        for corrupt in [
            b"corrupt previous".to_vec(),
            serde_json::to_vec(&state).unwrap(),
        ] {
            // The second input is valid JSON with an invalid checksum, requiring
            // the same complete validation as the authoritative current journal.
            std::fs::write(store(dir.path()).unwrap().join("previous.json"), corrupt).unwrap();
            assert!(status(dir.path(), None, None, None).is_err());
            assert_eq!(
                std::fs::read(&path).unwrap(),
                current,
                "valid current journal was clobbered"
            );
            assert_eq!(native(dir.path()).unwrap(), before);
        }
    }
    #[test]
    fn review_same_name_source_candidates_are_rejected_for_both_policies() {
        let _guard = TESTS.lock().unwrap();
        for (extension, bytes) in [
            ("mta", fixture()["data/actions/Normalize.mta"].clone()),
            ("mte", fixture()["export/Practice.mte"].clone()),
        ] {
            for replace in [false, true] {
                let dir = tempfile::tempdir().unwrap();
                config(dir.path());
                let source = tempfile::tempdir().unwrap();
                let files = Files::from([
                    (format!("first/Mixed.{extension}"), bytes.clone()),
                    (format!("second/mIXED.{extension}"), bytes.clone()),
                ]);
                write_stage(source.path(), &files).unwrap();
                let preview =
                    make_preview(dir.path(), source.path().to_str().unwrap(), window()).unwrap();
                let before = native(dir.path()).unwrap();
                let result = apply(
                    dir.path(),
                    &preview.token,
                    &files.keys().cloned().collect::<Vec<_>>(),
                    replace,
                    window(),
                );
                assert!(
                    result.is_err(),
                    "{extension} replace={replace}: ambiguous source selection was accepted"
                );
                let error = result.unwrap_err();
                for name in files.keys() {
                    assert!(error.contains(name), "{error}");
                }
                assert_eq!(native(dir.path()).unwrap(), before);
                assert!(!dir.path().join(STORE).exists());
            }
        }
    }
    #[test]
    fn no_journal_status_does_not_create_or_write_configuration_directories() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("not-created");
        let state = status(&directory, None, None, None).unwrap();
        assert_eq!(state.phase, "none");
        assert!(state.report.is_none());
        assert!(!directory.exists());
        std::fs::create_dir_all(&directory).unwrap();
        let state = status(&directory, None, None, None).unwrap();
        assert!(!state.restorable);
        assert!(!directory.join(STORE).exists());
    }
    #[test]
    fn populated_folder_preview_import_restart_and_full_content_restore() {
        let _guard = TESTS.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        config(dir.path());
        let path = make_source(source.path());
        let original_input = input(source.path()).unwrap();
        let before = native(dir.path()).unwrap();
        let preview = make_preview(dir.path(), &path, window()).unwrap();
        assert_eq!(preview.items.len(), 10);
        assert_eq!(
            preview.items.iter().filter(|item| item.supported).count(),
            7
        );
        assert!(preview
            .items
            .iter()
            .find(|item| item.file == "columns.ini")
            .unwrap()
            .warnings
            .iter()
            .any(|warning| warning.contains("fixture_unknown")));
        assert!(preview
            .items
            .iter()
            .find(|item| item.file == "usrfields.ini")
            .unwrap()
            .warnings
            .iter()
            .any(|warning| warning.contains("technical")));
        assert!(preview
            .items
            .iter()
            .find(|item| item.file == "data/actions/Needs review.mta")
            .unwrap()
            .warnings
            .iter()
            .any(|warning| warning.contains("999")));
        let applied = apply(dir.path(), &preview.token, &selected(), true, window()).unwrap();
        assert_eq!(applied.report.as_ref().unwrap().imported.len(), 6);
        assert!(applied.restorable);
        assert_eq!(
            crate::columns::load(&dir.path().join("columns.json"))
                .unwrap()
                .unwrap()
                .columns[0]
                .name,
            "Track title"
        );
        assert_eq!(
            crate::tag_panel::load(&dir.path().join("tag-panel.json"))
                .unwrap()
                .field_values
                .lists["GENRE"],
            ["Before genre"]
        );
        let groups = crate::action_host::load_library(&dir.path().join("actions.json")).unwrap();
        assert_eq!(groups.groups.len(), 2);
        assert!(groups.groups.iter().all(|group| !group.enabled));
        assert!(matches!(
            groups.groups[1].actions[0],
            tag_core::actions::ActionSpec::UnresolvedMta { .. }
        ));
        let exports = crate::exports::load(&dir.path().join("exports.json")).unwrap();
        let imported = exports
            .configurations
            .iter()
            .find(|item| item.name == "Practice")
            .unwrap();
        assert_eq!(imported.id, "before-practice");
        assert!(imported.template.contains("%artist%"));
        // A fresh status read uses only the persisted journal, not the preview map.
        PLANS.get().unwrap().lock().unwrap().clear();
        let restarted = status(dir.path(), None, None, None).unwrap();
        let id = restarted.pending.unwrap().id;
        assert!(
            status(dir.path(), None, Some(&id), None).is_err(),
            "cannot acknowledge an unclaimed workspace receipt"
        );
        status(dir.path(), Some(&id), None, None).unwrap();
        status(dir.path(), None, Some(&id), None).unwrap();
        assert!(status(dir.path(), None, None, None)
            .unwrap()
            .pending
            .is_none());
        std::fs::write(dir.path().join("lists.json"), b"later configured histories").unwrap();
        let restored = restore(dir.path(), window()).unwrap();
        assert!(!restored.restorable);
        assert_eq!(restored.pending.unwrap().window, window());
        assert_eq!(native(dir.path()).unwrap(), before);
        assert_eq!(input(source.path()).unwrap(), original_input);
        assert_eq!(
            std::fs::read(dir.path().join("tag-sources.json")).unwrap(),
            b"fixture credentials remain untouched"
        );
    }
    #[test]
    fn zip_inventory_and_explicit_keep_collisions_preserve_existing_items() {
        let _guard = TESTS.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        config(dir.path());
        let source = tempfile::tempdir().unwrap();
        let zip_path = source.path().join("profile.zip");
        let mut archive = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        for (name, bytes) in fixture() {
            archive
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            archive.write_all(&bytes).unwrap();
        }
        archive.finish().unwrap();
        let before = native(dir.path()).unwrap();
        let preview = make_preview(dir.path(), zip_path.to_str().unwrap(), window()).unwrap();
        let selected = [
            "data/actions/Normalize.mta",
            "export/Practice.mte",
            "export/Other.mte",
        ]
        .map(str::to_owned);
        let applied = apply(dir.path(), &preview.token, &selected, false, window()).unwrap();
        let report = applied.report.unwrap();
        assert_eq!(report.kept.len(), 2);
        assert_eq!(report.imported, ["export/Other.mte"]);
        assert_eq!(
            std::fs::read(dir.path().join("actions.json")).unwrap(),
            before["actions.json"]
        );
        assert_eq!(
            std::fs::read(dir.path().join("export/before-practice.mte")).unwrap(),
            before["export/before-practice.mte"]
        );
    }
    #[test]
    fn restore_removes_configuration_and_templates_that_were_absent_before_import() {
        let _guard = TESTS.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let path = make_source(source.path());
        assert!(native(dir.path()).unwrap().is_empty());
        let preview = make_preview(dir.path(), &path, window()).unwrap();
        apply(dir.path(), &preview.token, &selected(), true, window()).unwrap();
        assert!(dir.path().join("columns.json").is_file());
        assert!(!native(dir.path()).unwrap().is_empty());
        let restored = restore(dir.path(), window()).unwrap();
        assert_eq!(restored.phase, "restored");
        assert!(native(dir.path()).unwrap().is_empty());
        assert!(!dir.path().join("columns.json").exists());
    }
    #[test]
    fn cancelled_or_changed_source_target_template_or_browser_state_never_applies() {
        let _guard = TESTS.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        config(dir.path());
        let source = tempfile::tempdir().unwrap();
        let path = make_source(source.path());
        let before = native(dir.path()).unwrap();
        let _cancelled = make_preview(dir.path(), &path, window()).unwrap();
        assert_eq!(native(dir.path()).unwrap(), before);
        let first = make_preview(dir.path(), &path, window()).unwrap();
        std::fs::write(source.path().join("README.txt"), "source changed").unwrap();
        assert!(apply(dir.path(), &first.token, &selected(), true, window())
            .unwrap_err()
            .contains("changed after preview"));
        assert_eq!(native(dir.path()).unwrap(), before);
        let second = make_preview(dir.path(), &path, window()).unwrap();
        std::fs::write(
            dir.path().join("export/before-practice.mte"),
            "external new template",
        )
        .unwrap();
        assert!(
            apply(dir.path(), &second.token, &selected(), true, window())
                .unwrap_err()
                .contains("changed after preview")
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("export/before-practice.mte")).unwrap(),
            "external new template"
        );
        let third = make_preview(dir.path(), &path, window()).unwrap();
        let mut changed = window();
        changed.workspace.filter = "new filter".into();
        assert!(apply(dir.path(), &third.token, &selected(), true, changed)
            .unwrap_err()
            .contains("changed after preview"));
    }
    #[test]
    fn rejected_second_publication_recovers_exact_before_content_from_journal() {
        let dir = tempfile::tempdir().unwrap();
        config(dir.path());
        let before = native(dir.path()).unwrap();
        let mut after = before.clone();
        after.insert("actions.json".into(), b"new action bytes".to_vec());
        after.insert("columns.json".into(), b"new column bytes".to_vec());
        persist(dir.path(), &pending_state(&before, &after, "applying")).unwrap();
        let mut writes = 0;
        let refused = replace_with(dir.path(), &before, &after, |_| {
            writes += 1;
            if writes == 2 {
                Err("controlled write refusal".into())
            } else {
                Ok(())
            }
        });
        assert!(refused.unwrap_err().contains("controlled write refusal"));
        assert_ne!(native(dir.path()).unwrap(), before);
        let recovered = status(dir.path(), None, None, None).unwrap();
        assert_eq!(native(dir.path()).unwrap(), before);
        assert!(!recovered.restorable);
        assert!(recovered.pending.is_some());
    }
    #[test]
    fn interrupted_restore_and_external_conflict_retain_recoverable_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        config(dir.path());
        let before = native(dir.path()).unwrap();
        let mut imported = before.clone();
        imported.insert("actions.json".into(), b"imported action bytes".to_vec());
        imported.insert("columns.json".into(), b"imported column bytes".to_vec());
        write_stage(dir.path(), &imported).unwrap();
        let mut state = pending_state(&before, &imported, "restoring");
        state.rollback = Some(encode(&imported));
        persist(dir.path(), &state).unwrap();
        std::fs::write(dir.path().join("actions.json"), &before["actions.json"]).unwrap();
        let recovered = status(dir.path(), None, None, None).unwrap();
        assert!(recovered.restorable);
        assert_eq!(native(dir.path()).unwrap(), imported);
        let mut state = pending_state(&before, &imported, "applying");
        state.rollback = Some(encode(&before));
        persist(dir.path(), &state).unwrap();
        std::fs::write(
            dir.path().join("actions.json"),
            b"external edit during recovery",
        )
        .unwrap();
        assert!(status(dir.path(), None, None, None)
            .unwrap_err()
            .contains("snapshot retained"));
        assert_eq!(
            std::fs::read(dir.path().join("actions.json")).unwrap(),
            b"external edit during recovery"
        );
        assert!(store(dir.path()).unwrap().join("current.json").exists());
    }
    #[test]
    fn journal_and_restore_survive_fresh_processes_without_preview_memory() {
        let root = tempfile::tempdir().unwrap();
        let nonce = uuid::Uuid::new_v4().to_string();
        std::fs::write(root.path().join(".profile-native-test-sandbox"), &nonce).unwrap();
        for phase in ["apply", "restore", "consumed"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "profile_migration::tests::profile_restart_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("OPENTAG_PROFILE_TEST_ROOT", root.path())
                .env("OPENTAG_PROFILE_TEST_NONCE", &nonce)
                .env("OPENTAG_PROFILE_TEST_PHASE", phase)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "fresh-process {phase}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    #[test]
    fn review_interrupted_second_import_keeps_latest_receipt_across_processes() {
        let root = tempfile::tempdir().unwrap();
        let nonce = uuid::Uuid::new_v4().to_string();
        std::fs::write(root.path().join(".profile-native-test-sandbox"), &nonce).unwrap();
        for phase in [
            "apply",
            "second-interrupted",
            "second-recovered",
            "second-consumed",
        ] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "profile_migration::tests::profile_restart_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("OPENTAG_PROFILE_TEST_ROOT", root.path())
                .env("OPENTAG_PROFILE_TEST_NONCE", &nonce)
                .env("OPENTAG_PROFILE_TEST_PHASE", phase)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "fresh-process {phase}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    #[test]
    #[ignore = "controlled restart child; invoked only by the parent sandbox test"]
    fn profile_restart_child() {
        let root = std::path::PathBuf::from(
            std::env::var("OPENTAG_PROFILE_TEST_ROOT").expect("parent sandbox"),
        );
        let nonce = std::env::var("OPENTAG_PROFILE_TEST_NONCE").expect("parent nonce");
        assert_eq!(
            std::fs::read_to_string(root.join(".profile-native-test-sandbox")).unwrap(),
            nonce
        );
        uuid::Uuid::parse_str(&nonce).unwrap();
        let directory = root.join("config");
        match std::env::var("OPENTAG_PROFILE_TEST_PHASE")
            .unwrap()
            .as_str()
        {
            "apply" => {
                config(&directory);
                let source = root.join("source");
                std::fs::create_dir_all(&source).unwrap();
                let path = make_source(&source);
                let preview = make_preview(&directory, &path, window()).unwrap();
                apply(&directory, &preview.token, &selected(), true, window()).unwrap();
            }
            "second-interrupted" => {
                let pending = status(&directory, None, None, None)
                    .unwrap()
                    .pending
                    .unwrap();
                status(&directory, Some(&pending.id), None, None).unwrap();
                status(&directory, None, Some(&pending.id), None).unwrap();
                let old = journal(&directory).unwrap().unwrap();
                crate::action_host::atomic_output(
                    &directory.join(STORE).join("previous.json"),
                    &serde_json::to_vec(&old).unwrap(),
                    false,
                )
                .unwrap();
                let before = native(&directory).unwrap();
                let mut after = before.clone();
                after.insert(
                    "columns.json".into(),
                    b"second interrupted publication".to_vec(),
                );
                let mut state = pending_state(&before, &after, "applying");
                let mut latest = window();
                latest.workspace.filter = "latest pre-second-import workspace".into();
                state.window = latest.clone();
                state.rollback_window = Some(latest);
                state.previous = true;
                persist(&directory, &state).unwrap();
                std::fs::write(directory.join("columns.json"), &after["columns.json"]).unwrap();
            }
            "second-recovered" => {
                assert!(PLANS.get().is_none());
                let interrupted = journal(&directory).unwrap().unwrap();
                let before = decode(interrupted.rollback.as_ref().unwrap()).unwrap();
                let current = std::fs::read(directory.join(STORE).join("current.json")).unwrap();
                let error = recovered_with(&directory, interrupted, || {
                    assert_eq!(
                        std::fs::read(directory.join(STORE).join("current.json")).unwrap(),
                        current
                    );
                    Err("controlled crash before journal publication".into())
                })
                .err()
                .unwrap();
                assert!(error.contains("controlled crash"));
                assert_eq!(native(&directory).unwrap(), before);
                assert_eq!(
                    std::fs::read(directory.join(STORE).join("current.json")).unwrap(),
                    current
                );
            }
            "second-consumed" => {
                assert!(PLANS.get().is_none());
                let recovered = status(&directory, None, None, None).unwrap();
                assert!(recovered.restorable);
                let pending = recovered.pending.unwrap();
                assert_eq!(
                    pending.window.workspace.filter,
                    "latest pre-second-import workspace"
                );
                status(&directory, Some(&pending.id), None, None).unwrap();
                status(&directory, None, Some(&pending.id), None).unwrap();
                assert!(status(&directory, None, None, None)
                    .unwrap()
                    .pending
                    .is_none());
            }
            "restore" => {
                assert!(PLANS.get().is_none());
                let state = journal(&directory).unwrap().unwrap();
                let before = decode(&state.before).unwrap();
                assert_eq!(
                    status(&directory, None, None, None)
                        .unwrap()
                        .report
                        .unwrap()
                        .imported
                        .len(),
                    6
                );
                let restored = restore(&directory, window()).unwrap();
                assert_eq!(native(&directory).unwrap(), before);
                let pending = restored.pending.unwrap();
                status(&directory, Some(&pending.id), None, None).unwrap();
                status(&directory, None, Some(&pending.id), None).unwrap();
            }
            "consumed" => {
                let state = status(&directory, None, None, None).unwrap();
                assert_eq!(state.phase, "restored");
                assert!(state.pending.is_none());
                assert!(!state.restorable);
            }
            _ => panic!("unknown parent phase"),
        }
    }
    #[test]
    fn workspace_count_limit_explains_the_playlist_route_without_truncation() {
        let mut state = window();
        state.workspace.paths = vec![state.workspace.paths[0].clone(); 10001];
        let error = validate_window(&state).unwrap_err();
        assert!(error.contains("10000"));
        assert!(error.contains("playlist"));
        assert!(error.contains("no files are truncated"));
    }
    #[test]
    fn column_limit_is_reported_and_root_level_preset_folders_are_recognized() {
        let _guard = TESTS.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        config(dir.path());
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("columns")).unwrap();
        std::fs::create_dir(source.path().join("panels")).unwrap();
        let text = (0..201)
            .map(|index| format!("[#{index}]\nName=Column {index}\nValue=%title%\nWidth=100\n"))
            .collect::<String>();
        std::fs::write(source.path().join("columns/Large.ini"), text).unwrap();
        std::fs::write(
            source.path().join("panels/Small.ini"),
            b"[#0]\nname=Title\nvalue=TITLE\nversion=1\n",
        )
        .unwrap();
        let preview = make_preview(dir.path(), source.path().to_str().unwrap(), window()).unwrap();
        let columns = preview
            .items
            .iter()
            .find(|item| item.kind == "columns")
            .unwrap();
        assert!(columns.supported);
        assert!(columns
            .warnings
            .iter()
            .any(|warning| warning.contains("200: 1.")));
        assert!(preview
            .items
            .iter()
            .any(|item| item.kind == "panel" && item.supported));
    }
    #[test]
    fn corrupt_journal_and_unsafe_zip_entries_never_publish_configuration() {
        let dir = tempfile::tempdir().unwrap();
        config(dir.path());
        let before = native(dir.path()).unwrap();
        persist(dir.path(), &pending_state(&before, &before, "applying")).unwrap();
        let path = store(dir.path()).unwrap().join("current.json");
        let mut modified: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        modified["after"]["columns.json"] = serde_json::Value::String("Y29ycnVwdA==".into());
        std::fs::write(&path, serde_json::to_vec(&modified).unwrap()).unwrap();
        assert!(status(dir.path(), None, None, None)
            .unwrap_err()
            .contains("checksum"));
        assert_eq!(native(dir.path()).unwrap(), before);
        for names in [
            vec!["../outside.ini"],
            vec!["A.ini", "a.INI"],
            vec!["C:/outside.ini"],
        ] {
            let archive = dir.path().join("unsafe.zip");
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
            for name in names {
                zip.start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(b"fixture").unwrap();
            }
            zip.finish().unwrap();
            assert!(input(&archive).is_err());
            assert_eq!(native(dir.path()).unwrap(), before);
        }
    }
    #[test]
    fn explicit_legacy_codepage_uses_existing_importers_and_oversized_items_stay_visible() {
        let _guard = TESTS.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        config(dir.path());
        let source = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("export")).unwrap();
        std::fs::write(
            source.path().join("export/Legacy.mte"),
            b"$filename(txt)Caf\xe9",
        )
        .unwrap();
        std::fs::write(
            source.path().join("Legacy.mta"),
            b"[#0]\nT=5\nF=TITLE\n1=Caf\xe9\n",
        )
        .unwrap();
        std::fs::write(
            source.path().join("columns.ini"),
            vec![b'x'; 4 * 1024 * 1024 + 1],
        )
        .unwrap();
        let preview = preview(
            dir.path(),
            source.path().to_str().unwrap(),
            window(),
            Some("windows-1252".into()),
        )
        .unwrap();
        assert!(
            preview
                .items
                .iter()
                .find(|item| item.kind == "export")
                .unwrap()
                .supported
        );
        let columns = preview
            .items
            .iter()
            .find(|item| item.file == "columns.ini")
            .unwrap();
        assert!(!columns.supported);
        assert!(columns
            .warnings
            .iter()
            .any(|warning| warning.contains("4 MB")));
        assert!(
            preview
                .items
                .iter()
                .find(|item| item.kind == "actions")
                .unwrap()
                .supported
        );
        let plan = PLANS.get().unwrap().lock().unwrap();
        let Candidate::Export(configuration) =
            &plan[&preview.token].candidates["export/Legacy.mte"]
        else {
            panic!("export candidate")
        };
        assert!(configuration.template.contains("Café"));
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_journal_escape_credentials_and_unseen_selections_are_rejected() {
        let _guard = TESTS.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        config(dir.path());
        let source = tempfile::tempdir().unwrap();
        let path = make_source(source.path());
        let preview = make_preview(dir.path(), &path, window()).unwrap();
        assert!(apply(
            dir.path(),
            &preview.token,
            &["Mp3tagSettings.bin".into()],
            true,
            window()
        )
        .is_err());
        let mut invalid = window();
        invalid
            .preferences
            .insert("opentag.api-token".into(), "must not be captured".into());
        assert!(validate_window(&invalid).is_err());
        invalid = window();
        invalid.workspace.paths = vec!["content://provider/document".into()];
        invalid.workspace.selected.clear();
        invalid.workspace.anchor = None;
        assert!(validate_window(&invalid).is_err());
        std::os::unix::fs::symlink(
            dir.path().join("tag-sources.json"),
            source.path().join("outside.ini"),
        )
        .unwrap();
        assert!(input(source.path()).is_err());
        let before = native(dir.path()).unwrap();
        let mut state = pending_state(&before, &before, "applying");
        state.before.insert("../outside.json".into(), "YQ==".into());
        persist(dir.path(), &state).unwrap();
        assert!(status(dir.path(), None, None, None).is_err());
        assert_eq!(native(dir.path()).unwrap(), before);
    }
}
