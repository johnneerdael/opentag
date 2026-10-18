//! User-installed web sources (Mp3tag's Web Sources Framework): `.src`
//! description files in the sources folder of the configuration folder, with
//! their `.inc` includes and `.settings` configuration schemas.
//!
//! A search downloads `[IndexUrl]`, runs `[ParserScriptIndex]` and splits its
//! output by `[IndexFormat]` into result rows. Choosing a row downloads the
//! album page and runs `[ParserScriptAlbum]`, whose output buffers become a
//! [`SourceRelease`] for the shared review and import step.
#[cfg(test)]
mod compat;
mod engine;
mod installer;
mod source;

use super::http::{self, Channel};
use super::model::{self, Provider, SourceCover, SourceRelease, SourceTrack};
use reqwest::Url;
use serde::Serialize;
use serde_json::{Map, Value};
use source::{Block, SourceFile};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tag_core::Fields;
use tauri::Emitter;

/// Emitted after sources were installed so every window rebuilds its menu.
pub const CHANGED_EVENT: &str = "opentag://web-sources";
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// Sent unless a source requests the application's own agent with `[UserAgent]=1`.
/// Keep the existing wire identity across user-visible branding changes.
const BROWSER_AGENT: &str = concat!(
    "Mozilla/5.0 (compatible; OpenTag/",
    env!("CARGO_PKG_VERSION"),
    ")"
);

/// `<configuration folder>/data/sources`, as in Mp3tag.
pub fn folder(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::action_host::library_path(app)?
        .with_file_name("data")
        .join("sources"))
}

/// Subfolders of the sources folder searched for sources (a downloaded
/// archive unpacked with its own folder).
const MAX_DEPTH: usize = 3;

/// One plain file or folder name.
fn plain_name(name: &str) -> Result<&str, String> {
    let ok = !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', '\0'])
        && !name.starts_with('.');
    ok.then_some(name)
        .ok_or_else(|| format!("\"{name}\" is not a file name in the sources folder"))
}

/// Source files are addressed by their `/`-separated path inside the folder.
fn relative_name(name: &str) -> Result<Vec<&str>, String> {
    let parts: Vec<&str> = name.split('/').collect();
    if parts.len() > MAX_DEPTH + 1 || parts.iter().any(|part| plain_name(part).is_err()) {
        return Err(format!("\"{name}\" is not a file in the sources folder"));
    }
    Ok(parts)
}

/// The file `name` in the sources folder. Names written in scripts
/// (`[Include]`, `[Settings]`) match regardless of case, as on Windows.
fn locate(folder: &Path, name: &str) -> Result<PathBuf, String> {
    let parts = relative_name(name)?;
    let exact = parts
        .iter()
        .fold(folder.to_path_buf(), |path, part| path.join(part));
    if exact.exists() {
        return Ok(exact);
    }
    let mut path = folder.to_path_buf();
    for part in parts {
        let lower = part.to_lowercase();
        path = std::fs::read_dir(&path)
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .find(|entry| entry.file_name().to_string_lossy().to_lowercase() == lower)
            })
            .map(|entry| entry.path())
            .ok_or_else(|| format!("{name} was not found in the sources folder"))?;
    }
    Ok(path)
}

fn reader(folder: &Path) -> impl Fn(&str) -> Result<String, String> + '_ {
    move |name: &str| {
        let path = locate(folder, name)?;
        let size = std::fs::metadata(&path)
            .map_err(|_| format!("{name} was not found in the sources folder"))?
            .len();
        if size > MAX_FILE_BYTES {
            return Err(format!("{name} is larger than 4 MB"));
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("{name}: {e}"))?;
        Ok(source::decode(&bytes))
    }
}

fn load(folder: &Path, id: &str) -> Result<SourceFile, String> {
    relative_name(id)?;
    let mut file = source::load(id, &reader(folder))?;
    // A schema next to a source in a subfolder belongs to it.
    if let Some(settings) = file.settings.take() {
        let settings = settings.replace('\\', "/");
        file.settings = Some(
            source::sibling(id, &settings)
                .filter(|path| locate(folder, path).is_ok())
                .unwrap_or(settings),
        );
    }
    Ok(file)
}

fn extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default()
}
fn stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}
/// The menu path of a file: its name without extension; the folders it is
/// in are not part of it, so unpacked archives look as in Mp3tag.
fn menu_path(id: &str) -> String {
    stem(id.rsplit('/').next().unwrap_or(id)).to_owned()
}

