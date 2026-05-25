# Apple Reminders two-way sync

Settings → General gains a "Sync to Apple Reminders" toggle that
mirrors your kanban into a Reminders list and pulls anything tagged
`#attune` from Reminders into the kanban's inbox column.

v2 finding 076 / GET-78. EventKit-bridge native impl is the
follow-up; this PR ships the **opt-in flag** + a transitional
Shortcut recipe so the round-trip works today via the user's own
Apple ecosystem.

## How it works (transitional path)

While the EventKit bridge ships, use Shortcuts to round-trip:

1. Settings → General → **Sync to Apple Reminders** → ON. Pick the
   target list name (defaults to "Attune").
2. Import the helper Shortcut:
   - Trigger: every 5 minutes
   - For each completed task in the Attune list: drop a JSON file
     into `<vault>/.attune/inbox/` marking it done.
   - For each new uncompleted reminder tagged `#attune`: drop a
     `create-task` JSON into the inbox.

The wire format matches the inbox contract from GET-75; Attune's
fs-watcher picks the events up automatically.

## Native EventKit bridge (follow-up)

The native EventKit bridge replaces the Shortcut with an in-app
sync worker that polls Reminders.app every 60s and writes via
EKReminder. macOS asks for Reminders permission on first run.
Same wire shape as the transitional path, so the user's recipes
stop firing the moment the bridge takes over without losing any
state.
