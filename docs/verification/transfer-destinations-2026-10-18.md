# Copy and Move destination memory

Scope: the daily-friction lane of the approved Mp3tag migration roadmap, R08. This is a bounded workflow improvement; sidecar renaming, collision policy and original runtime timing remain separate work.

## Reference and behavior

Static leanrea inspection of the supplied Mp3tag 3.36.1 Windows x64 executable (SHA-256 `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`) found UTF-16LE settings identifiers `LastMoveFolderPath` at file offset `0x962840` and `LastCopyFolderPath` at `0x962868`. This supports separate destination memories; identifier presence does not prove the original dialog or save timing.

Copy and Move now pass their own last successful destination to the native directory picker. A completed transfer with at least one copied/moved file records the folder in the existing `directories.json`, alongside recent/favorite directories. Cancellation and a wholly rejected transfer keep the previous value. Preference-save failure preserves the completed transfer and the session's destination memory. Paths are preserved verbatim, including legal trailing whitespace.

The state remains version1. Old files default to no remembered destination, optional missing values are omitted on serialization, and existing configuration backups already include this file. No audio transfer algorithm, collision rule, pending-edit policy or undo behavior changes.

## Verification

The new controller tests first failed because no initial picker path or destination memory existed. After the change, all24 file-operation controller tests pass, including separate copy/move memories, restart, picker/confirmation cancellation, complete transfer failure and preference-write failure without retrying the file operation. The full562 frontend tests, production TypeScript/Vite build and strict pinned MkDocs build pass.

The native JSON round-trip test first failed because destination fields were discarded. The focused native file-operations suite then passed11 tests with the real-system-trash probe left ignored. The initial-batch workspace run passed1083 tests across71 suites, with6 intentionally ignored tests. Cross-platform CI remains required and is not inferred from this local run. Local strict Clippy and diff checks pass.

One unprefixed local `npm test` run completed all frontend tests but its fresh default-target Cargo phase failed because Homebrew GNU `ar` shadowed Apple's archiver. Native validation uses `AR=/usr/bin/ar RANLIB=/usr/bin/ranlib CARGO_TARGET_DIR=/tmp/taggroove-target OPENTAG_AUDIO_SOURCE_DIR=/tmp/taggroove-parity-audio-sources`; the offline native dependency archives were checked against the approved source manifest. No source or CI workaround is introduced for this local toolchain issue.

No original Windows runtime comparison or native picker interaction was performed locally. The live OS dialog's own handling of a remembered folder that was later deleted remains a platform acceptance case. Final-head genuine Codex CODE approval and complete current-main trusted CI remain required before merge.

## Review follow-up: later directory responses

Codex found that a transient failed preference save was followed by an older on-disk state from `remember_directories`, dropping the session-only memory. The regression reproduced `/copy-old` instead of `/copy-new` after opening another album. Successful destination choices now have a separate session overlay, merged into load/remember/save responses and subsequent preference writes. This retains fresh recent/favorite state while keeping the successful destinations. All25 file-operation tests and the production build pass after this correction; the earlier full frontend/native counts above identify the initial batch, and final-head CI remains required.


## Ledger reconciliation on approved main

The branch integrates approved main `696e419191aec8aabd95edec76a499de8b9a14a0`, including the shared native Boost Find engine and its remaining runtime-qualification limits. The older test counts above are checkpoint evidence, not results for this combined head.

The following corrections record existing implementations; this PR does not rebuild them:

- `crates/tag-core/src/converters.rs` compares the rendered suffix with the source extension through `ends_with_extension`, ignoring case. A matching explicit suffix is kept once; a nonmatching suffix still has the source extension appended. For `song.mp3`, old pattern `%1` and new pattern `%1.flac` produce `song.flac.mp3`. The ledger retains this narrower qualification gap; original behavior for that case remains unverified.
- `src/tag-panel.ts` handles Enter in a typed directory field through `typedDirectory` and `host.changeDirectory`; the production host in `src/main.ts` connects it to opening that directory.
- `automaticDefaults` in `src/tag-panel.ts` selects visible fields with configured defaults and no pending edit. `applyAutoDefaults` in `src/main.ts` applies those defaults on a new multi-file selection while preserving pending edits.

These are source and local-regression observations. Original runtime timing and native input equivalence remain unverified.


Combined-source validation after integrating approved main passed all 575 frontend tests and 1,088 Rust tests across 71 result blocks, with six existing ignores. The production build, strict guide build, formatting, strict local Clippy and diff checks passed. Hosted platform validation and a fresh Codex review of the final commit remain required.


## Review follow-up: explicit extension scope

The ledger's earlier extension reconciliation was too broad. Source-matching suffixes are deduplicated case-insensitively; nonmatching suffixes still receive the source extension and remain an original-runtime qualification case. The converter guide now shows four extension choices for `song.mp3`, including `%1.flac` producing `song.flac.mp3`. The existing native guide harness executes all eight published rearrangement rows; its nine converter-guide tests and two newline-portability tests pass. The strict guide build, formatting and diff checks pass. This correction changes documentation and the existing guide test's expected row count; application behavior is unchanged.
