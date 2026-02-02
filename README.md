# Velocitext

Velocitext is a local-first, desktop notes app built with Tauri and Svelte.
Notes are plain Markdown files synced by your tool of choice (e.g. Syncthing),
while a local SQLite FTS5 index keeps search fast.

## Features

- Markdown notes stored as files on disk
- Local full-text search via SQLite FTS5
- Frontmatter-driven metadata (title, date, tags, status)
- Archive toggle (soft delete)
- Focus/Zen mode
- Light theme UI

## Where notes are stored

- Notes folder: `~/Notes/Velocitext`
- Index database: `~/.local/share/velocitext/velocitext.db`

## Note format

Each note is a single `.md` file with YAML frontmatter.

Example:

```yaml
---
title: "Café du matin"
layout: note.njk
date: "17-01-2026"
description: ""
statut: chantier
tags:
  - café
updated: "17-01-2026 14:32"
archived: false
---
```

## Development

Install dependencies:

```bash
npm install
```

Run the web UI (mock data only):

```bash
npm run dev
```

Run the desktop app:

```bash
npm run tauri dev
```

Build the web bundle:

```bash
npm run build
```

## Build (Tauri)

```bash
npm run tauri build
```

## Sync model

Velocitext does not include cloud sync. Sync the notes folder with a tool like
Syncthing. The SQLite index must remain local and should not be synced.

## Project status

Early stage. The notes directory is currently fixed to `~/Notes/Velocitext`.
See `agent.md` for the product spec and planned behavior.

## License

MIT. See `LICENSE`.
