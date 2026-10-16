# Choosing a setup

**Requirements:** Gramps Web 26.10.0 or newer (gramps-web-api 3.23+), Docker.

The server can run as a local subprocess over stdio, or as a network service over HTTP. Which one you need depends on who has to reach it.

## Who needs to reach the server?

- **Only a local process on this machine** — the client and the server run side by side; nothing is exposed on the network.
- **Devices on your LAN or VPN** — the server listens over HTTP, reachable from your home network or VPN (Tailscale, WireGuard, etc.).
- **Anything on the internet** — the server is reachable over HTTPS through a reverse proxy.

Some MCP clients run in their provider's cloud rather than on your device, and need an internet-reachable HTTPS endpoint regardless of how many users you have — see the wiki's [Clients](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Clients) page for which.

## Where does Gramps Web run?

This machine / a home server / a public host — it maps directly to the three answers above.

## Who will use it?

Just you → one shared Gramps Web service account and a static token is enough. Several people, each with their own Gramps Web account → use OAuth instead; each user's own [Gramps Web role](roles.md) applies automatically.

## Modes

| Mode                      | Reachable from             | Requirements                          | Auth                  | Limitation                                                                                                                                      | Guide                                                             |
| ------------------------- | -------------------------- | ------------------------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| Local (stdio)             | this machine only          | Docker on the client machine          | none                  | —                                                                                                                                               | [setup-local.md](setup-local.md)                                  |
| LAN / VPN (HTTP)          | devices on your LAN or VPN | open port on your LAN/VPN             | none or static token  | —                                                                                                                                               | [setup-server-lan.md](setup-server-lan.md)                        |
| Public — own subdomain    | anything on the internet   | domain + TLS                          | static token or OAuth | —                                                                                                                                               | [setup-server-public.md](setup-server-public.md#own-subdomain)    |
| Public — shared host path | anything on the internet   | TLS (shared with Gramps Web's domain) | static token or OAuth | OAuth discovery documents are served at the host root (`/.well-known/oauth-*/<path>`), so the reverse proxy must route them to this server too. | [setup-server-public.md](setup-server-public.md#shared-host-path) |

## Recommendations

- Just trying it out → [Quickstart](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Quickstart) (wiki).
- Single user, client on the same machine → Local.
- Needs internet access, or several users → Public, with OAuth on its own subdomain.
- Details on auth modes → `security.md`, all variables → [configuration.md](configuration.md).
