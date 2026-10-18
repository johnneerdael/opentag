# R02 populated-profile migration: discovery and next implementation

Status: implementation resumed after the review-correction batch. The original
discovery below remains the design record; production modules, authored fixtures
and recovery controls now exist. Current work continues on `codex/parity-profile-migration` in the existing `parity-migration` worktree.
The branch starts at origin/main `4a1a95ff4eddb9da7492aaace68197c4f81347d0`.
PR #47 branding head `c1375a6ccf24e728f3ac29dfd442d4bf1071d47b` was normally
merged after checkpoint `2e6a8e7`. After the complete local profile checks,
head `36d7dea4e7b930d3a7d98facaca8498d0ef7c8de` was normally merged without
conflicts; its delta was limited to the Windows NSIS fixture/tests and README.
The root coordinator owns PR #52 and its later
review fixes in another worktree; do not alter that branch. New remote pushes,
PR creation and review requests are held until the root ancestry/currency check.

## Authorized outcome and ownership

Wrap existing importers in a real desktop Mp3tag folder/ZIP inventory →
preview/report → supported import → restart persistence → restore workflow.
Leave originals untouched. Use independently authored populated fixtures; no
private scripts, proprietary data or credentials enter the repository. Do not
infer undocumented global/binary Mp3tag configuration syntax. Unsupported
components stay visible with a reason, and do not count as migrated.

Root approved isolated `src/profile-migration.ts` and
`src-tauri/src/profile_migration.rs`, tests/fixtures, guide/verification docs,
and additive catalogs. Shared edits are limited to: main.ts import/new UI host,
profile operation entries in Operation union/File menu, handlers beside
`config-load` (baseline ~1913); lib.rs module declaration and new command
registrations beside utilities. Coordinate exact lines again before editing.
Root owns docs/PARITY.md and docs/research/remaining-gaps.md. Other lanes own
main.saveCopy/lib490 and main.rename1492+/FileOps/options files paths.

## Confirmed existing entry points

- `mp3tag_columns::parse_diagnostic(&str)` yields `ColumnImportReport` with
  layout, skipped strings and source-separated diagnostics. Decode bytes via
  `mp3tag_columns::decode`; reuse column validation/save.
- `mp3tag_tag_panel::parse_diagnostic(&str)` yields `TagPanelImport` with panel
  layout and skipped diagnostics. The importer reads `usrfields.ini` or saved
  panel files. It reports old per-field custom values instead of importing
  those; preserve this limit in the combined report.
- `action_host::import_group_diagnostic(path, legacy_encoding)` yields an MTA
  import report with unresolved actions preserved. `load_library` returns
  existing groups or example defaults; `save_library` checks unique names.
  Reuse group identity/diagnostics. Automation lane owns MTA semantics.
- `exports::import(path, legacy_encoding)` decodes an MTE into an owned
  Configuration (new UUID, name from filename, `file: None`). `exports::load`
  reads actual files from sibling `export/`; `exports::save` writes templates
  and metadata. A transaction must stage/snapshot both exports.json and the
  physical export directory, not only JSON. Read the rest of load/save before
  implementing: loading may offer bundled templates and writing may remove
  renamed/deleted template paths.
- `utilities::collect`/`restore` handle a seven-file JSON allow-list:
  settings/actions/exports/columns/tag-panel/directories/lists. Restore
  validates all files first but writes them individually; it is not a complete
  rollback transaction. It omits physical export templates, WebView histories,
  workspace/localStorage and source bundles. Keep ordinary configuration
  Save/Load behavior unchanged unless a small explicit factoring is justified.
- `crate::io` serializes native file operations through FILE_OPERATIONS.
  settings::save_settings also uses it. Put migration commit/restore inside
  this boundary so concurrent configuration writes cannot overwrite snapshots.
- Existing config-load handler flushes helperLists, then pauses it before
  native restore and window reload. Reuse that lifecycle: otherwise delayed
  list writes can overwrite restored state.
- Application-owned frontend state uses `opentag.*` localStorage keys across
  main layout, helper histories, language/theme, numbering, quick actions and
  tag-source choices. Capture a bounded explicit owned-state map at import;
  restore those keys on rollback/restart and preserve unrelated storage.
  Inspect remaining key definitions before deciding the exact schema.

