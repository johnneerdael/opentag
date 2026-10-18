use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tag_core::{Action, AudioEntry, Fields, FileError, ScanResult};
mod action_host;
mod columns;
mod converters;
mod cover_drag;
mod covers;
mod documents;
mod e2e;
mod evaluation;
mod exports;
mod file_hashes;
mod file_ops;
mod filter_cache;
mod history;
mod import_diagnostics;
mod languages;
mod library;
mod list_behaviour;
mod lists;
#[cfg(any(target_os = "android", test))]
mod managed_actions;
mod mp3tag_columns;
mod mp3tag_tag_panel;
mod playlists;
mod progress;
mod rename_plan;
mod settings;
mod sources;
mod tag_panel;
mod tray;
mod updates;
mod utilities;
#[cfg(desktop)]
mod window_state;

fn export_library_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(action_host::library_path(app)?.with_file_name("exports.json"))
}
#[tauri::command]
async fn load_export_library(app: tauri::AppHandle) -> Result<exports::Library, String> {
    let path = export_library_path(&app)?;
    io(move || exports::load(&path)).await
}
#[tauri::command]
async fn save_export_library(
    app: tauri::AppHandle,
    library: exports::Library,
) -> Result<(), String> {
    let path = export_library_path(&app)?;
    io(move || exports::save(&path, &library)).await
}
#[tauri::command]
async fn import_export_configuration(
    path: String,
    legacy_encoding: Option<String>,
) -> Result<exports::Configuration, String> {
    io(move || exports::import(&auxiliary_input(&path)?, legacy_encoding.as_deref())).await
}
#[tauri::command]
async fn run_export(
    app: tauri::AppHandle,
    paths: Vec<String>,
    columns: Option<Vec<evaluation::ColumnSpec>>,
    per_directory: Option<bool>,
) -> Result<action_host::ActionResult, String> {
    let path = export_library_path(&app)?;
    let mut settings = evaluation_settings(columns)?;
    io(move || {
        reject_provider_operation(&paths, "Export")?;
        if paths.is_empty() {
            return Err("Select files to export".into());
        }
        let library = exports::load(&path)?;
        let configuration = library
            .configurations
            .iter()
            .find(|c| Some(&c.id) == library.preferences.configuration.as_ref())
            .ok_or("Select an export configuration")?;
        (
            settings.export_write_bom,
            settings.export_create_directories,
        ) = exports::current_output_options(&library.preferences);
        settings.export_template_inline = true;
        let (filename, partition) = if per_directory.unwrap_or(false) {
            let (filename, partition) = exports::per_directory(
                &library.preferences.filename,
                &configuration.template,
                &paths,
            );
            (filename, Some(partition))
        } else {
            (
                library.preferences.filename.clone(),
                library.preferences.partition.clone(),
            )
        };
        Ok(action_host::execute(
            &paths,
            &[tag_core::actions::ActionGroup {
                name: format!("Export {}", configuration.name),
                enabled: true,
                actions: vec![tag_core::actions::ActionSpec::Export {
                    template: configuration.template.clone(),
                    filename_template: filename,
                    append: library.preferences.append,
                    partition_template: partition,
                }],
            }],
            settings,
        ))
    })
    .await
}

#[tauri::command]
async fn load_action_library(
    app: tauri::AppHandle,
) -> Result<tag_core::actions::ActionLibrary, String> {
    let path = action_host::library_path(&app)?;
    io(move || action_host::load_library(&path)).await
}
#[tauri::command]
async fn save_action_library(
    app: tauri::AppHandle,
    library: tag_core::actions::ActionLibrary,
) -> Result<(), String> {
    let path = action_host::library_path(&app)?;
    io(move || action_host::save_library(&path, &library)).await
}
#[cfg(test)]
#[tauri::command]
async fn import_action_group(
    path: String,
    legacy_encoding: Option<String>,
) -> Result<tag_core::actions::MtaImport, String> {
    io(move || {
        let input = auxiliary_input(&path)?;
        action_host::import_group(&input, legacy_encoding.as_deref())
    })
    .await
}
#[tauri::command(rename = "import_action_group")]
async fn import_action_group_with_diagnostics(
    path: String,
    legacy_encoding: Option<String>,
) -> Result<tag_core::actions::MtaImportReport, import_diagnostics::ImportFailure> {
    io(move || {
        let input = auxiliary_input(&path)?;
        Ok(action_host::import_group_diagnostic(
            &input,
            legacy_encoding.as_deref(),
        ))
    })
    .await
    .map_err(import_diagnostics::ImportFailure::Plain)?
    .map_err(import_diagnostics::ImportFailure::parts)
}
/// Import several `.mta` files, or the `.mta` files of folders, at once.
#[cfg(test)]
#[tauri::command]
async fn import_action_groups(
    paths: Vec<String>,
    legacy_encoding: Option<String>,
) -> Result<Vec<action_host::GroupFileImport>, String> {
    io(move || {
        let mut results = Vec::with_capacity(paths.len());
        // An input that cannot be read is one failed entry, not a failed call.
        for path in paths {
            match auxiliary_group_input(&path) {
                Ok((input, name)) => {
                    let mut imported =
                        action_host::import_groups(&[input], legacy_encoding.as_deref());
                    if path.starts_with("content://") {
                        for result in &mut imported {
                            result.path = path.clone();
                            if let (Some(imported), Some(name)) = (&mut result.imported, &name) {
                                if let Some(stem) =
                                    Path::new(name).file_stem().filter(|stem| !stem.is_empty())
                                {
                                    imported.group.name = stem.to_string_lossy().into_owned();
                                }
                            }
                        }
                    }
                    results.extend(imported);
                }
                Err(error) => results.push(action_host::GroupFileImport {
                    path,
                    imported: None,
                    error: Some(error),
                }),
            }
        }
        Ok(results)
    })
    .await
}
#[tauri::command(rename = "import_action_groups")]
async fn import_action_groups_with_diagnostics(
    paths: Vec<String>,
    legacy_encoding: Option<String>,
) -> Result<Vec<action_host::GroupFileImportReport>, String> {
    io(move || {
        let mut results = Vec::with_capacity(paths.len());
        // An input that cannot be read is one failed entry, not a failed call.
        for path in paths {
            match auxiliary_group_input(&path) {
                Ok((input, name)) => {
                    let mut imported =
                        action_host::import_groups_diagnostic(&[input], legacy_encoding.as_deref());
                    if path.starts_with("content://") {
                        for result in &mut imported {
                            result.result.path = path.clone();
                            if let (Some(imported), Some(name)) =
                                (&mut result.result.imported, &name)
                            {
                                if let Some(stem) =
                                    Path::new(name).file_stem().filter(|stem| !stem.is_empty())
                                {
                                    imported.group.name = stem.to_string_lossy().into_owned();
                                }
                            }
                        }
                    }
                    results.extend(imported);
                }
                Err(error) => results.push(action_host::GroupFileImportReport {
                    result: action_host::GroupFileImport {
                        path,
                        imported: None,
                        error: Some(error),
                    },
                    issue_diagnostics: Vec::new(),
                    error_cause: None,
                }),
            }
        }
        Ok(results)
    })
    .await
}
#[cfg(test)]
#[tauri::command]
async fn execute_action_groups(
    paths: Vec<String>,
    groups: Vec<tag_core::actions::ActionGroup>,
    columns: Option<Vec<evaluation::ColumnSpec>>,
) -> Result<action_host::ActionResult, String> {
    execute_action_groups_diagnostic(paths, groups, columns)
        .await
        .map(|report| report.result)
}
async fn execute_action_groups_diagnostic(
    paths: Vec<String>,
    groups: Vec<tag_core::actions::ActionGroup>,
    columns: Option<Vec<evaluation::ColumnSpec>>,
) -> Result<tag_core::batch_errors::Detailed<action_host::ActionResult>, String> {
    let settings = evaluation_settings(columns)?;
    io(move || {
        reject_provider_operation(&paths, "Action groups")?;
        let label = match groups.iter().filter(|g| g.enabled).collect::<Vec<_>>()[..] {
            [group] => format!("Action {}", group.name),
            ref enabled => format!("Actions ({} groups)", enabled.len()),
        };
        history::record_with_side_file_epochs_diagnostic(&label, None, &paths, |before_replace| {
            Ok(action_host::execute_with_side_file_epochs_diagnostic(
                &paths,
                &groups,
                settings,
                before_replace,
            ))
        })
    })
    .await
}

// Serialize operations to prevent two UI commands replacing each other's edits.
static FILE_OPERATIONS: Mutex<()> = Mutex::new(());
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
/// Folder the file list was last changed to, reported by the window.
static WORKING_DIRECTORY: Mutex<Option<String>> = Mutex::new(None);
fn evaluation_settings(
    columns: Option<Vec<evaluation::ColumnSpec>>,
) -> Result<evaluation::EvaluationSettings, String> {
    let mut settings = match APP_HANDLE.get() {
        Some(app) => evaluation::EvaluationSettings::for_app(app, columns)?,
        None => evaluation::EvaluationSettings {
            columns: columns.unwrap_or_default(),
            ..Default::default()
        },
    };
    settings.working_directory = WORKING_DIRECTORY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    Ok(settings)
}

#[tauri::command]
fn set_working_directory(path: Option<String>) {
    *WORKING_DIRECTORY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = path.filter(|path| !path.is_empty());
}

#[derive(Serialize)]
struct RuntimePlatform {
    family: &'static str,
    folder_picker_supported: bool,
    document_access_supported: bool,
}

#[tauri::command]
fn runtime_platform() -> RuntimePlatform {
    let family = if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "ios") {
        "ios"
    } else {
        "desktop"
    };
    RuntimePlatform {
        family,
        folder_picker_supported: family == "desktop",
        document_access_supported: family == "android",
    }
}

#[tauri::command]
async fn get_covers(path: String) -> Result<Vec<tag_core::CoverInfo>, String> {
    io(move || {
        require_audio_handle(Path::new(&path))?;
        settings::read_full(Path::new(&path)).map(|e| e.covers)
    })
    .await
}

static FILTER_CACHE: Mutex<Option<filter_cache::FilterCache>> = Mutex::new(None);