#[derive(Clone, Debug, Serialize)]
pub struct WebSourceInfo {
    /// File path in the sources folder (`/`-separated).
    pub id: String,
    /// Menu path: `#` separates submenus, `&` marks the access key.
    pub menu: String,
    pub name: String,
    pub search_by: Vec<source::SearchField>,
    /// No search results step: the search goes to the album page.
    pub direct: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct WebSettingsInfo {
    pub id: String,
    pub menu: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Problem {
    pub file: String,
    pub error: String,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Listing {
    pub folder: String,
    pub sources: Vec<WebSourceInfo>,
    pub settings: Vec<WebSettingsInfo>,
    pub problems: Vec<Problem>,
}

fn menu_order(a: &str, b: &str) -> std::cmp::Ordering {
    let key = |s: &str| s.replace('&', "").to_lowercase();
    key(a).cmp(&key(b)).then_with(|| a.cmp(b))
}

/// `/`-separated paths of the files in `folder` and its subfolders.
fn files_in(folder: &Path, prefix: &str, depth: usize, names: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if plain_name(&name).is_err() {
            continue;
        }
        let path = format!("{prefix}{name}");
        match entry.file_type() {
            Ok(kind) if kind.is_file() => names.push(path),
            Ok(kind) if kind.is_dir() && depth < MAX_DEPTH && name != "debug" => {
                files_in(&entry.path(), &format!("{path}/"), depth + 1, names)
            }
            _ => {}
        }
    }
}

/// Every source and configuration schema in `folder`, including those in
/// subfolders.
pub fn list(folder: &Path) -> Listing {
    let _guard = installer::LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Err(error) = installer::recover(folder) {
        return Listing {
            problems: vec![Problem {
                file: "sources".into(),
                error,
            }],
            ..Listing::default()
        };
    }
    let mut listing = Listing {
        folder: folder.to_string_lossy().into_owned(),
        ..Listing::default()
    };
    let mut names = Vec::new();
    files_in(folder, "", 0, &mut names);
    names.sort_by(|a, b| menu_order(&menu_path(a), &menu_path(b)).then_with(|| a.cmp(b)));
    for name in names {
        match extension(&name).as_str() {
            "src" => match load(folder, &name) {
                Ok(file) => listing.sources.push(WebSourceInfo {
                    menu: menu_path(&name),
                    id: name,
                    direct: file.direct(),
                    name: file.name,
                    search_by: file.search_by,
                }),
                Err(error) => listing.problems.push(Problem { file: name, error }),
            },
            "settings" => match schema(folder, &name) {
                Ok(_) => listing.settings.push(WebSettingsInfo {
                    menu: menu_path(&name),
                    id: name,
                }),
                Err(error) => listing.problems.push(Problem { file: name, error }),
            },
            _ => {}
        }
    }
    listing
}

/// A `.settings` configuration schema: an object with a `key` and `settings`.
fn schema(folder: &Path, id: &str) -> Result<Value, String> {
    relative_name(id)?;
    if extension(id) != "settings" {
        return Err(format!("{id} is not a .settings file"));
    }
    let schema: Value = serde_json::from_str(&reader(folder)(id)?)
        .map_err(|e| format!("{id} is not valid JSON: {e}"))?;
    let key_ok = schema
        .get("key")
        .and_then(Value::as_str)
        .is_some_and(|k| !k.is_empty());
    if !key_ok || !schema.get("settings").is_some_and(Value::is_array) {
        return Err(format!("{id} needs a \"key\" and a \"settings\" list"));
    }
    Ok(schema)
}

fn settings_store(folder: &Path) -> PathBuf {
    folder.join("settings.json")
}

fn stored_settings(folder: &Path) -> Result<Map<String, Value>, String> {
    match std::fs::read(settings_store(folder)) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(Value::Object(map)) => Ok(map),
            _ => Err("The web source settings file (settings.json) is not a JSON object.".into()),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(e.to_string()),
    }
}

/// Setting definitions that hold a value (not headings or separators).
fn value_settings(schema: &Value) -> impl Iterator<Item = &Value> {
    schema["settings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|setting| matches!(setting["type"].as_str(), Some("string" | "number" | "bool")))
        .filter(|setting| setting["key"].as_str().is_some())
}

/// The configured values of `schema`: stored values over the defaults.
fn settings_values(schema: &Value, stored: &Map<String, Value>) -> Map<String, Value> {
    let saved = stored
        .get(schema["key"].as_str().unwrap_or_default())
        .and_then(Value::as_object);
    value_settings(schema)
        .map(|setting| {
            let key = setting["key"].as_str().unwrap_or_default();
            let value = saved
                .and_then(|saved| saved.get(key))
                .filter(|value| kind_matches(setting, value))
                .or_else(|| setting.get("default"))
                .cloned()
                .unwrap_or(match setting["type"].as_str() {
                    Some("bool") => Value::Bool(false),
                    Some("number") => Value::from(0),
                    _ => Value::String(String::new()),
                });
            (key.to_owned(), value)
        })
        .collect()
}

fn kind_matches(setting: &Value, value: &Value) -> bool {
    match setting["type"].as_str() {
        Some("bool") => value.is_boolean(),
        Some("number") => value.is_number(),
        _ => value.is_string(),
    }
}

/// Configuration values as IfVar/SayVar read them.
fn vars(folder: &Path, file: &SourceFile) -> Result<BTreeMap<String, String>, String> {
    let Some(id) = &file.settings else {
        return Ok(BTreeMap::new());
    };
    let schema = schema(folder, id)?;
    Ok(settings_values(&schema, &stored_settings(folder)?)
        .into_iter()
        .map(|(key, value)| {
            let text = match value {
                Value::String(text) => text,
                other => other.to_string(),
            };
            (key, text)
        })
        .collect())
}

fn debug_enabled() -> bool {
    crate::settings::current()
        .pages
        .get("tag-sources")
        .and_then(|page| page.get("web_debug"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// A debug file name reduced to a plain, safe file name.
fn debug_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let clean: String = base
        .chars()
        .map(|c| {
            if c.is_control() || r#"<>:"|?*"#.contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean.trim().trim_start_matches('.').to_owned();
    if clean.is_empty() {
        "debug.out".into()
    } else {
        clean
    }
}

/// Write debug files to `<sources>/debug`; returns that folder when any were written.
fn write_debug(folder: &Path, files: &[engine::DebugFile]) -> Option<PathBuf> {
    let _guard = installer::LOCK.lock().ok()?;
    installer::recover(folder).ok()?;
    if files.is_empty() {
        return None;
    }
    let directory = folder.join("debug");
    let mut written = false;
    for file in files {
        let path = directory.join(debug_name(&file.name));
        written |= crate::action_host::atomic_output(&path, file.content.as_bytes(), false).is_ok();
    }
    written.then_some(directory)
}

/// Ordinary errors keep their existing string IPC shape. Script failures carry
/// provenance because `): ` is valid in both commands and diagnostic data.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum WebSourceError {
    Plain(String),
    Script {
        web_source_script: Box<ScriptFailure>,
    },
}
#[derive(Clone, Debug, Serialize)]
pub struct ScriptFailure {
    file: String,
    section: String,
    line: usize,
    command: String,
    message: String,
    debug_path: Option<String>,
}
impl From<String> for WebSourceError {
    fn from(message: String) -> Self {
        Self::Plain(message)
    }
}
impl From<&str> for WebSourceError {
    fn from(message: &str) -> Self {
        Self::Plain(message.into())
    }
}
impl std::fmt::Display for WebSourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plain(message) => write!(f, "{message}"),
            Self::Script {
                web_source_script: failure,
            } => {
                write!(
                    f,
                    "{} {} {}",
                    failure.file,
                    failure.section,
                    engine::ScriptError {
                        line: failure.line,
                        command: failure.command.clone(),
                        message: failure.message.clone()
                    }
                )?;
                if let Some(directory) = &failure.debug_path {
                    write!(f, ". Debug output is in {directory}")?;
                }
                Ok(())
            }
        }
    }
}
fn script_failure(
    block: &Block,
    section: &str,
    error: engine::ScriptError,
    debug_path: Option<PathBuf>,
) -> WebSourceError {
    WebSourceError::Script {
        web_source_script: Box::new(ScriptFailure {
            file: block.file.clone(),
            section: section.into(),
            line: error.line,
            command: error.command,
            message: error.message,
            debug_path: debug_path.map(|path| path.to_string_lossy().into_owned()),
        }),
    }
}

/// Run one parser script over a downloaded page.
fn execute(
    folder: &Path,
    id: &str,
    block: &Block,
    section: &str,
    page: &http::Page,
    vars: BTreeMap<String, String>,
    debug: bool,
) -> Result<BTreeMap<String, String>, WebSourceError> {
    let key = if section == "index" {
        "[ParserScriptIndex]"
    } else {
        "[ParserScriptAlbum]"
    };
    let script =
        engine::Script::compile(&block.lines).map_err(|e| script_failure(block, key, e, None))?;
    let name = stem(id);
    let env = engine::Environment {
        vars,
        current_url: page.url.as_str(),
        trace: debug.then(|| format!("{name}-{section}.out")),
    };
    let mut outcome = engine::run(&script, &page.text, &env);
    if debug {
        outcome.files.insert(
            0,
            engine::DebugFile {
                name: format!("{name}-{section}-input.out"),
                content: format!("{}\n\n{}", page.url, page.text),
            },
        );
    }
    let written = write_debug(folder, &outcome.files);
    outcome
        .result
        .map_err(|e| script_failure(block, key, e, written))
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct WebResult {
    pub url: String,
    pub cover: Option<String>,
    pub preview: Option<String>,
    /// One value per column.
    pub values: Vec<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct WebSearchPage {
    pub columns: Vec<String>,
    pub results: Vec<WebResult>,
    /// The only result is the album page itself (no index step).
    pub direct: bool,
}

/// `_url` takes the place of `%s` in `[AlbumUrl]` or is appended to it;
/// relative addresses resolve against the page.
fn album_address(file: &SourceFile, raw: &str, page: &Url) -> Option<Url> {
    let base = file.album_url.trim();
    let candidate = if base.is_empty() || Url::parse(raw).is_ok() {
        raw.to_owned()
    } else if base.contains("%s") {
        base.replace("%s", raw)
    } else {
        format!("{base}{raw}")
    };
    page.join(&candidate).ok().filter(http::web_allowed)
}

/// A value as written: `$verticalBar()` (evaluated by SayFormat, or left as
/// text by `Replace "|" "$verticalBar()"` on the input) is a literal pipe.
fn content(value: &str) -> String {
    const BAR: &[u8] = b"$verticalbar()";
    let mut text = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(ch) = rest.chars().next() {
        if ch == '$'
            && rest
                .as_bytes()
                .get(..BAR.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(BAR))
        {
            text.push('|');
            // The matched marker is ASCII, so its end is a UTF-8 boundary.
            rest = &rest[BAR.len()..];
        } else {
            text.push(if ch == engine::VERTICAL_BAR { '|' } else { ch });
            rest = &rest[ch.len_utf8()..];
        }
    }
    text.trim().to_owned()
}

/// A semicolon-terminated HTML reference. The WHATWG named set includes
/// non-BMP characters and references that expand to two Unicode scalars.
fn entity(name: &str) -> Option<String> {
    let (first, second) = if let Some(number) = name.strip_prefix('#') {
        let (digits, radix) = match number.strip_prefix(['x', 'X']) {
            Some(hex) => (hex, 16),
            None => (number, 10),
        };
        if digits.is_empty()
            || !digits.bytes().all(|byte| {
                if radix == 16 {
                    byte.is_ascii_hexdigit()
                } else {
                    byte.is_ascii_digit()
                }
            })
        {
            return None;
        }
        // With a nonempty valid digit sequence, a parse error only means
        // overflow: HTML replaces it just like any other out-of-range scalar.
        let code = u32::from_str_radix(digits, radix).unwrap_or(0x11_0000);
        let mut scalar = char::from_u32(code)
            .filter(|ch| *ch != '\0')
            .unwrap_or('\u{fffd}');
        if (0x80..=0x9f).contains(&code) {
            scalar = web_atoms::C1_REPLACEMENTS[(code - 0x80) as usize].unwrap_or(scalar);
        }
        (scalar as u32, 0)
    } else {
        *web_atoms::NAMED_ENTITIES.get(&format!("{name};"))?
    };
    let mut text = char::from_u32(first).filter(|c| *c != '\0')?.to_string();
    if second != 0 {
        text.push(char::from_u32(second)?);
    }
    Some(text)
}

/// Text as Mp3tag stores what a script output: HTML character references
/// (`&amp;`, `&#233;`, `&#x2019;`) and `\uXXXX` escapes are decoded. Scripts
/// rely on both, for example `Replace "\\\"" "&quot;"` to keep quotes out
/// of a pattern and `Say "\\u0009"` for a tab.
fn unescape(text: &str) -> String {
    if !text.contains(['&', '\\']) {
        return text.to_owned();
    }
    let hex4 = |s: &str| {
        s.get(..4)
            .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|h| u32::from_str_radix(h, 16).ok())
    };
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(['&', '\\']) {
        output.push_str(&rest[..at]);
        rest = &rest[at..];
        if rest.starts_with('&') {
            let end = rest[1..]
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'#')
                .count();
            let decoded = (rest.as_bytes().get(end + 1) == Some(&b';'))
                .then(|| entity(&rest[1..=end]))
                .flatten()
                .map(|text| (text, end + 2));
            match decoded {
                Some((text, used)) => {
                    output.push_str(&text);
                    rest = &rest[used..];
                }
                None => {
                    output.push('&');
                    rest = &rest[1..];
                }
            }
            continue;
        }
        let Some(high) = rest.strip_prefix("\\u").and_then(hex4) else {
            output.push('\\');
            rest = &rest[1..];
            continue;
        };
        let pair = (0xD800..0xDC00).contains(&high).then(|| {
            rest[6..]
                .strip_prefix("\\u")
                .and_then(hex4)
                .filter(|low| (0xDC00..0xE000).contains(low))
        });
        match pair.flatten() {
            Some(low) => {
                let code = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
                output.extend(char::from_u32(code));
                rest = &rest[12..];
            }
            None => {
                output.extend(char::from_u32(high).filter(|c| *c != '\0'));
                rest = &rest[6..];
            }
        }
    }
    output.push_str(rest);
    output
}

/// Search results: one per output line, its values split at `|` and named by
/// `[IndexFormat]`. Fields starting with `_` are not shown as columns.
fn index_results(file: &SourceFile, output: &str, page: &Url) -> Result<WebSearchPage, String> {
    let names = file.index_fields();
    let position = |name: &str| names.iter().position(|n| n.eq_ignore_ascii_case(name));
    let url_at = position("_url").ok_or("[IndexFormat] has no %_url% field")?;
    let (cover_at, preview_at) = (position("_coverurl"), position("_preview"));
    let visible: Vec<usize> = (0..names.len())
        .filter(|&i| !names[i].is_empty() && !names[i].starts_with('_'))
        .collect();
    let mut results = Vec::new();
    for line in output.split('\n') {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<String> = line.split('|').map(|v| content(&unescape(v))).collect();
        let get = |index: usize| values.get(index).map(String::as_str).unwrap_or_default();
        let Some(url) = album_address(file, get(url_at), page).filter(|_| !get(url_at).is_empty())
        else {
            continue;
        };
        let address = |index: Option<usize>| {
            index
                .map(get)
                .filter(|raw| !raw.is_empty())
                .and_then(|raw| page.join(raw).ok())
                .filter(http::web_allowed)
                .map(String::from)
        };
        results.push(WebResult {
            url: url.to_string(),
            cover: address(cover_at),
            preview: address(preview_at),
            values: visible.iter().map(|&i| get(i).to_owned()).collect(),
        });
    }
    Ok(WebSearchPage {
        columns: visible.iter().map(|&i| names[i].clone()).collect(),
        results,
        direct: false,
    })
}

/// Values of one field: `\\` separates multiple values.
fn field_values(text: &str) -> Vec<String> {
    unescape(text)
        .split("\\\\")
        .map(content)
        .filter(|v| !v.is_empty())
        .collect()
}

/// `m:ss`, `h:mm:ss`, or a plain number of milliseconds. Mp3tag documents
/// no format for `_LENGTH`; its own MusicBrainz source writes the API's
/// milliseconds there, and so do the iTunes sources, so a plain number is
/// read as milliseconds whatever its size.
fn parse_length(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if !text.contains(':') {
        return text
            .parse::<f64>()
            .ok()
            .filter(|ms| *ms >= 0.0)
            .map(|ms| ms / 1000.0);
    }
    text.split(':').try_fold(0.0, |total, part| {
        part.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| *v >= 0.0)
            .map(|v| total * 60.0 + v)
    })
}

fn leading_number(text: &str) -> Option<u32> {
    let digits: String = text
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok().filter(|n| *n > 0)
}

/// Album output buffers as a release. A buffer containing `|` holds one value
/// per track (`Title 1|Title 2|`); others apply to the whole album.
/// `COVERURL` lists cover images and `_LENGTH` track lengths. `TRACKS` lists
/// the tracks (one `|` each): it sets their number and gives their titles
/// when there is no `TITLE`. Other buffers starting with `_` are internal and
/// not written.
fn release_from(outputs: BTreeMap<String, String>, page: &Url, id: &str) -> SourceRelease {
    let mut album = Fields::new();
    let mut per_track: Vec<(String, Vec<String>)> = Vec::new();
    let mut covers: Vec<String> = Vec::new();
    let mut lengths: Vec<String> = Vec::new();
    let mut track_list: Option<Vec<String>> = None;
    let pieces = |value: &str| -> Vec<String> {
        let value = value.strip_suffix('|').unwrap_or(value);
        value.split('|').map(str::to_owned).collect()
    };
    for (name, value) in outputs {
        let value = value.replace("\r\n", "\n");
        if name.is_empty() || name == engine::DEFAULT_OUTPUT || name == "CURRENTURL" {
            continue;
        }
        match name.as_str() {
            "COVERURL" | "_COVERURL" => {
                covers.extend(pieces(&value).iter().map(|c| content(&unescape(c))))
            }
            "_LENGTH" => lengths = pieces(&value),
            "TRACKS" => track_list = (!value.is_empty()).then(|| pieces(&value)),
            internal if internal.starts_with('_') => {}
            _ if value.contains('|') => per_track.push((name, pieces(&value))),
            _ => model::put(&mut album, &name, field_values(&value)),
        }
    }
    if let Some(titles) = &track_list {
        let named = titles.iter().any(|title| !content(title).is_empty());
        if named && !per_track.iter().any(|(name, _)| name == "TITLE") {
            per_track.push(("TITLE".into(), titles.clone()));
        }
    }
    let count = track_list.as_ref().map(Vec::len).unwrap_or_else(|| {
        per_track
            .iter()
            .find(|(name, _)| name == "TITLE")
            .or_else(|| per_track.iter().max_by_key(|(_, values)| values.len()))
            .map_or(0, |(_, values)| values.len())
    });
    let first = |fields: &Fields, key: &str| fields.get(key).and_then(|v| v.first()).cloned();
    let tracks: Vec<SourceTrack> = (0..count)
        .map(|index| {
            let mut fields = Fields::new();
            for (name, values) in &per_track {
                if let Some(value) = values.get(index) {
                    model::put(&mut fields, name, field_values(value));
                }
            }
            let track = first(&fields, "TRACK");
            let disc = first(&fields, "DISCNUMBER").or_else(|| first(&album, "DISCNUMBER"));
            SourceTrack {
                position: track.clone().unwrap_or_else(|| (index + 1).to_string()),
                disc: disc.as_deref().and_then(leading_number).unwrap_or(1),
                number: track
                    .as_deref()
                    .and_then(leading_number)
                    .unwrap_or(index as u32 + 1),
                // A single-track release often gives TITLE for the album.
                title: first(&fields, "TITLE")
                    .or_else(|| (count == 1).then(|| first(&album, "TITLE")).flatten())
                    .unwrap_or_default(),
                artist: first(&fields, "ARTIST")
                    .or_else(|| first(&album, "ARTIST"))
                    .unwrap_or_default(),
                length: lengths.get(index).and_then(|l| parse_length(l)),
                fields,
            }
        })
        .collect();
    let covers = covers
        .into_iter()
        .filter(|c| !c.is_empty())
        .filter_map(|c| page.join(&c).ok().filter(http::web_allowed))
        .map(|url| SourceCover {
            url: url.to_string(),
            thumb: None,
            kind: "Cover".into(),
            width: None,
            height: None,
        })
        .collect();
    SourceRelease {
        provider: Provider::Web,
        id: id.to_owned(),
        url: page.to_string(),
        // ALBUM may be given per track (`Album|Album|`).
        title: first(&album, "ALBUM")
            .or_else(|| tracks.first().and_then(|t| first(&t.fields, "ALBUM")))
            .unwrap_or_default(),
        artist: first(&album, "ALBUMARTIST")
            .or_else(|| first(&album, "ARTIST"))
            .unwrap_or_default(),
        fields: album,
        tracks,
        covers,
        more_covers: false,
    }
}

/// Script work runs off the async runtime but, unlike file operations, does
/// not wait for the file operation lock.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

async fn blocking_web<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, WebSourceError> + Send + 'static,
) -> Result<T, WebSourceError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| WebSourceError::Plain(e.to_string()))?
}

