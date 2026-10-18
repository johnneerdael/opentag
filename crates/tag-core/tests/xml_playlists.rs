//! Independently authored fixtures following the XSPF and Microsoft WPL/ASX
//! references. These qualify local readers, not original-player execution.
use std::path::{Path, PathBuf};
use tag_core::playlist::{load_playlist, load_playlist_diagnostic, PLAYLIST_EXTENSIONS};

fn file(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"local fixture").unwrap();
    path
}
fn read(root: &Path, extension: &str, text: &str) -> tag_core::playlist::LoadedPlaylist {
    let path = root.join(format!("list.{extension}"));
    std::fs::write(&path, text).unwrap();
    load_playlist(&path, None).unwrap()
}
fn uri(path: &Path) -> String {
    format!(
        "file://{}{}",
        if cfg!(windows) { "/" } else { "" },
        path.to_string_lossy()
            .replace('\\', "/")
            .replace('%', "%25")
            .replace(' ', "%20")
    )
}
#[test]
fn xml_extensions_are_readable_without_changing_legacy_kind_or_writers() {
    for extension in ["xspf", "wpl", "asx"] {
        assert!(PLAYLIST_EXTENSIONS.contains(&extension));
    }
    assert_eq!(
        tag_core::playlist::playlist_kind(Path::new("list.cue")),
        Some(tag_core::playlist::PlaylistKind::Cue)
    );
    assert_eq!(
        tag_core::playlist::playlist_kind(Path::new("list.xspf")),
        None
    );
}
#[test]
fn xspf_locations_are_alternatives_in_track_order_and_foreign_extensions_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let first = file(dir.path(), "first.wav");
    let second = file(dir.path(), "second.flac");
    file(dir.path(), "injected.wav");
    let result = read(
        dir.path(),
        "XSPF",
        r#"<?xml version="1.0"?>
<x:playlist xmlns:x="http://xspf.org/ns/0/" xmlns:other="urn:other" version="1">
<x:extension application="urn:other"><other:track><other:location>injected.wav</other:location></other:track></x:extension>
<x:trackList>
<x:track><x:location>https://invalid.example/remote.wav</x:location><x:location>first.wav</x:location><x:location>second.flac</x:location></x:track>
<x:track><x:location>missing.wav</x:location><x:location>second.flac</x:location></x:track>
<x:track><x:location>first.wav</x:location></x:track>
<x:track><x:title>No location</x:title></x:track>
</x:trackList></x:playlist>"#,
    );
    assert_eq!(result.files, [first, second]);
    assert_eq!(result.missing.len(), 1);
    assert!(result.missing[0].1.contains("location"));
}
#[test]
fn xspf_uri_decoding_is_once_while_wpl_paths_keep_literal_percent_names() {
    let dir = tempfile::tempdir().unwrap();
    let spaced = file(dir.path(), "Song One.wav");
    let percent = file(dir.path(), "Song%20One.wav");
    let unicode = file(dir.path(), "café 猫.wav");
    let xspf = read(
        dir.path(),
        "xspf",
        r#"<playlist xmlns="http://xspf.org/ns/0/" version="1"><trackList>
<track><location>Song%20One.wav</location></track>
<track><location>Song%2520One.wav</location></track>
<track><location>caf%C3%A9%20%E7%8C%AB.wav</location></track>
</trackList></playlist>"#,
    );
    assert_eq!(xspf.files, [spaced, percent.clone(), unicode]);
    assert!(xspf.missing.is_empty());
    let wpl = read(
        dir.path(),
        "wpl",
        r#"<?wpl version="1.0"?><smil><body><seq><media src="Song%20One.wav"/></seq></body></smil>"#,
    );
    assert_eq!(wpl.files, [percent]);
}
#[test]
fn local_xml_base_is_inherited_and_remote_scope_does_not_redirect_to_local_files() {
    let dir = tempfile::tempdir().unwrap();
    let a = file(dir.path(), "Albums/one.wav");
    let b = file(dir.path(), "Shared/two.wav");
    let result = read(
        dir.path(),
        "xspf",
        r#"<playlist xmlns="http://xspf.org/ns/0/" version="1" xml:base="Albums/"><trackList>
<track><location>one.wav</location></track>
<track xml:base="../Shared/"><location>two.wav</location></track>
<track xml:base="https://invalid.example/"><location>../Albums/one.wav</location></track>
</trackList></playlist>"#,
    );
    assert_eq!(result.files, [a, b]);
    assert_eq!(result.missing.len(), 1);
    assert!(result.missing[0].1.contains("local") || result.missing[0].1.contains("scheme"));
}
#[test]
fn wpl_static_order_and_explicit_smart_playlist_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let a = file(dir.path(), "Album/second.wav");
    let b = file(dir.path(), "first.wav");
    file(dir.path(), "injected.wav");
    let result = read(
        dir.path(),
        "wpl",
        r#"<?wpl version="1.0"?><smil><head><meta name="description" content="test"/><extension><media src="injected.wav"/></extension></head><body><seq>
<media src="Album\second.wav"/><media src="first.wav"/>
<smartPlaylist version="1.0.0.0"><querySet><sourceFilter name="Library"/></querySet></smartPlaylist>
</seq></body></smil>"#,
    );
    assert_eq!(result.files, [a, b]);
    assert_eq!(result.missing.len(), 1);
    assert!(result.missing[0].1.contains("smartPlaylist"));
    assert!(
        result.missing[0].1.contains("unsupported")
            || result.missing[0].1.contains("not supported")
    );
}
#[test]
fn asx_ref_alternatives_choose_one_file_and_active_instructions_do_not_recurse() {
    let dir = tempfile::tempdir().unwrap();
    let a = file(dir.path(), "one.wav");
    let b = file(dir.path(), "two.wav");
    std::fs::write(
        dir.path().join("nested.asx"),
        r#"<ASX><ENTRY><REF HREF="injected.wav"/></ENTRY></ASX>"#,
    )
    .unwrap();
    file(dir.path(), "injected.wav");
    let result = read(
        dir.path(),
        "asx",
        r#"<Asx version="3.0">
<Entry><Ref href="missing.wav"/><Ref href="one.wav"/><Ref href="two.wav"/></Entry>
<entry><ref href="two.wav"/></entry>
<ENTRYREF HREF="nested.asx"/><REPEAT COUNT="2"><ENTRY><REF HREF="injected.wav"/></ENTRY></REPEAT>
<EVENT NAME="start"><ENTRY><REF HREF="injected.wav"/></ENTRY></EVENT>
</Asx>"#,
    );
    assert_eq!(result.files, [a, b]);
    assert_eq!(result.missing.len(), 3);
    for instruction in ["ENTRYREF", "REPEAT", "EVENT"] {
        assert!(result
            .missing
            .iter()
            .any(|(_, message)| message.contains(instruction)));
    }
}
#[test]
fn local_file_uris_work_and_remote_authorities_queries_fragments_and_unc_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let audio = file(dir.path(), "café & title.wav");
    let absolute = uri(&audio);
    let local = absolute
        .replacen("file:///", "file://localhost/", 1)
        .replace('&', "&amp;");
    let text = format!(
        r#"<playlist xmlns="http://xspf.org/ns/0/" version="1"><trackList>
<track><location>{local}</location></track>
<track><location>file://remote.invalid/share/song.wav</location></track>
<track><location>https://invalid.example/song.wav</location></track>
<track><location>song.wav?query=1</location></track>
<track><location>song.wav#part</location></track>
<track><location>file:////remote.invalid/share/song.wav</location></track>
</trackList></playlist>"#
    );
    let result = read(dir.path(), "xspf", &text);
    assert_eq!(result.files, [audio]);
    assert_eq!(result.missing.len(), 5);
}
#[test]
fn malformed_wrong_root_or_namespace_xml_and_dtds_are_fatal_instead_of_partial_success() {
    let dir = tempfile::tempdir().unwrap();
    file(dir.path(), "one.wav");
    for (extension, text) in [
        (
            "xspf",
            r#"<playlist xmlns="http://xspf.org/ns/0/" version="1"><trackList><track><location>one.wav</location></track>"#,
        ),
        (
            "xspf",
            r#"<playlist xmlns="urn:wrong" version="1"><trackList/></playlist>"#,
        ),
        (
            "xspf",
            r#"<playlist xmlns="http://xspf.org/ns/0/" version="1"><trackList/><trackList/></playlist>"#,
        ),
        ("wpl", "<not-smil><body><seq/></body></not-smil>"),
        ("asx", "[Reference]\nRef1=one.wav\n"),
        (
            "wpl",
            r#"<!DOCTYPE smil [<!ENTITY steal SYSTEM "file:///private/never-read">]><smil><body><seq><media src="&steal;"/></seq></body></smil>"#,
        ),
        (
            "wpl",
            r#"<smil><body><seq><media src="&unknown;"/></seq></body></smil>"#,
        ),
    ] {
        let path = dir.path().join(format!("invalid.{extension}"));
        std::fs::write(&path, text).unwrap();
        assert!(load_playlist(&path, None).is_err(), "{extension}: {text}");
    }
}
#[test]
fn unicode_boms_and_declared_legacy_encoding_decode_strictly() {
    let dir = tempfile::tempdir().unwrap();
    let audio = file(dir.path(), "café.wav");
    let path = dir.path().join("unicode.wpl");
    let text = r#"<?xml version="1.0" encoding="UTF-16"?><smil><body><seq><media src="café.wav"/></seq></body></smil>"#;
    let bytes: Vec<_> = [0xff, 0xfe]
        .into_iter()
        .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(
        load_playlist(&path, None).unwrap().files.as_slice(),
        std::slice::from_ref(&audio)
    );
    let legacy = b"<?xml version=\"1.0\" encoding=\"windows-1252\"?><smil><body><seq><media src=\"caf\xe9.wav\"/></seq></body></smil>";
    std::fs::write(&path, legacy).unwrap();
    assert_eq!(load_playlist(&path, None).unwrap().files, [audio]);
    for invalid in [
        b"<?xml version=\"1.0\"?><smil><body><seq><media src=\"bad\xff.wav\"/></seq></body></smil>"
            .as_slice(),
        b"<?xml version=\"1.0\" encoding=\"made-up\"?><smil><body><seq/></body></smil>".as_slice(),
    ] {
        std::fs::write(&path, invalid).unwrap();
        assert!(load_playlist(&path, None).is_err());
    }
}
#[test]
fn parser_limits_refuse_deep_and_oversized_xml_before_loading_any_paths() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("limited.wpl");
    let deep = format!(
        "<smil><body><seq>{}{} </seq></body></smil>",
        "<unknown>".repeat(80),
        "</unknown>".repeat(80)
    );
    std::fs::write(&path, deep).unwrap();
    let error = load_playlist(&path, None).unwrap_err();
    assert!(error.to_lowercase().contains("limit") || error.to_lowercase().contains("depth"));
    std::fs::write(&path, vec![b' '; 16 * 1024 * 1024 + 1]).unwrap();
    let error = load_playlist(&path, None).unwrap_err();
    assert!(error.to_lowercase().contains("limit") || error.to_lowercase().contains("large"));
}
#[test]
fn xml_entry_errors_keep_existing_indexed_diagnostics_and_raw_locations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad-locations.wpl");
    std::fs::write(&path, "<smil><body><seq>\n<media src=\"Help {error}.wav\"/>\n<media src=\"https://invalid.example/Title\"/>\n</seq></body></smil>").unwrap();
    let report = load_playlist_diagnostic(&path, None).unwrap();
    assert_eq!(report.result.missing.len(), 2);
    for (index, (_, message)) in report.result.missing.iter().enumerate() {
        assert_eq!(report.error_diagnostics[index].index, index);
        assert!(report.error_diagnostics[index]
            .diagnostic
            .to_string()
            .contains(message));
    }
    assert!(report.result.missing[0].1.contains("Help {error}.wav"));
}

