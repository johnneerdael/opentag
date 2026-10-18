//! Validate the converter guide's published examples with TagGroove's core.
mod guide_examples;
use guide_examples::text_blocks;

use tag_core::{
    evaluate, evaluate_filename_with_context,
    numbering::{plan_numbering, NumberingItem, NumberingOptions},
    CompiledFilenamePattern, EvalContext, Fields, FilenameConverter, TextImportOptions,
    TextTagConverter,
};

const TAG_FILENAME: &str = include_str!("../../../docs/user-guide/converters/tag-to-filename.md");
const FILENAME_TAG: &str = include_str!("../../../docs/user-guide/converters/filename-to-tag.md");
const FILENAME_FILENAME: &str =
    include_str!("../../../docs/user-guide/converters/filename-to-filename.md");
const TAG_TAG: &str = include_str!("../../../docs/user-guide/converters/tag-to-tag.md");
const TEXT_TAG: &str = include_str!("../../../docs/user-guide/converters/text-file-to-tag.md");
const NUMBERING: &str = include_str!("../../../docs/user-guide/converters/auto-numbering.md");

fn cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

fn inline(cell: &str) -> &str {
    cell.strip_prefix('`').unwrap().split('`').next().unwrap()
}

#[test]
fn tag_filename_table_evaluates_to_the_written_names() {
    let fields = [
        ("ARTIST".into(), vec!["Juniper Vale".into()]),
        ("ALBUM".into(), vec!["Harbour Lights".into()]),
        ("YEAR".into(), vec!["2024".into()]),
        ("TRACK".into(), vec!["3/8".into()]),
        ("TITLE".into(), vec!["Paper Boats".into()]),
    ]
    .into();
    let mut count = 0;
    for line in TAG_FILENAME.lines().filter(|line| line.starts_with("| `")) {
        let row = cells(line);
        if row.len() != 2 || !row[1].starts_with('`') {
            continue;
        }
        let name =
            evaluate_filename_with_context(inline(row[0]), &EvalContext::new(&fields)).unwrap();
        assert_eq!(format!("{name}.mp3"), inline(row[1]), "{line}");
        count += 1;
    }
    assert_eq!(count, 4);
}

#[test]
fn filename_tag_table_matches_complete_names_and_preserves_nonmatches() {
    let mut count = 0;
    for line in FILENAME_TAG.lines().filter(|line| line.starts_with("| `")) {
        let row = cells(line);
        let result = CompiledFilenamePattern::compile(inline(row[1]))
            .unwrap()
            .try_parse(inline(row[0]))
            .unwrap();
        if row[2] == "No match" {
            assert!(result.is_none(), "{line}");
        } else {
            let result = result.unwrap();
            let parts: Vec<_> = row[2].split('`').collect();
            for pair in parts[1..].chunks(4) {
                assert_eq!(result[pair[0]], [pair[2]], "{line}");
            }
            assert!(!result.contains_key("DUMMY"));
        }
        count += 1;
    }
    assert_eq!(count, 4);
    let blocks = text_blocks(FILENAME_TAG);
    let pattern = blocks
        .iter()
        .find(|block| block.contains("%artist%\\%album%"))
        .unwrap();
    let result = CompiledFilenamePattern::compile(pattern)
        .unwrap()
        .parse("Music/Juniper Vale/Harbour Lights (2024)/03 - Paper Boats.mp3")
        .unwrap();
    assert_eq!(result["ARTIST"], ["Juniper Vale"]);
    assert_eq!(result["ALBUM"], ["Harbour Lights"]);
    assert_eq!(result["YEAR"], ["2024"]);
}

#[test]
fn filename_rearrangement_table_runs_its_written_old_and_new_patterns() {
    let fields = Fields::new();
    let mut count = 0;
    for line in FILENAME_FILENAME
        .lines()
        .filter(|line| line.starts_with("| `"))
    {
        let row = cells(line);
        let result = FilenameConverter::compile(inline(row[1]), inline(row[2]))
            .unwrap()
            .render(inline(row[0]), &EvalContext::new(&fields))
            .unwrap();
        assert_eq!(result.target, inline(row[3]), "{line}");
        count += 1;
    }
    assert_eq!(count, 8);
}

#[test]
fn advanced_rearrangement_walkthrough_combines_the_written_patterns_and_folders() {
    let examples = text_blocks(FILENAME_FILENAME);
    assert_eq!(examples.len(), 4);
    let fields = Fields::new();
    let result = FilenameConverter::compile(&examples[1], &examples[2])
        .unwrap()
        .render(&examples[0], &EvalContext::new(&fields))
        .unwrap();
    assert_eq!(result.target, examples[3]);
    assert_eq!(result.captures["1"], ["Juniper Vale"]);
    assert_eq!(result.captures["5"], ["2024"]);
}

