//! Playlist and cue sheet reading and playlist writing
//! (https://docs.mp3tag.de/playlists/).
//!
//! Reading resolves the entries of `.m3u`, `.m3u8`, `.pls` and `.cue` files to
//! local paths in playlist order. Static `.xspf`, `.wpl` and XML `.asx` readers
//! share the resolution pipeline, with local-only URI rules and alternatives.
//! Writing renders `.m3u`/`.m3u8` (optionally
//! with `#EXTINF` extended info) and `.pls` playlists.

use crate::batch_errors::{BatchError, Detailed, ErrorCause, IndexedDiagnostic};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
mod xml;

/// File extensions that are loaded as playlists or cue sheets.
pub const PLAYLIST_EXTENSIONS: [&str; 7] = ["m3u", "m3u8", "pls", "cue", "xspf", "wpl", "asx"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaylistKind {
    /// `.m3u`: the system ANSI codepage; Windows-1252 where there is none.
    M3u,
    /// `.m3u8`: UTF-8.
    M3u8,
    /// `.pls`: INI-style `FileN=` entries, UTF-8.
    Pls,
    /// `.cue`: the tracks of a cue sheet, see [`crate::cue`].
    Cue,
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// The legacy text playlist kind. XML readers use `load_playlist` directly;
/// retain the flat parsing/writing API and cue-sheet comparisons unchanged.
pub fn playlist_kind(path: &Path) -> Option<PlaylistKind> {
    match extension(path).as_str() {
        "m3u" => Some(PlaylistKind::M3u),
        "m3u8" => Some(PlaylistKind::M3u8),
        "pls" => Some(PlaylistKind::Pls),
        "cue" => Some(PlaylistKind::Cue),
        _ => None,
    }
}

/// A raw entry as written in the playlist, with its 1-based line number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawEntry {
    pub line: usize,
    pub location: String,
}

/// Decode playlist text. Unicode BOMs are authoritative and `.m3u8` is UTF-8;
/// other files are UTF-8 when valid and otherwise use the ANSI codepage
/// (Windows-1252 when none is known).
pub fn decode_playlist(bytes: &[u8], kind: PlaylistKind, ansi_codepage: Option<&str>) -> String {
    if let Some((encoding, bom)) = encoding_rs::Encoding::for_bom(bytes) {
        return encoding
            .decode_without_bom_handling(&bytes[bom..])
            .0
            .into_owned();
    }
    if kind == PlaylistKind::M3u8 {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }
    ansi_encoding(ansi_codepage)
        .unwrap_or(encoding_rs::WINDOWS_1252)
        .decode_without_bom_handling(bytes)
        .0
        .into_owned()
}

pub(crate) fn ansi_encoding(codepage: Option<&str>) -> Option<&'static encoding_rs::Encoding> {
    let codepage = codepage?;
    let numeric = codepage
        .strip_prefix("windows-")
        .or_else(|| codepage.strip_prefix("cp"))
        .unwrap_or(codepage)
        .parse::<u16>()
        .ok();
    match numeric {
        Some(65001) => Some(encoding_rs::UTF_8),
        Some(932) => Some(encoding_rs::SHIFT_JIS),
        Some(936) => Some(encoding_rs::GBK),
        Some(949) => Some(encoding_rs::EUC_KR),
        Some(950) => Some(encoding_rs::BIG5),
        Some(value) => encoding_rs::Encoding::for_label(format!("windows-{value}").as_bytes()),
        None => encoding_rs::Encoding::for_label(codepage.as_bytes()),
    }
}

/// Entries of a decoded playlist in playlist order.
pub fn parse_entries(text: &str, kind: PlaylistKind) -> Vec<RawEntry> {
    let lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim_start_matches('\u{feff}').trim()));
    match kind {
        PlaylistKind::M3u | PlaylistKind::M3u8 => lines
            .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
            .map(|(line, location)| RawEntry {
                line,
                location: location.to_string(),
            })
            .collect(),
        PlaylistKind::Pls => {
            let mut entries: Vec<(u64, RawEntry)> = lines
                .filter_map(|(line, text)| {
                    let (key, value) = text.split_once('=')?;
                    let key = key.trim();
                    let number = key
                        .get(..4)
                        .filter(|prefix| prefix.eq_ignore_ascii_case("file"))
                        .and_then(|_| key[4..].parse::<u64>().ok())?;
                    let value = value.trim();
                    (!value.is_empty()).then(|| {
                        (
                            number,
                            RawEntry {
                                line,
                                location: value.to_string(),
                            },
                        )
                    })
                })
                .collect();
            entries.sort_by_key(|(number, entry)| (*number, entry.line));
            entries.into_iter().map(|(_, entry)| entry).collect()
        }
        PlaylistKind::Cue => lines
            .filter_map(|(line, text)| {
                let keyword = text.get(..5)?;
                if !keyword.eq_ignore_ascii_case("file ") {
                    return None;
                }
                let rest = text[5..].trim();
                let location = if let Some(quoted) = rest.strip_prefix('"') {
                    quoted.split('"').next().unwrap_or_default().to_string()
                } else {
                    // Unquoted names end before the trailing file type (WAVE, MP3, ...).
                    match rest.rsplit_once(char::is_whitespace) {
                        Some((name, _)) => name.trim().to_string(),
                        None => rest.to_string(),
                    }
                };
                (!location.is_empty()).then_some(RawEntry { line, location })
            })
            .collect(),
    }
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = value.get(index + 1..index + 3)?;
            output.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