fn user_agent(file: &SourceFile) -> Option<&'static str> {
    (!file.user_agent).then_some(BROWSER_AGENT)
}

async fn prepare(
    app: &tauri::AppHandle,
    id: &str,
) -> Result<(PathBuf, SourceFile, BTreeMap<String, String>), String> {
    let folder = folder(app)?;
    let id = id.to_owned();
    blocking(move || {
        let _guard = installer::LOCK.lock().map_err(|e| e.to_string())?;
        installer::recover(&folder)?;
        let file = load(&folder, &id)?;
        let vars = vars(&folder, &file)?;
        Ok((folder, file, vars))
    })
    .await
}

#[tauri::command]
pub async fn web_sources_list(app: tauri::AppHandle) -> Result<Listing, String> {
    let folder = folder(&app)?;
    blocking(move || Ok(list(&folder))).await
}

use installer::install;

#[tauri::command]
pub async fn web_sources_install(
    app: tauri::AppHandle,
    paths: Vec<String>,
) -> Result<Listing, String> {
    if paths.iter().any(|path| path.contains("://")) {
        return Err("Tag sources can only be installed from files on this computer.".into());
    }
    let folder = folder(&app)?;
    let listing = blocking(move || {
        install(&folder, &paths)?;
        Ok(list(&folder))
    })
    .await?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(listing)
}

