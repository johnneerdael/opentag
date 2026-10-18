//! File-menu operations: directory preferences, recycle-bin deletes, copy/move
//! to a folder, shell integration and command-line launch requests.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tag_core::{FileError, ScanResult};

const RECENT_LIMIT: usize = 10;
const PLAY_LIMIT: usize = 100;
pub const LAUNCH_EVENT: &str = "opentag://launch";

/// Recent and favorite folders. Read subfolders and the startup directory are
/// Options settings (`settings.rs` and the `directories-startup` options page).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DirectoryState {
    pub version: u32,
    /// Most recent first.
    pub recent: Vec<String>,
    /// The first favorite is the one used by File → Favorite directory and startup.
    pub favorites: Vec<String>,
}
impl Default for DirectoryState {
    fn default() -> Self {
        Self {
            version: 1,
            recent: vec![],
            favorites: vec![],
        }
    }
}

fn unique(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty() && seen.insert(p.clone()))
        .collect()
}
pub fn normalize(mut state: DirectoryState) -> Result<DirectoryState, String> {
    if state.version != 1 {
        return Err("Unsupported directory settings version".into());
    }
    state.recent = unique(state.recent);
    state.recent.truncate(RECENT_LIMIT);
    state.favorites = unique(state.favorites);
    Ok(state)
}
pub fn remember(mut state: DirectoryState, directories: &[String]) -> DirectoryState {
    for directory in directories {
        state.recent.retain(|p| p != directory);
        state.recent.insert(0, directory.clone());
    }
    state.recent.truncate(RECENT_LIMIT);
    state
}
pub fn load(path: &Path) -> Result<DirectoryState, String> {
    match std::fs::read(path) {
        Ok(bytes) => normalize(
            serde_json::from_slice(&bytes)
                .map_err(|e| format!("Cannot read directory settings: {e}"))?,
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DirectoryState::default()),
        Err(e) => Err(e.to_string()),
    }
}
pub fn save(path: &Path, state: DirectoryState) -> Result<DirectoryState, String> {
    let state = normalize(state)?;
    let bytes = serde_json::to_vec_pretty(&state).map_err(|e| e.to_string())?;
    crate::action_host::atomic_output(path, &bytes, false)?;
    Ok(state)
}
fn state_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::action_host::library_path(app)?.with_file_name("directories.json"))
}

#[tauri::command]
pub async fn load_directory_state(app: tauri::AppHandle) -> Result<DirectoryState, String> {
    let path = state_path(&app)?;
    super::io(move || load(&path)).await
}
#[tauri::command]
pub async fn save_directory_state(
    app: tauri::AppHandle,
    state: DirectoryState,
) -> Result<DirectoryState, String> {
    let path = state_path(&app)?;
    super::io(move || save(&path, state)).await
}
/// Record opened folders in the recent list. Missing paths and files are ignored.
#[tauri::command]
pub async fn remember_directories(
    app: tauri::AppHandle,
    paths: Vec<String>,
) -> Result<DirectoryState, String> {
    let path = state_path(&app)?;
    super::io(move || {
        let directories: Vec<_> = paths
            .into_iter()
            .filter(|p| !p.starts_with("content://") && Path::new(p).is_dir())
            .collect();
        save(&path, remember(load(&path)?, &directories))
    })
    .await
}

fn regular_file(path: &Path, verb: &str) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(format!("Only regular files can be {verb}")),
        Err(e) => Err(e.to_string()),
    }
}
fn unique_paths(paths: &[String]) -> Vec<&String> {
    let mut seen = HashSet::new();
    paths.iter().filter(|p| seen.insert(*p)).collect()
}