fn has_percent_escape(value: &str) -> bool {
    value
        .as_bytes()
        .windows(3)
        .any(|w| w[0] == b'%' && w[1].is_ascii_hexdigit() && w[2].is_ascii_hexdigit())
}

fn url_scheme(value: &str) -> Option<&str> {
    let (scheme, _) = value.split_once("://")?;
    (scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
    .then_some(scheme)
}

fn is_windows_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/'))
        || value.starts_with("\\\\")
}

/// Candidate local paths for one playlist entry, most literal first.
/// `file://` URLs are percent-decoded; on other systems than Windows,
/// backslash separators of Windows playlists are also tried as `/`.
pub fn entry_candidates(location: &str, playlist_directory: &Path) -> Result<Vec<PathBuf>, String> {
    entry_candidates_diagnostic(location, playlist_directory).map_err(|error| error.to_string())
}

pub(crate) fn entry_candidates_diagnostic(
    location: &str,
    playlist_directory: &Path,
) -> Result<Vec<PathBuf>, ErrorCause> {
    let location = location.trim();
    if let Some(scheme) = url_scheme(location) {
        if !scheme.eq_ignore_ascii_case("file") {
            return Err(ErrorCause::message(
                "{location}: only local files can be loaded, not {scheme} URLs",
                [
                    ("location", location.to_owned()),
                    ("scheme", scheme.to_owned()),
                ],
            ));
        }
        let rest = &location[scheme.len() + 3..];
        let decoded = percent_decode(rest).ok_or_else(|| {
            ErrorCause::message(
                "{location}: invalid file URL",
                [("location", location.to_owned())],
            )
        })?;
        let path = if let Some(local) = decoded.strip_prefix("localhost/") {
            format!("/{local}")
        } else if decoded.starts_with('/') {
            decoded
        } else {
            // file://server/share/... names a network share.
            format!("//{decoded}")
        };
        let bytes = path.as_bytes();
        // file:///C:/Music/... is a Windows drive path.
        let path = if bytes.len() >= 3 && bytes[0] == b'/' && bytes[2] == b':' {
            path[1..].to_string()
        } else {
            path
        };
        return Ok(vec![normalize(&native_separators(&path))]);
    }
    let mut candidates = Vec::new();
    let mut push = |value: &str| {
        let joined = normalize(&playlist_directory.join(value));
        if !candidates.contains(&joined) {
            candidates.push(joined);
        }
    };
    let alternate = (!cfg!(windows) && location.contains('\\') && !is_windows_absolute(location))
        .then(|| location.replace('\\', "/"));
    push(location);
    if let Some(alternate) = &alternate {
        push(alternate);
    }
    if has_percent_escape(location) {
        if let Some(decoded) = percent_decode(location) {
            push(&decoded);
            if !cfg!(windows) && decoded.contains('\\') {
                push(&decoded.replace('\\', "/"));
            }
        }
    }
    Ok(candidates)
}

/// Remove `.` and `..` components without touching the file system.
fn normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(output.components().next_back(), Some(Component::Normal(_))) {
                    output.pop();
                } else {
                    output.push("..");
                }
            }
            other => output.push(other.as_os_str()),
        }
    }
    output
}

fn native_separators(path: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(path.replace('/', "\\"))
    } else {
        PathBuf::from(path)
    }
}

/// The files of a playlist resolved against its folder, in playlist order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadedPlaylist {
    pub files: Vec<PathBuf>,
    /// Entries that could not be resolved, as (line, message).
    pub missing: Vec<(usize, String)>,
}

/// Resolve playlist entries; `exists` decides whether a candidate is present.
/// Duplicate references keep their first position.
pub fn resolve_entries(
    entries: &[RawEntry],
    playlist_directory: &Path,
    exists: impl Fn(&Path) -> bool,
) -> LoadedPlaylist {
    resolve_entries_diagnostic(entries, playlist_directory, exists).result
}

fn push_missing(report: &mut Detailed<LoadedPlaylist>, line: usize, cause: ErrorCause) {
    let error = cause.to_string();
    report.error_diagnostics.push(IndexedDiagnostic {
        index: report.result.missing.len(),
        diagnostic: BatchError::PlaylistLine { line, cause },
    });
    report.result.missing.push((line, error));
}

const MAX_XML_URI: usize = 64 * 1024;
type XmlBase = std::sync::Arc<Result<url::Url, ErrorCause>>;

