# Actions and action groups

Actions turn a repeated tagging job into a saved workflow. An action can
change a field, rename a file, adjust artwork or produce an output file.
A group combines actions so that the next step can use the previous step's
result.

Start with copies of a few files. Save or discard pending tag-panel edits,
select the files to process, then choose one of these routes:

| Route | Use it for | What the confirmation does |
|---|---|---|
| **Actions → Actions (Quick)…** (++alt+shift+6++) | One change, such as replacing underscores | **Apply action** runs the action and saves immediately |
| **Actions → Actions…** (++alt+6++) | A reusable sequence | **Save groups** keeps the workflow; **Apply checked groups** saves it and processes the selected files |
| A named group in the **Actions** menu | Running a saved workflow again | Runs that group on the selected files |

Alt is ⌥ Option on a Mac. The menus show the available commands when files
are selected and the current source permits writing.

A new installation starts with five unchecked groups under **Examples**.
Review and enable only the examples wanted for a run. Saving an empty library
keeps it empty; removing examples does not make them return at the next start.

## Choose an action

| Goal | Action |
|---|---|
| Change capitalization | [Case Conversion](case-conversion.md) |
| Repair a confirmed legacy encoding mistake | [Convert Codepage](convert-codepage.md) |
| Calculate one field or a name from tags | [Format Value](format-value.md) |
| Extract several fields from structured text | [Guess Values](guess-values.md) |
| Substitute exact text | [Replace](replace.md) |
| Match varying text with a pattern | [Replace with Regular Expression](replace-regexp.md) |
| Turn a delimited string into several values | [Split Field by Separator](split-field.md) |
| Join several field values into one | [Merge Duplicate Fields](merge-duplicate-fields.md) |
| Retain the first value or remove equal values | [Remove Duplicate Fields](remove-duplicate-fields.md) |
| Delete specified fields | [Remove Fields](remove-fields.md) |
| Delete everything except a keep list | [Remove Fields Except](remove-fields-except.md) |
| Embed an existing image | [Import Cover from File](import-cover.md) |
| Copy embedded images to files | [Export Cover to File](export-cover.md) |
| Resize or convert embedded images | [Adjust Cover](adjust-cover.md) |
| Change picture roles and descriptions | [Set Cover Properties](set-cover-properties.md) |
| Fill a field from a text file | [Import Text File](import-text-file.md) |
| Write a text report | [Export](export.md) |
| Write playlists grouped by folder or another value | [Generate Playlists](generate-playlists.md) |

## Try a quick action

1. Select one practice file whose title contains an underscore.
2. Choose **Actions → Actions (Quick)…**.
3. Choose **Replace** in **Action type**.
4. Enter `TITLE` in **Field**, `_` in **Original string**, and one space in
   **Replace with**.
5. Leave **Only as whole word** and **Case-sensitive comparison** off.
6. Choose **Apply action**, then check the title in the file list.

`Low_Tide` becomes `Low Tide`. This is a saved change; there is no extra
**Save tags** step. TagGroove remembers the last quick-action type and its
settings. Check them before the next application.

## Build a reusable group

1. Open **Actions → Actions…** and choose **New…** under **Groups**.
2. Give the group a clear name, such as `Titles: underscores and spaces`.
3. Select the group and choose **Edit actions…**.
4. Choose **New…**, select an action type, enter its settings and choose
   **Keep action**. Repeat to add more actions.
5. Use **↑ Move up** and **↓ Move down** to put the actions in order.
6. Keep the edited action list, then choose **Save groups**.
7. Check only the groups needed for this run and choose **Apply checked groups**.

A group's name does not need to match a field or filename. Duplicate it
with **Duplicate…** before adapting a workflow you want to preserve.
**Rename…** changes its name; **Remove** removes the saved group, not your
music files. Closing with unsaved group changes asks whether to discard them.
Hold Alt while choosing a move button to send the selected entries to the
beginning or end. The same controls arrange both groups and their actions.

## Understand the order

Checked groups run in their displayed order. Within a group, field and
file changes feed the following actions. For example, copy the guest
artist into `TITLE` before removing it from `ARTIST`.

**Export** and **Generate Playlists** are batch outputs: TagGroove produces
them after the other actions in that group. They see the group's final
values even if their row appears earlier. Use a separate group when an
output must capture a particular intermediate state.

A failed action warns for that file, then the file's remaining actions and
later groups continue. It still participates in batch exports and playlists.
Earlier successful writes remain; read every warning and inspect the practice
files before repeating.
Use **Edit → Undo** for recorded file changes in the current desktop
session; keep backups for recovery beyond that session. Export Cover includes
created and replaced images in its undo step. Generated reports and playlists
may need separate cleanup.

## Choose a field or a special target

Most field actions take a name such as `TITLE`, `ARTIST` or `ALBUMARTIST`.
Use the **Field** helper to insert names. The available special targets
depend on the action:

