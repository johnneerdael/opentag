# Load local XML playlists

Use **File → Load playlist/cue sheet…** to load `.xspf`, `.wpl` or `.asx`
files. Dropped playlists and forwarded file-open requests use the same reader.
Save intended pending edits before replacing the workspace. The files named
by the playlist stay on disk and their tags are read normally; playlist titles
and other descriptive metadata do not become tag edits.

## Supported static structures

| Input | Media references | Order and alternatives |
| --- | --- | --- |
| XSPF version 1 | `playlist/trackList/track/location` in the XSPF namespace | Track order; one local file from each track’s ordered location alternatives |
| WPL | `smil/body/seq/media` with `src` | Static media order |
| XML ASX version 3 | `ASX/ENTRY/REF` with `HREF` | Entry order; one local file from each entry’s ordered REF alternatives |

The XSPF namespace is `http://xspf.org/ns/0/`, even for version 1. A namespace
prefix can be used. Foreign extension subtrees and descriptive metadata are
ignored; a nested element merely named `location` or `media` is not enough to
make it an audio reference. ASX vocabulary names are matched without letter
case, while the document must still be well-formed XML.

WPL `smartPlaylist` library queries are not supported. ASX `ENTRYREF`, `REPEAT`
and `EVENT` are reported and skipped with their subtrees. ASX `BASE` is refused
as a fatal unsupported construct. TagGroove does not fetch an external playlist,
run a script, expand a query against a library, or infer media paths from titles.
INI-style ASX is outside this reader’s XML subset.

## Try a small XSPF list

Place copied practice files `02 - Low Tide.wav` and `01 - Paper Boats.wav`
beside this independently authored example, saved as UTF-8 `practice.xspf`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<playlist version="1" xmlns="http://xspf.org/ns/0/">
  <trackList>
    <track><location>02%20-%20Low%20Tide.wav</location></track>
    <track><location>01%20-%20Paper%20Boats.wav</location></track>
  </trackList>
</playlist>
```

Load it and expect Low Tide before Paper Boats. Replacing the list clears
column sorting so the playlist’s load order is visible. Repeated references
to the same resolved path retain their first position. That does not establish
that every filesystem alias is the same recording. Missing entries are reported
while other resolvable entries can load; audio-format readback can report
additional errors after reference resolution.

## URI escaping differs from raw paths

XSPF locations are URI references. Percent escapes are decoded once. With both
files present beside the playlist:

| Reference | Resulting filename |
| --- | --- |
| XSPF `Song%20One.wav` | `Song One.wav` |
| XSPF `Song%2520One.wav` | `Song%20One.wav` |
| WPL `src="Song%20One.wav"` as a raw relative path | Try the literal `Song%20One.wav` first, using the existing path fallback rules |

WPL and ASX accept ordinary relative/absolute filesystem paths as well as local
`file:` URIs. Windows drive paths must be absolute and require Windows. Relative
Windows backslash separators are also tried on other desktop systems. Non-ASCII
paths are supported when the declared text encoding and URI escapes are valid.

Local `file:` URIs may have an empty authority or `localhost`. Other authorities,
raw UNC shares, non-file schemes and URI query/fragment suffixes are rejected
without a network fetch. Filesystem access to already mounted volumes remains
subject to the OS. XSPF local `xml:base` values are inherited and joined as URIs;
remote bases are unsupported rather than silently treated as local folders.
`xml:base` is not supported for WPL or ASX in this subset.

## Errors and encoding

A malformed document, wrong root/namespace, unsupported XSPF/ASX version or fatal
encoding error rejects the playlist before the file list is scanned or replaced.
DTD/entity declarations and unknown entities are refused. XML predefined escapes
such as `&amp;` and numeric character references are supported; no external schema,
stylesheet or entity is loaded.

UTF-8, BOM-marked UTF-16 and supported declared legacy encodings are decoded
strictly. Invalid bytes and conflicting declarations are errors. UTF-16 requires
a byte-order mark. Keep the original and convert a copy to UTF-8 if an older
XML file has no usable encoding declaration. These rules do not change legacy
M3U/PLS/CUE decoding.

The reader limits encoded and decoded input to 16 MiB, nesting to 64 elements,
parsing to 400,000 events, authored locations and authored/resolved base URIs
to 64 KiB, distinct retained base storage to 16 MiB, and the XML declaration
to 4 KiB. Exceeding a parser limit is a fatal error, not a partially accepted list.

Inherited and repeated identical XML bases share stored URI/diagnostic objects.
A resolved media URI above 64 KiB is skipped with an entry diagnostic before
checking the filesystem; other entries and alternate locations remain usable.

These XML formats are input-only. Creating a playlist with an `.xspf`, `.wpl` or
`.asx` suffix is refused before writing, including when that path already exists.
Choose `.m3u`, `.m3u8` or `.pls` for output; changing a suffix does not convert a
playlist’s contents.

## Related

- [Playlist loading, ordering and output scopes](playlists.md)
- [Playlist writing options](../reference/options/playlist.md)
- [Opening files](../getting-started/opening-files.md)
