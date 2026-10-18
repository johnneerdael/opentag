//! Narrow, independent raw-envelope observation for ordinary unflagged ID3v2.3/2.4.
//! Do not use TagGroove's field mapper or Lofty's frame decoder for these checks.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::ops::Range;

pub struct RawFrame {
    pub id: [u8; 4],
    pub start: usize,
    pub body: Range<usize>,
    pub flags: [u8; 2],
}

fn syncsafe(data: &[u8]) -> usize {
    assert!(data.iter().all(|b| b & 0x80 == 0));
    data.iter().fold(0usize, |n, b| (n << 7) | usize::from(*b))
}

pub fn frames(data: &[u8]) -> Vec<RawFrame> {
    if !data.starts_with(b"ID3") {
        return Vec::new();
    }
    assert!(data.len() >= 10 && matches!(data[3], 3 | 4));
    assert_eq!(
        data[5], 0,
        "raw observation requires ordinary unflagged ID3"
    );
    let end = 10 + syncsafe(&data[6..10]);
    assert!(end <= data.len());
    let mut at = 10;
    let mut frames = Vec::new();
    while at + 10 <= end && data[at] != 0 {
        let size = if data[3] == 3 {
            u32::from_be_bytes(data[at + 4..at + 8].try_into().unwrap()) as usize
        } else {
            syncsafe(&data[at + 4..at + 8])
        };
        assert!(size > 0 && at + 10 + size <= end);
        frames.push(RawFrame {
            id: data[at..at + 4].try_into().unwrap(),
            start: at,
            body: at + 10..at + 10 + size,
            flags: data[at + 8..at + 10].try_into().unwrap(),
        });
        at += 10 + size;
    }
    assert!(
        data[at..end].iter().all(|b| *b == 0),
        "unexpected non-padding bytes after ID3 frames"
    );
    frames
}

fn latin1(data: &[u8]) -> String {
    data.iter().map(|b| char::from(*b)).collect()
}

fn text(data: &[u8], encoding: u8) -> String {
    if data.is_empty() {
        return String::new();
    }
    match encoding {
        0 => latin1(data),
        3 => String::from_utf8(data.to_vec()).unwrap(),
        1 | 2 => {
            let (little, data) = if data.starts_with(&[255, 254]) {
                (true, &data[2..])
            } else if data.starts_with(&[254, 255]) {
                (false, &data[2..])
            } else {
                assert_eq!(encoding, 2, "UTF-16 corpus strings require their BOM");
                (false, data)
            };
            assert_eq!(data.len() % 2, 0);
            let units = data
                .chunks_exact(2)
                .map(|b| {
                    if little {
                        u16::from_le_bytes([b[0], b[1]])
                    } else {
                        u16::from_be_bytes([b[0], b[1]])
                    }
                })
                .collect::<Vec<_>>();
            String::from_utf16(&units).unwrap()
        }
        _ => panic!("unsupported text encoding"),
    }
}

fn terminated(data: &[u8], encoding: u8) -> (String, usize) {
    let step = if matches!(encoding, 1 | 2) { 2 } else { 1 };
    let end = (0..data.len())
        .step_by(step)
        .find(|at| {
            data.get(*at..*at + step)
                .is_some_and(|b| b.iter().all(|v| *v == 0))
        })
        .unwrap();
    (text(&data[..end], encoding), end + step)
}

pub fn snapshot(data: &[u8]) -> Value {
    let mut result = Vec::new();
    for frame in frames(data) {
        if !matches!(&frame.id, b"POPM" | b"USLT" | b"UFID" | b"APIC") {
            continue;
        }
        debug_assert_eq!(frame.start + 10, frame.body.start);
        assert_eq!(
            frame.flags,
            [0, 0],
            "native profile does not qualify compressed/encrypted frames"
        );
        let body = &data[frame.body];
        let mut value = json!({"frame": latin1(&frame.id), "flags": frame.flags, "raw_body_base64": STANDARD.encode(body)});
        match &frame.id {
            b"POPM" => {
                let end = body.iter().position(|b| *b == 0).unwrap();
                value["owner"] = json!(latin1(&body[..end]));
                value["rating_byte"] = json!(body[end + 1]);
                let counter = &body[end + 2..];
                assert!(!counter.is_empty() && counter.len() <= 8);
                value["play_counter"] =
                    json!(counter.iter().fold(0u64, |n, b| (n << 8) | u64::from(*b)));
            }
            b"UFID" => {
                let end = body.iter().position(|b| *b == 0).unwrap();
                value["owner"] = json!(latin1(&body[..end]));
                value["identifier_base64"] = json!(STANDARD.encode(&body[end + 1..]));
            }
            b"USLT" => {
                value["language"] = json!(latin1(&body[1..4]));
                let (description, used) = terminated(&body[4..], body[0]);
                value["description"] = json!(description);
                value["text"] = json!(text(&body[4 + used..], body[0]));
            }
            b"APIC" => {
                let mime_end = body[1..].iter().position(|b| *b == 0).unwrap() + 1;
                value["mime"] = json!(latin1(&body[1..mime_end]));
                value["picture_type"] = json!(body[mime_end + 1]);
                let (description, used) = terminated(&body[mime_end + 2..], body[0]);
                value["description"] = json!(description);
                value["image_base64"] = json!(STANDARD.encode(&body[mime_end + 2 + used..]));
            }
            _ => unreachable!(),
        }
        result.push(value);
    }
    // Physical frame ordering is not a save-preservation requirement. Keep every
    // occurrence, including duplicates, while comparing native group/payload identity.
    result.sort_by_key(Value::to_string);
    json!(result)
}
