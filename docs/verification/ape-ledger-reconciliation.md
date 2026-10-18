# APE ledger reconciliation

The remaining-gaps register still described header-less APEv2 tags and an
APE tag containing `TAG` at the ID3v1 detection offset as broken. Both fixes
already exist in `crates/tag-core/src/ape_footer.rs` and were described in
[the later format verification](formats-gaps.md). This documentation change
removes that stale implementation gap; it changes no reader or writer.

## Evidence checked on 2026-10-18

The inspected source is PR #47 branding head
`864b1448f7c3e4ea95a16aa9f5da51ea7e25a08c`. Its APE implementation and
regressions are unchanged from `c1375a6ccf24e728f3ac29dfd442d4bf1071d47b`,
against which the following command completed successfully:

```sh
cargo test --locked -p tag-core --test formats_gaps ape
```

All three selected tests passed:

- `headerless_apev2_tags_are_read_and_written_without_losing_audio`: footer-only
  tags on synthetic MPEG and WavPack carriers can be read and saved while
  preserving the checked audio bytes and retained APE bytes.
- `tag_inside_ape_items_is_not_taken_for_id3v1`: `TAG` inside an APE item is
  not exposed as an ID3v1 tag; the checked save combinations retain the APE
  item and keep a newly written ID3v1 tag separate.
- `a_131_byte_ape_tag_keeps_its_header`: an ID3v2-only save preserves the
  existing 131-byte APE tag's header.

The local test log is `/tmp/taggroove-ape-ledger-regressions.log`. It is a
local receipt, not a published artifact or final-head hosted-CI result.

## Existing real-encoder coverage

The specialist audit also reran the existing FFmpeg-dependent integration
checks on the unchanged format implementation. FFmpeg and ffprobe 9.0.2
(libavformat 63.1.102) were available and successfully executed; none of
these checks took its missing-tool skip path:

```sh
cargo test --locked -p tag-core --test asf --test containers --test formats_edge ffmpeg -- --nocapture
```

All five selected tests passed: two WMA/ASF checks, two Matroska/WebM checks
including chapter edits, and one AAC/WavPack check. They create actual encoded
carriers, apply the tested metadata changes and use independent FFmpeg decode,
packet-hash or ffprobe checks as specified in each test. They are not limited
to hand-built container headers. The implementation already had these tests;
this change records their successful execution rather than adding duplicates.
The local log is `/tmp/taggroove-specialist-real-encoder-checks.log`.

This does not provide TAK/OptimFROG real-encoder fixtures, original-written
Nero/APE fixtures, or original-Mp3tag comparison. Those requirements remain
open in the roadmap.

## Qualification still required

These are synthetic regression fixtures, not original-Mp3tag captures or
real-encoder format certification. They do not establish equivalent original
save defaults, tag ordering, malformed-file handling, or every APE layout.
The ledger retains reference-file qualification and tag-kind policy gaps.
No original executable was run, no whole format area is marked complete,
and PR #47's merge hold remains in effect.
