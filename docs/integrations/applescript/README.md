# AppleScript / JXA dictionary

Attune ships a published `Attune.sdef` so Keyboard Maestro,
BetterTouchTool, Hammerspoon, Alfred, Automator, and any
30-year-old Mac power-user shell-script can drive the app
end-to-end. v2 roadmap finding 080 / GET-102.

## Bundling

The sdef file lives at `src-tauri/resources/Attune.sdef`. To make
it discoverable, the Tauri bundle's `Info.plist` needs:

```xml
<key>OSAScriptingDefinition</key>
<string>Attune.sdef</string>
<key>NSAppleScriptEnabled</key>
<true/>
```

The native ObjC handler classes referenced in `<cocoa class="…"/>`
attributes are the natural follow-up — they map AppleScript verbs to
the same `attune://` deep-link + `.attune/inbox/` JSON paths the
shortcuts integration uses (GET-75 + GET-76 + GET-103), so the
behavior stays consistent across every entry point.

## Verbs

| AppleScript | JXA | What it does |
|---|---|---|
| `tell application "Attune" to start recording` | `Application("Attune").startRecording()` | Begin a new session. |
| `tell application "Attune" to stop recording with and summarize true` | `Application("Attune").stopRecording({andSummarize:true})` | Stop and optionally fire Summarize. |
| `tell application "Attune" to add task "Send invoice" owner "@me" due "Friday"` | `Application("Attune").addTask("Send invoice", {owner:"@me", due:"Friday"})` | Drop a kanban task. |
| `tell application "Attune" to search memory "pricing"` | `Application("Attune").searchMemory("pricing")` | Returns JSON-encoded matches. |
| `tell application "Attune" to last meeting summary` | `Application("Attune").lastMeetingSummary()` | Markdown summary text. |
| `tell application "Attune" to open url "attune://prepare?url=…&title=…"` | `Application("Attune").openUrl("attune://…")` | Dispatch a deep link. |

## Inspecting the dictionary

Open Script Editor.app → File → Open Dictionary → Attune. The sdef
shows the same verbs, parameters, and result types listed above.

## Why a published sdef matters

A scriptable Mac app is a forever-integration: every shell-out
solution stops working when the SaaS vendor changes their API,
but an AppleScript verb that's been stable since System 7 still
works today. The contract is small + well-defined, the user owns
their automation, and the same recipe drops into Hammerspoon /
Keyboard Maestro / Shortcuts / cron without rewriting.
