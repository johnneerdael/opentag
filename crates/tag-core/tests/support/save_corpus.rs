//! Independent retained carriers shared by the regression suite and paired harness.
pub mod native_id3;
use base64::{engine::general_purpose::STANDARD, Engine};
use lofty::{
    config::{ParseOptions, WriteOptions},
    file::AudioFile,
    id3::v2::{BinaryFrame, Frame, FrameId},
    mp4::{Atom, AtomData, AtomIdent, DataType},
};
use serde_json::{json, Value};
use std::{fs, path::Path};
use tag_core::{read_entry, write_fields, Fields, TagOptions};

pub const FORMATS: [&str; 3] = ["mp3", "flac", "m4a"];
pub const NATIVE_ID3_NAME: &str = "native-groups.mp3";
const OPAQUE: &[u8] = b"TGQA\0\x01\x02\xffopaque";

pub fn fields(items: &[(&str, &[&str])]) -> Fields {
    items
        .iter()
        .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
        .collect()
}

pub fn seed(path: &Path) {
    let ext = path.extension().unwrap().to_str().unwrap();
    let carrier: &[u8] = match ext {
        "mp3" => include_bytes!("../fixtures/save-corpus/tone.mp3"),
        "flac" => include_bytes!("../fixtures/save-corpus/tone.flac"),
        "m4a" => include_bytes!("../fixtures/save-corpus/tone.m4a"),
        _ => panic!("unsupported corpus extension"),
    };
    fs::write(path, carrier).unwrap();
    write_fields(
        path,
        &fields(&[
            ("TITLE", &["Original café 日本"]),
            ("ARTIST", &["First", "Second"]),
            ("ALBUM", &["Preserve me"]),
            ("CUSTOM FIELD", &["one", "two"]),
            ("UNSYNCEDLYRICS", &["eng||Line one\nLínea dos 日本"]),
            ("RATING WMP", &["4"]),
            ("REPLAYGAIN_TRACK_GAIN", &["-7.25 dB"]),
            ("REPLAYGAIN_TRACK_PEAK", &["0.987654"]),
            ("REPLAYGAIN_ALBUM_GAIN", &["-5.50 dB"]),
            ("REPLAYGAIN_ALBUM_PEAK", &["0.998877"]),
        ]),
    )
    .unwrap();
    // Distinct images expose cover loss, duplication and reordering.
    for (color, kind) in [([255, 0, 0], "CoverFront"), ([0, 0, 255], "CoverBack")] {
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbImage::from_pixel(2, 2, image::Rgb(color))
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        tag_core::add_cover_bytes_with_options(
            path,
            png.into_inner(),
            "corpus cover",
            Some(kind),
            &TagOptions::default(),
        )
        .unwrap();
    }
    match ext {
        "mp3" => {
            let mut file = lofty::mpeg::MpegFile::read_from(
                &mut fs::File::open(path).unwrap(),
                ParseOptions::new(),
            )
            .unwrap();
            let mut tag = file.id3v2().cloned().unwrap();
            tag.insert(Frame::Binary(BinaryFrame::new(
                FrameId::new("XABC").unwrap(),
                OPAQUE.to_vec(),
            )));
            file.set_id3v2(tag);
            file.save_to_path(path, WriteOptions::default().use_id3v23(true))
                .unwrap();
        }
        "flac" => {
            // An APPLICATION block belongs to the container, outside Vorbis comments.
            let mut bytes = fs::read(path).unwrap();
            let mut block = vec![2, 0, 0, OPAQUE.len() as u8];
            block.extend(OPAQUE);
            assert_eq!(bytes[4] & 0x7f, 0, "STREAMINFO must remain first");
            bytes.splice(42..42, block);
            fs::write(path, bytes).unwrap();
        }
        "m4a" => {
            let mut file = lofty::mp4::Mp4File::read_from(
                &mut fs::File::open(path).unwrap(),
                ParseOptions::new(),
            )
            .unwrap();
            let mut tag = file.ilst().cloned().unwrap();
            tag.insert(Atom::new(
                AtomIdent::Fourcc(*b"xQA!"),
                AtomData::Unknown {
                    code: DataType::Reserved,
                    data: OPAQUE.to_vec(),
                },
            ));
            file.set_ilst(tag);
            file.save_to_path(path, WriteOptions::default()).unwrap();
        }
        _ => unreachable!(),
    }
}

