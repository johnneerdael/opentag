//! Bounded bundle intake and replacement. Private source text never leaves the machine.
use super::{extension, load, plain_name, relative_name, schema, MAX_DEPTH, MAX_FILE_BYTES};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;

pub(super) static LOCK: Mutex<()> = Mutex::new(());
const MAX_ENTRIES: usize = 4096;
const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXISTING_DEPTH: usize = 64;
const BACKUP: &str = ".opentag-sources-previous";
const MARKER: &str = ".opentag-source-install";

type Files = BTreeMap<String, Vec<u8>>;

fn supported(name: &str) -> bool {
    matches!(extension(name).as_str(), "src" | "inc" | "settings")
}

fn add(files: &mut Files, name: &str, bytes: Vec<u8>, total: &mut u64) -> Result<(), String> {
    relative_name(name)?;
    if files
        .keys()
        .any(|old| old.to_lowercase() == name.to_lowercase())
    {
        return Err(format!(
            "The bundle repeats {name} (file names ignore case)."
        ));
    }
    *total += bytes.len() as u64;
    if bytes.len() as u64 > MAX_FILE_BYTES || *total > MAX_BYTES || files.len() >= MAX_ENTRIES {
        return Err(
            "Source bundle exceeds the limit: 4 MB per source file, 128 MB total, 4096 files."
                .into(),
        );
    }
    files.insert(name.into(), bytes);
    Ok(())
}

fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() || meta.len() > limit {
        return Err(format!(
            "{} must be a regular file of at most {limit} bytes (no symbolic links).",
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
        return Err("File grew beyond the installation limit.".into());
    }
    Ok(bytes)
}

fn directory(
    path: &Path,
    prefix: &str,
    depth: usize,
    files: &mut Files,
    total: &mut u64,
    entries: &mut usize,
    all: bool,
) -> Result<(), String> {
    for entry in std::fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        *entries += 1;
        if *entries > MAX_ENTRIES {
            return Err("Source folder exceeds 4096 entries.".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Source folder has a non-Unicode file name.")?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!(
                "{name}: symbolic links are not accepted in source folders."
            ));
        }
        // Download metadata and documentation are never installed. Existing local
        // files are copied verbatim, including settings.json and debug artifacts.
        if !all && name.starts_with('.') {
            continue;
        }
        let relative = format!("{prefix}{name}");
        if kind.is_dir() {
            let depth_limit = if all { MAX_EXISTING_DEPTH } else { MAX_DEPTH };
            if depth >= depth_limit {
                return Err(format!(
                    "{relative}: source folders can be at most {depth_limit} levels deep."
                ));
            }
            if !all {
                plain_name(&name)?;
            }
            directory(
                &entry.path(),
                &format!("{relative}/"),
                depth + 1,
                files,
                total,
                entries,
                all,
            )?;
        } else if all || supported(&name) {
            if !all {
                plain_name(&name)?;
            }
            let bytes = read(&entry.path(), if all { MAX_BYTES } else { MAX_FILE_BYTES })?;
            *total += bytes.len() as u64;
            if *total > MAX_BYTES {
                return Err("Sources folder exceeds the 128 MB staging limit.".into());
            }
            if all {
                files.insert(relative, bytes);
            } else {
                *total -= bytes.len() as u64;
                add(files, &relative, bytes, total)?;
            }
        }
    }
    Ok(())
}

fn archive(
    path: &Path,
    files: &mut Files,
    total: &mut u64,
    entries: &mut usize,
) -> Result<(), String> {
    let bytes = read(path, MAX_BYTES)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    *entries += zip.len();
    if *entries > MAX_ENTRIES {
        return Err("Source ZIP exceeds 4096 entries.".into());
    }
    let mut seen = BTreeSet::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("ZIP entry {i}: {e}"))?;
        let name = entry.name().replace('\\', "/");
        let name = name.trim_end_matches('/');
        // Validate every archive entry, including ignored documentation. Never
        // extract any paths before this check.
        if name.starts_with('/')
            || name
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('\0'))
        {
            return Err(format!("ZIP contains an unsafe path: {name}"));
        }
        if name.contains(':') {
            return Err(format!("ZIP contains an unsafe path: {name}"));
        }
        if !seen.insert(name.to_lowercase()) {
            return Err(format!("The ZIP repeats {name} (file names ignore case)."));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(format!(
                "{name}: symbolic links are not accepted in ZIP files."
            ));
        }
        if entry.is_dir()
            || name
                .split('/')
                .any(|part| part.starts_with('.') || part == "__MACOSX")
            || !supported(name)
        {
            continue;
        }
        relative_name(name)?;
        if entry.size() > MAX_FILE_BYTES {
            return Err(format!("{name} is larger than 4 MB."));
        }
        let mut bytes = Vec::new();
        (&mut entry)
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("{name}: {e}"))?;
        add(files, name, bytes, total)?;
    }
    Ok(())
}

