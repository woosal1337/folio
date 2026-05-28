/**
 * Tauri IPC bridge stub for Playwright.
 *
 * `@tauri-apps/api/core::invoke` resolves the underlying transport
 * via `window.__TAURI_INTERNALS__`. In a real browser that global
 * doesn't exist and every IPC call throws. We inject a fake here
 * via `page.addInitScript` so the React app boots normally and we
 * can drive the auth + conductor + settings flows end-to-end.
 *
 * Test files declare command handlers via the `setIpcHandlers`
 * helper. Anything not mapped returns the result of
 * `defaultHandler(cmd, args)` — by default that resolves to `null`
 * which is the safest no-op for void-returning commands. Tests can
 * override with `passthroughUnknown: false` if they want unmapped
 * commands to throw so coverage gaps surface as test failures.
 */

import type { Page } from "@playwright/test";

export type IpcHandler = (
  args: Record<string, unknown> | undefined,
) => unknown | Promise<unknown>;

export interface IpcStubOptions {
  /** Map of `command_name` → handler. Each command is resolved by
   * the kebab/snake/camel key Tauri uses on the wire. */
  handlers: Record<string, IpcHandler>;
  /** When true (default), unmapped commands resolve to `null` so
   * the app doesn't crash on a route it doesn't strictly need.
   * Set false to make test gaps loud. */
  passthroughUnknown?: boolean;
}

/**
 * Installs the IPC stub + a few `window` polyfills that Tauri's
 * runtime expects (`__TAURI_OS_PLUGIN_INTERNALS__` etc. can stay
 * undefined; only `__TAURI_INTERNALS__.invoke` is mandatory).
 *
 * Call once per `page.goto`. Re-applies on navigation because the
 * init script runs at every new document load.
 */
export async function installTauriStub(
  page: Page,
  options: IpcStubOptions,
): Promise<void> {
  // Serialise handlers via stringification so Playwright can ship
  // them across the worker → page boundary. Each handler ends up
  // running inside the page context as a `new Function(...)`; if
  // the test needs to spy on calls, use the dedicated
  // `installSpyableTauriStub` below instead.
  await page.addInitScript(
    ([handlersSrc, passthroughUnknown]) => {
      const handlers = JSON.parse(handlersSrc as string) as Record<
        string,
        string
      >;
      const fns: Record<string, (a: unknown) => unknown> = {};
      for (const [k, body] of Object.entries(handlers)) {
        // eslint-disable-next-line no-new-func
        fns[k] = new Function("args", body) as (a: unknown) => unknown;
      }
      const log: Array<{ cmd: string; args: unknown }> = [];
      (window as unknown as Record<string, unknown>).__ATTUNE_IPC_LOG__ = log;
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args?: Record<string, unknown>) => {
          log.push({ cmd, args });
          const fn = fns[cmd];
          if (fn) {
            return await fn(args);
          }
          if (passthroughUnknown as boolean) return null;
          throw new Error(`unmapped IPC: ${cmd}`);
        },
        // Tauri's API may call `convertFileSrc` indirectly — keep a
        // no-op so it doesn't crash.
        convertFileSrc: (path: string) => path,
      };
    },
    [
      JSON.stringify(
        Object.fromEntries(
          Object.entries(options.handlers).map(([k, fn]) => [
            k,
            `return (${fn.toString()})(args);`,
          ]),
        ),
      ),
      options.passthroughUnknown ?? true,
    ] as const,
  );
}

/**
 * Convenience for tests that want to dump the full IPC trace after
 * a scenario. Returns the in-page log of every invoke call the
 * React app issued (cmd + args), in order.
 */
export async function ipcLog(
  page: Page,
): Promise<Array<{ cmd: string; args: unknown }>> {
  return await page.evaluate(
    () =>
      (window as unknown as Record<string, unknown>)
        .__ATTUNE_IPC_LOG__ as Array<{ cmd: string; args: unknown }>,
  );
}