#[test]
fn tag_to_tag_table_calculates_the_published_results() {
    let mut count = 0;
    for line in TAG_TAG
        .lines()
        .filter(|line| line.starts_with("| `") && cells(line).len() == 4)
    {
        let row = cells(line);
        let mut fields = Fields::new();
        if row[2].starts_with('`') {
            let input: Vec<_> = row[2].split('`').collect();
            fields.insert(input[1].into(), vec![input[3].into()]);
        }
        assert_eq!(
            evaluate(inline(row[1]), &fields).unwrap(),
            inline(row[3]),
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 5);
}

#[test]
fn text_import_examples_match_by_order_and_by_filename_key() {
    let blocks = text_blocks(TEXT_TAG);
    let selected = vec!["/Music/03.mp3".into(), "/Music/04.mp3".into()];
    let result = TextTagConverter::compile(&blocks[1])
        .unwrap()
        .plan(&blocks[0], &selected, &TextImportOptions::default())
        .unwrap();
    assert_eq!(result[0].fields["TITLE"], ["Paper Boats"]);
    assert_eq!(result[0].fields["TRACK"], ["3"]);
    assert_eq!(result[1].fields["TITLE"], ["North Pier"]);
    let records = blocks
        .iter()
        .find(|block| block.starts_with("04.mp3;"))
        .unwrap();
    let mask = blocks
        .iter()
        .find(|block| block.starts_with("%_filename_ext%;"))
        .unwrap();
    let result = TextTagConverter::compile(mask)
        .unwrap()
        .plan(records, &selected, &TextImportOptions::default())
        .unwrap();
    assert_eq!(result[0].path, "/Music/04.mp3");
    assert_eq!(result[0].fields["TITLE"], ["North Pier"]);
    assert_eq!(result[1].path, "/Music/03.mp3");
}

#[test]
fn numbering_table_uses_the_written_start_width_and_total() {
    let items: Vec<_> = (1..=3)
        .map(|number| NumberingItem {
            path: format!("/Music/{number}.mp3"),
            fields: Fields::new(),
        })
        .collect();
    let mut count = 0;
    for line in NUMBERING
        .lines()
        .filter(|line| line.starts_with("| 1 |") || line.starts_with("| 5 |"))
    {
        let row = cells(line);
        let options = NumberingOptions {
            track_start: row[0].parse().unwrap(),
            track_digits: row[1].parse().unwrap(),
            track_total: row[2]
                .strip_prefix("On, ")
                .map(|total| total.parse().unwrap()),
            ..Default::default()
        };
        let result = plan_numbering(&items, &options).unwrap();
        let outputs: Vec<_> = row[3].split('`').skip(1).step_by(2).collect();
        assert_eq!(outputs.len(), 3);
        for (item, expected) in result.iter().zip(outputs) {
            assert_eq!(item.fields["TRACK"], [expected], "{line}");
        }
        count += 1;
    }
    assert_eq!(count, 3);
}

#[test]
fn partial_keyed_import_leaves_unlisted_selection_and_skips_other_records() {
    let section = TEXT_TAG
        .split("<!-- partial-keyed-example:start -->")
        .nth(1)
        .unwrap()
        .split("<!-- partial-keyed-example:end -->")
        .next()
        .unwrap();
    let records = text_blocks(section).remove(0);
    let mask = text_blocks(TEXT_TAG)
        .into_iter()
        .find(|block| block.starts_with("%_filename_ext%;"))
        .unwrap();
    let selected = vec!["/Music/03.mp3".into(), "/Music/05.mp3".into()];
    let plan = TextTagConverter::compile(&mask)
        .unwrap()
        .plan(&records, &selected, &TextImportOptions::default())
        .unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].path, selected[0]);
    assert_eq!(plan[0].fields["TITLE"], ["Paper Boats"]);
    assert_eq!(plan[0].fields["TRACK"], ["3"]);
}

#[test]
fn per_folder_shortcut_examples_use_each_runs_last_assigned_number() {
    let table = NUMBERING
        .split("<!-- per-folder-numbering-examples:start -->")
        .nth(1)
        .unwrap()
        .split("<!-- per-folder-numbering-examples:end -->")
        .next()
        .unwrap();
    let examples: Vec<_> = table
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(cells)
        .collect();
    assert_eq!(examples.len(), 3);
    for group in [&examples[..2], &examples[2..]] {
        let items: Vec<_> = group
            .iter()
            .flat_map(|row| {
                (1..=inline(row[2]).parse::<usize>().unwrap()).map(|number| NumberingItem {
                    path: format!("/Music/{}/{number}.mp3", inline(row[1])),
                    fields: Fields::new(),
                })
            })
            .collect();
        let options = NumberingOptions {
            track_start: inline(group[0][0]).parse().unwrap(),
            track_digits: 2,
            count_track_total: true,
            next_disc_on_directory: true,
            reset_track_on_next_disc: true,
            ..Default::default()
        };
        let plan = plan_numbering(&items, &options).unwrap();
        let mut offset = 0;
        for row in group {
            let count = inline(row[2]).parse::<usize>().unwrap();
            assert_eq!(plan[offset].fields["TRACK"], [inline(row[3])]);
            assert_eq!(plan[offset + count - 1].fields["TRACK"], [inline(row[4])]);
            offset += count;
        }
    }
}
