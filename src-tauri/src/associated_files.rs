//! Opt-in, reviewed same-basename lyrics/karaoke moves. No vendor runtime claim.
use serde::Serialize;

use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::Read,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// Saved policy; absent on older profiles and false by default.
pub fn enabled() -> bool {
    crate::settings::current()
        .pages
        .get("files")
        .and_then(|page| page.get("associated_files"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    len: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    identity: (u64, u64),
    digest: Option<[u8; 32]>,
}
impl Snapshot {
    fn read(path: &Path, hash: bool) -> Result<Self, String> {
        let metadata =
            std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if !metadata.is_file() {
            return Err(format!(
                "{}: only regular files can be moved",
                path.display()
            ));
        }
        let digest = if hash {
            let mut input = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mut digest = Sha256::new();
            let mut buffer = [0; 64 * 1024];
            loop {
                let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
                if count == 0 {
                    break;
                }
                digest.update(&buffer[..count]);
            }
            Some(digest.finalize().into())
        } else {
            None
        };
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            identity: (metadata.dev(), metadata.ino()),
            digest,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Item {
    pub source: String,
    pub target: PathBuf,
    snapshot: Snapshot,
}
impl Item {
    fn new(source: String, target: PathBuf, associated: bool) -> Result<Self, String> {
        let snapshot = Snapshot::read(Path::new(&source), associated)?;
        Ok(Self {
            source,
            target,
            snapshot,
        })
    }
    fn validate(&self) -> Result<(), String> {
        if Snapshot::read(Path::new(&self.source), self.snapshot.digest.is_some())? != self.snapshot
        {
            return Err(format!(
                "{} changed after preview; preview again",
                self.source
            ));
        }
        target_free(Path::new(&self.source), &self.target)
    }
}
#[derive(Clone, Debug)]
pub struct Group {
    pub audio: Item,
    pub associated: Vec<Item>,
    enabled: bool,
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub groups: Vec<Group>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Preview {
    pub from: String,
    pub to: String,
}

/// Use exact stems and case-insensitive supported extensions. Never guess at
/// album artwork, suffixes (song.en.lrc), other stems or unsupported file types.
pub fn is_companion(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lrc") || ext.eq_ignore_ascii_case("cdg"))
}
fn companions(source: &Path) -> Result<Vec<PathBuf>, String> {
    let parent = source.parent().ok_or("The source has no folder")?;
    let stem = source.file_stem().ok_or("The source has no basename")?;
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(parent).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.file_stem() == Some(stem)
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    ext.eq_ignore_ascii_case("lrc") || ext.eq_ignore_ascii_case("cdg")
                })
        {
            Snapshot::read(&path, false)?;
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}
fn key(path: &Path) -> String {
    let parent = path.parent().and_then(|p| dunce::canonicalize(p).ok());
    parent
        .map(|p| p.join(path.file_name().unwrap_or_default()))
        .unwrap_or_else(|| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase()
}
fn target_free(source: &Path, target: &Path) -> Result<(), String> {
    use tag_core::operations::{destination_state, Destination};
    if matches!(destination_state(source, target), Destination::Taken) {
        return Err(format!("{} already exists", target.display()));
    }
    // Also reject portable case collisions on case-sensitive volumes.
    if let Some(parent) = target.parent().filter(|p| p.is_dir()) {
        let name = target
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let original = dunce::canonicalize(source).ok();
        for entry in std::fs::read_dir(parent).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name().to_string_lossy().to_lowercase() == name
                && dunce::canonicalize(entry.path()).ok() != original
            {
                return Err(format!(
                    "{} already exists (including letter case)",
                    entry.path().display()
                ));
            }
        }
    }
    Ok(())
}
impl Plan {
    pub fn new(audio: Vec<(String, PathBuf)>, enabled: bool) -> Result<Self, String> {
        let mut groups = Vec::new();
        let mut sources = HashSet::new();
        let mut targets = HashSet::new();
        for (source, target) in audio {
            let mut associated = Vec::new();
            if enabled {
                for path in companions(Path::new(&source))? {
                    let mut side_target = target.clone();
                    side_target
                        .set_extension(path.extension().ok_or("Missing companion extension")?);
                    associated.push(Item::new(
                        path.to_string_lossy().into_owned(),
                        side_target,
                        true,
                    )?);
                }
            }
            let audio = Item::new(source, target, false)?;
            for item in std::iter::once(&audio).chain(&associated) {
                if !sources.insert(key(Path::new(&item.source))) {
                    return Err(format!(
                        "{} belongs to more than one selected file",
                        item.source
                    ));
                }
                if !targets.insert(key(&item.target)) {
                    return Err(format!(
                        "More than one file would become {}",
                        item.target.display()
                    ));
                }
                item.validate()?;
            }
            groups.push(Group {
                audio,
                associated,
                enabled,
            });
        }
        Ok(Self { groups })
    }
    pub fn validate(&self) -> Result<(), String> {
        for group in &self.groups {
            group.validate()?;
        }
        Ok(())
    }
    pub fn preview(&self) -> Vec<Preview> {
        self.groups
            .iter()
            .flat_map(|g| g.associated.iter())
            .map(|item| Preview {
                from: item.source.clone(),
                to: item.target.to_string_lossy().into_owned(),
            })
            .collect()
    }
}
impl Group {
    fn validate(&self) -> Result<(), String> {
        if self.enabled
            && companions(Path::new(&self.audio.source))?
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .ne(self.associated.iter().map(|item| item.source.clone()))
        {
            return Err(format!(
                "Associated files for {} changed after preview; preview again",
                self.audio.source
            ));
        }
        for item in std::iter::once(&self.audio).chain(&self.associated) {
            item.validate()?;
        }
        Ok(())
    }
    /// Move companions first and audio last. If a member fails, reverse each
    /// completed move without replacing anything. Report incomplete rollback.
    pub fn execute(
        &self,
        operation: &mut dyn FnMut(&Path, &Path) -> Result<(), String>,
    ) -> Result<(), String> {
        self.validate()?;
        let mut completed: Vec<&Item> = Vec::new();
        for item in self.associated.iter().chain(std::iter::once(&self.audio)) {
            // Recheck immediately before each move, after the initial undo capture.
            let result = item
                .validate()
                .and_then(|()| operation(Path::new(&item.source), &item.target));
            if let Err(error) = result {
                let mut rollback_errors = Vec::new();
                for done in completed.into_iter().rev() {
                    let rollback = Snapshot::read(&done.target, true).and_then(|current| {
                        if done.snapshot.digest.is_some() && current.digest != done.snapshot.digest
                        {
                            return Err(
                                "The moved companion changed; retained it at its destination"
                                    .into(),
                            );
                        }
                        tag_core::operations::rename_no_clobber(
                            &done.target,
                            Path::new(&done.source),
                        )
                    });
                    if let Err(rollback) = rollback {
                        rollback_errors.push(format!(
                            "{} → {}: {rollback}",
                            done.target.display(),
                            done.source
                        ));
                    }
                }
                return Err(if rollback_errors.is_empty() {
                    format!("{error}; completed group moves were rolled back")
                } else {
                    format!(
                        "{error}; rollback incomplete: {}",
                        rollback_errors.join("; ")
                    )
                });
            }
            completed.push(item);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    fn pair(root: &Path, name: &str) -> (String, std::path::PathBuf) {
        let source = root.join(format!("{name}.wav"));
        std::fs::write(&source, b"audio").unwrap();
        (
            source.to_string_lossy().into_owned(),
            root.join(format!("{name}-new.wav")),
        )
    }
    #[test]
    fn opt_in_collects_only_exact_basename_supported_companions() {
        let dir = tempfile::tempdir().unwrap();
        let audio = pair(dir.path(), "Song");
        for name in ["Song.lrc", "Song.CDG", "Song.txt", "Other.lrc"] {
            std::fs::write(dir.path().join(name), name).unwrap();
        }
        assert!(Plan::new(vec![audio.clone()], false).unwrap().groups[0]
            .associated
            .is_empty());
        let plan = Plan::new(vec![audio], true).unwrap();
        assert_eq!(plan.groups[0].associated.len(), 2);
        assert!(plan.groups[0]
            .associated
            .iter()
            .any(|item| item.target.ends_with("Song-new.CDG")));
        assert!(plan.groups[0]
            .associated
            .iter()
            .any(|item| item.target.ends_with("Song-new.lrc")));
    }
    #[test]
    fn changed_and_appeared_companions_invalidate_reviewed_membership() {
        let dir = tempfile::tempdir().unwrap();
        let audio = pair(dir.path(), "Song");
        let lyric = dir.path().join("Song.lrc");
        std::fs::write(&lyric, b"first").unwrap();
        let plan = Plan::new(vec![audio.clone()], true).unwrap();
        std::fs::write(&lyric, b"other").unwrap();
        assert!(plan.validate().is_err());
        let plan = Plan::new(vec![audio], true).unwrap();
        std::fs::write(dir.path().join("Song.cdg"), b"late").unwrap();
        assert!(plan.validate().is_err());
        assert!(lyric.exists());
    }
    #[test]
    fn companion_collisions_case_collisions_and_rename_cycles_touch_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let first = pair(dir.path(), "a");
        let second = pair(dir.path(), "b");
        std::fs::write(dir.path().join("a.lrc"), b"lyrics").unwrap();
        std::fs::write(dir.path().join("A-NEW.LRC"), b"keep").unwrap();
        assert!(Plan::new(vec![first.clone()], true).is_err());
        assert!(Plan::new(
            vec![
                (first.0.clone(), second.0.clone().into()),
                (second.0, first.0.clone().into())
            ],
            true
        )
        .is_err());
        assert_eq!(
            std::fs::read(dir.path().join("A-NEW.LRC")).unwrap(),
            b"keep"
        );
        assert!(Path::new(&first.0).exists());
    }
    #[cfg(unix)]
    #[test]
    fn symlink_companions_and_late_symlink_substitution_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let audio = pair(dir.path(), "Song");
        let lyric = dir.path().join("Song.lrc");
        let other = dir.path().join("other");
        std::fs::write(&other, b"keep").unwrap();
        std::os::unix::fs::symlink(&other, &lyric).unwrap();
        assert!(Plan::new(vec![audio.clone()], true).is_err());
        std::fs::remove_file(&lyric).unwrap();
        std::fs::write(&lyric, b"lyric").unwrap();
        let plan = Plan::new(vec![audio], true).unwrap();
        std::fs::remove_file(&lyric).unwrap();
        std::os::unix::fs::symlink(&other, &lyric).unwrap();
        assert!(plan.validate().is_err());
        assert_eq!(std::fs::read(other).unwrap(), b"keep");
    }
    #[test]
    fn failed_audio_move_rolls_back_completed_companions() {
        let dir = tempfile::tempdir().unwrap();
        let audio = pair(dir.path(), "Song");
        std::fs::write(dir.path().join("Song.lrc"), b"lyric").unwrap();
        let plan = Plan::new(vec![audio.clone()], true).unwrap();
        let mut calls = 0;
        let result = plan.groups[0].execute(&mut |source, target| {
            calls += 1;
            if calls == 2 {
                Err("injected audio failure".into())
            } else {
                tag_core::operations::rename_no_clobber(source, target)
            }
        });
        assert!(result.is_err());
        assert_eq!(
            std::fs::read(dir.path().join("Song.lrc")).unwrap(),
            b"lyric"
        );
        assert!(Path::new(&audio.0).exists());
        assert!(!audio.1.exists());
        assert!(!dir.path().join("Song-new.lrc").exists());
    }
    #[test]
    fn late_audio_failure_and_occupied_rollback_name_preserve_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let audio = pair(dir.path(), "Song");
        let lyric = dir.path().join("Song.lrc");
        std::fs::write(&lyric, b"original lyric").unwrap();
        let plan = Plan::new(vec![audio.clone()], true).unwrap();
        let mut calls = 0;
        let result = plan.groups[0].execute(&mut |source, target| {
            calls += 1;
            if calls == 2 {
                std::fs::write(&lyric, b"new occupant").unwrap();
                Err("injected audio failure".into())
            } else {
                tag_core::operations::rename_no_clobber(source, target)
            }
        });
        assert!(result.unwrap_err().contains("rollback incomplete"));
        assert_eq!(std::fs::read(&lyric).unwrap(), b"new occupant");
        assert_eq!(
            std::fs::read(dir.path().join("Song-new.lrc")).unwrap(),
            b"original lyric"
        );
        assert!(Path::new(&audio.0).exists());
    }
    #[test]
    fn changed_moved_companion_is_not_overwritten_during_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let audio = pair(dir.path(), "Song");
        std::fs::write(dir.path().join("Song.lrc"), b"original lyric").unwrap();
        let plan = Plan::new(vec![audio.clone()], true).unwrap();
        let moved = dir.path().join("Song-new.lrc");
        let mut calls = 0;
        let result = plan.groups[0].execute(&mut |source, target| {
            calls += 1;
            if calls == 2 {
                std::fs::write(&moved, b"external change").unwrap();
                Err("injected audio failure".into())
            } else {
                tag_core::operations::rename_no_clobber(source, target)
            }
        });
        assert!(result.unwrap_err().contains("rollback incomplete"));
        assert_eq!(std::fs::read(moved).unwrap(), b"external change");
        assert!(!dir.path().join("Song.lrc").exists());
        assert!(Path::new(&audio.0).exists());
    }

    #[cfg(unix)]
    #[test]
    fn replaced_audio_identity_and_symbolic_link_sources_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let audio = pair(dir.path(), "Song");
        let original_time = std::fs::metadata(&audio.0).unwrap().modified().unwrap();
        let plan = Plan::new(vec![audio.clone()], true).unwrap();
        let replacement = dir.path().join("replacement");
        std::fs::write(&replacement, b"audio").unwrap();
        std::fs::rename(&replacement, &audio.0).unwrap();
        std::fs::File::open(&audio.0)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(original_time))
            .unwrap();
        assert!(
            plan.validate().is_err(),
            "same bytes, size and mtime must not hide replaced physical source"
        );
        let link = dir.path().join("link.wav");
        std::os::unix::fs::symlink(&audio.0, &link).unwrap();
        assert!(Plan::new(vec![(link.to_string_lossy().into_owned(), audio.1)], true).is_err());
    }
}