/// Match the listed files against `query`. The window sends only files whose
/// filter fields changed since its previous query (see `filter_cache`).
#[tauri::command]
async fn filter_matches(
    query: String,
    request: filter_cache::FilterRequest,
) -> Result<filter_cache::FilterOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut cache = FILTER_CACHE
            .lock()
            .map_err(|_| "Filter cache lock failed".to_string())?;
        cache
            .get_or_insert_with(Default::default)
            .matches(&query, request, |columns| {
                evaluation_settings(Some(columns))
            })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn preview_numbering(
    items: Vec<tag_core::numbering::NumberingItem>,
    options: tag_core::numbering::NumberingOptions,
) -> Result<Vec<tag_core::numbering::NumberingUpdate>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        tag_core::numbering::plan_numbering(&items, &options)
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn io<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = FILE_OPERATIONS
            .lock()
            .map_err(|_| "File operation lock failed".to_string())?;
        if let Some(app) = APP_HANDLE.get() {
            documents::initialize(app)?;
        }
        work()
    })
    .await
    .map_err(|e| e.to_string())?
}

// Resolve auxiliary URIs before entering a pure audio mutation closure.
fn auxiliary_input(path: &str) -> Result<PathBuf, String> {
    #[cfg(not(target_os = "android"))]
    if !path.starts_with("content://") {
        return Ok(PathBuf::from(path));
    }
    #[cfg(test)]
    if let Some(result) = documents::command_fixture::snapshot_input(path) {
        return result;
    }
    documents::snapshot_input(
        APP_HANDLE.get().ok_or("Document access is unavailable")?,
        path,
    )
}
// Bulk group imports also need the display filename already returned by the
// provider snapshot; the private snapshot's UUID is not an action group name.
fn auxiliary_group_input(path: &str) -> Result<(PathBuf, Option<String>), String> {
    #[cfg(not(target_os = "android"))]
    if !path.starts_with("content://") {
        return Ok((PathBuf::from(path), None));
    }
    #[cfg(test)]
    if let Some(result) = documents::command_fixture::snapshot_input_with_name(path) {
        return result.map(|(path, name)| (path, Some(name)));
    }
    documents::snapshot_input_with_name(
        APP_HANDLE.get().ok_or("Document access is unavailable")?,
        path,
    )
}
fn mutate_document(
    path: &Path,
    work: impl FnOnce(&Path) -> Result<AudioEntry, String>,
) -> Result<AudioEntry, String> {
    #[cfg(target_os = "android")]
    return documents::mutate(path, work);
    #[cfg(not(target_os = "android"))]
    if documents::is_managed(path) {
        documents::mutate(path, work)
    } else {
        work(path)
    }
}
fn require_audio_handle(path: &Path) -> Result<(), String> {
    if cfg!(target_os = "android") && !documents::is_managed(path) {
        return Err("Open audio with the provider document picker before editing".into());
    }
    Ok(())
}
fn provider_filename(path: &Path) -> Result<String, String> {
    require_audio_handle(path)?;
    if documents::is_managed(path) {
        let mut entry = settings::read_summary(path)?;
        documents::project(&mut entry);
        Ok(entry.filename)
    } else {
        // A chapter row is named after its file.
        let path = path.to_string_lossy();
        Ok(tag_core::subsong_parts(&path).map_or(path.to_string(), |(file, _)| file.to_owned()))
    }
}
fn preview_technical_fields(entry: &AudioEntry, index: usize) -> Fields {
    let mut technical = evaluation::technical_fields(entry, index);
    if documents::is_managed(Path::new(&entry.path)) {
        for key in [
            "_FOLDERPATH",
            "_FOLDERPATH_REL",
            "_DIRECTORY",
            "_PARENT_DIRECTORY",
        ] {
            technical.remove(key);
        }
    }
    technical
}
fn reject_provider_operation(paths: &[String], operation: &str) -> Result<(), String> {
    if cfg!(target_os = "android")
        || paths
            .iter()
            .any(|p| p.starts_with("content://") || documents::is_managed(Path::new(p)))
    {
        return Err(format!(
            "{operation} is not yet supported for provider documents"
        ));
    }
    Ok(())
}
#[tauri::command]
async fn pick_document_files(app: tauri::AppHandle) -> Result<ScanResult, String> {
    io(move || documents::pick(&app)).await
}
#[tauri::command]
async fn document_states(
) -> Result<std::collections::BTreeMap<String, documents::DocumentState>, String> {
    io(documents::states).await
}
#[tauri::command]
async fn document_save_copy(
    path: String,
    updates: Fields,
    covers: Option<Vec<list_behaviour::PendingCover>>,
    version: String,
) -> Result<ScanResult, String> {
    io(move || documents::save_copy(&path, &updates, covers.as_deref(), &version)).await
}
#[tauri::command]
async fn document_working_entries(paths: Vec<String>) -> Result<ScanResult, String> {
    io(move || Ok(batch(&paths, documents::read_working))).await
}
#[tauri::command]
async fn restore_document_recovery() -> Result<ScanResult, String> {
    io(documents::restore_recovery).await
}

fn batch(
    paths: &[String],
    mut work: impl FnMut(&Path) -> Result<AudioEntry, String>,
) -> ScanResult {
    let mut result = ScanResult {
        entries: vec![],
        errors: vec![],
    };
    let list_chapters = settings::tag_options().advanced.list_chapters;
    // A cue sheet (an undone cue sheet edit) stands for its track rows.
    let mut targets = Vec::new();
    for path in paths {
        if tag_core::playlist::playlist_kind(Path::new(path))
            != Some(tag_core::playlist::PlaylistKind::Cue)
        {
            targets.push(path.clone());
            continue;
        }
        match tag_core::playlist::load_playlist(Path::new(path), None) {
            Ok(loaded) => targets.extend(
                loaded
                    .files
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .filter(|p| tag_core::subsong_parts(p).is_some()),
            ),
            Err(error) => result.errors.push(FileError {
                path: path.clone(),
                error,
            }),
        }
    }
    let mut seen = std::collections::HashSet::new();
    for path in &targets {
        if !seen.insert(path) {
            continue;
        }
        match work(Path::new(path)) {
            Ok(mut entry) => {
                documents::project(&mut entry);
                // With List chapters as separate files, a file is its chapter rows.
                if list_chapters && !entry.chapters.is_empty() {
                    result.entries.extend(tag_core::chapter_rows(&entry));
                } else {
                    result.entries.push(entry);
                }
            }
            Err(error) => result.errors.push(FileError {
                path: path.clone(),
                error,
            }),
        }
    }
    result
}

/// [`batch`] for commands that work on whole files (Remove tag, covers,
/// Utils): a chapter row stands for its file, which is processed once for
/// all its rows and comes back as its rows (or as the file when it no longer
/// has chapters). Rows of a loaded cue sheet edit the sheet, so whole-file
/// commands are refused for them.
fn whole_file_batch(
    paths: &[String],
    work: impl FnMut(&Path) -> Result<AudioEntry, String>,
) -> ScanResult {
    let mut files: Vec<String> = Vec::new();
    let mut refused = Vec::new();
    for path in paths {
        let file = match tag_core::subsong_parts(path) {
            Some((file, _))
                if tag_core::playlist::playlist_kind(Path::new(file))
                    == Some(tag_core::playlist::PlaylistKind::Cue) =>
            {
                refused.push(FileError {
                    path: path.clone(),
                    error: CUE_ROW_OPERATION.into(),
                });
                continue;
            }
            Some((file, _)) => file.to_owned(),
            None => path.clone(),
        };
        if !files.contains(&file) {
            files.push(file);
        }
    }
    let mut result = batch(&files, work);
    result.errors.extend(refused);
    result
}
const CUE_ROW_OPERATION: &str =
    "Cue sheet tracks only support editing tag fields, which are written into the cue sheet";

/// Paths split into ordinary files and chapter rows grouped by their file
/// (rows of one file are written together), each with its original index.
type RowGroup = (String, Vec<(usize, usize)>);
fn group_rows(paths: &[String]) -> (Vec<usize>, Vec<RowGroup>) {
    let mut plain = Vec::new();
    let mut groups: Vec<RowGroup> = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        match tag_core::subsong_parts(path) {
            Some((file, number)) => match groups.iter_mut().find(|(f, _)| f == file) {
                Some((_, rows)) => rows.push((index, number)),
                None => groups.push((file.to_owned(), vec![(index, number)])),
            },
            None => plain.push(index),
        }
    }
    (plain, groups)
}

/// Write the chapter rows of one file; one outcome per row, in row order.
fn write_row_group(
    file: &str,
    rows: &[(usize, Fields)],
    paths: &[String],
) -> Vec<Result<AudioEntry, String>> {
    let options = settings::tag_options();
    if documents::is_managed(Path::new(file)) {
        return rows
            .iter()
            .map(|_| Err("Chapter rows of provider documents cannot be edited".into()))
            .collect();
    }
    let numbered: Vec<(usize, Fields)> = rows
        .iter()
        .map(|(index, updates)| {
            let number = tag_core::subsong_parts(&paths[*index]).map_or(0, |(_, n)| n);
            (number, updates.clone())
        })
        .collect();
    match tag_core::write_chapter_rows(Path::new(file), &numbered, false, &options) {
        Ok(entries) => entries.into_iter().map(Ok).collect(),
        Err(error) => rows.iter().map(|_| Err(error.clone())).collect(),
    }
}

/// `job` names the operation for progress events and [`progress::cancel_job`].
/// `refresh` (File → Reload tags) reads every file even when Options →
/// Library holds it.
#[tauri::command]
async fn scan_files(
    paths: Vec<String>,
    recursive: bool,
    job: Option<String>,
    refresh: Option<bool>,
) -> Result<ScanResult, String> {
    io(move || {
        #[cfg(target_os = "android")]
        {
            let _ = (recursive, job, refresh);
            Ok(batch(&paths, documents::reload))
        }
        #[cfg(not(target_os = "android"))]
        {
            let job = progress::Job::new(job);
            let (managed, ordinary): (Vec<_>, Vec<_>) = paths
                .into_iter()
                .partition(|p| documents::is_managed(Path::new(p)));
            let mut result = library::scan(
                &ordinary,
                recursive,
                &settings::tag_options(),
                settings::extensions().as_deref(),
                &job,
                refresh.unwrap_or(false),
            );
            let restored = batch(&managed, documents::reload);
            result.entries.extend(restored.entries);
            result.errors.extend(restored.errors);
            for entry in &mut result.entries {
                documents::project(entry);
            }
            Ok(result)
        }
    })
    .await
}

