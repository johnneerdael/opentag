#[path = "support/save_corpus.rs"]
mod corpus;

use std::fs;
use tag_core::history::{Journal, Limits};

#[test]
fn common_saves_preserve_audio_opaque_metadata_and_exact_undo() {
    for format in corpus::FORMATS {
        for operation in ["edit-one", "edit-many"] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(format!("café 日本.{format}"));
            corpus::seed(&path);
            let before_bytes = fs::read(&path).unwrap();
            let before = corpus::snapshot(&path);
            assert_eq!(before["opaque_marker_count"], 1, "{format}");
            assert_eq!(before["covers"].as_array().unwrap().len(), 2);
            let mut journal =
                Journal::open(&dir.path().join("journal"), Limits::default()).unwrap();
            let capture = journal.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
            corpus::edit(&path, operation);
            journal.commit(capture, &[], &[]);
            let after_bytes = fs::read(&path).unwrap();
            let after = corpus::snapshot(&path);
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
            assert_eq!(fs::read(&path).unwrap(), before_bytes, "{format}: undo");
            assert_eq!(corpus::snapshot(&path), before);
            assert!(journal.redo().unwrap().errors.is_empty());
            assert_eq!(fs::read(&path).unwrap(), after_bytes, "{format}: redo");
            let capture =
                journal.begin("Invalid save", None, &[path.to_string_lossy().into_owned()]);
            assert!(
                tag_core::write_fields(&path, &corpus::fields(&[("TITLE", &["bad\0value"])]))
                    .is_err()
            );
            journal.commit(capture, &[], &[]);
            assert_eq!(
                fs::read(&path).unwrap(),
                after_bytes,
                "{format}: rejected save"
            );
            assert_eq!(journal.state().undo.unwrap().label, "Save tags");
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
