# Gramps Web roles

Roles are a property of a Gramps Web user account, defined by `gramps-web-api` (`auth/const.py`), not by this server. In static-token mode, one account's role applies to every MCP client; in OAuth mode, each signed-in user's own role applies to their own session.

| Role        | Value | Can do                                    |
| ----------- | ----- | ----------------------------------------- |
| Guest       | 0     | View public (non-private) data only.      |
| Member      | 1     | + view private records — read everything. |
| Contributor | 2     | + add new objects.                        |
| Editor      | 3     | + edit and delete objects.                |
| Owner       | 4     | + manage the tree and its users.          |
| Admin       | 5     | + manage all trees on the instance.       |

## Minimum role per tool group

Verified against the `gramps-web-api` endpoints the tools call (`require_permissions(...)` in its resource handlers).

| Tool group                                                 | Permission(s) required                                         | Minimum role                |
| ---------------------------------------------------------- | -------------------------------------------------------------- | --------------------------- |
| Read (search, get, query, timelines, relations, tree info) | none for public data; `ViewPrivate` to see private records too | Member (to read everything) |
| Create                                                     | `AddObject`                                                    | Contributor                 |
| Update                                                     | `EditObject`                                                   | Editor                      |
| Delete                                                     | `DeleteObject`                                                 | Editor                      |
| Merge                                                      | `EditObject` + `DeleteObject`                                  | Editor                      |
| Media file replace                                         | `EditObject`                                                   | Editor                      |

## Recommendations

- Read-only use (`GRAMPS_READONLY=true`) → **Member**. Guest also works, but misses private records.
- Normal use (create/update/delete/merge) → **Editor**. It covers every write operation the tools perform; nothing here needs Owner or Admin.
- Never use **Owner** or **Admin** for the MCP account — those add tree- and user-management rights the server never needs, and in static-token mode that power is shared by every client that holds the token.

In static token mode, one account's role applies to everyone using the server. In OAuth mode, each user's own Gramps Web role applies automatically — give each person the role you'd want them to have anyway.
