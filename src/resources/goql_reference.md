# GOQL — Gramps Object Query Language Reference

Used by `query_object`'s `where_expr`, and recommended as the `values` of a `MatchesQuery`
rule in `get_object`'s `rules` (see `get_rules_reference`) — same expression language
either way.

## Overview

GOQL is an "almost Python" expression language that compiles to SQL server-side. Write it
like a real Python boolean expression — comparisons, `and`/`or`/`not`, even comprehensions
for relationship checks — and it is parsed as pure syntax (never executed), then translated
into SQL. It is dramatically faster than browsing a full collection client-side, but it only
understands paths, comparisons, and the specific relationship/collection names below — not
arbitrary Gramps object methods.

## Operators

| Operator | Meaning |
|----------|---------|
| `==` `!=` | Equality / inequality |
| `<` `<=` `>` `>=` | Comparison |
| `is` `is not` | Sugar for `==`/`!=` (value equality, not identity) |
| `in` / `not in` | Membership: `path in [v1, v2, ...]` |
| `"substring" in path` | Substring test — same `in`, disambiguated by a string literal on the left |
| `like(path, 'pattern%')` | SQL-style pattern match — a function call, not an infix operator |
| `regex(path, 'pattern')` | Regular-expression match — also a function call |
| `and` `or` `not` | Boolean combinators, standard Python precedence (`not` > `and` > `or`) |

Either side of `==`/`!=`/`<`/`<=`/`>`/`>=` may be the path — `5 < gender` and `gender > 5`
mean the same thing.

## Paths

A path is a bare identifier optionally followed by `.attr` / `[index]` segments:

- `gender`, `gramps_id`, `private`
- `primary_name.surname_list[0].surname`
- Crosses a relationship by name (see table below): `father.surname`, `birth.place.title`

## Relationships (single-hop, usable anywhere in a path)

| From | Name | To |
|------|------|----|
| Person | `birth`, `death` | Event |
| Family | `father`, `mother` | Person |
| Event | `place` | Place |
| Citation | `source` | Source |
| Place | `enclosed_by` | Place (self) |

## Collections (one-to-many — only as the target of `exists`/`any`/`count`, never in a dotted path)

| Type | Collections |
|------|-------------|
| Person | `families`, `parent_families`, `child_refs`, `associations`, `events`, `notes`, `citations`, `media`, `tags` |
| Family | `children`, `events`, `notes`, `citations`, `media`, `tags` |
| Event | `notes`, `citations`, `media`, `tags` |
| Place | `enclosing_places`, `notes`, `citations`, `media`, `tags` |
| Source | `repositories`, `notes`, `media`, `tags` |
| Citation | `notes`, `media`, `tags` |
| Repository | `notes`, `tags` |
| Media | `notes`, `citations`, `tags` |
| Note | `tags` |
| every type | `backlinks` — objects referring to this one |

## Checking collections: `exists`, `any`, `count`

- `exists(children)` — at least one related row at all.
- `exists(children, given_name == "Steve")` — condition evaluated against the *related*
  type directly, no loop-variable prefix.
- `any(children, given_name == "Steve")` — identical to `exists` for a collection name.
- `any(media_list)` — one argument that is **not** a collection name: "this array has at
  least one element", equivalent to `len(media_list) > 0`.
- `count(children)` / `count(children, given_name == "Steve")` — number of (matching)
  related rows; compare it like any number: `count(children) > 2`.
- **Comprehension sugar** (preferred — reads like real Python): `any(cond for x in rel if
  ...)` desugars to `exists(rel, cond)`; `len([... for x in rel if ...])` desugars to
  `count(rel, cond)`. The loop variable is stripped, so write the condition as if it were
  already evaluated against the related row: `any(x.given_name == "Steve" for x in
  children)` → condition is just `given_name == "Steve"`.

## Type constants — `ClassName.CONST`

Instead of magic integers, compare against the real Gramps class constant; it resolves to
the same value `gender == 1` would, but is self-documenting:

```
gender == Person.MALE
type.value == EventType.BIRTH
date.modifier == Date.MOD_ABOUT
```

Available classes: `Person`, `Citation`, `Note`, `Date`, `AttributeType`, `ChildRefType`,
`EventRoleType`, `EventType`, `FamilyRelType`, `MarkerType`, `NameOriginType`, `NameType`,
`NoteType`, `PlaceType`, `RepositoryType`, `SourceMediaType`, `SrcAttributeType`,
`StyledTextTagType`, `UrlType`. Each exposes its ALL_CAPS members (`Person.MALE`,
`Person.FEMALE`, `EventType.BIRTH`, `EventType.DEATH`, `EventType.MARRIAGE`, ...).

## Dates — `Date('...')`

Parses a human date string into a comparable integer (Julian day number), so it works with
ordinary comparisons:

```
birth.date.sortval >= Date('Jan 1, 1968')
```

## Beyond `where_expr`: the rest of the `query_object` body

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
# Surname equals "Smith"
primary_name.surname_list[0].surname == "Smith"

# First name contains a Cyrillic prefix
"Ив" in primary_name.first_name

# Male persons born after 1900
gender == Person.MALE and birth.date.sortval >= Date('Jan 1, 1900')

# People with at least one child named Steve
any(given_name == "Steve" for x in children)
# equivalently: exists(children, given_name == "Steve")

# Families with more than 3 children
count(children) > 3

# All Birth events
type.value == EventType.BIRTH

# Private notes mentioning "David" (pattern match)
private == True and like(text.string, '%David%')

# Father's surname is "Smith"
father.primary_name.surname_list[0].surname == "Smith"
```
