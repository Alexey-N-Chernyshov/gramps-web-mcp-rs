Gramps records: person, family, event, place, source, citation, note, media,
repository, tag. A family links a father and/or mother (persons) to child
persons; events (birth, death, marriage, ...) attach to a person or family
and may reference a place; citations attach to a person/family/event/etc.
and reference a source.

Most tools take or return `handle` (the stable internal ID; create_* tools
return one). `get_object` also accepts `gramps_id` (the human-readable ID,
e.g. I0001) as an alternative lookup key.

To find records: use `search` for full-text lookup, or `query_object` for
structured filtering/sorting. Call `get_goql_reference` or
`get_rules_reference` before writing a `where_expr`/`rules` filter, not
after a failed one. Use `query_object`'s `select` and `limit` to avoid
pulling full objects you don't need. Once you have a `handle`, read the
full record with `get_object`.