#[derive(Debug, Default, Serialize)]
pub struct TrashResult {
    pub removed: Vec<String>,
    pub errors: Vec<FileError>,
}
/// Delete each regular file with `remove`, reporting failures per file.
pub fn trash_paths(
    paths: &[String],
    mut remove: impl FnMut(&Path) -> Result<(), String>,
) -> TrashResult {
    let mut result = TrashResult::default();
    for path in unique_paths(paths) {
        match regular_file(Path::new(path), "deleted").and_then(|()| remove(Path::new(path))) {
            Ok(()) => result.removed.push(path.clone()),
            Err(error) => result.errors.push(FileError {
                path: path.clone(),
                error,
            }),
        }
    }
    result
}
#[cfg(desktop)]
fn move_to_trash(path: &Path) -> Result<(), String> {
    #[allow(unused_mut)]
    let mut context = trash::TrashContext::default();
    // NSFileManager avoids asking for permission to control Finder.
    #[cfg(target_os = "macos")]
    trash::macos::TrashContextExtMacos::set_delete_method(
        &mut context,
        trash::macos::DeleteMethod::NsFileManager,
    );
    context
        .delete(path)
        .map_err(|e| format!("Cannot move to the trash: {e}"))
}
#[cfg(mobile)]
fn move_to_trash(_: &Path) -> Result<(), String> {
    Err("Deleting files is not available here".into())
}
#[tauri::command]
pub async fn trash_files(paths: Vec<String>) -> Result<TrashResult, String> {
    super::io(move || {
        super::reject_provider_operation(&paths, "Delete")?;
        let result = trash_paths(&paths, move_to_trash);
        super::library::changed(&paths, &[], &[]);
        Ok(result)
    })
    .await
}
/// Remove regular files from the disk without the trash; they cannot be restored.
pub fn delete_paths(paths: &[String]) -> TrashResult {
    trash_paths(paths, |path| {
        std::fs::remove_file(path).map_err(|e| format!("Cannot delete the file: {e}"))
    })
}
/// Edit → Delete permanently (Ctrl+Shift+Del).
#[tauri::command]
pub async fn delete_files_permanently(paths: Vec<String>) -> Result<TrashResult, String> {
    super::io(move || {
        super::reject_provider_operation(&paths, "Delete")?;
        let result = delete_paths(&paths);
        super::library::changed(&paths, &[], &[]);
        Ok(result)
    })
    .await
}

