# File List behaviours: save on move, manual order, folder image, artwork in tag copy, column auto-size

Branch `feat/list-behaviour` (from `feat/core-parity` @ 8a8d6f9). Closes gaps 7, 8, 9 and 12 of `docs/research/remaining-gaps.md` and the column auto-size runner-up.

Sources: Mp3tag 3.36.1 documentation pages `customization/options/tags`, `keyboard`, `menus/edit`, `customization/file-list`. Option labels follow the documented names; no documentation text was copied into the app.

Native e2e (`tests/e2e/list-behaviour.js`) runs only headless in CI (`native-e2e` job). It was not run locally.

## Behaviour

| Item | Mp3tag | OpenTag now | Where | Tests |
|---|---|---|---|---|
| Options → Tags → Save tags when using arrow keys/single mouse click | Pending edits are saved when the file loses focus: another file or the empty list area is selected with the mouse, the arrow keys move in the File List, or Ctrl+N is pressed (also from the Tag Panel) | Off by default. When the selection changes through a click, the empty list area, arrow/Page/Home/End keys, Space, Ctrl+N, Unselect all, Invert selection or Select similar, the files left behind that have pending edits are saved in one `save_fields_batch` call (journaled, one undo step, progress). Focus returns to the list row or tag panel field. Provider documents keep their edits pending. Ctrl/⌘ N now works from a tag panel field and keeps focus there | `src/list-behaviour.ts` `autoSaveTargets`; `src/main.ts` `saveOnMove` | node; e2e (click, arrow, Ctrl+N from the panel) |
| Click on the empty File List area | Deselects (counts as moving away) | Clears the selection (not on scrollbars, header or the empty-list placeholder) | `src/main.ts` | e2e (indirect) |
| Alt+↑ / Alt+↓, Alt+Page Up / Page Down | Move the selected files one row, or to the beginning/end | Same. Non-contiguous selections keep their relative order; a file at the edge stays while the others move. A sorted list switches to unsorted, starting from the displayed order (the unsorted layout is saved like View → Unsorted). With a filter, hidden files keep their positions | `moveSelection`, `mergeOrder`; `reorderFiles` | node; e2e |
| Alt+drag | Change file order | Dragging selected rows (or the pressed row) with Alt shows a drop line and moves them; the click that ends the drag does not change the selection | `bindRowDrag`, `dropSelection` | node; e2e (synthetic pointer events) |
| Order persistence | Manual order is the list order | The order is the list order used by Playlist (all files), auto-numbering, Export and every command that takes the selection in list order, while the files stay listed. Renames now keep a file's place (before, a renamed file moved to the end of the unsorted order) | `applyRenamed` | e2e (numbering preview follows the dragged order) |
| Options → Tags → Don't display first image from file directory as cover art | Without embedded art, the Tag Panel shows the folder's first image | Off by default, so the fallback shows. Applies when no selected file has (or is about to get) a cover and all selected files share the active file's folder. The first readable `jpg/jpeg/png/gif/bmp/webp` in case-insensitive name order is used; the pane titles it "Folder image", names the file and states "Not embedded in the file", with a dashed frame. Read only: nothing is written. Cached per folder (8 folders), cleared when files load or the list is refreshed. Not offered for provider documents | `list_behaviour::folder_image` (Rust, no file lock); `folderImageFolder`; `CoverUI.markup({folder})` | Rust (first image, corrupt image skipped, folder unchanged); node; e2e |
| Edit → Copy / Cut / Paste tags | Tags are copied with their artwork | Copy and Cut read the selected files' covers in the backend (`copy_tag_covers`), which keeps the image bytes by content digest (up to 256 MB) and returns their details and one copy of each image for display. Paste stages each target's cover list (one source for all targets, or one per target, like the fields); a source without covers removes the target's covers; targets that already have the same covers are left alone. Cut stages cover removal. Pasted covers are pending edits: the cover pane shows them with "Pasted covers are written when you save tags" (or "The covers are removed…"), the row shows the pending marker, and Save writes them with the fields in the same journaled `save_fields_batch` step (undo restores both). Images still needed by unsaved pastes survive later copies. Cover commands that act on stored covers (copy, cut, extract) are held until the paste is saved | `src/tag-clipboard.ts` `pasteCoverPlan`, `cutCoverPlan`; `list_behaviour::copy_tag_covers`, `cover_changes`; `FieldUpdate.covers` | node; Rust (save writes and removes covers, stale artwork fails one file without writing); e2e (copy → paste → save → undo; cut → save → undo) |
| Column auto-size | Ctrl+Num + sizes the File List columns to their contents | Ctrl/⌘ Num + sizes every visible column; double-clicking a header divider sizes that column. Width = widest of the header label (with sort indicator) and the evaluated cell values of up to 5000 listed rows, measured with the cells' font plus padding and the pending marker; clamped to 24–4000 px; saved with the column layout | `fitWidth`, `textWidths`; `autoSizeColumns`; `ColumnUI` host `autoSize` | node; e2e (divider double-click, Ctrl+Num +, saved layout) |

The shortcuts dialog (Help → Keyboard shortcuts) lists Alt ↑/↓, Alt Page Up/Down and ⌘/Ctrl Num +.

No option for copying artwork is documented, so none was added.

## Checks

- `cargo test --workspace --locked`: 513 passed after merging `feat/core-parity` @ cf1c48d (new: `list_behaviour::tests::folder_image_is_the_first_image_by_name_and_never_written`, `list_behaviour::tests::pending_covers_resolve_from_the_clipboard_store`, `tests::saving_pasted_tags_writes_and_removes_copied_covers`).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean. `cargo fmt --all -- --check`: clean.
- `npx tsc --noEmit`: clean.
- `node --test tests/*.test.mjs`: 231 passed (new `tests/list-behaviour.test.mjs`, 11 tests; `tests/document-coordination.test.mjs` harness updated for the pending-cover state).
- Native e2e: `tests/e2e/list-behaviour.js` with `tests/e2e/list-behaviour-fixtures.sh` (folder.png and an MP3 with an embedded cover) added to the `native-e2e` suite list in `.github/workflows/build.yml`. CI only.

Provider **Save a copy → Edited copy** now includes pending artwork. Copy,
Paste and Cut stage covers from readable working copies even for read-only
or recovery-required sources; source publication remains blocked in those
states. See [the provider-copy verification](provider-copy-artwork.md) for
request, recovery and testing boundaries. Actual-device provider qualification
remains open.

## Remaining gaps

- Saving on move is not triggered by the cell editor's Enter/Tab advance, Find, Select all or Select directory (they are not "moving to another file" in the documented sense).
- Alt+drag has no drag image beyond the drop line, and auto-scrolls only while the pointer moves near the list edges.
- The manual order lasts while the files are listed; it is not stored across sessions.
- The folder image is taken from the active file's folder only; a cover-art directory option (Options → Directories) is still missing.
