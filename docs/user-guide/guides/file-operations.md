# Copy, move and delete files

File operations change where audio files exist, rather than changing their
metadata. Start with copied practice files and a separate destination folder.
Check the selected-file count: a filter can hide selected rows without
removing them from the operation.

## Choose the operation by its effect

| Goal | Command | Disk and list effect |
| --- | --- | --- |
| Make another file in a folder | **Edit → Copy files to…** | Create a destination copy; retain the source and its listed row |
| Relocate the file | **Edit → Move files to…** | Move the file and update its listed path; it may then leave the list |
| Stop showing selected rows | **Edit → Remove from list** | Remove rows and their pending edits; keep the files on disk |
| Empty the workspace | **File → Clear list** | Remove every listed row, including hidden rows; keep files on disk |
| Delete through the system trash | **Edit → Delete files…** | Send successfully deleted files to the desktop trash and remove their rows |
| Delete without the trash | **Edit → Delete files permanently…** | Remove successfully deleted files directly from disk and the list |

**Copy tags**, **Cut tags** and **Paste tags** are different commands. They
transfer metadata, not the audio files. [Removing a tag](remove-tags.md) also
leaves the audio file in place.

## Copy one album file to a practice folder

Suppose the original is `/Music/Juniper Vale/03 - Paper Boats.flac` and the
existing destination folder is `/Practice/Harbour Lights`.

1. Save intended pending tag edits, or deliberately discard them. Copy/move
   commands require the selected files' pending changes to be resolved.
2. Select the ordinary file row. Clear an old selection first if necessary;
   do not use the number of visible rows as the target count.
3. Choose **Edit → Copy files to…** (++cmd+shift+c++ / ++ctrl+shift+c++).
4. Choose the existing destination folder. Read the confirmation's destination,
   file count and shortened name list before accepting.
5. Check the result and any file errors. The intended destination is
   `/Practice/Harbour Lights/03 - Paper Boats.flac`.
6. Use **Add files…** or **Add directory…** to inspect the new copy. Copying
   does not automatically add its destination row to the file list.

The confirmation abbreviates long name lists; it is not a per-file rename
preview. Check the full selection and destination yourself before accepting.

After at least one file is copied successfully, the next **Copy files to…**
dialog starts at that destination. **Move files to…** remembers its own
destination separately. Both choices survive restarting OpenTag. Cancelling
the picker or confirmation, or a transfer that moves/copies no files, keeps
the previous choice. If preferences cannot be saved, the destination is
remembered for the current session and the completed transfer still stands.

The command retains the filename and extension. It places selected files
**directly in the chosen folder**; it does not recreate the source folder
tree. To create a metadata-derived folder layout, use
[Tag → Filename](../converters/tag-to-filename.md) on practice files instead.

### When a copy or move is rejected

The transfer planner checks the complete selection before starting. An
ordinary existing destination name, two selected sources with the same
basename, a source already in the destination folder, an unreadable source,
or an invalid destination can reject the plan. Selected basenames are also
compared without letter case, so `Song.flac` and `song.flac` are treated as a
selection conflict even on a volume that can store both.

For example, selecting both `/Set A/03 - Paper Boats.flac` and
`/Set B/03 - Paper Boats.flac` for one destination does not merge them or
invent a suffix. Choose separate destination folders or deliberately rename
copies before transferring them.

These checks are useful safeguards, not a universal filesystem alias or
collision guarantee. Links, case rules, network volumes and changes made by
another process have their own limits. A clean empty practice destination
makes the intended result easier to inspect. Do not approve an operation on
the assumption that it will resolve every ambiguous name for you.

Planning failure leaves that plan unstarted. A failure **during** execution
is different: successful earlier transfers can remain. Read each error and
inspect both folders before retrying; the batch is not one rollback transaction.

## Move files and understand disappearing rows

Use **Edit → Move files to…** (++cmd+shift+v++ / ++ctrl+shift+v++) with the
same selection/destination checks. The destination filename is unchanged.
A move can succeed even if reading the destination's metadata afterward
fails, so an error does not always mean the file stayed at its old path.

