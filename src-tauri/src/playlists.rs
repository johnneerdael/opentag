//! Playlists (https://docs.mp3tag.de/playlists/): File → Load Playlist/Cuesheet,
//! File → Playlist (all/selected files), File → Generate Playlists and the
//! writing options of Options → Playlist (`pages.playlist` in settings).

use crate::{evaluation, settings};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tag_core::playlist::{PlaylistItem, PlaylistWriteOptions};
use tag_core::{AudioEntry, Fields, FileError};

/// Options → Playlist values that change how playlists are written.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaylistOptions {
    pub extended: bool,
    pub extinf_format: String,
    pub relative: bool,
    pub bom: bool,
    pub create_directories: bool,
}
impl Default for PlaylistOptions {
    fn default() -> Self {
        Self {
            extended: true,
            extinf_format: "%artist% - %title%".into(),
            relative: true,
            bom: false,
            create_directories: true,
        }
    }
}
impl PlaylistOptions {
    /// Read `pages.playlist`, keeping defaults for unsaved or malformed values.
    pub fn from_page(page: Option<&serde_json::Value>) -> Self {
        let mut options = Self::default();
        let flag = |key: &str| page.and_then(|p| p.get(key)).and_then(|v| v.as_bool());
        if let Some(value) = flag("extended") {
            options.extended = value;
        }
        if let Some(value) = page
            .and_then(|p| p.get("extinf_format"))
            .and_then(|v| v.as_str())
        {
            options.extinf_format = value.into();
        }
        if let Some(value) = flag("relative") {
            options.relative = value;
        }
        if let Some(value) = flag("write_bom") {
            options.bom = value;
        }
        if let Some(value) = flag("dont_create_directories") {
            options.create_directories = !value;
        }
        options
    }
    pub fn current() -> Self {
        Self::from_page(settings::current().pages.get("playlist"))
    }
    pub fn write_options(&self) -> PlaylistWriteOptions {
        PlaylistWriteOptions {
            extended: self.extended,
            relative: self.relative,
            bom: self.bom,
            ansi_codepage: ansi_codepage(),
            root: None,
        }
    }
}

fn ansi_codepage() -> Option<String> {
    evaluation::EvaluationSettings::default().ansi_codepage()
}

/// Write one playlist after checking its folder against "Don't create directories".
pub fn write(target: &Path, bytes: &[u8], options: &PlaylistOptions) -> Result<(), String> {
    if !options.create_directories && !target.parent().unwrap_or(Path::new(".")).is_dir() {
        return Err(format!(
            "Playlist directory does not exist: {}",
            target.parent().unwrap_or(Path::new(".")).display()
        ));
    }
    crate::action_host::atomic_output(target, bytes, false)
}

#[derive(Serialize)]
pub struct PlaylistContents {
    pub files: Vec<String>,
    /// Entries that could not be resolved; `path` is the playlist.
    pub errors: Vec<FileError>,
}

/// Resolve the files referenced by a playlist or cue sheet in playlist order.
#[cfg(test)]
#[tauri::command]
pub async fn read_playlist(path: String) -> Result<PlaylistContents, String> {
    read_playlist_diagnostic(path)
        .await
        .map(|report| report.result)
}
pub async fn read_playlist_diagnostic(
    path: String,
) -> Result<tag_core::batch_errors::Detailed<PlaylistContents>, String> {
    crate::io(move || {
        crate::reject_provider_operation(std::slice::from_ref(&path), "Playlists")?;
        let report = tag_core::playlist::load_playlist_diagnostic(
            Path::new(&path),
            ansi_codepage().as_deref(),
        )?;
        let loaded = report.result;
        Ok(tag_core::batch_errors::Detailed {
            result: PlaylistContents {
                files: loaded
                    .files
                    .into_iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
                errors: loaded
                    .missing
                    .into_iter()
                    .map(|(line, error)| FileError {
                        path: path.clone(),
                        error: format!("Line {line}: {error}"),
                    })
                    .collect(),
            },
            error_diagnostics: report.error_diagnostics,
        })
    })
    .await
}

