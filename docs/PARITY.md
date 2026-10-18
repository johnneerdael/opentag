# Mp3tag 3.36.1 parity gaps

This ledger lists the known gaps: what is missing, partial or unverified. It is built from audits and verification notes and is not an exhaustive proof; the [verification notes](verification/) and [feature inventory](FEATURE-INVENTORY.md) hold the detail. No area is certified as exact parity, because none has been compared run for run against the original program.

Reference target: `../Mp3tag.exe`, SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`. Current as of 2026-10-17 at `main` `9d3764b`. The item-level list with impact and size is the [remaining-gaps register](research/remaining-gaps.md).

| Area | Remaining gap |
| --- | --- |
| Toolbars | The original registers 75 toolbar command ids. TagGroove's custom toolbar offers 28 commands plus tools and action groups, so about 47 menu commands (copy/move/rename/delete files, most cover commands, selection commands, Utils, Help, file-list and configuration commands) cannot be placed on a toolbar. Movable toolbar bands are missing |
| Menus and keyboard | No File → Exit item (the macOS app menu quits). Alt+Enter opens Extended Tags instead of the Windows properties dialog |
| Extended Tags | "Keep changes" only stages the fields and needs a later Save tags; the configured immediate-save mode applies only when moving between files ([keyboard-ui](verification/keyboard-ui.md)) |
| Tag panel | The floating panel is an in-window overlay, not a separate OS window |
| File table | Find uses JavaScript `RegExp`, not the Boost bridge. Mp3tag `columns.ini` import covers known keys only because the layout is undocumented |
| Scripting | Unverified against the original: `$caps` word-start characters, truth of non-boolean conditions, and errors inside a format string (TagGroove aborts the whole string). `%_md5audio%` works in exports only and is empty for MP4 and Ogg; tag-size placeholders are absent for WAV, AIFF, MP4, Ogg, Matroska, ASF and DSD |
| Filters | A plain search that mixes quoted and bare words is rejected. AND/OR precedence and locale behaviour are unverified |
| Actions | Adjust Cover "Original" requires JPEG or PNG for TIFF and unknown formats. Remove Fields Except leaves unmapped frames (PRIV, GEOB). Split Field keeps empty pieces (unverified). Convert Codepage starts at 1251 in action group dialogs instead of the last choice. File, cover-file and rename actions on mobile provider documents. `.mta` export of Adjust Cover, Set Cover Properties and Generate Playlists uses TagGroove-only type IDs 1001 to 1003, which Mp3tag lists as unknown actions |
| Converters | Filename → Filename appends the extension again when the new pattern contains one. Text File → Tag refuses any record-count mismatch by design |
| File operations | Copy and Move do not reopen at the last destination folder. Associated `.lrc` and `.cdg` files do not follow a rename |
| Formats | TAK and OptimFROG run on a stand-in and are tested only with hand-built headers. Nero `udta/indx` is kept but not shown or edited, and Nero covers are always written to `covr`. A longer value for a Nero-held field in an MP4 file without chapters only reaches the iTunes tag. ADTS after junk is found only in `.aac` files, within 64 KiB and after three frames. Compressed or encrypted ID3 frames cannot be kept in an unsynchronised tag or one with an extended header or footer (also DSF and DSDIFF). QuickTime chapter URLs and ID3v2 CHAP artwork are not shown or edited, and playlist operations on chapter rows are refused because the original's subsong path format is unknown ([chapter-fields](verification/chapter-fields.md)). MP4 cover type and description (format limit). No Mp3tag-, WMP- or Nero-written fixtures |
| Covers | Original resampler, quality and rounding equivalence |
| Sources | Live-provider checks are bounded (one default-HTTPS gnudb query passed). Keyed services need a build or user key. `[MinAppVersion*]` is not enforced in `.src` scripts. No keychain on mobile. Audio-CD lookup, Discogs OAuth and the web-sources download service are out of scope |
| Settings | Only English, Dutch and German UI catalogs (the original ships 43 languages). Proxy credentials, SOCKS and PAC. Setting names found in the binary with no confirmed TagGroove equivalent: per-converter "show dialog" flags, `ToolOverwriteDefault`, `CoverPopup`, `HideID3Bar`, `ContextMenuDisabled` (Windows shell entry) |
| Undo/redo | History is not kept across sessions. Non-cloning volumes keep the 2 GiB full-size cap, and the first unpadded save counts in full |
| CLI and OS integration | `open -a … --args` cannot reach a running app on macOS. No Windows or Linux file associations. The Windows shell extension is out of scope |
| Smaller limits | Further limits are listed in the [register](research/remaining-gaps.md): Matroska and ASF edge cases, Utils refusals, playlist formats (XSPF, WPL, ASX), freedb server list, "No action" default tool, tag-panel directory switcher, toolbar file import, configuration storage, and others. Header-less APEv2 and the APE/ID3v1 `TAG` collision already have implemented fixes and passing synthetic regressions ([reconciliation](verification/ape-ledger-reconciliation.md)); original-executable comparison remains open |
| Platforms and distribution | Windows builds are unsigned, macOS builds are ad-hoc signed, iOS is simulator-only and has no document access or save-back yet. Mobile needs real-device qualification, directory trees and cloud providers. Android provider documents ignore MPEG/APE read precedence and user field mappings on read and reload, and "Save a copy" does not copy pending artwork. Provider documents have no playlist loading or creation, no CSV/export operations and no Undo/Redo. Final dependency license audit and reproducible builds |
| Performance | No same-corpus comparison with the original, no Windows or Linux numbers, and APE, RIFF, Ogg and other containers still rewrite whole files |
| Native UI proof | The headless suites use synthetic DOM events, so OS input, IME and hit-testing are untested. Native screen-reader qualification is open |

Differential evidence against the original (field ordering and mapping, padding and reset edge cases, export sort, locale and encoding, query corpus) is open for every area.

## Beyond Mp3tag

- Builds for Windows, macOS, Linux, Android and iOS, with phone and tablet layouts.
- Extra tag sources: iTunes, Deezer, LRCLIB lyrics, Last.fm, fanart.tv and AcoustID with fingerprints computed inside the app (no external `fpcalc`).
- A Tools menu, Load configuration, Previous/Next cover and a larger cover viewer.
- DFF/DSDIFF and AIFF text chunks beyond the documented format list.
- Language catalogs are plain JSON files, so users can add a language or override wording without a rebuild. The UI language switches live.
- Undo accounting is clone-aware on APFS, so a 100-file FLAC save stays one undoable step. FLAC and ID3v2/MP4 edits are written in place inside existing padding.
- Configuration backup and restore as one validated JSON bundle.
- Open source (MIT) with a published user guide at <https://taggroove.neerdael.nl/>.
