# Recorded results

Snowflake results recorded by `sqlquack-record`, one `<case hash>.json` metadata file and, for
row outcomes, one `<case hash>.arrow` Arrow IPC file per case.

These files are append-only. CI rejects any change to or deletion of an existing file. A changed
case has a new hash and is recorded as a new result.
