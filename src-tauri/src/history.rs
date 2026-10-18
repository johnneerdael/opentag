//! Session undo and redo for commands that write audio files. Each mutating
//! command runs through [`record`], which snapshots its local targets in the
//! portable journal before the work and records the outcome afterwards.
use crate::action_host::{ActionResult, PathChange};
use serde::Serialize;
use std::{path::Path, sync::Mutex};
use tag_core::batch_errors::ErrorCause;
use tag_core::history::{HistoryState, Journal, Limits};
use tag_core::ScanResult;
use tauri::Manager;

static JOURNAL: Mutex<Option<Journal>> = Mutex::new(None);

fn with_journal<T>(work: impl FnOnce(&mut Journal) -> T) -> Result<T, String> {
    with_journal_diagnostic(work).map_err(|cause| cause.to_string())
}
fn with_journal_diagnostic<T>(work: impl FnOnce(&mut Journal) -> T) -> Result<T, ErrorCause> {
    let mut guard = JOURNAL
        .lock()
        .map_err(|_| ErrorCause::owned("Undo history lock failed"))?;
    if guard.is_none() {
        let root = match super::APP_HANDLE.get() {
            Some(app) => app
                .path()
                .app_cache_dir()
                .map_err(ErrorCause::external)?
                .join("undo"),
            None => std::env::temp_dir().join("opentag-undo"),
        };
        *guard = Some(Journal::open(&root, Limits::default()).map_err(ErrorCause::external)?);
    }
    Ok(work(guard.as_mut().expect("journal was opened")))
}

/// Path changes reported by a command, used to find each file afterwards.
pub trait Changes {
    fn renamed(&self) -> Vec<(String, String)> {
        Vec::new()
    }
    fn directories(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}
impl Changes for ScanResult {}
impl Changes for crate::file_ops::TransferResult {
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .chain(&self.associated_renamed)
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}
impl<T: Changes> Changes for tag_core::batch_errors::Detailed<T> {
    fn renamed(&self) -> Vec<(String, String)> {
        self.result.renamed()
    }
    fn directories(&self) -> Vec<(String, String)> {
        self.result.directories()
    }
}

impl Changes for super::FilenameImport {}
impl Changes for super::RenameResult {
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .chain(&self.associated_renamed)
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}
impl Changes for crate::converters::FilenameResult {
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}
impl Changes for ActionResult {
    fn renamed(&self) -> Vec<(String, String)> {
        self.renamed
            .iter()
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
    fn directories(&self) -> Vec<(String, String)> {
        self.moved_directories
            .iter()
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect()
    }
}

#[cfg(test)]
pub(crate) static TEST_ROOTS: Mutex<Vec<std::path::PathBuf>> = Mutex::new(Vec::new());
/// Tests that undo through the shared journal hold this so their steps do not interleave.
#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

// Provider documents publish through their own recovery copies and are not journaled.
fn journaled(path: &str) -> bool {
    // Commands share one journal; tests opt in so parallel tests cannot interleave steps.
    #[cfg(test)]
    if !TEST_ROOTS
        .lock()
        .unwrap()
        .iter()
        .any(|root| Path::new(path).starts_with(root))
    {
        return false;
    }
    !cfg!(target_os = "android")
        && !path.starts_with("content://")
        && !super::documents::is_managed(Path::new(path))
}

/// Run `work` with undo snapshots of `paths`. Operations sharing `group`
/// form one undo step. Journal failures never block the operation itself.
pub fn record<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    record_with_side_files(label, group, paths, |_| work())
}

/// [`record`] for work that also creates or replaces files that are not audio
/// (a saved cover image). `work` calls the hook it receives with each such
/// path before writing it; an `Err` from the hook means the file must not be
/// written because its undo copy failed.
pub fn record_with_side_files<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path) -> Result<(), String>) -> Result<T, String>,
) -> Result<T, String> {
    record_with_side_file_epochs(label, group, paths, |capture| {
        work(&mut |path| capture(path, 0))
    })
}

/// Side-file recording whose hook also receives the completed directory-move count.
pub fn record_with_side_file_epochs<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path, usize) -> Result<(), String>) -> Result<T, String>,
) -> Result<T, String> {
    record_with_side_file_epochs_diagnostic(label, group, paths, |capture| {
        work(&mut |path, epoch| capture(path, epoch).map_err(|cause| cause.to_string()))
    })
}

/// The same history/cache transaction, retaining the side-file hook's error provenance.
pub fn record_with_side_file_epochs_diagnostic<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path, usize) -> Result<(), ErrorCause>) -> Result<T, String>,
) -> Result<T, String> {
    let result = journal_work_diagnostic(label, group, paths, work);
    // The library reads written, renamed and moved files again.
    let (renamed, directories) = result
        .as_ref()
        .map(|r| (r.renamed(), r.directories()))
        .unwrap_or_default();
    super::library::changed(paths, &renamed, &directories);
    result
}

