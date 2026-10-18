#[path = "support/save_corpus.rs"]
mod corpus;

use std::{fs, path::Path};
use tag_core::history::{Journal, Limits};

#[test]
fn common_saves_preserve_audio_opaque_metadata_and_exact_undo() {
    for format in corpus::FORMATS {
        for operation in ["edit-one", "edit-many"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(format!("café 日本.{format}"));
            corpus::seed(&path);
            assert_save_operation(&path, operation);
        }
    }
}

fn assert_save_operation(path: &Path, operation: &str) {
    let format = path.file_name().unwrap().to_string_lossy();
    let before_bytes = fs::read(path).unwrap();
    let before = corpus::snapshot(path);
    for (key, expected) in [
        ("REPLAYGAIN_TRACK_GAIN", "-7.25 dB"),
        ("REPLAYGAIN_TRACK_PEAK", "0.987654"),
        ("REPLAYGAIN_ALBUM_GAIN", "-5.50 dB"),
        ("REPLAYGAIN_ALBUM_PEAK", "0.998877"),
    ] {
        assert_eq!(
            before["fields"][key],
            serde_json::json!([expected]),
            "{format}: {key}"
        );
    }
    assert_eq!(before["opaque_marker_count"], 1, "{format}");
    assert_eq!(before["covers"].as_array().unwrap().len(), 2);
    let mut journal =
        Journal::open(&path.parent().unwrap().join("journal"), Limits::default()).unwrap();
    let capture = journal.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
    corpus::edit(path, operation);
    journal.commit(capture, &[], &[]);
    let after_bytes = fs::read(path).unwrap();
    let after = corpus::snapshot(path);
    let mut expected = before.clone();
    expected["fields"]["TITLE"] = serde_json::json!(["Changed café 日本"]);
    if operation == "edit-many" {
        expected["fields"].as_object_mut().unwrap().remove("ALBUM");
        expected["fields"]["ARTIST"] = serde_json::json!(["Third", "Fourth"]);
    }
    assert_eq!(
        after, expected,
        "{format} {operation}: unrelated data changed"
    );
    assert!(journal.undo().unwrap().errors.is_empty());
    assert_eq!(fs::read(path).unwrap(), before_bytes, "{format}: undo");
    assert_eq!(corpus::snapshot(path), before);
    assert!(journal.redo().unwrap().errors.is_empty());
    assert_eq!(fs::read(path).unwrap(), after_bytes, "{format}: redo");
    let capture = journal.begin("Invalid save", None, &[path.to_string_lossy().into_owned()]);
    assert!(tag_core::write_fields(path, &corpus::fields(&[("TITLE", &["bad\0value"])])).is_err());
    journal.commit(capture, &[], &[]);
    assert_eq!(
        fs::read(path).unwrap(),
        after_bytes,
        "{format}: rejected save"
    );
    assert_eq!(journal.state().undo.unwrap().label, "Save tags");
}

#[test]
fn native_owner_groups_and_payloads_survive_unrelated_edits_and_exact_replay() {
    for operation in ["edit-one", "edit-many"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(corpus::NATIVE_ID3_NAME);
        corpus::seed_native_id3(&path);
        let native = corpus::native_id3::snapshot(&fs::read(&path).unwrap());
        let native = native.as_array().unwrap();
        let profile = corpus::native_profile();
        for rating in profile["popularimeters"].as_array().unwrap() {
            assert!(native.iter().any(|frame| frame["frame"] == "POPM"
                && frame["owner"] == rating["owner"]
                && frame["rating_byte"] == rating["rating"]
                && frame["play_counter"] == rating["counter"]));
        }
        assert_eq!(native.iter().filter(|f| f["frame"] == "POPM").count(), 3);
        for lyrics in profile["lyrics"].as_array().unwrap() {
            assert!(native.iter().any(|frame| frame["frame"] == "USLT"
                && frame["language"] == lyrics["language"]
                && frame["description"] == lyrics["description"]
                && frame["text"] == lyrics["text"]));
        }
        assert_eq!(native.iter().filter(|f| f["frame"] == "USLT").count(), 2);
        assert!(native.iter().any(|frame| frame["frame"] == "UFID"
            && frame["owner"] == profile["unique_file_id"]["owner"]
            && frame["identifier_base64"] == profile["unique_file_id"]["identifier_base64"]));
        for cover in profile["covers"].as_array().unwrap() {
            assert!(native.iter().any(|frame| frame["frame"] == "APIC"
                && frame["description"] == cover["description"]
                && frame["picture_type"] == cover["picture_type"]));
        }
        assert_eq!(native.iter().filter(|f| f["frame"] == "APIC").count(), 2);
        assert_save_operation(&path, operation);
    }
}

