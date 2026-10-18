# File menu

Use **File** to load audio, write or reload tags, make reports and manage the
application configuration. Loading files into the list does not convert their
audio. A command mentioning all files has a different scope from one using
the current selection.

## Tags and copies

| Item | What it does |
| --- | --- |
| **Save tags** | Write pending field/cover changes to the selected files when save-back is supported |
| **Remove tag…** | Remove the tag families selected under Options, rather than deleting the audio |
| **Save a copy…** | Keep edited data through the supported provider-document copy workflow when available; check its format and cover limitations |
| **Reload tags** | Read selected files again from their current storage |

For example, selecting two Juniper Vale tracks and changing `ARTIST` stages
that value for both. **Save tags** writes the pending change. A field left at
its keep-current-values state does not become an instruction to overwrite
it. To replace or clear a field, make that edit explicitly and inspect the
pending selection.

For a supported provider document, **Save a copy…** offers **Edited copy**
with pending tag-field and artwork changes, including removal of all covers,
or **Original backup** from the last verified source. The original backup
ignores pending changes. The source, pending edits and recovery copies are
retained, and the written copy is checked before a successful result is
reported. Cancelling the destination picker or failing to write the copy
leaves those source edits available for another attempt.

**Reload tags** is useful after an external edit, but first resolve any
pending changes you want to retain. It is different from **Refresh**, which
revisits loaded folders as well. **Remove tag…** follows the read/write/remove
policy in [Tags → Mpeg](../options/tags-mpeg.md) and other format options;
removing one tag family can leave another readable family in the same file.

## Playlists and reports

| Item | Input and result |
| --- | --- |
| **Playlist (all files)** | Create a playlist from the loaded list |
| **Playlist (selected files)** | Create one from the selection |
| **Playlist per folder (all files)** | Create playlists grouped by their audio folders |
| **Generate playlists…** | Open the dialog for format-based playlist generation |
| **Export…** | Run or edit a saved export configuration, depending on the selection-dialog preference |
| **Export CSV…** | Write a direct CSV export; use Export… for a configurable report template |

Playlist paths and display titles are controlled separately from audio tags.
**Playlist per folder (all files)** is the Primary+Shift+P command; it is not
the same dialog as **Generate playlists…**. See [Playlist options](../options/playlist.md)
for path style, filename, encoding and BOM choices.

[Export](../../export/index.md) produces a report from selected files. The
saved template controls text, loops and encoding. Primary+Shift+E requests
per-directory export. An output report is a new text file, not a tag change
waiting for **Save tags**.

## Load and navigate

| Item | What changes |
| --- | --- |
| **Change directory…** | Replace the loaded list with a chosen folder |
| **Add directory…** | Add a folder's supported files to the existing list |
| **Add files…** | Choose individual files |
| **Load playlist/cue sheet…** | Load the audio references in a playlist or cue sheet |
| **Upper directory** | Replace the view with the working folder's parent |
| **Previous directory** / **Next directory** | Move backward/forward through directory history |
| **Favorite directory** | Load the first configured favorite |
| **Favorite directories…** | Manage the ordered favorite-folder list |
| **Recent directories** / **Favorite directories** | Open an entry from the corresponding submenu when entries exist |
| **Clear list** | Remove loaded rows without deleting their files |
| **Open in Finder/Explorer/file manager** | Reveal selected files; without a selection, open the working folder when one is available |
| **Play** | Open selected files with their associated player |

The [Directories options](../options/directories.md) control subfolder scanning
and related behavior. Changing a directory can ask how to handle pending
edits. A failed scan keeps the previous working directory. Adding a directory
does not replace the working directory or silently save pending tags.

A loaded playlist determines an input order, but an active column sort can
show a different order. Use **View → Unsorted (load and playlist order)** to
inspect the load order. Loading a playlist does not rewrite the playlist or
move its audio files. Cue-sheet chapter/track handling depends on the supported
file/container workflow; it does not split the audio into new files.

See [command line](../command-line.md) for opening paths from scripts or
shortcuts. Where folder picking is unavailable, use **Add files…** and respect
the current provider-document/save-back limitations.

## Configuration and closing

| Item | Result |
| --- | --- |
| **Options…** | Open application preferences |
| **Save configuration…** | Save a configuration archive for backup/transfer |
| **Load configuration…** | Choose and load a supported configuration archive |
| **Open configuration folder** | Open the folder containing the app's settings/configuration files |

A configuration archive is not a backup of the audio library. Keep original
media and any report outputs separately. Loading configuration changes app
settings, actions, exports, layouts and remembered lists. After confirmation,
loading discards pending edits and reloads the window. Inspect the selected
archive and save edits you need first. The Discogs access token is not included
in the configuration archive.

The current webview File menu has no separate **Exit** item. Close the window
or quit through the operating system's app controls. Resolve pending edits
when asked. [General options](../options/general.md) can make minimizing use
the notification area on supported platforms; a minimized window is not the
same as an exited process.

Use the [keyboard reference](../keyboard-shortcuts.md) for platform-specific
keys. Here, Primary means Ctrl on Windows/Linux and Command on macOS.
