---
status: draft
---

# Coming from Mp3tag

!!! abstract "This page is being written"

    It will cover:

    - What is the same (format strings, actions, .mta/.src/.mte compatibility, shortcuts)
    - What differs (⌘ on macOS, pending vs saved, no native menus, missing features list)
    - Importing usrfields.ini, action groups, export configs, web sources

    Until then, the [reference section](../reference/index.md) has the details.

## Bring an action group safely

Keep a copy of the original `.mta` file, then choose **Actions → Actions… →
Import .mta…**. Inspect the imported actions before enabling or running the
group. An unknown action type or setting becomes an **Unresolved imported
action**, and the imported group starts disabled.

Use **Export .mta…** to retain those unresolved settings for later inspection
or recovery. Encoded line breaks and literal backslash text survive export
and re-import. The action remains unresolved; this does not prove that its
settings work in TagGroove or that the original app accepts the exported
file. Export preserves decoded values rather than the original bytes, section
numbers or escape spellings. Recognized actions with actual multiline
settings still report an export error.

See [Import and export an action group](../actions/index.md#import-and-export-an-action-group)
for encoding choices and unresolved-action handling. Importing or exporting
a group does not run its actions against audio files.
