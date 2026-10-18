# Cut, copy and paste tags

Copying tags transfers structured fields, multiple values and available
embedded artwork. It is useful when replacing one encoding of a recording
with another. Pasting replaces the target's field set, so use copied practice
files before transferring an album.

## Copy one file to several targets

1. Select one source file with the desired tags. Inspect **Extended tags…**
   and its artwork first.
2. Choose **Edit → Copy tags** (++cmd+c++ / ++ctrl+c++ with the file list focused).
3. Select the destination files. Check the selected count.
4. Choose **Edit → Paste tags** (++cmd+v++ / ++ctrl+v++) and confirm when asked.
5. Inspect the pending fields and covers. Choose **File → Save tags** only
   for the targets that should receive them.

Copying reads the current displayed field values, including pending edits.
It does not first save the source. The tag clipboard is held inside TagGroove;
a textual version is also offered to the system clipboard when possible.
Copying text elsewhere does not import a new structured tag clipboard.

### Understand replacement before saving

For this example, the source has `TITLE=Paper Boats` and
`ARTIST=Juniper Vale`, with no `COMMENT`. The target has
`TITLE=Old title`, `ARTIST=Another artist` and `COMMENT=Device copy`.
After Paste, expect this pending result:

<!-- tag-paste-example:start -->
| Field | Source values | Target values before Paste | Target values after Paste |
|---|---|---|---|
| `TITLE` | `["Paper Boats"]` | `["Old title"]` | `["Paper Boats"]` |
| `ARTIST` | `["Juniper Vale"]` | `["Another artist"]` | `["Juniper Vale"]` |
| `COMMENT` | `[]` | `["Device copy"]` | `[]` |
<!-- tag-paste-example:end -->

The missing source comment removes the target comment; Paste is not a merge
that keeps target-only fields. Multiple source values stay separate values
in TagGroove's editing model. The destination writer still has its own
[format limits](../concepts/multi-value-fields.md#storage-depends-on-the-tag-format).
For copying only selected fields, use the field clipboard in
[Extended tags](extended-tags.md), or calculate a chosen field with
[Tag → Tag](../converters/tag-to-tag.md).

## Transfer several files in order

Copy sources in the intended file-list order, then select targets in the
matching order. Sorting affects that order; the order in which rows were
clicked is not a separate track-matching system.

<!-- tag-paste-counts:start -->
| Copied source count | Selected target count | Result |
|---|---|---|
| `1` | `3` | Same source tags for all targets |
| `2` | `2` | First source to first target; second source to second target |
| `2` | `1` | Refused: counts do not match |
| `2` | `3` | Refused: counts do not match |
<!-- tag-paste-counts:end -->

There is no automatic title/duration matching here. Sort both sets by a
reliable track order and inspect the first and last results before saving.
For an unfamiliar album, transfer one file at a time instead.

## Include or leave artwork deliberately

A successfully copied cover list replaces each target's embedded cover list
when pasted tags are saved. A source known to have no embedded covers carries
an empty list, so it can remove the target's covers. Cover roles, descriptions
and order are copied with the image data when available.

If source artwork could not be copied, the operation reports that limitation
and Paste leaves the target's covers alone. That is different from a verified
empty cover list. Folder-preview images are not copied as embedded artwork.
Inspect the status and [cover states](covers.md) instead of assuming every
text-tag copy included all pictures.

Pasted artwork stays pending until **Save tags**. **Discard pending changes**
abandons it with the selected files' other pending edits. Provider-document
sources and destinations have additional access/copy limits. For a supported
provider document, **Save a copy… → Edited copy** includes pending tag fields
and pasted artwork, including a pending removal of all covers. **Original
backup** uses the last verified source and ignores pending changes. Check the
saved copy before discarding the source or recovery files; cancelling or
failing the copy retains the source's pending edits.

For a **Source read-only** or **Recovery required** document, Copy, Paste and
Cut can still stage cover changes from its readable working copy. Choose
**Save a copy… → Edited copy** to keep those changes; **Save tags** and
immediate cover writes remain unavailable. If retained metadata is unreadable,
clipboard operations leave its artwork alone instead of staging a replacement
or removal from an unknown cover list.

## Cut without deleting the audio

**Edit → Cut tags** copies the selected tags, then stages removal of their
fields and supported covers. The source audio files remain in place. Saving
the source selection commits that removal; pasting to a destination does
not itself save either set.

If the cut was unintended, use **Edit → Discard pending changes** on the
source before saving. After a recorded save, inspect **Edit → Undo** instead.
If the status says artwork could not be copied, do not save the cut expecting
the clipboard to recover that artwork. Keep the original files until the
destination's tags and artwork have been saved and checked. Cut is not a move of the audio file, and Paste does not
rename it from the copied source filename.

## Copy a readable text report

For an email, track list or spreadsheet note, use **Edit → Copy as Text…**:

1. Select tracks and choose **Copy as Text…**.
2. Enter a format string, such as the example below.
3. Inspect the preview, which shows up to 5 files.
4. Choose **Copy** to calculate text for the whole selection.

<!-- copy-text-fields:start -->
For `TRACK=3/8`, `ARTIST=Juniper Vale` and `TITLE=Paper Boats`:
<!-- copy-text-fields:end -->

<!-- copy-text-example:start -->
| Format string | Copied line |
|---|---|
| `$num(%track%,2). %artist% — %title%` | `03. Juniper Vale — Paper Boats` |
<!-- copy-text-example:end -->

Each selected file contributes one evaluated entry, joined with a newline.
A field value containing line breaks can itself produce several text lines.
The remembered format history keeps the ten most recent distinct expressions.
This command changes no tags and supplies text rather than a structured
Paste-tags payload. Use [Export](../export/index.md) for encoded files,
headers, loops and a report that should be saved to disk.

## Gotchas

- Copy/Cut/Paste shortcuts inside a text-entry field operate on text. Focus
  the list or use the named **Edit** commands to transfer complete tags.
- A one-source paste onto a whole album also copies its title and track;
  it is unsuitable for setting only a shared album artist.
- Confirmations depend on **Options → Messages**. Disabling one does not
  change whether Cut and Paste are pending or whether Paste replaces fields.
- Keep backups. In-session undo history concerns recorded file writes; it
  is not a permanent clipboard archive or an external-change backup.

## Related

- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [The tag panel](tag-panel.md)
- [Extended tags](extended-tags.md)
- [Edit menu](../reference/menus/edit.md)

<!-- verified: src/tag-clipboard.ts:copyTags, pastePlan, pasteCoverPlan, cutPlan, cutCoverPlan, clipboardText; src/main.ts:paths, clipTags, stageTagUpdates, copy-text; src/utils-ui.ts:copyAsTextDialog; src/edit-history.ts:editShortcut. Written examples exercise actual models/core, not installed UI/save-back. -->
