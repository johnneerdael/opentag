# R08 associated-file move qualification — 18 October 2026

Scope: opt-in same-basename `.lrc`/`.cdg` handling for Tag → Filename and desktop
Move files to. This batch does not complete area 4 or R08. PR #50 remains a
separate delivery for remembered Copy/Move destinations and scoped ledger
corrections. This worktree retained its original local PR #50 head
`c31ca77b482704e18e617675298f964ab74ef48c` before branching from
`4a1a95ff4eddb9da7492aaace68197c4f81347d0`; live PR review/CI state is tracked separately.

## Original evidence and limits

Seen in the user-supplied Mp3tag 3.36.1 distribution: `Mp3tagVersion.txt:22`,
dated 2026-08-04, describes updating renamed associated `.cdg` and `.lrc` files
when configured to be listed in the File List. Its executable SHA-256 is
`484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911`.
The shipped 2023-12-19 note also refers to associated-file Move/Copy fixes.
These are static distribution observations, not original-app execution or
certification of its association rules. The new disabled-by-default policy,
exact-basename matching and per-group rollback are explicit TagGroove behavior.

## Local evidence

The regression corpus uses disposable local files, including a valid generated
WAV carrier where metadata readback or history is exercised. It covers:

- Policy absent/off, supported extension case and exact-basename matching.
- Changed companion bytes, newly appearing companions and edited audio after review.
- Consumed/expired review receipts, changed selection/policy and late destinations.
- Existing names, conservative letter-case collisions, ambiguous companions and rename cycles.
- Symlink sources/companions, substitution after review and unchanged referents.
- Injected audio-move failure after a companion moved; reversal of completed members.
- Occupied rollback names or externally changed moved companions; explicit partial
  rollback error and preservation of both the new occupant and retained destination.
- Letter-case-only and folder moves, session Undo/Redo, and external lyric edits
  that remain untouched when Undo is refused.
- Frontend default-off Options control, one Move confirmation, displayed companion
  paths, frozen token delivery, cancelled confirmation and unchanged Copy behavior.

The actual OptionsUI compact page-picker branch was exercised at 420 × 700 in
Happy DOM with the production stylesheet (whitespace after `@media` normalized
for its parser). The test saves/reopens/restores the default policy and reads
the actual `main.ts` FileOps host preference. It provides DOM/CSS evidence,
not native pixel or input qualification.

Before branding synchronization, the full Rust workspace passed 1,095 tests
with 6 explicitly ignored probes and `npm run test:ui` passed 564 tests.
The source checkpoint is `c9b2c2ae80cbd7b73b3e5383b66169bee163d182`.

Merged the verified branding commit
`c1375a6ccf24e728f3ac29dfd442d4bf1071d47b` normally, producing
`a683a5872cd600b7dd83207ae1bb379a1ab996f0`. On that synchronized code:

| Check | Result |
| --- | --- |
| Full frontend suite | 566 passed, zero failed |
| `cargo test -p opentag --lib --locked` | 419 passed, 6 explicitly ignored |
| Production frontend build | Passed |
| Strict MkDocs guide build | Passed |
| Workspace/all-target Clippy with `-D warnings` | Passed |
| Brand verification | 249 supplied files and 78 integrated copies verified; app/MSI/gnudb identities unchanged |
| Catalog extraction check, Cargo format, diff whitespace | Passed |

These are local checks. Current-main CI and exact-final-head GitHub Codex
code review remain separate requirements; synchronization does not authorize
merging another owner's PR.

Screenshot verification was not completed: a local browser dependency failed
its native signature validation, and the alternate browser rejected a
`file:` fixture URL with an explicit policy against workarounds. No policy
or signature bypass was attempted.

This evidence does not qualify native OS keyboard/mouse/IME/screen-reader behavior,
Windows/Linux filesystems, network/cross-volume moves, original differential
behavior, installer/release results or a released artifact.

## Area 4 work still open

| Initiative | Remaining scope |
| --- | --- |
| R07 | Extended Tags staged/write/navigation clarity and native keyboard/close/cancel journeys |
| R08 | Other converters, inline and action-driven renames, Copy companions, text-record mismatch policy, original runtime comparison and platform fault/cross-volume qualification |
| R09 | Useful file/selection/cover/playlist/source toolbar commands and real migration layout qualification; the second custom toolbar already exists |
| R10 | Playlist order/refresh and XSPF/WPL/ASX readers; external export-template preservation and encoding/locale/sort qualification |
| R11 | Completion and default-tool behavior, selected directory/manual order persistence, locale refresh; retain implemented typed-directory and automatic panel defaults |
| R12 | Reproducible native-input, IME, accessibility and large-collection latency/memory/cancellation measurements on each desktop OS |
| R13 | OS file opening/forwarding, install/upgrade identity, signing/notarization, published update/download and reproducibility qualification |

Root owns current-main CI, exact-head GitHub Codex code review, protected merge
and the central parity ledger. Passing local tests does not close those gates.
