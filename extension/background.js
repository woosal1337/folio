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

function pingAttune({ url, title }) {
  const target = new URL("attune://prepare");
  target.searchParams.set("url", url);
  if (title) target.searchParams.set("title", title);

  chrome.tabs.create({ url: target.toString(), active: false }, (tab) => {
    if (tab?.id) {
      setTimeout(() => chrome.tabs.remove(tab.id), 300);
    }
  });
}

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
