# Source-bundle installation and remaining migration work

This is the first R02–R04 delivery from the 2026-10-18 Mp3tag parity roadmap.
It completes bounded folder/ZIP installation and records qualification limits.
It does not complete setup migration or certify either provider's tagging journey.

## Evidence boundary

The reference is the supplied Mp3tag 3.36.1 x64 distribution. Leanrea reused
its PE inventory and extracted strings under the local temporary
`re/mp3tag-3.36.1-roadmap` extraction. The shipped `Mp3tagVersion.txt` entries
at lines 8–10 (2026-08-21) establish a full-cover query-results button,
`%_coverfullurl%` in `ParserScriptIndex`, and an album-parser preview URL.
These are static shipped evidence; no Windows execution or exact original
interaction was captured. The current result mapper recognizes `_coverurl`
and `_preview`; full-cover and album-preview behavior remain R04 work.

The independently authored automated fixtures exercise complete folders and
nested ZIPs, preserved saved values/unrelated files, invalid second files,
missing includes/settings, explicit-file incremental updates, duplicate case variants, traversal/drive paths,
symbolic links, bounded payloads, case-preserving updates and recovery after
an interrupted directory swap. No vendor/community script is included in the
repository. `source-bundle-receipt.json` records private-file hashes and counts
without script text or configured credentials.

## Private bundle qualification

Both supplied local folders were read without modifying the originals. The
installer wrote only disposable temporary destinations, and the real loader,
schema reader and parser compiler ran there.

| Bundle | Intake/loader result | Parser compilation | Dated live result (2026-10-18) |
| --- | --- | --- | --- |
| Traxsource by Jordi and Claude v3.0 | 8 sources, 4 includes, 1 schema; all dependencies resolve | 14 index/album blocks compile | One artist search for Kerri Chandler received HTTP 403 during page download; the parser was not reached. |
| SoundCloud WS v2.28 by Jordi and Claude | 1 source, 1 include, 1 schema; all dependencies resolve | 2 index/album blocks compile | Search Bonobo / Kerala returned 11 index rows and parsed artist metadata plus 1 cover. The backend release had an empty album title and scalar TITLE with zero explicit tracks. The existing review expands album-wide outputs to the selected files; an independent DOM regression verifies scalar TITLE reaches review and Apply for one or two selected files. |

The SoundCloud check was repeated once to inspect substantive result counts;
requests were bounded to 45 seconds per run. No debug traces, raw script text,
response bodies or credentials were retained in the repository. No anti-bot
bypass was attempted for Traxsource. A provider refusal is not evidence of a
parser defect. The zero-track backend result alone is not a defect: `showReview` explicitly
creates a slot per selected file for album-wide web-source fields. For a
single-track source, select only the intended local file; selecting several
files applies scalar TITLE to every matched file. The independent regression
checks this existing behavior, not original Mp3tag semantics. Full native
review/save/reload/undo remains unqualified for the private sources.

To repeat local preflight without network access:

```sh
OPENTAG_SOURCE_BUNDLE=/path/to/user-owned-download cargo test -p opentag --lib qualify_private_bundle --locked -- --ignored --nocapture
```

`compat::community_script` remains the existing opt-in live/recorded-response
runner. Keep provider credentials and downloaded responses outside the public
repository. Parser compilation, recorded response execution, provider requests,
review/apply, save/reload and undo are separate acceptance results.

## Remaining R03 acceptance

The UI reaches folder and files/ZIP pickers through the existing install menu.
DOM tests check both routes and cancellation; Rust tests check the production
installer. Native file-picker interaction and released desktop behavior still
require platform CI/native evidence. The backup is a complete last-install
snapshot beside `sources`; manual rollback instructions are in the user guide.
The installer is not a general Mp3tag configuration ZIP importer.

## R02 inventory and next acceptance

Reuse these existing entry points rather than introducing parallel parsers:

| Profile component | Current entry point | Remaining guided-migration work |
| --- | --- | --- |
| Columns | `src-tauri/src/mp3tag_columns.rs` / `import_mp3tag_columns` | Inventory a selected profile, preview imported/skipped columns and preserve user-authored labels. |
| Tag panel | `src-tauri/src/mp3tag_tag_panel.rs` / `import_mp3tag_tag_panel` | Show partial/unsupported controls and combine with the columns report. |
| Action groups | `import_action_group(s)` in `src-tauri/src/lib.rs`, existing MTA diagnostic reports | Wrap imports without losing unsupported-action diagnostics; automation lane owns serialization changes. |
| Export templates | `import_export_configuration` and `src-tauri/src/exports.rs` | Preview named MTE templates and preserve existing templates on collisions. |
| Sources | Existing list/settings/parser/review plus this installer | Install complete bundles once, then report provider-specific qualification separately. |
| Configuration restore | `src-tauri/src/utilities.rs` `collect`/`restore` | Existing seven-file JSON allow-list covers settings/actions/exports/columns/tag-panel/directories/lists. It does not include WebView histories/workspace or a general Mp3tag folder/ZIP importer. |

A subsequent R02 change must provide one inventory/preview/report over a
populated profile, leave the original untouched, exercise restart persistence,
and restore all promised state. Histories and workspace data need an explicit
portable schema before they can be counted as migrated. Avoid storing provider
secrets in general-purpose backups.

## R04 next acceptance

1. Complete the native single-file SoundCloud review/save/reload/undo journey;
   keep the existing album-wide fallback and explain multi-selection behavior.
2. Qualify recorded input parsing for direct-ID and search flows for both
   bundles; preserve diagnostic file/line provenance and configurable values.
3. Establish full-cover and album-preview semantics from original/reference
   evidence, then expose distinct full-image and release-preview actions.
4. Complete settings changes, multi-track matching, selected-field/artwork
   import, save/reload and undo on disposable audio files. Capture native UI
   and separate dated live-provider evidence. Traxsource HTTP 403 remains a
   provider-access limitation until a permitted request succeeds.
5. Express capability limits without comparing Mp3tag `MinAppVersionWin` or
   `MinAppVersionMac` with TagGroove's unrelated application version.

## Review follow-up: folder opening and existing metadata

Two Codex findings were reproduced with failing regressions. The old Open sources folder action could recreate an empty active directory during a staged swap; folder preparation now takes the same installer lock and restores a marked interrupted backup before creating anything. The concurrency test holds an installation swap open and verifies folder preparation waits without recreating its destination. A second test covers recovery through the folder-opening path.

Existing hidden directories were previously rejected by incoming-source name validation. Updates now preserve existing unrelated directories, including nested `.git` history, without relaxing incoming source-name/ZIP restrictions. Existing metadata traversal is bounded to64 levels; new source bundles retain the existing source nesting limit, and staging byte/entry limits still apply.

After the correction, all61 web-source tests pass with2 opt-in tests ignored, strict local Clippy passes, and catalog extraction consistency/diff checks pass. The initial batch's full frontend/workspace checks above remain historical; final-head trusted cross-platform CI and fresh clean Codex review are still required.
