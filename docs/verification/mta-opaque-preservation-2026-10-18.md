# R06: preserve opaque action settings through export

Local branch: `codex/parity-automation-mta-next`, based on PR47 snapshot
`36d7dea4e7b930d3a7d98facaca8498d0ef7c8de`. PR47 remains open and changing;
this receipt records the inspected base rather than approval to publish or
merge it. PR49 is frozen and is not amended by this work.

## Demonstrated accepted-input defect

An independently authored file, not a vendor fixture, contains an unknown
record with `T=9999`, `F=TITLE`, `1=line\x0abreak`, `2=left\x0dright` and
`9=\u000a\x0d`. The existing importer accepts it, decodes LF/CR values and
retains the complete property map in an unresolved action with its group
disabled. The exporter then rejects the same record with
`Line breaks cannot be stored in an .mta setting`.

The red test `opaque_mta_values_with_escaped_line_breaks_can_be_exported_and_reimported`
failed at that export before the implementation changed. Its local transcript
is `/tmp/taggroove-mta-preservation-red.log`. This proves a TagGroove
accepted-input preservation defect; it does not establish how the original
app interprets any unknown action, setting or control character.

## Narrow change

Allow the existing MTA value escaper to encode CR/LF only for unresolved
records, using `\x0d`/`\x0a`, which the importer already decodes. Preserve the
existing multiline refusal for recognized actions. Leave all action IDs,
option schemas, validators and executable behavior unchanged. An unresolved
action remains unresolved, and re-import leaves its group disabled.

The preservation promise covers decoded opaque property values and action
order. It does not cover original bytes, property order, section numbering,
source encoding or escape spelling. Supported actions retain existing
normalization: Replace flag `0` exports as the equivalent app spelling `0|0`.
The tests compare opaque maps exactly and compare supported actions by their
interpreted action values, without asserting byte-for-byte equivalence.

## Independent regression cases

`crates/tag-core/tests/mta_interop_preservation.rs` covers:

- Unknown action IDs with escaped LF, CR and combined control values.
- A known action ID with an unknown flag: retain the complete opaque map,
  including CRLF, Unicode, equals signs and an unrelated property.
- Actual control characters beside literal `\x0a`/`\u000a` text, trailing
  backslashes and mixed section ordering, so export cannot reinterpret literals.
- Export/re-import through UTF-8, BOM-marked UTF-16LE (the native host's output
  encoding) and BOM-marked UTF-16BE.
- Continued rejection of LF/CR/CRLF in recognized actions, with a control that
  literal escape text still exports, imports and stays executable.

The focused suite passes all three tests. No audio files or installed user
configuration are used. Native host serialization is represented by the same
UTF-16 encoding boundary; no claim of native UI execution is made.

## Local validation

On macOS/Apple Silicon with Rust/Clippy 1.92, using the established offline
audio archive directory and Apple archive tools:

- `npm test`: 561 frontend and 1,085 Rust tests passed; six existing Rust tests
  remained ignored. Transcript: `/tmp/taggroove-mta-preservation-npm-test.log`.
- `cargo test -p tag-core --locked`: 644 tests passed, including the three new
  cases. Transcript: `/tmp/taggroove-mta-preservation-tag-core.log`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `npm run build`, strict MkDocs, `cargo fmt --all -- --check`,
  `node scripts/i18n-extract.mjs --check` and `git diff --check`: passed.

Cargo printed an existing global-cache auto-clean permission warning; its
commands completed successfully. The unrelated cache was not changed manually.
No remote publication, review request, CI dispatch or merge was performed.

## Original shipped evidence and limits

Reference executable: Mp3tag 3.36.1 Windows x64, SHA-256 freshly verified as
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The four supplied shipped files were inventoried directly; no vendor contents
were copied into the repository:

| Shipped action file | SHA-256 | Observed type IDs |
| --- | --- | --- |
| CD-R.mta | `ccf92feca6bf8ea7606ba905ba36ee38627f21a101aa3af10e911e2467d79597` | 2 |
| Case conversion.mta | `9dcf4db6d968ef54c1906e22d28f3a0cfb75352526ef741546199af614684aac` | 1 |
| Schreibweise aendern.mta | `9dcf4db6d968ef54c1906e22d28f3a0cfb75352526ef741546199af614684aac` | 1 |
| Standard.mta | `e6df2a5c8782fa3c433191ef9185529476402a45f4276f71c7979b1edad6b4d7` | 2 |

A temporary local read-only probe loaded each private file with TagGroove's
importer, exported it, encoded the result as UTF-16LE/BOM and imported it again.
It found no import issues and equal interpreted actions for all four files:
19, 1, 1 and 6 records respectively. Its transcript is
`/tmp/taggroove-mta-shipped-reference-probe.log`; its temporary harness was
removed. This is execution of TagGroove against shipped input, not execution
of the original application.

These are static examples of Case Conversion/Replace records and observed
Latin-character `\xNN` escapes. They contain no opaque/control-character
case for this fix, no special-action serialization evidence and no runtime
results. Existing app import/export unit tests for other numeric schemas are
app consistency checks; their names do not independently qualify the original
application's serialization.

R06 still needs original-generated option matrices, unknown-option samples,
original re-import/readback and the three special-action mappings. R05 still
needs original expression results for the retained scripting/context gaps.
This local preservation fix closes none of those reference-proof requirements.
