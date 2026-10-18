# Remaining parity gaps versus Mp3tag 3.36.1

Baseline as of 2026-10-17 at `main` `9d3764b`; the Find entry is updated by the [2026-10-18 automation verification](../verification/automation-parity-2026-10-18.md). Destination memory and three previously implemented file/tag-panel capabilities are reconciled against approved main `696e419` in the [transfer-destination verification](../verification/transfer-destinations-2026-10-18.md). This register lists the known open items. It is built from audits and verification notes and is not an exhaustive proof. Everything the 2026-10-15 audit found that later batches closed has been removed; the closing evidence lives in the [verification notes](../verification/). The [parity ledger](../PARITY.md) summarises this list.

Method: the original audit compared every documentation page against the code. On 2026-10-17 the Windows binary's own tables were checked as well: script function names, placeholder names, field names, toolbar command ids and persisted setting names. Nothing was launched. "Unverified" means the behaviour is not documented and has not been compared with the original.

Legend: **Impact** H = daily tagging, M = regular, L = rare or legacy. **Size** S = hours, M = a day or two, L = a batch of its own.

## Open items

| Area | Item | Detail | Impact | Size |
|---|---|---|---|---|
| Toolbars | Toolbar command catalogue | The binary lists 75 toolbar command ids (`file_*`, `edit_*`, `cover_*`, `utils_*`, `help_*`, `view_*`, `convert_*`, `tagsources_*`). `TOOLBAR_COMMANDS` in `src/toolbar.ts` has 28. Not assignable: copy/move/rename/delete files, directory up, favorites, Open folder in file manager, copy format, select similar (and next), invert and unselect, cover add/delete/replace/save/adjust/description/cut/copy/paste, Utils commands, Help commands, customize columns and toolbars, random sort, save and open configuration folder, playlist selected and all-multiple, freedb and local freedb, quick tag source, properties | M | M |
| Toolbars | Movable toolbar bands | One fixed band per toolbar | L | M |
| Menus | File → Exit | No menu item (macOS quits from the app menu) | L | S |
| Menus | Alt+Enter | Opens Extended Tags; the original opens the Windows properties dialog (Windows-only) | L | S |
| Tag panel | Floating panel | In-window overlay, not a separate OS window | L | M |
| File table | Find runtime qualification | Find now uses the shared native Boost bridge (`src-tauri/src/find.rs`), covered by local cross-context regressions. Comparison against the original and native-input qualification on each desktop platform remain open ([automation verification](../verification/automation-parity-2026-10-18.md)) | L | S |
| File table | `columns.ini` import | Known keys only; the file layout is undocumented | L | S |
| Scripting | `$caps`, `$caps2`, `$caps3` word starts | Only whitespace starts a word, so `o'brien (live)` becomes `O'brien (live)`. Original set unverified | M | S |
| Scripting | Truth of non-boolean conditions | Literal `1` is false; functions are true when an argument field exists (`script.rs`). Unverified | M | M |
| Scripting | Errors inside a format string | `$div(x,0)`, bad `$char` or an unknown function abort the whole string. The original probably degrades inline | L | S |
| Filters | Mixed quoted and bare words | Rejected as a syntax error (`filter.rs`) | L | S |
| Filters | AND/OR precedence | AND binds tighter. Unverified | L | S |
| Actions | Adjust Cover "Original" format | TIFF and unknown formats require JPEG or PNG; original resampler, quality and rounding equivalence remain unverified ([script-export](../verification/script-export.md)) | L | S |
| Actions | Remove Fields Except | Unmapped frames (PRIV, GEOB, chapters) survive | L | M |
| Actions | Split Field | Keeps empty pieces (`a;;b`). Unverified | L | S |
| Actions | Convert Codepage default | Action group dialogs start at 1251 instead of the last choice (`src/action-ui.ts:285`) | L | S |
| Actions | `.mta` type IDs | Adjust Cover, Set Cover Properties and Generate Playlists export with TagGroove-only type IDs 1001 to 1003 (`crates/tag-core/src/actions.rs`); Mp3tag lists them as unknown actions | M | M |
| Actions | Mobile provider documents | File, cover-file and rename actions refused (`managed_actions.rs`) | L | L |
| Converters | Filename → Filename with a nonmatching explicit suffix | A suffix matching the source extension is deduplicated case-insensitively, but a different suffix still receives the original extension (`song.mp3` with old pattern `%1` and new pattern `%1.flac` becomes `song.flac.mp3`). Original behavior for nonmatching suffixes remains unverified | L | S |
| Converters | Text File → Tag record-count mismatch | Any mismatch refuses the import; the original applies records in order | L | M |
| File operations | Associated `.lrc` and `.cdg` files | Not renamed or moved with the audio file | L | M |
| Formats | TAK and OptimFROG | Implemented on a stand-in; only hand-built fixtures, no real-encoder files | L | S |
| Formats | Nero details | `udta/indx` kept but not shown or edited; covers always written to `covr`; a longer value for a Nero-held field in an MP4 file without chapters reaches only the iTunes tag ([formats-gaps](../verification/formats-gaps.md)) | L | S |
| Formats | Chapter rows | QuickTime chapter `href` URLs and ID3v2 CHAP artwork are not shown or edited; playlist operations on chapter rows are refused because the original's subsong path format is unknown ([chapter-fields](../verification/chapter-fields.md)) | L | M |
| Formats | ADTS and ID3 limits | ADTS after junk is found only in `.aac` files, within 64 KiB and after three frames. Compressed or encrypted ID3 frames cannot be kept in an unsynchronised tag or one with an extended header or footer, also for DSF and DSDIFF | L | M |
| Formats | MP4 cover type and description | Format limit | L | — |
| Formats | Reference files | No Mp3tag- or WMP-written fixtures | M | M |
| Covers | Resampler | Original resampler, quality and rounding equivalence unproven | L | M |
| Sources | Live-provider coverage | One default-HTTPS gnudb query passed; broader live read and artwork outcomes unverified ([gnudb](../verification/gnudb.md)) | M | S |
| Sources | Keyed services | AcoustID, Last.fm and fanart.tv need a build or user key | L | — |
| Sources | `.src` scripts | `[MinAppVersion*]` not enforced | L | S |
| Sources | Mobile | No keychain for the Discogs token | L | M |
| Formats | Native-field limits | Text `UNIQUEFILEID` values require an owner and an identifier of at most 64 bytes, one per owner; binary identifiers stay opaque. `UNSYNCEDLYRICS <description>` accepts one text per language and description. Original-executable syntax and grouping equivalence remain unverified ([formats-edge](../verification/formats-edge.md), [CRITICAL-MAPPING](../CRITICAL-MAPPING.md)) | L | M |
| Matroska | Edge cases | `TRACK` such as `3/12` is stored in `PART_NUMBER` without a `TOTAL_PARTS` split; a file without a SeekHead gets unindexed Tags, and a SeekHead with no spare room fails the write; the Segment Info title is not shown; chapters bound to other segments or ordered editions are kept but not interpreted ([formats-dsd-mkv](../verification/formats-dsd-mkv.md)) | L | M |
| Formats | ASF/WMA | Headers over 512 MiB are refused; every write rewrites the whole file | L | M |
| Formats | APE | A 131-byte APE tag whose items put "TAG" where ID3v1 starts is misread; header-less APEv2 tags are mishandled ([formats-edge](../verification/formats-edge.md)) | L | S |
| Formats | Tag kinds | ID3v2 on Ape/Mpc files is not displayed (it can be removed); WAV/AIFF ID3v2 keeps its existing revision; removing ID3v1 does not remove a dependent Lyrics3 tag | M | S |
| Tag preferences | Defaults | Write and remove defaults, cross-kind mirroring, ID3v1 truncation, multi-value joining and the encoding of existing Latin-1 frames in UTF-16 mode are not captured from the original ([tag-preferences](../verification/tag-preferences.md)) | M | S |
| Chapters | Whole-file commands | Remove tag and Strip chapters MP4 are refused on chapter and cue rows; `%_subsong%` base and Nero field-name case are unconfirmed | L | S |
| Utils | Optimize and Remove Unsupported Fields | Fragmented MP4 is refused and `stco` is not promoted to `co64`; FLAC with a leading ID3v2 tag is refused; Remove Unsupported Fields handles ID3v2 on MP3, AAC, AIFF and WAV only | L | M |
| Scripting | `%_md5audio%` | Computed by the export host only; empty for MP4 and Ogg and unavailable to filters (`src-tauri/src/file_hashes.rs`) | L | M |
| Scripting | `%_vbr%` and tag sizes | `%_vbr%` for Musepack and WavPack and tag sizes for chunk and box containers are unfinished ([script-export](../verification/script-export.md)) | L | S |
| Export | Dropped files | A `.mte` file dropped into the export folder while the Export dialog is open is retired if the dialog saves | L | S |
| Playlists | Reading and ordering | XSPF, WPL and ASX are not read; F5 does not re-read a loaded playlist; loading one clears the saved column sort and there is no "Playlist order" sort criterion | L | S |
| Covers | Linux drag-out | The GTK drag source writes `file://` URIs without percent-encoding, so temporary names are limited to ASCII letters, digits and `-_.()` | L | S |
| File list | Inline rename and order | Inline filename edits cannot move a file to another folder; manual order is not kept across sessions; save-on-move is not triggered by cell-editor Enter/Tab, Find, Select all or Select directory; Refresh reports an error for a file added individually and later renamed | L | M |
| File operations | Details | Play opens each file separately with no temporary playlist; the native folder dialog has no subfolders checkbox; macOS uses Ctrl+H and Ctrl+M for Add and Favorite directory because ⌘H and ⌘M are reserved | L | S |
| Tag panel | Details | The directory dropdown selects entries in the listed folder rather than changing the working directory; format-string fields are not implemented. Typed-directory Enter and automatic multi-selection defaults are already implemented (`tag-panel.ts:139,315`, `main.ts:1047`) | L | M |
| Toolbars | Custom toolbar | Buttons are text labels with no symbol choice and none for individual tag sources; Mp3tag `toolbar.ini` and `toolbar_custom.ini` are not imported | L | M |
| Tools | Default tool | Selecting "No action" for the default tool still opens Extended Tags on a file-list double-click; tools receive path placeholders only, `.lnk` and `.bat` forms are not special-cased, and a per-file launch is capped at 100 files | L | S |
| Sources | freedb | "Current list of freedb servers" restores two built-in entries instead of fetching a current list; artist/album and artwork search is limited to `gnudb.org` hosts and local lookups are exact | M | M |
| Sources | Review step | Utils are buttons, not a menu; "Suggest sorting" entries are one-shot actions; the exact Mp3tag behaviour of Ctrl+Shift+I is undocumented | L | S |
| Rename | Reserved characters | Mp3tag's handling of `: * ? " < > |` in tag values is undocumented and TagGroove replaces them with `_`; Replace and Regex replace on `_FILENAME` can still produce invalid Windows characters | L | S |
| Converters | Path edge cases | Filename → Filename resolves relative targets against the file's folder; two partitions writing one output file are refused; `/` as a folder marker in the old pattern is unverified | L | S |
| Actions | Plan clashes and codepages | Filename → Filename refuses the whole reviewed plan when one destination clashes; Convert Codepage has no EBCDIC or ISO-2022 pages; Import Text File has no explicit codepage choice and reads a UTF-8-valid ANSI file as UTF-8; the quality preset numbers and default field list are TagGroove choices | L | M |
| Configuration | Storage | Quick-action memories and field-list and format-string histories live in WebView storage, so configuration export does not include them; Mp3tag configuration ZIPs cannot be imported | M | M |
| Updates | Install | The update check reads GitHub releases and offers no download or install | L | M |
| General | Auto-completion | Fills from the first matching drop-down value only, not in multiline fields, with no filtered suggestions while typing | L | S |
| Library | Locale | Locale-formatted technical fields such as `%_file_mod_date%` are stored as formatted and a locale change does not refresh them | L | S |
| Undo | External changes | Outside Unix, external changes are detected by size and mtime only | L | S |
| Launch | Open With | `.mkv` and `.webm` appear in Finder's Open With only when the system or another app declares their types | L | — |
| Settings | UI languages | English, Dutch and German bundled; the original ships 43 | M | L |
| Settings | Networking | Proxy credentials, SOCKS and PAC | L | S |
| Settings | Setting names in the binary with no confirmed equivalent | `ConvFTFShowDlg`, `ConvFTTShowDlg`, `ConvTFFShowDlg`, `ConvTTFShowDlg`, `ConvReplaceShowDlg` (per-converter "show dialog"), `ToolOverwriteDefault`, `CoverPopup`, `HideID3Bar`, `ContextMenuDisabled` (Windows shell entry). Feature meaning for the first four groups not documented | L | S |
| Settings | `ID3LeadingZeros` | The documented leading-zero option belongs to the numbering wizard; whether the binary also has a global switch is unknown | L | S |
| Undo | Persistence | History is not kept across sessions | L | L |
| Undo | Non-cloning volumes | 2 GiB full-size cap; the first unpadded save counts in full | L | M |
| CLI | `open -a … --args` | macOS drops arguments to a running app | L | — |
| CLI | File associations | Not registered on Windows or Linux | L | S |
| Platforms | Signing | Windows unsigned, macOS ad-hoc, iOS simulator-only | M | M |
| Platforms | iOS documents | iOS has no provider picker, document access or save-back (`runtime_platform()` in `src-tauri/src/lib.rs` enables document access for Android only) | L | L |
| Tag panel | Extended Tags save | "Keep changes" only stages the fields and needs a later Save tags; immediate-save mode applies only when moving between files ([keyboard-ui](../verification/keyboard-ui.md)) | L | S |
| Platforms | Mobile provider documents | Reads and reloads of Android provider documents use the default read path, so MPEG/APE read precedence and user field mappings do not apply (`src-tauri/src/documents/engine.rs`); "Save a copy" writes fields only, so pending artwork is not copied; playlist loading and creation, CSV/export operations and Undo/Redo are disabled for provider documents (`src/main.ts`, [playlists-export](../verification/playlists-export.md)) | L | M |
| Platforms | Mobile | Directory trees, cloud providers and real-device qualification | L | L |
| Distribution | Audit | Final dependency license audit; reproducible builds | M | M |
| Performance | Comparison | No same-corpus comparison with the original; no Windows or Linux numbers; APE, RIFF, Ogg and other containers rewrite whole files | M | M |
| Native UI proof | Input | The headless suites use synthetic DOM events, so OS input, IME and hit-testing are untested; native screen-reader qualification open | M | L |
| All areas | Differential proof | No area has been compared run for run with the original (field ordering and mapping, padding and reset edge cases, export sort, locale and encoding, query corpus) | H | L |

