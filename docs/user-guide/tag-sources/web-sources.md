# Web sources (.src)

A web source describes how to search a site/API and turn its response into
proposed tags. It is a text-based parser, not a browser that runs a website's
JavaScript. Prefer a documented JSON/XML API or stable server-rendered text
when one is available.

A search usually has two stages: parse a result list, then parse the selected
release. A direct source can skip the result-list stage and look up one release
identifier. Both routes lead to review: a fetched album is not automatically
the right edition or track mapping for your selected files.

## Install the complete source

1. Obtain the complete source download. Keep its `.src`, `.inc` and
   `.settings` files together and inspect what they are intended to query.
2. Choose **Tag Sources → Install tag sources…** on a desktop with folder
   access. Choose **Folder…** for an unpacked download, or **Files or ZIP…**
   for a ZIP archive or a selection of all companion files.
3. Read the installation result. Missing includes, missing/invalid settings
   schemas and invalid source headers stop the entire update before replacement.
4. Open **Tag Sources** and choose the installed entry. Installation checks
   dependencies and schemas; a real lookup must still qualify the parser and
   provider response.

The installer accepts local source files up to 4 MB each, ZIPs up to 128 MB,
4096 entries, and source paths at most three folders deep. It preserves nested
folders, source filenames, submenu/access-key markers and existing source IDs
when updating the same path with different filename case. ZIP paths must be
relative, without traversal, drive prefixes, symbolic links or repeated names
(including names differing only by case). Downloaded documentation is ignored.

Folder/ZIP updates require the complete collection: an old installed include
cannot satisfy a missing companion in the new download. Explicit individual-file
selections also support incremental updates, reusing installed companions after
validating the combined staged files. The installer stages
all files beside the sources folder before replacement. Existing unrelated
files and `settings.json` values remain; incoming downloads cannot replace that
values file. Saved values still need the same schema root key and value type.
The combined sources folder must fit the 128 MB/4096-entry staging limits;
move large debug captures elsewhere if necessary. Existing symbolic links or
folders deeper than three levels must be moved aside before installation.

After a successful replacement, `data/.opentag-sources-previous` retains the
complete previous sources folder, including its configured values. Another
successful update replaces that snapshot. To roll back manually, close OpenTag,
move `data/sources` aside, and rename `.opentag-sources-previous` to `sources`.
If an interrupted replacement leaves the sources folder missing, OpenTag
restores this snapshot automatically on the next source-list/settings read.
The original downloaded folder/ZIP is never modified.

**Tag Sources → Open sources folder** opens
`data/sources` inside TagGroove's configuration folder. **File → Open
configuration folder** is another way to find it. Files can be copied there
manually; returning focus to TagGroove refreshes the source list. Complete
script sets can occupy subfolders. Keep companions beside their source;
include and schema filenames are found there regardless of case, with the
source root as a fallback. An include cannot name an arbitrary outside path.

Save new source files as UTF-8. Unicode BOMs are recognized; otherwise valid
UTF-8 is used, with Windows-1252 as the legacy fallback. If characters are
wrong, inspect the actual saved bytes and response encoding before changing
field text to compensate.

## Organize menu entries

The filename determines the menu path. For example,
`Practice#&Harbour.src` makes a **Practice → Harbour…** entry, with H as an
access key. `#` separates submenu levels; `&` marks a mnemonic character.
The source's `[Name]` supplies descriptive source information, not a different
filesystem location.

A schema named `Practice#Settings.settings` can create a separate **Practice →
Settings…** item. Source entries and settings entries are separated in the
menu. Source-list problems are reported there too; inspect the problem text
when an expected entry is absent.

## Search, review and apply

Save pending tag edits and select one album's practice files. Open the source,
inspect its search fields and run the search. Their labels, initial values and
URL construction come from `[SearchBy]` and the source headers.

For a result list, verify the visible release details and choose the intended
row. A thumbnail can help identify an edition, but it is not a match guarantee.
A preview address can open more release information. A direct source instead
uses the entered identifier to reach the album page.

At review, check proposed fields, enabled states, artwork and the mapping
between local files and source tracks. A parser's track order is not a promise
that automatic matching or ordering suggestions preserved the intended local
order. Match by known titles/positions when needed. If a source supplies only
album-wide values without a track list, its checked fields apply to every
selected file. For a single-track lookup, select only the intended local file. Apply only the fields and
artwork you intend to change, then read the reported result and file errors.

The source parser does not interpret arbitrary tag fields in its command
arguments as selected-file data. Its named outputs and special contexts have
separate rules; see the [script reference](web-source-reference.md). The
[writing lesson](writing-web-sources.md) builds a small source from local JSON
so the parsing steps can be understood before adapting a live site.

## Configure a source

Open its settings menu item when it supplies a `.settings` schema. String
fields can be free text or choices, number fields hold numbers, and booleans
use checkboxes. Headings and separators organize the dialog without holding
values. **Save** stores the chosen settings; **Cancel** leaves them alone.

Schemas provide defaults. Saved values override those defaults when their
value types match. Newly added settings can therefore use a default without
requiring an old settings file to be edited manually. A source's root schema
`key` identifies its section in the shared `data/sources/settings.json` file;
settings with a different root key remain separate.

`IfVar`, `IfNotVar` and `SayVar` read the configured values. These are not the
same as output buffers or editable audio tags. A source can use a setting to
include an optional COMMENT or choose a response language, but only if its
script actually implements that choice.

## Debug the downloaded text

**Options → Tag Sources** can enable web-source debug output. It retains the
downloaded input and a parser trace under the sources folder's `debug`
subfolder. A script can also request `Debug` or `DebugWriteInput`; the
reference explains filenames, caps and the difference between original and
transformed input.

Read a failure in this order:

1. Check the source file and parser section named in the error.
2. Check the line/command and the input near its cursor.
3. Verify that the downloaded text actually contains the expected data.
4. Simplify to one field, then add navigation and loops one step at a time.

If the site returns a sign-in page, rate-limit response or JavaScript shell,
changing `FindLine` offsets will not create the missing album data. Check the
provider's API/authentication requirements. A web source does not inherit an
unrelated browser's login session merely because the site works there.

## Compatibility and limits

A third-party source can rely on headers, parser details or remote behavior
that differs in TagGroove. Minimum-version declarations written for another
application do not certify TagGroove compatibility. Check the actual command
reference and test a small copied selection.

Keep unsupported headers and matching assumptions visible while adapting a
source. In particular, do not infer ordering guarantees or duration-matching
accuracy from successful parser output alone. Those belong to release
conversion and review/matching as well. The examples in this guide establish
specified pure parser results, not every provider or native-platform workflow.

Only web addresses accepted by the source workflow can be fetched. A release
address must come from the current search/lookup, so if it becomes stale,
search again rather than trying to reuse an old internal result. Offline
fixtures and a successful HTTP response are different from a verified live
provider import.

## Further resources

The official Mp3tag Community has [Web Sources Scripts](https://community.mp3tag.de/c/development/web-sources-scripts/12)
for contributed sources and [Web Sources Discussion](https://community.mp3tag.de/c/development/web-sources-discussion/15)
for authoring questions. Treat examples there as source material to inspect,
adapt and test against TagGroove's current commands and request behavior;
community publication does not establish unchanged compatibility here.