#[derive(Clone, Debug)]
enum SourceLocation {
    Legacy(String),
    Xml {
        value: String,
        base: XmlBase,
        uri: bool,
    },
}
#[derive(Clone, Debug)]
struct InputEntry {
    line: usize,
    locations: Result<Vec<SourceLocation>, ErrorCause>,
}
fn resolve_entries_diagnostic(
    entries: &[RawEntry],
    playlist_directory: &Path,
    exists: impl Fn(&Path) -> bool,
) -> Detailed<LoadedPlaylist> {
    let entries: Vec<_> = entries
        .iter()
        .map(|entry| InputEntry {
            line: entry.line,
            locations: Ok(vec![SourceLocation::Legacy(entry.location.clone())]),
        })
        .collect();
    resolve_input_entries(&entries, playlist_directory, exists)
}
/// Share ordering, alternative selection, deduplication and indexed errors for
/// every reader; preserve the legacy single-location wrapper and its messages.
fn resolve_input_entries(
    entries: &[InputEntry],
    playlist_directory: &Path,
    exists: impl Fn(&Path) -> bool,
) -> Detailed<LoadedPlaylist> {
    let mut loaded = Detailed::plain(LoadedPlaylist::default());
    let mut seen = HashSet::new();
    for entry in entries {
        let locations = match &entry.locations {
            Ok(locations) if !locations.is_empty() => locations,
            Ok(_) => {
                push_missing(
                    &mut loaded,
                    entry.line,
                    ErrorCause::owned("The playlist entry has no media location"),
                );
                continue;
            }
            Err(cause) => {
                push_missing(&mut loaded, entry.line, cause.clone());
                continue;
            }
        };
        let mut failures = Vec::new();
        let mut found = None;
        for location in locations {
            let (authored, candidates) = match location {
                SourceLocation::Legacy(value) => (
                    value,
                    entry_candidates_diagnostic(value, playlist_directory),
                ),
                SourceLocation::Xml { value, base, uri } => {
                    (value, xml_candidates(value, base, *uri, playlist_directory))
                }
            };
            match candidates {
                Ok(candidates) => match candidates.into_iter().find(|path| exists(path)) {
                    Some(path) => {
                        found = Some(path);
                        break;
                    }
                    None => failures.push(ErrorCause::message(
                        "{location}: file not found",
                        [("location", authored.clone())],
                    )),
                },
                Err(cause) => failures.push(cause),
            }
        }
        if let Some(path) = found {
            if seen.insert(path.clone()) {
                loaded.result.files.push(path);
            }
        } else {
            let cause = if failures.len() == 1 {
                failures.remove(0)
            } else {
                ErrorCause::children(
                    "No usable local alternative: {cause}",
                    [("cause", ErrorCause::joined(failures, "; "))],
                )
            };
            push_missing(&mut loaded, entry.line, cause);
        }
    }
    loaded
}
fn network_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && matches!(bytes[0], b'/' | b'\\') && matches!(bytes[1], b'/' | b'\\')
}
fn xml_candidates(
    value: &str,
    base: &XmlBase,
    uri: bool,
    directory: &Path,
) -> Result<Vec<PathBuf>, ErrorCause> {
    let value = value.trim();
    if value.len() > MAX_XML_URI {
        return Err(ErrorCause::owned("XML playlist URI size limit exceeded"));
    }
    if value.is_empty() {
        return Err(ErrorCause::owned(
            "The playlist entry has no media location",
        ));
    }
    if network_prefix(value) {
        return Err(ErrorCause::message(
            "{location}: network shares are not supported by XML playlists",
            [("location", value.into())],
        ));
    }
    let windows_drive = value.as_bytes().get(1) == Some(&b':')
        && value.starts_with(|c: char| c.is_ascii_alphabetic());
    if !uri && windows_drive {
        if !is_windows_absolute(value) {
            return Err(ErrorCause::message(
                "{location}: Windows drive paths must be absolute",
                [("location", value.into())],
            ));
        }
        if !cfg!(windows) {
            return Err(ErrorCause::message(
                "{location}: Windows drive paths require Windows",
                [("location", value.into())],
            ));
        }
        return entry_candidates_diagnostic(value, directory);
    }
    let scheme = value.split_once(':').is_some_and(|(prefix, _)| {
        !prefix.is_empty()
            && prefix.starts_with(|c: char| c.is_ascii_alphabetic())
            && prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    });
    if uri || scheme {
        if percent_decode(value).is_none() || value.contains('\\') {
            return Err(ErrorCause::message(
                "{location}: invalid XML playlist URI",
                [("location", value.into())],
            ));
        }
        let resolved = base
            .as_ref()
            .as_ref()
            .map_err(Clone::clone)?
            .join(value)
            .map_err(|_| {
                ErrorCause::message(
                    "{location}: invalid XML playlist URI",
                    [("location", value.into())],
                )
            })?;
        Ok(vec![local_xml_uri(&resolved, value)?])
    } else {
        // Encoded leading separators may be an authored literal filename, but
        // must never introduce an implicit UNC fallback during path decoding.
        if percent_decode(value).is_some_and(|decoded| network_prefix(&decoded)) {
            return Ok(vec![normalize(&directory.join(value))]);
        }
        entry_candidates_diagnostic(value, directory)
    }
}
fn local_xml_uri(uri: &url::Url, authored: &str) -> Result<PathBuf, ErrorCause> {
    if uri.as_str().len() > MAX_XML_URI {
        return Err(ErrorCause::owned(
            "XML playlist resolved URI size limit exceeded",
        ));
    }
    if uri.scheme() != "file" {
        return Err(ErrorCause::message(
            "{location}: only local file URIs are supported by XML playlists",
            [("location", authored.into())],
        ));
    }
    if uri
        .host_str()
        .is_some_and(|host| !host.eq_ignore_ascii_case("localhost"))
        || uri.path().starts_with("//")
    {
        return Err(ErrorCause::message(
            "{location}: non-local file URI authority is not supported",
            [("location", authored.into())],
        ));
    }
    if uri.query().is_some() || uri.fragment().is_some() {
        return Err(ErrorCause::message(
            "{location}: URI queries and fragments are not supported by XML playlists",
            [("location", authored.into())],
        ));
    }
    let path = uri.to_file_path().map_err(|_| {
        ErrorCause::message(
            "{location}: file URI cannot be used on this platform",
            [("location", authored.into())],
        )
    })?;
    if network_prefix(&path.to_string_lossy()) {
        return Err(ErrorCause::message(
            "{location}: non-local file URI authority is not supported",
            [("location", authored.into())],
        ));
    }
    Ok(path)
}

