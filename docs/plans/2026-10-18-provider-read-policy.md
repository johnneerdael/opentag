# Provider-document read preferences

R19 requires Android provider reads to honor the existing tag-kind and field
mapping preferences. `Store::entry`, reload preflight and edited-copy summaries
currently use tag-core's default readers. Desktop settings readers already
pass the selected options and mappings explicitly.

Keep transport, durable records, recovery and source-publication rules intact.
Give the store an options supplier, passed as `settings::tag_options` by the
Android adapter. Read the current options on each read rather than caching the
configuration at startup. Use the same captured options for a working-copy read
and its permitted verified-baseline fallback. Apply the shared core's mapped
full/summary readers at every provider read site; retain provider URI projection.

Verify the real persistence engine with filesystem-backed provider fixtures:
conflicting MPEG tag kinds, custom field mappings, changing options after import,
reload, post-publication descriptions and restart recovery. Confirm unchanged
provider bytes for read-only operations and preserve existing failure tests.
These tests do not establish Android device or cloud-provider qualification.

Do not modify Save a copy's artwork behavior here; the separate frozen PR #53
owns that change. Keep this branch local/draft and aligned with #47. No merge,
ready transition, review request or CI dispatch during the branding hold.
