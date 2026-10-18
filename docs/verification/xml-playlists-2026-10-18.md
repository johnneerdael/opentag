# R10 local XML playlist readers — 18 October 2026

Implement a bounded local-file import subset for XSPF version 1, static WPL and
well-formed XML ASX version 3. Start independently from branding commit
`c1375a6ccf24e728f3ac29dfd442d4bf1071d47b`; the coordinating owner reported newer
branding head `864b1448f7c3e4ea95a16aa9f5da51ea7e25a08c` during this work. Final
publication requires a clean coordinated branding sync and fresh validation.
The frozen sidecar and toolbar deliveries in PRs #54 and #55 are not dependencies.

## Primary format references and scope choices

[XSPF version 1](https://www.xspf.org/spec) defines namespace-aware ordered
tracks with alternative locations and XML Base resolution. Apply local
[XML Base URI inheritance](https://www.w3.org/TR/xmlbase/) using URI joining;
never silently reinterpret a remote base as a local folder. Retain only the
first resolvable local alternative for one logical track.

Microsoft documents WPL [media sources](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/wmp/media-element)
and [static versus smart sequences](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/wmp/seq-element).
Read static `media` children only; report `smartPlaylist` queries as unsupported.

Microsoft’s [ASX REF reference](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/wmp/ref-element)
defines multiple REF elements within an ENTRY as rollover alternatives. Select
one local file per ENTRY. Do not recursively process
[ENTRYREF](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/wmp/entryref-element),
REPEAT or EVENT; report and skip those subtrees. Refuse ASX BASE explicitly.
Metadata and foreign-extension subtrees do not become media references.

These are independently authored reader rules based on primary references,
not an original Mp3tag execution comparison or full Windows Media Player emulation.

## One resolution and diagnostic pipeline

Keep public legacy PlaylistKind/RawEntry/flat parser APIs unchanged. Add XML
recognition inside the loader and route ordered logical entries through the
existing resolution/error pipeline, generalized to ordered alternative groups.
Preserve first-occurrence deduplication and line-indexed owned diagnostics.
Legacy single-location wrappers retain existing candidate behavior and English
messages. Cue-sheet row behavior is unchanged.

XSPF URI references decode percent escapes once. WPL/ASX raw filesystem paths
retain existing literal-first path fallback behavior; explicit URIs use strict
URI resolution. Permit empty/localhost file-URI authorities, local absolute
paths and native absolute Windows drives. Refuse other schemes/authorities,
raw UNC paths, URI queries/fragments and drive-relative Windows paths. Filesystem
access to already mounted volumes remains OS behavior; no network client,
external entity resolver, stylesheet or schema loader is introduced.

Use already locked `quick-xml` 0.42.0 and `url` 2.5.8 as direct tag-core
dependencies without upgrading packages. The
[namespace-aware streaming parser](https://docs.rs/quick-xml/0.42.0/quick_xml/reader/struct.NsReader.html)
runs over strictly decoded Unicode, with only predefined/numeric XML escapes.
Reject DTDs and unknown entities, malformed/wrong-root documents, invalid or
conflicting encodings and limits before returning paths. The parser’s
[encoding documentation](https://docs.rs/quick-xml/0.42.0/quick_xml/index.html)
distinguishes native event parsing from decoding; existing encoding_rs handles
UTF-8, BOM UTF-16 and supported declared legacy encodings without replacement.

Limits: 16 MiB encoded and decoded data, 64 nested elements, 400,000 events,
64 KiB authored locations/authored bases/resolved bases and 4 KiB declarations.
Distinct retained base storage is limited to 16 MiB, counting the intern key,
owned payload and fixed object allowance once per unique stored object. Inherited
and repeated identical bases use shared immutable handles without repeat charges.
Invalid bases retain compact line diagnostics rather than authored URI copies.
Resolved media URIs above 64 KiB are per-entry failures before filesystem probing.

Modest red controls demonstrated 128 base URL allocations for 128 inherited
locations, repeated large invalid-base diagnostics, and missing authored/resolved
base caps. The fixed controls assert one shared URL allocation, compact errors,
controlled distinct-base budget exhaustion at 8 KiB, repeated/inherited bases
fitting that budget, and no filesystem probe for an oversized joined location.
No large-memory or OOM execution was used. Other controlled negative fixtures
cover input, nesting, events, locations and character-reference refusal.

An independent review also identified a full inherited namespace URI copy on
both Start and End events, including ignored foreign subtrees. A modest 8 KiB
foreign namespace with 128 short empty prefixed elements supplies 256 such
events. The regression first failed with owned namespace containers; it now
checks that the exact event-loop boundary carries at most a one-byte
classification with no URI payload, while the foreign subtree stays ignored.
Classify unbound, XSPF and other namespaces by borrowing the resolver value;
long foreign URIs fail the XSPF length comparison immediately. Keep unknown
prefix errors and the existing attribute namespace validation. This removes
per-event URI copying without applying the base-storage budget to inherited
namespace data or changing XML format rules.

## Application and packaging integration

Recognize the extensions in dropped/queued file-open paths and the existing
Load playlist picker. Execute the actual main loadPaths function in frontend
regressions: central read_playlist expansion precedes ordered scan; fatal XML
failure keeps the old list unscanned and unchanged; replacing a sorted list
clears column sorting after loading. Tauri IPC tests use the existing generated
valid WAV fixture producer, resolve all three formats, then read the files
through the real scan command in playlist order.

Append only xspf/wpl/asx to the existing macOS playlist extension declaration in
`tauri.macos.conf.json`, preserving its name, Viewer role and Alternate rank.
The locked tauri-utils 2.10.1 extension-to-UTI mapping has no inference for these
three suffixes, so the extension-only declaration remains effective. Preserve
the derived association regression. No Windows/Linux association, installer,
signing, upgrade identity or installed Finder registration outcome is claimed.

Refuse XML output suffixes in the existing playlist writer/create flow before
writing, including an occupied destination. Preserve supported M3U/M3U8/PLS and
unknown-suffix legacy output behavior. Read-only input support must not result
in M3U bytes silently written with an XML suffix.

## Verification state and remaining scope

Focused local evidence covers XML namespaces and injection, order/alternatives,
percent-literal versus URI decoding, Unicode, XML Base, native/local path rules,
unsupported dynamic instructions, strict encodings, fatal parse handling,
controlled limits, indexed diagnostics, real WAV IPC/scan and output refusal.
Full local qualification before the namespace-copy follow-up, after normal
merge of branding revision
`36d7dea4e7b930d3a7d98facaca8498d0ef7c8de`:

- `npm test` with the pinned offline audio sources and shared Cargo target:
  566 frontend tests and 1,106 Rust tests passed; 6 existing Rust tests ignored.
- XML integration: 16 passed. Playlist unit/resolver controls: 19 passed,
  including a focused rerun using platform-native temporary-directory file URIs.
- Actual XML load/launch/picker and updated branding fixture checks: 6 passed.
- `npm run build`, strict MkDocs guide, catalog extraction check, formatting
  check and strict workspace/all-target Clippy passed.
- Branding verifier: 249 supplied files and 78 integrated copies verified.

After the namespace-copy follow-up, the complete `tag-core` suite passed
664 tests with no ignored tests, including the new bounded namespace control
and all 16 XML integration cases. All 5 actual XML UI tests, production build,
strict workspace/all-target Clippy, catalog extraction and formatting checks
also passed. The previous full-workspace result remains recorded separately;
it was not rerun for this private parser classification change.

The routine large-bundle build notice and shared Cargo registry auto-clean
permission notice did not fail these checks. No registry or build policy was
changed. These results are local qualification, not protected CI, a GitHub code
review, deployment or native OS/runtime proof. Draft publication remains subject
to the coordinator's fresh branding-head currency/ancestry check.

Native mouse/key/IME/accessibility, screenshots, installed OS file opening,
Windows/Linux filesystems and original differential behavior remain unqualified.
The earlier browser URL-policy denial is respected; no workaround is attempted.
R10 still retains playlist refresh/reload qualification, externally added export
template preservation, and export encoding/locale/sort qualification. Dynamic
WPL, ASX active/nested features and XML playlist writing are unsupported subsets,
not silently counted as achieved parity. Area 4 remains open.