## Proposed bounded flow

1. User chooses Folder or ZIP. Intake is read-only, bounded, rejects symlinks,
   traversal/absolute/drive paths and case-duplicate entries. Match ZIP
   dependency feature/version to PR #52 (`zip` 2.4, default features disabled,
   deflate only; resolved 2.6.1) for merge-compatible Cargo.toml/lock additions.
   Do not copy the entire source installer as another general extractor.
2. Inventory every bounded input file. Recognize columns.ini/column presets,
   usrfields.ini/panel presets, MTA action groups and MTE templates. Report
   unknown global/binary settings, source collections and other files
   explicitly. Sources should point to the separate complete-bundle installer;
   this milestone does not claim a general vendor-backup restore.
3. Preview supported and partial items with existing diagnostics. Select at
   most one column layout and one panel layout; select named groups/templates.
   Keep or replace conflicting names only through an explicit user choice.
   Show unsupported entries and selected counts before Apply.
4. Bind Apply to exact preview inputs (SHA receipt) and revalidate current
   target state; reject changed inputs/targets rather than applying unseen
   data. Stage supported native JSON plus export files using existing parsers,
   validators and save methods. Never run imported actions/templates while
   migrating setup.
5. Before writing, persist a private pre-import snapshot and recovery journal.
   Snapshot every promised native component plus export files and owned WebView
   state. Credentials remain excluded and untouched. Commit supported changes
   as one recoverable operation; roll back all touched files on any failure.
   Define interrupted-commit recovery before wiring product Apply.
6. Persist a migration report and explicit Restore setup action across restart.
   Restore configuration state, including absent/new files and frontend owned
   state, separately from audio Undo. Native settings caches must reload and
   helper-list writes remain paused through restore/window reload.

## Design questions to settle before implementation

- Exact snapshot scope and storage schema: full promised native seven-file
  state plus export tree and app-owned frontend values; source/credential
  files are untouched, not implicitly restored. Avoid claiming all app state
  if some component is intentionally outside the snapshot.
- Crash-safe transaction/journal phases, cleanup and recovery location; handle
  rollback failure by retaining a named complete snapshot, never deleting it.
- Preview receipt lifetime/input hash and target-conflict checks; do not accept
  arbitrary frontend-provided serialized native configurations as trusted plans.
- Whether unsupported MTA groups default unchecked while retaining unresolved
  actions, and exact per-item collision reporting/selection semantics.
- Legacy encoding options for MTA/MTE must be explicit and shared with current
  importers rather than silently forcing UTF-8 or guessed code pages.

## Acceptance and remaining scope

Use an independently authored populated profile with multiple columns/panels,
partial unsupported controls, valid plus unresolved MTA groups, several MTEs,
name collisions, unknown global/binary files and complete source collections
reported separately. Cover folder/ZIP, cancellation, unchanged originals,
preview/apply changes, partial diagnostics, rejection without partial writes,
restart readback and exact configuration restoration (including files absent
before import and frontend histories/workspace). Test recovery after an
interrupted transaction and preserve credentials separately.

Run npm test, npm run build, relevant native suites, strict guide build,
formatter/extraction/diff checks, then a clean feature commit/PR and one genuine
Codex review request on its final head. Root owns review/CI dispatch/merges.
Use only `/tmp/opentag-gh-owner.py` for GitHub auth/push. Local builds use
CARGO_TARGET_DIR=/tmp/taggroove-target,
OPENTAG_AUDIO_SOURCE_DIR=/tmp/taggroove-parity-audio-sources,
AR=/usr/bin/ar and RANLIB=/usr/bin/ranlib. Existing strict guide interpreter:
`/tmp/opentag-pr45-docs-venv/bin/python`.

R02 remains open for undocumented global settings and original-populated-profile
comparisons. R04 remains next: native private-source review/save/reload/undo,
recorded/direct-ID/settings/matching journeys, and distinct full-cover plus
album-preview evidence/implementation. Do not count installation/compilation
or partial live parsing as those completed outcomes.
