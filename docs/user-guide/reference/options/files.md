# Files options

Open **File → Options… → Files** to restrict folder scans, control separators inside tag-derived names, and preserve file modification times.

!!! tip "New to this?"
    A tag value is data; a separator typed into a filename format string is a request to create a folder. This page keeps those two uses separate.

## Associated files

**Move matching .lrc and .cdg files with audio** defaults to off. Enable it to
include exact-basename lyrics and karaoke companions in **Tag → Filename** and
**Move files to**. Review their source and destination paths before applying.
Other rename commands and **Copy files to** do not use this policy. See
[associated files](../../guides/associated-files.md) for matching rules, errors
and undo behavior.

## Read files

**Restrict incoming files to** defaults to empty, meaning all supported formats. Enter extensions to limit files discovered while scanning a folder:

```text
*.mp3;*.flac
```

`mp3`, `.mp3`, and `*.mp3` are accepted forms. Separate entries with semicolons, commas, or whitespace. Matching is case-insensitive, and repeated extensions are ignored. This is an extension list, not a general filename pattern: `album*.mp3` is invalid. An allowed extension must still be a format TagGroove can read.

The restriction applies to folders. Files explicitly added individually are still read. To inspect a single M4A alongside an MP3-only folder scan, add that file directly. Changing the restriction does not itself remove already loaded rows; rescan the folder to use the new choice.

## Rename files from tags

**Replace folder separator in tag values with** defaults to `_`. TagGroove replaces both backslashes and forward slashes inside tag values used for file renaming. This applies to Tag → Filename and filename-changing Format Value, Guess Values, and Tag → Tag operations.

| `ALBUM` value | Replacement | Tag-derived name component |
|---|---|---|
| `Harbour/Live` | `_` | `Harbour_Live` |
| `Harbour/Live` | ` - ` | `Harbour - Live` |
| `Harbour/Live` | Empty | `HarbourLive` |

The replacement may contain up to 16 characters. It cannot itself contain `\ / : * ? " < > |` or control characters. Leaving it empty removes separators; it does not leave them unchanged.

Separators deliberately typed into the format string still form folders. With `%artist%\%album%\%title%`, the two typed backslashes request artist and album folders; a slash inside an actual album value is replaced. Preview the complete path before applying a rename, especially after changing this setting.

## Write files

**Preserve timestamps of modified files** defaults to off. Enable to restore the original modification time after writing or removing tags. It preserves that filesystem timestamp; it does not preserve the complete file bytes or prevent metadata changes.

A backup or sync tool that relies only on modification times may overlook an edit when this is enabled. The [Library](library.md) also uses modification time and size to recognize external changes. If an external editor preserves both, use **Reload tags** to force a fresh read.

## Related

- [Directories and folder scanning](directories.md)
- [Tag write policies](tags-mpeg.md)
- [Library cache](library.md)

<!-- verified: src/options-ui.ts parseExtensions, folderSeparatorProblem; src-tauri/src/settings.rs parse_extensions; crates/tag-core/src/preferences.rs TagOptions; crates/tag-core/src/lib.rs DEFAULT_FOLDER_SEPARATOR; crates/tag-core/src/tags.rs write paths preserve_modified_time; crates/library/src/lib.rs Stamp. -->
