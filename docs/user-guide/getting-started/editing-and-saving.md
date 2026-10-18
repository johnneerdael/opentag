# Editing, saving and undo

Changing a value on screen and writing it into an audio file are separate
steps. Use this page to practice that distinction, save a reviewed selection,
and recover from a change before continuing with a whole album.

## Make one change you can recognize

1. Work on a copied audio file. Select it in the file list.
2. In the tag panel, replace its **Album** with `Harbour Lights — practice`.
3. Look for the pending marker beside the row and the **unsaved files** count.
   The displayed album changes immediately; this is still a pending edit.
4. Choose **File → Save tags** (++cmd+s++ / ++ctrl+s++).
5. Inspect the operation result. After a successful save, that file's pending
   marker clears. **File → Reload tags** reads the saved result again.

The shortcuts with Cmd use ⌘ on macOS; use Ctrl on Windows and Linux. While
learning, use menu items if a text field's focus makes a shortcut ambiguous.

A field left unchanged is kept. An explicitly cleared field requests removal.
A format string typed into an ordinary tag-panel field is stored as text;
use [Tag → Tag](../converters/tag-to-tag.md) to calculate it.

## Save the intended selection

**Save tags** writes pending edits for the selected files. The status bar's
unsaved-file count covers the loaded list, so it can include files outside
that selection. A successful save of one selection need not clear every
pending marker in the window.

Before saving a batch, check the selected count, the fields changed and any
active filter. A filter can hide an already selected row; hiding it does not
necessarily remove it from the selection. Select the intended files again
when the scope is uncertain.

A batch can partly succeed. For example, all three files below have pending
album edits, but only the first two are selected for saving:

<!-- save-outcomes:start -->
| File | Selected for Save | Result | Pending afterward |
|---|---|---|---|
| `01 - Paper Boats.mp3` | Yes | Saved | No |
| `02 - North Pier.mp3` | Yes | Failed | Yes |
| `03 - Quiet Harbour.mp3` | No | Not attempted | Yes |
<!-- save-outcomes:end -->

A failure does not turn the successful first write into a failure too.
Read each file error, fix its cause, and retry the files that still need saving.
**Cancel** stops remaining work; it does not roll back files already written.
Unattempted edits remain pending. Do not close the application merely because
some files in the batch were saved.

## Discard, reload or undo

| Goal | Command | What to expect |
|---|---|---|
| Abandon unsaved edits in selected files | **Edit → Discard pending changes** | Return to the loaded values without reading the files again |
| Read selected files again after an external edit | **File → Reload tags** | Read current storage and discard those files' pending edits after confirmation |
| Reverse a recorded write | **Edit → Undo …** | Restore the operation named in the menu, when its history step is available |
| Reapply a reversed operation | **Edit → Redo …** | Restore its redo state, when available |

Discarding pending edits is different from undoing a saved operation.
**Undo** does not mean “clear every unsaved edit”: pending edits are retained
when recorded file history is replayed. Inspect them afterward so a later
save does not unintentionally write those pending values over the restored file.

History is for the current desktop session. Read its operation label and file
count, including any failure notice. It is not a durable backup of the library,
and changes made by another program are not automatically reversible here.
A batch history operation can concern several files, beyond the current
selection.

The **Redo** menu slot can instead say **Repeat**. Repeat runs the available
last action or action group again on the current selection; it does not
restore a historical snapshot. Use [action groups](../actions/index.md) to
understand what that repeated operation will do.

## Know which commands write immediately

Tag-panel text edits, metadata cells, **Keep changes** in Extended tags,
Cut/Paste tags and staged auto-numbering wait for **Save tags**. A filename
cell edits a path instead of a metadata field.

Actions, applied converters, **Remove tag…**, and direct artwork commands
write when run. They do not leave their completed results waiting for a later
Save command. Managed cover edits have their own **OK** commit step. Follow
the specific dialog's preview and confirmation instructions.

## Check automatic saving and prompts

Under **File → Options… → Tags**, **Save tags when using arrow keys or a
single mouse click** is off by default. When enabled, a supported selection
change can save pending edits from the previous selection of local files.
This includes mouse/arrow navigation, **Select next file**, and leaving the
selection through commands such as **Unselect all**. Extending a selection
can also trigger a save of previously selected pending files. Treat selection
changes as potentially writing when this preference is on.

Extended tags has a separate navigation save-mode setting for its **<<** and
**>>** buttons. See [Tags options](../reference/options/tags.md) for its
pending, ask, ignore and save choices.

Commands that would replace the list, reload or close with pending edits can
ask whether to discard them. **Options → Messages** controls these prompts.
Disabling a prompt does not save the edits automatically. Resolve the edits
first when they need to survive closing the window.

## Respect storage and access limits

**Options → Files → Preserve timestamps of modified files** preserves the
file's modification time during supported writes. It does not make a save
invisible to backups, preserve every filesystem date, or stop metadata bytes
from changing.

Tag storage is format-specific. An ordinary save can synchronize several
configured tag types and remove a type selected for removal but not writing.
Review [Tags options](../reference/options/tags.md) before changing storage
policies to solve a player compatibility issue.

Read-only local files can fail to save. Provider-backed documents have their
own access and recovery state: follow **Source read-only**, **Source access
unknown** or **Recovery required** messages. **Save a copy…** can preserve
supported edited tag fields and pending artwork, or an original backup when
offered. The edited copy includes pending cover removal; the original backup
ignores all pending changes. The source and its pending edits remain
available after a copy, including cancellation or failure. It is not a general
desktop “Save As” replacement. See the [File menu](../reference/menus/file.md).

## Related

- [Your first tagging session](../learn/first-tagging-session.md)
- [The tag panel](../guides/tag-panel.md)
- [Cut, copy and paste tags](../guides/copy-paste-tags.md)
- [Removing tags](../guides/remove-tags.md)

<!-- verified: src/main.ts:draftField, saveDrafts, discardDrafts, replayHistory, saveOnMove, operateImpl; src/document-state.ts:reconcileDrafts; src/list-behaviour.ts:autoSaveTargets; crates/tag-core/src/tags.rs:staged_mutation; src-tauri/src/history.rs. Source/model evidence; no installed UI/save-back claim. -->