#[tauri::command]
pub async fn web_sources_open_folder(app: tauri::AppHandle) -> Result<(), String> {
    let folder = folder(&app)?;
    blocking(move || {
        installer::prepare_folder_open(&folder)?;
        #[cfg(desktop)]
        return tauri_plugin_opener::open_path(&folder, None::<&str>).map_err(|e| e.to_string());
        #[cfg(mobile)]
        return Err("Opening folders is not available here".into());
    })
    .await
}

#[tauri::command]
pub async fn web_source_search(
    app: tauri::AppHandle,
    source: String,
    values: Vec<String>,
    ticket: u64,
) -> Result<WebSearchPage, WebSourceError> {
    http::claim(Channel::Search, ticket);
    let (folder, file, vars) = prepare(&app, &source).await?;
    if file.direct() {
        let raw = file.direct_url(&values);
        let url = Url::parse(&raw)
            .ok()
            .filter(http::web_allowed)
            .ok_or_else(|| {
                format!(
                    "{name} produced an address TagGroove cannot open: {url}",
                    name = file.name,
                    url = raw
                )
            })?;
        http::register_web_url(&url);
        let preview = file
            .direct_preview(&values)
            .and_then(|raw| Url::parse(&raw).ok())
            .filter(http::web_allowed);
        if let Some(preview) = &preview {
            http::register_web_url(preview);
        }
        return Ok(WebSearchPage {
            columns: Vec::new(),
            results: vec![WebResult {
                url: url.to_string(),
                preview: preview.map(String::from),
                ..WebResult::default()
            }],
            direct: true,
        });
    }
    let page = http::web_page(
        &file.name,
        &file.search_url(&values),
        user_agent(&file),
        Channel::Search,
        ticket,
    )
    .await?;
    let debug = debug_enabled();
    let found = blocking_web(move || {
        let block = file
            .index_script
            .as_ref()
            .ok_or("This source has no index script")?;
        let outputs = execute(&folder, &source, block, "index", &page, vars, debug)?;
        index_results(
            &file,
            outputs
                .get(engine::DEFAULT_OUTPUT)
                .map(String::as_str)
                .unwrap_or_default(),
            &page.url,
        )
        .map_err(WebSourceError::from)
    })
    .await?;
    for result in &found.results {
        for address in [
            Some(&result.url),
            result.cover.as_ref(),
            result.preview.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Ok(url) = Url::parse(address) {
                http::register_web_url(&url);
            }
        }
    }
    Ok(found)
}

#[tauri::command]
pub async fn web_source_release(
    app: tauri::AppHandle,
    source: String,
    url: String,
    ticket: u64,
) -> Result<SourceRelease, WebSourceError> {
    http::claim(Channel::Release, ticket);
    let address = Url::parse(&url)
        .ok()
        .filter(http::web_registered)
        .ok_or("This address did not come from the tag source's search. Search again.")?;
    let (folder, file, vars) = prepare(&app, &source).await?;
    let page = http::web_page(
        &file.name,
        address.as_str(),
        user_agent(&file),
        Channel::Release,
        ticket,
    )
    .await?;
    let debug = debug_enabled();
    let release = blocking_web(move || {
        let block = file
            .album_script
            .as_ref()
            .ok_or("This source has no album script")?;
        let outputs = execute(&folder, &source, block, "album", &page, vars, debug)?;
        Ok(release_from(outputs, &page.url, address.as_str()))
    })
    .await?;
    for address in std::iter::once(&release.url).chain(release.covers.iter().map(|c| &c.url)) {
        if let Ok(url) = Url::parse(address) {
            http::register_web_url(&url);
        }
    }
    Ok(release)
}

