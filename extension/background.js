/**
 * Attune meeting-context extension — background service worker.
 *
 * Detects when the user lands on a Zoom / Google Meet / Microsoft Teams
 * tab and pings the local Attune app via `attune://prepare?...` so the
 * Record page can pre-fill the meeting label without the user typing
 * anything.
 *
 * Privacy: every payload goes through the `attune://` URL scheme,
 * which routes to the local app only. The extension does not contact
 * any remote server.
 *
 * v2 roadmap finding 078 / GET-100.
 */

const MEETING_HOSTS = [
  /\.zoom\.us$|^zoom\.us$/i,
  /^meet\.google\.com$/i,
  /\.teams\.microsoft\.com$|^teams\.microsoft\.com$/i,
];

function isMeetingTab(url) {
  try {
    const u = new URL(url);
    return MEETING_HOSTS.some((re) => re.test(u.hostname));
  } catch {
    return false;
  }
}

/** Fire-and-forget the prepare URL. Browsers don't surface a confirm
 *  for custom-scheme handlers the user has previously allowed, so
 *  the call is silent on the happy path. */
function pingAttune({ url, title }) {
  const target = new URL("attune://prepare");
  target.searchParams.set("url", url);
  if (title) target.searchParams.set("title", title);
  // Opening + closing a hidden tab is the cleanest way to fire a
  // custom-scheme URL from a MV3 worker without using the
  // declarativeContent API (which Safari doesn't support).
  chrome.tabs.create({ url: target.toString(), active: false }, (tab) => {
    if (tab?.id) {
      setTimeout(() => chrome.tabs.remove(tab.id), 300);
    }
  });
}

// Toolbar click — explicit user action, the most reliable trigger.
chrome.action.onClicked.addListener(async (tab) => {
  if (!tab?.url) return;
  if (!isMeetingTab(tab.url)) {
    chrome.notifications?.create?.({
      type: "basic",
      iconUrl: "icons/icon-128.png",
      title: "Attune",
      message: "Open a Zoom / Meet / Teams tab first.",
    });
    return;
  }
  pingAttune({ url: tab.url, title: tab.title ?? "" });
});

// Auto-fire on URL change so users don't have to click for every
// meeting. Throttled per tab so a SPA-style route swap inside
// Meet doesn't spam the local app.
const recentlyPinged = new Map();
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (changeInfo.status !== "complete") return;
  if (!tab.url || !isMeetingTab(tab.url)) return;
  const last = recentlyPinged.get(tabId) ?? 0;
  if (Date.now() - last < 30_000) return;
  recentlyPinged.set(tabId, Date.now());
  pingAttune({ url: tab.url, title: tab.title ?? "" });
});

chrome.tabs.onRemoved.addListener((tabId) => recentlyPinged.delete(tabId));