| Target | Meaning | Available for |
|---|---|---|
| `_TAG` | Process every tag field | Case Conversion, Replace, Replace with Regular Expression, Convert Codepage, Merge Duplicate Fields, Split Field by Separator |
| `_ALL` | Process tag fields and the filename | Case Conversion, Replace, Replace with Regular Expression, Convert Codepage |
| `_ALL` in Remove Duplicate Fields | Process every tag field; does not rename files | Remove Duplicate Fields |
| `_FILENAME` | Process or construct the filename | Case Conversion, Replace, Replace with Regular Expression, Convert Codepage, Format Value |
| `_DIRECTORY` | Process or construct the containing folder's name | Case Conversion, Replace, Replace with Regular Expression, Convert Codepage, Format Value |
| `_FIELDNAME` | Change field-name spelling, keeping its values | Case Conversion, on APEv2 and Vorbis Comment tags |

The regex action includes the extension in `_FILENAME`; the other text
transformations operate on the name without it. Format Value builds a new
name and preserves the audio extension. Read the action page before using
a filename or folder target.

`COVER` and `PICTURE` have a separate meaning in **Remove Fields** and
**Remove Fields Except**: they refer to embedded artwork.

## Organize, select and repeat groups

Use **Sort by name** to alphabetize groups, or **Sort by state** to put
checked groups first. Sorting changes execution order, so review the result.
Checking a group controls whether **Apply checked groups** runs it; selecting
a row chooses which group to edit. Those are different operations.

A **Selection preset** remembers which groups are checked. Check the groups,
choose **Save preset…**, and give the selection a name. Choose that name
from **Selection preset** to restore the checks. **Remove preset…** removes
the saved selection while keeping the groups themselves. Save groups after
changing presets.

For nested menu entries, use `#` in a name: `Cleanup#Titles` places **Titles**
inside a **Cleanup** submenu. A single `&` marks a mnemonic character;
`&&` displays a literal ampersand. Keep names descriptive even without the
mnemonic.

For example, name a group `&Cleanup#&Titles`. Outside a text box, press
++alt+a++ to open the **Actions** menu, then `C` and `T` to run that group.
The letters come from the characters following `&`, not from a separate
arbitrary shortcut assignment.

After a run, **Edit → Repeat** can run the last action selection again.
The **Redo/Repeat** command depends on the current undo history; read its
label before using ++cmd+y++ / ++ctrl+y++.

## Import and export an action group

Choose **Import .mta…** and select an action-group file. UTF-8 and BOM-marked
UTF-16 are accepted; for an older file, enter its original encoding when
asked, such as `Windows-1252`. An imported name already in use receives a
numbered suffix.

Inspect every imported action. An **Unresolved imported action** keeps the
uninterpreted settings visible and prevents checked groups from running.
Replace or correct that action explicitly; importing a file is not proof
that every setting has an equivalent in TagGroove.

Choose **Export .mta…** for the selected group to write an exchange file.
Review any reported error rather than assuming a partial export is usable.
Unresolved imported actions can be exported for preservation: their decoded
settings survive export and re-import, including encoded line breaks. Literal
backslash sequences remain literal text. The action stays unresolved and its
group stays disabled on re-import; export does not assign a supported meaning
to unknown settings.

This preserves setting values, not the original file's exact bytes: section
numbers, property order, encoding and escape spellings can change. Keep the
original `.mta` file when its exact form matters. Settings containing actual
line breaks in recognized actions still report an export error. Acceptance
by the original app for an unknown action or option remains unqualified.

## Keyboard controls in the group list

Use Cmd on macOS and Ctrl on Windows or Linux for the primary modifier.

| Key | Effect |
|---|---|
| ++insert++ | New group or action |
| ++delete++ | Remove the selected entries; Backspace also works on Mac keyboards |
| ++space++ | Toggle checked groups |
| ++ctrl+a++ / ++cmd+a++ | Select all rows |
| ++ctrl+shift+a++ / ++cmd+shift+a++ | Invert the row selection |
| ++ctrl+e++ / ++cmd+e++ | Enable all groups |
| ++ctrl+d++ / ++cmd+d++ | Disable all groups |
| ++ctrl+s++ / ++cmd+s++ | Save groups |
| ++alt+arrow-up++ / ++alt+arrow-down++ | Move selected rows |
| ++alt+page-up++ / ++alt+page-down++ | Move selected rows to the beginning or end |
| ++f2++ in the group list | Rename the current group |
| ++f4++ or ++enter++ in the group list | Edit the group's actions; double-clicking also opens them |
| ++f4++ in the action list | Edit the current action |

Cmd/Ctrl-click toggles individual row selections; Shift-click selects a
range. These controls apply inside the relevant lists, not while typing a
field value.

## Related

- [Format Value](format-value.md)
- [Replace](replace.md)
- [Replace with Regular Expression](replace-regexp.md)
- [Move guest artists to titles](../cookbook/feat-artists-to-title.md)