/// An interrupted directory swap retains the old complete folder beside it.
/// Only restore/delete a backup carrying our marker, never an unrelated folder.
pub(super) fn recover(folder: &Path) -> Result<(), String> {
    let parent = folder.parent().ok_or("The sources folder has no parent.")?;
    let backup = parent.join(BACKUP);
    if !backup.exists() {
        return Ok(());
    }
    if std::fs::symlink_metadata(&backup)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
        || std::fs::read(backup.join(MARKER)).ok().as_deref()
            != Some(b"source installer backup v1\n")
    {
        return Err(format!(
            "Installation recovery folder {} has no marker; move it aside before installing.",
            backup.display()
        ));
    }
    if !folder.exists() {
        std::fs::rename(&backup, folder)
            .map_err(|e| format!("Restore {} to {}: {e}", backup.display(), folder.display()))?;
        let _ = std::fs::remove_file(folder.join(MARKER));
    }
    Ok(())
}

fn write_all(folder: &Path, files: &Files) -> Result<(), String> {
    for (name, bytes) in files {
        let path = folder.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(())
}

pub(super) fn prepare_folder_open(folder: &Path) -> Result<(), String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    recover(folder)?;
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())
}

pub(super) fn install(folder: &Path, paths: &[String]) -> Result<(), String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    recover(folder)?;
    if paths.len() > MAX_ENTRIES {
        return Err("Choose at most 4096 source paths.".into());
    }
    if paths.is_empty() {
        return Err("Choose a source folder, ZIP, or .src, .inc and .settings files.".into());
    }
    let mut incoming = Files::new();
    let mut complete = false;
    let mut total = 0;
    let mut entries = 0;
    for path in paths {
        let path = Path::new(path);
        let meta =
            std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.is_symlink() {
            return Err("Symbolic links cannot be installed as source bundles.".into());
        }
        if meta.is_dir() {
            complete = true;
            directory(path, "", 0, &mut incoming, &mut total, &mut entries, false)?;
        } else if extension(&path.to_string_lossy()) == "zip" {
            complete = true;
            archive(path, &mut incoming, &mut total, &mut entries)?;
        } else {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Source file has no usable name.")?;
            if !supported(name) {
                return Err(format!(
                    "{name} is not a .src, .inc or .settings file or ZIP."
                ));
            }
            add(&mut incoming, name, read(path, MAX_FILE_BYTES)?, &mut total)?;
        }
    }
    if incoming.is_empty() {
        return Err("The bundle contains no .src, .inc or .settings files.".into());
    }
    let parent = folder.parent().ok_or("The sources folder has no parent.")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let stage = tempfile::Builder::new()
        .prefix(".opentag-sources-stage-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let mut existing = Files::new();
    if folder.exists() {
        let meta = std::fs::symlink_metadata(folder).map_err(|e| e.to_string())?;
        if !meta.is_dir() || meta.is_symlink() {
            return Err("The sources folder must be a real directory.".into());
        }
        directory(folder, "", 0, &mut existing, &mut 0, &mut 0, true)?;
    }
    // Canonicalize every path component, including existing folder spelling.
    // Otherwise a case-sensitive host could create both Shop/ and shop/ and
    // break Windows-style companion lookup in a later installation.
    let mut spellings = BTreeMap::new();
    for name in existing.keys() {
        let mut prefix = String::new();
        for part in name.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            if let Some(previous) = spellings.insert(prefix.to_lowercase(), prefix.clone()) {
                if previous != prefix {
                    return Err(format!("Installed paths differ only in case: {previous} and {prefix}. Rename one before installing."));
                }
            }
        }
    }
    let incoming = incoming
        .into_iter()
        .map(|(name, bytes)| {
            let mut canonical = String::new();
            for part in name.split('/') {
                if !canonical.is_empty() {
                    canonical.push('/');
                }
                canonical.push_str(part);
                let known = spellings
                    .entry(canonical.to_lowercase())
                    .or_insert_with(|| canonical.clone());
                canonical = known.clone();
            }
            (canonical, bytes)
        })
        .collect::<Files>();
    let mut combined_entries = BTreeSet::new();
    for name in existing.keys().chain(incoming.keys()) {
        let mut prefix = String::new();
        for part in name.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            combined_entries.insert(prefix.clone());
        }
    }
    if combined_entries.len() > MAX_ENTRIES {
        return Err("Source folder exceeds 4096 entries.".into());
    }
    let combined_bytes = existing
        .iter()
        .filter(|(name, _)| !incoming.contains_key(*name))
        .map(|(_, bytes)| bytes.len() as u64)
        .chain(incoming.values().map(|bytes| bytes.len() as u64))
        .sum::<u64>();
    if combined_bytes > MAX_BYTES {
        return Err("Combined sources exceed the 128 MB staging limit.".into());
    }
    write_all(stage.path(), &existing)?;
    write_all(stage.path(), &incoming)?;
    // Folder/ZIP bundles must be self-contained. Explicit-file selections also
    // support incremental updates using validated installed companions.
    let check = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
    write_all(check.path(), &incoming)?;
    let checked = if complete { check.path() } else { stage.path() };
    for name in incoming.keys() {
        match extension(name).as_str() {
            "src" => {
                let file = load(checked, name)
                    .map_err(|e| format!("{name}: {e}. Choose the complete bundle."))?;
                if let Some(settings) = file.settings {
                    schema(checked, &settings)
                        .map_err(|e| format!("{name}: [Settings]={settings}: {e}"))?;
                }
            }
            "settings" => {
                schema(checked, name)?;
            }
            "inc" => {
                super::source::validate_includes(name, &super::reader(checked))?;
            }
            _ => {}
        }
    }
    // A shared include/settings update must not break a previously working
    // source outside the incoming collection. Keep pre-existing problems visible
    // without making an unrelated old broken source block every new install.
    let validate_source = |root: &Path, name: &str| -> Result<(), String> {
        let source = load(root, name)?;
        if let Some(settings) = source.settings {
            schema(root, &settings)?;
        }
        Ok(())
    };
    for name in existing.keys().filter(|name| extension(name) == "src") {
        if validate_source(folder, name).is_ok() {
            validate_source(stage.path(), name)
                .map_err(|e| format!("{name}: {e}. Choose the complete bundle."))?;
        }
    }
    let backup = parent.join(BACKUP);
    if backup.exists() {
        // recover already checked the marker. Keep the last successful snapshot
        // until another fully validated install is ready to replace it.
        std::fs::remove_dir_all(&backup).map_err(|e| e.to_string())?;
    }
    if folder.exists() {
        std::fs::write(folder.join(MARKER), b"source installer backup v1\n")
            .map_err(|e| e.to_string())?;
        std::fs::rename(folder, &backup).map_err(|e| e.to_string())?;
    }
    if let Err(error) = std::fs::rename(stage.path(), folder) {
        if backup.exists() {
            std::fs::rename(&backup, folder).map_err(|restore| {
                format!(
                    "Install failed ({error}); restore {} to {}: {restore}",
                    backup.display(),
                    folder.display()
                )
            })?;
            let _ = std::fs::remove_file(folder.join(MARKER));
        }
        return Err(format!("Sources were not replaced: {error}"));
    }
    // TempDir now points at the old staging path, so dropping it cannot remove
    // the installed folder. The complete previous folder remains recoverable.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    const SOURCE: &str = "[Settings]=Options.settings\n[Include]=Common.inc\n";
    const INCLUDE: &str =
        "[AlbumUrl]=https://example.invalid/%s\n[ParserScriptAlbum]=...\nSayRest\n";
    const SETTINGS: &str =
        r#"{"key":"example","settings":[{"type":"string","key":"language","default":"en"}]}"#;
    fn collection(path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        std::fs::write(path.join("Shop#&Album.src"), SOURCE).unwrap();
        std::fs::write(path.join("Common.inc"), INCLUDE).unwrap();
        std::fs::write(path.join("Options.settings"), SETTINGS).unwrap();
    }
    fn selected(path: &Path) -> Vec<String> {
        vec![path.to_string_lossy().into_owned()]
    }
    fn zip(path: &Path, entries: &[(&str, &str)]) {
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, text) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(text.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
    }
    #[test]
    fn folder_updates_preserve_values_other_files_and_previous_complete_install() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("data/sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        let values = br#"{"example":{"language":"de"}}"#;
        std::fs::write(target.join("settings.json"), values).unwrap();
        std::fs::write(target.join("notes.txt"), "local file").unwrap();
        std::fs::write(
            bundle.join("Common.inc"),
            INCLUDE.replace("example.invalid", "changed.invalid"),
        )
        .unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert_eq!(std::fs::read(target.join("settings.json")).unwrap(), values);
        assert_eq!(
            std::fs::read_to_string(target.join("notes.txt")).unwrap(),
            "local file"
        );
        assert!(
            std::fs::read_to_string(target.parent().unwrap().join(BACKUP).join("Common.inc"))
                .unwrap()
                .contains("example.invalid")
        );
        let listing = super::super::list(&target);
        assert_eq!(listing.sources.len(), 1);
        assert_eq!(listing.sources[0].menu, "Shop#&Album");
        assert!(listing.problems.is_empty());
    }
    #[test]
    fn existing_hidden_directories_survive_an_update() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("data/sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        let metadata = target.join(".git/logs/refs/remotes/origin");
        std::fs::create_dir_all(&metadata).unwrap();
        std::fs::write(metadata.join("main"), b"retained local history").unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert_eq!(
            std::fs::read(metadata.join("main")).unwrap(),
            b"retained local history"
        );
        assert_eq!(super::super::list(&target).sources.len(), 1);
    }

    #[test]
    fn folder_open_restores_an_interrupted_installation_before_creating_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sources");
        let backup = dir.path().join(BACKUP);
        collection(&backup);
        std::fs::write(backup.join(MARKER), b"source installer backup v1\n").unwrap();
        prepare_folder_open(&target).unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        assert!(!backup.exists());
    }

    #[test]
    fn folder_open_waits_for_an_installation_swap() {
        use std::sync::mpsc;
        use std::time::Duration;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sources");
        let backup = dir.path().join(BACKUP);
        let stage = dir.path().join("stage");
        collection(&target);
        collection(&stage);
        std::fs::write(stage.join("notes.txt"), "new complete installation").unwrap();
        let guard = LOCK.lock().unwrap();
        std::fs::write(target.join(MARKER), b"source installer backup v1\n").unwrap();
        std::fs::rename(&target, &backup).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let folder = target.clone();
        let opener = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let result = prepare_folder_open(&folder);
            done_tx.send(result).unwrap();
        });
        started_rx.recv().unwrap();
        assert!(matches!(
            done_rx.recv_timeout(Duration::from_millis(250)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        assert!(
            !target.exists(),
            "folder opening must not recreate the swap destination"
        );
        std::fs::rename(&stage, &target).unwrap();
        drop(guard);
        done_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
        opener.join().unwrap();
        assert_eq!(
            std::fs::read_to_string(target.join("notes.txt")).unwrap(),
            "new complete installation"
        );
        assert!(backup.join("Common.inc").is_file());
    }

    #[test]
    fn second_file_and_dependency_errors_leave_every_installed_file_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        std::fs::write(bundle.join("Common.inc"), "updated").unwrap();
        std::fs::write(bundle.join("Options.settings"), "invalid JSON").unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        std::fs::write(bundle.join("Options.settings"), SETTINGS).unwrap();
        std::fs::remove_file(bundle.join("Common.inc")).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("complete bundle"));
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
    }
    #[test]
    fn explicit_file_update_reuses_installed_companions_without_weakening_bundle_preflight() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        std::fs::remove_file(bundle.join("Common.inc")).unwrap();
        std::fs::remove_file(bundle.join("Options.settings")).unwrap();
        install(&target, &selected(&bundle.join("Shop#&Album.src"))).unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        std::fs::write(
            bundle.join("Common.inc"),
            format!("{INCLUDE}\n[Settings]=Absent.settings\n"),
        )
        .unwrap();
        assert!(install(&target, &selected(&bundle.join("Common.inc"))).is_err());
        assert_eq!(
            std::fs::read_to_string(target.join("Common.inc")).unwrap(),
            INCLUDE
        );
        assert_eq!(super::super::list(&target).sources.len(), 1);
    }

    #[test]
    fn nested_zip_preflights_all_companions_and_rejects_duplicate_and_escape_paths() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bundle.zip");
        let target = dir.path().join("sources");
        zip(
            &archive,
            &[
                ("Shop/Album.src", SOURCE),
                ("Shop/Common.inc", INCLUDE),
                ("Shop/Options.settings", SETTINGS),
                (".DS_Store", "ignored download metadata"),
                ("__MACOSX/._Shop", "ignored download metadata"),
            ],
        );
        install(&target, &selected(&archive)).unwrap();
        assert!(target.join("Shop/Album.src").exists());
        for entries in [
            vec![("../outside.src", INCLUDE)],
            vec![("A.src", INCLUDE), ("a.SRC", INCLUDE)],
            vec![("C:/evil.src", INCLUDE)],
            vec![("foo/../../evil.src", INCLUDE)],
        ] {
            zip(&archive, &entries);
            assert!(install(&target, &selected(&archive)).is_err());
            assert!(target.join("Shop/Album.src").exists());
        }
        assert!(!dir.path().join("outside.src").exists());
    }
    #[test]
    fn updates_reuse_installed_directory_spelling_and_preserve_ids() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle.join("Shop"));
        install(&target, &selected(&bundle)).unwrap();
        std::fs::rename(bundle.join("Shop"), bundle.join("shop")).unwrap();
        std::fs::rename(
            bundle.join("shop/Shop#&Album.src"),
            bundle.join("shop/shop#&album.SRC"),
        )
        .unwrap();
        std::fs::write(bundle.join("shop/Extra.inc"), INCLUDE).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        assert!(target.join("Shop/Extra.inc").is_file());
        let directories = std::fs::read_dir(&target)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(directories, ["Shop"]);
        assert_eq!(
            super::super::list(&target).sources[0].id,
            "Shop/Shop#&Album.src"
        );
    }

    #[test]
    fn interrupted_swap_restores_previous_complete_sources_on_next_read() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        install(&target, &selected(&bundle)).unwrap();
        install(&target, &selected(&bundle)).unwrap();
        std::fs::remove_dir_all(&target).unwrap();
        assert_eq!(super::super::list(&target).sources.len(), 1);
        assert!(!dir.path().join(BACKUP).exists());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_folder_and_zip_symlinks_and_bounded_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("download");
        let target = dir.path().join("sources");
        collection(&bundle);
        std::os::unix::fs::symlink(bundle.join("Common.inc"), bundle.join("Link.inc")).unwrap();
        assert!(install(&target, &selected(&bundle))
            .unwrap_err()
            .contains("symbolic links"));
        std::fs::remove_file(bundle.join("Link.inc")).unwrap();
        std::fs::write(bundle.join("Big.inc"), vec![0; MAX_FILE_BYTES as usize + 1]).unwrap();
        assert!(install(&target, &selected(&bundle)).is_err());
        let archive = dir.path().join("links.zip");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        writer
            .add_symlink(
                "Link.src",
                "elsewhere",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        writer.finish().unwrap();
        assert!(install(&target, &selected(&archive))
            .unwrap_err()
            .contains("symbolic links"));
        assert!(!target.exists());
    }
    #[test]
    #[ignore = "read-only qualification of user-owned local source files; requires OPENTAG_SOURCE_BUNDLE"]
    fn qualify_private_bundle() {
        let bundle = std::env::var("OPENTAG_SOURCE_BUNDLE").expect("OPENTAG_SOURCE_BUNDLE");
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("sources");
        install(&target, &[bundle]).expect("bundle preflight/install failed");
        let listing = super::super::list(&target);
        assert!(listing.problems.is_empty());
        let mut scripts = 0;
        for source in &listing.sources {
            let file = load(&target, &source.id).unwrap();
            for block in [file.index_script, file.album_script].into_iter().flatten() {
                super::super::engine::Script::compile(&block.lines)
                    .expect("parser compilation failed");
                scripts += 1;
            }
        }
        println!("qualification: {} sources, {} settings schemas, {scripts} parser blocks compile; no provider requests, metadata changes or source text recorded", listing.sources.len(), listing.settings.len());
    }
}
