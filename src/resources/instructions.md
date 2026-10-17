Gramps records: person, family, event, place, source, citation, note, media,
repository, tag. A family links a father and/or mother (persons) to child
persons; events (birth, death, marriage, ...) attach to a person or family
and may reference a place; citations attach to a person/family/event/etc.
and reference a source.

Most tools take or return `handle` (the stable internal ID; create_* tools
return one). `get_object` also accepts `gramps_id` (the human-readable ID,
e.g. I0001) as an alternative lookup key for a single object.

To find records: use `search` for full-text lookup; `list_objects` for
native Gramps filter rules (relationship, ancestor/descendant, tag-based,
...) or a plain paginated browse; `query_object` for GOQL-based structured
filtering/sorting with column selection. Call `get_goql_reference` or
`get_rules_reference` before writing a `where_expr`/`rules` filter, not
after a failed one. Use `query_object`'s `select` and `limit` to avoid
pulling full objects you don't need. `get_object` and `list_objects` take
`keys` for the same purpose, but only top-level field names (e.g.
`"primary_name"`, not `"primary_name.first_name"`) — a requested nested
field still comes back whole. Empty fields are omitted by default — a
missing field means empty. For overview listings of people, prefer
`profile: ["self"]` together with `keys: ["gramps_id", "parent_family_list"]`
over full objects. Once you have a `handle` or `gramps_id`, read the full
record with `get_object`. When you need several already-known objects
(e.g. the parents of a list of families), use `list_objects`'s `handles`
in one call instead of a series of `get_object` calls.

`update_*` tools apply `data` as a JSON Merge Patch (RFC 7396): pass in only
the fields you want to change, and every other field already on the record
is preserved. A nested object (e.g. `primary_name`) is merged recursively,
field-by-field, so a partial patch like `{"primary_name": {"first_name":
"Petr"}}` changes just that field and leaves `surname_list`/`type`/etc.
alone. Lists (e.g. `surname_list`, `citation_list`, `note_list`) are
replaced whole, not merged — to change one item, read the list with
`get_object` first and send back the complete list. A `null` value removes
a field (Gramps Web may reject removing a required field — to clear a
value, send an empty value instead). If `data` includes `handle`, it must
match the `handle` argument.
