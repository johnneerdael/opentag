//! Independently authored accepted-input preservation cases. These are not
//! original Mp3tag-generated fixtures or proof of its runtime semantics.
use tag_core::actions::{export_mta, import_mta, ActionGroup, ActionSpec};

#[test]
fn opaque_mta_values_with_escaped_line_breaks_can_be_exported_and_reimported() {
    let input = b"[#0]\nT=9999\nF=TITLE\n1=line\\x0abreak\n2=left\\x0dright\n9=\\u000a\\x0d\n";
    let imported = import_mta("Opaque preservation".into(), input, None).unwrap();
    assert!(!imported.group.enabled);
    assert_eq!(imported.issues.len(), 1);
    assert_eq!(imported.records[0].properties["1"], "line\nbreak");
    assert_eq!(imported.records[0].properties["2"], "left\rright");
    assert_eq!(imported.records[0].properties["9"], "\n\r");
    assert!(matches!(
        &imported.group.actions[0],
        ActionSpec::UnresolvedMta { type_id, .. } if type_id == "9999"
    ));

    let exported =
        export_mta(&imported.group).expect("accepted opaque properties must survive export");
    assert!(exported.contains("1=line\\x0abreak\r\n"));
    assert!(exported.contains("2=left\\x0dright\r\n"));
    let reimported = import_mta("Opaque preservation".into(), exported.as_bytes(), None).unwrap();
    assert_eq!(
        reimported.records[0].properties,
        imported.records[0].properties
    );
    assert_eq!(reimported.group.actions, imported.group.actions);
    assert!(!reimported.group.enabled);
    assert_eq!(reimported.issues, imported.issues);
}

#[test]
fn opaque_future_options_preserve_controls_literals_unicode_and_action_order() {
    // A known regex action with an unknown future flag must remain opaque.
    // The numerical section order is intentional, independent of file order.
    let input = concat!(
        "[#12]\nT=4\nF=TITLE\n3=future-flag\n",
        r"1=left\x0d\x0aright\\x0a",
        "\n",
        r"2=literal \\u000a versus \u000a; \\x0d versus \x0d",
        "\n",
        "extra=é 猫 😀 = unchanged\n",
        r"99=\\",
        "\n",
        "[#2]\nT=2\nF=TITLE\n1=old\n2=new\n3=0\n",
    );
    let imported = import_mta("Future options".into(), input.as_bytes(), None).unwrap();
    assert_eq!(
        imported
            .records
            .iter()
            .map(|record| record.index)
            .collect::<Vec<_>>(),
        [2, 12]
    );
    assert_eq!(imported.issues.len(), 1);
    assert_eq!(imported.issues[0].type_id, "4");
    assert!(!imported.group.enabled);
    let properties = &imported.records[1].properties;
    assert_eq!(properties["1"], "left\r\nright\\x0a");
    assert_eq!(
        properties["2"],
        "literal \\u000a versus \n; \\x0d versus \r"
    );
    assert_eq!(properties["99"], "\\");
    assert!(
        matches!(&imported.group.actions[0], ActionSpec::Replace { from, to, .. } if from == "old" && to == "new")
    );
    assert!(
        matches!(&imported.group.actions[1], ActionSpec::UnresolvedMta { type_id, .. } if type_id == "4")
    );

    let exported = export_mta(&imported.group).unwrap();
    assert!(exported.starts_with("[#0]\r\nT=2\r\n"));
    assert!(exported.contains("1=left\\x0d\\x0aright\\\\x0a\r\n"));
    // Cover UTF-16LE/BOM used by the host, plus UTF-8 and the importer's
    // UTF-16BE path, without writing a user file.
    let utf16le = [0xff, 0xfe]
        .into_iter()
        .chain(exported.encode_utf16().flat_map(u16::to_le_bytes))
        .collect::<Vec<_>>();
    let utf16be = [0xfe, 0xff]
        .into_iter()
        .chain(exported.encode_utf16().flat_map(u16::to_be_bytes))
        .collect::<Vec<_>>();
    for bytes in [exported.as_bytes(), utf16le.as_slice(), utf16be.as_slice()] {
        let reimported = import_mta("Future options".into(), bytes, None).unwrap();
        assert_eq!(reimported.group.actions, imported.group.actions);
        assert!(!reimported.group.enabled);
        assert_eq!(reimported.issues.len(), 1);
        assert_eq!(reimported.issues[0].type_id, "4");
        assert_eq!(reimported.issues[0].reason, imported.issues[0].reason);
        // Resolved Replace flags use their existing canonical spelling (0|0).
        // Only the opaque map claims exact decoded-property preservation.
        assert_eq!(
            reimported.records[1].properties,
            imported.records[1].properties
        );
    }
}

#[test]
fn resolved_actions_keep_multiline_refusal_and_literal_escape_roundtrips() {
    for template in ["left\nright", "left\rright", "left\r\nright"] {
        let group = ActionGroup {
            name: "Resolved multiline".into(),
            enabled: true,
            actions: vec![ActionSpec::FormatValue {
                field: "COMMENT".into(),
                template: template.into(),
            }],
        };
        assert_eq!(
            export_mta(&group).unwrap_err(),
            "Line breaks cannot be stored in an .mta setting"
        );
    }
    let group = ActionGroup {
        name: "Resolved literals".into(),
        enabled: true,
        actions: vec![ActionSpec::FormatValue {
            field: "COMMENT".into(),
            template: r"literal \x0a \u000d".into(),
        }],
    };
    let exported = export_mta(&group).unwrap();
    let reimported = import_mta("Resolved literals".into(), exported.as_bytes(), None).unwrap();
    assert!(reimported.issues.is_empty());
    assert!(reimported.group.enabled);
    assert_eq!(reimported.group.actions, group.actions);
}