**Options → General → Keep moved files in the file list** controls whether
files moved outside the listed folders remain visible. With that option off,
a successfully moved file can disappear from the list. That disappearance
is not deletion. Inspect the reported destination in the file manager or
open the destination folder in TagGroove.

Copy/move commands do not create a session tag-Undo step. To reverse a move,
inspect the destination first, then deliberately move the file back when its
original name/location is available. To remove an unwanted copy, identify the
copy rather than deleting the original. Do not treat an older **Undo Save
tags** menu item as an undo of the more recent file transfer.

## Remove rows while retaining files

1. Select the rows to stop showing.
2. Choose **Edit → Remove from list**. With the file list focused, Delete
   performs this list removal; macOS also uses Backspace.
3. If those rows have pending edits, read the discard confirmation. Accepting
   removes their unsaved field/artwork intent along with the rows.
4. Reopen the file or its folder when needed. The disk still contains the
   last saved metadata, not the discarded pending values.

**File → Clear list** applies to the entire workspace, rather than only the
selection. It can include rows outside the current filter. Neither operation
is recorded as an Undo action for list membership. Opening files again is
not a recovery of discarded edits. Resolve edits first when keeping them matters.

## Delete through the trash or delete permanently

For ordinary local files, **Delete files…** requests the desktop system's
Recycle Bin/trash. The normal confirmation describes the selected names and
count. **Options → Messages → at deleting files** controls whether that
confirmation appears; a missing prompt does not mean that the command is a
list-only operation.

After the operation, successful files leave the list and failures remain
reported. A locked or unavailable file can fail while another is deleted.
Do not repeat the whole selection without checking what already happened.
If recovery is needed, inspect the system trash and use its restore command,
then reopen the restored files. Trash behavior and recoverability depend on
the OS, volume and permissions; this guide does not qualify a native trash
round-trip on your machine.

!!! danger "Permanent deletion bypasses the trash"

    **Delete files permanently…** does not create a TagGroove Undo step or a
    trash copy. Keep a separate backup before using it. Its confirmation can
    also be suppressed by the deletion-message preference. Use the menu label
    deliberately instead of experimenting with modifier keys on real files.

The inspected backend accepts regular files for transfer/deletion and rejects
folders and symbolic-link entries. This is not a recursive folder-deletion
command. File operations require desktop/local filesystem access. Chapter
rows are not separate audio files: copy/move/delete are unavailable for such
rows. Turn off **List chapters as separate files** and reload the directory
to work with the parent file. Provider documents have separate access and
publication rules; do not substitute these desktop instructions for them.

## Reveal or play without changing metadata

**File → Open in Finder/Explorer/file manager** reveals selected files. With
no selection, it can open the current working folder. Alt+Enter provides the
file-list reveal route. For filesystem properties, reveal the file and use
the file manager's properties/info command.

**File → Play** (F4 with the file list focused) asks the desktop to open files
with their default application. More than ten selected rows triggers a
confirmation; the backend limits the distinct resolved playback files to
100. Chapter/CUE playback resolves the referenced audio file, not an
independent new file per displayed row. Starting an application is not proof
that it played every track or understood chapter positions.

Reveal and Play do not save pending tag edits. A player reads the saved file,
which can differ from what TagGroove currently shows as pending. Configured
[external tools](../customize/tools.md) have their own per-file and argument
rules; they are not the same as default playback.

## Gotchas and recovery

Keep the result message, destination and per-file errors until the state is
clear. If a copy/move partially completes, compare the original and destination
folders before choosing the smaller remaining selection. If deletion partly
completes, inspect the trash or backup before doing anything permanent.

Close or pause other programs that are rewriting the same files. Pending tag
edits, session Undo and an external application's changes are three different
states; none replaces an independent audio-file backup.

## Related

- [Select and order files](file-list.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Undo and history](undo-history.md)
- [Remove tags](remove-tags.md)
- [Playlists](playlists.md)

<!-- verified: src/file-ops.ts:FileOpsUI.operate; src/main.ts:operationReason, removeEntries, applyRenamed; src-tauri/src/file_ops.rs:regular_file, plan_transfer, transfer, trash_paths, existing, play_files. Source/controller evidence, not native filesystem/trash/player qualification. -->
