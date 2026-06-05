import type { NavigateFunction, NavigateOptions } from "react-router-dom";

type QueuedNav = { to: string; options?: NavigateOptions };

let _navigate: NavigateFunction | null = null;
const _queue: QueuedNav[] = [];

export function registerNavigateFn(fn: NavigateFunction): void {
  _navigate = fn;
  for (const { to, options } of _queue.splice(0)) {
    fn(to, options);
  }
}

export function bridgeNavigate(to: string, options?: NavigateOptions): void {
  if (_navigate) {
    _navigate(to, options);
  } else {
    _queue.push({ to, options });
  }
}

export function assertInternalPath(path: string): void {
  if (!path.startsWith("/")) {
    throw new Error(
      `bridgeNavigate: path must start with "/" — received: ${path}. ` +
        `Use bridgeNavigate("/editor/...") not a raw URL.`
    );
  }
}
