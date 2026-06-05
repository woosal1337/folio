# AppleScript / JXA dictionary

Folio ships a published `Folio.sdef` so Keyboard Maestro,
BetterTouchTool, Hammerspoon, Alfred, Automator, and any
30-year-old Mac power-user shell-script can drive the app
end-to-end. v2 roadmap finding 080.

## Bundling

The sdef file lives at `src-tauri/resources/Folio.sdef`. To make
it discoverable, the Tauri bundle's `Info.plist` needs:

```xml
<key>OSAScriptingDefinition</key>
<string>Folio.sdef</string>
<key>NSAppleScriptEnabled</key>
<true/>
```

The native ObjC handler classes referenced in `<cocoa class="…"/>`
attributes are the natural follow-up — they map AppleScript verbs to
the same `folio://` deep-link + `.folio/inbox/` JSON paths the
shortcuts integration uses ( + +), so the
behavior stays consistent across every entry point.

## Verbs

| AppleScript                                                                    | JXA                                                                         | What it does                        |
| ------------------------------------------------------------------------------ | --------------------------------------------------------------------------- | ----------------------------------- |
| `tell application "Folio" to start recording`                                  | `Application("Folio").startRecording`                                       | Begin a new session.                |
| `tell application "Folio" to stop recording with and summarize true`           | `Application("Folio").stopRecording({andSummarize:true})`                   | Stop and optionally fire Summarize. |
| `tell application "Folio" to add task "Send invoice" owner "@me" due "Friday"` | `Application("Folio").addTask("Send invoice", {owner:"@me", due:"Friday"})` | Drop a kanban task.                 |
| `tell application "Folio" to search memory "pricing"`                          | `Application("Folio").searchMemory("pricing")`                              | Returns JSON-encoded matches.       |
| `tell application "Folio" to last meeting summary`                             | `Application("Folio").lastMeetingSummary`                                   | Markdown summary text.              |
| `tell application "Folio" to open url "folio://prepare?url=…&title=…"`         | `Application("Folio").openUrl("folio://…")`                                 | Dispatch a deep link.               |

## Inspecting the dictionary

Open Script Editor.app → File → Open Dictionary → Folio. The sdef
shows the same verbs, parameters, and result types listed above.

## Why a published sdef matters

A scriptable Mac app is a forever-integration: every shell-out
solution stops working when the SaaS vendor changes their API,
but an AppleScript verb that's been stable since System 7 still
works today. The contract is small + well-defined, the user owns
their automation, and the same recipe drops into Hammerspoon /
Keyboard Maestro / Shortcuts / cron without rewriting.
