/**
 * Navigation bridge — single choke-point for all external entry points
 * (GET-214).
 *
 * ## Problem
 *
 * Attune has several surfaces that trigger navigation from OUTSIDE the
 * React component tree: the tray menu, the meeting HUD "Take Notes" button,
 * Cmd-K quick-open, and future `attune://` deep links. Each previously
 * called `navigate()` (or `window.location`) independently, which can
 * produce diverging history stacks and a broken back button — exactly the
 * failure mode Granola documented in their "back button" post.
 *
 * ## Solution
 *
 * Every external navigation MUST go through `bridgeNavigate()`. It:
 *   1. Delegates to the active `NavigateFunction` registered by the main
 *      `<App>` component via `registerNavigateFn`.
 *   2. Guards against calls that arrive before the React tree is mounted
 *      (queues them and flushes on register).
 *   3. Never touches `window.location` or `window.history` directly.
 *
 * ## Rules (enforced by this module, documented for future contributors)
 *
 *   - ONLY route through `bridgeNavigate` for external entry points.
 *   - NEVER call `window.location.href =` or `window.history.pushState`
 *     inside the main-window React app.
 *   - Frameless companion windows (recording-bar, meeting-hud) must NOT
 *     assume they share the main window's React Router history. Use Tauri
 *     events to tell the main window to navigate.
 *   - Deep links (`attune://`) must resolve to a path and call
 *     `bridgeNavigate` rather than manipulating the URL directly.
 */

import type { NavigateFunction, NavigateOptions } from "react-router-dom";

type QueuedNav = { to: string; options?: NavigateOptions };

let _navigate: NavigateFunction | null = null;
const _queue: QueuedNav[] = [];

/**
 * Called once by `<App>` after the `useNavigate` hook is available.
 * Flushes any navigations that arrived before mount.
 */
export function registerNavigateFn(fn: NavigateFunction): void {
  _navigate = fn;
  for (const { to, options } of _queue.splice(0)) {
    fn(to, options);
  }
}

/**
 * Navigate from an external entry point (tray, HUD, deep link, Cmd-K).
 *
 * If the React tree isn't mounted yet the call is queued and replayed
 * once `registerNavigateFn` is called. This makes it safe to call
 * from Tauri event listeners that fire before the first render.
 */
export function bridgeNavigate(to: string, options?: NavigateOptions): void {
  if (_navigate) {
    _navigate(to, options);
  } else {
    _queue.push({ to, options });
  }
}

/**
 * Guard: throws if called with a raw `window.location`-style path.
 * Use in deep-link handlers to catch mistakes early.
 *
 * Valid: `/library`, `/editor/2026-01-01-10-00-00`
 * Invalid: `http://...`, `attune://...`, `javascript:...`
 */
export function assertInternalPath(path: string): void {
  if (!path.startsWith("/")) {
    throw new Error(
      `bridgeNavigate: path must start with "/" — received: ${path}. ` +
        `Use bridgeNavigate("/editor/...") not a raw URL.`
    );
  }
}
