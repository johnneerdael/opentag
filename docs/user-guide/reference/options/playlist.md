# Playlist options

Open **File → Options… → Playlist** to configure **Playlist (all files)**, **Playlist (selected files)**, and **Generate playlists…**. A playlist stores references to audio files; creating one does not copy or transcode the music.

!!! tip "New to this?"
    Use `.m3u8` and relative entries for a Unicode playlist that can move with its album folder. Keep the filename-confirmation dialog enabled until destinations are predictable.

## Write extended info

Defaults to on. Extended info supplies a duration and display label in addition to the file path. The default **Extended info format string** is:

```text
%artist% - %title%
```

For `ARTIST=Juniper Vale`, `TITLE=Harbour Lights`, the label is `Juniper Vale - Harbour Lights`. A player may use this label without rereading tags. If tags later change, regenerate the playlist to update stored labels. Disable extended info for a consumer that expects paths alone; the consumer decides how it uses optional labels.

## Relative entries

**Entries relative to working directory (if possible)** defaults to on. TagGroove attempts relative paths instead of full machine-specific paths. For a playlist beside an album's files, `01 - Harbour Lights.flac` is easier to move than `/Music/Juniper Vale/Harbour Lights/01 - Harbour Lights.flac`.

The actual output location and common filesystem roots determine whether relative paths are possible. Paths across different Windows drives cannot always be represented relatively. Test a generated playlist after moving its folder as a unit; a relative playlist still breaks if its audio files are moved independently.

XSPF (`.xspf`), WPL (`.wpl`) and XML ASX (`.asx`) are input-only formats. The
writer refuses these suffixes rather than putting M3U text in an XML-named
file. Use `.m3u`, `.m3u8` or `.pls` for output. See
[XML playlist examples](../../guides/xml-playlists.md).

## Automatic playlists

**Create playlist automatically after saving tags or renaming files** defaults to off. Enable to regenerate the configured playlist after those operations. Automatic creation uses the saved configuration without asking for a filename each time.

Use a stable filename and confirm the selection/list scope before relying on this for a collection. A tag edit can otherwise have the extra effect of rewriting a playlist. Errors in automatic creation are reported separately; a successful tag write is not proof that the playlist also succeeded.

## Filename of playlist

Defaults to `%_directory%.m3u8`. The value is a filename format string, and may include a relative or absolute path. Relative output names are placed under the working directory. A folder called `Harbour Lights` therefore supplies `Harbour Lights.m3u8` with the default.

Loaded-playlist placeholders can reuse its name/location:

| Placeholder | Meaning |
|---|---|
| `%_playlist_filename%` | Loaded playlist's filename without extension. |
| `%_playlist_filename_ext%` | Loaded playlist's filename including extension. |
| `%_playlist_folderpath%` | Loaded playlist's containing folder path. |

These describe a loaded playlist, not whichever audio file is selected. Without that context, choose a filename that does not depend on them.

### Encoding and extension

| Extension | Output |
|---|---|
| `.m3u8` | UTF-8 M3U, suitable for Unicode names. |
| `.m3u` | M3U using the Windows ANSI codepage on Windows. On macOS/Linux, Windows-1252 is used only when TagGroove can read it back unchanged; otherwise UTF-8 is used. |
| `.pls` | PLS playlist. |

For example, a filename containing `Â©` would become the Windows-1252 bytes
`C2 A9`, which also form valid UTF-8 for the different text `©`. TagGroove uses
UTF-8 for that case and for characters outside Windows-1252. `.m3u8` always
uses UTF-8. Check the receiving player as well as loading the playlist back
into TagGroove.

**Write BOM for UTF-8 playlists (.m3u8, .pls)** defaults to off. Enable only when the target reader needs a byte-order mark for encoding detection. This setting is independent of [Export's BOM option](export.md).

## Confirm the filename

**Show confirm filename dialog** defaults to on. TagGroove first evaluates the configured filename format string, then shows the proposed path. Text entered into this confirmation dialog is a literal filename: new placeholders typed there are not evaluated.

For example, change the configured format string to `%album%.m3u8` if the album should supply the name. Typing that same expression into the confirmation field instead asks for a name containing those literal characters. Automatic playlist creation bypasses this dialog.

## Don't create directories

Defaults to off. Relative and absolute playlist output paths can create parent folders. Enable **Don't create directories** to require the destination folders to exist already. This prevents unexpected folder creation; it does not disable writing into an existing folder or turn a playlist into an audio copy.

## Related

- [Export options](export.md)
- [Messages and opening the result](messages.md)
- [Directories and working folder](directories.md)

<!-- verified: src/playlist-ui.ts PLAYLIST_DEFAULTS, playlistOptionsPage, PlaylistUI.create; crates/tag-core/src/playlist.rs path and encoding behavior; src-tauri/src playlist commands. Baseline007a806 controls unchanged inPR20. -->
