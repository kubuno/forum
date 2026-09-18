<!--
  SPDX-FileCopyrightText: 2026 Kubuno contributors
  SPDX-License-Identifier: AGPL-3.0-or-later
-->

<div align="center">

<img src=".github/logo.png" alt="Kubuno Forum logo" width="120">

# Kubuno — Forum

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-edition_2021-orange.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)
![Status](https://img.shields.io/badge/status-alpha-yellow.svg)
![Module](https://img.shields.io/badge/Kubuno-module-4D38DB.svg)

**Full-featured, self-hosted discussion boards for Kubuno — categories, forums, topics and posts, with moderation, ranks, polls, private messages and per-forum permissions.**

Forum is a module for [Kubuno](https://github.com/kubuno/core), the self-hosted, libre (AGPLv3) cloud platform — a sovereign alternative to Google Workspace and Microsoft 365. It gives your instance a classic community board that reuses the platform's accounts, notifications and file storage, so members already have a profile and a place to talk.

</div>

---

## ✨ Features

- 🗂️ **Board hierarchy** — categories → forums (with recursive sub-forums) → topics → posts, with breadcrumbs, a jumpbox and full pagination through long forums and topics.
- ✍️ **Writing** — Markdown messages with a Write/Preview composer, author-attributed quoting, syntax-highlighted code blocks, per-post attachments (stored in the Drive module), autosaved drafts and a dedicated Drafts page.
- 📌 **Topics** — normal / sticky / announcement / global types, locking, solved-topic marking, unread tracking, per-topic subscriptions, view and reply counters, and shareable per-message permalinks.
- 🗳️ **Polls** — attach a single- or multiple-choice poll (2–20 options, optional run time) when starting a topic; votes obey the topic's own access rules.
- 🛡️ **Moderation** — lock, move, split, merge, bulk-moderate and trash/restore topics; an approval queue and a curated-reason report queue; a full tamper-aware moderation log; account, IP and email bans; and a server-side word censor that cannot be bypassed from the browser.
- 👥 **Members & profiles** — profile pages (avatar, rank, join date, counts, bio, location, website, custom title), a sortable members directory, a "Who's online" presence list, a top-contributor leaderboard and administrator-defined custom profile fields.
- 🏅 **Ranks & signatures** — post-count-based ranks plus assignable special ranks, and per-member signatures honouring both the board policy and each reader's preference.
- ✉️ **Private messages** — one-to-one or small-group conversations kept entirely separate from the boards and never indexed by search, with an inbox, unread badge and per-member blocking.
- 🔔 **Shared notifications** — replies, mentions, watched-topic and moderation events flow into the platform's single header bell in real time, are grouped when a topic is busy, and are caught up on your next visit if you were offline.
- 💬 **@mentions** — keyboard-navigable member autocomplete in the composer; mentioning someone in a forum they cannot see never notifies them or reveals the topic.
- 🔍 **Search & feeds** — PostgreSQL full-text search (relevance ranking, highlighted excerpts, phrase and exclusion operators) with an advanced filter panel; paginated Recent / Unanswered / Popular / Unread / Mine feeds; per-user revocable RSS/Atom feeds; and an editable FAQ.
- 🔒 **Permissions** — role-based per-forum access (guest / user / moderator) that can be additively widened to specific instance groups, plus a personal ignore list ("foes").

## 🏗️ Architecture

Like every Kubuno app, Forum is an **independent process**, not a library linked into the core. It registers with the [core](https://github.com/kubuno/core) at startup; the core then proxies its routes (`/api/v1/forum/*`), distributes platform events to it, serves its runtime-loaded React frontend bundle and manages its lifecycle.

- **Port** — the backend listens on `127.0.0.1:3117` and is reached only through the core's reverse proxy.
- **Backend** — `src/`: Axum + SQLx over PostgreSQL, confined to the `forum` schema; migrations in `migrations/`. It publishes topic/post/report events on the platform bus and reacts to account deletions.
- **Frontend** — `frontend/`: a React 19 bundle built to `entry.js` + `entry.css`, consuming `@kubuno/sdk`, `@ui` (`@kubuno/ui`) and `@kubuno/drive`. At runtime those specifiers are `external` and resolved by the host's import map to its single shared instances; the npm packages are used only for building and type-checking.
- **Trust boundary** — proxied requests are authenticated from a signed `X-Kubuno-Auth` token minted by the core (see `kubuno-modauth`), never from plain `X-Kubuno-User-*` headers.

## 📦 Install

The easiest way to self-host a full Kubuno instance (core + every module) is the **all-in-one Docker image** (`ghcr.io/kubuno/kubuno`), which already bundles this module — see **[kubuno/docker](https://github.com/kubuno/docker)** for `docker compose` instructions.

To add the module to an existing instance, install its **`.kbpkg`** — the single, cross-platform package format a Kubuno server unpacks by itself (no `.deb`/`.rpm`/`.exe`/`.pkg`, and no external tools). Each tagged release (`v*`) attaches a Linux `.kbpkg` (built by `build.yml`) and Windows/macOS `.kbpkg` files (built by `dist.yml`) to its [GitHub Release](https://github.com/kubuno/forum/releases):

```bash
# From the admin console: Modules → Install, then drop the .kbpkg — or, offline, from the CLI:
sudo kubuno modules:install dist/forum-<version>-<os>-<arch>.kbpkg
sudo systemctl restart kubuno     # the core loads the module on (re)start
```

## 🛠️ Build & development

**Requirements:** Rust ≥ 1.82, Node.js ≥ 24, PostgreSQL 16. No `kubuno/core` checkout is needed — shared Rust crates come from tagged git dependencies, and the `@kubuno/*` frontend libraries from the public npm scope.

```bash
cargo build --release                     # → target/release/kubuno-forum
cd frontend && npm ci && npm run build     # → dist/{entry.js, entry.css}

bash build_kbpkg.sh                        # → dist/forum-<version>-<os>-<arch>.kbpkg
bash build_kbpkg.sh --install              # build, install into the module store, restart
```

Once the module has been installed at least once, iterate quickly without repackaging:

```bash
bash ../_tools/deploy_local.sh forum             # backend + frontend
bash ../_tools/deploy_local.sh forum --frontend  # frontend only (fastest)
```

## 📦 Tech stack

Rust 2021 · Axum 0.7 · Tokio · SQLx 0.8 (PostgreSQL 16, schema `forum`, full-text search) — React 19 · TypeScript · Vite · Tailwind CSS v4 · Zustand · React Query, on the shared `@kubuno/sdk`, `@ui` and `@kubuno/drive` surfaces.

## 🤝 Contributing

Contributions are welcome. Please open an issue to discuss any significant change before submitting a pull request.

## 📄 License

[AGPL-3.0-or-later](LICENSE) © Kubuno contributors.