#[test]
fn metadata_processing_instructions_do_not_fetch_or_inject_media() {
    let dir = tempfile::tempdir().unwrap();
    let audio = file(dir.path(), "one.wav");
    let result = read(
        dir.path(),
        "wpl",
        r#"<?xml-stylesheet type="text/xsl" href="https://invalid.example/schema.xsl"?><smil xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:noNamespaceSchemaLocation="https://invalid.example/schema.xsd"><body><seq><media src="one.wav"/></seq></body></smil>"#,
    );
    assert_eq!(result.files, [audio]);
    assert!(result.missing.is_empty());
}
#[test]
fn event_location_and_character_reference_limits_are_controlled_failures() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("limited.wpl");
    for text in [
        format!(
            "<smil><body><seq>{}</seq></body></smil>",
            "<unknown/>".repeat(200_001)
        ),
        format!(
            "<smil><body><seq><media src=\"{}\"/></seq></body></smil>",
            "x".repeat(65_537)
        ),
        "<smil><body><seq><media src=\"bad&#1;.wav\"/></seq></body></smil>".into(),
    ] {
        std::fs::write(&path, text).unwrap();
        let error = load_playlist(&path, None).unwrap_err();
        assert!(
            error.contains("limit") || error.contains("character"),
            "{error}"
        );
    }
}
#[test]
fn xml_suffixes_are_refused_by_the_unchanged_legacy_writers() {
    use tag_core::playlist::{render_playlist, PlaylistWriteOptions};
    for extension in ["xspf", "wpl", "asx"] {
        let error = render_playlist(
            Path::new(&format!("output.{extension}")),
            &[],
            &PlaylistWriteOptions::default(),
        )
        .unwrap_err();
        assert!(error.contains("read-only"));
    }
    assert!(render_playlist(
        Path::new("output.unknown"),
        &[],
        &PlaylistWriteOptions::default()
    )
    .is_ok());
}
#[test]
fn raw_local_absolute_paths_and_native_windows_drive_rules_are_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let audio = file(dir.path(), "Absolute café.wav");
    let escaped = audio
        .to_string_lossy()
        .replace('&', "&amp;")
        .replace('"', "&quot;");
    let text = format!(r#"<smil><body><seq><media src="{escaped}"/></seq></body></smil>"#);
    assert_eq!(read(dir.path(), "wpl", &text).files, [audio]);
    #[cfg(not(windows))]
    {
        let result = read(
            dir.path(),
            "wpl",
            r#"<smil><body><seq><media src="C:\Music\One.wav"/></seq></body></smil>"#,
        );
        assert!(result.files.is_empty());
        assert!(result.missing[0].1.contains("Windows"));
    }
}

#[test]
fn xml_predefined_escapes_decode_once_and_empty_entries_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let literal = file(dir.path(), "A&amp;.wav");
    file(dir.path(), "A&.wav");
    let result = read(
        dir.path(),
        "wpl",
        r#"<smil><body><seq><media src="A&amp;amp;.wav"/><media src=""/></seq></body></smil>"#,
    );
    assert_eq!(result.files, [literal]);
    assert_eq!(result.missing.len(), 1);
    assert!(result.missing[0].1.contains("no media location"));
}