/// Read a playlist or cue sheet file from disk.
pub fn load_playlist(path: &Path, ansi_codepage: Option<&str>) -> Result<LoadedPlaylist, String> {
    load_playlist_diagnostic(path, ansi_codepage).map(|report| report.result)
}

/// Read a playlist with owned presentation data indexed by `result.missing`.
/// The legacy result, entry order, duplicate handling and English strings are retained.
pub fn load_playlist_diagnostic(
    path: &Path,
    ansi_codepage: Option<&str>,
) -> Result<Detailed<LoadedPlaylist>, String> {
    if let Some(kind) = xml::Kind::from_path(path) {
        let entries = xml::read(path, kind)?;
        return Ok(resolve_input_entries(
            &entries,
            path.parent().unwrap_or(Path::new(".")),
            |path| path.is_file(),
        ));
    }
    let kind = playlist_kind(path)
        .ok_or_else(|| format!("{} is not a playlist or cue sheet", path.display()))?;
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if kind == PlaylistKind::Cue {
        crate::cue::set_ansi_codepage(ansi_codepage.map(str::to_owned));
        return Ok(load_cue_diagnostic(path, &crate::cue::decode(&bytes).0));
    }
    let text = decode_playlist(&bytes, kind, ansi_codepage);
    let directory = path.parent().unwrap_or(Path::new("."));
    Ok(resolve_entries_diagnostic(
        &parse_entries(&text, kind),
        directory,
        |p| p.exists(),
    ))
}

/// The entries of a cue sheet: one row per AUDIO track with an `INDEX 01`
/// (a row path of the cue sheet, see [`crate::subsong_path`]), numbered in
/// sheet order. As in Mp3tag, a sheet with one track per FILE also lists its
/// tracks rather than the files, and its rows edit the sheet.
fn load_cue_diagnostic(path: &Path, text: &str) -> Detailed<LoadedPlaylist> {
    let sheet = crate::cue::CueSheet::parse(text);
    let directory = path.parent().unwrap_or(Path::new("."));
    let mut loaded = Detailed::plain(LoadedPlaylist::default());
    for (number, index) in sheet.subsongs().into_iter().enumerate() {
        let track = &sheet.tracks[index];
        let line = track.line() + 1;
        let Some(location) = track.file.as_deref() else {
            push_missing(
                &mut loaded,
                line,
                ErrorCause::owned("The track has no FILE line"),
            );
            continue;
        };
        let audio = match entry_candidates_diagnostic(location, directory) {
            Ok(candidates) => candidates.into_iter().find(|p| p.is_file()),
            Err(error) => {
                push_missing(&mut loaded, line, error);
                continue;
            }
        };
        if audio.is_none() {
            if !loaded
                .result
                .missing
                .iter()
                .any(|(_, m)| m.starts_with(location))
            {
                push_missing(
                    &mut loaded,
                    line,
                    ErrorCause::message(
                        "{location}: file not found",
                        [("location", location.to_owned())],
                    ),
                );
            }
            continue;
        }
        let entry = PathBuf::from(crate::subsong_path(&path.to_string_lossy(), number + 1));
        if !loaded.result.files.contains(&entry) {
            loaded.result.files.push(entry);
        }
    }
    loaded
}

/// One file written to a playlist.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaylistItem {
    pub path: PathBuf,
    /// Seconds; written rounded in extended info.
    pub duration: f64,
    /// Extended info text (`#EXTINF` title or PLS `TitleN`).
    pub title: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlaylistWriteOptions {
    /// Write `#EXTM3U`/`#EXTINF` (M3U) or `TitleN`/`LengthN` (PLS).
    pub extended: bool,
    /// Write paths relative to the playlist folder where possible.
    pub relative: bool,
    /// The working directory (Options → Playlist "Entries relative to working
    /// directory"): entries are relative only when both the file and the
    /// playlist lie inside it; other entries keep absolute paths. Without a
    /// working directory, any path sharing a root is made relative.
    pub root: Option<PathBuf>,
    /// Byte order mark for UTF-8 playlists.
    pub bom: bool,
    /// System ANSI codepage for `.m3u`. Without one (macOS, Linux) `.m3u` is
    /// written in Windows-1252, as it is read, unless it would not read back
    /// unchanged: then it is written in UTF-8.
    pub ansi_codepage: Option<String>,
}

/// Path of `source` relative to the folder of `playlist` when both share a root.
pub fn relative_path(source: &Path, playlist: &Path) -> PathBuf {
    crate::operations::playlist_path(source, playlist)
}