## Out of scope

| Item | Reason |
|---|---|
| Windows Explorer shell extension, context menu, Ctrl-drag from Explorer | Windows shell integration; TagGroove uses the CLI forms and single-instance forwarding |
| Help → Donation and the vendor website and forum targets | Belong to the original vendor |
| Bundled Mp3tag `.src`/`.inc` scripts and the web-sources download service | Proprietary content |
| Mp3tag language files and the Appearance → Symbols icon set | Proprietary assets |
| Audio-CD lookup for freedb | Optical-drive I/O per platform |
| Discogs OAuth with an embedded consumer secret | An open-source app cannot keep the secret |
| ANSI `.m3u` on macOS and Linux | No system ANSI code page; TagGroove writes Windows-1252 when the text round-trips and falls back to UTF-8 otherwise |
| About → portable/Store install type | Windows packaging detail |

## Binary tables checked (2026-10-17)

Source: strings from `Mp3tag.exe`, compared with the code.

- Script functions: all 52 names in the function table exist in `crates/tag-core/src/script.rs`.
- Placeholders: every name in the placeholder table is produced (`src-tauri/src/evaluation.rs`, `crates/tag-core/src/technical.rs`, `mp4_chapters.rs`, `media_info.rs`).
- Field names: the 99 names in the Fields menu table are in the mapping registry.
- Toolbar command ids: 75 in the binary against 28 in TagGroove (first row above).
- Persisted setting names: about 60 checked; the unmatched ones are listed under Settings and File operations.

## 2026-10-18 ledger reconciliation

- Filename → Filename already deduplicates an explicitly included suffix when it matches the source extension, ignoring case (`crates/tag-core/src/converters.rs`, `ends_with_extension`). This corrects the earlier broad missing-feature claim. A nonmatching suffix still gets the source extension appended; that narrower case remains in the register, with original runtime comparison open.
- Typed-directory Enter and automatic defaults for multi-selection already exist and are connected. The remaining Tag Panel row keeps the separate dropdown-selection and format-string-field limits.
- Copy and Move now retain separate last-successful destinations in `directories.json`; cancellation and wholly failed operations retain the previous choice. See [the bounded verification receipt](../verification/transfer-destinations-2026-10-18.md). Native dialog interaction and original timing are not claimed from the unit tests.
