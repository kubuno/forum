<!--
  SPDX-FileCopyrightText: 2026 Kubuno contributors
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

# Kubuno Forum

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-edition_2021-orange.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)
![Module](https://img.shields.io/badge/Kubuno-module-4D38DB.svg)

**Kubuno Forum — self-hosted discussion boards (categories, forums, topics, posts), inspired by phpBB.**

A module for [Kubuno](https://github.com/kubuno/core), the self-hosted, libre (AGPLv3) cloud platform.

## Features

- **Hierarchy** — categories → forums (with recursive sub-forums) → topics → posts.
- **Posting** — Markdown messages with quoting, editing (with edit history metadata) and per-post attachments (stored in the drive module).
- **Topics** — normal / sticky / announcement / global types, locking, unread tracking, view & reply counters.
- **Moderation** — lock, move, split and merge topics; a report queue; per-forum moderators.
- **Ranks & profiles** — phpBB-style ranks based on post count, plus per-user signatures.
- **Permissions** — simplified per-forum roles (guest / user / moderator).
- **Search** — full-text style search across topic titles and post bodies.

## Architecture

A standalone Rust process that registers with the [core](https://github.com/kubuno/core) at startup; the core proxies its routes (`/forum/*`) and serves its runtime-loaded React frontend bundle.

- **Backend** — `src/`: Axum + SQLx (PostgreSQL, schema `forum`); migrations in `migrations/`. Listens on port `3117`.
- **Frontend** — `frontend/`: a React bundle built to `entry.js`, consuming `@kubuno/sdk`, `@kubuno/ui` and `@kubuno/drive` from npm (provided by the host at runtime via the import map).

## Build

**Requirements:** Rust ≥ 1.82, Node.js ≥ 20, PostgreSQL 16.

```bash
cargo build --release                      # → target/release/kubuno-forum
cd frontend && npm ci && npm run build      # → dist/{entry.js, entry.css}
bash build_deb.sh                           # → dist/kubuno-forum_*.deb
```

> Shared dependencies come from Kubuno — no `kubuno/core` checkout required:
> - **Rust** — shared crates via tagged git dependencies on `kubuno/core`.
> - **Frontend** — `@kubuno/sdk`, `@kubuno/ui`, `@kubuno/drive` from the `@kubuno` npm scope.

## License

[AGPL-3.0-or-later](LICENSE) © Kubuno contributors.