pub fn record_with_side_files_diagnostic<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path) -> Result<(), ErrorCause>) -> Result<T, String>,
) -> Result<T, String> {
    record_with_side_file_epochs_diagnostic(label, group, paths, |capture| {
        work(&mut |path| capture(path, 0))
    })
}

fn journal_work_diagnostic<T: Changes>(
    label: &str,
    group: Option<&str>,
    paths: &[String],
    work: impl FnOnce(&mut dyn FnMut(&Path, usize) -> Result<(), ErrorCause>) -> Result<T, String>,
) -> Result<T, String> {
    // Chapter and cue sheet rows are snapshots of their file.
    let mut local: Vec<String> = Vec::new();
    for path in paths {
        let file = tag_core::subsong_parts(path).map_or(path.as_str(), |(file, _)| file);
        if journaled(file) && !local.iter().any(|p| p == file) {
            local.push(file.to_owned());
        }
    }
    if local.is_empty() {
        return work(&mut |_, _| Ok(()));
    }
    let mut capture = with_journal(|journal| journal.begin(label, group, &local));
    let result = work(&mut |path, epoch| match capture.as_mut() {
        Ok(capture) if journaled(&path.to_string_lossy()) => with_journal_diagnostic(|journal| {
            journal.capture_side_file_after_moves_diagnostic(capture, path, epoch)
        })?,
        _ => Ok(()),
    });
    if let Ok(capture) = capture {
        let (renamed, directories) = result
            .as_ref()
            .map(|r| (r.renamed(), r.directories()))
            .unwrap_or_default();
        let _ = with_journal(|journal| journal.commit(capture, &renamed, &directories));
    }
    result
}

#[derive(Serialize)]
pub struct HistoryResult {
    #[serde(flatten)]
    result: ScanResult,
    renamed: Vec<PathChange>,
    moved_directories: Vec<PathChange>,
    label: String,
    history: HistoryState,
}

#[tauri::command]
pub fn history_state() -> Result<HistoryState, String> {
    with_journal(|journal| journal.state())
}

#[cfg(test)]
fn replay(backward: bool) -> Result<HistoryResult, String> {
    replay_diagnostic(backward).map(|report| report.result)
}
fn replay_diagnostic(
    backward: bool,
) -> Result<tag_core::batch_errors::Detailed<HistoryResult>, String> {
    let report = with_journal(|journal| {
        journal.set_tag_options(super::settings::tag_options());
        if backward {
            journal.undo_diagnostic()
        } else {
            journal.redo_diagnostic()
        }
    })??;
    let replay = report.result;
    super::library::changed(&replay.paths, &replay.renamed, &replay.directories);
    let audio_paths: Vec<_> = replay
        .paths
        .iter()
        .filter(|path| !crate::associated_files::is_companion(Path::new(path)))
        .cloned()
        .collect();
    let mut result = super::batch(&audio_paths, super::settings::read_summary);
    let offset = result.errors.len();
    let diagnostics = report
        .error_diagnostics
        .into_iter()
        .map(|mut item| {
            item.index += offset;
            item
        })
        .collect();
    result.errors.extend(replay.errors);
    let changes = |pairs: Vec<(String, String)>| {
        pairs
            .into_iter()
            .map(|(from, to)| PathChange { from, to })
            .collect()
    };
    Ok(tag_core::batch_errors::Detailed {
        result: HistoryResult {
            result,
            renamed: changes(replay.renamed),
            moved_directories: changes(replay.directories),
            label: replay.label,
            history: with_journal(|journal| journal.state())?,
        },
        error_diagnostics: diagnostics,
    })
}

#[cfg(test)]
#[tauri::command]
pub async fn undo_last() -> Result<HistoryResult, String> {
    super::io(|| replay(true)).await
}

#[cfg(test)]
#[tauri::command]
pub async fn redo_last() -> Result<HistoryResult, String> {
    super::io(|| replay(false)).await
}

/// Keep callable String/result APIs above; the same command identifiers return additive wire companions.
pub mod ipc {
    #[tauri::command(rename = "undo_last")]
    pub async fn undo_last_with_diagnostics(
    ) -> Result<tag_core::batch_errors::Detailed<super::HistoryResult>, String> {
        crate::io(|| super::replay_diagnostic(true)).await
    }
    #[tauri::command(rename = "redo_last")]
    pub async fn redo_last_with_diagnostics(
    ) -> Result<tag_core::batch_errors::Detailed<super::HistoryResult>, String> {
        crate::io(|| super::replay_diagnostic(false)).await
    }
    #[tauri::command(rename = "convert_tag_to_tag")]
    pub async fn convert_tag_to_tag_with_diagnostics(
        paths: Vec<String>,
        field: String,
        template: String,
        columns: Option<Vec<crate::evaluation::ColumnSpec>>,
    ) -> Result<tag_core::batch_errors::Detailed<crate::action_host::ActionResult>, String> {
        super::convert_tag_to_tag_diagnostic(paths, field, template, columns).await
    }
}

