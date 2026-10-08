# Rules Reference

Used by `get_object`'s `rules` parameter — Gramps' own native filter-rule system (the same
mechanism behind the Custom Filter Editor in the desktop app).

## Shape

```json
{
  "function": "and",
  "invert": false,
  "rules": [
    {"name": "RuleClassName", "values": ["..."], "regex": false}
  ]
}
```

- `function` — `"and"`, `"or"`, or `"one"` (exactly one rule matches). Defaults to `"and"`.
- `invert` — negate the combined result. Defaults to `false`.
- `rules` — one or more rule objects. Each has:
  - `name` — the rule's class name (e.g. `"HasTag"`, `"MatchesQuery"`). Case-sensitive,
    exact match required.
  - `values` — positional parameter values for the rule, as strings/numbers/booleans.
    How many and what they mean depends on the rule — see `get_filter_rules`.
  - `regex` — treat text values as regular expressions instead of literal matches.
    Defaults to `false`.

Omitting `rules` (or passing an empty array) matches everything.

## Recommended default: `MatchesQuery` (GOQL)

Every object type has a `MatchesQuery` rule that takes exactly one value: a GOQL
expression (call `get_goql_reference` for syntax). This is the simplest, most flexible way
to filter with `rules` — for ordinary column/relationship filtering, use this instead of
hunting through the native rule catalog:

```json
{"rules": [{"name": "MatchesQuery", "values": ["gender == Person.MALE and birth.date.sortval >= Date('Jan 1, 1900')"]}]}
```

## The full native rule catalog

Gramps ships dozens of purpose-built rules per object type (e.g. `HasTag`, `HasNote`,
`IsDescendantOf`, date-range rules, …) that `MatchesQuery`/GOQL cannot reach (they query
Gramps' internal object graph directly, not the flat JSON `where_expr` sees). Call
`get_filter_rules` with an `object_type` to fetch the live catalog for that type — each
entry gives the rule's `name`, a human `description`, its parameter `labels`, and `types`
(the expected shape of each value: text, boolean, date, integer, a `select` with fixed
options, a tag name, a Gramps ID, a nested filter name, or `goql` for an inline GOQL
expression). Use that catalog only when `MatchesQuery` genuinely can't express what you
need — it's a much bigger, less predictable surface for an LLM to get right than GOQL.

## Combining rules

```json
{
  "function": "and",
  "rules": [
    {"name": "HasTag", "values": ["Immigrant"]},
    {"name": "MatchesQuery", "values": ["gender == Person.FEMALE"]}
  ]
}
```

## Nesting across object types

A rule item may instead carry its own nested `{"function", "rules": [...]}` (optionally
with `"namespace": "..."` to switch object type mid-filter, only across specific supported
bridges, e.g. Person → Family, Family → Event). This is advanced and rarely needed — prefer
a single `MatchesQuery`/GOQL expression, which can already cross relationships itself
(see `get_goql_reference`).