#[tauri::command]
async fn save_fields(
    paths: Vec<String>,
    updates: Fields,
    history_group: Option<String>,
) -> Result<ScanResult, String> {
    io(move || {
        history::record("Save tags", history_group.as_deref(), &paths, || {
            let (plain, groups) = group_rows(&paths);
            let plain: Vec<String> = plain.into_iter().map(|i| paths[i].clone()).collect();
            let mut result = batch(&plain, |p| {
                mutate_document(p, |p| settings::write_summary(p, &updates))
            });
            for (file, rows) in groups {
                let rows: Vec<(usize, Fields)> = rows
                    .into_iter()
                    .map(|(i, _)| (i, updates.clone()))
                    .collect();
                for ((index, _), outcome) in rows.iter().zip(write_row_group(&file, &rows, &paths))
                {
                    match outcome {
                        Ok(entry) => result.entries.push(entry),
                        Err(error) => result.errors.push(FileError {
                            path: paths[*index].clone(),
                            error,
                        }),
                    }
                }
            }
            Ok(result)
        })
    })
    .await
}

#[derive(Deserialize)]
struct FieldUpdate {
    path: String,
    updates: Fields,
    /// Pasted covers that replace the file's covers (Paste/Cut tags).
    #[serde(default)]
    covers: Option<Vec<list_behaviour::PendingCover>>,
}

/// Save per-file field changes as one undo step, writing files in parallel
/// with progress. A cancelled save leaves the remaining files untouched and
/// returns the files written so far.
#[tauri::command]
async fn save_fields_batch(
    items: Vec<FieldUpdate>,
    job: Option<String>,
) -> Result<ScanResult, String> {
    io(move || {
        let job = progress::Job::new(job);
        let mut seen = std::collections::HashSet::new();
        let items: Vec<FieldUpdate> = items
            .into_iter()
            .filter(|item| seen.insert(item.path.clone()))
            .collect();
        let paths: Vec<String> = items.iter().map(|item| item.path.clone()).collect();
        let cover_plans: std::collections::HashMap<
            String,
            Result<Vec<tag_core::CoverChange>, String>,
        > = items
            .iter()
            .filter_map(|item| {
                let covers = item.covers.as_ref()?;
                Some((item.path.clone(), list_behaviour::cover_changes(covers)))
            })
            .collect();
        history::record("Save tags", None, &paths, || {
            use tag_core::parallel::Progress;
            // Each unit is one file: an ordinary item or the chapter rows of a file.
            let (plain, groups) = group_rows(&paths);
            let units: Vec<(Option<String>, Vec<usize>)> =
                plain
                    .into_iter()
                    .map(|index| (None, vec![index]))
                    .chain(groups.into_iter().map(|(file, rows)| {
                        (Some(file), rows.into_iter().map(|(i, _)| i).collect())
                    }))
                    .collect();
            job.start(units.len());
            let options = settings::tag_options();
            // Provider documents publish through shared recovery state; keep them sequential.
            let threads = if items
                .iter()
                .any(|i| documents::is_managed(Path::new(&i.path)))
            {
                1
            } else {
                std::thread::available_parallelism().map_or(4, |n| n.get().clamp(1, 8))
            };
            let written =
                tag_core::parallel::map_ordered_on(threads, &units, &job, |(file, indices)| {
                    if let Some(file) = file {
                        // Covers belong to the whole file: rows that bring
                        // covers must agree, and the file gets them once.
                        let mut covers = indices.iter().filter_map(|i| {
                            items[*i].covers.as_ref().map(|c| (c, &items[*i].path))
                        });
                        if let Some((first, path)) = covers.next() {
                            let outcome = if covers.any(|(other, _)| other != first) {
                                Err("Covers belong to the whole file; give its chapter rows the same covers".to_string())
                            } else {
                                cover_plans
                                    .get(path)
                                    .cloned()
                                    .unwrap_or_else(|| Ok(Vec::new()))
                                    .and_then(|changes| {
                                        tag_core::apply_cover_changes_with_options(
                                            Path::new(file),
                                            &changes,
                                            &options,
                                        )
                                    })
                            };
                            if let Err(error) = outcome {
                                return indices.iter().map(|_| Err(error.clone())).collect();
                            }
                        }
                        let rows: Vec<(usize, Fields)> = indices
                            .iter()
                            .map(|i| (*i, items[*i].updates.clone()))
                            .collect();
                        return write_row_group(file, &rows, &paths);
                    }
                    let item = &items[indices[0]];
                    vec![mutate_document(Path::new(&item.path), |p| {
                        if let Some(plan) = cover_plans.get(&item.path) {
                            let changes = plan.as_ref().map_err(Clone::clone)?;
                            tag_core::apply_cover_changes_with_options(p, changes, &options)?;
                        }
                        tag_core::write_fields_with_options(
                            p,
                            &item.updates,
                            false,
                            &options.mappings,
                            &options,
                        )
                    })]
                });
            let mut outcomes: Vec<(usize, Result<AudioEntry, String>)> = units
                .iter()
                .zip(written)
                .filter_map(|((_, indices), outcome)| Some(indices.iter().copied().zip(outcome?)))
                .flatten()
                .collect();
            outcomes.sort_by_key(|(index, _)| *index);
            let mut result = ScanResult::default();
            for (index, outcome) in outcomes {
                match outcome {
                    Ok(mut entry) => {
                        documents::project(&mut entry);
                        result.entries.push(entry);
                    }
                    Err(error) => result.errors.push(FileError {
                        path: items[index].path.clone(),
                        error,
                    }),
                }
            }
            Ok(result)
        })
    })
    .await
}

#[tauri::command]
async fn remove_tags(paths: Vec<String>) -> Result<ScanResult, String> {
    io(move || {
        history::record("Remove tags", None, &paths, || {
            Ok(whole_file_batch(&paths, |p| {
                mutate_document(p, |p| {
                    tag_core::remove_tags_with_options(p, false, &settings::tag_options())
                })
            }))
        })
    })
    .await
}

#[tauri::command]
async fn add_cover(paths: Vec<String>, image_path: String) -> Result<ScanResult, String> {
    io(move || {
        let image = auxiliary_input(&image_path)?;
        history::record("Add cover", None, &paths, || {
            Ok(whole_file_batch(&paths, |p| {
                mutate_document(p, |p| {
                    tag_core::add_cover_with_options(p, &image, false, &settings::tag_options())
                })
            }))
        })
    })
    .await
}

#[tauri::command]
async fn remove_cover(paths: Vec<String>, index: usize) -> Result<ScanResult, String> {
    io(move || {
        history::record("Remove cover", None, &paths, || {
            Ok(whole_file_batch(&paths, |p| {
                mutate_document(p, |p| {
                    tag_core::remove_cover_with_options(p, index, false, &settings::tag_options())
                })
            }))
        })
    })
    .await
}

#[derive(Serialize)]
struct Preview {
    path: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_token: Option<String>,
}

#[tauri::command]
async fn preview_format(
    paths: Vec<String>,
    template: String,
    columns: Option<Vec<evaluation::ColumnSpec>>,
    filename: Option<bool>,
) -> Result<Vec<Preview>, String> {
    let filename = filename.unwrap_or(false);
    let settings = evaluation_settings(columns)?;
    io(move || {
        paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| {
                require_audio_handle(Path::new(&path))?;
                let mut entry = settings::read_summary(Path::new(&path))?;
                documents::project(&mut entry);
                let technical = preview_technical_fields(&entry, index + 1);
                let value = if filename {
                    settings.evaluate_filename(
                        &template,
                        &entry.fields,
                        &technical,
                        &entry.path,
                        true,
                    )?
                } else {
                    settings.evaluate(&template, &entry.fields, &technical, &entry.path, true)?
                };
                Ok(Preview {
                    path,
                    value,
                    plan_token: None,
                })
            })
            .collect()
    })
    .await
}

/// One file of the Filename → Tag preview. A file whose name does not match
/// the pattern has no `fields`; Apply leaves it unchanged, like Mp3tag.
#[derive(Serialize)]
struct ParsedPreview {
    path: String,
    fields: Option<Fields>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tauri::command]
async fn preview_filename(
    paths: Vec<String>,
    pattern: String,
) -> Result<Vec<ParsedPreview>, String> {
    io(move || {
        let pattern = tag_core::CompiledFilenamePattern::compile(&pattern)?;
        Ok(paths
            .into_iter()
            .map(|path| {
                match provider_filename(Path::new(&path)).and_then(|name| pattern.try_parse(&name))
                {
                    Ok(fields) => ParsedPreview {
                        path,
                        fields,
                        error: None,
                    },
                    Err(error) => ParsedPreview {
                        path,
                        fields: None,
                        error: Some(error),
                    },
                }
            })
            .collect())
    })
    .await
}

/// Filename → Tag result: the written files, plus the files whose name did
/// not match the pattern and were left unchanged.
#[derive(Serialize)]
struct FilenameImport {
    #[serde(flatten)]
    result: ScanResult,
    unmatched: Vec<String>,
}

#[tauri::command]
async fn filename_to_tags(paths: Vec<String>, pattern: String) -> Result<FilenameImport, String> {
    io(move || {
        let pattern = tag_core::CompiledFilenamePattern::compile(&pattern)?;
        history::record("Filename → Tag", None, &paths, || {
            let mut unmatched = Vec::new();
            let mut result = batch(&paths, |p| {
                let Some(updates) = pattern.try_parse(&provider_filename(p)?)? else {
                    unmatched.push(p.to_string_lossy().into_owned());
                    return Err(String::new());
                };
                mutate_document(p, |p| settings::write_summary(p, &updates))
            });
            // A file that did not match is skipped, not failed.
            let skipped: std::collections::HashSet<&String> = unmatched.iter().collect();
            result.errors.retain(|error| !skipped.contains(&error.path));
            Ok(FilenameImport { result, unmatched })
        })
    })
    .await
}

#[cfg(test)]
#[tauri::command]
async fn apply_action(
    paths: Vec<String>,
    action: Action,
) -> Result<action_host::ActionResult, String> {
    apply_action_diagnostic(paths, action)
        .await
        .map(|report| report.result)
}
async fn apply_action_diagnostic(
    paths: Vec<String>,
    action: Action,
) -> Result<tag_core::batch_errors::Detailed<action_host::ActionResult>, String> {
    let settings = evaluation_settings(None)?;
    let group = tag_core::actions::ActionGroup {
        name: "Quick action".into(),
        enabled: true,
        actions: vec![tag_core::actions::from_quick_action(&action)?],
    };
    io(move || {
        reject_provider_operation(&paths, "Actions")?;
        history::record_with_side_file_epochs_diagnostic(
            "Quick action",
            None,
            &paths,
            |before_replace| {
                Ok(action_host::execute_with_side_file_epochs_diagnostic(
                    &paths,
                    &[group],
                    settings,
                    before_replace,
                ))
            },
        )
    })
    .await
}

