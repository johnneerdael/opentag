# R09 toolbar catalog qualification — 18 October 2026

This batch exposes useful commands that already have application routes. It
does not add a second toolbar, new file/source operations, or vendor website
buttons. It starts from verified branding commit
`c1375a6ccf24e728f3ac29dfd442d4bf1071d47b` on its own feature branch, independent
of the frozen associated-file delivery in PR #54.

## Behavior and compatibility

Retain the original 28 catalog command identities and every original default
item position. Retain the eight visible default commands. Append the additional
file, directory, selection, cover, playlist and built-in-source choices hidden
in main-toolbar configurations, including older saved configurations. Preserve
custom-toolbar order, visibility, authored labels, tool names and action-group
references. The custom toolbar gains choices but no automatically inserted items.

Use the existing main and custom renderers, button wiring, enabled/reason guards
and dispatcher. No separate file/source/cover implementation is introduced.
There is no new standalone cover extract/adjust route. Existing Tag → Filename
remains the rename converter; Edit in file list uses the existing editable
filename column when present. The application’s confirmations, pending-edit
requirements, provider/chapter restrictions and credential checks remain in effect.

## Local regression evidence

`tests/toolbar-catalog.test.mjs` exercises:

- Original catalog order, complete original default prefix and unchanged visible defaults.
- Hidden additions to older layouts without rewriting authored order/labels or input objects.
- New configuration persistence alongside named tools and action groups.
- Actual main/custom toolbar renderer and custom-choice construction.
- Actual main dispatcher routes for every added identity, with effects replaced by spies,
  and the production generic button binding reaching the dispatcher.
- Actual custom-toolbar dialog cancellation/save while retaining named tool/action state.
- Bundled German/Dutch translations for new hints and verbatim saved user labels.
- User-guide coverage of every added command label.
- Actual file, chapter, provider-document and credential eligibility policies for
  empty selections, pending edits, chapter rows, read-only provider sources,
  desktop-only folders, missing favorites, missing AcoustID keys, cover absence
  and busy state. A stale programmatic invocation remains refused.

This is source, model and DOM/control-flow evidence. It is not native keyboard,
screen-reader, screenshot, original-app differential or live-provider qualification.
The earlier browser file-URL denial explicitly prohibited workarounds; no new
browser or screenshot attempt is part of this batch.

## Validation receipt

| Check | Result |
| --- | --- |
| Full frontend suite (`npm run test:ui`) | 569 passed, zero failed |
| Focused toolbar catalog suite | 8 passed |
| Existing executable configuration/tools guide examples | 3 passed |
| Production frontend build (`npm run build`) | Passed |
| Strict MkDocs guide build | Passed |
| English catalog extraction check and bundled catalog validation | Passed |
| Brand verification | 249 supplied files and 78 integrated copies verified; app/MSI/gnudb identities unchanged |
| Diff whitespace check | Passed |

Production changes are limited to the toolbar catalog/default list and
additive English/German/Dutch keys. No main handler, Rust, storage identifier,
protocol identifier or metadata writer is changed. Native/platform CI and
exact-final-head GitHub Codex code review remain separate gates. Publication
and merging are coordinated with PR #47 and remain on hold until authorized.

## Work still open

R09 still needs a real migration-profile toolbar ordering and original toolbar
layout import format, native accessibility/keyboard/pointer qualification and any
additional cover commands justified by real operation routes. This batch makes
no vendor-button-count equivalence claim.

R10 remains a separate batch: playlist order/refresh qualification, XSPF/WPL/ASX
readers, externally added export-template preservation, and export encoding,
locale and sort behavior. Exposing existing playlist buttons does not implement
those readers or certify their output.

R07/R08/R11–R13 also remain open, including native save/edit journeys, the separate
associated-file delivery and remaining rename/copy flows, completion/tool details,
large-collection measurements, and desktop install/upgrade/signing outcomes.
The coordinating owner retains central parity-ledger and merge/CI ownership.