/// Convert → Tag - Tag: format one field (or `_FILENAME`/`_DIRECTORY`) of each
/// selected file from a format string, written immediately like Mp3tag.
#[cfg(test)]
#[tauri::command]
pub async fn convert_tag_to_tag(
    paths: Vec<String>,
    field: String,
    template: String,
    columns: Option<Vec<crate::evaluation::ColumnSpec>>,
) -> Result<ActionResult, String> {
    convert_tag_to_tag_diagnostic(paths, field, template, columns)
        .await
        .map(|report| report.result)
}
pub async fn convert_tag_to_tag_diagnostic(
    paths: Vec<String>,
    field: String,
    template: String,
    columns: Option<Vec<crate::evaluation::ColumnSpec>>,
) -> Result<tag_core::batch_errors::Detailed<ActionResult>, String> {
    let settings = super::evaluation_settings(columns)?;
    let field = field.trim().to_owned();
    if field.is_empty() {
        return Err("Enter a field name".into());
    }
    super::io(move || {
        super::reject_provider_operation(&paths, "Tag → Tag")?;
        let group = tag_core::actions::ActionGroup {
            name: "Tag → Tag".into(),
            enabled: true,
            actions: vec![tag_core::actions::ActionSpec::FormatValue { field, template }],
        };
        record("Tag → Tag", None, &paths, || {
            Ok(crate::action_host::execute_diagnostic(
                &paths,
                &[group],
                settings,
            ))
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tag_core::Fields;

    fn wav(path: &Path, title: &str) {
        let samples = vec![0u8; 8820];
        let mut data = b"RIFF".to_vec();
        data.extend((36 + samples.len() as u32).to_le_bytes());
        data.extend(b"WAVEfmt ");
        data.extend(16u32.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(44100u32.to_le_bytes());
        data.extend(88200u32.to_le_bytes());
        data.extend(2u16.to_le_bytes());
        data.extend(16u16.to_le_bytes());
        data.extend(b"data");
        data.extend((samples.len() as u32).to_le_bytes());
        data.extend(samples);
        std::fs::write(path, data).unwrap();
        tag_core::write_fields(path, &Fields::from([("TITLE".into(), vec![title.into()])]))
            .unwrap();
    }
    fn title(path: &Path) -> Vec<String> {
        tag_core::read_entry_summary(path).unwrap().fields["TITLE"].clone()
    }
    fn step() -> (String, usize) {
        let step = history_state().unwrap().undo.unwrap();
        (step.label, step.files)
    }
    fn run<T>(future: impl std::future::Future<Output = Result<T, String>>) -> T {
        tauri::async_runtime::block_on(future).unwrap()
    }

    #[test]
    fn diagnostic_replay_offsets_owned_errors_after_read_summary_failures() {
        let _serial = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let raw = root.join("Help; raw.txt");
        let raw_to = root.join("Title; moved.txt");
        std::fs::write(&raw, b"Not audio").unwrap();
        let audio = root.join("Help.wav");
        let audio_to = root.join("Title.wav");
        wav(&audio, "Original");
        let original = std::fs::read(&audio).unwrap();
        let mut journal = Journal::open(&root.join("undo"), Limits::default()).unwrap();
        let capture = journal.begin(
            "Help / action 1: Title",
            None,
            &[
                raw.to_string_lossy().into_owned(),
                audio.to_string_lossy().into_owned(),
            ],
        );
        tag_core::operations::rename_no_clobber(&raw, &raw_to).unwrap();
        tag_core::operations::rename_no_clobber(&audio, &audio_to).unwrap();
        journal.commit(
            capture,
            &[
                (
                    raw.to_string_lossy().into_owned(),
                    raw_to.to_string_lossy().into_owned(),
                ),
                (
                    audio.to_string_lossy().into_owned(),
                    audio_to.to_string_lossy().into_owned(),
                ),
            ],
            &[],
        );
        std::fs::write(&audio, b"External replacement").unwrap();
        struct Restore(Option<Journal>);
        impl Drop for Restore {
            fn drop(&mut self) {
                *JOURNAL.lock().unwrap_or_else(|error| error.into_inner()) = self.0.take();
            }
        }
        let _restore = Restore(JOURNAL.lock().unwrap().replace(journal));
        let report = tauri::async_runtime::block_on(ipc::undo_last_with_diagnostics()).unwrap();
        assert_eq!(report.result.result.errors.len(), 2);
        assert_eq!(report.error_diagnostics.len(), 1);
        assert_eq!(report.error_diagnostics[0].index, 1);
        assert_eq!(
            report.result.result.errors[1].error,
            report.error_diagnostics[0].diagnostic.to_string()
        );
        let wire = serde_json::to_value(&report).unwrap();
        assert_eq!(wire["error_diagnostics"][0]["index"], 1);
        assert_eq!(
            wire["errors"][1]["error"],
            report.result.result.errors[1].error
        );
        assert_eq!(std::fs::read(&raw).unwrap(), b"Not audio");
        assert_eq!(std::fs::read(&audio).unwrap(), b"External replacement");
        assert_eq!(std::fs::read(&audio_to).unwrap(), original);
    }

    fn check_new_cover_undo(move_folder: bool) {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let album = root.join("Album");
        std::fs::create_dir(&album).unwrap();
        let audio = album.join("a.wav");
        wav(&audio, "Original");
        let mut image = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut image, image::ImageFormat::Png)
            .unwrap();
        let image = image.into_inner();
        let input = root.join("input.png");
        std::fs::write(&input, &image).unwrap();
        tag_core::add_cover(&audio, &input).unwrap();
        let original = std::fs::read(&audio).unwrap();
        let mut actions = vec![tag_core::actions::ActionSpec::ExportCover {
            filename_template: "art/newcover".into(),
            picture_type: None,
            export_duplicates: false,
        }];
        if move_folder {
            actions.extend([
                tag_core::actions::ActionSpec::FormatValue {
                    field: "TITLE".into(),
                    template: "Changed".into(),
                },
                tag_core::actions::ActionSpec::FormatValue {
                    field: "_DIRECTORY".into(),
                    template: "Moved".into(),
                },
            ]);
        }
        let current = if move_folder {
            root.join("Moved")
        } else {
            album.clone()
        };
        let result = run(crate::execute_action_groups(
            vec![audio.to_string_lossy().into_owned()],
            vec![tag_core::actions::ActionGroup {
                name: "Create cover".into(),
                enabled: true,
                actions,
            }],
            None,
        ));
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(
            std::fs::read(current.join("art/newcover.png")).unwrap(),
            image
        );
        let written = std::fs::read(current.join("a.wav")).unwrap();
        let undone = run(undo_last());
        assert!(
            undone.result.errors.is_empty(),
            "{:?}",
            undone.result.errors
        );
        assert!(!album.join("art/newcover.png").exists());
        assert_eq!(std::fs::read(&audio).unwrap(), original);
        let redone = run(redo_last());
        assert!(
            redone.result.errors.is_empty(),
            "{:?}",
            redone.result.errors
        );
        assert_eq!(
            std::fs::read(current.join("art/newcover.png")).unwrap(),
            image
        );
        assert_eq!(std::fs::read(current.join("a.wav")).unwrap(), written);
        // Remove this test's final undo entry from the shared command journal.
        assert!(run(undo_last()).result.errors.is_empty());
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn action_cover_creation_only_has_undo_and_redo() {
        check_new_cover_undo(false);
    }

    #[test]
    fn action_cover_creation_then_folder_move_has_undo_and_redo() {
        check_new_cover_undo(true);
    }

    #[cfg(unix)]
    #[test]
    fn action_cover_symlink_replacement_is_journaled_through_the_command() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let a = root.join("a.wav");
        let b = root.join("b.wav");
        wav(&a, "Alpha");
        wav(&b, "Beta");
        let image = |width| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(width, width))
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            bytes.into_inner()
        };
        let (first, previous) = (image(2), image(3));
        let original = root.join("original.png");
        let first_image = root.join("first.png");
        std::fs::write(&original, &previous).unwrap();
        std::fs::write(&first_image, &first).unwrap();
        tag_core::add_cover(&a, &first_image).unwrap();
        tag_core::add_cover(&b, &original).unwrap();
        let target = root.join("shared.png");
        std::os::unix::fs::symlink("original.png", &target).unwrap();
        let text = |p: &Path| p.to_string_lossy().into_owned();
        let group = tag_core::actions::ActionGroup {
            name: "Cover link".into(),
            enabled: true,
            actions: vec![tag_core::actions::ActionSpec::ExportCover {
                filename_template: "shared".into(),
                picture_type: None,
                export_duplicates: false,
            }],
        };
        let result = run(crate::execute_action_groups(
            vec![text(&a), text(&b)],
            vec![group],
            None,
        ));
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert!(result.errors[0].path.ends_with("b.wav"));
        assert!(result.errors[0]
            .error
            .contains("already written for another file"));
        assert_eq!(std::fs::read(&target).unwrap(), first);
        assert!(!std::fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read(&original).unwrap(), previous);
        let undone = run(undo_last());
        assert!(
            undone.result.errors.is_empty(),
            "{:?}",
            undone.result.errors
        );
        assert_eq!(
            std::fs::read_link(&target).unwrap(),
            Path::new("original.png")
        );
        assert_eq!(std::fs::read(&target).unwrap(), previous);
        let redone = run(redo_last());
        assert!(
            redone.result.errors.is_empty(),
            "{:?}",
            redone.result.errors
        );
        assert!(!std::fs::symlink_metadata(&target)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read(&target).unwrap(), first);
        assert_eq!(std::fs::read(&original).unwrap(), previous);
        // An externally substituted link is a different directory entry,
        // even when it exposes the exact bytes the journal expects.
        std::fs::remove_file(&target).unwrap();
        std::os::unix::fs::symlink("first.png", &target).unwrap();
        let refused = run(undo_last());
        assert!(refused
            .result
            .errors
            .iter()
            .any(|error| error.error.contains("Changed outside")));
        assert_eq!(std::fs::read_link(&target).unwrap(), Path::new("first.png"));
        // The app's existing full-alias protection rejects a dangling
        // destination visibly before any cover write or link replacement.
        let dangling = root.join("dangling.png");
        std::os::unix::fs::symlink("missing.png", &dangling).unwrap();
        let result = run(crate::execute_action_groups(
            vec![text(&a)],
            vec![tag_core::actions::ActionGroup {
                name: "Dangling link".into(),
                enabled: true,
                actions: vec![tag_core::actions::ActionSpec::ExportCover {
                    filename_template: "dangling".into(),
                    picture_type: None,
                    export_duplicates: false,
                }],
            }],
            None,
        ));
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert_eq!(
            std::fs::read_link(&dangling).unwrap(),
            Path::new("missing.png")
        );
        assert!(!root.join("missing.png").exists());
        assert!(!std::fs::read_dir(&root).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".opentag-link-")));
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn action_cover_undo_follows_later_directory_moves_and_keeps_the_first_snapshot() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for (repeat_after_move, start_identical, move_folder) in [
            (false, false, true),
            (true, false, true),
            (true, true, true),
            (true, true, false),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let root = dunce::canonicalize(dir.path()).unwrap();
            TEST_ROOTS.lock().unwrap().push(root.clone());
            let album = root.join("Album");
            std::fs::create_dir(&album).unwrap();
            let audio = album.join("a.wav");
            wav(&audio, "Original");
            let image = |width| {
                let mut bytes = std::io::Cursor::new(Vec::new());
                image::DynamicImage::ImageRgb8(image::RgbImage::new(width, width))
                    .write_to(&mut bytes, image::ImageFormat::Png)
                    .unwrap();
                bytes.into_inner()
            };
            let embedded = image(2);
            let previous = if start_identical {
                embedded.clone()
            } else {
                image(3)
            };
            let later = image(4);
            let input = root.join("input.png");
            let next = root.join("next.png");
            std::fs::write(&input, &embedded).unwrap();
            std::fs::write(&next, &later).unwrap();
            tag_core::add_cover(&audio, &input).unwrap();
            let before_audio = std::fs::read(&audio).unwrap();
            std::fs::write(album.join("shared.png"), &previous).unwrap();
            let export = || tag_core::actions::ActionSpec::ExportCover {
                filename_template: "shared".into(),
                picture_type: None,
                export_duplicates: false,
            };
            let rename = |name: &str| tag_core::actions::ActionSpec::FormatValue {
                field: "_DIRECTORY".into(),
                template: name.into(),
            };
            let mut actions = vec![export()];
            if move_folder {
                actions.push(rename("Renamed"));
            }
            if repeat_after_move {
                actions.extend([
                    tag_core::actions::ActionSpec::ImportCover {
                        filename_template: next.to_string_lossy().into_owned(),
                        picture_type: None,
                        delete_existing: true,
                    },
                    export(),
                ]);
                if move_folder {
                    actions.push(rename("Final"));
                }
                actions.push(export());
            }
            let final_folder = if move_folder {
                root.join(if repeat_after_move {
                    "Final"
                } else {
                    "Renamed"
                })
            } else {
                album.clone()
            };
            let result = run(crate::execute_action_groups(
                vec![audio.to_string_lossy().into_owned()],
                vec![tag_core::actions::ActionGroup {
                    name: "Move cover folder".into(),
                    enabled: true,
                    actions,
                }],
                None,
            ));
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let after_image = if repeat_after_move { &later } else { &embedded };
            assert_eq!(
                std::fs::read(final_folder.join("shared.png")).unwrap(),
                *after_image
            );
            let after_audio = std::fs::read(final_folder.join("a.wav")).unwrap();
            if move_folder {
                assert!(!album.exists());
            }
            let undone = run(undo_last());
            assert!(
                undone.result.errors.is_empty(),
                "{:?}",
                undone.result.errors
            );
            assert_eq!(std::fs::read(album.join("shared.png")).unwrap(), previous);
            assert_eq!(std::fs::read(&audio).unwrap(), before_audio);
            if move_folder {
                assert!(!final_folder.exists());
            }
            let redone = run(redo_last());
            assert!(
                redone.result.errors.is_empty(),
                "{:?}",
                redone.result.errors
            );
            assert_eq!(
                std::fs::read(final_folder.join("shared.png")).unwrap(),
                *after_image
            );
            assert_eq!(
                std::fs::read(final_folder.join("a.wav")).unwrap(),
                after_audio
            );
            if move_folder {
                assert!(!album.exists());
            }
            // Leave no undo step in the shared command journal for the next
            // fixture; its first command will clear this final redo step.
            let cleanup = run(undo_last());
            assert!(
                cleanup.result.errors.is_empty(),
                "{:?}",
                cleanup.result.errors
            );
            TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
        }
    }

    #[test]
    fn cover_export_refuses_an_existing_output_before_exceeding_undo_budget() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let audio = root.join("a.wav");
        wav(&audio, "Original");
        let input = root.join("input.png");
        let mut image = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
            .write_to(&mut image, image::ImageFormat::Png)
            .unwrap();
        std::fs::write(&input, image.into_inner()).unwrap();
        tag_core::add_cover(&audio, &input).unwrap();
        let before = std::fs::read(&audio).unwrap();
        let output = root.join("cover.png");
        std::fs::write(&output, [7; 64]).unwrap();
        struct RestoreJournal(Option<Journal>);
        impl Drop for RestoreJournal {
            fn drop(&mut self) {
                *JOURNAL.lock().unwrap_or_else(|e| e.into_inner()) = self.0.take();
            }
        }
        let test_journal = Journal::open(
            &root.join("undo"),
            Limits {
                bytes: before.len() as u64 + 32,
                min_free: 0,
                ..Limits::default()
            },
        )
        .unwrap();
        let _restore = RestoreJournal(JOURNAL.lock().unwrap().replace(test_journal));
        let result = run(crate::execute_action_groups(
            vec![audio.to_string_lossy().into_owned()],
            vec![tag_core::actions::ActionGroup {
                name: "Limited cover".into(),
                enabled: true,
                actions: vec![tag_core::actions::ActionSpec::ExportCover {
                    filename_template: "cover".into(),
                    picture_type: None,
                    export_duplicates: false,
                }],
            }],
            None,
        ));
        assert_eq!(result.errors.len(), 1, "{:?}", result.errors);
        assert!(
            result.errors[0].error.contains("undo byte limit"),
            "{:?}",
            result.errors
        );
        assert_eq!(std::fs::read(&output).unwrap(), [7; 64]);
        assert_eq!(std::fs::read(&audio).unwrap(), before);
        assert!(history_state().unwrap().undo.is_none());
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn cover_undo_tracks_outputs_created_at_a_reused_folder_path() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        for obstruct in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let root = dunce::canonicalize(dir.path()).unwrap();
            TEST_ROOTS.lock().unwrap().push(root.clone());
            let album = root.join("Album");
            let moved = root.join("Moved");
            std::fs::create_dir(&album).unwrap();
            let audio = album.join("a.wav");
            wav(&audio, "Original");
            let input = root.join("input.png");
            let mut image = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(image::RgbImage::new(2, 2))
                .write_to(&mut image, image::ImageFormat::Png)
                .unwrap();
            let image = image.into_inner();
            std::fs::write(&input, &image).unwrap();
            tag_core::add_cover(&audio, &input).unwrap();
            let before = std::fs::read(&audio).unwrap();
            std::fs::write(album.join("shared.png"), b"Original output").unwrap();
            let export = |name| tag_core::actions::ActionSpec::ExportCover {
                filename_template: name,
                picture_type: None,
                export_duplicates: false,
            };
            let result = run(crate::execute_action_groups(
                vec![audio.to_string_lossy().into_owned()],
                vec![tag_core::actions::ActionGroup {
                    name: "Reuse old folder".into(),
                    enabled: true,
                    actions: vec![
                        export("shared".into()),
                        tag_core::actions::ActionSpec::FormatValue {
                            field: "_DIRECTORY".into(),
                            template: "Moved".into(),
                        },
                        export(album.join("nested/new").to_string_lossy().into_owned()),
                    ],
                }],
                None,
            ));
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            assert_eq!(std::fs::read(moved.join("shared.png")).unwrap(), image);
            assert_eq!(std::fs::read(album.join("nested/new.png")).unwrap(), image);
            if obstruct {
                std::fs::write(album.join("external.txt"), b"External data").unwrap();
            }
            let undone = run(undo_last());
            if obstruct {
                assert!(!undone.result.errors.is_empty());
                assert_eq!(
                    std::fs::read(album.join("external.txt")).unwrap(),
                    b"External data"
                );
                assert_eq!(
                    std::fs::read(album.join("nested/new.png")).unwrap(),
                    image,
                    "rolled back side-file removal"
                );
                assert_eq!(std::fs::read(moved.join("shared.png")).unwrap(), image);
                assert_eq!(std::fs::read(moved.join("a.wav")).unwrap(), before);
            } else {
                assert!(
                    undone.result.errors.is_empty(),
                    "{:?}",
                    undone.result.errors
                );
                assert_eq!(
                    std::fs::read(album.join("shared.png")).unwrap(),
                    b"Original output"
                );
                assert_eq!(std::fs::read(&audio).unwrap(), before);
                assert!(!album.join("nested/new.png").exists() && !moved.exists());
                let redone = run(redo_last());
                assert!(
                    redone.result.errors.is_empty(),
                    "{:?}",
                    redone.result.errors
                );
                assert_eq!(std::fs::read(moved.join("shared.png")).unwrap(), image);
                assert_eq!(std::fs::read(album.join("nested/new.png")).unwrap(), image);
                assert_eq!(std::fs::read(moved.join("a.wav")).unwrap(), before);
                assert!(run(undo_last()).result.errors.is_empty());
            }
            TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
        }
    }

    #[test]
    fn tag_to_tag_metadata_outputs_follow_preview_contract() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let audio = root.join("preview.wav");
        wav(&audio, "Unchanged");
        let path = audio.to_string_lossy().into_owned();
        let separator = char::from(92).to_string().repeat(2);
        let cases = [
            (separator.clone(), vec![]),
            (
                format!(" Jazz {separator} {separator} Blues{separator}"),
                vec!["Jazz", "Blues"],
            ),
            (" Live ".into(), vec![" Live "]),
            (format!("AC{}DC", char::from(92)), vec![r"AC\DC"]),
            (
                format!("{0}Jazz{0}{separator}{0}", char::from(0x85)),
                vec!["Jazz"],
            ),
            (
                format!("{0}Jazz{0}{separator}", char::from_u32(0xfeff).unwrap()),
                vec!["\u{feff}Jazz\u{feff}"],
            ),
        ];
        for (template, expected) in cases {
            tag_core::write_fields(
                &audio,
                &Fields::from([("GENRE".into(), vec!["Previous".into()])]),
            )
            .unwrap();
            let before = std::fs::read(&audio).unwrap();
            let preview = run(crate::preview_format(
                vec![path.clone()],
                template.clone(),
                None,
                Some(false),
            ));
            assert_eq!(preview[0].value, template);
            let result = run(convert_tag_to_tag(
                vec![path.clone()],
                "GENRE".into(),
                template.clone(),
                None,
            ));
            assert!(result.errors.is_empty(), "{:?}", result.errors);
            let fields = tag_core::read_entry_summary(&audio).unwrap().fields;
            assert_eq!(
                fields.get("GENRE").map(Vec::as_slice).unwrap_or(&[]),
                expected,
                "{template:?}"
            );
            assert_eq!(fields["TITLE"], ["Unchanged"]);
            let undo = run(undo_last());
            assert!(undo.result.errors.is_empty(), "{:?}", undo.result.errors);
            assert_eq!(std::fs::read(&audio).unwrap(), before);
        }
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn commands_record_grouped_saves_tag_conversions_and_renames() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let a = root.join("a.wav");
        let b = root.join("b.wav");
        wav(&a, "Alpha");
        wav(&b, "Beta");
        let key = |p: &Path| p.to_string_lossy().into_owned();
        let original = (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());

        // One Save click writes files one at a time but is one undo step.
        for (path, value) in [(&a, "One"), (&b, "Two")] {
            let saved = run(crate::save_fields(
                vec![key(path)],
                Fields::from([("TITLE".into(), vec![value.into()])]),
                Some("save-click".into()),
            ));
            assert!(saved.errors.is_empty());
        }
        assert_eq!(step(), ("Save tags".into(), 2));

        let both = vec![key(&a), key(&b)];
        let formatted = run(convert_tag_to_tag(
            both.clone(),
            "ALBUM".into(),
            "$upper(%title%) album".into(),
            None,
        ));
        assert!(formatted.errors.is_empty(), "{:?}", formatted.errors);
        assert_eq!(
            tag_core::read_entry_summary(&a).unwrap().fields["ALBUM"],
            ["ONE album"]
        );
        assert_eq!(step(), ("Tag → Tag".into(), 2));

        let renamed = run(convert_tag_to_tag(
            vec![key(&a)],
            "_FILENAME".into(),
            "%title% renamed".into(),
            None,
        ));
        let moved = root.join("One renamed.wav");
        assert_eq!(renamed.renamed[0].to, key(&moved));
        assert!(moved.exists() && !a.exists());

        let undone = run(undo_last());
        assert_eq!(undone.label, "Tag → Tag");
        assert_eq!(undone.renamed[0].from, key(&moved));
        assert_eq!(undone.renamed[0].to, key(&a));
        assert_eq!(undone.result.entries[0].path, key(&a));
        assert!(a.exists() && !moved.exists());
        run(undo_last());
        assert!(!tag_core::read_entry_summary(&a)
            .unwrap()
            .fields
            .contains_key("ALBUM"));
        let undone = run(undo_last());
        assert_eq!(undone.label, "Save tags");
        assert_eq!(undone.result.entries.len(), 2);
        assert_eq!(
            (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap()),
            original
        );
        assert!(undone.history.undo.is_none());
        assert_eq!(undone.history.redo.unwrap().label, "Save tags");

        for _ in 0..3 {
            let redone = run(redo_last());
            assert!(
                redone.result.errors.is_empty(),
                "{:?}",
                redone.result.errors
            );
        }
        assert!(moved.exists());
        assert_eq!(title(&moved), ["One"]);
        assert_eq!(title(&b), ["Two"]);

        // Tag → Filename renames are reversed; external edits are reported.
        let preview = run(crate::preview_rename(vec![key(&b)], "Second".into(), None));
        run(crate::rename_files(
            vec![key(&b)],
            "Second".into(),
            preview[0].plan_token.clone(),
        ));
        let second = root.join("Second.wav");
        assert_eq!(step(), ("Tag → Filename".into(), 1));
        tag_core::write_fields(
            &second,
            &Fields::from([("TITLE".into(), vec!["Else".into()])]),
        )
        .unwrap();
        let refused = run(undo_last());
        assert!(refused.result.errors[0].error.contains("Changed outside"));
        assert!(second.exists() && !b.exists());
        assert_eq!(title(&second), ["Else"]);
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }

    #[test]
    fn filename_to_tag_skips_files_that_do_not_match_in_one_undo_step() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let key = |p: &Path| p.to_string_lossy().into_owned();
        let a = root.join("Ada - One.wav");
        let odd = root.join("no separator.wav");
        let b = root.join("Bob - Two.wav");
        for path in [&a, &odd, &b] {
            wav(path, "Old");
        }
        let skipped = std::fs::read(&odd).unwrap();
        let originals = (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        let paths = vec![key(&a), key(&odd), key(&b)];

        let preview = run(crate::preview_filename(
            paths.clone(),
            "%artist% - %title%".into(),
        ));
        assert_eq!(preview.len(), 3);
        assert_eq!(preview[0].fields.as_ref().unwrap()["ARTIST"], ["Ada"]);
        assert!(preview[1].fields.is_none() && preview[1].error.is_none());
        assert_eq!(preview[2].fields.as_ref().unwrap()["TITLE"], ["Two"]);

        let imported = run(crate::filename_to_tags(paths, "%artist% - %title%".into()));
        assert!(
            imported.result.errors.is_empty(),
            "{:?}",
            imported.result.errors
        );
        assert_eq!(imported.unmatched, [key(&odd)]);
        let written: Vec<_> = imported.result.entries.iter().map(|e| &e.path).collect();
        assert_eq!(written, [&key(&a), &key(&b)]);
        assert_eq!(title(&a), ["One"]);
        assert_eq!(title(&b), ["Two"]);
        assert_eq!(std::fs::read(&odd).unwrap(), skipped);
        assert_eq!(step(), ("Filename → Tag".into(), 2));

        let undone = run(undo_last());
        assert_eq!(undone.label, "Filename → Tag");
        assert_eq!(
            (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap()),
            originals
        );
        // A pattern error is still an error for the whole conversion.
        assert!(tauri::async_runtime::block_on(crate::filename_to_tags(
            vec![key(&a)],
            "%artist%%title%".into()
        ))
        .is_err());
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }
    #[test]
    fn companion_moves_and_case_only_renames_undo_without_audio_parse_errors() {
        let _journal = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        TEST_ROOTS.lock().unwrap().push(root.clone());
        let key = |p: &Path| p.to_string_lossy().into_owned();
        for destination in [root.join("SONG.wav"), root.join("Moved/renamed.wav")] {
            let audio = root.join("Song.wav");
            let lyric = root.join("Song.lrc");
            wav(&audio, "Title");
            std::fs::write(&lyric, b"lyrics").unwrap();
            let original = std::fs::read(&audio).unwrap();
            let plan =
                crate::associated_files::Plan::new(vec![(key(&audio), destination.clone())], true)
                    .unwrap();
            let group = &plan.groups[0];
            let moved_lyric = group.associated[0].target.clone();
            let targets = vec![key(&audio), key(&lyric)];
            let result = record("Move files", None, &targets, || {
                group.execute(&mut tag_core::operations::rename_no_clobber)?;
                Ok(crate::file_ops::TransferResult {
                    result: ScanResult {
                        entries: vec![],
                        errors: vec![],
                    },
                    copied: vec![],
                    renamed: vec![crate::file_ops::PathChange {
                        from: key(&audio),
                        to: key(&destination),
                    }],
                    associated_renamed: vec![crate::file_ops::PathChange {
                        from: key(&lyric),
                        to: key(&moved_lyric),
                    }],
                })
            })
            .unwrap();
            assert_eq!(result.associated_renamed.len(), 1);
            let undone = run(undo_last());
            assert!(
                undone.result.errors.is_empty(),
                "{:?}",
                undone.result.errors
            );
            assert_eq!(undone.result.entries.len(), 1);
            assert_eq!(std::fs::read(&audio).unwrap(), original);
            assert_eq!(std::fs::read(&lyric).unwrap(), b"lyrics");
            let names: Vec<_> = std::fs::read_dir(&root)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert!(names.contains(&std::ffi::OsString::from("Song.lrc")));
            let redone = run(redo_last());
            assert!(
                redone.result.errors.is_empty(),
                "{:?}",
                redone.result.errors
            );
            assert_eq!(std::fs::read(&moved_lyric).unwrap(), b"lyrics");
            // A companion edited elsewhere must be retained, even if the audio can undo.
            std::fs::write(&moved_lyric, b"external lyric edit").unwrap();
            let refused = run(undo_last());
            assert!(refused
                .result
                .errors
                .iter()
                .any(|e| e.error.contains("Changed outside")));
            assert_eq!(std::fs::read(&moved_lyric).unwrap(), b"external lyric edit");
            std::fs::remove_file(&audio).unwrap();
            std::fs::remove_file(&moved_lyric).unwrap();
        }
        TEST_ROOTS.lock().unwrap().retain(|r| r != &root);
    }
}