#[test]
fn native_loss_controls_expose_counter_owner_language_group_uid_and_cover_changes() {
    for axis in [
        "counter",
        "owner",
        "lyrics-language",
        "lyrics-description",
        "lyrics-group",
        "uid",
        "cover-description",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(corpus::NATIVE_ID3_NAME);
        corpus::seed_native_id3(&path);
        let before = corpus::snapshot(&path);
        let audio = corpus::payload(&path);
        let mut bytes = fs::read(&path).unwrap();
        let frames = corpus::native_id3::frames(&bytes);
        let id = match axis {
            "counter" | "owner" => b"POPM",
            "uid" => b"UFID",
            "cover-description" => b"APIC",
            _ => b"USLT",
        };
        let frame = frames
            .iter()
            .find(|frame| {
                &frame.id == id
                    && (!matches!(axis, "counter" | "owner")
                        || bytes[frame.body.clone()]
                            .starts_with(b"Windows Media Player 9 Series\0"))
            })
            .unwrap();
        let body = frame.body.clone();
        match axis {
            "counter" => bytes[body.end - 1] ^= 1,
            "owner" => bytes[body.start] ^= 32,
            "uid" => bytes[body.end - 1] ^= 1,
            "lyrics-language" => bytes[body.start + 1] = b'f',
            "lyrics-description" => bytes[body.start + 6] ^= 32, // UTF-16 BOM, then first character
            "cover-description" => {
                let mime_end = bytes[body.start + 1..body.end]
                    .iter()
                    .position(|b| *b == 0)
                    .unwrap()
                    + body.start
                    + 1;
                bytes[mime_end + 4] ^= 32; // type, UTF-16 BOM, then first character
            }
            "lyrics-group" => {
                let size = body.end - frame.start;
                let tag_end = 10
                    + bytes[6..10]
                        .iter()
                        .fold(0usize, |n, b| (n << 7) | usize::from(*b));
                bytes.copy_within(body.end..tag_end, frame.start);
                bytes[tag_end - size..tag_end].fill(0);
            }
            _ => unreachable!(),
        }
        fs::write(&path, bytes).unwrap();
        let after = corpus::snapshot(&path);
        assert_ne!(before["native_id3"], after["native_id3"], "{axis}");
        assert_eq!(
            audio,
            corpus::payload(&path),
            "{axis}: control must leave encoded audio untouched"
        );
        if axis == "counter" {
            // Default WMP/MM star projections omit native play counters.
            assert_eq!(before["fields"], after["fields"]);
        }
    }
}

#[test]
fn album_gain_loss_controls_are_detected_on_every_common_format() {
    for format in corpus::FORMATS {
        for key in ["REPLAYGAIN_ALBUM_GAIN", "REPLAYGAIN_ALBUM_PEAK"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(format!("gain.{format}"));
            corpus::seed(&path);
            let before = corpus::snapshot(&path);
            let audio = corpus::payload(&path);
            tag_core::write_fields(&path, &corpus::fields(&[(key, &[])])).unwrap();
            assert_ne!(
                before["fields"],
                corpus::snapshot(&path)["fields"],
                "{format}: {key}"
            );
            assert_eq!(audio, corpus::payload(&path));
        }
    }
}

#[test]
fn competing_tag_policy_exposes_gain_location_and_can_be_undone() {
    let dir = tempfile::tempdir().unwrap();
    for preserve in [false, true] {
        let result =
            corpus::competing_mpeg(&dir.path().join(format!("gain-{preserve}.mp3")), preserve);
        assert_eq!(
            result["displayed_before"]["Title"],
            serde_json::json!(["APE wins display"])
        );
        assert_eq!(result["audio_preserved"], true);
        assert_eq!(result["undo_exact"], true);
        assert_eq!(result["undo_errors"], serde_json::json!([]));
        if preserve {
            assert_eq!(
                result["native_ape_after"]["mp3gain_undo"],
                "Text(\"+001,+001,N\")"
            );
            assert_eq!(
                result["native_ape_after"]["mp3gain_minmax"],
                "Text(\"094,205\")"
            );
            assert_eq!(
                result["native_ape_after"]["opaque_companion"],
                "Binary([1, 0, 255, 17])"
            );
        } else {
            assert!(
                result["native_ape_after"].is_null(),
                "default save removes unwritten APE"
            );
            assert_eq!(
                result["displayed_after"]["MP3GAIN_UNDO"],
                serde_json::json!(["+001,+001,N"]),
                "displayed text surviving in ID3 does not prove native gain-consumer compatibility"
            );
        }
    }
}

#[test]
fn negative_controls_detect_actual_field_cover_opaque_and_audio_loss() {
    for format in corpus::FORMATS {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(format!("control.{format}"));
        corpus::seed(&path);
        let bytes = fs::read(&path).unwrap();
        let before = corpus::snapshot(&path);
        tag_core::write_fields(&path, &corpus::fields(&[("CUSTOM FIELD", &[])])).unwrap();
        assert_ne!(corpus::snapshot(&path)["fields"], before["fields"]);
        fs::write(&path, &bytes).unwrap();
        tag_core::remove_cover(&path, 0).unwrap();
        assert_ne!(corpus::snapshot(&path)["covers"], before["covers"]);
        let mut damaged = bytes.clone();
        let marker = b"TGQA\0\x01\x02\xffopaque";
        let at = damaged
            .windows(marker.len())
            .position(|v| v == marker)
            .unwrap();
        damaged[at + marker.len() - 1] ^= 1;
        fs::write(&path, damaged).unwrap();
        assert_ne!(
            corpus::snapshot(&path)["opaque_marker_count"],
            before["opaque_marker_count"]
        );
        assert_ne!(
            corpus::snapshot(&path)["opaque_native"],
            before["opaque_native"]
        );
        let audio = corpus::payload(&path);
        let mut damaged = bytes;
        let at = damaged
            .windows(audio.len())
            .position(|v| v == audio)
            .unwrap();
        damaged[at + audio.len() - 1] ^= 1;
        fs::write(&path, damaged).unwrap();
        assert_ne!(
            corpus::payload(&path),
            audio,
            "{format}: actual payload corruption must be detected"
        );
    }
}
