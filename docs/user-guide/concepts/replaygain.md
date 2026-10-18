# ReplayGain and loudness

Why can two tracks have the same peak level but sound different in volume?
Peak amplitude and perceived loudness describe different things. ReplayGain
stores measured playback adjustments as metadata; a supporting player can
use them when playing the audio. Editing those fields is different from
measuring the recording or changing the encoded audio samples.

## The four common fields

| Field | Meaning |
|---|---|
| `REPLAYGAIN_TRACK_GAIN` | Gain adjustment measured for this track. |
| `REPLAYGAIN_ALBUM_GAIN` | Gain adjustment measured for the album as a group. |
| `REPLAYGAIN_TRACK_PEAK` | Peak amplitude recorded for this track. |
| `REPLAYGAIN_ALBUM_PEAK` | Peak amplitude recorded for the album. |

Track mode levels individual songs. Album mode uses a common adjustment to
retain the loudness differences within an album. Negative gain means reduce
level; positive gain means increase it. Peak metadata can help a player
avoid clipping after gain, subject to its settings. See the
[ReplayGain specification](https://wiki.hydrogenaudio.org/index.php?title=ReplayGain_1.0_specification)
for the measurement and playback model.

A stored gain is not a volume percentage, a peak reading, or the track's
bitrate. Changing the number by hand does not run a loudness analysis.
Whether playback uses track gain, album gain, additional preamp gain or no
ReplayGain depends on the player. TagGroove's field display is not a playback
meter.

## Read the stored text before changing it

The following values are invented metadata for a practice copy. They are
not measurements of the guide's example album and should not be copied to
real tracks as a normalization recipe.

<!-- replaygain-concept-fields:start -->
| Field | Practice value |
|---|---|
| `REPLAYGAIN_TRACK_GAIN` | `-4.25 dB` |
| `REPLAYGAIN_ALBUM_GAIN` | `-3.00 dB` |
| `REPLAYGAIN_TRACK_PEAK` | `0.950000` |
| `REPLAYGAIN_ALBUM_PEAK` | `0.990000` |
<!-- replaygain-concept-fields:end -->

TagGroove can inspect these stored fields in **Extended tags**. A custom
column or export can read their placeholders, for example
`%replaygain_track_gain%`. Unknown text fields use the tag family's supported
custom-field path, so storage and player interpretation can differ by format.
ID3v1 has no general custom-field space for this set. Do not assume that an
arbitrary similarly named field is an alias for another loudness scheme.

For a deliberate metadata correction, **Keep changes** stages Extended tags
edits and **Save tags** writes them. Actions and converters have their own
write timing. Reopen/read back the saved file before deciding that a player's
lack of volume change is a failed tag save. It may be ignoring the field or
using a different playback mode. Use an audio-analysis tool that supports
your target workflow to calculate gains from the recording; this guide's
TagGroove workflows edit the stored metadata.

## Preservation during MPEG saves has conditions

An MP3 can have ReplayGain data in an APEv2 tag while TagGroove displays its
ID3v2 tag. A normal save may remove the APE tag according to **Mpeg**
preferences. In that path, when ID3v2 is being written, TagGroove has a specific
safeguard: it copies nonempty `REPLAYGAIN_` and `MP3GAIN_` fields from the
removed, undisplayed APE tag if the full displayed values and existing ID3v2
tag do not already contain that field name. Existing values take precedence;
this is not a conflict-merging or recalculation operation.

These written cases use APE **Read** off, ID3v2 **Read/Write** on, APE
**Write** off and APE **Remove** on. Each practice file starts with ID3v2
`TITLE` = `Paper Boats`; saving `ARTIST` = `Juniper Vale` triggers the normal
field-save path.

<!-- replaygain-concept-migration:start -->
| APE track gain | Existing ID3v2 track gain | Saved ID3v2 track gain |
|---|---|---|
| `-4.25 dB` | Absent | `-4.25 dB` |
| `-4.25 dB` | `-1.50 dB` | `-1.50 dB` |
| Absent | Absent | Absent |
<!-- replaygain-concept-migration:end -->

Do not generalize that safeguard to **Remove tag**, an explicit field
removal action, a save without an ID3v2 target, or every container format.
A broad keep-list action can erase loudness fields that are not listed.
`MP3GAIN_UNDO` is another tool's restoration information, not a ReplayGain
track gain or TagGroove's **Edit → Undo** history. Preserve it when needed.

The storage location matters as well as the displayed text. With the default
MPEG policy, an ordinary save removes an existing APE tag while writing ID3.
Copied `MP3GAIN_` text in ID3 does not establish that a gain tool expecting
APEv2 can still use its restoration information. For a workflow that needs
the native APE fields, enable **Write APEv2** under **Tags → Mpeg** before
saving a disposable copy, then verify the result with that gain tool.
Writing both tag types synchronizes their displayed fields; it does not
preserve independently conflicting titles. The core preservation checks
verify native APE gain fields and undo, but do not execute MP3Gain itself.

## What this means in TagGroove

Before cleaning tags, inspect the actual tag families and stored field names,
and retain original files. Keep track and album measurements together where
your workflow needs both. If the audio was replaced, edited or transcoded,
consider whether the old measurements still describe that recording rather
than copying numbers blindly. After an operation, verify the written fields
and the receiving player's settings separately.

Removing gain metadata does not itself reverse audio changes made by a
different program. TagGroove session undo can restore recorded file operations
while available; it does not reconstruct missing analysis or another tool's
lost recovery information.

## Related

- [Tag formats and read/write policy](tag-formats.md)
- [Extended tags](../guides/extended-tags.md)
- [Mpeg tag preferences](../reference/options/tags-mpeg.md)
- [Remove unwanted fields with a keep list](../cookbook/remove-junk-fields.md)

<!-- verified: media_concept_guide.rs round-trips the four written values/deletion and three undisplayed-APE migration cases against real generated MP3 fixtures under LF/CRLF, retaining audio payloads. Source tag_policy.rs and tags.rs; no loudness analysis or player playback test. -->
