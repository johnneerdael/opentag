# Tag → Filename

Use **Tag → Filename** to give files consistent names from their tags.
The same converter can build folders and move the files into them. Check
that the source fields are correct before using them to construct paths.

## Rename a few tracks

1. Keep copies, save pending edits and select practice files.
2. Choose **Convert → Tag → Filename…** (++alt+1++; Alt is ⌥ Option on a Mac).
3. Enter this **Format string**:

    ```text
    $num(%track%,2) - %title%
    ```

4. Choose **Preview**. For track `3/8` and title `Paper Boats`, expect
   `03 - Paper Boats.mp3` for an MP3 file.
5. Check every destination, then choose **Apply to … files**.

The source tags remain unchanged. The rename happens when applied; there
is no separate **Save tags** step. The audio extension is retained, so
usually leave it out of the expression. An expression already ending in
that extension does not receive another copy.

## Understand the expression

| Part | Meaning |
|---|---|
| `%track%` | Read the track value, such as `3/8` |
| `$num(%track%,2)` | Keep its leading number and pad to at least two digits: `03` |
| ` - ` | Insert the literal separator, including the two spaces |
| `%title%` | Read the title |

`$num` reads a number; it does not preserve a vinyl side such as `B2`.
Preview those cases before using a numeric format on a mixed collection.

With artist `Juniper Vale`, album `Harbour Lights`, year `2024`, track
`3/8` and title `Paper Boats`:

| Format string | Result for an MP3 file |
|---|---|
| `%artist% - %title%` | `Juniper Vale - Paper Boats.mp3` |
| `$num(%track%,3). %title%` | `003. Paper Boats.mp3` |
| `%album%\$num(%track%,2) - %title%` | `Harbour Lights\03 - Paper Boats.mp3` |
| `%artist%\%album% (%year%)\$num(%track%,2) - %title%` | `Juniper Vale\Harbour Lights (2024)\03 - Paper Boats.mp3` |

The backslashes in the last two rows separate folders on every platform.

## Build folders deliberately

A relative folder format creates the destination beneath the audio file's
current folder. Use an absolute destination when reorganizing files from
different locations into one collection. For example, on Windows:

```text
D:\Music\%artist%\%album%\$num(%track%,2) - %title%
```

On macOS or Linux, use the intended absolute root, for example
`/Music/%artist%/%album%/$num(%track%,2) - %title%`. Choose a writable location
appropriate to the machine; the example root need not exist there.

Literal separators in the expression create folders. Separators originating
inside tag values are treated as characters in a component and use the
configured folder-separator replacement. Other forbidden filename characters
are sanitized. See [filename-mode rules](../reference/format-strings.md#file-and-folder-names).

## Associated lyrics and karaoke files

Enable **Options → Files → Move matching .lrc and .cdg files with audio** to
rename matching companions alongside the audio. Their paths appear below the
audio destination in **Preview**. The policy defaults to off. Review every path;
a changed companion or collision requires another preview. See
[associated files](../guides/associated-files.md) for the exact matching rules,
group rollback and session Undo limits.

## Gotchas

- A missing title can produce an incomplete but valid name. Check the
  preview rather than assuming missing fields make the whole conversion fail.
- Two files can produce the same destination when track or disc information
  is omitted. Include enough information to make each path unique.
- Changing the selection or format invalidates the preview. Run it again.
  A changed file or occupied destination can still prevent applying a plan.
- Moving a file may remove it from the list depending on **Keep moved files**.
  Inspect the destination folder before assuming the file disappeared.
- Keep originals and inspect **Edit → Undo** for recorded renames.

<!-- verified: crates/tag-core/src/script.rs:evaluate_filename_with_context; src/main.ts:converter; src-tauri/src/lib.rs:preview_rename, rename_files -->

## Related

- [Format strings](../reference/format-strings.md)
- [Filename → Tag](filename-to-tag.md)
- [Filename → Filename](filename-to-filename.md)
- [Zero-padded track numbers](../cookbook/rename-zero-padded-track-numbers.md)