#[derive(Serialize)]
pub struct SettingsForm {
    pub schema: Value,
    pub values: Map<String, Value>,
}

#[tauri::command]
pub async fn web_source_settings(
    app: tauri::AppHandle,
    id: String,
) -> Result<SettingsForm, String> {
    let folder = folder(&app)?;
    blocking(move || {
        let _guard = installer::LOCK.lock().map_err(|e| e.to_string())?;
        installer::recover(&folder)?;
        let schema = schema(&folder, &id)?;
        let values = settings_values(&schema, &stored_settings(&folder)?);
        Ok(SettingsForm { schema, values })
    })
    .await
}

/// Keep the values the schema defines with the right type; other sources'
/// entries in the shared settings file stay as they are.
fn save_settings(folder: &Path, id: &str, values: Map<String, Value>) -> Result<(), String> {
    let _guard = installer::LOCK.lock().map_err(|e| e.to_string())?;
    installer::recover(folder)?;
    let schema = schema(folder, id)?;
    let mut checked = Map::new();
    for setting in value_settings(&schema) {
        let key = setting["key"].as_str().unwrap_or_default();
        if let Some(value) = values.get(key) {
            if !kind_matches(setting, value) {
                return Err(format!(
                    "{key} needs a {} value",
                    setting["type"].as_str().unwrap_or("text")
                ));
            }
            checked.insert(key.to_owned(), value.clone());
        }
    }
    let mut stored = stored_settings(folder)?;
    stored.insert(
        schema["key"].as_str().unwrap_or_default().to_owned(),
        Value::Object(checked),
    );
    let bytes = serde_json::to_vec_pretty(&Value::Object(stored)).map_err(|e| e.to_string())?;
    crate::action_host::atomic_output(&settings_store(folder), &bytes, false)
}

