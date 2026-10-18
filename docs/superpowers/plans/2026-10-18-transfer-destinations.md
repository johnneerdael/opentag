# Remember transfer destinations implementation plan

> Execute inline in the daily-friction lane. The user approved the five-area parity roadmap and parallel implementation. The bounded behavior below implements its existing R08 outcome.

**Goal:** Copy and Move each reopen their chooser at their own last successful destination, including after restart.

**Architecture:** Add optional destinations to the existing version1 directories.json state; pass each through the existing directory-picker host into the native dialog defaultPath. Record a destination only after at least one file was copied/moved. Keep the successful file outcome if preference persistence fails. Existing configurations deserialize with empty destinations and configuration backups carry this existing file.

**Reference evidence:** Mp3tag.exe SHA484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911 UTF16LE file offsets0x962840 LastMoveFolderPath and0x962868 LastCopyFolderPath. Identifier presence supports separate memories; exact original timing is unknown. This app records completed operations only.

**Files:** src/file-ops.ts (host/defaultPath/state/update); src/main.ts (native picker defaultPath only); src-tauri/src/file_ops.rs (optional serde fields/default/normalize); tests/file-ops.test.mjs and existing Rust state tests; docs/user-guide/guides/file-operations.md and a unique verification receipt.

- [x] Add failing UI controller tests for separate destinations, persistence/restart, cancellation, failure and preference-write failure.
- [x] Run node --test tests/file-ops.test.mjs; confirm behavioral failures.
- [x] Implement optional picker start path and remembered destinations only after successful transfer.
- [x] Add native serialization/default/normalization tests and implement optional fields, preserving version1 compatibility.
- [x] Run focused tests, npm test, npm run build, cargo fmt --check and relevant Rust tests/Clippy, plus strict guide build.
- [ ] Update user instructions and verification limits; commit and open PR with substantive release notes.
- [ ] Obtain genuine final-head clean CODE review, full current-main CI and protected merge. Do not claim this batch finishes R08 sidecar/rename scope.