/// The command name and legacy fields stay unchanged; only this wire adapter adds companions.
pub mod ipc {
    #[tauri::command(rename = "read_playlist")]
    pub async fn read_playlist_with_diagnostics(
        path: String,
    ) -> Result<tag_core::batch_errors::Detailed<super::PlaylistContents>, String> {
        super::read_playlist_diagnostic(path).await
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlaylistRequest {
    pub paths: Vec<String>,
    pub filename_template: String,
    /// Partition format of the Playlist Generator; one playlist when absent.
    #[serde(default)]
    pub partition_template: Option<String>,
    /// Folder for relative playlist names; the first file's folder when absent.
    #[serde(default)]
    pub base_directory: Option<String>,
    /// A confirmed literal file name; placeholders are not evaluated.
    #[serde(default)]
    pub output_path: Option<String>,
    /// The loaded playlist for %_playlist_filename% and related placeholders.
    #[serde(default)]
    pub playlist: Option<String>,
    #[serde(default)]
    pub columns: Option<Vec<evaluation::ColumnSpec>>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct PlannedPlaylist {
    pub path: String,
    pub files: usize,
}
#[derive(Debug, Serialize)]
pub struct PlaylistPlan {
    pub playlists: Vec<PlannedPlaylist>,
    pub errors: Vec<FileError>,
}

/// %_playlist_filename%, %_playlist_filename_ext% and %_playlist_folderpath%.
pub fn playlist_fields(playlist: Option<&str>) -> Fields {
    let mut fields = Fields::new();
    if let Some(playlist) = playlist.filter(|p| !p.is_empty()) {
        let path = Path::new(playlist);
        let text = |value: Option<&std::ffi::OsStr>| {
            vec![value.unwrap_or_default().to_string_lossy().into_owned()]
        };
        fields.insert("_PLAYLIST_FILENAME".into(), text(path.file_stem()));
        fields.insert("_PLAYLIST_FILENAME_EXT".into(), text(path.file_name()));
        let mut folder = path
            .parent()
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .into_owned();
        if !folder.is_empty() && !folder.ends_with(std::path::MAIN_SEPARATOR) {
            folder.push(std::path::MAIN_SEPARATOR);
        }
        fields.insert("_PLAYLIST_FOLDERPATH".into(), vec![folder]);
    }
    fields
}

struct Member {
    entry: AudioEntry,
    technical: Fields,
}
struct Group {
    target: PathBuf,
    members: Vec<Member>,
}

fn plan(
    request: &PlaylistRequest,
    settings: &evaluation::EvaluationSettings,
) -> Result<(Vec<Group>, Vec<FileError>), String> {
    crate::reject_provider_operation(&request.paths, "Playlists")?;
    if request.paths.is_empty() {
        return Err("Select files for the playlist".into());
    }
    let context = playlist_fields(request.playlist.as_deref());
    let mut errors = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    let mut groups: Vec<Vec<Member>> = Vec::new();
    let mut seen = HashSet::new();
    for path in &request.paths {
        if !seen.insert(path) {
            continue;
        }
        let entry = match settings::read_summary(Path::new(path)) {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(FileError {
                    path: path.clone(),
                    error,
                });
                continue;
            }
        };
        let mut technical = evaluation::technical_fields(&entry, seen.len());
        technical.extend(context.clone());
        let key = match &request.partition_template {
            Some(template) => {
                settings.evaluate(template, &entry.fields, &technical, &entry.path, true)?
            }
            None => String::new(),
        };
        let member = Member { entry, technical };
        match keys.iter().position(|k| *k == key) {
            Some(index) => groups[index].push(member),
            None => {
                keys.push(key);
                groups.push(vec![member]);
            }
        }
    }
    if request.output_path.is_some() && groups.len() > 1 {
        return Err("A confirmed playlist name applies to one playlist only".into());
    }
    let inputs: HashSet<String> = request
        .paths
        .iter()
        .map(|p| crate::action_host::normalized_output(Path::new(p)))
        .collect::<Result<_, _>>()?;
    let mut targets = HashSet::new();
    let mut planned = Vec::with_capacity(groups.len());
    for members in groups {
        let first = &members[0];
        let name = match &request.output_path {
            Some(path) => path.trim().to_string(),
            None => settings
                .evaluate(
                    &request.filename_template,
                    &first.entry.fields,
                    &first.technical,
                    &first.entry.path,
                    true,
                )?
                .trim()
                .to_string(),
        };
        if name.is_empty() {
            return Err("The playlist file name is empty".into());
        }
        let base = request
            .base_directory
            .as_deref()
            .filter(|b| !b.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(&first.entry.path)
                    .parent()
                    .unwrap_or(Path::new("."))
                    .to_path_buf()
            });
        // Resolve symlinked folders so relative entries match the scanned file paths.
        let target = crate::action_host::normalized_output_path(&base.join(name))?;
        let normalized = crate::action_host::normalized_output(&target)?;
        if inputs.contains(&normalized) {
            return Err("A playlist cannot replace an input audio file".into());
        }
        if !targets.insert(normalized) {
            return Err(format!(
                "Multiple playlist partitions use {}",
                target.display()
            ));
        }
        planned.push(Group { target, members });
    }
    Ok((planned, errors))
}

fn summary(groups: &[Group], errors: Vec<FileError>) -> PlaylistPlan {
    PlaylistPlan {
        playlists: groups
            .iter()
            .map(|g| PlannedPlaylist {
                path: g.target.to_string_lossy().into_owned(),
                files: g.members.len(),
            })
            .collect(),
        errors,
    }
}

/// The playlists that would be written, without writing them.
#[tauri::command]
pub async fn plan_playlists(request: PlaylistRequest) -> Result<PlaylistPlan, String> {
    let settings = crate::evaluation_settings(request.columns.clone())?;
    crate::io(move || {
        let (groups, errors) = plan(&request, &settings)?;
        Ok(summary(&groups, errors))
    })
    .await
}

pub fn create(
    request: &PlaylistRequest,
    settings: &evaluation::EvaluationSettings,
    options: &PlaylistOptions,
) -> Result<PlaylistPlan, String> {
    let (groups, errors) = plan(request, settings)?;
    if groups.is_empty() {
        return Err("No readable files for the playlist".into());
    }
    let mut write_options = options.write_options();
    write_options.root = working_root(request.base_directory.as_deref(), settings);
    let mut outputs = Vec::with_capacity(groups.len());
    // Render every playlist before writing any of them.
    for group in &groups {
        let items = group
            .members
            .iter()
            .map(|member| {
                let title = if options.extended {
                    settings.evaluate(
                        &options.extinf_format,
                        &member.entry.fields,
                        &member.technical,
                        &member.entry.path,
                        true,
                    )?
                } else {
                    String::new()
                };
                Ok(PlaylistItem {
                    path: PathBuf::from(&member.entry.path),
                    duration: member.entry.duration,
                    title,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        outputs.push(tag_core::playlist::render_playlist(
            &group.target,
            &items,
            &write_options,
        )?);
    }
    for (group, bytes) in groups.iter().zip(outputs) {
        write(&group.target, &bytes, options)?;
    }
    Ok(summary(&groups, errors))
}

/// The working directory that bounds relative entries: the folder the request
/// names (the File List's working directory), else the evaluation settings' one.
pub fn working_root(
    base: Option<&str>,
    settings: &evaluation::EvaluationSettings,
) -> Option<PathBuf> {
    base.or(settings.working_directory.as_deref())
        .filter(|path| !path.trim().is_empty())
        .map(|path| {
            crate::action_host::normalized_output_path(Path::new(path))
                .unwrap_or_else(|_| PathBuf::from(path))
        })
}

/// Write the planned playlists with the Options → Playlist settings.
#[tauri::command]
pub async fn create_playlists(request: PlaylistRequest) -> Result<PlaylistPlan, String> {
    let settings = crate::evaluation_settings(request.columns.clone())?;
    crate::io(move || create(&request, &settings, &PlaylistOptions::current())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(names: &[&str]) -> (tempfile::TempDir, Vec<String>) {
        let directory = tempfile::tempdir().unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let audio = vec![0u8; 8820];
        let mut bytes = b"RIFF".to_vec();
        bytes.extend((36 + audio.len() as u32).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(44100u32.to_le_bytes());
        bytes.extend(88200u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend((audio.len() as u32).to_le_bytes());
        bytes.extend(audio);
        let paths = names
            .iter()
            .map(|name| {
                let path = root.join(name);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, &bytes).unwrap();
                path.to_string_lossy().into_owned()
            })
            .collect();
        (directory, paths)
    }

    fn request(paths: &[String], filename: &str) -> PlaylistRequest {
        PlaylistRequest {
            paths: paths.to_vec(),
            filename_template: filename.into(),
            partition_template: None,
            base_directory: None,
            output_path: None,
            playlist: None,
            columns: None,
        }
    }

    #[test]
    fn options_page_values_override_defaults() {
        let page = serde_json::json!({"extended":false,"extinf_format":"%title%","relative":false,"write_bom":true,"dont_create_directories":true,"filename":"x"});
        let options = PlaylistOptions::from_page(Some(&page));
        assert_eq!(
            options,
            PlaylistOptions {
                extended: false,
                extinf_format: "%title%".into(),
                relative: false,
                bom: true,
                create_directories: false,
            }
        );
        let malformed = serde_json::json!({"extended":"yes"});
        assert_eq!(
            PlaylistOptions::from_page(Some(&malformed)),
            PlaylistOptions::default()
        );
    }

    #[test]
    fn playlist_placeholders_describe_the_loaded_playlist() {
        let path = Path::new("music").join("Best of.m3u8");
        let fields = playlist_fields(Some(&path.to_string_lossy()));
        assert_eq!(fields["_PLAYLIST_FILENAME"], ["Best of"]);
        assert_eq!(fields["_PLAYLIST_FILENAME_EXT"], ["Best of.m3u8"]);
        assert_eq!(
            fields["_PLAYLIST_FOLDERPATH"],
            [format!("music{}", std::path::MAIN_SEPARATOR)]
        );
        assert!(playlist_fields(None).is_empty());
    }

    #[test]
    fn creates_partitioned_playlists_in_list_order_and_reads_them_back() {
        let (_directory, paths) = fixture(&["b/2.wav", "a/1.wav", "b/1.wav"]);
        let mut generate = request(&paths, "%_directory%.m3u8");
        generate.partition_template = Some("%_folderpath%".into());
        let settings = evaluation::EvaluationSettings::default();
        let options = PlaylistOptions {
            extinf_format: "%_filename%".into(),
            ..Default::default()
        };
        let plan = create(&generate, &settings, &options).unwrap();
        let root = Path::new(&paths[0]).parent().unwrap().parent().unwrap();
        assert_eq!(
            plan.playlists,
            [
                PlannedPlaylist {
                    path: root.join("b").join("b.m3u8").to_string_lossy().into_owned(),
                    files: 2
                },
                PlannedPlaylist {
                    path: root.join("a").join("a.m3u8").to_string_lossy().into_owned(),
                    files: 1
                },
            ]
        );
        let text = std::fs::read_to_string(root.join("b").join("b.m3u8")).unwrap();
        assert_eq!(
            text,
            "#EXTM3U\r\n#EXTINF:0,2\r\n2.wav\r\n#EXTINF:0,1\r\n1.wav\r\n"
        );
        let loaded =
            tag_core::playlist::load_playlist(&root.join("b").join("b.m3u8"), None).unwrap();
        assert_eq!(
            loaded.files,
            [PathBuf::from(&paths[0]), PathBuf::from(&paths[2])]
        );
    }

    #[test]
    fn confirmed_names_are_literal_and_outputs_are_protected() {
        let (directory, paths) = fixture(&["1.wav", "2.wav"]);
        let settings = evaluation::EvaluationSettings::default();
        let options = PlaylistOptions::default();
        let mut literal = request(&paths, "ignored");
        literal.output_path = Some("%album%.pls".into());
        literal.playlist = Some("/old/list.m3u".into());
        create(&literal, &settings, &options).unwrap();
        let root = dunce::canonicalize(directory.path()).unwrap();
        let pls = std::fs::read_to_string(root.join("%album%.pls")).unwrap();
        assert!(pls.starts_with("[playlist]\r\nFile1=1.wav\r\nTitle1= - \r\nLength1=0\r\n"));

        let mut named = request(&paths, "%_playlist_filename%-copy.m3u8");
        named.playlist = Some(root.join("old.m3u").to_string_lossy().into_owned());
        named.base_directory = Some(root.join("lists").to_string_lossy().into_owned());
        let plan = create(&named, &settings, &options).unwrap();
        assert_eq!(
            plan.playlists[0].path,
            root.join("lists").join("old-copy.m3u8").to_string_lossy()
        );

        let before = std::fs::read(&paths[0]).unwrap();
        let mut overwrite = request(&paths, "%_filename_ext%");
        overwrite.output_path = Some(paths[0].clone());
        assert!(create(&overwrite, &settings, &options)
            .unwrap_err()
            .contains("input audio"));
        assert!(create(&request(&paths, "%_filename_ext%"), &settings, &options).is_err());
        let mut collide = request(&paths, "same.m3u8");
        collide.partition_template = Some("%_filename%".into());
        assert!(create(&collide, &settings, &options)
            .unwrap_err()
            .contains("Multiple"));
        assert!(!root.join("same.m3u8").exists());
        assert_eq!(std::fs::read(&paths[0]).unwrap(), before);

        let strict = PlaylistOptions {
            create_directories: false,
            ..Default::default()
        };
        assert!(
            create(&request(&paths, "missing/x.m3u8"), &settings, &strict)
                .unwrap_err()
                .contains("does not exist")
        );
        assert!(!root.join("missing").exists());
    }

    #[cfg(unix)]
    #[test]
    fn relative_entries_survive_a_symlinked_working_directory() {
        let (directory, paths) = fixture(&["album/1.wav", "album/2.wav"]);
        let root = dunce::canonicalize(directory.path()).unwrap();
        let link = tempfile::tempdir().unwrap();
        let alias = link.path().join("alias");
        std::os::unix::fs::symlink(root.join("album"), &alias).unwrap();
        let mut list = request(&paths, "list.m3u8");
        list.base_directory = Some(alias.to_string_lossy().into_owned());
        let settings = evaluation::EvaluationSettings::default();
        let plan = create(&list, &settings, &PlaylistOptions::default()).unwrap();
        assert_eq!(
            plan.playlists[0].path,
            root.join("album").join("list.m3u8").to_string_lossy()
        );
        let loaded = tag_core::playlist::load_playlist(&alias.join("list.m3u8"), None).unwrap();
        assert_eq!(loaded.missing, []);
        assert_eq!(loaded.files.len(), 2);
    }

    #[test]
    fn unreadable_files_are_reported_and_skipped() {
        let (directory, mut paths) = fixture(&["1.wav"]);
        let missing = directory.path().join("gone.wav");
        paths.insert(0, missing.to_string_lossy().into_owned());
        let settings = evaluation::EvaluationSettings::default();
        let plan = create(
            &request(&paths, "list.m3u8"),
            &settings,
            &PlaylistOptions {
                extended: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(plan.errors.len(), 1);
        assert_eq!(plan.playlists[0].files, 1);
        let root = dunce::canonicalize(directory.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("list.m3u8")).unwrap(),
            "1.wav\r\n"
        );
    }
    #[test]
    fn xml_playlist_ipc_loads_real_audio_in_document_order_before_the_existing_scan() {
        let (directory, files) = fixture(&["second.wav", "first.wav"]);
        let root = dunce::canonicalize(directory.path()).unwrap();
        for (extension, text) in [
            (
                "xspf",
                r#"<playlist xmlns="http://xspf.org/ns/0/" version="1"><trackList><track><location>second.wav</location></track><track><location>first.wav</location></track></trackList></playlist>"#,
            ),
            (
                "wpl",
                r#"<smil><body><seq><media src="second.wav"/><media src="first.wav"/></seq></body></smil>"#,
            ),
            (
                "asx",
                r#"<ASX VERSION="3.0"><ENTRY><REF HREF="second.wav"/></ENTRY><ENTRY><REF HREF="first.wav"/></ENTRY></ASX>"#,
            ),
        ] {
            let playlist = root.join(format!("list.{extension}"));
            std::fs::write(&playlist, text).unwrap();
            let loaded = tauri::async_runtime::block_on(ipc::read_playlist_with_diagnostics(
                playlist.to_string_lossy().into_owned(),
            ))
            .unwrap();
            assert_eq!(loaded.result.files, files);
            assert!(loaded.result.errors.is_empty());
            let scanned = tauri::async_runtime::block_on(crate::scan_files(
                loaded.result.files,
                false,
                None,
                Some(true),
            ))
            .unwrap();
            assert!(scanned.errors.is_empty(), "{:?}", scanned.errors);
            assert_eq!(
                scanned
                    .entries
                    .iter()
                    .map(|entry| entry.path.clone())
                    .collect::<Vec<_>>(),
                files
            );
        }
    }
    #[test]
    fn xml_output_suffix_is_refused_by_the_existing_create_flow_without_a_file() {
        let (directory, files) = fixture(&["one.wav"]);
        let target = directory.path().join("output.xspf");
        let request = request(&files, &target.to_string_lossy());
        let result = create(
            &request,
            &evaluation::EvaluationSettings::default(),
            &PlaylistOptions::default(),
        );
        assert!(matches!(result, Err(error) if error.contains("read-only")));
        assert!(!target.exists());
        std::fs::write(&target, b"keep existing playlist").unwrap();
        assert!(create(
            &request,
            &evaluation::EvaluationSettings::default(),
            &PlaylistOptions::default()
        )
        .is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"keep existing playlist");
    }
}
