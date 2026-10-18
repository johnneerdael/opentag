# Provider-document read policy

R19 includes a confirmed mismatch: provider-document import, reload, recovery
and post-publication descriptions used default tag-core readers, so selected
tag-kind read switches and custom field mappings did not apply. The Android
store now receives the existing `settings::tag_options` supplier and uses the
shared full/summary readers with those options and mappings. Options are read
on demand, rather than captured once at store initialization. The working-copy
and verified-baseline fallback use one captured policy for the entry.

## Reproduction and regression scope

Two new filesystem-backed persistence-engine tests failed before the reader
change: a carrier with conflicting MPEG APE/ID3v2 titles returned the default
APE title instead of the selected and mapped ID3v2 title, and an existing
handle's summary ignored a newly selected mapping. The local red log is
`/tmp/taggroove-provider-read-policy-red.log`.

The controls exercise import, ready reload, post-publication description,
edited-working summary, changed policy on an existing handle and restart
recovery through a verified baseline when the working file is unreadable.
The current-options control changes the values returned by one stable supplier,
without replacing its function pointer. A third test uses a specification-derived
WavPack block with competing APE/ID3v1 tags to cover the Ape, Mpc read switches
and an APEv2 custom mapping.
They retain provider URI projection and check that read-only operations do
not change provider bytes. The fixtures use hand-built MPEG frames and WavPack
block headers with serialized competing tag kinds; they are not original-Mp3tag
captures or real-encoder qualification.

The two initial focused tests passed after the reader change. The full local
workspace suite then passed 1,084 tests with six existing ignored tests. After
the test refinements, all three focused cases and the affected app library's
409 tests passed, with six existing ignored tests. Strict all-target workspace
Clippy (`-D warnings`), the final app-target Clippy check, formatting, diff checks
and the strict guide build passed. Logs are `/tmp/taggroove-provider-read-policy-green.log`,
`/tmp/taggroove-provider-read-policy-workspace.log`,
`/tmp/taggroove-provider-read-policy-clippy.log` and
`/tmp/taggroove-provider-read-policy-guide.log`; the final test/Clippy receipts
are `/tmp/taggroove-provider-read-policy-refined.log`,
`/tmp/taggroove-provider-read-policy-final-lib.log` and
`/tmp/taggroove-provider-read-policy-final-clippy.log`.

The full suite includes the existing provider failure/recovery tests. No provider
transport, baseline lifetime, publication authorization or artwork-copy policy
is changed here. In particular, Save a copy artwork changes remain in the
separate draft PR #53.

This fix concerns reads and returned descriptions. It does not qualify the
existing Save a copy pending-field writer's mapping/write-policy behavior;
that default writer is a separate concern, not covered by the new read tests.

## Remaining qualification

No Android device, SAF implementation, cloud provider or iOS document route
was exercised by these desktop engine tests. Real-device read/edit/save/reload,
permission loss, interruption and recovery remain open. This change does not
close R19 or establish original-Mp3tag parity.

PR #47's branding merge hold remains active. Final-head Codex review and
authorized current-main CI are still required before any merge.
