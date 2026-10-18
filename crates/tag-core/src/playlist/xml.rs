//! Bounded static XML playlist readers. No schema/entity/network resolution.
use super::{
    extension, local_xml_uri, ErrorCause, InputEntry, SourceLocation, XmlBase, MAX_XML_URI,
};
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader, XmlVersion};
use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
    path::Path,
    sync::Arc,
};
use url::Url;

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_EVENTS: usize = 400_000;
const MAX_LOCATION: usize = MAX_XML_URI;
const MAX_BASE_STORAGE: usize = 16 * 1024 * 1024;
const XSPF: &str = "http://xspf.org/ns/0/";
const XML: &str = "http://www.w3.org/XML/1998/namespace";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Xspf,
    Wpl,
    Asx,
}
impl Kind {
    pub(super) fn from_path(path: &Path) -> Option<Self> {
        match extension(path).as_str() {
            "xspf" => Some(Self::Xspf),
            "wpl" => Some(Self::Wpl),
            "asx" => Some(Self::Asx),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Namespace {
    Unbound,
    Xspf,
    Other,
}
fn resolve_namespace(namespace: ResolveResult<'_>) -> Result<Namespace, &'static str> {
    // Classification compares length before bytes; long inherited foreign URIs
    // stay borrowed in NsReader and are never copied for each Start/End event.
    match namespace {
        ResolveResult::Bound(namespace) if namespace.as_ref() == XSPF => Ok(Namespace::Xspf),
        ResolveResult::Bound(_) => Ok(Namespace::Other),
        ResolveResult::Unbound => Ok(Namespace::Unbound),
        ResolveResult::Unknown(_) => Err("Unknown XML namespace prefix"),
    }
}
fn xml_character(c: char) -> bool {
    matches!(c, '\u{9}' | '\u{a}' | '\u{d}' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
}
fn declaration_start(bytes: &[u8]) -> bool {
    bytes.starts_with(b"<?xml") && bytes.get(5).is_some_and(u8::is_ascii_whitespace)
}
fn declaration_encoding(text: &str) -> Result<Option<String>, String> {
    if !declaration_start(text.as_bytes()) {
        return Ok(None);
    }
    let end = text.find("?>").ok_or("Unclosed XML declaration")? + 2;
    if end > 4096 {
        return Err("XML declaration exceeds the size limit".into());
    }
    match NsReader::from_str(&text[..end])
        .read_event()
        .map_err(|e| e.to_string())?
    {
        Event::Decl(decl) => decl
            .encoding()
            .transpose()
            .map(|value| value.map(|value| value.into_owned()))
            .map_err(|e| e.to_string()),
        _ => Err("Invalid XML declaration".into()),
    }
}
fn decode(bytes: &[u8]) -> Result<String, String> {
    let (encoding, skip) = encoding_rs::Encoding::for_bom(bytes).unwrap_or((encoding_rs::UTF_8, 0));
    let bom = skip > 0;
    let preliminary = if bom {
        encoding
            .decode_without_bom_handling_and_without_replacement(&bytes[skip..])
            .ok_or("Invalid XML text encoding")?
            .into_owned()
    } else {
        // The declaration is ASCII in ASCII-compatible legacy encodings. Never
        // inspect or lossy-decode the body before selecting its stated decoder.
        let header = if declaration_start(bytes) {
            let end = bytes
                .windows(2)
                .position(|pair| pair == b"?>")
                .ok_or("Unclosed XML declaration")?
                + 2;
            if end > 4096 {
                return Err("XML declaration exceeds the size limit".into());
            }
            std::str::from_utf8(&bytes[..end]).map_err(|_| "Invalid XML declaration encoding")?
        } else {
            ""
        };
        let label = declaration_encoding(header)?;
        let selected = match label.as_deref() {
            Some(label) => encoding_rs::Encoding::for_label(label.as_bytes())
                .ok_or("Unsupported XML text encoding")?,
            None => encoding_rs::UTF_8,
        };
        if selected == encoding_rs::UTF_16LE || selected == encoding_rs::UTF_16BE {
            return Err("UTF-16 XML requires a byte-order mark".into());
        }
        selected
            .decode_without_bom_handling_and_without_replacement(bytes)
            .ok_or("Invalid XML text encoding")?
            .into_owned()
    };
    if bom {
        if let Some(label) = declaration_encoding(&preliminary)? {
            let stated = encoding_rs::Encoding::for_label(label.as_bytes())
                .ok_or("Unsupported XML text encoding")?;
            let generic_utf16 = label.eq_ignore_ascii_case("utf-16")
                && (encoding == encoding_rs::UTF_16LE || encoding == encoding_rs::UTF_16BE);
            if stated != encoding && !generic_utf16 {
                return Err("XML encoding declaration conflicts with its byte-order mark".into());
            }
        }
    }
    if preliminary.len() > MAX_BYTES {
        return Err("Decoded XML playlist exceeds the size limit".into());
    }
    if preliminary.chars().any(|c| !xml_character(c)) {
        return Err("XML playlist contains an invalid character".into());
    }
    Ok(preliminary)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Root,
    TrackList,
    Track,
    Location,
    Body,
    Seq,
    Media,
    Entry,
    Ref,
    Ignore,
}
struct Frame {
    role: Role,
    base: XmlBase,
    entry: Option<InputEntry>,
    text: String,
}
struct Parser {
    kind: Kind,
    frames: Vec<Frame>,
    entries: Vec<InputEntry>,
    base: XmlBase,
    bases: HashMap<String, XmlBase>,
    retained_base_bytes: usize,
    base_budget: usize,
    root_seen: bool,
    track_lists: usize,
    bodies: usize,
    sequences: usize,
}
impl Parser {
    // Intern each distinct retained base once. Inherited bases and repeated
    // explicit values clone only Arc handles, never URLs or diagnostics.
    fn retain_base(&mut self, value: Result<Url, ErrorCause>) -> Result<XmlBase, String> {
        let key = match &value {
            Ok(url) => {
                if url.as_str().len() > MAX_XML_URI {
                    return Err("XML playlist resolved base URI size limit exceeded".into());
                }
                format!("u:{url}")
            }
            Err(cause) => format!("e:{cause}"),
        };
        if let Some(base) = self.bases.get(&key) {
            return Ok(Arc::clone(base));
        }
        // Charge both the interning key and the owned payload, plus a fixed
        // allowance for the Arc/hash entry. No charge for cheap shared clones.
        let cost = key.len().saturating_mul(2).saturating_add(256);
        let total = self.retained_base_bytes.saturating_add(cost);
        if total > self.base_budget {
            return Err("XML playlist retained base URI size limit exceeded".into());
        }
        self.retained_base_bytes = total;
        let base = Arc::new(value);
        self.bases.insert(key, Arc::clone(&base));
        Ok(base)
    }

    fn unsupported(&mut self, line: usize, element: &str) {
        self.entries.push(InputEntry {
            line,
            locations: Err(ErrorCause::message(
                "{element} is not supported in static XML playlists",
                [("element", element.into())],
            )),
        });
    }
    fn start(
        &mut self,
        name: &str,
        namespace: Namespace,
        attrs: BTreeMap<String, String>,
        line: usize,
    ) -> Result<(), String> {
        if self.frames.len() >= MAX_DEPTH {
            return Err("XML playlist depth limit exceeded".into());
        }
        let parent = self.frames.last().map(|frame| frame.role);
        let qualified = if self.kind == Kind::Xspf {
            namespace == Namespace::Xspf
        } else {
            namespace == Namespace::Unbound
        };
        let name = if self.kind == Kind::Asx {
            name.to_ascii_lowercase()
        } else {
            name.into()
        };
        let role = if parent.is_none() {
            if self.root_seen {
                return Err("XML playlist has more than one root element".into());
            }
            self.root_seen = true;
            let expected = match self.kind {
                Kind::Xspf => "playlist",
                Kind::Wpl => "smil",
                Kind::Asx => "asx",
            };
            if !qualified || name != expected {
                return Err("XML root or namespace does not match the playlist extension".into());
            }
            if self.kind == Kind::Xspf && attrs.get("version").map(String::as_str) != Some("1") {
                return Err("Only XSPF version 1 is supported".into());
            }
            if self.kind == Kind::Asx
                && attrs.get("version").is_some_and(|v| v != "3" && v != "3.0")
            {
                return Err("Only XML ASX version 3 is supported".into());
            }
            Role::Root
        } else if parent == Some(Role::Location) {
            return Err("XSPF location elements must contain text only".into());
        } else if parent == Some(Role::Ignore) || !qualified {
            Role::Ignore
        } else {
            match (self.kind, parent, name.as_str()) {
                (Kind::Xspf, Some(Role::Root), "trackList") => {
                    self.track_lists += 1;
                    Role::TrackList
                }
                (Kind::Xspf, Some(Role::TrackList), "track") => Role::Track,
                (Kind::Xspf, Some(Role::Track), "location") => Role::Location,
                (Kind::Wpl, Some(Role::Root), "body") => {
                    self.bodies += 1;
                    Role::Body
                }
                (Kind::Wpl, Some(Role::Body), "seq") => {
                    self.sequences += 1;
                    Role::Seq
                }
                (Kind::Wpl, Some(Role::Seq), "media") => Role::Media,
                (Kind::Wpl, Some(Role::Seq), "smartPlaylist") => {
                    self.unsupported(line, "smartPlaylist");
                    Role::Ignore
                }
                (Kind::Asx, Some(Role::Root), "entry") => Role::Entry,
                (Kind::Asx, Some(Role::Entry), "ref") => Role::Ref,
                (Kind::Asx, Some(Role::Root), "entryref" | "repeat" | "event") => {
                    self.unsupported(line, &name.to_ascii_uppercase());
                    Role::Ignore
                }
                (Kind::Asx, Some(Role::Root | Role::Entry), "base") => {
                    return Err("ASX BASE is not supported in static XML playlists".into())
                }
                _ => Role::Ignore,
            }
        };
        let mut base = self
            .frames
            .last()
            .map(|frame| frame.base.clone())
            .unwrap_or_else(|| self.base.clone());
        if let Some(value) = attrs.get("xml:base") {
            if self.kind == Kind::Xspf {
                if value.len() > MAX_XML_URI {
                    return Err("XML playlist authored base URI size limit exceeded".into());
                }
                let resolved = match Url::parse(value) {
                    Ok(url) => Some(Ok(url)),
                    Err(_) => match base.as_ref() {
                        Ok(parent) => Some(parent.join(value)),
                        // Preserve the compact shared diagnostic for an invalid
                        // inherited base instead of copying it per descendant.
                        Err(_) => None,
                    },
                };
                if let Some(resolved) = resolved {
                    if resolved
                        .as_ref()
                        .is_ok_and(|url| url.as_str().len() > MAX_XML_URI)
                    {
                        return Err("XML playlist resolved base URI size limit exceeded".into());
                    }
                    let resolved = resolved
                        .ok()
                        .filter(|url| local_xml_uri(url, "").is_ok())
                        .ok_or_else(|| {
                            ErrorCause::message(
                                "Invalid or non-local XML base URI at line {line}",
                                [("line", line.to_string())],
                            )
                        });
                    base = self.retain_base(resolved)?;
                }
            } else if role != Role::Ignore {
                return Err("xml:base is supported only in XSPF playlists".into());
            }
        }
        let entry = matches!(role, Role::Track | Role::Media | Role::Entry).then(|| InputEntry {
            line,
            locations: Ok(Vec::new()),
        });
        let mut frame = Frame {
            role,
            base,
            entry,
            text: String::new(),
        };
        if role == Role::Media || role == Role::Ref {
            let key = if role == Role::Media { "src" } else { "href" };
            let value = attrs.get(key).cloned().unwrap_or_default();
            if value.len() > MAX_LOCATION {
                return Err("XML playlist location size limit exceeded".into());
            }
            let location = SourceLocation::Xml {
                value,
                base: frame.base.clone(),
                uri: false,
            };
            if role == Role::Media {
                frame
                    .entry
                    .as_mut()
                    .unwrap()
                    .locations
                    .as_mut()
                    .unwrap()
                    .push(location);
            } else if let Some(entry) = self
                .frames
                .last_mut()
                .and_then(|frame| frame.entry.as_mut())
            {
                if let Ok(locations) = &mut entry.locations {
                    locations.push(location);
                }
            }
        }
        self.frames.push(frame);
        Ok(())
    }
    fn text(&mut self, value: &str) -> Result<(), String> {
        if let Some(frame) = self.frames.last_mut() {
            if frame.role == Role::Location {
                if frame.text.len().saturating_add(value.len()) > MAX_LOCATION {
                    return Err("XML playlist location size limit exceeded".into());
                }
                frame.text.push_str(value);
            }
        } else if !value.trim().is_empty() {
            return Err("Text outside the XML playlist root".into());
        }
        Ok(())
    }
    fn end(&mut self) -> Result<(), String> {
        let frame = self.frames.pop().ok_or("Unexpected XML end element")?;
        if frame.role == Role::Location {
            if let Some(entry) = self
                .frames
                .last_mut()
                .and_then(|frame| frame.entry.as_mut())
            {
                if let Ok(locations) = &mut entry.locations {
                    locations.push(SourceLocation::Xml {
                        value: frame.text.trim().into(),
                        base: frame.base,
                        uri: true,
                    });
                }
            }
        } else if let Some(entry) = frame.entry {
            self.entries.push(entry);
        }
        Ok(())
    }
}

pub(super) fn read(path: &Path, kind: Kind) -> Result<Vec<InputEntry>, String> {
    read_with_base_budget(path, kind, MAX_BASE_STORAGE)
}
fn read_with_base_budget(
    path: &Path,
    kind: Kind,
    base_budget: usize,
) -> Result<Vec<InputEntry>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() > MAX_BYTES {
        return Err(format!(
            "{}: XML playlist exceeds the size limit",
            path.display()
        ));
    }
    let text = decode(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let base = Url::from_file_path(absolute)
        .map_err(|_| ErrorCause::owned("Cannot form a local playlist base URI"));
    let mut parser = Parser {
        kind,
        frames: Vec::new(),
        entries: Vec::new(),
        base: Arc::new(Ok(Url::parse("file:///").expect("static local URI"))),
        bases: HashMap::new(),
        retained_base_bytes: 0,
        base_budget,
        root_seen: false,
        track_lists: 0,
        bodies: 0,
        sequences: 0,
    };
    parser.base = parser.retain_base(base)?;
    let mut reader = NsReader::from_str(&text);
    reader.config_mut().enable_all_checks(true);
    reader.config_mut().expand_empty_elements = true;
    let lines: Vec<_> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(index, _)| index + 1))
        .collect();
    let mut declaration = false;
    let mut events = 0;
    loop {
        events += 1;
        if events > MAX_EVENTS {
            return Err(format!(
                "{}: XML playlist event limit exceeded",
                path.display()
            ));
        }
        let position = reader.buffer_position() as usize;
        let line = lines.partition_point(|start| *start <= position);
        let failure = |error: String| {
            format!(
                "{}: Invalid XML playlist at line {line}: {error}",
                path.display()
            )
        };
        let (namespace, event) = reader
            .read_resolved_event()
            .map_err(|e| failure(e.to_string()))?;
        let namespace = resolve_namespace(namespace).map_err(|error| failure(error.into()))?;
        match event {
            Event::Start(start) => {
                let mut attrs = BTreeMap::new();
                for attribute in start.attributes() {
                    let attribute = attribute.map_err(|e| failure(e.to_string()))?;
                    let value = attribute
                        .normalized_value_with(
                            XmlVersion::Explicit1_0,
                            1,
                            quick_xml::escape::resolve_xml_entity,
                        )
                        .map_err(|e| failure(e.to_string()))?
                        .into_owned();
                    if value.chars().any(|c| !xml_character(c)) {
                        return Err(failure("Invalid XML character reference".into()));
                    }
                    let (ns, local) = reader.resolver().resolve_attribute(attribute.key);
                    let key = match ns {
                        ResolveResult::Unbound => {
                            if kind == Kind::Asx {
                                local.as_ref().to_ascii_lowercase()
                            } else {
                                local.as_ref().into()
                            }
                        }
                        ResolveResult::Bound(ns)
                            if ns.as_ref() == XML && local.as_ref() == "base" =>
                        {
                            "xml:base".into()
                        }
                        ResolveResult::Unknown(_) => {
                            return Err(failure("Unknown XML attribute namespace prefix".into()))
                        }
                        _ => continue,
                    };
                    if attrs.insert(key, value).is_some() {
                        return Err(failure("Duplicate XML playlist attribute".into()));
                    }
                }
                parser
                    .start(start.local_name().as_ref(), namespace, attrs, line)
                    .map_err(failure)?;
            }
            Event::End(_) => parser.end().map_err(failure)?,
            Event::Text(value) => parser.text(&value.xml10_content()).map_err(failure)?,
            Event::CData(value) => parser
                .text(&value.xml_content(XmlVersion::Explicit1_0))
                .map_err(failure)?,
            Event::GeneralRef(value) => {
                if let Some(character) = value
                    .resolve_char_ref()
                    .map_err(|e| failure(e.to_string()))?
                {
                    if !xml_character(character) {
                        return Err(failure("Invalid XML character reference".into()));
                    }
                    parser.text(&character.to_string()).map_err(failure)?;
                } else {
                    let resolved = quick_xml::escape::resolve_xml_entity(&value)
                        .ok_or_else(|| failure("Unknown XML entity reference".into()))?;
                    parser.text(resolved).map_err(failure)?;
                }
            }
            Event::DocType(_) => {
                return Err(failure(
                    "DTD and entity declarations are not supported".into(),
                ))
            }
            Event::Decl(decl) => {
                if declaration || parser.root_seen || position != 0 {
                    return Err(failure("Unexpected XML declaration".into()));
                }
                declaration = true;
                if decl.version().map_err(|e| failure(e.to_string()))?.as_ref() != "1.0" {
                    return Err(failure("Only XML version 1.0 is supported".into()));
                }
            }
            Event::Eof => break,
            Event::Empty(_) => return Err(failure("Unexpected XML parser state".into())),
            Event::Comment(_) | Event::PI(_) => {}
        }
    }
    if !parser.root_seen || !parser.frames.is_empty() {
        return Err(format!("{}: Incomplete XML playlist", path.display()));
    }
    if (kind == Kind::Xspf && parser.track_lists != 1)
        || (kind == Kind::Wpl && (parser.bodies != 1 || parser.sequences != 1))
    {
        return Err(format!(
            "{}: Invalid XML playlist structure",
            path.display()
        ));
    }
    Ok(parser.entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn local_base(segment: &str) -> String {
        format!(
            "{}{segment}/",
            Url::from_directory_path(std::env::temp_dir()).unwrap()
        )
    }
    fn fixture(base: &str, tracks: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bases.xspf");
        std::fs::write(&path, format!(r#"<playlist xmlns="http://xspf.org/ns/0/" version="1" xml:base="{base}"><trackList>{tracks}</trackList></playlist>"#)).unwrap();
        (dir, path)
    }
    #[test]
    fn inherited_base_storage_is_shared_across_many_short_locations() {
        let base = local_base(&"a".repeat(2048));
        let (_dir, path) = fixture(
            &base,
            &"<track><location>one.wav</location></track>".repeat(128),
        );
        let entries = read(&path, Kind::Xspf).unwrap();
        let pointers: std::collections::HashSet<_> = entries
            .iter()
            .flat_map(|entry| entry.locations.as_ref().unwrap())
            .map(|location| match location {
                SourceLocation::Xml { base, .. } => {
                    base.as_ref().as_ref().unwrap().as_str().as_ptr() as usize
                }
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            pointers.len(),
            1,
            "one inherited URI must not be copied once per track"
        );
    }
    #[test]
    fn invalid_inherited_base_diagnostics_do_not_repeat_large_authored_values() {
        let base = format!("https://invalid.example/{}/", "a".repeat(2048));
        let (_dir, path) = fixture(
            &base,
            &"<track><location>one.wav</location></track>".repeat(128),
        );
        let entries = read(&path, Kind::Xspf).unwrap();
        for entry in &entries {
            for location in entry.locations.as_ref().unwrap() {
                if let SourceLocation::Xml { base, .. } = location {
                    assert!(base.as_ref().as_ref().unwrap_err().to_string().len() < 256);
                }
            }
        }
    }
    #[test]
    fn authored_and_joined_base_limits_are_checked_before_retention() {
        let (_dir, path) = fixture(&local_base(&"a".repeat(MAX_LOCATION + 1)), "");
        assert!(read(&path, Kind::Xspf).is_err());
        let parent = local_base(&"a".repeat(33_000));
        let child = format!(
            r#"<track xml:base="{}/"><location>one.wav</location></track>"#,
            "b".repeat(33_000)
        );
        let (_dir, path) = fixture(&parent, &child);
        assert!(read(&path, Kind::Xspf).is_err());
    }
    #[test]
    fn base_budget_charges_distinct_objects_but_not_inherited_or_identical_bases() {
        let base = local_base(&"a".repeat(1024));
        let (_dir, path) = fixture(
            &base,
            &"<track><location>one.wav</location></track>".repeat(128),
        );
        assert_eq!(
            read_with_base_budget(&path, Kind::Xspf, 8192)
                .unwrap()
                .len(),
            128
        );
        let repeated =
            format!(r#"<track xml:base="{base}"><location>one.wav</location></track>"#).repeat(128);
        let (_dir, path) = fixture(&base, &repeated);
        assert_eq!(
            read_with_base_budget(&path, Kind::Xspf, 8192)
                .unwrap()
                .len(),
            128
        );
        let distinct = (0..16)
            .map(|index| {
                format!(r#"<track xml:base="{index}/"><location>one.wav</location></track>"#)
            })
            .collect::<String>();
        let (_dir, path) = fixture(&base, &distinct);
        let error = read_with_base_budget(&path, Kind::Xspf, 8192).unwrap_err();
        assert!(error.contains("retained base URI size limit"), "{error}");
    }
    #[test]
    fn long_inherited_foreign_namespace_has_constant_size_event_classification() {
        let namespace = format!("urn:{}", "x".repeat(8192 - 4));
        let tracks = format!(
            r#"<extension xmlns:f="{namespace}">{}</extension>"#,
            "<f:x/>".repeat(128)
        );
        let (_dir, path) = fixture("", &tracks);
        assert!(read(&path, Kind::Xspf).unwrap().is_empty());
        let text = std::fs::read_to_string(&path).unwrap();
        let mut reader = NsReader::from_str(&text);
        reader.config_mut().expand_empty_elements = true;
        let mut foreign_events = 0;
        let mut largest_classification = 0;
        loop {
            let (resolved, event) = reader.read_resolved_event().unwrap();
            if matches!(&resolved, ResolveResult::Bound(value) if value.as_ref().len() == 8192) {
                foreign_events += 1;
            }
            // Exercise the exact event-loop boundary: its value must carry no
            // owned namespace payload, even for ignored Start and End events.
            let classification = resolve_namespace(resolved).unwrap();
            largest_classification =
                largest_classification.max(std::mem::size_of_val(&classification));
            if matches!(event, Event::Eof) {
                break;
            }
        }
        assert_eq!(foreign_events, 256);
        assert!(largest_classification <= 1, "event classification must not own inherited namespace URI data: {largest_classification} bytes");
    }
}
