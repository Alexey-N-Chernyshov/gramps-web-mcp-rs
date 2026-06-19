# GQL — Gramps Query Language Reference

## Syntax

`property operator value` — optionally combined with `and`, `or`, and parentheses.

String values **must be quoted** with double quotes (required for non-ASCII / Cyrillic).
Numbers and booleans are unquoted.

## Operators

| Operator | Meaning |
|----------|---------|
| `=` `!=` | Equality / inequality |
| `>` `>=` `<` `<=` | Comparison (strings and numbers) |
| `~` `!~` | Contains / does not contain |
| *(no operator)* | Boolean — truthy check |

## Special properties

- `length` — length of an array: `media_list.length > 0`
- `any` / `all` — quantifiers: `event_ref_list.any.ref.get_event.description ~ "Birth"`
- `get_person`, `get_event`, `get_place`, … — dereference a handle to follow relations
- `[N]` — array index: `child_ref_list[0].ref`

## Object types

`person`, `family`, `event`, `place`, `citation`, `source`, `repository`, `media`, `note`, `tag`

## Common properties by type

**person** — `gramps_id`, `gender` (0=unknown 1=male 2=female), `private`,
`primary_name.first_name`, `primary_name.surname_list[0].surname`,
`birth_ref_index`, `death_ref_index`, `event_ref_list`, `family_list`,
`parent_family_list`, `media_list`, `note_list`, `tag_list`

**family** — `gramps_id`, `father_handle`, `mother_handle`,
`child_ref_list`, `child_ref_list.length`, `event_ref_list`, `tag_list`

**event** — `gramps_id`, `type.string`, `description`,
`date.dateval[2]` (year), `date.modifier` (0=normal),
`date.sortval`, `place`

**place** — `gramps_id`, `title`, `name.value`, `place_type.string`, `lat`, `long`

**note** — `gramps_id`, `type.string`, `text.string`, `private`

**source** — `gramps_id`, `title`, `author`, `pubinfo`, `abbrev`

**citation** — `gramps_id`, `page`, `confidence`, `source_handle`

**media** — `gramps_id`, `path`, `mime`, `desc`

**repository** — `gramps_id`, `name`, `type.string`

**tag** — `name`, `color`, `priority`

## Examples

```
# Surname contains "Ivanov" (always quote strings)
primary_name.surname_list[0].surname ~ "Ivanov"

# First name starts with a Cyrillic prefix
primary_name.first_name ~ "Ив"

# Private notes mentioning "David"
private and text.string ~ "David"

# Families with more than 5 children
child_ref_list.length > 5

# Events after year 1900 with exact date
date.modifier = 0 and date.dateval[2] > 1900

# People with at least one media reference
media_list.length > 0

# Objects with a note containing "immigrant"
note_list.any.get_note.text.string ~ "immigrant"

# Families where all children are female
child_ref_list.all.ref.get_person.gender = 2
```
