# Pinned Windows reference capture: diagnostic first

This bounded R01 batch starts from approved branding base
`c1375a6ccf24e728f3ac29dfd442d4bf1071d47b`. It is independent of the frozen
PR51 save producer/consumer and imports none of its implementation. It adds
a pinned runtime preparer, an explicit Windows-only diagnostic and capture
manifest validation. It does not add a workflow hook or execute Windows CI.
The merge/review/CI hold remains in force.

## Verified acquisition

The [official download page](https://www.mp3tag.de/en/download.html) links
through its [download landing page](https://www.mp3tag.de/en/dodownload64.html)
to this exact versioned HTTPS URL:

`https://download.mp3tag.de/mp3tag-v3.36.1-x64-setup.exe`

On 18 October 2026, a verified-TLS download and 7-Zip extraction under
`$TMPDIR/re/mp3tag-3.36.1-reference-download` produced:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| Official x64 installer | 6,008,656 | `a977861003a87b1f00f2e0c91263c61bf12d457c6d457b367ce5f69de28fbea0` |
| Extracted `Mp3tag.exe` | 25,073,528 | `484794eec4e2b9a649d04eaab63b4cc73127ceb48a9e46075ea488c0e1eac911` |

The extracted executable matches the supplied original exactly. Its PE
identity is x86-64 Windows GUI. The independently recheckable local receipt
is `/tmp/taggroove-mp3tag-reference-feasibility-2026-10-18.json`; it records
the actual local paths, sizes, hashes, official URLs and extraction argv.
These are download/static observations. **Windows was not executed.**

The [official license](https://docs.mp3tag.de/license/) permits use subject to
its conditions and restricts redistribution. This implementation downloads
at runtime or reads an operator-owned installer, verifies both pins before
execution, and keeps all distribution files outside the repository. It does
not relicense or package Mp3tag. Never upload the runtime/download trees,
vendor binaries, source scripts, language files or other shipped resources.

## Why a diagnostic precedes automation

The [official command-line interface](https://docs.mp3tag.de/command-line/)
opens a directory or file, optionally adding it to the current list. It does
not provide a documented batch save/undo/redo interface. The
[keyboard reference](https://docs.mp3tag.de/keyboard/) documents Save, Read,
Undo and Options; the [Edit menu reference](https://docs.mp3tag.de/menus/edit/)
documents Redo sharing its shortcut with Repeat. Focus can also distinguish
text undo from a file operation. Sending a shortcut is not proof that the
intended operation occurred.

The [installation guide](https://docs.mp3tag.de/getting-started/installation/)
documents standard configuration under `%APPDATA%\Mp3tag` and portable
configuration beside the executable. The author's
[portable-mode explanation](https://community.mp3tag.de/t/just-installed-a-script-and-its-not-showing-up-under-tag-sources/46202/11)
associates portable mode with `mp3tag.cfg` beside the executable. The
preparer creates an empty candidate there. Whether this exact version
initializes it correctly, and which configuration path it actually uses,
remain runtime questions. Environment overrides alone do not prove isolation.

The diagnostic uses Microsoft's
[window enumeration](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumchildwindows)
and [UI Automation tree APIs](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-obtainingelements).
It records native classes/control IDs, UIA names/IDs/patterns/toggle states,
process creation identity, session, executable hash and shutdown result.
Every observed window/control is restricted to the newly started PID.
Observation and shutdown recheck creation time and image path. It performs
no selection, editing, option changes, Save, Undo or Redo actions.

## Prepare without executing the reference

Use a new directory inside the OS temporary directory, or `$RUNNER_TEMP`
when running on an authorized GitHub runner. Existing directories are refused.
An installed `7z` or `7zz` is required; the helper installs no tools and never
runs the downloaded installer.

```sh
python3 scripts/reference-capture/collector.py prepare-runtime TEMP_WORKSPACE
```

To use a previously downloaded installer instead of making a network request:

```sh
python3 scripts/reference-capture/collector.py prepare-runtime TEMP_WORKSPACE --installer OWNED_INSTALLER --seven-zip EXISTING_7ZIP
```

Both installer and extracted executable must match the fixed byte counts
and hashes. Redirects to a different URL, mirrors and unversioned/latest
substitutes are refused. Normal TLS verification stays enabled. A local CA
failure is an error; do not disable certificate validation. No successful
runtime preparation record is written for a wrong installer or executable.

`runtime.json` is explicitly `static_runtime_preparation` / `unexecuted`.
Its `mode_observed` and `path_observed` fields are false. Downloaded/extracted
files remain in `download/` and `runtime/`; neither is an evidence artifact.

## Future authorized Windows diagnostic

Do not run this command while the hold is active. After clean review and
separate root authorization for an appropriate Windows runner:

```powershell
powershell -NoProfile -File scripts/reference-capture/probe.ps1 -Workspace TEMP_WORKSPACE -RunReference
```

The probe refuses non-Windows hosts, non-interactive desktops, an existing
Mp3tag process, changed pins, reparse points, a nonempty configuration seed
or prior output. It opens only an owned empty directory. APPDATA,
LOCALAPPDATA, TEMP and TMP overrides apply to the child process only. The
script changes no global environment or registry setup.

The only diagnostic output is `evidence/windows-probe.json`. Native and UIA
enumeration are bounded to 512 records and UIA depth eight. Application
initialization and shutdown waits are bounded; a future runner wrapper also
needs an overall timeout because a UIA provider call can stall. A normal
close is attempted only on the process whose PID, image and creation time
were recorded. Any required forced termination is recorded separately.

A changed, nonempty candidate config after shutdown is evidence of a write
at that path. It does not by itself establish portable mode, selected
defaults, all locations the application accessed, or successful tagging.
The diagnostic keeps mode unverified and defaults null. A first-launch
dialog, missing desktop or unsupported control provider is a diagnostic
outcome to investigate, not permission to invent control selectors.

## Capture manifest and consumer compatibility

After the runtime route is proven, a subsequent authorized batch can map
actual controls, observe **Help → About** and the configuration path, then
capture actual Options selections before tagging. Do not populate defaults
from TagGroove source or static vendor labels. No such controls are mapped
or acted upon by this batch.

The validator accepts only `taggroove.mp3tag-reference/v1` records with
`observation_type: runtime_save_capture` and `status: complete`. Static,
probe-only and synthetic records cannot qualify. A completed manifest needs:

- The pinned distribution identity and actual Windows version, PID, session,
  process creation timestamp and collector source hash.
- Observed portable configuration path/hash, selected MPEG read/write/remove
  switches and ID3v2 encoding, with hashed JSON evidence and timestamps.
- The exact prepared PR51-style schema-1 contract hash and input hashes.
- Exactly one case per prepared file and `edit-one`/`edit-many` operation,
  with actual ordered edit/save/undo/redo/reload operator steps and evidence.
- Every stage at `evidence/OPERATION/STAGE/FILENAME`, its size and SHA-256.

Configuration/default observations must precede tagging during the recorded
process lifetime. Stage files are retained separately; no field decoding or
original exact-byte undo guarantee is inferred here. JSON and recognized
MP3/FLAC/M4A container headers are admissible evidence; executable/runtime
files, changed hashes, missing cases/stages and escaping/symlink paths fail.
Container-header checks are not an audio decoder or metadata qualification.

```sh
python3 scripts/reference-capture/collector.py validate CAPTURE_WORKSPACE --contract PREPARED_CONTRACT
python3 scripts/reference-capture/collector.py export-identity CAPTURE_WORKSPACE --contract PREPARED_CONTRACT --destination NEW_IDENTITY_JSON
```

The exporter writes a compatible identity JSON only. It does not copy media,
runtime files, scratch directories or vendor bodies. Future paired acceptance
must use the validated stage files with the separate PR51 consumer once that
dependency is available and the hold is released. This branch neither imports
nor duplicates that consumer. Manifest validation checks integrity and explicit
operator claims; it cannot authenticate who produced a supplied file. It
does not perform or certify a parity comparison.

## Evidence achieved and remaining

Observed locally: the exact official download/executable hash match; successful
runtime staging marked unexecuted; native-Windows refusal on macOS; PowerShell
syntax parsing and compilation of its native declarations without API calls;
eleven synthetic containment/integrity tests. No reference process was run.

```sh
python3 -m unittest discover -s scripts/tests -p test_reference_capture.py -v
```

Remaining: an authorized interactive Windows run, observed isolation and
portable mode, selected defaults, actual control mapping, save/undo/redo and
reload captures, and original-versus-TagGroove comparison. A future gated CI
hook is a separate proposal. No workflow, trigger, controller, credential or
review authorization behavior changes in this batch.
