# Provider Save a copy: pending artwork

This change fixes the pending-artwork portion of roadmap R19. It does not
complete the specialist parity area (R14–R20), qualify Android/iOS devices,
or establish run-for-run Mp3tag equivalence.

## Request and write contract

`src/main.ts:saveCopy` snapshots pending fields and the optional pending cover
list into one `document_save_copy` request. The cover entries reuse the tag
clipboard's digest, picture type and description contract.

| Request | Edited copy | Original backup |
| --- | --- | --- |
| No pending cover list | Keep artwork already in the retained edited bytes | Copy the last verified source bytes |
| Nonempty pending cover list | Replace artwork in the supplied order, retaining image bytes, types and descriptions where the container supports them | Ignore the list |
| Explicit empty cover list | Remove all covers from the new copy | Ignore the removal |
| Stale clipboard digest | Fail before opening the destination picker | Ignore unapplied draft artwork |

Clipboard Copy, Paste and Cut use the readable retained working copy even
when the source is read-only or requires recovery. These operations stage
intent only. The source access policy continues to block **Save tags** and
immediate cover mutations, and an unreadable working copy is excluded from
artwork staging.

The native wrapper resolves all requested clipboard image bytes before
`CREATE_DOCUMENT`. The document engine starts from retained edited bytes or
the verified original baseline, stages cover and field changes on the new
copy, then publishes through the existing durable pending/recovery flow.
The `io` command worker already initializes the document store while holding
`FILE_OPERATIONS`; the copy handler uses that initialization path.

The source provider document, source working copy, verified backups and UI
drafts are retained. Cancellation produces no copy record. A publication
failure retains the new copy's edited working bytes and its recovery state;
its provider output may be partial or unverified. No rollback of an unknown
provider output is attempted. A later copy can recover the retained edited
bytes without reading the source provider.

Copy writing continues to use the existing default tag policy. Applying user
read precedence and field mappings throughout provider read/reload/copy paths
remains separate work. MP4 artwork still has no cover type or description.

## Regression evidence

- Before the UI fix, the new `copyRequest` test failed because the request
  omitted `covers` entirely.
- Restoring the original fields-only writer branch made the native regression
  fail: the output contained `CoverFront / Original front` instead of
  `CoverFront / New front` followed by `CoverBack / New back`.
- `src-tauri/src/documents/copy_command_tests.rs` calls the production
  `document_save_copy`, clipboard and cover-read commands. Its only provider
  substitution is the filesystem-backed command fixture. Every output is
  explicitly selected inside a disposable temporary directory and created
  exclusively.
- Native cases cover read-only sources, ordered replacement artwork, image
  byte equality, cover metadata, field deletion, unchanged artwork, explicit
  remove-all, original-backup isolation including stale draft artwork,
  cancellation before record allocation, missing clipboard data before
  destination creation, recovery-source copies, failed publication and retry.
- `src-tauri/src/documents/tests.rs` covers replacement and remove-all through
  partial provider writes, failed fresh reads and mismatched publication
  hashes. Reopening the store retains the edited fields and covers, permits
  a fresh edited copy without source access, and preserves original backups.
- WAV fixtures contain valid PCM data. An independent RIFF chunk walk checks
  unchanged `data` bytes after writes. PNG images are generated from distinct
  RGB values; native cover readback checks their original encoded bytes.
- `tests/document-coordination.test.mjs` executes production `operateImpl`,
  `clipTags`, `stageTagUpdates`, `coverTargets`, `saveCopy`, `enabled` and
  `sourcePolicy` with the real `documentSelection` and tag-clipboard models.
  Copy → Paste and Cut → edited/original copy journeys cover both read-only
  and recovery-required sources. No pending-cover injection or permissive
  policy stub is used. With the old publication gate in `coverTargets`, all
  16 copy-only-source staging/retention cases failed because no artwork intent
  was staged; removing that gate makes them pass. Checks also cover explicit
  remove-all, artwork-only pastes, unavailable working metadata and blocked
  source writes. The controlled transport and dialog boundaries still make
  this model-level UI evidence; it does not exercise OS input or a real picker.

Validation commands and their final results are recorded in the PR. These
fixtures exercise real metadata and audio files with a substituted SAF
transport. They do not prove Android binder behavior, cloud-provider
publication or real-device interruption handling. No Mp3tag runtime was
executed and no user music library was operated on.

## Specialist program remaining scope

The roadmap baseline is `4a1a95ff4eddb9da7492aaace68197c4f81347d0`.
The following are outstanding acceptance work, not completed claims. Their
priority follows the approved parity roadmap; a stale register entry must be
reproduced against current source before implementation.

| Initiative | Remaining acceptance work |
| --- | --- |
| R14: specialist metadata/formats | Obtain provenance-recorded real-encoder and reference-writer fixtures for TAK, OptimFROG, Nero, APE, ASF, Matroska, ADTS and uncommon ID3 variants. Independently inspect metadata and unchanged audio after edits. Qualify chapter URLs/artwork, subsong syntax and safe refusals, native field grammar, tag-family interactions, technical hashes and tag sizes. Separate container limits from defects. |
| R15: cover conversion/transfer | Measure resampling, size rounding, quality and Original-format fallback using paired results. Qualify Linux drag-out names and transfer on Linux. Preserve unchanged image bytes where no conversion is requested. |
| R16: networking/legacy sources | Qualify actual keyed-provider setup, needed proxy authentication/SOCKS/PAC cases and gnudb server-list/local lookup behavior. Interpret source minimum-version declarations as required capabilities. Keep live HTTP receipts distinct from parser replay. |
| R17: persistent recovery | Design and validate desktop restart recovery/undo retention and stale-baseline rejection, Windows external-edit detection and snapshot budgets on non-cloning volumes. This PR exercises existing provider restart recovery only; it does not persist desktop Undo history. |
| R18: desktop customization | Observe original File Exit, Windows Properties, floating panels and movable toolbar workflows before selecting platform behavior. Treat binary-only setting names and a possible global leading-zero option as unknown until independently established. |
| R19: mobile documents | Apply read precedence and field mappings to provider read/reload/copy paths; qualify provider undo, exports, playlists and file actions; implement iOS provider selection/save-back and qualify directory/cloud providers. Run permission-loss, interrupted-write and restart recovery cases on real Android/iOS devices. Mobile credentials remain separate work. |
| R20: languages/regions | Add demand-led, independently authored catalogs after high-impact workflows stabilize; qualify complete journeys, Unicode, locale-sensitive formatting, layout and input. Do not reuse Mp3tag proprietary catalogs or infer regional correctness from catalog count. |

Signing and distribution qualification belong to the broader platform/release
work. This PR supplies no new Windows signature, macOS notarization, iOS
physical-device artifact, reproducibility result or dependency-license audit.