/// Modifier keys held at this moment. Native file drops carry no key state, so
/// the File List asks when files are dropped: Ctrl (⌘ on macOS) adds them to the
/// list, a plain drop replaces it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ModifierKeys {
    pub control: bool,
    pub command: bool,
}
#[cfg(target_os = "macos")]
fn held_modifiers() -> ModifierKeys {
    use objc2_app_kit::{NSEvent, NSEventModifierFlags};
    let flags = NSEvent::modifierFlags_class();
    ModifierKeys {
        control: flags.contains(NSEventModifierFlags::Control),
        command: flags.contains(NSEventModifierFlags::Command),
    }
}
#[cfg(target_os = "windows")]
fn held_modifiers() -> ModifierKeys {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_RWIN,
    };
    // The high bit is set while the key is down.
    let down = |key: u16| unsafe { GetAsyncKeyState(i32::from(key)) } < 0;
    ModifierKeys {
        control: down(VK_CONTROL),
        command: down(VK_LWIN) || down(VK_RWIN),
    }
}
#[cfg(target_os = "linux")]
fn held_modifiers() -> ModifierKeys {
    // GDK_CONTROL_MASK and GDK_META_MASK.
    let state = gdk::Display::default()
        .and_then(|display| gdk::Keymap::for_display(&display))
        .map(|keymap| keymap.modifier_state())
        .unwrap_or(0);
    ModifierKeys {
        control: state & (1 << 2) != 0,
        command: state & (1 << 28) != 0,
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn held_modifiers() -> ModifierKeys {
    ModifierKeys::default()
}
#[tauri::command]
pub async fn modifier_keys(app: tauri::AppHandle) -> Result<ModifierKeys, String> {
    // GDK may only be used on the main thread.
    let (sender, receiver) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(held_modifiers());
    })
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || receiver.recv().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferMode {
    Copy,
    Move,
}
#[derive(Debug, Serialize)]
pub struct PathChange {
    pub from: String,
    pub to: String,
}
#[derive(Debug, Serialize)]
pub struct TransferResult {
    #[serde(flatten)]
    pub result: ScanResult,
    /// Moved files, in the same shape as rename results so the UI can re-key them.
    pub renamed: Vec<PathChange>,
    pub copied: Vec<PathChange>,
    pub associated_renamed: Vec<PathChange>,
}

/// Map every source to `destination/<filename>` and validate the whole plan.
/// Any conflict rejects the plan before a file is touched.
pub fn plan_transfer(
    paths: &[String],
    destination: &Path,
) -> Result<Vec<(String, PathBuf)>, Vec<FileError>> {
    let fail = |error: String| {
        vec![FileError {
            path: destination.to_string_lossy().into_owned(),
            error,
        }]
    };
    let destination = match dunce::canonicalize(destination) {
        Ok(path) if path.is_dir() => path,
        Ok(_) => return Err(fail("The destination is not a folder".into())),
        Err(e) => return Err(fail(format!("Cannot open the destination folder: {e}"))),
    };
    let mut plan = vec![];
    let mut errors = vec![];
    // Compare names case-insensitively: case-insensitive volumes would otherwise collide.
    let mut names = HashSet::new();
    for path in unique_paths(paths) {
        let source = Path::new(path);
        let target = (|| {
            regular_file(source, "copied or moved")?;
            let name = source.file_name().ok_or("The file has no name")?;
            let parent = source
                .parent()
                .and_then(|p| dunce::canonicalize(p).ok())
                .ok_or("Cannot resolve the source folder")?;
            // The same folder may be spelled in another case on case-insensitive volumes.
            if parent == destination || tag_core::operations::is_case_alias(&parent, &destination) {
                return Err("The file is already in the destination folder".to_string());
            }
            let target = destination.join(name);
            if std::fs::symlink_metadata(&target).is_ok() {
                return Err(format!("{} already exists", target.display()));
            }
            if !names.insert(name.to_string_lossy().to_lowercase()) {
                return Err(format!(
                    "Another selected file is also named {}",
                    name.to_string_lossy()
                ));
            }
            Ok(target)
        })();
        match target {
            Ok(target) => plan.push((path.clone(), target)),
            Err(error) => errors.push(FileError {
                path: path.clone(),
                error,
            }),
        }
    }
    if errors.is_empty() {
        Ok(plan)
    } else {
        Err(errors)
    }
}
pub fn transfer(paths: &[String], destination: &Path, mode: TransferMode) -> TransferResult {
    let mut output = TransferResult {
        result: ScanResult {
            entries: vec![],
            errors: vec![],
        },
        renamed: vec![],
        copied: vec![],
        associated_renamed: vec![],
    };
    let plan = match plan_transfer(paths, destination) {
        Ok(plan) => plan,
        Err(errors) => {
            output.result.errors = errors;
            return output;
        }
    };
    for (path, target) in plan {
        let source = Path::new(&path);
        let operation = match mode {
            TransferMode::Copy => tag_core::operations::copy_no_clobber(source, &target),
            TransferMode::Move => tag_core::operations::rename_no_clobber(source, &target),
        };
        let to = target.to_string_lossy().into_owned();
        match operation {
            Err(error) => output.result.errors.push(FileError { path, error }),
            Ok(()) if mode == TransferMode::Copy => {
                output.copied.push(PathChange { from: path, to })
            }
            Ok(()) => {
                output.renamed.push(PathChange {
                    from: path,
                    to: to.clone(),
                });
                match crate::settings::read_summary(&target) {
                    Ok(entry) => output.result.entries.push(entry),
                    Err(error) => output.result.errors.push(FileError { path: to, error }),
                }
            }
        }
    }
    output
}
#[derive(Serialize)]
pub struct TransferPreview {
    pub plan_token: String,
    pub associated: Vec<crate::associated_files::Preview>,
}
fn transfer_key(destination: &str) -> String {
    format!("move-associated\0{destination}")
}
fn preview_transfer_plan(paths: &[String], destination: &str) -> Result<TransferPreview, String> {
    let targets = plan_transfer(paths, Path::new(destination)).map_err(|errors| {
        errors
            .into_iter()
            .map(|e| format!("{}: {}", e.path, e.error))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let plan = crate::associated_files::Plan::new(targets.clone(), true)?;
    let associated = plan.preview();
    let plan_token = crate::rename_plan::record_associated(
        paths,
        &transfer_key(destination),
        targets.into_iter().map(|(_, target)| target).collect(),
        Some(plan),
    )?;
    Ok(TransferPreview {
        plan_token,
        associated,
    })
}
/// Freeze companion membership and bytes before the existing Move confirmation.
#[tauri::command]
pub async fn preview_transfer(
    paths: Vec<String>,
    destination: String,
) -> Result<TransferPreview, String> {
    super::io(move || {
        super::reject_provider_operation(&paths, "Move")?;
        super::reject_provider_operation(std::slice::from_ref(&destination), "Destination")?;
        if !crate::associated_files::enabled() {
            return Err("Enable associated files in Options → Files first".into());
        }
        preview_transfer_plan(&paths, &destination)
    })
    .await
}
fn transfer_reviewed(
    paths: &[String],
    destination: &str,
    mode: TransferMode,
    plan_token: Option<String>,
    enabled: bool,
) -> Result<TransferResult, String> {
    if mode == TransferMode::Copy {
        return Ok(transfer(paths, Path::new(destination), mode));
    }
    if !enabled && plan_token.is_none() {
        return Ok(transfer(paths, Path::new(destination), mode));
    }
    let token = plan_token.ok_or("Preview the associated files before moving")?;
    let (_, plan) =
        crate::rename_plan::consume_associated(&token, paths, &transfer_key(destination))?;
    if !enabled {
        return Err("The associated-file policy changed; preview again".into());
    }
    let plan = plan.ok_or("The move preview has no associated-file plan")?;
    let mut output = TransferResult {
        result: ScanResult {
            entries: vec![],
            errors: vec![],
        },
        renamed: vec![],
        copied: vec![],
        associated_renamed: vec![],
    };
    for group in plan.groups {
        let path = group.audio.source.clone();
        match group.execute(&mut tag_core::operations::rename_no_clobber) {
            Err(error) => output.result.errors.push(FileError { path, error }),
            Ok(()) => {
                output
                    .associated_renamed
                    .extend(group.associated.iter().map(|item| PathChange {
                        from: item.source.clone(),
                        to: item.target.to_string_lossy().into_owned(),
                    }));
                let to = group.audio.target.to_string_lossy().into_owned();
                output.renamed.push(PathChange {
                    from: path,
                    to: to.clone(),
                });
                match crate::settings::read_summary(&group.audio.target) {
                    Ok(entry) => output.result.entries.push(entry),
                    Err(error) => output.result.errors.push(FileError { path: to, error }),
                }
            }
        }
    }
    Ok(output)
}
#[tauri::command]
pub async fn transfer_files(
    paths: Vec<String>,
    destination: String,
    mode: TransferMode,
    plan_token: Option<String>,
) -> Result<TransferResult, String> {
    super::io(move || {
        super::reject_provider_operation(&paths, "Copy and move")?;
        super::reject_provider_operation(std::slice::from_ref(&destination), "Destination")?;
        if mode == TransferMode::Move {
            let mut targets = paths.clone();
            if let Some(token) = &plan_token {
                targets.extend(crate::rename_plan::associated_sources(
                    token,
                    &paths,
                    &transfer_key(&destination),
                )?);
            }
            super::history::record("Move files", None, &targets, || {
                transfer_reviewed(
                    &paths,
                    &destination,
                    mode,
                    plan_token,
                    crate::associated_files::enabled(),
                )
            })
        } else {
            Ok(transfer(&paths, Path::new(&destination), mode))
        }
    })
    .await
}

fn existing(paths: &[String], verb: &str) -> Result<Vec<PathBuf>, String> {
    super::reject_provider_operation(paths, verb)?;
    if paths.is_empty() {
        return Err("Select files first".into());
    }
    // Chapter and cue sheet rows stand for the audio file they play from.
    let files = paths
        .iter()
        .map(|p| tag_core::subsong_file(Path::new(p)).map(|f| f.to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    unique_paths(&files)
        .into_iter()
        .map(|p| {
            regular_file(Path::new(p), "opened")
                .map(|()| PathBuf::from(p))
                .map_err(|e| format!("{p}: {e}"))
        })
        .collect()
}
#[cfg(desktop)]
fn shell<T>(result: Result<T, tauri_plugin_opener::Error>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}
/// Show files selected in Finder, Explorer or the desktop file manager.
#[tauri::command]
pub async fn reveal_files(paths: Vec<String>) -> Result<(), String> {
    super::io(move || {
        let paths = existing(&paths, "Showing files")?;
        #[cfg(desktop)]
        return shell(tauri_plugin_opener::reveal_items_in_dir(&paths));
        #[cfg(mobile)]
        return Err(format!("Cannot show {} files here", paths.len()));
    })
    .await
}
#[tauri::command]
pub async fn open_directory(path: String) -> Result<(), String> {
    super::io(move || {
        super::reject_provider_operation(std::slice::from_ref(&path), "Opening folders")?;
        if !Path::new(&path).is_dir() {
            return Err(format!("{path} is not a folder"));
        }
        #[cfg(desktop)]
        return shell(tauri_plugin_opener::open_path(&path, None::<&str>));
        #[cfg(mobile)]
        return Err("Opening folders is not available here".into());
    })
    .await
}
/// Open each file with the system's default application for its type.
#[tauri::command]
pub async fn play_files(paths: Vec<String>) -> Result<(), String> {
    super::io(move || {
        let paths = existing(&paths, "Playing files")?;
        if paths.len() > PLAY_LIMIT {
            return Err(format!("Select at most {PLAY_LIMIT} files to play"));
        }
        #[cfg(desktop)]
        for path in paths {
            shell(tauri_plugin_opener::open_path(&path, None::<&str>))
                .map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(())
    })
    .await
}

/// Files and folders passed on the command line or by a second launch.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct LaunchRequest {
    pub directories: Vec<String>,
    pub files: Vec<String>,
    /// `/add`: append to the file list instead of replacing it.
    pub add: bool,
}

/// Parse Mp3tag-style switches: `/fp:<folder>`, `/fn:<file>` and `/add`.
/// Switches also accept a `-` prefix and are case-insensitive; a switch with an
/// empty value takes the next argument. Bare arguments are files or folders.
pub fn parse_launch(
    args: impl IntoIterator<Item = String>,
    cwd: &Path,
    is_dir: impl Fn(&Path) -> bool,
) -> Option<LaunchRequest> {
    let mut request = LaunchRequest::default();
    let mut args = args.into_iter();
    let resolve = |value: &str| {
        let value = value.trim().trim_matches('"');
        (!value.is_empty()).then(|| cwd.join(value).to_string_lossy().into_owned())
    };
    while let Some(arg) = args.next() {
        let switch = arg
            .strip_prefix('/')
            .or_else(|| arg.strip_prefix('-'))
            .map(|s| s.to_ascii_lowercase());
        match switch.as_deref() {
            Some("add") => request.add = true,
            Some(s) if s.starts_with("fp:") || s.starts_with("fn:") => {
                let mut value = arg[4..].to_string();
                if value.trim().is_empty() {
                    value = args.next().unwrap_or_default();
                }
                if let Some(path) = resolve(&value) {
                    if s.starts_with("fp:") {
                        request.directories.push(path);
                    } else {
                        request.files.push(path);
                    }
                }
            }
            // Unix absolute paths start with '/', so only known switches are consumed there.
            Some(s) if cfg!(windows) || !arg.starts_with('/') || s.starts_with("psn_") => {}
            _ => {
                if let Some(path) = resolve(&arg) {
                    if is_dir(Path::new(&path)) {
                        request.directories.push(path);
                    } else {
                        request.files.push(path);
                    }
                }
            }
        }
    }
    (!request.directories.is_empty() || !request.files.is_empty()).then_some(request)
}

/// Paths from a macOS open-documents event: Finder "Open With", files dropped on
/// the Dock icon and `open -a TagGroove <paths>`. Like a drop or Mp3tag's Explorer
/// context menu they replace the file list. URLs other than `file:` are ignored.
pub fn opened_request(
    urls: &[tauri::Url],
    is_dir: impl Fn(&Path) -> bool,
) -> Option<LaunchRequest> {
    let mut request = LaunchRequest::default();
    for url in urls.iter().filter(|url| url.scheme() == "file") {
        let Ok(path) = url.to_file_path() else {
            continue;
        };
        // Finder sends folders with a trailing slash; the list and the recent
        // folders use the plain path.
        let path: PathBuf = path.components().collect();
        let value = path.to_string_lossy().into_owned();
        if url.path().ends_with('/') || is_dir(&path) {
            request.directories.push(value);
        } else {
            request.files.push(value);
        }
    }
    (!request.directories.is_empty() || !request.files.is_empty()).then_some(request)
}

#[derive(Default)]
pub struct PendingLaunches(Mutex<Vec<LaunchRequest>>);
impl PendingLaunches {
    fn push(&self, request: LaunchRequest) {
        if let Ok(mut queue) = self.0.lock() {
            queue.push(request);
        }
    }
}
#[tauri::command]
pub fn take_launch_requests(state: tauri::State<'_, PendingLaunches>) -> Vec<LaunchRequest> {
    state
        .0
        .lock()
        .map(|mut queue| std::mem::take(&mut *queue))
        .unwrap_or_default()
}

/// Bring the main window forward for a second launch or an opened document.
fn focus_main(app: &tauri::AppHandle) {
    #[cfg(desktop)]
    if let Some(window) = tauri::Manager::get_webview_window(app, "main") {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    #[cfg(not(desktop))]
    let _ = app;
}

/// Queue a request for the UI, which opens queued requests in order.
fn forward(app: &tauri::AppHandle, request: LaunchRequest) {
    use tauri::{Emitter, Manager};
    app.state::<PendingLaunches>().push(request);
    let _ = app.emit(LAUNCH_EVENT, ());
}

/// Handle `RunEvent::Opened` (macOS). Events that arrive before the page has
/// loaded stay queued until the UI takes them at startup.
pub fn open_urls(app: &tauri::AppHandle, urls: &[tauri::Url]) {
    if let Some(request) = opened_request(urls, Path::is_dir) {
        focus_main(app);
        forward(app, request);
    }
}

/// Install launch handling. A second desktop launch forwards its arguments to
/// this instance, which queues them and notifies the UI.
pub fn install(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    let pending = PendingLaunches::default();
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(request) = parse_launch(std::env::args().skip(1), &cwd, Path::is_dir) {
            pending.push(request);
        }
    }
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
        focus_main(app);
        let args = argv.into_iter().skip(1);
        if let Some(request) = parse_launch(args, Path::new(&cwd), Path::is_dir) {
            forward(app, request);
        }
    }));
    builder.manage(pending)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn directory_state_dedupes_caps_and_orders_recent_folders() {
        let mut state = DirectoryState::default();
        for i in 0..12 {
            state = remember(state, &[format!("/music/{i}")]);
        }
        state = remember(state, &strings(&["/music/5"]));
        assert_eq!(state.recent.len(), RECENT_LIMIT);
        assert_eq!(state.recent[0], "/music/5");
        assert_eq!(state.recent.iter().filter(|p| *p == "/music/5").count(), 1);
        assert!(!state.recent.contains(&"/music/0".to_string()));
        let normalized = normalize(DirectoryState {
            favorites: strings(&[" /a ", "/a", "", "/b"]),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(normalized.favorites, strings(&["/a", "/b"]));
        assert!(normalize(DirectoryState {
            version: 2,
            ..Default::default()
        })
        .is_err());
    }

    #[test]
    fn directory_state_round_trips_and_defaults_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/directories.json");
        assert_eq!(load(&path).unwrap(), DirectoryState::default());
        let state = DirectoryState {
            favorites: strings(&["/music"]),
            ..Default::default()
        };
        save(&path, state.clone()).unwrap();
        assert_eq!(load(&path).unwrap(), state);
        // Older files with since-moved option fields still load.
        std::fs::write(
            &path,
            br#"{"version":1,"recent":["/x"],"subfolders":false,"startup":"favorite"}"#,
        )
        .unwrap();
        let partial = load(&path).unwrap();
        assert_eq!(partial.recent, strings(&["/x"]));
        assert!(partial.favorites.is_empty());
        std::fs::write(&path, b"not json").unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn trash_reports_each_failure_and_never_touches_folders() {
        let dir = tempfile::tempdir().unwrap();
        let ok = dir.path().join("ok.mp3");
        let fails = dir.path().join("locked.mp3");
        std::fs::write(&ok, b"a").unwrap();
        std::fs::write(&fails, b"b").unwrap();
        let paths: Vec<String> = [
            ok.clone(),
            ok.clone(),
            fails.clone(),
            dir.path().join("missing.mp3"),
            dir.path().to_path_buf(),
        ]
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
        let mut calls = vec![];
        let result = trash_paths(&paths, |path| {
            calls.push(path.to_path_buf());
            if path == fails {
                Err("Permission denied".into())
            } else {
                std::fs::remove_file(path).map_err(|e| e.to_string())
            }
        });
        assert_eq!(calls, vec![ok.clone(), fails.clone()]);
        assert_eq!(result.removed, vec![ok.to_string_lossy().into_owned()]);
        assert_eq!(result.errors.len(), 3);
        assert!(dir.path().is_dir() && fails.exists());
    }

    #[test]
    fn permanent_delete_removes_regular_files_only() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.mp3");
        std::fs::write(&file, b"x").unwrap();
        let paths: Vec<String> = [
            file.clone(),
            dir.path().join("missing.mp3"),
            dir.path().to_path_buf(),
        ]
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
        let result = delete_paths(&paths);
        assert_eq!(result.removed, vec![paths[0].clone()]);
        assert_eq!(result.errors.len(), 2);
        assert!(!file.exists() && dir.path().is_dir());
    }

    #[cfg(desktop)]
    #[test]
    #[ignore = "moves a probe file into the real system trash"]
    fn system_trash_removes_the_probe_file() {
        let dir = tempfile::tempdir().unwrap();
        let probe = dir.path().join("opentag-trash-probe.txt");
        std::fs::write(&probe, b"TagGroove trash probe; safe to delete").unwrap();
        let path = probe.to_string_lossy().into_owned();
        let result = trash_paths(std::slice::from_ref(&path), move_to_trash);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        assert_eq!(result.removed, vec![path]);
        assert!(!probe.exists());
    }

    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, Vec<String>) {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let target = dir.path().join("target");
        std::fs::create_dir_all(source.join("disc2")).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        let files = ["one.mp3", "two.mp3"]
            .map(|name| {
                let path = source.join(name);
                std::fs::write(&path, name.as_bytes()).unwrap();
                path.to_string_lossy().into_owned()
            })
            .to_vec();
        (dir, source, target, files)
    }

    #[test]
    fn copy_keeps_originals_and_reports_copies() {
        let (_dir, _source, target, files) = fixture();
        let result = transfer(&files, &target, TransferMode::Copy);
        assert!(
            result.result.errors.is_empty(),
            "{:?}",
            result.result.errors
        );
        assert_eq!(result.copied.len(), 2);
        assert!(result.renamed.is_empty() && result.result.entries.is_empty());
        for file in &files {
            assert!(Path::new(file).exists());
        }
        assert_eq!(std::fs::read(target.join("two.mp3")).unwrap(), b"two.mp3");
    }

    #[test]
    fn move_relocates_files_and_reports_renames_even_if_unreadable() {
        let (_dir, _source, target, files) = fixture();
        let result = transfer(&files, &target, TransferMode::Move);
        assert_eq!(result.renamed.len(), 2);
        assert_eq!(result.renamed[0].from, files[0]);
        assert!(result.renamed[0].to.ends_with("one.mp3"));
        // The fixtures are not audio: moves succeed and the read failure is per file.
        assert_eq!(result.result.errors.len(), 2);
        assert!(files.iter().all(|f| !Path::new(f).exists()));
        assert_eq!(std::fs::read(target.join("one.mp3")).unwrap(), b"one.mp3");
    }

    #[test]
    fn wrapped_destination_errors_reject_copy_and_move_without_changes() {
        let (_dir, _source, target, files) = fixture();
        let not_folder = target.join("Help");
        std::fs::write(&not_folder, b"keep destination").unwrap();
        let missing = target.join("My  Files").join("absent");
        for mode in [TransferMode::Copy, TransferMode::Move] {
            for destination in [&not_folder, &missing] {
                let result = transfer(&files, destination, mode);
                assert!(result.copied.is_empty() && result.renamed.is_empty());
                assert!(result.result.entries.is_empty());
                assert_eq!(result.result.errors.len(), 1);
                let frame = &result.result.errors[0];
                assert_eq!(frame.path, destination.to_string_lossy());
                if destination == &not_folder {
                    assert_eq!(frame.error, "The destination is not a folder");
                } else {
                    assert!(frame
                        .error
                        .starts_with("Cannot open the destination folder: "));
                }
                for path in &files {
                    assert_eq!(
                        std::fs::read(path).unwrap(),
                        Path::new(path)
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .as_bytes()
                    );
                }
                assert_eq!(std::fs::read(&not_folder).unwrap(), b"keep destination");
                assert!(!missing.exists());
            }
        }
    }

    #[test]
    fn conflicts_reject_the_whole_plan_before_touching_files() {
        let (_dir, source, target, mut files) = fixture();
        std::fs::write(target.join("two.mp3"), b"existing").unwrap();
        let result = transfer(&files, &target, TransferMode::Move);
        assert_eq!(result.result.errors.len(), 1);
        assert!(result.renamed.is_empty());
        assert!(files.iter().all(|f| Path::new(f).exists()));
        assert_eq!(std::fs::read(target.join("two.mp3")).unwrap(), b"existing");

        std::fs::remove_file(target.join("two.mp3")).unwrap();
        let duplicate = source.join("disc2/ONE.mp3");
        std::fs::write(&duplicate, b"other").unwrap();
        files.push(duplicate.to_string_lossy().into_owned());
        let result = transfer(&files, &target, TransferMode::Copy);
        assert_eq!(result.result.errors.len(), 1);
        assert!(result.result.errors[0]
            .error
            .contains("Another selected file"));
        assert!(std::fs::read_dir(&target).unwrap().next().is_none());

        let same = transfer(&files[..1], &source, TransferMode::Move);
        assert!(same.result.errors[0]
            .error
            .contains("already in the destination"));
        let missing = transfer(&files, &target.join("absent"), TransferMode::Copy);
        assert!(missing.result.errors[0]
            .error
            .contains("destination folder"));
    }

    #[test]
    fn opened_urls_become_a_replacing_request() {
        let base = std::env::temp_dir().join("Open With");
        let file = base.join("a b#1.mp3");
        let folder = base.join("Album");
        let url = |path: &Path, dir: bool| {
            if dir {
                tauri::Url::from_directory_path(path).unwrap()
            } else {
                tauri::Url::from_file_path(path).unwrap()
            }
        };
        let text = |path: &Path| path.to_string_lossy().into_owned();
        assert!(url(&file, false).as_str().contains("a%20b%231.mp3"));
        let urls = [
            url(&file, false),
            url(&folder, true),
            tauri::Url::parse("https://example.org/a.mp3").unwrap(),
            url(&base.join("Singles"), false),
        ];
        let request = opened_request(&urls, |p| p.ends_with("Singles")).unwrap();
        assert_eq!(
            request,
            LaunchRequest {
                directories: vec![text(&folder), text(&base.join("Singles"))],
                files: vec![text(&file)],
                add: false,
            }
        );
        assert!(!request.directories[0].ends_with(['/', '\\']));
        assert_eq!(opened_request(&[], |_| false), None);
        let web = [tauri::Url::parse("opentag://launch").unwrap()];
        assert_eq!(opened_request(&web, |_| true), None);
    }

    #[test]
    fn launch_arguments_follow_mp3tag_switches() {
        let cwd = Path::new("work");
        let at = |p: &str| cwd.join(p).to_string_lossy().into_owned();
        let dirs = |p: &Path| p.ends_with("Albums");
        let parse = |args: &[&str]| parse_launch(strings(args), cwd, dirs);
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&["/add"]), None);
        assert_eq!(
            parse(&["/fp:\"Rock Music\""]),
            Some(LaunchRequest {
                directories: vec![at("Rock Music")],
                ..Default::default()
            })
        );
        assert_eq!(
            parse(&["/FN:song.flac", "/add"]),
            Some(LaunchRequest {
                files: vec![at("song.flac")],
                add: true,
                ..Default::default()
            })
        );
        assert_eq!(
            parse(&[
                "-fp:",
                "Albums",
                "-psn_0_1234",
                "--unknown",
                "a.mp3",
                "Albums"
            ]),
            Some(LaunchRequest {
                directories: vec![at("Albums"), at("Albums")],
                files: vec![at("a.mp3")],
                add: false,
            })
        );
        let absolute = std::env::temp_dir().join("a.mp3");
        let absolute = absolute.to_string_lossy().into_owned();
        assert_eq!(parse(&[&absolute]).unwrap().files, vec![absolute.clone()]);
    }
    #[test]
    fn reviewed_move_freezes_membership_source_bytes_and_single_use_token() {
        let (_dir, source, target, files) = fixture();
        let destination = target.to_string_lossy().into_owned();
        let lyric = source.join("one.lrc");
        std::fs::write(&lyric, b"first").unwrap();
        let review = preview_transfer_plan(&files, &destination).unwrap();
        assert_eq!(review.associated.len(), 1);
        std::fs::write(&lyric, b"other").unwrap();
        assert!(transfer_reviewed(
            &files,
            &destination,
            TransferMode::Move,
            Some(review.plan_token),
            true
        )
        .is_err());
        assert!(files.iter().all(|p| Path::new(p).exists()));
        let review = preview_transfer_plan(&files, &destination).unwrap();
        std::fs::write(source.join("one.cdg"), b"appeared").unwrap();
        assert!(transfer_reviewed(
            &files,
            &destination,
            TransferMode::Move,
            Some(review.plan_token),
            true
        )
        .is_err());
        let review = preview_transfer_plan(&files, &destination).unwrap();
        let token = review.plan_token;
        let moved = transfer_reviewed(
            &files,
            &destination,
            TransferMode::Move,
            Some(token.clone()),
            true,
        )
        .unwrap();
        assert_eq!(moved.renamed.len(), 2);
        assert_eq!(moved.associated_renamed.len(), 2);
        assert!(!lyric.exists());
        assert_eq!(std::fs::read(target.join("one.lrc")).unwrap(), b"other");
        assert_eq!(std::fs::read(target.join("one.cdg")).unwrap(), b"appeared");
        assert!(
            transfer_reviewed(&files, &destination, TransferMode::Move, Some(token), true).is_err()
        );
    }
    #[test]
    fn reviewed_move_rejects_audio_edits_late_collisions_and_policy_changes() {
        let (_dir, source, target, files) = fixture();
        let destination = target.to_string_lossy().into_owned();
        std::fs::write(source.join("one.lrc"), b"lyrics").unwrap();
        let review = preview_transfer_plan(&files, &destination).unwrap();
        std::fs::write(&files[0], b"changed audio").unwrap();
        assert!(transfer_reviewed(
            &files,
            &destination,
            TransferMode::Move,
            Some(review.plan_token),
            true
        )
        .is_err());
        let review = preview_transfer_plan(&files, &destination).unwrap();
        std::fs::write(target.join("one.lrc"), b"keep").unwrap();
        assert!(transfer_reviewed(
            &files,
            &destination,
            TransferMode::Move,
            Some(review.plan_token),
            true
        )
        .is_err());
        assert_eq!(std::fs::read(target.join("one.lrc")).unwrap(), b"keep");
        std::fs::remove_file(target.join("one.lrc")).unwrap();
        let review = preview_transfer_plan(&files, &destination).unwrap();
        assert!(transfer_reviewed(
            &files,
            &destination,
            TransferMode::Move,
            Some(review.plan_token),
            false
        )
        .is_err());
        assert!(files.iter().all(|p| Path::new(p).exists()));
        assert!(transfer_reviewed(&files, &destination, TransferMode::Move, None, true).is_err());
    }
}