#[cfg(test)]
fn rename_target(entry: &AudioEntry, template: &str) -> Result<PathBuf, String> {
    rename_target_with_settings(
        entry,
        template,
        1,
        &evaluation::EvaluationSettings::default(),
    )
}

fn rename_target_with_settings(
    entry: &AudioEntry,
    template: &str,
    index: usize,
    settings: &evaluation::EvaluationSettings,
) -> Result<PathBuf, String> {
    let generated = settings.filename(template, entry, index)?;
    filename_target(entry, generated)
}

fn filename_target(entry: &AudioEntry, generated: String) -> Result<PathBuf, String> {
    if generated.trim().is_empty() {
        return Err("Filename evaluates to an empty string".into());
    }
    // Either separator separates folders. Targets use the platform's own, so
    // they are spelled like the paths the scanner lists.
    let generated = if cfg!(windows) {
        generated.replace('/', "\\")
    } else {
        generated.replace('\\', "/")
    };
    let relative = Path::new(&generated);
    let source = Path::new(&entry.path);
    let mut target = if relative.is_absolute() {
        relative.to_path_buf()
    } else {
        source.parent().unwrap_or(Path::new(".")).join(relative)
    };
    if let Some(ext) = source.extension() {
        // `%title%.MP3` names an .mp3 file once, as Mp3tag does.
        if !target
            .extension()
            .is_some_and(|current| current.eq_ignore_ascii_case(ext))
        {
            let mut name = target.into_os_string();
            name.push(".");
            name.push(ext);
            target = PathBuf::from(name);
        }
    }
    Ok(target)
}

#[tauri::command]
async fn preview_rename(
    paths: Vec<String>,
    template: String,
    columns: Option<Vec<evaluation::ColumnSpec>>,
) -> Result<Vec<Preview>, String> {
    let settings = evaluation_settings(columns)?;
    io(move || {
        reject_provider_operation(&paths, "Rename")?;
        let mut targets = Vec::new();
        for (index, path) in paths.iter().enumerate() {
            let entry = settings::read_summary(Path::new(path))?;
            targets.push(rename_target_with_settings(
                &entry,
                &template,
                index + 1,
                &settings,
            )?);
        }
        let token = rename_plan::record(&paths, &template, targets.clone())?;
        Ok(paths
            .into_iter()
            .zip(targets)
            .map(|(path, target)| Preview {
                path,
                value: target.to_string_lossy().into_owned(),
                plan_token: Some(token.clone()),
            })
            .collect())
    })
    .await
}

#[derive(Serialize)]
struct Renamed {
    from: String,
    to: String,
}
#[derive(Serialize)]
struct RenameResult {
    #[serde(flatten)]
    result: ScanResult,
    renamed: Vec<Renamed>,
}

#[cfg(test)]
#[tauri::command]
async fn rename_files(
    paths: Vec<String>,
    template: String,
    plan_token: Option<String>,
) -> Result<RenameResult, String> {
    rename_files_diagnostic(paths, template, plan_token)
        .await
        .map(|report| report.result)
}
async fn rename_files_diagnostic(
    paths: Vec<String>,
    template: String,
    plan_token: Option<String>,
) -> Result<tag_core::batch_errors::Detailed<RenameResult>, String> {
    let evaluation = evaluation_settings(None)?;
    let targets = paths.clone();
    io(move || {
        reject_provider_operation(&paths, "Rename")?;
        history::record("Tag → Filename", None, &targets, move || {
            rename_planned_diagnostic(paths, &template, plan_token, &evaluation)
        })
    })
    .await
}

mod diagnostics_ipc {
    #[tauri::command(rename = "execute_action_groups")]
    pub async fn execute_action_groups_with_diagnostics(
        paths: Vec<String>,
        groups: Vec<tag_core::actions::ActionGroup>,
        columns: Option<Vec<crate::evaluation::ColumnSpec>>,
    ) -> Result<tag_core::batch_errors::Detailed<crate::action_host::ActionResult>, String> {
        super::execute_action_groups_diagnostic(paths, groups, columns).await
    }
    #[tauri::command(rename = "apply_action")]
    pub async fn apply_action_with_diagnostics(
        paths: Vec<String>,
        action: tag_core::Action,
    ) -> Result<tag_core::batch_errors::Detailed<crate::action_host::ActionResult>, String> {
        super::apply_action_diagnostic(paths, action).await
    }
    #[tauri::command(rename = "rename_files")]
    pub async fn rename_files_with_diagnostics(
        paths: Vec<String>,
        template: String,
        plan_token: Option<String>,
    ) -> Result<tag_core::batch_errors::Detailed<super::RenameResult>, String> {
        super::rename_files_diagnostic(paths, template, plan_token).await
    }
}

/// Destinations compare case-insensitively where volumes usually are.
fn rename_key(path: &Path) -> String {
    let path = path.to_string_lossy();
    if cfg!(any(windows, target_os = "macos")) {
        path.to_lowercase()
    } else {
        path.into_owned()
    }
}

#[cfg(test)]
fn rename_planned(
    paths: Vec<String>,
    template: &str,
    plan_token: Option<String>,
    evaluation: &evaluation::EvaluationSettings,
) -> Result<RenameResult, String> {
    rename_planned_diagnostic(paths, template, plan_token, evaluation).map(|report| report.result)
}
fn rename_planned_diagnostic(
    paths: Vec<String>,
    template: &str,
    plan_token: Option<String>,
    evaluation: &evaluation::EvaluationSettings,
) -> Result<tag_core::batch_errors::Detailed<RenameResult>, String> {
    {
        let mut diagnostics = Vec::new();
        let mut result = ScanResult {
            entries: vec![],
            errors: vec![],
        };
        let mut plans = vec![];
        let mut destinations = std::collections::HashSet::new();
        let candidates = if let Some(token) = plan_token {
            rename_plan::consume(&token, &paths, template)?
                .into_iter()
                .map(|(path, target)| (path, Ok(target)))
                .collect::<Vec<_>>()
        } else {
            paths
                .into_iter()
                .enumerate()
                .map(|(index, path)| {
                    let plan = settings::read_summary(Path::new(&path)).and_then(|entry| {
                        rename_target_with_settings(&entry, template, index + 1, evaluation)
                    });
                    (path, plan)
                })
                .collect()
        };
        for (path, plan) in candidates {
            match plan {
                Ok(target)
                    if destinations.insert(rename_key(&target))
                        && (dunce::canonicalize(&path).ok().as_ref() == Some(&target)
                            || !matches!(
                                tag_core::operations::destination_state(Path::new(&path), &target),
                                tag_core::operations::Destination::Taken
                            )) =>
                {
                    plans.push((path, target))
                }
                Ok(target) => tag_core::batch_errors::push_error(
                    &mut result.errors,
                    &mut diagnostics,
                    path,
                    tag_core::batch_errors::BatchError::DestinationCollision {
                        path: target.to_string_lossy().into_owned(),
                    },
                ),
                Err(error) => result.errors.push(FileError { path, error }),
            }
        }
        let mut renamed = vec![];
        // A file whose destination clashes is skipped and reported; the
        // others are still renamed.
        for (path, target) in plans {
            let operation = if dunce::canonicalize(&path).ok().as_ref() == Some(&target) {
                Ok(())
            } else {
                tag_core::operations::rename_no_clobber(Path::new(&path), &target)
            };
            match operation {
                Ok(()) => {
                    let to = target.to_string_lossy().into_owned();
                    renamed.push(Renamed {
                        from: path,
                        to: to.clone(),
                    });
                    match settings::read_summary(&target) {
                        Ok(entry) => result.entries.push(entry),
                        Err(error) => result.errors.push(FileError { path: to, error }),
                    }
                }
                Err(error) => result.errors.push(FileError { path, error }),
            }
        }
        Ok(tag_core::batch_errors::Detailed {
            result: RenameResult { result, renamed },
            error_diagnostics: diagnostics,
        })
    }
}

fn export_entries(paths: &[String], output: &Path) -> Result<Vec<AudioEntry>, String> {
    reject_provider_operation(paths, "Export")?;
    reject_provider_operation(
        &[output.to_string_lossy().into_owned()],
        "Export destination",
    )?;
    if paths.iter().any(|p| Path::new(p) == output) {
        return Err("Export cannot replace an input audio file".into());
    }
    paths
        .iter()
        .map(|p| settings::read_summary(Path::new(p)))
        .collect()
}

fn write_export(output: &Path, text: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| e.to_string())?;
    file.write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}

fn export_field(fields: &Fields, format: &str, key: &str, separator: &str) -> String {
    let lookup = |name: &str| {
        fields
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, values)| values.join(separator))
            .unwrap_or_default()
    };
    let format = format.to_ascii_lowercase();
    if key == "TRACK"
        && ["flac", "ogg", "vorbis", "opus", "speex"]
            .iter()
            .any(|name| format.contains(name))
    {
        return lookup("TRACKNUMBER");
    }
    let value = lookup(key);
    if value.is_empty() {
        match key {
            "TRACK" => lookup("TRACKNUMBER"),
            "YEAR" => lookup("DATE"),
            _ => value,
        }
    } else {
        value
    }
}