/// Render playlist bytes for `target`; the format follows its extension
/// (`.pls`, `.m3u8`, otherwise M3U). Lines end with CR LF.
pub fn render_playlist(
    target: &Path,
    items: &[PlaylistItem],
    options: &PlaylistWriteOptions,
) -> Result<Vec<u8>, String> {
    if xml::Kind::from_path(target).is_some() {
        return Err("XML playlists are read-only; write .m3u, .m3u8 or .pls instead".into());
    }
    let kind = playlist_kind(target).unwrap_or(PlaylistKind::M3u8);
    let mut text = String::new();
    let mut entries = Vec::with_capacity(items.len());
    for item in items {
        let inside = |path: &Path| {
            options
                .root
                .as_ref()
                .is_none_or(|root| path.starts_with(root))
        };
        let path = if options.relative
            && inside(&item.path)
            && inside(target.parent().unwrap_or(Path::new(".")))
        {
            relative_path(&item.path, target)
        } else {
            item.path.clone()
        };
        let path = path.to_string_lossy().into_owned();
        if path.contains(['\r', '\n']) {
            return Err("Playlist paths cannot contain line breaks".into());
        }
        entries.push(path);
    }
    let title = |item: &PlaylistItem| item.title.replace(['\r', '\n'], " ");
    let seconds = |item: &PlaylistItem| {
        if item.duration.is_finite() && item.duration > 0.0 {
            item.duration.round() as i64
        } else {
            -1
        }
    };
    if kind == PlaylistKind::Pls {
        text.push_str("[playlist]\r\n");
        for (index, (item, path)) in items.iter().zip(&entries).enumerate() {
            let number = index + 1;
            text.push_str(&format!("File{number}={path}\r\n"));
            if options.extended {
                text.push_str(&format!("Title{number}={}\r\n", title(item)));
                text.push_str(&format!("Length{number}={}\r\n", seconds(item)));
            }
        }
        text.push_str(&format!("NumberOfEntries={}\r\nVersion=2\r\n", items.len()));
    } else {
        if options.extended {
            text.push_str("#EXTM3U\r\n");
        }
        for (item, path) in items.iter().zip(&entries) {
            if options.extended {
                text.push_str(&format!("#EXTINF:{},{}\r\n", seconds(item), title(item)));
            }
            text.push_str(path);
            text.push_str("\r\n");
        }
    }
    let system = options.ansi_codepage.as_deref();
    let ansi = (kind == PlaylistKind::M3u)
        .then(|| match system {
            Some(codepage) => ansi_encoding(Some(codepage)),
            None => Some(encoding_rs::WINDOWS_1252),
        })
        .flatten()
        .filter(|encoding| *encoding != encoding_rs::UTF_8)
        // Without a system codepage, Windows-1252 is used only when it reads
        // back unchanged. Reading tries UTF-8 first, so 1252 bytes that happen
        // to be valid UTF-8 (`Â©` is C2 A9, read as `©`), or text 1252 cannot
        // store, are written as UTF-8 instead. A real system codepage is kept.
        .filter(|encoding| {
            system.is_some() || {
                let (bytes, _, unmappable) = encoding.encode(&text);
                !unmappable && (bytes.is_ascii() || std::str::from_utf8(&bytes).is_err())
            }
        });
    match ansi {
        Some(encoding) => {
            let (bytes, _, unmappable) = encoding.encode(&text);
            if unmappable {
                return Err(format!(
                    "{} uses characters that the {} codepage of .m3u playlists cannot store. Use the .m3u8 extension for a UTF-8 playlist.",
                    target.display(),
                    encoding.name()
                ));
            }
            Ok(bytes.into_owned())
        }
        None => {
            let mut bytes = if options.bom {
                vec![0xef, 0xbb, 0xbf]
            } else {
                Vec::new()
            };
            bytes.extend_from_slice(text.as_bytes());
            Ok(bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(text: &str, kind: PlaylistKind) -> Vec<String> {
        parse_entries(text, kind)
            .into_iter()
            .map(|e| e.location)
            .collect()
    }

    #[test]
    fn nonlocal_uri_diagnostics_preserve_the_authored_scheme() {
        assert_eq!(
            entry_candidates("Title://song", Path::new(".")).unwrap_err(),
            "Title://song: only local files can be loaded, not Title URLs"
        );
    }

    #[test]
    fn playlist_diagnostics_preserve_missing_order_and_authored_parts() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("existing.mp3"), b"x").unwrap();
        let missing = "Help {error}: Title; missing.mp3";
        let remote = "Title://Help/{error}: raw";
        let invalid = "file:///Help%ZZ{error}";
        let path = directory.path().join("practice.m3u8");
        std::fs::write(
            &path,
            format!(
                "#EXTM3U\r\nexisting.mp3\r\n{missing}\r\n{remote}\r\n{invalid}\r\nexisting.mp3\r\n"
            ),
        )
        .unwrap();
        let legacy = load_playlist(&path, None).unwrap();
        assert_eq!(legacy.files, [directory.path().join("existing.mp3")]);
        assert_eq!(
            legacy.missing,
            [
                (3, format!("{missing}: file not found")),
                (
                    4,
                    format!("{remote}: only local files can be loaded, not Title URLs")
                ),
                (5, format!("{invalid}: invalid file URL")),
            ]
        );
        let report = load_playlist_diagnostic(&path, None).unwrap();
        assert_eq!(report.result, legacy);
        assert_eq!(report.error_diagnostics.len(), 3);
        let details = serde_json::to_value(&report.error_diagnostics).unwrap();
        for (at, (line, error)) in legacy.missing.iter().enumerate() {
            assert_eq!(details[at]["index"], at);
            assert_eq!(details[at]["diagnostic"]["kind"], "playlist_line");
            assert_eq!(details[at]["diagnostic"]["line"], *line);
            assert_eq!(
                report.error_diagnostics[at].diagnostic.to_string(),
                format!("Line {line}: {error}")
            );
        }
        assert_eq!(details[0]["diagnostic"]["cause"]["kind"], "owned");
        assert_eq!(
            details[0]["diagnostic"]["cause"]["params"]["location"],
            missing
        );
        assert_eq!(
            details[1]["diagnostic"]["cause"]["params"]["location"],
            remote
        );
        assert_eq!(
            details[1]["diagnostic"]["cause"]["params"]["scheme"],
            "Title"
        );
        assert_eq!(
            details[2]["diagnostic"]["cause"]["params"]["location"],
            invalid
        );
    }

    #[test]
    fn playlist_diagnostics_keep_cue_missing_file_and_duplicate_behavior() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("practice.cue");
        std::fs::write(
            &path,
            "TRACK 01 AUDIO\n  INDEX 01 00:00:00\nFILE \"Help {error}.flac\" WAVE\n  TRACK 02 AUDIO\n    INDEX 01 00:00:00\n  TRACK 03 AUDIO\n    INDEX 01 00:01:00\n",
        )
        .unwrap();
        let legacy = load_playlist(&path, None).unwrap();
        assert!(legacy.files.is_empty());
        assert_eq!(
            legacy.missing,
            [
                (1, "The track has no FILE line".into()),
                (3, "Help {error}.flac: file not found".into())
            ]
        );
        let report = load_playlist_diagnostic(&path, None).unwrap();
        assert_eq!(report.result, legacy);
        assert_eq!(report.error_diagnostics.len(), 2);
        for (at, (line, error)) in legacy.missing.iter().enumerate() {
            assert_eq!(report.error_diagnostics[at].index, at);
            assert_eq!(
                report.error_diagnostics[at].diagnostic.to_string(),
                format!("Line {line}: {error}")
            );
        }
        let details = serde_json::to_value(&report.error_diagnostics).unwrap();
        assert_eq!(
            details[0]["diagnostic"]["cause"]["key"],
            "The track has no FILE line"
        );
        assert_eq!(
            details[1]["diagnostic"]["cause"]["params"]["location"],
            "Help {error}.flac"
        );
    }

    #[test]
    fn m3u_entries_skip_comments_and_extended_info() {
        let text =
            "\u{feff}#EXTM3U\r\n#EXTINF:12,A - B\r\n01 a.mp3\r\n\r\n  sub/02 b.flac  \n# note\n";
        assert_eq!(
            entries(text, PlaylistKind::M3u8),
            ["01 a.mp3", "sub/02 b.flac"]
        );
        assert_eq!(parse_entries(text, PlaylistKind::M3u)[1].line, 5);
    }

    #[test]
    fn pls_entries_follow_their_numbers() {
        let text = "[playlist]\nfile2=b.mp3\nTitle2=B\nFile1=a.mp3\nFile10=j.mp3\nNumberOfEntries=3\nVersion=2\n";
        assert_eq!(
            entries(text, PlaylistKind::Pls),
            ["a.mp3", "b.mp3", "j.mp3"]
        );
    }

    #[test]
    fn cue_sheets_list_their_file_lines() {
        let text = "REM GENRE Rock\nPERFORMER \"X\"\nFILE \"Disc 1.flac\" WAVE\n  TRACK 01 AUDIO\nFILE track two.mp3 MP3\nfile 'odd'\n";
        assert_eq!(
            entries(text, PlaylistKind::Cue),
            ["Disc 1.flac", "track two.mp3", "'odd'"]
        );
    }

    #[test]
    fn decoding_honours_bom_and_falls_back_to_ansi() {
        let utf16: Vec<u8> = [0xff, 0xfe]
            .into_iter()
            .chain("é.mp3".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        assert_eq!(decode_playlist(&utf16, PlaylistKind::M3u, None), "é.mp3");
        assert_eq!(
            decode_playlist(b"caf\xe9.mp3", PlaylistKind::M3u, None),
            "café.mp3"
        );
        assert_eq!(
            decode_playlist("café.mp3".as_bytes(), PlaylistKind::M3u, None),
            "café.mp3"
        );
        assert_eq!(
            decode_playlist(b"\xef\xbb\xbfa.mp3", PlaylistKind::M3u8, None),
            "a.mp3"
        );
    }

    #[test]
    fn entries_resolve_relative_url_and_windows_forms() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        std::fs::create_dir_all(root.join("Album")).unwrap();
        for name in ["Album/01 a.mp3", "b c.mp3", "Album/ü.flac"] {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        let absolute = root.join("b c.mp3");
        let url = format!(
            "file://{}{}",
            if cfg!(windows) { "/" } else { "" },
            absolute
                .to_string_lossy()
                .replace('\\', "/")
                .replace(' ', "%20")
        );
        let raw = [
            "Album\\01 a.mp3",
            "Album/01 a.mp3",
            url.as_str(),
            "Album/%C3%BC.flac",
            "missing.mp3",
            "http://radio.example/stream",
        ];
        let entries: Vec<RawEntry> = raw
            .iter()
            .enumerate()
            .map(|(index, location)| RawEntry {
                line: index + 1,
                location: location.to_string(),
            })
            .collect();
        let loaded = resolve_entries(&entries, root, |p| p.exists());
        let names: Vec<_> = loaded
            .files
            .iter()
            .map(|p| {
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        assert_eq!(names, ["Album/01 a.mp3", "b c.mp3", "Album/ü.flac"]);
        assert_eq!(
            loaded
                .missing
                .iter()
                .map(|(line, _)| *line)
                .collect::<Vec<_>>(),
            [5, 6]
        );
        assert!(loaded.missing[1].1.contains("http URLs"));
    }

    #[test]
    fn loading_a_cue_sheet_lists_every_audio_track() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("disc.flac"), b"x").unwrap();
        std::fs::write(directory.path().join("bonus.flac"), b"x").unwrap();
        let cue = directory.path().join("disc.cue");
        std::fs::write(
            &cue,
            "FILE \"disc.flac\" WAVE\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    INDEX 01 00:10:00\nFILE bonus.flac WAVE\n  TRACK 03 AUDIO\n    INDEX 01 00:00:00\n  TRACK 04 MODE1/2352\n    INDEX 01 00:20:00\n  TRACK 05 AUDIO\nFILE gone.flac WAVE\n  TRACK 06 AUDIO\n    INDEX 01 00:00:00\n",
        )
        .unwrap();
        let loaded = load_playlist(&cue, None).unwrap();
        let row = |n| PathBuf::from(crate::subsong_path(&cue.to_string_lossy(), n));
        // Data tracks and tracks without INDEX 01 are not listed; a FILE
        // with one track still lists that track.
        assert_eq!(loaded.files, [row(1), row(2), row(3)]);
        assert_eq!(loaded.missing, [(12, "gone.flac: file not found".into())]);
    }

    fn item(path: &str, duration: f64, title: &str) -> PlaylistItem {
        PlaylistItem {
            path: PathBuf::from(path),
            duration,
            title: title.into(),
        }
    }

    #[test]
    fn m3u8_writes_extended_info_and_relative_paths() {
        let items = [
            item("/music/a/01.mp3", 61.4, "A - One"),
            item("/music/b/02.mp3", 0.0, "B\nTwo"),
        ];
        let options = PlaylistWriteOptions {
            extended: true,
            relative: true,
            bom: true,
            ansi_codepage: None,
            root: None,
        };
        let bytes = render_playlist(Path::new("/music/a/list.m3u8"), &items, &options).unwrap();
        let separator = std::path::MAIN_SEPARATOR;
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            format!("\u{feff}#EXTM3U\r\n#EXTINF:61,A - One\r\n01.mp3\r\n#EXTINF:-1,B Two\r\n..{separator}b{separator}02.mp3\r\n")
        );
        let plain = PlaylistWriteOptions::default();
        assert_eq!(
            render_playlist(Path::new("/music/a/list.m3u8"), &items[..1], &plain).unwrap(),
            b"/music/a/01.mp3\r\n"
        );
    }

    #[test]
    fn relative_entries_stay_inside_the_working_directory() {
        let items = [
            item("/music/album/a/01.mp3", 1.0, ""),
            item("/music/album/b/02.mp3", 1.0, ""),
            item("/elsewhere/03.mp3", 1.0, ""),
        ];
        let options = PlaylistWriteOptions {
            relative: true,
            root: Some(PathBuf::from("/music/album")),
            ..Default::default()
        };
        let separator = std::path::MAIN_SEPARATOR;
        let render = |target: &str| {
            String::from_utf8(render_playlist(Path::new(target), &items, &options).unwrap())
                .unwrap()
        };
        // In the working directory and in a folder below it.
        assert_eq!(
            render("/music/album/list.m3u8"),
            format!("a{separator}01.mp3\r\nb{separator}02.mp3\r\n/elsewhere/03.mp3\r\n")
        );
        assert_eq!(
            render("/music/album/a/list.m3u8"),
            format!("01.mp3\r\n..{separator}b{separator}02.mp3\r\n/elsewhere/03.mp3\r\n")
        );
        // A playlist outside the working directory lists absolute paths.
        assert_eq!(
            render("/music/list.m3u8"),
            "/music/album/a/01.mp3\r\n/music/album/b/02.mp3\r\n/elsewhere/03.mp3\r\n"
        );
        let unbounded = PlaylistWriteOptions {
            root: None,
            ..options.clone()
        };
        assert_eq!(
            String::from_utf8(
                render_playlist(Path::new("/music/list.m3u8"), &items[..1], &unbounded).unwrap()
            )
            .unwrap(),
            format!("album{separator}a{separator}01.mp3\r\n")
        );
    }

    #[test]
    fn pls_and_ansi_m3u_output() {
        let items = [item("/m/é.mp3", 3.0, "É")];
        let options = PlaylistWriteOptions {
            extended: true,
            relative: true,
            bom: true,
            ansi_codepage: Some("1252".into()),
            root: None,
        };
        assert_eq!(
            String::from_utf8(render_playlist(Path::new("/m/x.pls"), &items, &options).unwrap())
                .unwrap(),
            "\u{feff}[playlist]\r\nFile1=é.mp3\r\nTitle1=É\r\nLength1=3\r\nNumberOfEntries=1\r\nVersion=2\r\n"
        );
        let ansi = render_playlist(Path::new("/m/x.m3u"), &items, &options).unwrap();
        assert_eq!(ansi, b"#EXTM3U\r\n#EXTINF:3,\xc9\r\n\xe9.mp3\r\n");
        let cat = [item("/m/猫.mp3", 3.0, "")];
        assert!(render_playlist(Path::new("/m/x.m3u"), &cat, &options)
            .unwrap_err()
            .contains(".m3u8"));
        let no_codepage = PlaylistWriteOptions {
            ansi_codepage: None,
            ..options
        };
        // Without a system codepage .m3u uses Windows-1252, as reading does,
        // and UTF-8 only when an entry needs it.
        let fallback = render_playlist(Path::new("/m/x.m3u"), &items, &no_codepage).unwrap();
        assert_eq!(fallback, b"#EXTM3U\r\n#EXTINF:3,\xc9\r\n\xe9.mp3\r\n");
        assert_eq!(
            decode_playlist(&fallback, PlaylistKind::M3u, None),
            "#EXTM3U\r\n#EXTINF:3,É\r\né.mp3\r\n"
        );
        // Windows-1252 bytes that are also valid UTF-8 would read back as
        // other characters, so such a playlist is UTF-8, and every name
        // round-trips through the reader.
        for (name, bom) in ["Â©.mp3", "Ã©t©.mp3", "plain.mp3", "é.mp3", "猫.mp3"]
            .into_iter()
            .flat_map(|name| [(name, true), (name, false)])
        {
            let items = [item(&format!("/m/{name}"), 3.0, name)];
            let options = PlaylistWriteOptions {
                bom,
                ..no_codepage.clone()
            };
            let bytes = render_playlist(Path::new("/m/x.m3u"), &items, &options).unwrap();
            let text = decode_playlist(&bytes, PlaylistKind::M3u, None);
            assert_eq!(
                parse_entries(&text, PlaylistKind::M3u)
                    .into_iter()
                    .map(|entry| entry.location)
                    .collect::<Vec<_>>(),
                [name],
                "{name}"
            );
            assert!(text.contains(&format!("#EXTINF:3,{name}")), "{name}");
        }
        let mojibake = [item("/m/Â©.mp3", 3.0, "")];
        let bytes = render_playlist(Path::new("/m/x.m3u"), &mojibake, &no_codepage).unwrap();
        assert!(bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        let unicode = render_playlist(Path::new("/m/x.m3u"), &cat, &no_codepage).unwrap();
        assert_eq!(
            decode_playlist(&unicode, PlaylistKind::M3u, None),
            "#EXTM3U\r\n#EXTINF:3,\r\n猫.mp3\r\n"
        );
    }

    #[test]
    fn written_playlists_load_back_in_order() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let names = ["b.mp3", "a.mp3", "sub/c.flac"];
        std::fs::create_dir_all(root.join("sub")).unwrap();
        for name in names {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        for (target, relative) in [
            ("list.m3u8", true),
            ("list.pls", false),
            ("out/list.m3u", true),
        ] {
            let target = root.join(target);
            let items: Vec<_> = names
                .iter()
                .map(|name| PlaylistItem {
                    path: root.join(name),
                    duration: 1.0,
                    title: name.to_string(),
                })
                .collect();
            let options = PlaylistWriteOptions {
                extended: true,
                relative,
                ..Default::default()
            };
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(&target, render_playlist(&target, &items, &options).unwrap()).unwrap();
            let loaded = load_playlist(&target, None).unwrap();
            assert_eq!(
                loaded.files,
                names.iter().map(|n| root.join(n)).collect::<Vec<_>>(),
                "{}",
                target.display()
            );
        }
    }
    #[test]
    fn xml_resolution_never_attempts_mixed_or_percent_encoded_network_paths() {
        let directory = tempfile::tempdir().unwrap();
        let base = url::Url::from_file_path(directory.path().join("list.wpl")).unwrap();
        let locations = [
            r"\/remote.invalid/share/song.wav",
            r"/\remote.invalid/share/song.wav",
            "%5C%5Cremote.invalid%5Cshare%5Csong.wav",
            "file:///%2Fremote.invalid/share/song.wav",
        ];
        for value in locations {
            let entries = [InputEntry {
                line: 1,
                locations: Ok(vec![SourceLocation::Xml {
                    value: value.into(),
                    base: std::sync::Arc::new(Ok(base.clone())),
                    uri: value.starts_with("file:"),
                }]),
            }];
            let loaded = resolve_input_entries(&entries, directory.path(), |path| {
                assert!(
                    !network_prefix(&path.to_string_lossy()),
                    "network path reached filesystem predicate: {}",
                    path.display()
                );
                false
            });
            assert!(loaded.result.files.is_empty());
            assert_eq!(loaded.result.missing.len(), 1);
        }
    }
    #[test]
    fn oversized_joined_xml_location_never_reaches_filesystem_probe() {
        let base = std::sync::Arc::new(Ok(url::Url::parse(&format!(
            "file:///tmp/{}/",
            "a".repeat(33_000)
        ))
        .unwrap()));
        let entries = [InputEntry {
            line: 1,
            locations: Ok(vec![SourceLocation::Xml {
                value: format!("{}.wav", "b".repeat(33_000)),
                base,
                uri: true,
            }]),
        }];
        let report = resolve_input_entries(&entries, Path::new("/tmp"), |_| {
            panic!("oversized resolved URI reached filesystem probe")
        });
        assert!(report.result.files.is_empty());
        assert_eq!(report.result.missing.len(), 1);
        assert!(report.result.missing[0]
            .1
            .contains("resolved URI size limit"));
    }
}
