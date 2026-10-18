# Move lyrics and karaoke files with audio

A separate lyrics or karaoke file can follow an audio file when it has exactly
the same basename. This support is optional and applies to local desktop files.

1. Open **File → Options… → Files**.
2. Enable **Move matching .lrc and .cdg files with audio** and save the options.
   The default is off, including for existing profiles.
3. Select the audio files. Save intended pending tag edits first.
4. Use **Convert → Tag → Filename…** and choose **Preview**, or use
   **Edit → Move files to…** and choose the existing destination folder.
5. Read the companion source and destination paths in the rename preview or
   Move confirmation. Cancel if any path is unexpected.
6. Apply, then inspect the result and any reported errors. Use **Edit → Undo**
   to reverse a recorded move during this session.

For example, renaming `03 - Paper Boats.flac` to `Juniper Vale - Paper Boats.flac`
also renames `03 - Paper Boats.lrc` to `Juniper Vale - Paper Boats.lrc`. A matching
`.cdg` follows in the same way. The contents are retained; this operation does
not edit lyric timestamps or karaoke data.

## Which companions follow

Only `.lrc` and `.cdg` are supported. Extension matching ignores letter case and
retains the original extension spelling. The basename must match exactly:
`Song.flac` matches `Song.LRC`, while `Song.en.lrc`, `Other.lrc`, artwork and other
file types stay where they are. Symbolic-link entries and directories are
rejected. Two audio files sharing one companion, such as `Song.mp3` and
`Song.flac`, are ambiguous and reject the reviewed plan.

Companions do not become editable audio rows. The option currently affects
**Tag → Filename** and **Move files to**. **Copy files to**, **Filename → Filename**,
inline filename editing, Tag → Tag and action-driven renames keep their existing
behavior. Move those companions separately when using one of those commands.

## Review, failures and undo

The reviewed plan freezes companion membership and destinations. Changed or
missing files, a newly appearing companion, a changed policy, an occupied name,
or an expired preview requires another review. Names that differ only in letter
case are treated conservatively to avoid portable collisions. Rename cycles are
rejected rather than temporarily overwriting another member of the selection.

Each audio file and its companions form a move group. A group moves its
companions first and the audio last. If a member fails during execution, the
operation attempts to reverse the completed moves without replacing anything.
A new occupant or an externally edited companion can prevent that rollback;
the error reports **rollback incomplete** and the affected paths. Earlier
successful groups can remain moved. Inspect both locations before retrying.
This is not a single transaction across the collection.

Undo and redo record the physical companion paths as well as the audio paths,
including changes to letter case. Files changed elsewhere are reported and
retained. Other files in the same history step can still reverse, so an undo
error requires inspecting both locations. History is bounded and session-only;
keep independent backups. Copying does not create a transfer Undo step.

## Related

- [Copy, move and delete files](file-operations.md)
- [Tag → Filename](../converters/tag-to-filename.md)
- [Files options](../reference/options/files.md)
- [Undo and history](undo-history.md)
