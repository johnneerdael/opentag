# Save and restore your configuration

Save a configuration before reorganizing your workspace, changing many options,
or moving to another computer. The bundle preserves supported **saved
configuration**. It is separate from saving tag edits and backing up music.

For a user-owned Mp3tag folder/ZIP, use the separate [guided setup import and
local restore point](../getting-started/coming-from-mp3tag.md). Its recovery
snapshot includes physical export templates and supported window/workspace
state. The named JSON backup described here retains its existing smaller scope.

## Make a named configuration backup

1. Finish the configuration dialogs you want to preserve: choose **OK** or
   **Apply** in Options and **Save groups** for edited action groups. Unsaved
   dialog work is not a replacement for saved configuration files.
2. Choose **File → Save configuration…**. Select a backup location and a
   recognizable name, such as `Harbour-workspace-before-tools.json`.
3. Check the status message and the destination file. TagGroove reports how
   many existing supported configuration files it collected. A zero-file
   bundle is not a useful restore checkpoint: empty bundles are rejected on
   load. Finish saving the intended settings/definitions first.
4. Keep this backup separately from the application installation and from
   the audio-file backup. A new application package does not carry your whole
   personal workspace inside it.

Saving configuration does not write pending tag-panel edits to audio. It also
does not guarantee that an in-progress typed history entry or unsaved action
edit has become part of the saved files. Finish those changes deliberately
before treating the bundle as a checkpoint.

## What the bundle includes

TagGroove uses its own JSON bundle, identified internally as
`opentag-configuration`, version `1`. It collects the following files when
they already exist in the current configuration folder:

| Saved file | What it represents |
| --- | --- |
| `settings.json` | Saved Options, including tool definitions, toolbar/layout preferences and other option pages |
| `actions.json` | Saved action groups and their definitions/order/state |
| `exports.json` | Saved export identities/preferences; keep the physical `export/` template files separately |
| `columns.json` | File-list column configuration |
| `tag-panel.json` | Tag-panel field configuration |
| `directories.json` | Favorite and recent directories |
| `lists.json` | Supported remembered helper/history/value lists, including their saved names and favorites |

The list is a whitelist, not a recursive copy of the configuration folder.
An absent component is omitted. The backup filename is your label for the
bundle; it does not rename action groups, tools, favorites or list entries
inside it.

### Keep these separately

- Audio files and their backups, including artwork already embedded in them.
- Pending tag edits and unsaved configuration-dialog changes.
- Discogs access credentials: the access token is deliberately excluded.
- Physical `export/` templates, browser/window preferences and the current loaded file list.
- Installed web-source files, externally referenced images/text files and
  other resources outside the supported saved components.
- Library-cache data, session Undo snapshots, and provider-document access or
  recovery state. A configuration bundle does not transfer a phone/provider
  permission grant to another computer.

Definitions can contain paths to external resources without copying those
resources. Likewise, a configuration can contain private folder names, tool
parameters or service settings. Token exclusion does not anonymize it; inspect
a copy before sharing a workspace publicly.

## Restore without losing your current workspace

1. Save or deliberately discard pending audio edits. A configuration load
   reloads the application window and discards pending tag changes.
2. Save the **current** configuration under another name, such as
   `Harbour-workspace-before-restore.json`. Keep the incoming bundle too.
3. Choose **File → Load configuration…** and select the intended TagGroove JSON
   bundle. Read the replacement warning before confirming.
4. After a successful load, the window reloads. Inspect Options, action groups,
   export definitions, columns, tag-panel fields and favorite directories.
5. Test one copied audio file before applying a restored action group or
   external tool to a larger selection. Restoring its definition is not a
   review of what that definition will do to your current files.

This is not an importer for another editor's ZIP backup. It also does not
combine every saved item from two workspaces. Each included component replaces
its matching saved file; files omitted from the bundle remain in place.

<!-- configuration-restore-cases:start -->
| Target before loading | Incoming bundle contains | Expected saved component afterward |
| --- | --- | --- |
| An existing action library | A different valid `actions.json` | The incoming action library replaces that component; the old groups are not automatically merged |
| Existing favorites | No `directories.json` | The existing directory component is kept |
| Existing custom toolbar settings | An incoming `settings.json` | Incoming saved settings, including their toolbar definition, replace that component |
| Existing columns | An invalid included `columns.json` | Validation rejects the bundle before component replacement begins |
<!-- configuration-restore-cases:end -->

Validation happens for every included component before replacements begin.
An unsupported bundle version, unknown filename, invalid component or empty
bundle is rejected. Import also limits a bundle to 32 MiB. Do not remove
validation errors by renaming an arbitrary archive to `.json`.

A later filesystem/write failure is different: included files are replaced
sequentially, so the whole bundle is not one all-or-nothing transaction.
Keep the before-restore backup, record the error, and inspect the saved state
before retrying. A failed reload or error dialog is not proof that no saved
component changed. Do not delete the entire configuration folder to experiment.

## Move to another computer or operating system

Copy the configuration backup and required external resources, then install a
suitable TagGroove package on the destination. Save its current configuration
before loading the incoming bundle. Recheck anything that names a location:

| Item | What to recheck |
| --- | --- |
| Tool **Path** and parameters | Executable locations and command-line conventions can differ by OS |
| Favorite/recent/startup directories | The old computer's paths may not exist on the new one |
| Actions and exports | Referenced input/output folders, cover files and text resources |
| Web sources and credentials | Install the required source files and reconfigure credentials separately |
| Layout and columns | Available screen space, display preferences and the intended sort/format expressions |

Keep your named definitions and labels recognizable while correcting paths.
A successful configuration load validates the supported saved formats; it does
not prove that an external executable exists, that a service account is
available, or that every referenced file has been copied.

For a reusable personal setup, retain a known baseline and a separate copy
before each substantial experiment. Do not use session Undo as configuration
version control: recorded audio-file history has a different scope.

## Locate the current configuration folder

On desktop, choose **File → Open configuration folder**. This opens the folder
used by the current application, avoiding guesses based on another editor's
standard/portable/Store paths. The application identifies its per-user storage
with the stable identifier `org.opentag.editor`; copying an application bundle or AppImage does not create a new
portable configuration beside that executable.

Opening folders is not available through the inspected mobile command.
Use the build's supported file/document dialogs where offered and keep its
release/access limits in mind. Do not infer desktop configuration-transfer or
save-back support from a familiar-looking menu.

Avoid editing live saved files while TagGroove is writing them. The built-in
load route validates included formats before replacing components; direct
live file changes bypass that validation and have different reload risks. When a configuration cannot load, include the exact
error and TagGroove version in a minimal issue report, without publishing
credentials or a personal library.

## Related

- [Download, updates and configuration basics](../getting-started/install.md)
- [Toolbars](toolbars.md)
- [External tools](tools.md)
- [Editing, saving and undo](../getting-started/editing-and-saving.md)
- [Troubleshooting](../help/troubleshooting.md)
