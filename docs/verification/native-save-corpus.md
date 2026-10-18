# Native metadata preservation extension

This bounded R01 batch extends the producer/corpus in frozen PR51 head
`f59edd7b7fbf6d6679c8623d8aa427619225fb76`, locally aligned with PR47 head
`36d7dea4e7b930d3a7d98facaca8498d0ef7c8de`. It reuses the existing synthetic
carriers, save/undo/redo runner and comparison consumer. It changes no
application save policy, field codec or default. It includes no reference
collector, Windows execution or original-app expected outputs.

## Starting coverage and the demonstrated gap

The frozen six-case corpus already covers two ordered artist/custom values,
one lyrics value, a WMP rating convenience field, track ReplayGain gain/peak,
two front/back images, format-specific opaque metadata, audio payload
preservation, exact local undo/redo and rejected NUL writes. Its separate
MPEG probe covers APE display priority, configured APE removal versus writing,
MP3Gain undo/minmax text and a binary companion. Native APE gain-consumer
compatibility remains unexecuted.

The broader repository already has native rating/lyrics/UFID codecs and
described/multilingual frame tests, including WAV and optionally generated
fixtures. Those are implemented features; this batch does not rebuild them.
What the paired preservation corpus lacked was independent observation of
native owner/group/counter data during unrelated saves.

Before code changes, a disposable frozen-producer MP3 was altered only in
the WMP POPM play counter, from zero to one. The full existing snapshot stayed
identical: displayed fields, covers, opaque metadata, audio and tag kinds.
The raw POPM body and whole-file hashes changed. The WMP star projection
does not include its play counter, and the previous native ID3 observation
selected only `Frame::Binary`. The receipt is
`/tmp/taggroove-popm-observer-gap-2026-10-18.json`.

This proves an observer gap. It does not prove that TagGroove corrupts a
counter, that Mp3tag behaves a particular way, or that a player's rating
scale matches the field projection.

## Bounded acceptance matrix

| Inputs | Edit operations | Added preservation assertions |
| --- | --- | --- |
| Existing MP3, FLAC and M4A carriers | Existing `edit-one` and `edit-many`; six cases | All four track/album ReplayGain gain and peak values, alongside the existing multivalues, lyrics, rating field, covers and opaque data. |
| `native-groups.mp3`, built from the same frozen MP3 carrier | Same unrelated edits; two cases | Three POPM owners with nonzero counters, including an eight-byte counter; two USLT language/description groups; a generic binary UFID; two front covers with distinct descriptions and images. |

The current producer therefore creates **eight** ordinary save cases.
`edit-one` changes only TITLE. `edit-many` changes TITLE/ARTIST and removes
ALBUM. Neither operation edits ratings, lyrics, identifiers, gain values or
artwork. Their original native payloads must survive these unrelated edits.
Every case retains the existing exact-byte undo/redo and rejected-write
checks. The configured MPEG tag-kind removal probe stays separate: an
intentional removal is not presented as an untouched-frame retention case.

## Authored inputs and independent byte provenance

`crates/tag-core/tests/fixtures/save-corpus/native-id3-profile.json` is the
authored profile. Native Lofty constructors build its frames on the frozen
independently encoded carrier. The profile is not an original-generated
file and contains no vendor or user metadata. Its owner strings and rating
bytes are preservation test values, not new scale/owner compatibility claims.

A separate raw ID3 envelope reader in
`crates/tag-core/tests/support/native_id3.rs` observes POPM, USLT, UFID and
APIC bodies without TagGroove's mapper or Lofty's frame decoder. It retains
every occurrence, its flags and complete raw body. It also exposes owner,
rating byte/play counter, language/description/text, binary identifier and
cover description/type/image bytes. Sorting allows physical frame reordering while
retaining group cardinality and every payload. Exact physical restoration
is still checked by the existing local undo/redo file hashes.

An independent Python inspection of the generated file decoded the raw
ID3v2.3 envelope and Latin-1/UTF-16 bytes, checked all eight native records
against the authored profile, verified the two PNG payloads and hashed the
actual frame bodies. The receipt is
`/tmp/taggroove-native-id3-byte-inspection-2026-10-18.json`; it includes native
frame offsets, flags, raw body bytes/hashes and decoded input values. The
profile freezes those eight body SHA-256 values. A future codec/fixture
change must supply new independent byte evidence before changing the pins.

The existing observer now includes this native evidence. Python receipts
add per-body and complete native-projection SHA-256 values. The new source
module and authored profile are included in producer source identities.
These are input/preservation identities, not original-app output goldens.

## Controls that must fail

Seven native controls independently alter a counter, owner, lyrics language,
lyrics description, lyrics-group membership, binary UID byte and cover
description. All leave encoded audio unchanged and must alter the independent
native observation. The hidden-counter control additionally requires the
displayed field map to remain unchanged.

Six gain controls remove album gain or album peak independently on each
common format and must be detected. Together with the original twelve
controls, the Rust suite now exercises 25 real metadata/audio loss controls.
These deliberate field removals are negative controls, not claims about
unrelated-save behavior.

The Python paired consumer also receives a synthetic reference copy with
only a WMP counter byte changed. Displayed stars and audio remain identical,
but native payload hashes differ and the comparison must fail. This is a
consumer regression test; no original application produced that copy.

A separate representation control adds a leading zero byte to the same
numeric counter. Owner, rating, counter and all other decoded observations
remain unchanged, but its raw body hash differs. The paired report classifies
this as `serialization_only`, distinct from `semantic_observation` differences.
Independent semantic identities include owner/counter, language/description/
text, UFID owner/binary value and APIC type/description/image bytes. No
original encoding or normalization output is assumed.

`passed` retains its strict observed-equality meaning. A false strict match
alone is not a data-loss or original incompatibility finding. Inspect the
typed native observations, difference classification, configured policies
and raw payloads before drawing a behavioral conclusion.

## Run and interpret

```sh
cargo test -p tag-core --test save_acceptance --locked
python3 -m unittest discover -s scripts/tests -p test_save_acceptance.py -v
python3 scripts/save-acceptance.py prepare NEW_DISPOSABLE_WORKSPACE
python3 scripts/save-acceptance.py run NEW_DISPOSABLE_WORKSPACE
```

Keep the source/build identity fixed between prepare, run and comparison.
The raw native observer qualifies ordinary unflagged ID3v2.3/2.4 frames in
this corpus. It does not qualify ID3v2.2, extended/unsynchronized tags,
compressed/encrypted frames, counters wider than eight bytes or UTF-16
strings without the expected BOM. Unsupported layouts fail observation;
this is not a claim that the application lacks those codecs.

Raw native payload differences can also result from an explicitly configured
encoding/format conversion. Review that policy and the actual bytes before
calling a difference an application bug. This matrix uses one canonical
native input representation and unrelated local edits. Explicit rating or
lyrics edits remain separate codec tests with different expected changes.

Observed locally: all enriched unrelated saves preserve the asserted native
payloads, covers, fields and encoded audio; exact replay remains available.
No application save bug was demonstrated by this extension.

Original selected defaults, original paired save/reload/undo/redo behavior,
native UI/cancellation/interruption, gain consumers, released artifacts and
restart recovery remain unqualified. The R01 initiative and the other
roadmap initiatives remain open. No original-runtime or full-parity
certification follows from this corpus expansion.