#[tauri::command]
async fn export_csv(paths: Vec<String>, output_path: String) -> Result<(), String> {
    io(move || {
        let entries = export_entries(&paths, Path::new(&output_path))?;
        let names = [
            "TITLE", "ARTIST", "ALBUM", "TRACK", "YEAR", "GENRE", "COMMENT",
        ];
        let mut rows = vec![names
            .iter()
            .map(|s| s.to_string())
            .chain(["PATH".to_string()])
            .collect()];
        for entry in entries {
            rows.push(
                names
                    .iter()
                    .map(|key| export_field(&entry.fields, &entry.format, key, "\\\\"))
                    .chain([entry.path])
                    .collect(),
            );
        }
        write_export(
            Path::new(&output_path),
            &tag_core::operations::render_csv(&rows),
        )
    })
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    e2e::install(file_ops::install(tauri::Builder::default()))
        .setup(|app| {
            let _ = APP_HANDLE.set(app.handle().clone());
            tauri::Manager::manage(app, settings::SettingsStore::open(app.handle()));
            // Cue sheet rows that are not UTF-8 decode with the system codepage.
            tag_core::cue::set_ansi_codepage(
                evaluation::EvaluationSettings::default().ansi_codepage(),
            );
            #[cfg(desktop)]
            if let Some(window) = tauri::Manager::get_webview_window(app, "main") {
                window_state::restore(&window);
            }
            Ok(())
        })
        .on_window_event(|_window, _event| {
            #[cfg(desktop)]
            window_state::track(_window, _event);
            tray::track(_window, _event);
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_document_access::init())
        .invoke_handler(tauri::generate_handler![
            e2e::e2e_report,
            progress::cancel_job,
            library::library_status,
            library::cleanup_library,
            library::clear_library,
            save_fields_batch,
            e2e::e2e_exit,
            e2e::e2e_open_urls,
            load_export_library,
            save_export_library,
            lists::load_helper_lists,
            cover_drag::drag_cover_out,
            lists::save_helper_lists,
            import_export_configuration,
            run_export,
            load_action_library,
            save_action_library,
            import_action_group_with_diagnostics,
            import_action_groups_with_diagnostics,
            diagnostics_ipc::execute_action_groups_with_diagnostics,
            action_host::export_action_group,
            runtime_platform,
            set_working_directory,
            pick_document_files,
            document_states,
            document_working_entries,
            document_save_copy,
            restore_document_recovery,
            get_covers,
            filter_matches,
            preview_numbering,
            scan_files,
            save_fields,
            remove_tags,
            add_cover,
            remove_cover,
            covers::cover_set,
            covers::apply_cover_plan,
            covers::add_cover_image,
            covers::stage_cover_file,
            covers::stage_adjusted_cover,
            covers::cover_start_directory,
            covers::stage_cover_data,
            covers::extract_cover,
            covers::copy_cover_to_clipboard,
            covers::paste_cover_from_clipboard,
            list_behaviour::folder_image,
            list_behaviour::copy_tag_covers,
            preview_format,
            preview_filename,
            preview_rename,
            filename_to_tags,
            diagnostics_ipc::apply_action_with_diagnostics,
            diagnostics_ipc::rename_files_with_diagnostics,
            converters::preview_filename_conversion,
            converters::apply_filename_conversion,
            converters::preview_text_import,
            converters::apply_text_import,
            history::history_state,
            history::ipc::undo_last_with_diagnostics,
            history::ipc::redo_last_with_diagnostics,
            history::ipc::convert_tag_to_tag_with_diagnostics,
            export_csv,
            playlists::ipc::read_playlist_with_diagnostics,
            playlists::plan_playlists,
            playlists::create_playlists,
            file_ops::load_directory_state,
            file_ops::save_directory_state,
            file_ops::remember_directories,
            file_ops::trash_files,
            file_ops::delete_files_permanently,
            file_ops::modifier_keys,
            file_ops::transfer_files,
            file_ops::reveal_files,
            file_ops::open_directory,
            file_ops::play_files,
            file_ops::take_launch_requests,
            columns::load_columns,
            columns::save_columns,
            columns::import_columns,
            mp3tag_columns::import_mp3tag_columns_with_diagnostics,
            columns::export_columns,
            columns::evaluate_columns,
            tag_panel::load_tag_panel,
            tag_panel::save_tag_panel,
            tag_panel::import_tag_panel_layout,
            mp3tag_tag_panel::import_mp3tag_tag_panel,
            tray::set_tray_options,
            tag_panel::export_tag_panel_layout,
            tag_panel::import_value_list,
            tag_panel::export_value_list,
            settings::load_settings,
            settings::save_settings,
            sources::tag_source_search,
            sources::tag_source_release,
            sources::acoustid_file_matches,
            sources::audio_fingerprint_backend,
            sources::tag_source_covers,
            sources::freedb_cover_search,
            sources::tag_source_image,
            sources::tag_source_cancel,
            sources::local_freedb_index_status,
            sources::build_local_freedb_index,
            sources::ipc::tag_source_apply_with_diagnostics,
            sources::tag_source_open_page,
            sources::tag_source_save_cover,
            sources::load_tag_source_settings,
            sources::save_tag_source_settings,
            sources::web::web_sources_list,
            sources::web::web_sources_install,
            sources::web::web_sources_open_folder,
            sources::web::web_source_search,
            sources::web::web_source_release,
            sources::web::web_source_settings,
            sources::web::save_web_source_settings,
            utilities::remove_tag_types,
            utilities::apply_util,
            utilities::strip_mp4_chapters,
            utilities::run_tool,
            utilities::export_configuration,
            utilities::import_configuration,
            utilities::open_configuration_folder,
            languages::language_catalogs,
            languages::open_languages_folder,
            languages::prepare_languages_folder,
            utilities::open_help_page,
            utilities::write_clipboard_text,
            updates::check_for_updates,
            updates::open_release_page
        ])
        .build(tauri::generate_context!())
        .expect("Unable to run TagGroove")
        .run(|_app, _event| {
            // Finder "Open With", Dock drops and `open -a TagGroove <paths>`.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = &_event {
                file_ops::open_urls(_app, urls);
            }
            #[cfg(desktop)]
            if let tauri::RunEvent::Exit = _event {
                window_state::save(_app);
            }
        });
}

