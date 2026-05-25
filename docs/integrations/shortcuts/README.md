# Apple Shortcuts integration

Attune ships five copy-paste shortcut recipes that hook into the
`attune://` URL scheme + the `.attune/inbox/` filesystem contract.
Each one is a stock Shortcuts.app pipeline you can rebuild in 30
seconds — no third-party action required.

v2 roadmap finding 074 / GET-76. The fuller App-Intents Swift
sidecar (which surfaces these as first-class actions inside Siri
and the Action Button picker) stays as a follow-up; the recipes
below ship the Cmd-Space / Action-Button trigger today.

## Setup

1. Make sure Attune is running. Settings → Webhooks should show the
   `attune://` scheme registered (GET-103).
2. Find your vault: Settings → Storage → Memory directory.
3. Open Shortcuts.app and create the recipes below.

## 1. Start recording

```
[Open URL]  attune://recording/start
```

When Attune is running, this brings the app forward and flips the
big red button on the Record route.

## 2. Stop and summarize

```
[Run Shell Script]  /bin/zsh
  printf '%s' '{"kind":"stop-recording"}' > "$ATTUNE_VAULT/.attune/inbox/stop-$(date +%s).json"
[Open URL]  attune://agent/run?agent_id=summarize
```

`$ATTUNE_VAULT` should expand to your vault path (set it in the
shortcut's environment block). Attune polls the inbox and fires
the matching action.

## 3. Add task

```
[Ask for Input]      "Task title"
[Run Shell Script]   /bin/zsh
  TITLE="$1"
  printf '%s' "{\"kind\":\"create-task\",\"title\":\"$TITLE\"}" > "$ATTUNE_VAULT/.attune/inbox/task-$(date +%s).json"
```

Drops a new task on the kanban via the filesystem contract.

## 4. Search memories

```
[Ask for Input]      "Search query"
[Run Shell Script]   /bin/zsh
  attune-cli memory-search "$1" --limit 10
[Show Result]
```

Surfaces the results in a dialog. Requires the `attune-cli` binary
on `PATH` (it ships with the app under `/Applications/Attune.app/
Contents/MacOS/attune-cli` — symlink to `/usr/local/bin` once).

## 5. Get last meeting summary

```
[Run Shell Script]   /bin/zsh
  LATEST=$(attune-cli sessions --limit 1 | jq -r .session_dir)
  cat "$LATEST/agent_runs/summarize.json" | jq -r .response
[Show Result]
```

Reads the latest summarize agent run straight from disk.

## Follow-up: App Intents Swift sidecar

The Swift sidecar (bundled as part of the `.app` so the actions
appear as first-class entries inside Siri / Action Button / the
Shortcuts library without any user-side recipe assembly) is
tracked separately. The recipes above are the wire-format contract
the sidecar will speak; once it lands, importing these recipes
becomes unnecessary.
