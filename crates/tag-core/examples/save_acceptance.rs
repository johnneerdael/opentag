//! Backend producer/consumer for scripts/save-acceptance.py. Operate on copies only.
#[path = "../tests/support/save_corpus.rs"]
mod corpus;

use serde_json::json;
use std::{env, fs, path::Path};
use tag_core::history::{Journal, Limits};

fn main() {
    let args: Vec<_> = env::args().collect();
    assert!(
        args.len() >= 3,
        "prepare DIRECTORY | observe FILE | exercise FILE OPERATION"
    );
    let path = Path::new(&args[2]);
    match args[1].as_str() {
        "defaults" => println!("{}", json!(tag_core::TagOptions::default())),
        "prepare" => {
            fs::create_dir(path).expect("destination must not exist");
            for format in corpus::FORMATS {
                corpus::seed(&path.join(format!("café 日本.{format}")));
            }
            println!(
                "{}",
                json!({"taggroove_defaults": tag_core::TagOptions::default()})
            );
        }
        "observe" => println!("{}", corpus::snapshot(path)),
        "policy" => {
            fs::create_dir(path).expect("destination must not exist");
            println!(
                "{}",
                json!({
                    "default": corpus::competing_mpeg(&path.join("default.mp3"), false),
                    "explicit_write_ape": corpus::competing_mpeg(&path.join("write-ape.mp3"), true),
                })
            );
        }
        "exercise" => {
            assert!((4..=5).contains(&args.len()));
            let retain = |stage: &str| {
                if let Some(directory) = args.get(4) {
                    let stage = Path::new(directory).join(stage);
                    fs::create_dir_all(&stage).unwrap();
                    let destination = stage.join(path.file_name().unwrap());
                    assert!(!destination.exists(), "capture must not exist");
                    fs::copy(path, destination).unwrap();
                }
            };
            let temporary = tempfile::tempdir().unwrap();
            let mut journal =
                Journal::open(&temporary.path().join("journal"), Limits::default()).unwrap();
            let capture = journal.begin("Save tags", None, &[path.to_string_lossy().into_owned()]);
            corpus::edit(path, &args[3]);
            journal.commit(capture, &[], &[]);
            let saved = corpus::snapshot(path);
            retain("saved");
            let save_bytes = fs::read(path).unwrap();
            let undo_errors = journal.undo().unwrap().errors;
            let undone = corpus::snapshot(path);
            retain("undone");
            let redo_errors = journal.redo().unwrap().errors;
            let redone = corpus::snapshot(path);
            retain("redone");
            let rejected =
                tag_core::write_fields(path, &corpus::fields(&[("TITLE", &["bad\0value"])]))
                    .is_err();
            let rejection_unchanged = save_bytes == fs::read(path).unwrap();
            println!(
                "{}",
                json!({"saved": saved, "undone": undone, "redone": redone,
                "undo_errors": undo_errors, "redo_errors": redo_errors,
                "invalid_write_rejected": rejected, "rejection_unchanged": rejection_unchanged})
            );
        }
        _ => panic!("unknown command"),
    }
}