#[tauri::command]
pub async fn save_web_source_settings(
    app: tauri::AppHandle,
    id: String,
    values: Map<String, Value>,
) -> Result<(), String> {
    let folder = folder(&app)?;
    blocking(move || save_settings(&folder, &id, values)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn whole_direct_addresses_still_use_the_web_url_policy() {
        let file = source::load("Direct.src", &|_| {
            Ok(
                "[SearchBy]=URL||%www%||%s\n[PreviewUrl]=%s\n[ParserScriptAlbum]=...\nSayRest\n"
                    .into(),
            )
        })
        .unwrap();
        for (address, allowed) in [
            ("https://host.example/release?id=1&view=full#tracks", true),
            (
                "https://host.example/a%2Fb/space%20here?q=rock%26roll#part%202",
                true,
            ),
            ("file:///tmp/release", false),
            ("javascript:alert(1)", false),
            ("https://user:secret@host.example/release", false),
        ] {
            for raw in [
                file.direct_url(&[address.into()]),
                file.direct_preview(&[address.into()]).unwrap(),
            ] {
                let url = Url::parse(&raw).unwrap();
                assert_eq!(http::web_allowed(&url), allowed, "{address}");
                assert_eq!(url.as_str(), address);
            }
        }
    }

    fn write(folder: &Path, name: &str, text: &str) {
        std::fs::write(folder.join(name), text).unwrap();
    }

    const SETTINGS: &str = r#"{"key":"fixture","title":"Fixture","settings":[
        {"type":"heading","key":"h","title":"Heading"},
        {"type":"bool","key":"comment","title":"Comment","default":false},
        {"type":"string","key":"lang","title":"Language","choices":["en","de"],"default":"en"},
        {"type":"number","key":"limit","title":"Limit","default":3}]}"#;

    #[test]
    fn source_header_diagnostics_keep_file_names_and_tokens() {
        assert_eq!(
            plain_name("Help/Title").unwrap_err(),
            "\"Help/Title\" is not a file name in the sources folder"
        );
        let file = SourceFile::default();
        assert_eq!(
            index_results(&file, "", &Url::parse("https://example.invalid").unwrap()).unwrap_err(),
            "[IndexFormat] has no %_url% field"
        );
    }

    #[test]
    fn listing_menu_names_problems_and_settings() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        write(folder, "Shop#&Albums.src", "[Name]=Shop\n[IndexUrl]=https://shop.example/?q=%s\n[IndexFormat]=%_url%|%Album%\n[ParserScriptIndex]=...\nSayRest\n[ParserScriptAlbum]=...\nSayRest\n");
        write(
            folder,
            "Direct.src",
            "[AlbumUrl]=https://x.example/%s\n[ParserScriptAlbum]=...\nSayRest\n",
        );
        write(folder, "Broken.src", "[Name]=Broken\n");
        write(folder, "Shop#Settings.settings", SETTINGS);
        write(folder, "Bad.settings", "{}");
        write(folder, "notes.txt", "ignored");
        let listing = list(folder);
        let menus: Vec<_> = listing
            .sources
            .iter()
            .map(|s| (s.menu.as_str(), s.direct))
            .collect();
        assert_eq!(menus, [("Direct", true), ("Shop#&Albums", false)]);
        assert_eq!(listing.sources[1].name, "Shop");
        assert_eq!(listing.settings.len(), 1);
        assert_eq!(listing.settings[0].menu, "Shop#Settings");
        let problems: Vec<_> = listing.problems.iter().map(|p| p.file.as_str()).collect();
        assert_eq!(problems, ["Bad.settings", "Broken.src"]);
        assert!(list(&folder.join("missing")).sources.is_empty());
        assert!(load(folder, "../x.src").is_err());
    }

    #[test]
    fn settings_merge_defaults_validate_and_reach_scripts() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        write(folder, "S.settings", SETTINGS);
        let schema = schema(folder, "S.settings").unwrap();
        let values = settings_values(&schema, &stored_settings(folder).unwrap());
        assert_eq!(
            Value::Object(values),
            serde_json::json!({"comment":false,"lang":"en","limit":3})
        );
        write(
            folder,
            "settings.json",
            r#"{"other":{"x":1},"fixture":{"lang":5}}"#,
        );
        let mut update = Map::new();
        update.insert("comment".into(), Value::Bool(true));
        update.insert("unknown".into(), Value::Bool(true));
        save_settings(folder, "S.settings", update).unwrap();
        let stored = stored_settings(folder).unwrap();
        assert_eq!(stored["other"], serde_json::json!({"x":1}));
        assert_eq!(stored["fixture"], serde_json::json!({"comment":true}));
        let mut wrong = Map::new();
        wrong.insert("limit".into(), Value::String("many".into()));
        assert!(save_settings(folder, "S.settings", wrong)
            .unwrap_err()
            .contains("number"));
        let file = SourceFile {
            settings: Some("S.settings".into()),
            ..SourceFile::default()
        };
        let vars = vars(folder, &file).unwrap();
        assert_eq!(
            (
                vars["comment"].as_str(),
                vars["lang"].as_str(),
                vars["limit"].as_str()
            ),
            ("true", "en", "3")
        );
        assert!(schema_error(folder, "notes.txt").contains("not a .settings"));
    }
    fn schema_error(folder: &Path, id: &str) -> String {
        schema(folder, id).unwrap_err()
    }

    #[test]
    fn install_copies_only_source_files() {
        let from = tempfile::tempdir().unwrap();
        let to = tempfile::tempdir().unwrap();
        let target = to.path().join("data").join("sources");
        write(
            from.path(),
            "A#B.src",
            "[AlbumUrl]=https://x/%s\n[ParserScriptAlbum]=...\nSayRest\n",
        );
        write(from.path(), "A.inc", "");
        write(from.path(), "evil.sh", "");
        let path = |name: &str| from.path().join(name).to_string_lossy().into_owned();
        install(&target, &[path("A#B.src"), path("A.inc")]).unwrap();
        assert!(target.join("A#B.src").is_file() && target.join("A.inc").is_file());
        assert!(install(&target, &[path("evil.sh")])
            .unwrap_err()
            .contains("not a .src"));
        assert!(install(&target, &[path("missing.src")]).is_err());
        assert!(install(&target, &[]).is_err());
        assert_eq!(list(&target).sources[0].menu, "A#B");
    }

    #[test]
    fn literal_pipe_markers_preserve_unicode_case_and_nonmarkers() {
        let input = format!(
            " \t猫$verticalBar()é{}$VERTICALBAR()🙂$VeRtIcAlBaR()尾\n",
            engine::VERTICAL_BAR
        );
        assert_eq!(content(&input), "猫|é||🙂|尾");
        for value in [
            "$verticalBar(",
            "$verticalBar ()",
            "$verticalBar( )",
            "$vertıcalBar()",
            "$verticalBar()x",
        ] {
            let expected = if value == "$verticalBar()x" {
                "|x"
            } else {
                value
            };
            assert_eq!(content(value), expected);
        }
        assert_eq!(content("$vertical$verticalBar()bar()"), "$vertical|bar()");
    }

    #[test]
    fn near_limit_pipe_markers_decode_without_changing_other_text() {
        let marker = "$VeRtIcAlBaR()";
        let count = (16 * 1024 * 1024 - 16) / marker.len();
        let input = format!(" 猫{}尾 ", marker.repeat(count));
        assert_eq!(content(&input), format!("猫{}尾", "|".repeat(count)));
    }

    #[test]
    fn index_output_becomes_search_results() {
        let file = SourceFile {
            album_url: "https://shop.example/album/".into(),
            index_format: "%_url%|%Artist%|%Album%|%_coverurl%|%_preview%".into(),
            ..SourceFile::default()
        };
        let page = Url::parse("https://shop.example/search?q=x").unwrap();
        let output = format!(
            "12|Band|One{}Two|/img/12.jpg|https://shop.example/p/12\r\n\nhttps://other.example/a|X|Y||\n|no url|||\njavascript:alert(1)|J|K||\n",
            engine::VERTICAL_BAR
        );
        let found = index_results(&file, &output, &page).unwrap();
        assert_eq!(found.columns, ["Artist", "Album"]);
        assert_eq!(
            found.results,
            [
                WebResult {
                    url: "https://shop.example/album/12".into(),
                    cover: Some("https://shop.example/img/12.jpg".into()),
                    preview: Some("https://shop.example/p/12".into()),
                    values: vec!["Band".into(), "One|Two".into()],
                },
                WebResult {
                    url: "https://other.example/a".into(),
                    cover: None,
                    preview: None,
                    values: vec!["X".into(), "Y".into()],
                },
            ]
        );
        let no_url = SourceFile {
            index_format: "%album%".into(),
            ..SourceFile::default()
        };
        assert!(index_results(&no_url, "", &page)
            .unwrap_err()
            .contains("%_url%"));
    }

    #[test]
    fn album_outputs_become_a_release() {
        let page = Url::parse("https://shop.example/album/12").unwrap();
        let comment = format!("a{}b", engine::VERTICAL_BAR);
        let outputs: BTreeMap<String, String> = [
            ("ALBUM", "Discovery"),
            ("ARTIST", "Daft Punk"),
            ("GENRE", "House\\\\Electronic"),
            ("YEAR", " 2001 "),
            ("TITLE", "One More Time|Aerodynamic|"),
            ("TRACK", "1|2|"),
            ("ARTIST2", "|Guest|"),
            ("_LENGTH", "5:20|3:27|"),
            ("COVERURL", "/cover.jpg"),
            ("_INTERNAL", "x"),
            ("COMMENT", comment.as_str()),
            ("", "ignored"),
            ("CURRENTURL", "ignored"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let release = release_from(outputs, &page, "https://shop.example/album/12");
        assert_eq!(release.provider, Provider::Web);
        assert_eq!(
            (release.title.as_str(), release.artist.as_str()),
            ("Discovery", "Daft Punk")
        );
        assert_eq!(release.fields["GENRE"], ["House", "Electronic"]);
        assert_eq!(release.fields["YEAR"], ["2001"]);
        assert_eq!(release.fields["COMMENT"], ["a|b"]);
        assert!(
            !release.fields.contains_key("_INTERNAL") && !release.fields.contains_key("CURRENTURL")
        );
        assert_eq!(release.tracks.len(), 2);
        let second = &release.tracks[1];
        assert_eq!(
            (
                second.title.as_str(),
                second.number,
                second.disc,
                second.length
            ),
            ("Aerodynamic", 2, 1, Some(207.0))
        );
        assert_eq!(second.fields["ARTIST2"], ["Guest"]);
        assert!(!release.tracks[0].fields.contains_key("ARTIST2"));
        assert_eq!(release.tracks[0].artist, "Daft Punk");
        assert_eq!(release.covers[0].url, "https://shop.example/cover.jpg");
        // Album-only output (a covers source) has no tracks.
        let covers_only: BTreeMap<String, String> = [(
            "COVERURL".to_string(),
            "https://img.example/a.png|https://img.example/b.png|".to_string(),
        )]
        .into();
        let release = release_from(covers_only, &page, "x");
        assert!(release.tracks.is_empty());
        assert_eq!(release.covers.len(), 2);
        assert_eq!(parse_length("1:02:03"), Some(3723.0));
        assert_eq!(parse_length("61500"), Some(61.5));
        assert_eq!(parse_length("5000"), Some(5.0));
        assert_eq!(parse_length("10800000"), Some(10800.0));
        assert_eq!(parse_length("3:00:00"), Some(10800.0));
        assert_eq!(parse_length("x"), None);
    }

    #[test]
    fn numeric_html_references_apply_the_c1_replacement_table() {
        // WHATWG numeric-character-reference end state: unmapped C1 controls
        // remain themselves, rather than all being replaced or discarded.
        let expected = [
            '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
            '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ',
            '\u{9d}', 'ž', 'Ÿ',
        ];
        for (offset, ch) in expected.into_iter().enumerate() {
            let code = 0x80 + offset;
            assert_eq!(unescape(&format!("&#{code};")), ch.to_string());
            assert_eq!(unescape(&format!("&#X{code:X};")), ch.to_string());
        }
    }

    #[test]
    fn numeric_html_references_replace_invalid_scalars_but_keep_other_contracts() {
        for reference in [
            "&#0;",
            "&#x0000;",
            "&#55296;",
            "&#xDFFF;",
            "&#1114112;",
            "&#x110000;",
            "&#4294967296;",
            "&#999999999999999999999999999999;",
            "&#xFFFFFFFFFFFFFFFFFFFFFFFF;",
        ] {
            assert_eq!(unescape(reference), "\u{fffd}", "{reference}");
        }
        assert_eq!(
            unescape("&#65; &#x1F600; &#x10FFFF; &#xFDD0; &#xFFFF;"),
            "A 😀 \u{10ffff} \u{fdd0} \u{ffff}"
        );
        assert_eq!(unescape("&#127; &#160; &#9;"), "\u{7f} \u{a0} \t");
        for malformed in [
            "&#;",
            "&#x;",
            "&#xGG;",
            "&#128x;",
            "&#-1;",
            "&#128",
            "&unknown;",
        ] {
            assert_eq!(unescape(malformed), malformed);
        }
        assert_eq!(unescape("&amp;#128;"), "&#128;");
        // The separate backslash-u decoder keeps its existing behavior.
        assert_eq!(unescape(r"x\u0000y\ud800z"), "xyz");
    }

    #[test]
    fn numeric_html_references_survive_actual_release_conversion() {
        let page = Url::parse("https://shop.example/numeric").unwrap();
        let outputs = [
            ("ALBUM", "The &#128; sessions"),
            ("TITLE", "&#x80;|x&#0;y|&#xD800;|&#1114112;|&#x1F600;|"),
            ("COMMENT", "&#147;quoted&#148; &amp;#128;"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
        let release = release_from(outputs, &page, "numeric-entities");
        assert_eq!(release.fields["ALBUM"], ["The € sessions"]);
        let titles: Vec<_> = release
            .tracks
            .iter()
            .map(|track| track.title.as_str())
            .collect();
        assert_eq!(titles, ["€", "x�y", "�", "�", "😀"]);
        assert_eq!(release.fields["COMMENT"], ["“quoted” &#128;"]);
    }

    #[test]
    fn named_html_references_survive_source_import_as_unicode() {
        let page = Url::parse("https://shop.example/a").unwrap();
        let outputs = [
            ("ALBUM", "&Alpha; &Aopf; &NotEqualTilde;"),
            (
                "TITLE",
                "&CounterClockwiseContourIntegral;|&frac12; &euro; &#x1F600;|",
            ),
            ("COMMENT", "&unknown; &alpha &AMP; &amp;Alpha;"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
        let release = release_from(outputs, &page, "named-entities");
        assert_eq!(release.fields["ALBUM"], ["Α 𝔸 ≂̸"]);
        assert_eq!(release.tracks[0].title, "∳");
        assert_eq!(release.tracks[1].title, "½ € 😀");
        assert_eq!(release.fields["COMMENT"], ["&unknown; &alpha & &Alpha;"]);
    }

    #[test]
    fn output_conventions_of_published_scripts() {
        let page = Url::parse("https://shop.example/a").unwrap();
        let outputs: BTreeMap<String, String> = [
            // TRACKS lists the tracks; its values are titles when TITLE is absent.
            ("TRACKS", "One|Two|"),
            // Character references and \u escapes are decoded; \\ splits values.
            ("PUBLISHER", "R&amp;B &#x2019;n&#39; Co"),
            ("COMMENT", "a\\u0009b\\u000d\\u000ac"),
            ("URL", "x\\u005c\\u005cy"),
            // A pipe kept as `$verticalBar()` text by Replace on the input.
            ("NOTE", "a$verticalBar()b"),
            // Plain lengths are milliseconds.
            ("_LENGTH", "320357|5:20|"),
            ("COVERURL", "/img?a=1&amp;b=2"),
            ("OUTPUT", "default buffer"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let release = release_from(outputs, &page, "x");
        let titles: Vec<_> = release
            .tracks
            .iter()
            .map(|t| (t.title.as_str(), t.length))
            .collect();
        assert_eq!(titles, [("One", Some(320.357)), ("Two", Some(320.0))]);
        assert_eq!(release.fields["PUBLISHER"], ["R&B \u{2019}n' Co"]);
        assert_eq!(release.fields["COMMENT"], ["a\tb\r\nc"]);
        assert_eq!(release.fields["URL"], ["x", "y"]);
        assert_eq!(release.fields["NOTE"], ["a|b"]);
        assert!(!release.fields.contains_key("TRACKS") && !release.fields.contains_key("OUTPUT"));
        assert_eq!(release.covers[0].url, "https://shop.example/img?a=1&b=2");
        // A single track takes the album's TITLE and ALBUM may come per track.
        let single: BTreeMap<String, String> =
            [("TITLE", "Only"), ("ALBUM", "Disc|"), ("GENRE", "House|")]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        let release = release_from(single, &page, "x");
        assert_eq!(release.tracks[0].title, "Only");
        assert_eq!(release.title, "Disc");
        assert_eq!(
            unescape("&unknown; & \\x \\ud83d\\ude00"),
            "&unknown; & \\x \u{1F600}"
        );
        // %s in [AlbumUrl] takes the result address.
        let file = SourceFile {
            album_url: "https://api.example/releases/%s?full=1".into(),
            ..SourceFile::default()
        };
        assert_eq!(
            album_address(&file, "42", &page).unwrap().as_str(),
            "https://api.example/releases/42?full=1"
        );
    }

    #[test]
    fn sources_in_subfolders_and_names_in_any_case() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path();
        let unpacked = folder.join("Shop v2").join("scripts");
        std::fs::create_dir_all(&unpacked).unwrap();
        write(&unpacked, "Shop#&Release.src", "[SearchBy]=ID||%id%||https://shop.example/r/%s\n[Include]=shop.INC\n[Settings]=Shop#Options.Settings\n");
        write(&unpacked, "Shop.inc", "[ParserScriptAlbum]=...\nSayRest\n");
        write(&unpacked, "Shop#Options.settings", SETTINGS);
        std::fs::create_dir_all(folder.join("debug")).unwrap();
        write(&folder.join("debug"), "Ignored.src", "x");
        let listing = list(folder);
        assert!(listing.problems.is_empty(), "{:?}", listing.problems);
        assert_eq!(listing.sources.len(), 1);
        let source = &listing.sources[0];
        assert_eq!(
            (source.id.as_str(), source.menu.as_str(), source.direct),
            ("Shop v2/scripts/Shop#&Release.src", "Shop#&Release", true)
        );
        assert_eq!(listing.settings[0].menu, "Shop#Options");
        let file = load(folder, &source.id).unwrap();
        assert_eq!(
            file.settings.as_deref(),
            Some("Shop v2/scripts/Shop#Options.Settings")
        );
        assert_eq!(vars(folder, &file).unwrap()["lang"], "en");
        assert!(load(folder, "Shop v2/../x.src").is_err());
    }

    #[test]
    fn debug_files_get_safe_names() {
        assert_eq!(debug_name("../../etc/passwd"), "passwd");
        assert_eq!(debug_name("C:\\x\\a?.out"), "a_.out");
        assert_eq!(debug_name(".."), "debug.out");
        let dir = tempfile::tempdir().unwrap();
        let written = write_debug(
            dir.path(),
            &[engine::DebugFile {
                name: "../trace.out".into(),
                content: "x".into(),
            }],
        )
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(written.join("trace.out")).unwrap(),
            "x"
        );
        assert!(write_debug(dir.path(), &[]).is_none());
    }

    /// A local HTTP server answering each path with fixed content.
    fn serve(
        pages: Vec<(&'static str, &'static str, Vec<u8>)>,
    ) -> (u16, std::thread::JoinHandle<Vec<String>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..pages.len() {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let n = socket.read(&mut request).unwrap();
                let request = String::from_utf8_lossy(&request[..n]).into_owned();
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                let page = pages.iter().find(|(prefix, _, _)| path.starts_with(prefix));
                let (status, kind, body) = match page {
                    Some((_, kind, body)) => ("200 OK", *kind, body.clone()),
                    None => ("404 Not Found", "text/plain", Vec::new()),
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: {kind}\r\nconnection: close\r\ncontent-length: {}\r\n\r\n",
                    body.len()
                );
                socket.write_all(head.as_bytes()).unwrap();
                socket.write_all(&body).unwrap();
                requests.push(request);
            }
            requests
        });
        (port, handle)
    }

    #[test]
    fn script_failure_ipc_preserves_opaque_commands_and_diagnostics() {
        let folder = tempfile::tempdir().unwrap();
        let page = http::Page {
            url: Url::parse("https://example.invalid").unwrap(),
            text: "page".into(),
        };
        for section in ["index", "album"] {
            for command in [
                "IfVar Alpha): Beta",
                "If \"Alpha): Beta\"",
                "If \"Alpha\\\"): Beta\"",
                "FindLine \"Alpha): Beta\"",
            ] {
                let block = Block {
                    file: "Help  Title): .src".into(),
                    lines: vec![(17, command.into())],
                };
                let error = execute(
                    folder.path(),
                    "Help.src",
                    &block,
                    section,
                    &page,
                    BTreeMap::new(),
                    true,
                )
                .unwrap_err();
                let serialized = serde_json::to_value(&error).unwrap();
                let ipc_error: tauri::ipc::InvokeError = error.clone().into();
                assert_eq!(ipc_error.0, serialized);
                let failure = &serialized["web_source_script"];
                assert_eq!(failure["file"], block.file);
                assert_eq!(
                    failure["section"],
                    if section == "index" {
                        "[ParserScriptIndex]"
                    } else {
                        "[ParserScriptAlbum]"
                    }
                );
                assert_eq!(failure["line"], 17);
                assert_eq!(failure["command"], command);
                let message = if command.starts_with("FindLine") {
                    "\"Alpha): Beta\" was not found"
                } else {
                    "this block has no Endif"
                };
                assert_eq!(failure["message"], message);
                let expected = format!(
                    "{} {} line 17 ({command}): {message}",
                    block.file,
                    failure["section"].as_str().unwrap()
                );
                let shown = error.to_string();
                if command.starts_with("FindLine") {
                    let path = failure["debug_path"].as_str().unwrap();
                    assert_eq!(shown, format!("{expected}. Debug output is in {path}"));
                    assert!(Path::new(path).is_dir());
                } else {
                    assert!(failure["debug_path"].is_null());
                    assert_eq!(shown, expected);
                }
            }
        }
        assert_eq!(
            serde_json::to_value(WebSourceError::from("Cancelled: superseded")).unwrap(),
            "Cancelled: superseded"
        );
    }

    /// The whole pipeline against local pages: search URL, index script,
    /// album script over JSON, release fields and a registered cover address.
    #[test]
    fn search_and_album_scripts_run_against_local_pages() {
        let index = br#"<ul>
<li><a href="/album/7.json">Fixture Artist - Fixture Album</a> (1999)</li>
<li><a href="/album/8.json">Other - Second</a> (2005)</li>
</ul>"#
            .to_vec();
        let album = br#"{"title":"Fixture Album","artist":"Fixture Artist","year":1999,
            "cover":"/cover.png","tracks":[{"title":"First","ms":61000},{"title":"Second","ms":62000}]}"#
            .to_vec();
        let (port, server) = serve(vec![
            ("/search", "text/html; charset=utf-8", index),
            ("/album/7.json", "application/json", album),
        ]);
        let source_text = format!(
            r#"[Name]=Local
[IndexUrl]=http://127.0.0.1:{port}/search?q=%s
[AlbumUrl]=http://127.0.0.1:{port}
[WordSeparator]=+
[IndexFormat]=%_url%|%Release%|%Year%
[SearchBy]=Artist||%artist%||%s
[ParserScriptIndex]=...
Do
FindLine "<li>" 1 1
IfNot "<li>"
Else
FindInLine "href=\""
SayUntil "\""
Say "|"
FindInLine ">"
SayUntil "<"
Say "|"
FindInLine "("
SayUntil ")"
SayNewline
MoveLine 1
Endif
While "<li>"
[ParserScriptAlbum]=...
json "on"
OutputTo "ALBUM"
json_select "title"
SayRest
OutputTo "ARTIST"
json_select "artist"
SayRest
OutputTo "YEAR"
json_select "year"
SayRest
OutputTo "COVERURL"
json_select "cover"
SayRest
json_foreach "tracks"
OutputTo "TITLE"
json_select "title"
SayRest
Say "|"
OutputTo "_LENGTH"
json_select "ms"
SayDuration "ms"
Say "|"
json_foreach_end
"#
        );
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Local.src", &source_text);
        let file = load(dir.path(), "Local.src").unwrap();
        let address = file.search_url(&["Fixture Artist".into()]);
        assert_eq!(
            address,
            format!("http://127.0.0.1:{port}/search?q=Fixture+Artist")
        );
        let run = |address: String, channel| {
            tauri::async_runtime::block_on(http::web_page(
                "Local",
                &address,
                user_agent(&file),
                channel,
                u64::MAX,
            ))
            .unwrap()
        };
        let page = run(address, Channel::Search);
        let outputs = execute(
            dir.path(),
            "Local.src",
            file.index_script.as_ref().unwrap(),
            "index",
            &page,
            BTreeMap::new(),
            false,
        )
        .unwrap();
        let found = index_results(&file, &outputs[engine::DEFAULT_OUTPUT], &page.url).unwrap();
        assert_eq!(found.columns, ["Release", "Year"]);
        assert_eq!(found.results.len(), 2);
        assert_eq!(
            found.results[0].url,
            format!("http://127.0.0.1:{port}/album/7.json")
        );
        assert_eq!(
            found.results[0].values,
            ["Fixture Artist - Fixture Album", "1999"]
        );
        let page = run(found.results[0].url.clone(), Channel::Release);
        let outputs = execute(
            dir.path(),
            "Local.src",
            file.album_script.as_ref().unwrap(),
            "album",
            &page,
            BTreeMap::new(),
            true,
        )
        .unwrap();
        let release = release_from(outputs, &page.url, &found.results[0].url);
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 2);
        for request in requests {
            let agent = request.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("user-agent")
                    .then(|| value.trim())
            });
            assert_eq!(
                agent,
                Some(concat!(
                    "Mozilla/5.0 (compatible; OpenTag/",
                    env!("CARGO_PKG_VERSION"),
                    ")"
                )),
                "web-source search and album requests must preserve their wire identity"
            );
        }
        assert_eq!(release.title, "Fixture Album");
        assert_eq!(release.fields["YEAR"], ["1999"]);
        let titles: Vec<_> = release
            .tracks
            .iter()
            .map(|t| (t.title.as_str(), t.length))
            .collect();
        assert_eq!(titles, [("First", Some(61.0)), ("Second", Some(62.0))]);
        assert_eq!(
            release.covers[0].url,
            format!("http://127.0.0.1:{port}/cover.png")
        );
        // Debug was on for the album run: its trace and input are in debug/.
        let debug = dir.path().join("debug");
        assert!(std::fs::read_to_string(debug.join("Local-album.out"))
            .unwrap()
            .contains("json_foreach_end"));
        assert!(std::fs::read_to_string(debug.join("Local-album-input.out"))
            .unwrap()
            .contains("Fixture Album"));
        // Script errors name the file, script and line.
        let broken = Block {
            file: "Local.src".into(),
            lines: vec![(41, "FindLine \"nowhere\"".into())],
        };
        let error = execute(
            dir.path(),
            "Local.src",
            &broken,
            "album",
            &page,
            BTreeMap::new(),
            true,
        )
        .unwrap_err()
        .to_string();
        assert!(error.starts_with("Local.src [ParserScriptAlbum] line 41 (FindLine \"nowhere\"): \"nowhere\" was not found"), "{error}");
        assert!(error.contains("Debug output is in"), "{error}");
    }
}
