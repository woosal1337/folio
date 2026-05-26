# Attune for Raycast

Official Raycast extension wrapping `attune-cli`.

## Commands

| Command | What it does |
|---|---|
| Search Memory | Free-text query against your local Attune memory store. |
| Recent Meetings | List the last 20 recording sessions, reveal in Finder, open in Attune. |
| Add Task | Append a task to your Attune `tasks.json` (Raycast Form, no LLM required). |
| Start Recording | Trigger Attune via the `attune://` deep link and start a new recording. |

## Preferences

- **attune-cli path** — Absolute path to the `attune-cli` binary. Defaults to `attune-cli` on `$PATH`.
- **Attune vault** — Path to your vault root. Defaults to `~/Documents/Attune`. The extension reads `<vault>/recordings/`, `<vault>/memory/`, `<vault>/tasks/tasks.json`.

## Development

```bash
cd integrations/raycast
npm install
npm run dev
```

Raycast's `ray develop` watches `src/` and hot-reloads the extension into the Raycast app.

## Publishing

```bash
npm run lint
npm run build
npm run publish
```

v2 finding 077 / GET-79.
