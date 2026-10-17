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
record with `get_object`.

`update_*` tools do a shallow merge: pass in `data` only the fields you
want to change, and every other field already on the record is preserved.
A list or nested object you do include (e.g. `citation_list`, `note_list`,
`primary_name`) replaces that value as a whole rather than being merged
field-by-field — to add an item to a list, fetch the current one with
`get_object` first and send back the complete list. If `data` includes
`handle`, it must match the `handle` argument.
