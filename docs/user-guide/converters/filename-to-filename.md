# Filename → Filename

Use **Filename → Filename** to rearrange parts of existing names without
reading or changing tags. Mark the parts in the old pattern with `%1`
through `%9`, then reuse those parts in the new pattern.

## Swap an artist and title

Start with `Juniper Vale - Paper Boats.mp3` and make
`Paper Boats - Juniper Vale.mp3`:

1. Save pending edits and select copied practice files.
2. Choose **Convert → Filename → Filename…** (++alt+3++; Alt is ⌥ Option on a Mac).
3. Set **Old filename pattern** to `%1 - %2`.
4. Set **New filename pattern** to `%2 - %1`.
5. Choose **Preview** and inspect the source and destination rows.
6. Choose **Rename … files** to apply that reviewed plan.

The old pattern captures `%1` = `Juniper Vale` and `%2` = `Paper Boats`.
The new pattern outputs them in reverse order. The audio extension is
retained; it need not appear in either pattern.

## Keep, drop and transform parts

| Input filename | Old filename pattern | New filename pattern | Result |
|---|---|---|---|
| `003 Juniper Vale - Paper Boats.mp3` | `%1 %2 - %3` | `%2 - %3` | `Juniper Vale - Paper Boats.mp3` |
| `Juniper Vale__Paper Boats.flac` | `%1__%2` | `%1 - %2` | `Juniper Vale - Paper Boats.flac` |
| `Juniper Vale - Paper Boats.mp3` | `%1 - %2` | `$upper(%1) - %2` | `JUNIPER VALE - Paper Boats.mp3` |
| `03 - Paper Boats.mp3` | `%1 - %2` | `$num(%1,3). %2` | `003. Paper Boats.mp3` |

The old pattern is a matching pattern: literal punctuation must match
exactly. It accepts only numbered parts, with no closing percent sign
after the number. `%artist%` is not an old-pattern capture.

The new pattern is an output format. It can use scripting functions and
existing tag placeholders alongside the numbered parts. A part omitted
from the new pattern is discarded from the name. Reusing a part there
is allowed, for example `%2 - %2`.

## Keep the audio extension

Leaving the extension out of the new pattern is the simplest choice: the
converter keeps the source extension automatically. If the new pattern
already ends in that same extension, ignoring upper/lowercase, it appears
only once.

For source `song.mp3` and **Old filename pattern** `%1`:

| Input filename | Old filename pattern | New filename pattern | Result |
|---|---|---|---|
| `song.mp3` | `%1` | `%1` | `song.mp3` |
| `song.mp3` | `%1` | `%1.mp3` | `song.mp3` |
| `song.mp3` | `%1` | `%1.MP3` | `song.MP3` |
| `song.mp3` | `%1` | `%1.flac` | `song.flac.mp3` |

A different suffix becomes part of the name, followed by the source extension.
The audio data stays in its original format. Check **Preview** for an
unwanted extra suffix before applying the rename.

## Read and create folders

Backslashes in the old pattern mark folder boundaries. For example:

| Setting | Value |
|---|---|
| Input path | `Music/Juniper Vale - Harbour Lights/03 Paper Boats.mp3` |
| **Old filename pattern** | `%1 - %2\%3 %4` |
| **New filename pattern** | `%1 - %2 - %3 - %4` |

The four parts are `Juniper Vale`, `Harbour Lights`, `03` and `Paper Boats`.
The output name is `Juniper Vale - Harbour Lights - 03 - Paper Boats.mp3`.
The path's earlier `Music` component is outside this relative match.

Backslashes in the new pattern create destination folders. A relative
destination is based on the source file's containing folder, not the
earlier root omitted from the matching pattern. Use an absolute destination
when moving several source folders into one new layout. Preview the full
paths before applying.

## Combine folder captures, functions and literal brackets

Suppose this incoming file needs a new archive layout:

```text
D:\Incoming\Juniper Vale - Harbour Lights\03 Paper Boats (2024_DEMO).mp3
```

Set **Old filename pattern** to:

```text
%1 - %2\%3 %4 (%5_DEMO)
```

The pattern captures five parts: `%1` is `Juniper Vale`, `%2` is
`Harbour Lights`, `%3` is `03`, `%4` is `Paper Boats`, and `%5` is `2024`.
`_DEMO` is fixed text describing the source naming convention, so it does
not become part of the year capture.

Set **New filename pattern** to:

```text
D:\Archive\$upper(%1) - %2 '['%5']'\$num(%3,2). %4 (Demo)
```

The expected destination is:

```text
D:\Archive\JUNIPER VALE - Harbour Lights [2024]\03. Paper Boats (Demo).mp3
```

Work through it before applying:

1. `D:\Archive\` gives the absolute destination root. On macOS or Linux,
   choose the intended absolute root there instead.
2. `$upper(%1)` capitalizes the captured artist, without changing a tag.
3. `%2` retains the captured album. `'['%5']'` writes actual square brackets
   around the captured year; unquoted brackets would mark an optional section.
4. The next backslash starts the filename inside that new album folder.
5. `$num(%3,2)` pads the captured track number, and `%4` retains the title.
6. `(Demo)` adds a normalized suffix, and the converter keeps `.mp3`.

Preview the full destination. Unlike a relative output, this layout does
not nest a new album folder inside the incoming album folder. Check that
the chosen archive root is writable and the destination is not occupied.

## Working folder and replacement characters

The new-name format can use `%_workingdir%` and `%_workingpath%` from the
app’s working folder. They are separate from each source file’s own folder.
Slashes inside captured tag values use the replacement configured under
**Options → Files**, rather than always becoming underscores. Literal
backslashes in the new pattern still create destination folders.

## Gotchas

- `%10`, an undefined numbered part, or a repeated number in the old
  pattern is rejected. Use each old-pattern number once, from `%1` to `%9`.
- Adjacent captures such as `%1%2` have no separator and are ambiguous.
- Files whose old names do not match are listed as **No match, not changed**.
  Matching files can still be renamed together in one undo step. Check each
  planned path and reported error before assuming every file was renamed.
- Preview plans expire and are tied to the inputs, selection and source
  files. If the plan is stale or a file changed, run **Preview** again.
- Avoid duplicate destinations and existing files with those names.
  Keep copies and inspect **Edit → Undo** for recorded renames.

<!-- verified: crates/tag-core/src/converters.rs:FilenameConverter, numbered_pattern; src/main.ts:reviewedConverter -->

## Related

- [Filename → Tag](filename-to-tag.md)
- [Tag → Filename](tag-to-filename.md)
- [Format strings](../reference/format-strings.md)
- [Converters](index.md)
