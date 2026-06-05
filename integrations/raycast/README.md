# Folio for Raycast

Official Raycast extension wrapping `folio-cli`.

## Commands

| Command         | What it does                                                              |
| --------------- | ------------------------------------------------------------------------- |
| Search Memory   | Free-text query against your local Folio memory store.                    |
| Recent Meetings | List the last 20 recording sessions, reveal in Finder, open in Folio.     |
| Add Task        | Append a task to your Folio `tasks.json` (Raycast Form, no LLM required). |
| Start Recording | Trigger Folio via the `folio://` deep link and start a new recording.     |

## Preferences

- **folio-cli path** — Absolute path to the `folio-cli` binary. Defaults to `folio-cli` on `$PATH`.
- **Folio vault** — Path to your vault root. Defaults to `~/Documents/Folio`. The extension reads `<vault>/recordings/`, `<vault>/memory/`, `<vault>/tasks/tasks.json`.

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

.