pub fn native_profile() -> Value {
    serde_json::from_str(include_str!(
        "../fixtures/save-corpus/native-id3-profile.json"
    ))
    .unwrap()
}

pub fn seed_native_id3(path: &Path) {
    use lofty::{
        id3::v2::{
            AttachedPictureFrame, PopularimeterFrame, UniqueFileIdentifierFrame,
            UnsynchronizedTextFrame,
        },
        picture::PictureType,
        TextEncoding,
    };
    seed(path);
    let profile = native_profile();
    let mut file =
        lofty::mpeg::MpegFile::read_from(&mut fs::File::open(path).unwrap(), ParseOptions::new())
            .unwrap();
    let mut tag = file.id3v2().cloned().unwrap();
    let pictures = (&tag)
        .into_iter()
        .filter_map(|frame| {
            if let Frame::Picture(p) = frame {
                Some(p.picture.clone().into_owned())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    tag.retain(|frame| {
        !matches!(
            frame,
            Frame::Picture(_)
                | Frame::Popularimeter(_)
                | Frame::UnsynchronizedText(_)
                | Frame::UniqueFileIdentifier(_)
        )
    });
    for rating in profile["popularimeters"].as_array().unwrap() {
        tag.insert(Frame::Popularimeter(PopularimeterFrame::new(
            rating["owner"].as_str().unwrap().to_owned(),
            rating["rating"].as_u64().unwrap() as u8,
            rating["counter"].as_u64().unwrap(),
        )));
    }
    for lyrics in profile["lyrics"].as_array().unwrap() {
        tag.insert(Frame::UnsynchronizedText(UnsynchronizedTextFrame::new(
            TextEncoding::UTF16,
            lyrics["language"]
                .as_str()
                .unwrap()
                .as_bytes()
                .try_into()
                .unwrap(),
            lyrics["description"].as_str().unwrap().to_owned(),
            lyrics["text"].as_str().unwrap().to_owned(),
        )));
    }
    let uid = &profile["unique_file_id"];
    tag.insert(Frame::UniqueFileIdentifier(UniqueFileIdentifierFrame::new(
        uid["owner"].as_str().unwrap().to_owned(),
        STANDARD
            .decode(uid["identifier_base64"].as_str().unwrap())
            .unwrap(),
    )));
    for cover in profile["covers"].as_array().unwrap() {
        let mut picture = pictures[cover["image_index"].as_u64().unwrap() as usize].clone();
        picture.set_pic_type(PictureType::from_u8(
            cover["picture_type"].as_u64().unwrap() as u8,
        ));
        picture.set_description(Some(cover["description"].as_str().unwrap().to_owned()));
        tag.insert(Frame::Picture(AttachedPictureFrame::new(
            TextEncoding::UTF16,
            picture,
        )));
    }
    file.set_id3v2(tag);
    file.save_to_path(path, WriteOptions::default().use_id3v23(true))
        .unwrap();
}

/// Encoded audio payload, excluding tags/container headers. Fail on malformed carriers.
pub fn payload(path: &Path) -> Vec<u8> {
    let b = fs::read(path).unwrap();
    match path
        .extension()
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase()
        .as_str()
    {
        "mp3" => {
            let start = if b.starts_with(b"ID3") {
                assert!(b[6..10].iter().all(|v| v & 0x80 == 0));
                10 + b[6..10]
                    .iter()
                    .fold(0usize, |n, v| (n << 7) | usize::from(*v))
            } else {
                0
            };
            let mut end = b.len();
            if end >= 128 && &b[end - 128..end - 125] == b"TAG" {
                end -= 128;
            }
            if end >= 32 && &b[end - 32..end - 24] == b"APETAGEX" {
                let size = u32::from_le_bytes(b[end - 20..end - 16].try_into().unwrap()) as usize;
                end -= size;
                if end >= 32 && &b[end - 32..end - 24] == b"APETAGEX" {
                    end -= 32;
                }
            }
            assert!(start < end);
            b[start..end].to_vec()
        }
        "flac" => {
            assert!(b.starts_with(b"fLaC"));
            let mut at = 4;
            loop {
                let last = b[at] & 0x80 != 0;
                let size =
                    ((b[at + 1] as usize) << 16) | ((b[at + 2] as usize) << 8) | b[at + 3] as usize;
                at += 4 + size;
                assert!(at < b.len());
                if last {
                    break;
                }
            }
            b[at..].to_vec()
        }
        "m4a" => {
            let mut at = 0;
            let mut payload = Vec::new();
            while at < b.len() {
                let n = u32::from_be_bytes(b[at..at + 4].try_into().unwrap()) as usize;
                assert!(
                    n >= 8 && at + n <= b.len(),
                    "corpus requires ordinary 32-bit MP4 atoms"
                );
                if &b[at + 4..at + 8] == b"mdat" {
                    payload.extend(&b[at + 8..at + n]);
                }
                at += n;
            }
            assert!(!payload.is_empty());
            payload
        }
        _ => panic!("unsupported corpus extension"),
    }
}

pub fn snapshot(path: &Path) -> Value {
    let entry = read_entry(path).unwrap();
    let bytes = fs::read(path).unwrap();
    let opaque_native = match path
        .extension()
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase()
        .as_str()
    {
        "mp3" => {
            let file = lofty::mpeg::MpegFile::read_from(
                &mut fs::File::open(path).unwrap(),
                ParseOptions::new(),
            )
            .unwrap();
            let id3 = file
                .id3v2()
                .map(|tag| {
                    tag.into_iter()
                        .filter(|frame| matches!(frame, Frame::Binary(_)))
                        .map(|frame| format!("{frame:?}"))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let ape = file
                .ape()
                .map(|tag| {
                    tag.into_iter()
                        .filter(|item| matches!(item.value(), lofty::tag::ItemValue::Binary(_)))
                        .map(|item| format!("{item:?}"))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json!({"id3_binary_frames": id3, "ape_binary_items": ape})
        }
        "flac" => {
            let mut at = 4;
            let mut blocks = Vec::new();
            loop {
                let last = bytes[at] & 0x80 != 0;
                let size = ((bytes[at + 1] as usize) << 16)
                    | ((bytes[at + 2] as usize) << 8)
                    | bytes[at + 3] as usize;
                let kind = bytes[at] & 0x7f;
                if !matches!(kind, 0 | 1 | 4 | 6) {
                    blocks.push(json!({"kind": kind, "data_base64": STANDARD.encode(&bytes[at + 4..at + 4 + size])}));
                }
                at += 4 + size;
                if last {
                    break;
                }
            }
            json!(blocks)
        }
        "m4a" => {
            let file = lofty::mp4::Mp4File::read_from(
                &mut fs::File::open(path).unwrap(),
                ParseOptions::new(),
            )
            .unwrap();
            json!(file
                .ilst()
                .map(|tag| tag
                    .into_iter()
                    .filter(|atom| atom
                        .data()
                        .any(|value| matches!(value, AtomData::Unknown { .. })))
                    .map(|atom| format!("{atom:?}"))
                    .collect::<Vec<_>>())
                .unwrap_or_default())
        }
        _ => unreachable!(),
    };
    json!({
        "fields": entry.fields, "covers": entry.covers.iter().map(|c| json!({
            "mime": c.mime, "description": c.description, "picture_type": c.picture_type,
            "data_base64": c.data_base64,
        })).collect::<Vec<_>>(),
        "opaque_marker_count": bytes.windows(OPAQUE.len()).filter(|v| *v == OPAQUE).count(),
        "opaque_native": opaque_native,
        "native_id3": if path.extension().unwrap().eq_ignore_ascii_case("mp3") { native_id3::snapshot(&bytes) } else { Value::Null },
        "audio_payload_base64": STANDARD.encode(payload(path)),
        "tag_kinds": entry.technical.get("_TAG"),
    })
}

/// Exercise the explicitly configured MPEG policy, including gain data's native location.
pub fn competing_mpeg(path: &Path, preserve_ape: bool) -> Value {
    use lofty::{
        ape::{ApeItem, ApeTag},
        prelude::TagExt,
        tag::ItemValue,
    };
    seed(path);
    let mut ape = ApeTag::new();
    for (name, value) in [
        ("Title", "APE wins display"),
        ("MP3GAIN_UNDO", "+001,+001,N"),
        ("MP3GAIN_MINMAX", "094,205"),
        ("REPLAYGAIN_TRACK_GAIN", "-3.00 dB"),
    ] {
        ape.insert(ApeItem::new(name.into(), ItemValue::Text(value.into())).unwrap());
    }
    ape.insert(
        ApeItem::new(
            "Opaque gain companion".into(),
            ItemValue::Binary(vec![1, 0, 255, 17]),
        )
        .unwrap(),
    );
    ape.save_to_path(path, WriteOptions::default()).unwrap();
    let before_bytes = fs::read(path).unwrap();
    let before_payload = payload(path);
    let displayed_before = read_entry(path).unwrap().fields;
    let temporary = tempfile::tempdir().unwrap();
    let mut journal = tag_core::history::Journal::open(
        &temporary.path().join("journal"),
        tag_core::history::Limits::default(),
    )
    .unwrap();
    let capture = journal.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
    let mut options = TagOptions::default();
    options.mpeg.write_ape = preserve_ape;
    tag_core::write_fields_with_options(
        path,
        &fields(&[("TITLE", &["Changed café 日本"])]),
        false,
        &[],
        &options,
    )
    .unwrap();
    journal.commit(capture, &[], &[]);
    let file =
        lofty::mpeg::MpegFile::read_from(&mut fs::File::open(path).unwrap(), ParseOptions::new())
            .unwrap();
    let native_ape = file.ape().map(|tag| json!({
        "mp3gain_undo": tag.get("MP3GAIN_UNDO").map(|item| format!("{:?}", item.value())),
        "mp3gain_minmax": tag.get("MP3GAIN_MINMAX").map(|item| format!("{:?}", item.value())),
        "opaque_companion": tag.get("Opaque gain companion").map(|item| format!("{:?}", item.value())),
    }));
    let audio_preserved = payload(path) == before_payload;
    let displayed_after = read_entry(path).unwrap().fields;
    let undo_errors = journal.undo().unwrap().errors;
    json!({"options": options, "displayed_before": displayed_before, "displayed_after": displayed_after,
        "native_ape_after": native_ape, "audio_preserved": audio_preserved,
        "undo_errors": undo_errors, "undo_exact": fs::read(path).unwrap() == before_bytes,
        "gain_consumer_runtime": "not executed; native APE location inspected only"})
}

pub fn edit(path: &Path, operation: &str) {
    let updates = match operation {
        "edit-one" => fields(&[("TITLE", &["Changed café 日本"])]),
        "edit-many" => fields(&[
            ("TITLE", &["Changed café 日本"]),
            ("ALBUM", &[]),
            ("ARTIST", &["Third", "Fourth"]),
        ]),
        _ => panic!("unknown operation"),
    };
    write_fields(path, &updates).unwrap();
}
