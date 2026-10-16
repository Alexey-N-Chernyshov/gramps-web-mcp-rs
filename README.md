# gramps-web-mcp-rs

Connect your MCP client to your [Gramps Web](https://www.grampsweb.org/) family tree, and research, edit and clean it by describing what you want.

<!-- TODO: demo gif -->

## What you can do

- "Find everyone born in Ohio before 1900 whose death date is unknown."
- "Add this family to the tree from the source I'm pasting, with a citation for each fact."
- "Find likely duplicate people in the tree and merge the ones that match."
- "Build a timeline of everything recorded about this person's life."

## Why this server

Built with three priorities: efficiency in every sense (small footprint, low token usage), security, and reliability.

- Apache-2.0 licensed.
- Full CRUD for every Gramps object type (person, family, event, place, source, citation, repository, note, media, tag), plus merge for every type Gramps itself supports merging on.
- Structured queries: GOQL expressions and Gramps' own native filter rules — not just free-text search.
- Per-user OAuth 2.1 (Authorization Code + PKCE + Dynamic Client Registration) or a static bearer token, for the HTTP transport.
- Read-only mode that exposes only read tools.

See the wiki's [Alternatives](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Alternatives) page for how this compares to other options.

## Requirements

Gramps Web 26.10.0 or newer (gramps-web-api 3.23+).

## Get started

- No Gramps Web yet → [wiki Quickstart](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Quickstart) (5 minutes, Docker).
- Have Gramps Web → [docs/choose-setup.md](docs/choose-setup.md).

## Documentation

Full documentation index: [docs/README.md](docs/README.md). See also the [wiki](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki).

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for building, running, and testing locally.

## License

[Apache-2.0](LICENSE)
