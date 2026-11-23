# Security

## Trust boundaries

`client → MCP server → Gramps Web API` — two separate boundaries, each with its own, independently-configured auth:

1. **Client → MCP server**: is this caller allowed to use this server at all? Only meaningful over HTTP — stdio has no such boundary, see below.
2. **MCP server → Gramps Web API**: which Gramps Web identity does the server act as on this caller's behalf? Applies the same way regardless of transport.

The two are independent, with one exception: turning OAuth on answers both at once (see the OAuth rows below).

## stdio

No network exposure, so no boundary 1 at all. The server is a child process of the AI client, talking over stdin/stdout; the OS process boundary is the only one there is. Boundary 2 (which Gramps identity) still applies exactly as below.

## Boundary 1: client → MCP server (HTTP only)

| Mode         | Config                                               | Revocation                                                                                                                                                                                                | When to use                                                                                                                   |
| ------------ | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| None         | neither `MCP_AUTH_TOKEN` nor `MCP_OAUTH_ENABLED` set | There's no token to revoke — anyone who can reach the port already has full access. The server logs a warning ("HTTP auth disabled — set MCP_AUTH_TOKEN for public deployments") when it starts this way. | Never on the open internet. A trusted LAN/VPN only.                                                                           |
| Static token | `MCP_AUTH_TOKEN`                                     | Change it and restart.                                                                                                                                                                                    | Single user or a small trusted group, simplest to set up.                                                                     |
| OAuth        | `MCP_OAUTH_ENABLED=true`                             | Per session — see boundary 2's OAuth row below; this server has no separate per-session revoke of its own.                                                                                                | Several people, where each should also keep their own Gramps Web role (forces boundary 2 into per-user mode too — see below). |

## Boundary 2: MCP server → Gramps Web API

| Mode              | Config                              | Identity                                                                 | Revocation                                                                                                                                                                            |
| ----------------- | ----------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Username/password | `GRAMPS_USERNAME`/`GRAMPS_PASSWORD` | One fixed service account, shared by every caller that passes boundary 1 | Change the account's password in Gramps Web and restart with the new one.                                                                                                             |
| Sync token        | `GRAMPS_SYNC_TOKEN`                 | Same — one fixed service account                                         | Revoke the token itself from Gramps Web's Access tokens screen (Settings → User settings → Account → Access tokens) — no restart, no `SECRET_KEY` rotation, scoped to that one token. |
| Per-user (OAuth)  | `MCP_OAUTH_ENABLED=true`            | Whichever Gramps Web account signed in at `/authorize` for that session  | Disable or delete the user in Gramps Web — takes effect within ~15 minutes. Changing just their password does **not** revoke access.                                                  |

Choose exactly one:

1. **Username/password** — kept for backward compatibility; prefer sync token or OAuth instead.
2. **Sync token** — exchanged for a short-lived JWT via `POST /api/token/sync/`, re-exchanged automatically as that JWT expires, nothing to refresh manually. Narrower blast radius than username/password if it leaks: the exchanged JWT can read/write tree data but can't change the account's own password/email or manage its access tokens, which a full password login can. Requires Gramps Web 26.10.0+ (gramps-web-api 3.23+ — see [gramps-project/gramps-web-api#1032](https://github.com/gramps-project/gramps-web-api/pull/1032), which added it).
3. **OAuth** — per-user; only exists when boundary 1 is also OAuth.

Username/password and sync token can pair with any boundary-1 mode above.

## OAuth details

Authorization Code + PKCE (`code_challenge_method=S256` only — `authorize.rs` rejects anything else) + Dynamic Client Registration. A user signs in with their own Gramps Web username and password on the server's own `/authorize` form; the server exchanges those credentials once against Gramps Web's `/api/token/` and keeps only the **refresh token** it gets back — the password itself is never stored.

The on-disk store (registered clients, the mapping to each user's Gramps refresh token) is encrypted with AES-256-GCM under `MCP_OAUTH_STORAGE_KEY`. Within that store, access tokens are kept in memory only, never written to disk — a server restart drops all of them, so every client has to use its refresh token to get a new one, not sign in again. Losing or rotating `MCP_OAUTH_STORAGE_KEY` is more disruptive than that: the server can no longer decrypt the existing store file at all and **fails to start**. To actually rotate the key, also clear (or repoint `MCP_OAUTH_STORAGE_PATH` away from) the old store file — that starts clean, and every user has to sign in again.

Dynamic Client Registration (`/register`) is unauthenticated by design (RFC 7591) — anyone who can reach it gets a `client_id`. That alone grants nothing: `/authorize` still requires a real Gramps Web username and password.

Token lifetimes are in [endpoints.md](endpoints.md).

## MCP_ALLOWED_HOSTS

DNS-rebinding protection: the server only accepts requests whose `Host` header matches this list (default: `localhost`/`127.0.0.1`/`::1` only). Required whenever a reverse proxy sits in front of the server, since the proxy forwards the client's original `Host` header, not `localhost`.

## TLS

Required whenever the endpoint is reachable from the internet — bearer tokens travel in a plain header, and in OAuth mode so does the user's Gramps Web password during the `/authorize` login. The server itself only speaks plain HTTP; TLS termination is the reverse proxy's job (see [setup-server-public.md](setup-server-public.md)).

## Origin validation and CORS

Most MCP clients don't run in a browser and don't send an `Origin` header at all — these are unaffected by anything below.

For requests that do carry an `Origin` header (i.e. from a browser), two independent checks apply:

- `/mcp` itself rejects a disallowed `Origin` with `403`, regardless of CORS. By default (`MCP_ALLOWED_ORIGINS` unset), any present `Origin` is rejected. Set `MCP_ALLOWED_ORIGINS` to a comma-separated allow-list (each entry needs a scheme, e.g. `https://app.example.com` or `http://localhost:*` for any port) to let specific browser-based clients — e.g. MCP Inspector — through.
- The server only sends CORS headers (`Access-Control-Allow-Origin`, etc.) when `MCP_ALLOWED_ORIGINS` is set, and only for origins on that list — never a wildcard. With it unset, no CORS headers are sent at all, so a browser page can't read any response even if something else let the request through.

A plain HTML form submission to `/authorize` (the OAuth login page) is not subject to either check in a way that would break it: it's a top-level navigation, not a `fetch`/XHR, so CORS doesn't apply, and `/authorize` isn't behind the `/mcp` Origin check.

## Unauthenticated routes

`/health`, and in OAuth mode the discovery documents, `/register`, `/authorize`, and `/token` — see [endpoints.md](endpoints.md) for the full list. `/authorize` is the one of these that still gates on something: a real Gramps Web login.

## Read-only mode is not a security boundary

`GRAMPS_READONLY=true` hides write tools from the AI client at the MCP layer only. Anyone with shell or network access to the server's own Gramps credentials (the static service account, or a stolen OAuth session) can call the Gramps Web API directly and bypass it entirely. The only real control is the Gramps Web role attached to those credentials — see [roles.md](roles.md).

## Reporting vulnerabilities

Please report security issues via [GitHub private security advisories](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/security/advisories/new) rather than a public issue. See [SECURITY.md](../SECURITY.md).
