# Playlists and cue sheets

A playlist is an ordered list of references to audio files. Saving one does
not copy, transcode or retag the recordings. Use it to preserve a listening
order or create a list for another player, then test that the references work
in their intended location.

These workflows assume ordinary local desktop files and writable folders.
Provider documents have different access limits. If the file list contains
chapter or CUE-track rows, follow [Chapters and cue sheets as rows](chapters.md)
and return to whole audio files before creating playlists.

## Load a playlist and inspect its order

1. Save or deliberately discard pending edits before replacing the list.
2. Choose **File → Load playlist/cue sheet…** and select a `.m3u`, `.m3u8`,
   `.pls`, `.cue`, `.xspf`, `.wpl` or `.asx` file. Inspect the pending-edit prompt if one appears.
3. Check the loaded rows and error messages. Missing references can be
   reported while other valid entries are loaded.

Opening this way replaces the loaded workspace. Playlist order becomes load
order, with column sorting cleared so it does not obscure that order. An
ordinary M3U/M3U8 uses path-line order; PLS uses the numeric `File1`, `File2`,
… keys, even if those lines were written in another order. Repeated
references to the same resolved path keep their first position. This is not
a promise about every filesystem alias identifying the same recording.

Relative references resolve beside the playlist, not beside TagGroove's
executable. Local `file://` references are accepted; network stream URLs such
as HTTP are reported as unsupported by this local-file loader. A broken
reference is not repaired by a playlist label or by renaming its suffix.

For example, with both named WAV files present beside `list.m3u8`:

<!-- playlist-load-example:start -->
```text
#EXTM3U
02 - Low Tide.wav
01 - Paper Boats.wav
02 - Low Tide.wav
missing.wav
https://example.invalid/live
```
<!-- playlist-load-example:end -->

TagGroove resolves two unique local files, in the order `02 - Low Tide.wav`,
then `01 - Paper Boats.wav`. It reports `missing.wav` on line 5 and the HTTP
entry on line 6. It does not fetch that URL as an audio stream. Actual audio
reading can report further errors after reference resolution.

## Read XML playlists

XSPF version 1, static WPL and XML ASX version 3 can be loaded through the same
command or file-drop path. Tracks retain document order. Multiple XSPF
`location` elements or ASX `REF` elements within one track are alternatives;
only the first resolvable local file is loaded for that track. XML playlists
are read-only formats here: save new playlists as `.m3u`, `.m3u8` or `.pls`.

See [XML playlist rules and examples](xml-playlists.md) for URI escaping,
local XML Base, unsupported smart/active instructions and error behavior.
A fatal XML parse or encoding error keeps the previous list unchanged. A
valid playlist with missing/unsupported entries can still load its other files
and report those entries. A valid empty list can replace the list with no rows.

## Add files and choose the order

Use **File → Add files…** or **File → Add directory…** to add more audio files.
A dropped/opened playlist is expanded into its referenced entries. When
dropping onto the file list, hold Command on macOS or Control on Windows/Linux
to add; without an add modifier, a file drop replaces the list. Use the Add
commands when modifier detection or the drop target is uncertain. Adding to
an existing workspace differs from the explicit replace operation: it need
not clear the current sort or replace the remembered loaded-playlist context.

Review the [file-list order](file-list.md) before writing. Header sorting and
manual row ordering while holding Alt (Option on macOS) affect the sequence
supplied to a playlist. A
playlist generator groups that sequence; it does not independently sort by
track number. Changing the loaded list does not rewrite the playlist on disk.

## Choose the writing scope

The labels below use “all files,” but the current host supplies the visible
file-list rows for that scope. A filter can therefore change it. Selected
scope can include selected rows that the filter hides. Inspect both the
visible and selected counts.

<!-- playlist-scope-examples:start -->
| Command | Scope used | Filename handling |
|---|---|---|
| **Playlist (all files)** | All currently visible rows, in list order | Configured filename; confirmation if enabled. |
| **Playlist (selected files)** | Selected rows, in list order | Configured filename; confirmation if enabled. |
| **Playlist per folder (all files)** | All currently visible rows, grouped by containing folder | Writes per folder without individual filename dialogs. |
| **Generate playlists…** | Selection when any rows are selected; otherwise all visible rows | Filename and partition fields, live preview, then **OK**. |
<!-- playlist-scope-examples:end -->

Before exporting only a filtered result, use **Edit → Unselect all**, then
select the intended visible rows. Before generating from the entire visible
list, unselect everything if the generator would otherwise choose a smaller
selection. Avoid chapter/CUE rows for these whole-file playlists.

## Save a small practice playlist

1. Keep original music and existing playlists. Save intended tag edits first:
   the backend reads saved fields for filenames and extended labels.
2. Arrange two short copied WAV clips in this order:

    <!-- playlist-write-items:start -->

    | Filename | Seconds | Artist | Title |
    |---|---|---|---|
    | `01 - Paper Boats.wav` | `2` | `Juniper Vale` | `Paper Boats` |
    | `02 - Low Tide.wav` | `3` | `Juniper Vale` | `Low Tide` |

    <!-- playlist-write-items:end -->

3. Open **File → Options… → Playlist**. For this example, enable **Write
   extended info** and relative entries, use <!-- playlist-extinf-format -->`%artist% - %title%` as the
   extended-info format, leave BOM off, and keep filename confirmation on.
