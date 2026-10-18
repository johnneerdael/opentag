# Bring an Mp3tag setup across

Use **File → Import Mp3tag setup…** on a desktop to inspect a configuration
folder or ZIP and choose the supported parts to import. TagGroove reads the
original without changing it. This imports definitions and preferences; it
does not run an action, export a playlist, contact a tag-source provider or
write audio tags.

## Prepare the profile and your current session

1. Locate the configuration folder or backup ZIP from your own Mp3tag setup.
   Keep a separate original copy outside the TagGroove configuration folder.
   The selected folder or ZIP must neither contain nor be inside that folder;
   overlapping locations are rejected before recovery or import can write.
   The importer does not infer undocumented binary/global settings from their filenames.
2. Save or close edited action-group dialogs. Save pending audio/artwork edits,
   or explicitly discard them when prompted. Configuration restoration cannot
   recover discarded in-memory audio edits.
3. Choose **File → Import Mp3tag setup…**, then **Folder…** or **ZIP…**.
   For older action/export files, enter their original legacy codepage when
   known, such as `windows-1252`. Leave it empty to use the existing individual
   importers' defaults. The chosen codepage is retained in the report.
4. Read the inventory. Unsupported and invalid files stay visible, with their
   reasons; seeing a file in the inventory does not mean it can be transferred.
   Select only one incoming action group or export template for each destination
   name. Choosing two source files with the same name, including case variants
   in different subfolders, is rejected under both keep and replace policies.
   Those policies decide collisions with the current setup only.

Inputs are bounded to 128 MB and 4096 entries, with paths at most 64 folders
deep. Supported text definitions are limited to 4 MB each. ZIP paths must be
relative and cannot contain traversal, drive prefixes, symbolic links or
case-duplicate names. Linked files/folders are not followed. Split or simplify
an oversized profile rather than renaming unsupported files to another suffix.
Window recovery is bounded to 2 MB, 10000 listed files and 10000 roots. For a
larger workspace, save the file list as a playlist and clear the list before
inspecting the setup; the importer never silently truncates it.

## Choose what comes across

| Profile component | What the guided importer does |
| --- | --- |
| `columns.ini` or INI presets under a `columns` directory | Preview the existing column importer result; choose at most one layout to replace the active layout. Unsupported expressions/columns are reported. |
| `usrfields.ini` or INI presets under `panels` | Preview supported tag-panel controls; choose at most one layout. Preserve existing TagGroove field-value lists; report unsupported source controls/custom-value options. |
| `.mta` action groups | Reuse the existing MTA importer and retain unresolved actions with their diagnostics. Imported groups remain disabled until reviewed. |
| `.mte` export templates | Decode with the existing export importer and retain the template/name. Importing never executes the template. |
| `.src`, `.inc`, `.settings` source collections | Report them separately. Use [Install tag sources](../tag-sources/web-sources.md) for complete source bundles, then qualify a real lookup. |
| Other/global/binary files | List them as unsupported; leave their original bytes untouched. No silent guessed mapping is applied. |

Select the intended items. Selected column/panel layouts replace their active
layouts. For existing action-group/export-template names, choose **Keep current
items** or **Replace current items**. A replacement export retains its existing
TagGroove identity and physical filename. Partial imports keep their warnings;
an unresolved action does not silently become another action type.

For example, a profile might contain two valid columns and a third using an
unknown function, a panel with Title and Comment plus an unsupported technical
control, an action group and a text export. Selecting those components imports
the two supported columns, supported panel fields, the disabled action group
and export. The skipped column/control and any unknown global-settings file
remain in the report. This is a partial profile transfer, not full editor parity.

## Import, restart and verify

Choose **Import selected setup** after reviewing the replacements. The native
backend binds the preview to the exact source inventory, current managed
configuration, physical export templates and captured window state. If any of
those change, it rejects Apply and requires a fresh inspection.

TagGroove stages the supported outputs with the existing parsers/validators,
then records a local recovery journal before replacing managed files. The
window reloads to read the saved configuration. It rescans the captured local
workspace and restores file order/selection, location, filter and the explicitly
supported window preferences. A workspace receipt is consumed only after that
restore succeeds; later starts do not replay a consumed workspace. Recovery
runs before newly queued file-opening requests, so an explicit new launch is
then honored as a replace or add operation. Successful recovery suppresses
only the usual favorite/previous-folder fallback, not explicit launches.

Check the columns, panel, disabled action groups and export definitions after
reload. Review action behavior and external paths before using a definition
on copied audio. A successful definition import is not proof of equivalent
Mp3tag behavior, valid external commands or working provider credentials.

If workspace files are unavailable, their recovery stays pending in **File →
Restore setup…**. Make those files accessible and choose **Retry workspace**.
A failed file-read attempt is not automatically replayed over later work on
every start. If the native attempt could not be durably claimed, startup stays
in a blocking retry dialog before admitting new launches. An acknowledgement
failure retains the restored workspace and a pending, actionable receipt.
Audio contents are reread from disk; they are not restored from the setup snapshot.

## Restore the configuration from before import

Open **File → Restore setup…** to read the saved full report and current state.
Choose **Restore previous setup** to replace later managed configuration changes
with the pre-import snapshot. Save/discard pending audio edits deliberately
before proceeding. This is separate from the audio-file **Undo** command.

The local restore point contains the seven managed configuration files
(settings, actions, exports, columns, tag panel, directories and remembered
lists), the physical `export/` tree, and an explicit allowlist of application
window preferences plus the typed local workspace. Files created by the import
are removed when they were absent before it; prior managed file bytes are
restored. Field/history lists therefore retain their saved names/favorites,
and export definitions retain their actual template files.

API credential stores, installed source files, the library cache, audio-file
history and document-provider permissions are outside this restore point and
remain untouched. Browser globals and arbitrary storage keys are not captured.
The snapshot is local under `.profile-migration` in the current configuration
folder; it can contain personal paths and definitions and is not a public share
format. A later completed import replaces the offered restore point.

Native file recovery and browser/window restoration are separate phases. A
failed file replacement attempts rollback; an interrupted applying/restoring
phase recovers before configuration-backed frontend services initialize.
Unexpected external edits, a damaged journal or a failed rollback retain the
snapshot and show a recovery error rather than overwrite unknown content.
Browser-storage failures keep their workspace receipt pending. Read the error
and use the offered reload/retry; do not delete the recovery journal to hide it.

The older **Save configuration… / Load configuration…** JSON format stays
compatible. Its [seven-file backup](../customize/configuration.md) has a smaller
scope and does not include the new local recovery journal, physical export
files or all browser/workspace state. Keep ordinary named backups and their
external resources separately.

## What remains to qualify

The populated automated fixture is independently authored. It exercises the
supported formats and negative/recovery cases; it is not a captured original
Mp3tag profile or a Windows differential comparison. Global/binary settings,
unsupported mappings, native desktop picker/restart interaction, and any real
profile-specific differences still need separate evidence. No complete R02 or
full-parity claim follows from this import workflow.
