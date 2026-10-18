# Android

## Open a document through Android

Choose **Add files** and select audio files in Android's document picker.
The picker grants access to individual documents, including documents exposed
by another app. A document address such as `content://…` is not an ordinary
filesystem path.

TagGroove keeps a private working copy and a verified baseline. A successful
save publishes the edited copy back through the provider and verifies the
result. A read-only document requires **Save a copy** instead. Provider
permissions and availability can change after selection; keep the original
and any recovery copies until the destination has been checked.

## Choose which tags to read

Provider-document reads use the current **Options → Tags** preferences,
including [Mpeg](../reference/options/tags-mpeg.md),
[Ape, Mpc](../reference/options/tags-ape-mpc.md) and
[Mapping](../reference/options/tags-mapping.md). These preferences apply
when adding files, reloading them and describing a completed save or retained
recovery copy.

For example, an MP3 can contain different titles in its APE and ID3v2 tags.
With both kinds enabled, APE takes priority. To inspect the ID3v2 title:

1. Open **Options → Tags → Mpeg**.
2. Leave ID3v2 reading enabled and disable APE reading.
3. Return to the file and choose **Reload tags**. Confirm discarding pending
   edits only if those edits are no longer needed.
4. Inspect the title and tag-kind information before saving anything.

Changing the read checkboxes does not remove the other tag. The independent
write and removal checkboxes control later changes on disk. See the
[tag-policy reference](../reference/options/tags-mpeg.md) before choosing a
cleanup policy.

A custom mapping changes the field name exposed to the editor. For instance,
a mapping from `ARTIST` to `WRITER` shows that native value under `WRITER`;
it does not translate the text or calculate a new value. Reload an already
opened document after changing mappings. The displayed document name and
provider address remain the selected document's identity, rather than the
private working-copy path.

## Recover after an interrupted save

If TagGroove reports that a document needs recovery, stop retrying writes to
that source. An interruption, changed source or lost permission can leave the
provider's current contents uncertain. The app retains its verified baseline
and available edited working copy and blocks further writes to that source.

Use **Save a copy** to recover the appropriate version to an accessible
destination. **Original backup** uses the retained baseline; **Edited copy**
uses the retained working bytes. Check the saved fields and artwork before
discarding either retained version. An unreadable edited copy is not silently
presented as successful recovery: where a baseline can be shown instead, the
document still remains marked as requiring recovery.

## Current limits and verification

Provider documents do not currently support ordinary Undo/Redo, playlist
loading or creation, CSV/export operations, or file/cover-file/rename actions.
Directory-tree and cloud-provider behavior still needs broader qualification.
The compact layout does not turn a provider address into a desktop path.

The read-policy fix has filesystem-backed engine regression coverage. That
coverage does not certify an Android device, document provider, permission
grant or interrupted save on a real phone. The
[verification receipt](https://github.com/johnneerdael/TagGroove/blob/main/docs/verification/provider-read-policy.md) records
the exercised cases and remaining device checks.
