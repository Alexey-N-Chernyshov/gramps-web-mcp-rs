# OQL — Object Query Language Reference

## Overview

OQL is a Python-based query language that operates directly on Gramps objects.
Queries use standard Python syntax with dot notation, comparison operators, and built-in functions.

The current object is referred to by its lowercase type name (`person`, `family`, `event`, etc.)
or by the generic `obj` when the type is unknown.

## Operators

| Operator | Meaning |
|----------|---------|
| `==` | Equality |
| `!=` | Inequality |
| `<` `<=` `>` `>=` | Comparison |
| `in` | Substring / membership test |
| `not in` | Negated membership |
| `and` `or` `not` | Boolean combinators |

## Object methods and properties

Access Gramps object attributes and methods directly via dot notation.

**Common properties:**
- `gramps_id` — string ID (e.g. `"I0001"`)
- `private` — boolean privacy flag

**Common methods:**
- `get_note_list()` — list of note handles
- `get_media_list()` — list of media references
- `get_tag_list()` — list of tag handles
- `get_citation_list()` — list of citation handles

## SimpleAccess helper (`sa`)

The `sa` object provides high-level accessor methods:

| Method | Returns |
|--------|---------|
| `sa.name(person)` | Full name string |
| `sa.first_name(person)` | First name string |
| `sa.surname(person)` | Surname string |
| `sa.gender(person)` | Gender string (`"male"`, `"female"`, `"unknown"`) |
| `sa.birth_date(person)` | Birth date string |
| `sa.death_date(person)` | Death date string |
| `sa.birth_place(person)` | Birth place string |
| `sa.death_place(person)` | Death place string |
| `sa.spouse(person)` | First spouse object (or `None`) |
| `sa.children(person)` | List of child objects |
| `sa.parents(person)` | List of parent objects |
| `sa.father(person)` | Father object (or `None`) |
| `sa.mother(person)` | Mother object (or `None`) |
| `sa.get_note(handle)` | Note object by handle |
| `sa.get_event(handle)` | Event object by handle |
| `sa.get_place(handle)` | Place object by handle |

## Object type names

`person`, `family`, `event`, `place`, `citation`, `source`, `repository`, `media`, `note`, `tag`

## Examples

```python
# Person by Gramps ID
person.gramps_id == "I0001"

# Surname contains a string
"Ivanov" in sa.surname(person)

# First name match (Cyrillic works too)
"Ив" in sa.first_name(person)

# Find people married to someone named Donna
sa.first_name(sa.spouse(person)) == "Donna"

# People with at least one note
len(person.get_note_list()) > 0

# People whose notes mention "immigrant"
any("immigrant" in sa.get_note(h).get_styledtext().get_string()
    for h in person.get_note_list())

# Private events
event.private == True

# Events with a description containing "birth"
"birth" in str(event.get_description()).lower()

# Families with more than 3 children
len(family.get_child_ref_list()) > 3

# Generic query (type-independent)
"42" in obj.gramps_id
```
