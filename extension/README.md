# Attune meeting-context browser extension

Manifest V3 service worker that pings the local Attune app via the
`attune://prepare?url=...&title=...` URL scheme whenever the user
lands on a Zoom / Google Meet / Microsoft Teams tab. Pre-fills the
Record page so the user just hits the big red button.

**Privacy contract**: every payload goes through `attune://`, which
routes to the local Attune app on the user's machine. The
extension does not contact any remote server. The
`host_permissions` list is the union of meeting hosts the
extension watches; revoking individual entries disables the
auto-fire for that provider.

v2 roadmap finding 078 / GET-100.

## Loading the unpacked extension

### Chrome / Edge / Brave

1. Visit `chrome://extensions`.
2. Toggle "Developer mode" on.
3. Click "Load unpacked" and pick this `extension/` directory.

### Safari

Run `xcrun safari-web-extension-converter ./extension` and open the
generated Xcode project; build + run the host app once to register
the extension with Safari.

## Triggering Attune

The extension fires `attune://prepare?url=<encoded>&title=<encoded>`
twice:
- when the user clicks the toolbar icon on an active meeting tab
- when a meeting tab finishes loading (throttled to one fire per
  30 seconds per tab)

The Attune app's deep-link handler (`src/chrome/deep-link-handler.tsx`,
GET-103) receives the URL and routes it to the Record page. The
'prepare' deep-link verb is the natural follow-up — for now the
URL surfaces as a toast and the user knows the extension is
working.

## Follow-ups

- Pre-fill the Record route's recording label from the meeting
  title (Record route reads from a Zustand store the deep-link
  handler writes to).
- Add a per-provider opt-out toggle in the extension popup so the
  user can disable auto-fire for one host without uninstalling.
- Ship signed builds in the Chrome Web Store + Safari Extensions
  Gallery so the install path is one click.