4. Choose **File → Playlist (selected files)** after selecting the two clips.
5. In the confirmation dialog, choose a new `practice.m3u8` beside the clips.
   This field is a literal filename: placeholders typed here are not evaluated.
6. Choose **OK** and inspect the resulting text or load the new playlist.
   No additional **Save tags** step writes the playlist.

The M3U8 text is:

<!-- playlist-write-m3u8:start -->
```text
#EXTM3U
#EXTINF:2,Juniper Vale - Paper Boats
01 - Paper Boats.wav
#EXTINF:3,Juniper Vale - Low Tide
02 - Low Tide.wav
```
<!-- playlist-write-m3u8:end -->

`#EXTINF` stores a rounded duration and display label. A player can use that
label without rereading tags, so later tag edits can require regenerating the
playlist. TagGroove writes CRLF line endings; the displayed example represents
the same lines. Multi-line labels are flattened to spaces; line breaks in
entry paths are rejected instead of becoming extra records. Disabling
extended info writes path lines alone.

A `.pls` destination for the same ordered items produces:

<!-- playlist-write-pls:start -->
```text
[playlist]
File1=01 - Paper Boats.wav
Title1=Juniper Vale - Paper Boats
Length1=2
File2=02 - Low Tide.wav
Title2=Juniper Vale - Low Tide
Length2=3
NumberOfEntries=2
Version=2
```
<!-- playlist-write-pls:end -->

## Paths, names and encoding

Relative entries are calculated from the output playlist's folder. The
configured working-directory root bounds when relative entries are used;
entries or outputs outside it can remain absolute. Different filesystem
roots, such as Windows drives, may also prevent relative paths. Move a
playlist with its referenced directory layout intact and test it in the
receiving player.

The configured **Filename of playlist** is a format string. Relative output
names are based on the working directory. The per-folder command resolves a
relative configured name beside each group's first file; an absolute name
overrides that base and still needs a distinct destination for each group.
A loaded playlist can provide
`%_playlist_filename%`, `%_playlist_filename_ext%` and `%_playlist_folderpath%`.
Those are context values for that loaded playlist, not the active audio file.
See [Playlist options](../reference/options/playlist.md) for exact controls.

- `.m3u8` is UTF-8; `.pls` is written as UTF-8. The optional BOM setting is
  separate from Export's BOM setting.
- `.m3u` uses the Windows system ANSI codepage on Windows. On macOS/Linux,
  Windows-1252 is used only when the playlist reads back unchanged; otherwise
  output falls back to UTF-8. This includes unrepresentable characters and
  ambiguous Windows-1252 bytes such as `Â©`, which could otherwise decode as
  the different UTF-8 text `©`.
- The loader honors a Unicode BOM, reads `.m3u8` as UTF-8, and otherwise tries
  valid UTF-8 before the ANSI fallback. A consumer can have different rules.
- CUE loading is a separate workflow. Do not choose `.cue` as the extension
  for this M3U/PLS writer and expect a cue-sheet structure to be generated.

Without a system codepage, these entry names illustrate the `.m3u` fallback:

<!-- playlist-encoding-cases:start -->

| Entry name | Output encoding |
|---|---|
| `Maré.wav` | Windows-1252 |
| `Â©.wav` | UTF-8 |
| `港.wav` | UTF-8 |

<!-- playlist-encoding-cases:end -->

A configured ANSI codepage that cannot represent a name can cause a write
error. Choose `.m3u8` for Unicode instead of deleting characters merely to fit
that playlist encoding. A successful write still needs a receiving-player check.

## Generate several playlists or update automatically

**File → Generate playlists…** groups files by equal **Partition format**
results. **Playlist file name** is evaluated using each group's first file.
The preview shows the count and at most the first 50 output paths; **OK**
plans and writes from the current request. Preview does not reserve a path.
See [One playlist per album folder](../cookbook/playlist-per-album.md).

The [Generate Playlists action](../actions/generate-playlists.md) uses selected
files and runs after the action group's per-file changes, regardless of its
position inside that group. Use a separate group when an earlier intermediate
state is required. It has different interaction and timing from the generator
dialog's preview.

Automatic playlist creation is off by default. The option can request a
playlist after successful tag saves or normal renames, using the current
visible-list scope and saved filename configuration without prompting for a
name. Keep it off until that extra output is intentional. An automatic
playlist error is separate from a successful tag save.

## Gotchas and recovery

- Creating a playlist writes an external file immediately. It does not make
  its references portable by copying the audio.
- Choose new practice output names. Existing outputs can be replaced; do not
  rely on an absent confirmation or on every alias/overwrite case being blocked.
- **Don't create directories** requires the output parent to exist. It does
  not make writing into an existing folder a preview-only operation.
- Several outputs are rendered before writing, but a later filesystem failure
  does not establish that earlier outputs were rolled back.
- Do not assume **Edit → Undo** deletes or restores generated playlist files.
  Keep originals and remove unwanted practice outputs deliberately.

## Related

- [One playlist per album folder](../cookbook/playlist-per-album.md)
- [Chapters and cue sheets as rows](chapters.md)
- [Playlist options](../reference/options/playlist.md)
- [Generate Playlists action](../actions/generate-playlists.md)
- [The file list](file-list.md)

<!-- verified: playlist_chapter_guide.rs executes actual written playlist formats/loading/encoding; guide-playlist-chapters.test.mjs checks actual UI request scopes. main.ts/playlist-ui.ts/playlists.rs supply current saved-file context and timing. No native player/application/provider or all-platform overwrite qualification. -->
