# Security

## Trust boundaries

`client → MCP server → Gramps Web API`. The MCP server holds the credentials that reach Gramps Web: a single service account's username/password (static mode), or, per user, that user's Gramps Web refresh token (OAuth mode). Whoever passes the MCP server's own authentication gets to act as that Gramps identity, with its Gramps Web role's rights.

## stdio

No network exposure. The server is a child process of the AI client, talking over stdin/stdout; the OS process boundary is the only boundary there is.

## HTTP auth modes

| Mode         | Gramps identity used                                                    | Revocation                                                                                                                                                                         | When to use                                                                                                                                                                |
| ------------ | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| None         | The configured service account (`GRAMPS_USERNAME`/`GRAMPS_PASSWORD`)    | There's no token to revoke — anyone who can reach the port already has full access. Set `MCP_AUTH_TOKEN` to close this off.                                                        | Never on the open internet. A trusted LAN/VPN only — the server logs a warning ("HTTP auth disabled — set MCP_AUTH_TOKEN for public deployments") when it starts this way. |
| Static token | Same single service account                                             | Change `MCP_AUTH_TOKEN` and restart.                                                                                                                                                | Single user or a small trusted group, simplest to set up.                                                                                                                  |
| OAuth        | Whichever Gramps Web account signed in at `/authorize` for that session | Disable or delete the user in Gramps Web — takes effect within ~15 minutes. Changing just their password does **not** revoke access. This server has no per-session revoke of its own. | Several people who should each keep their own Gramps Web role.                                                                                                             |

## OAuth details

Authorization Code + PKCE (`code_challenge_method=S256` only — `authorize.rs` rejects anything else) + Dynamic Client Registration. A user signs in with their own Gramps Web username and password on the server's own `/authorize` form; the server exchanges those credentials once against Gramps Web's `/api/token/` and keeps only the **refresh token** it gets back — the password itself is never stored.

The on-disk store (registered clients, the mapping to each user's Gramps refresh token) is encrypted with AES-256-GCM under `MCP_OAUTH_STORAGE_KEY`. Within that store, access tokens are kept in memory only, never written to disk — a server restart drops all of them, so every client has to use its refresh token to get a new one, not sign in again. Losing or rotating `MCP_OAUTH_STORAGE_KEY` is more disruptive than that: the server can no longer decrypt the existing store file at all and **fails to start**. To actually rotate the key, also clear (or repoint `MCP_OAUTH_STORAGE_PATH` away from) the old store file — that starts clean, and every user has to sign in again.

Dynamic Client Registration (`/register`) is unauthenticated by design (RFC 7591) — anyone who can reach it gets a `client_id`. That alone grants nothing: `/authorize` still requires a real Gramps Web username and password.

Token lifetimes are in [endpoints.md](endpoints.md).

## MCP_ALLOWED_HOSTS

DNS-rebinding protection: the server only accepts requests whose `Host` header matches this list (default: `localhost`/`127.0.0.1`/`::1` only). Required whenever a reverse proxy sits in front of the server, since the proxy forwards the client's original `Host` header, not `localhost`.

## TLS

Required whenever the endpoint is reachable from the internet — bearer tokens travel in a plain header, and in OAuth mode so does the user's Gramps Web password during the `/authorize` login. The server itself only speaks plain HTTP; TLS termination is the reverse proxy's job (see [setup-server-public.md](setup-server-public.md)).

## CORS

`src/main.rs` currently sets `Access-Control-Allow-Origin: *` (any origin, any method among GET/POST/DELETE, any header). In practice this matters less than it sounds: every request still needs a valid bearer token (static or OAuth) to do anything, and an arbitrary website's script has no way to obtain one.

## Unauthenticated routes

`/health`, and in OAuth mode the discovery documents, `/register`, `/authorize`, and `/token` — see [endpoints.md](endpoints.md) for the full list. `/authorize` is the one of these that still gates on something: a real Gramps Web login.

## Read-only mode is not a security boundary

`GRAMPS_READONLY=true` hides write tools from the AI client at the MCP layer only. Anyone with shell or network access to the server's own Gramps credentials (the static service account, or a stolen OAuth session) can call the Gramps Web API directly and bypass it entirely. The only real control is the Gramps Web role attached to those credentials — see [roles.md](roles.md).

## Reporting vulnerabilities

Please report security issues via [GitHub private security advisories](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/security/advisories/new) rather than a public issue. See [SECURITY.md](../SECURITY.md).
