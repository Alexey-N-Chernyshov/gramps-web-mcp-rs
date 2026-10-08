# GOQL — Gramps Object Query Language Reference

Used by the `query_object` tool's `where_expr` parameter.

## Overview

GOQL is a structured, column-based filter expression that compiles to SQL server-side
(`POST /api/<type>/query/`), used by `query_object`. It is **not** the same language as
`oql` (used by `get_object`'s `oql` parameter): GOQL only compares raw Gramps data fields —
it cannot call object methods (no `sa.spouse()`, no `len()`, no `any()`/`all()`). Use
`query_object` + `where_expr` for simple column filters on large trees (much faster); fall
back to `get_object` + `oql` when you need to call a method or walk a computed relationship
`oql` supports but GOQL doesn't.

GOQL operates on the same raw Gramps JSON shape as `oql`'s property access, so
`type.string` is empty for built-in enum types — use `type.value = <integer>` instead (see
the EventType table below).

## Syntax

`property operator value` — combined with `and`, `or`, `not`, and parentheses.
String values **must be quoted** with double quotes.

## Operators

| Operator | Meaning |
|----------|---------|
| `==` `!=` | Equality / inequality |
| `<` `<=` `>` `>=` | Comparison (strings and numbers) |
| `like` | SQL-style pattern match (`%` wildcard) |
| `regex` | Regular-expression match |
| `contains` | Substring / membership test |
| `in` | Membership in a list |
| `and` `or` `not` | Boolean combinators |

## Column paths

Dot notation addresses nested fields; `[N]` indexes into an array; a path segment that
names a relationship (e.g. `father`, `mother`) dereferences across objects:

- `primary_name.surname_list[0].surname`
- `father.primary_name.surname_list[0].surname`
- `gramps_id`, `handle`, `private`, `gender` (0=unknown 1=male 2=female)

## Type fields (`type.value`)

**Common EventType integers:**

| Value | Type |
|-------|------|
| 1 | Marriage |
| 7 | Divorce |
| 11 | Adopted |
| 12 | Birth |
| 13 | Death |
| 15 | Baptism |
| 19 | Burial |
| 21 | Census |
| 22 | Christening |
| 28 | Emigration |
| 30 | Immigration |
| 37 | Occupation |
| 42 | Residence |
| 44 | Will |

> To find a value for an unlisted type: fetch any event of that type and read `type.value`.

## Object types

`person`, `family`, `event`, `place`, `citation`, `source`, `repository`, `media`, `note`, `tag`

## Common properties by type

**person** — `gramps_id`, `gender`, `private`, `primary_name.first_name`,
`primary_name.surname_list[0].surname`, `birth_ref_index`, `death_ref_index`

**family** — `gramps_id`, `father_handle`, `mother_handle`, `child_ref_list`

**event** — `gramps_id`, `type.value`, `description`, `date.dateval[2]` (year),
`date.sortval`

**place** — `gramps_id`, `title`, `name.value`, `place_type.value`, `lat`, `long`

**note** — `gramps_id`, `type.value`, `text.string`, `private`

**source** — `gramps_id`, `title`, `author`, `pubinfo`, `abbrev`

**citation** — `gramps_id`, `page`, `confidence`, `source_handle`

**media** — `gramps_id`, `path`, `mime`, `desc`

**repository** — `gramps_id`, `name`, `type.value`

**tag** — `name`, `color`, `priority`

## Beyond `where_expr`: the rest of the query body

`query_object` also accepts:

- `select` — list of column paths to return; omit to return every column.
- `order_by` — list of `{column, direction}` ("asc"/"desc"), applied in order.
- `limit` — max rows per call (1-1000, default 50).
- `after` — opaque cursor from the previous response's `next_after`, for paging past
  `limit` rows. This is keyset pagination, not an offset/page number — always pass back
  the exact `next_after` value rather than computing your own offset.
- `count` — when `true`, the response includes a `total_count` field (the full match
  count, computed server-side; slightly slower).

## Examples

```
# Surname equals "Ivanov"
primary_name.surname_list[0].surname == "Ivanov"

# First name starts with a Cyrillic prefix
primary_name.first_name contains "Ив"

# All Birth events
type.value == 12

# Death events after year 1900 with exact date
type.value == 13 and date.dateval[2] > 1900

# Private notes mentioning "David"
private == true and text.string contains "David"

# Father's surname is "Smith"
father.primary_name.surname_list[0].surname == "Smith"
```