#[cfg(test)]
#[path = "documents/copy_command_tests.rs"]
mod document_copy_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_single_and_bulk_companions_keep_per_file_context_and_foreign_causes() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/import-diagnostics.json"))
                .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let first = root.join(if cfg!(unix) {
            "Help: Title\nGroup.mta"
        } else {
            "Help; Title.mta"
        });
        let second = root.join("Title.mta");
        let missing = root.join("Missing; Help.mta");
        std::fs::write(&first, fixture["mta"]["text"].as_str().unwrap()).unwrap();
        std::fs::write(&second, b"[#0]\nT=4\nF=TITLE\n1=(\n2=\n3=0\n").unwrap();
        let one = tauri::async_runtime::block_on(import_action_group_with_diagnostics(
            first.to_string_lossy().into_owned(),
            None,
        ))
        .unwrap();
        assert_eq!(one.issue_diagnostics.len(), 3);
        assert_eq!(
            one.result.group.name,
            first.file_stem().unwrap().to_string_lossy()
        );
        let paths = vec![
            first.to_string_lossy().into_owned(),
            second.to_string_lossy().into_owned(),
            missing.to_string_lossy().into_owned(),
        ];
        let rows = tauri::async_runtime::block_on(import_action_groups_with_diagnostics(
            paths.clone(),
            None,
        ))
        .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows.iter()
                .map(|row| row.result.path.clone())
                .collect::<Vec<_>>(),
            paths
        );
        assert_eq!(rows[0].issue_diagnostics.len(), 3);
        assert!(matches!(
            rows[1].issue_diagnostics[0].cause,
            tag_core::batch_errors::ErrorCause::External { .. }
        ));
        assert!(matches!(
            rows[2].error_cause,
            Some(tag_core::batch_errors::ErrorCause::External { .. })
        ));
        let legacy = tauri::async_runtime::block_on(import_action_groups(paths, None)).unwrap();
        for (current, old) in rows.iter().zip(legacy) {
            assert_eq!(
                serde_json::to_value(&current.result).unwrap(),
                serde_json::to_value(old).unwrap()
            );
        }
        let failure = tauri::async_runtime::block_on(import_action_group_with_diagnostics(
            missing.to_string_lossy().into_owned(),
            None,
        ))
        .unwrap_err();
        let wire = serde_json::to_value(&failure).unwrap();
        assert_eq!(wire["import_failure"]["kind"], "external");
        assert_eq!(wire["message"], failure.to_string());
    }

    #[test]
    fn bulk_group_import_keeps_provider_uris_and_expands_local_folder_paths() {
        use documents::command_fixture::Session;
        let dir = tempfile::tempdir().unwrap();
        let audio = dir.path().join("provider.wav");
        wav(&audio);
        let session = Session::import(&dir.path().join("private"), &audio, "provider.wav", true);
        let source = dir.path().join("Titles.mta");
        std::fs::write(&source, "[#0]\nT=9\nF=TITLE\n1=Imported\n").unwrap();
        let uri = format!("{}/titles", session.uri);
        session.add_input(&uri, &source, "Titles.mta");
        let bad = dir.path().join("Broken.mta");
        std::fs::write(&bad, "not an action group").unwrap();
        let bad_uri = format!("{}/broken", session.uri);
        session.add_input(&bad_uri, &bad, "Broken.mta");
        let folder = dir.path().join("groups");
        std::fs::create_dir(&folder).unwrap();
        let local = folder.join("Local.mta");
        std::fs::copy(&source, &local).unwrap();
        let results = tauri::async_runtime::block_on(import_action_groups(
            vec![
                uri.clone(),
                bad_uri.clone(),
                folder.to_string_lossy().into_owned(),
            ],
            None,
        ))
        .unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].path, uri);
        assert!(results[0].imported.is_some());
        assert_eq!(results[0].imported.as_ref().unwrap().group.name, "Titles");
        assert!(results[0].error.is_none());
        assert_eq!(results[1].path, bad_uri);
        assert!(results[1].imported.is_none());
        assert!(results[1].error.is_some());
        assert_eq!(results[2].path, local.to_string_lossy());
        assert_eq!(results[2].imported.as_ref().unwrap().group.name, "Local");
    }

    #[test]
    fn bulk_group_import_reports_an_unreadable_input_and_imports_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let group = dir.path().join("Titles.mta");
        std::fs::write(&group, "[#0]\nT=9\nF=TITLE\n1=Imported\n").unwrap();
        let missing = "content://opentag.test/missing/group.mta".to_string();
        let results = tauri::async_runtime::block_on(import_action_groups(
            vec![missing.clone(), group.to_string_lossy().into_owned()],
            None,
        ))
        .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, missing);
        assert!(results[0].imported.is_none());
        assert!(results[0].error.is_some());
        assert_eq!(results[1].imported.as_ref().unwrap().group.name, "Titles");
    }

    #[test]
    fn chapter_rows_are_grouped_by_their_file() {
        let row = |file: &str, n| tag_core::subsong_path(file, n);
        let paths = vec![
            "/m/a.mp3".to_string(),
            row("/m/book.m4b", 2),
            row("/m/live.cue", 1),
            row("/m/book.m4b", 1),
            "/m/b.mp3".to_string(),
        ];
        let (plain, groups) = group_rows(&paths);
        assert_eq!(plain, [0, 4]);
        assert_eq!(
            groups,
            [
                ("/m/book.m4b".to_string(), vec![(1, 2), (3, 1)]),
                ("/m/live.cue".to_string(), vec![(2, 1)])
            ]
        );
    }

    #[test]
    fn whole_file_commands_run_once_per_file_and_refuse_cue_rows() {
        let row = |file: &str, n| tag_core::subsong_path(file, n);
        let mut seen = Vec::new();
        let result = whole_file_batch(
            &[
                row("/m/book.m4b", 2),
                "/m/a.mp3".to_string(),
                row("/m/book.m4b", 1),
                row("/m/live.cue", 1),
            ],
            |p| {
                seen.push(p.to_string_lossy().into_owned());
                Err("not read".into())
            },
        );
        assert_eq!(seen, ["/m/book.m4b", "/m/a.mp3"]);
        let errors: Vec<_> = result.errors.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            errors,
            ["/m/book.m4b", "/m/a.mp3", row("/m/live.cue", 1).as_str()]
        );
        assert_eq!(result.errors[2].error, CUE_ROW_OPERATION);
    }

    #[test]
    fn numbering_command_plans_unopened_files_without_io() {
        use tag_core::numbering::{NumberingItem, NumberingOptions};
        let plan = tauri::async_runtime::block_on(preview_numbering(
            vec![
                NumberingItem {
                    path: "/unopened/a/one.flac".into(),
                    fields: Fields::new(),
                },
                NumberingItem {
                    path: "/unopened/b/two.mp3".into(),
                    fields: Fields::new(),
                },
            ],
            NumberingOptions {
                disc_enabled: true,
                next_disc_on_directory: true,
                reset_track_on_next_disc: true,
                ..Default::default()
            },
        ))
        .unwrap();
        assert_eq!(plan[0].fields["TRACK"], ["01"]);
        assert_eq!(plan[1].fields["TRACK"], ["01"]);
        assert_eq!(plan[1].fields["DISCNUMBER"], ["2"]);
    }

    fn wav(path: &Path) {
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
    }

    #[test]
    fn managed_commands_publish_real_source_and_reject_private_only_writes() {
        use documents::command_fixture::Session;
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("provider.wav");
        wav(&source);
        let mut signal = std::fs::read(&source).unwrap();
        for (i, byte) in signal[44..].iter_mut().enumerate() {
            *byte = (i.wrapping_mul(37) % 251) as u8;
        }
        std::fs::write(&source, signal).unwrap();
        let session = Session::import(
            &dir.path().join("private"),
            &source,
            "Artist - 音楽.wav",
            true,
        );
        let paths = vec![session.path.clone()];
        let initial = std::fs::read(&source).unwrap();
        let updates = Fields::from([("TITLE".into(), vec!["Published".into()])]);
        let saved =
            tauri::async_runtime::block_on(save_fields(paths.clone(), updates, None)).unwrap();
        assert!(saved.errors.is_empty());
        assert_eq!(
            tag_core::read_entry(&source).unwrap().fields["TITLE"],
            ["Published"],
            "command must publish source, not only cache"
        );
        assert_eq!(saved.entries[0].filename, "Artist - 音楽.wav");
        assert_eq!(
            saved.entries[0].technical["_PATH"],
            std::slice::from_ref(&session.uri)
        );
        assert!(std::fs::read(&source)
            .unwrap()
            .windows(8820)
            .any(|b| b == &initial[44..]));
        let previews = tauri::async_runtime::block_on(preview_format(
            paths.clone(),
            "%_path%|%_filename_ext%|%_directory%".into(),
            None,
            None,
        ))
        .unwrap();
        assert_eq!(
            previews[0].value,
            format!("{}|Artist - 音楽.wav|", session.uri)
        );
        let parsed = tauri::async_runtime::block_on(preview_filename(
            paths.clone(),
            "%artist% - %title%".into(),
        ))
        .unwrap();
        assert_eq!(parsed[0].fields.as_ref().unwrap()["TITLE"], ["音楽"]);
        let imported = tauri::async_runtime::block_on(filename_to_tags(
            paths.clone(),
            "%artist% - %title%".into(),
        ))
        .unwrap();
        assert!(imported.result.errors.is_empty());
        assert_eq!(
            tag_core::read_entry(&source).unwrap().fields["ARTIST"],
            ["Artist"]
        );
        use base64::Engine;
        let image = dir.path().join("cover.png");
        let png = base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=").unwrap();
        std::fs::write(&image, &png).unwrap();
        let image_uri = format!("{}/cover", session.uri);
        session.add_input(&image_uri, &image, "cover.png");
        let covered = tauri::async_runtime::block_on(add_cover(paths.clone(), image_uri)).unwrap();
        assert!(covered.errors.is_empty(), "{:?}", covered.errors);
        assert_eq!(tag_core::read_entry(&source).unwrap().covers.len(), 1);
        assert_eq!(
            tauri::async_runtime::block_on(get_covers(session.path.clone())).unwrap()[0]
                .data_base64,
            base64::engine::general_purpose::STANDARD.encode(&png)
        );
        let removed = tauri::async_runtime::block_on(remove_cover(paths.clone(), 0)).unwrap();
        assert!(removed.errors.is_empty());
        assert!(tag_core::read_entry(&source).unwrap().covers.is_empty());
        let cleared = tauri::async_runtime::block_on(remove_tags(paths.clone())).unwrap();
        assert!(cleared.errors.is_empty());
        assert!(!tag_core::read_entry(&source)
            .unwrap()
            .fields
            .contains_key("TITLE"));
        assert!(std::fs::read(&source)
            .unwrap()
            .windows(8820)
            .any(|b| b == &initial[44..]));
        let group_file = dir.path().join("group.mta");
        std::fs::write(&group_file, "[#0]\nT=9\nF=TITLE\n1=From URI\n").unwrap();
        let group_uri = format!("{}/group", session.uri);
        session.add_input(&group_uri, &group_file, "group.mta");
        assert!(tauri::async_runtime::block_on(import_action_group(group_uri, None)).is_ok());
        let external = Fields::from([("TITLE".into(), vec!["External".into()])]);
        tag_core::write_fields(&source, &external).unwrap();
        session.rename("Other - Changed.wav");
        let reloaded =
            tauri::async_runtime::block_on(scan_files(paths.clone(), false, None, None)).unwrap();
        assert_eq!(reloaded.entries[0].fields["TITLE"], ["External"]);
        assert_eq!(reloaded.entries[0].filename, "Other - Changed.wav");
        let renamed_preview = tauri::async_runtime::block_on(preview_filename(
            paths.clone(),
            "%artist% - %title%".into(),
        ))
        .unwrap();
        assert_eq!(
            renamed_preview[0].fields.as_ref().unwrap()["TITLE"],
            ["Changed"],
            "parse current provider name rather than stale cache filename"
        );
        #[cfg(unix)]
        {
            let alias = dir.path().join("alias.wav");
            std::os::unix::fs::symlink(&session.path, &alias).unwrap();
            let alias_paths = vec![alias.to_string_lossy().into_owned()];
            let alias_preview = tauri::async_runtime::block_on(preview_format(
                alias_paths.clone(),
                "%_path%|%_directory%".into(),
                None,
                None,
            ))
            .unwrap();
            assert_eq!(alias_preview[0].value, format!("{}|", session.uri));
            let alias_filename = tauri::async_runtime::block_on(preview_filename(
                alias_paths,
                "%artist% - %title%".into(),
            ))
            .unwrap();
            assert_eq!(
                alias_filename[0].fields.as_ref().unwrap()["TITLE"],
                ["Changed"]
            );
            assert!(
                tauri::async_runtime::block_on(apply_action(
                    vec![alias.to_string_lossy().into_owned()],
                    Action::Remove {
                        fields: vec!["TITLE".into()]
                    }
                ))
                .is_err(),
                "provider aliases must fail before action-host mutation"
            );
        }
        let before = std::fs::read(&session.path).unwrap();
        let output = dir
            .path()
            .join("blocked.csv")
            .to_string_lossy()
            .into_owned();
        let group = tag_core::actions::ActionGroup {
            name: "Must not run".into(),
            enabled: true,
            actions: vec![tag_core::actions::from_quick_action(&Action::Remove {
                fields: vec!["TITLE".into()],
            })
            .unwrap()],
        };
        assert!(tauri::async_runtime::block_on(execute_action_groups(
            paths.clone(),
            vec![group],
            None
        ))
        .err()
        .unwrap()
        .contains("provider"));
        assert!(tauri::async_runtime::block_on(apply_action(
            paths.clone(),
            Action::Remove {
                fields: vec!["TITLE".into()]
            }
        ))
        .is_err());
        assert!(
            tauri::async_runtime::block_on(preview_rename(paths.clone(), "new".into(), None))
                .is_err()
        );
        assert!(tauri::async_runtime::block_on(rename_files(
            paths.clone(),
            "new".into(),
            Some("invalid".into())
        ))
        .err()
        .unwrap()
        .contains("provider"));
        assert!(tauri::async_runtime::block_on(export_csv(paths.clone(), output.clone())).is_err());
        let playlist_request = |paths: Vec<String>, output: String| playlists::PlaylistRequest {
            paths,
            filename_template: "list.m3u8".into(),
            partition_template: None,
            base_directory: None,
            output_path: Some(output),
            playlist: None,
            columns: None,
        };
        assert!(
            tauri::async_runtime::block_on(playlists::create_playlists(playlist_request(
                paths.clone(),
                output.clone()
            )))
            .is_err()
        );
        assert!(tauri::async_runtime::block_on(playlists::read_playlist(
            "content://command-fixture/list.m3u8".into()
        ))
        .err()
        .unwrap()
        .contains("provider"));
        assert!(!Path::new(&output).exists());
        assert!(tauri::async_runtime::block_on(export_csv(
            Vec::new(),
            "content://command-fixture/output".into()
        ))
        .err()
        .unwrap()
        .contains("provider"));
        let token = rename_plan::record(&paths, "new", vec![dir.path().join("never.wav")]).unwrap();
        assert!(tauri::async_runtime::block_on(rename_files(
            paths.clone(),
            "new".into(),
            Some(token.clone())
        ))
        .is_err());
        assert!(
            rename_plan::consume(&token, &paths, "new").is_ok(),
            "native guard must precede consumption"
        );
        let request = converters::FilenameRequest {
            paths: paths.clone(),
            old_pattern: "%title%".into(),
            new_pattern: "new".into(),
            columns: None,
        };
        assert!(
            tauri::async_runtime::block_on(converters::preview_filename_conversion(
                request.clone()
            ))
            .is_err()
        );
        assert!(
            tauri::async_runtime::block_on(converters::apply_filename_conversion(
                request,
                "invalid".into()
            ))
            .err()
            .unwrap()
            .to_string()
            .contains("provider")
        );
        let text = converters::TextRequest {
            paths: paths.clone(),
            source: converters::TextSource::Text {
                text: "Overwrite".into(),
            },
            mask: "%title%".into(),
            options: Default::default(),
        };
        assert!(
            tauri::async_runtime::block_on(converters::preview_text_import(text.clone())).is_err()
        );
        assert!(
            tauri::async_runtime::block_on(converters::apply_text_import(text, "invalid".into()))
                .err()
                .unwrap()
                .to_string()
                .contains("provider")
        );
        assert_eq!(std::fs::read(&session.path).unwrap(), before);
        session.fail_publish();
        let failure = tauri::async_runtime::block_on(save_fields(
            paths.clone(),
            Fields::from([("TITLE".into(), vec!["Retain edited".into()])]),
            None,
        ))
        .unwrap();
        assert!(failure.entries.is_empty());
        assert!(failure.errors[0].error.contains("publication failed"));
        assert_eq!(
            tag_core::read_entry(&source).unwrap().fields["TITLE"],
            ["External"]
        );
        assert_eq!(
            tag_core::read_entry(Path::new(&session.path))
                .unwrap()
                .fields["TITLE"],
            ["Retain edited"]
        );
        assert_eq!(
            tauri::async_runtime::block_on(document_states()).unwrap()[&session.path].state,
            "recovery_required"
        );
        let recovered = tauri::async_runtime::block_on(restore_document_recovery()).unwrap();
        assert!(recovered
            .entries
            .iter()
            .any(|e| e.path == session.path && e.fields["TITLE"] == ["Retain edited"]));
        let blocked = tauri::async_runtime::block_on(scan_files(paths, false, None, None)).unwrap();
        assert!(blocked.entries.is_empty());
        assert!(blocked.errors[0].error.contains("recovery"));
        let readonly =
            Session::import(&dir.path().join("readonly"), &source, "Readonly.wav", false);
        let before = std::fs::read(&readonly.path).unwrap();
        let result =
            tauri::async_runtime::block_on(remove_tags(vec![readonly.path.clone()])).unwrap();
        assert!(result.entries.is_empty());
        assert!(result.errors[0].error.contains("Read-only"));
        assert_eq!(std::fs::read(&readonly.path).unwrap(), before);
    }

    #[test]
    fn working_entries_describe_failed_immediate_mutations_without_source_access() {
        use base64::Engine;
        use documents::command_fixture::Session;
        for operation in [
            "remove_tags",
            "filename_to_tags",
            "add_cover",
            "remove_cover",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let source = dir.path().join("provider.wav");
            wav(&source);
            tag_core::write_fields(
                &source,
                &Fields::from([
                    ("TITLE".into(), vec!["Old title".into()]),
                    ("ARTIST".into(), vec!["Original artist".into()]),
                ]),
            )
            .unwrap();
            let image = dir.path().join("cover.png");
            let png = base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j4n8AAAAASUVORK5CYII=").unwrap();
            std::fs::write(&image, &png).unwrap();
            if operation == "remove_cover" || operation == "remove_tags" {
                tag_core::add_cover(&source, &image).unwrap();
            }
            let session = Session::import(
                &dir.path().join("private"),
                &source,
                "Filename artist - Filename title.wav",
                true,
            );
            let paths = vec![session.path.clone()];
            let original = std::fs::read(&source).unwrap();
            session.fail_publish();
            let failed = tauri::async_runtime::block_on(async {
                match operation {
                    "remove_tags" => remove_tags(paths.clone()).await,
                    "filename_to_tags" => {
                        filename_to_tags(paths.clone(), "%artist% - %title%".into())
                            .await
                            .map(|import| import.result)
                    }
                    "add_cover" => {
                        add_cover(paths.clone(), image.to_string_lossy().into_owned()).await
                    }
                    "remove_cover" => remove_cover(paths.clone(), 0).await,
                    _ => unreachable!(),
                }
            })
            .unwrap();
            assert!(failed.entries.is_empty());
            assert!(failed.errors[0].error.contains("publication failed"));
            assert_eq!(std::fs::read(&source).unwrap(), original);
            // Removing provider bytes makes any accidental provider reload fail.
            std::fs::remove_file(&source).unwrap();
            let before =
                tauri::async_runtime::block_on(document_states()).unwrap()[&session.path].clone();
            let local = tauri::async_runtime::block_on(document_working_entries(paths)).unwrap();
            assert!(local.errors.is_empty(), "{:?}", local.errors);
            let entry = &local.entries[0];
            assert_eq!(entry.path, session.path);
            assert_eq!(entry.filename, "Filename artist - Filename title.wav");
            assert_eq!(entry.technical["_PATH"], std::slice::from_ref(&session.uri));
            match operation {
                "remove_tags" => {
                    assert!(entry.fields.is_empty());
                    assert!(entry.covers.is_empty());
                }
                "filename_to_tags" => {
                    assert_eq!(entry.fields["TITLE"], ["Filename title"]);
                    assert_eq!(entry.fields["ARTIST"], ["Filename artist"]);
                }
                "add_cover" => {
                    assert_eq!(entry.covers.len(), 1);
                    assert!(entry.covers[0].data_base64.is_empty());
                    assert_eq!(
                        tauri::async_runtime::block_on(get_covers(session.path.clone())).unwrap()
                            [0]
                        .data_base64,
                        base64::engine::general_purpose::STANDARD.encode(&png)
                    );
                }
                "remove_cover" => assert!(entry.covers.is_empty()),
                _ => unreachable!(),
            }
            let after =
                tauri::async_runtime::block_on(document_states()).unwrap()[&session.path].clone();
            assert_eq!(after.state, "recovery_required");
            assert_eq!(after.state, before.state);
            assert_eq!(after.error, before.error);
        }
    }

    #[test]
    fn working_entries_reject_unmanaged_paths_and_never_fall_back_to_backup() {
        use documents::command_fixture::Session;
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("provider.wav");
        wav(&source);
        let session = Session::import(&dir.path().join("private"), &source, "Provider.wav", true);
        session.fail_publish();
        tauri::async_runtime::block_on(remove_tags(vec![session.path.clone()])).unwrap();
        std::fs::write(&session.path, b"unreadable retained audio").unwrap();
        let local = tauri::async_runtime::block_on(document_working_entries(vec![
            session.path.clone(),
            source.to_string_lossy().into_owned(),
        ]))
        .unwrap();
        assert!(
            local.entries.is_empty(),
            "must not present backup as edited bytes"
        );
        assert_eq!(local.errors.len(), 2);
        assert_eq!(local.errors[0].path, session.path);
        assert!(local.errors[1].error.contains("Unknown provider document"));
        // Recovery's separate baseline salvage remains available.
        let recovered = tauri::async_runtime::block_on(restore_document_recovery()).unwrap();
        assert!(recovered
            .entries
            .iter()
            .any(|entry| entry.path == session.path));
    }

    #[test]
    fn diagnostic_rename_collision_preserves_legacy_english_paths_and_file_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let source = root.join("Help.wav");
        let target = root.join("Title; Help.wav");
        wav(&source);
        wav(&target);
        let mut target_bytes = std::fs::read(&target).unwrap();
        target_bytes[44] = 99;
        std::fs::write(&target, &target_bytes).unwrap();
        let source_bytes = std::fs::read(&source).unwrap();
        let paths = vec![source.to_string_lossy().into_owned()];
        let template = "'Title; Help'".to_string();
        let report = tauri::async_runtime::block_on(
            diagnostics_ipc::rename_files_with_diagnostics(paths.clone(), template.clone(), None),
        )
        .unwrap();
        assert_eq!(report.result.result.errors.len(), 1);
        assert_eq!(report.error_diagnostics.len(), 1);
        assert_eq!(report.error_diagnostics[0].index, 0);
        assert_eq!(
            report.result.result.errors[0].error,
            report.error_diagnostics[0].diagnostic.to_string()
        );
        let wire = serde_json::to_value(&report).unwrap();
        assert_eq!(
            wire["error_diagnostics"][0]["diagnostic"]["kind"],
            "destination_collision"
        );
        assert_eq!(
            wire["error_diagnostics"][0]["diagnostic"]["path"],
            target.to_string_lossy().as_ref()
        );
        let legacy = tauri::async_runtime::block_on(rename_files(paths, template, None)).unwrap();
        assert_eq!(
            legacy.result.errors[0].error,
            report.result.result.errors[0].error
        );
        assert_eq!(std::fs::read(source).unwrap(), source_bytes);
        assert_eq!(std::fs::read(target).unwrap(), target_bytes);
    }

    #[test]
    fn rename_batch_skips_and_reports_only_clashing_destinations() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        let c = dir.path().join("c.wav");
        let taken = dir.path().join("taken.wav");
        for path in [&a, &b, &c, &taken] {
            wav(path);
        }
        // a and b map to the same name, c to an existing file.
        let result = tauri::async_runtime::block_on(rename_files(
            [&a, &b, &c]
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            "$if($strcmp(%_filename%,c),taken,same)".into(),
            None,
        ))
        .unwrap();
        assert_eq!(result.renamed.len(), 1);
        assert_eq!(result.renamed[0].from, a.to_string_lossy());
        assert!(!a.exists() && dir.path().join("same.wav").exists());
        assert!(b.exists() && c.exists() && taken.exists());
        let failed: Vec<_> = result
            .result
            .errors
            .iter()
            .map(|error| error.path.clone())
            .collect();
        assert_eq!(
            failed,
            [
                b.to_string_lossy().into_owned(),
                c.to_string_lossy().into_owned()
            ]
        );
        assert!(result.result.errors[0].error.contains("Destination exists"));
        assert_eq!(result.result.entries.len(), 1);
    }

    #[test]
    fn tag_to_filename_and_inline_edits_change_only_letter_case() {
        let dir = tempfile::tempdir().unwrap();
        let names = || {
            let mut names: Vec<_> = std::fs::read_dir(dir.path())
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        };
        let path = dir.path().join("song.wav");
        wav(&path);
        let mut fields = Fields::new();
        fields.insert("TITLE".into(), vec!["Song".into()]);
        tag_core::write_fields(&path, &fields).unwrap();
        let rename = |path: &Path, template: &str| {
            tauri::async_runtime::block_on(rename_files(
                vec![path.to_string_lossy().into_owned()],
                template.into(),
                None,
            ))
            .unwrap()
        };
        let result = rename(&path, "%title%");
        assert!(
            result.result.errors.is_empty(),
            "{:?}",
            result.result.errors
        );
        assert_eq!(names(), ["Song.wav"]);
        // The file list's inline filename edit sends a quoted literal.
        let result = rename(&dir.path().join("Song.wav"), "'SONG'");
        assert!(
            result.result.errors.is_empty(),
            "{:?}",
            result.result.errors
        );
        assert_eq!(names(), ["SONG.wav"]);
        // A different file that differs only in case is never replaced.
        let other = dir.path().join("Other.wav");
        wav(&other);
        let insensitive = dir.path().join("other.wav").exists();
        let result = rename(&dir.path().join("SONG.wav"), "'other'");
        assert_eq!(!result.result.errors.is_empty(), insensitive);
        assert!(other.exists());
        assert_eq!(names().len(), 2);
    }

    #[test]
    fn renamed_paths_are_spelled_like_the_listed_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        for (name, template) in [
            ("a.wav", "Moved/%_filename%-1"),
            ("b.wav", "Moved\\%_filename%-2"),
        ] {
            let path = root.join(name);
            wav(&path);
            let result = rename_planned(
                vec![path.to_string_lossy().into_owned()],
                template,
                None,
                &evaluation::EvaluationSettings::default(),
            )
            .unwrap();
            assert!(
                result.result.errors.is_empty(),
                "{:?}",
                result.result.errors
            );
            // The interface matches the rename to the listed entry by this string.
            assert_eq!(result.renamed[0].to, result.result.entries[0].path);
            assert!(Path::new(&result.renamed[0].to).starts_with(root.join("Moved")));
        }
    }

    #[test]
    fn rename_preview_and_result_use_identical_sanitized_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.wav");
        wav(&path);
        let mut fields = Fields::new();
        fields.insert("TITLE".into(), vec!["A/B".into()]);
        tag_core::write_fields(&path, &fields).unwrap();
        let paths = vec![path.to_string_lossy().into_owned()];
        let preview = tauri::async_runtime::block_on(preview_rename(
            paths.clone(),
            "Album/%title%".into(),
            None,
        ))
        .unwrap();
        let result = tauri::async_runtime::block_on(rename_files(
            paths,
            "Album/%title%".into(),
            preview[0].plan_token.clone(),
        ))
        .unwrap();
        assert!(result.result.errors.is_empty());
        assert_eq!(preview[0].value, result.renamed[0].to);
        assert!(result.renamed[0].to.ends_with("A_B.wav"));
        assert_eq!(result.result.entries[0].fields["TITLE"], ["A/B"]);
    }

    #[test]
    fn dotted_filename_preview_and_import_preserve_complete_title() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Artist - title.live.wav");
        wav(&path);
        let paths = vec![path.to_string_lossy().into_owned()];
        let previews = tauri::async_runtime::block_on(preview_filename(
            paths.clone(),
            "%artist% - %title%".into(),
        ))
        .unwrap();
        assert_eq!(
            previews[0].fields.as_ref().unwrap()["TITLE"],
            ["title.live"]
        );
        let result =
            tauri::async_runtime::block_on(filename_to_tags(paths, "%artist% - %title%".into()))
                .unwrap();
        assert!(result.result.errors.is_empty());
        assert_eq!(result.result.entries[0].fields["TITLE"], ["title.live"]);
    }

    #[test]
    fn saving_pasted_tags_writes_and_removes_copied_covers() {
        let _serial = list_behaviour::TEST_CLIPBOARD
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let (source, target) = (dir.path().join("source.wav"), dir.path().join("target.wav"));
        wav(&source);
        wav(&target);
        let image = dir.path().join("front.png");
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::new(4, 3))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        std::fs::write(&image, png.into_inner()).unwrap();
        let text = |path: &Path| path.to_string_lossy().into_owned();
        use tauri::async_runtime::block_on as run;
        run(covers::add_cover_image(
            vec![text(&source)],
            serde_json::from_value(serde_json::json!({ "path": text(&image) })).unwrap(),
            None,
        ))
        .unwrap();
        let copied = serde_json::to_value(
            run(list_behaviour::copy_tag_covers(vec![text(&source)], vec![])).unwrap(),
        )
        .unwrap();
        let covers = copied["covers"][0].clone();
        assert_eq!(covers.as_array().unwrap().len(), 1);
        assert_eq!(copied["images"].as_object().unwrap().len(), 1);
        let save = |covers: serde_json::Value, title: &str| {
            let items: Vec<FieldUpdate> = serde_json::from_value(serde_json::json!([
                { "path": text(&target), "updates": { "TITLE": [title] }, "covers": covers }
            ]))
            .unwrap();
            run(save_fields_batch(items, None)).unwrap()
        };
        let pasted = save(covers.clone(), "Pasted");
        assert!(pasted.errors.is_empty(), "{:?}", pasted.errors);
        let written = settings::read_full(&target).unwrap();
        assert_eq!(written.fields["TITLE"], ["Pasted"]);
        assert_eq!(written.covers.len(), 1);
        assert_eq!(
            written.covers[0].digest,
            covers[0]["digest"].as_str().unwrap()
        );
        // Cut tags saves an empty cover list.
        let cut = save(serde_json::json!([]), "Cut");
        assert!(cut.errors.is_empty(), "{:?}", cut.errors);
        assert!(settings::read_full(&target).unwrap().covers.is_empty());
        // Artwork the clipboard no longer holds fails that file and writes nothing.
        run(list_behaviour::copy_tag_covers(vec![], vec![])).unwrap();
        let stale = save(covers, "Stale");
        assert_eq!(stale.errors.len(), 1);
        assert_eq!(
            settings::read_full(&target).unwrap().fields["TITLE"],
            ["Cut"]
        );
    }

    #[test]
    fn remove_action_really_removes_key_from_saved_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        wav(&path);
        let mut fields = Fields::new();
        fields.insert("TITLE".into(), vec!["Keep".into()]);
        fields.insert("COMMENT".into(), vec!["Remove".into()]);
        tag_core::write_fields(&path, &fields).unwrap();
        let result = tauri::async_runtime::block_on(apply_action(
            vec![path.to_string_lossy().into_owned()],
            Action::Remove {
                fields: vec!["COMMENT".into()],
            },
        ))
        .unwrap();
        assert!(result.errors.is_empty());
        assert!(!result.entries[0].fields.contains_key("COMMENT"));
        assert_eq!(result.entries[0].fields["TITLE"], ["Keep"]);
    }

    #[test]
    fn export_refuses_input_and_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        wav(&path);
        let before = std::fs::read(&path).unwrap();
        let result = tauri::async_runtime::block_on(export_csv(
            vec![path.to_string_lossy().into_owned()],
            path.to_string_lossy().into_owned(),
        ));
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let out = dir.path().join("old.csv");
        std::fs::write(&out, b"keep").unwrap();
        assert!(write_export(&out, "new").is_err());
        assert_eq!(std::fs::read(out).unwrap(), b"keep");
    }

    #[test]
    fn exports_preserve_native_tag_casing_aliases_and_multivalues() {
        use std::process::Command;
        if Command::new("ffmpeg").arg("-version").output().is_err() {
            eprintln!("Skipping encoded export regression: ffmpeg is required");
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(dir.path()).unwrap();
        let path = root.join("Café 猫.flac");
        let output = Command::new("ffmpeg")
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=0.1",
                "-metadata",
                "title=Café; 猫 \"live\"",
                "-metadata",
                "album=Native album",
                "-metadata",
                "date=2020-04-18",
                "-metadata",
                "track=03/06",
                "-metadata",
                "genre=Ambient",
                "-metadata",
                "comment=first line\nsecond line",
            ])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        tag_core::write_fields_summary(
            &path,
            &Fields::from([
                ("ARTIST".into(), vec!["Mémoire".into(), "Guest 猫".into()]),
                ("COMMENT".into(), vec!["first line\nsecond line".into()]),
            ]),
        )
        .unwrap();
        let entry = tag_core::read_entry_summary(&path).unwrap();
        assert!(entry.fields.contains_key("title"));
        assert!(entry.fields.contains_key("date"));
        let before = std::fs::read(&path).unwrap();
        let paths = vec![path.to_string_lossy().into_owned()];
        let csv = root.join("tags.csv");
        tauri::async_runtime::block_on(export_csv(
            paths.clone(),
            csv.to_string_lossy().into_owned(),
        ))
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(csv).unwrap(),
            format!("TITLE;ARTIST;ALBUM;TRACK;YEAR;GENRE;COMMENT;PATH\r\n\"Café; 猫 \"\"live\"\"\";Mémoire\\\\Guest 猫;Native album;03/06;2020-04-18;Ambient;\"first line\nsecond line\";{}\r\n", entry.path),
        );
        let playlist = root.join("list.m3u8");
        tauri::async_runtime::block_on(playlists::create_playlists(playlists::PlaylistRequest {
            paths,
            filename_template: String::new(),
            partition_template: None,
            base_directory: None,
            output_path: Some(playlist.to_string_lossy().into_owned()),
            playlist: None,
            columns: None,
        }))
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(playlist).unwrap(),
            "#EXTM3U\r\n#EXTINF:0,Mémoire - Café; 猫 \"live\"\r\nCafé 猫.flac\r\n",
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
    }

    #[test]
    fn export_lookup_keeps_comment_description_and_explicit_aliases_distinct() {
        let fields = Fields::from([
            ("comment".into(), vec!["Unicode 猫".into()]),
            ("DESCRIPTION".into(), vec!["Independent description".into()]),
            ("year".into(), vec!["1998".into()]),
            ("DATE".into(), vec!["2020-04-18".into()]),
            ("track".into(), vec!["2/9".into()]),
            ("TRACKNUMBER".into(), vec!["3/9".into()]),
        ]);
        assert_eq!(
            export_field(&fields, "WAV", "COMMENT", "\\\\"),
            "Unicode 猫"
        );
        assert_eq!(
            export_field(&fields, "WAV", "DESCRIPTION", "\\\\"),
            "Independent description"
        );
        assert_eq!(export_field(&fields, "WAV", "YEAR", "\\\\"), "1998");
        assert_eq!(export_field(&fields, "WAV", "TRACK", "\\\\"), "2/9");
        for format in ["FLAC", "Vorbis", "Opus", "Speex", "Ogg"] {
            assert_eq!(export_field(&fields, format, "TRACK", "\\\\"), "3/9");
        }
    }

    #[test]
    fn rename_functions_receive_original_unsanitized_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("song.wav");
        wav(&path);
        let mut fields = Fields::new();
        fields.insert("TITLE".into(), vec!["A:B".into()]);
        let entry = tag_core::write_fields(&path, &fields).unwrap();
        let target = rename_target(&entry, "$replace(%title%,':','-')").unwrap();
        assert_eq!(target.file_name().unwrap(), "A-B.wav");
    }
}
