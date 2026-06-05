import type { Page } from "@playwright/test";

export type IpcHandler = (
  args: Record<string, unknown> | undefined
) => unknown | Promise<unknown>;

export interface IpcStubOptions {
  handlers: Record<string, IpcHandler>;

  passthroughUnknown?: boolean;
}

export async function installTauriStub(
  page: Page,
  options: IpcStubOptions
): Promise<void> {
  await page.addInitScript(
    ([handlersSrc, passthroughUnknown]) => {
      const handlers = JSON.parse(handlersSrc as string) as Record<string, string>;
      const fns: Record<string, (a: unknown) => unknown> = {};
      for (const [k, body] of Object.entries(handlers)) {
        // eslint-disable-next-line no-new-func
        fns[k] = new Function("args", body) as (a: unknown) => unknown;
      }
      const log: Array<{ cmd: string; args: unknown }> = [];
      (window as unknown as Record<string, unknown>).__FOLIO_IPC_LOG__ = log;
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

        convertFileSrc: (path: string) => path,
      };
    },
    [
      JSON.stringify(
        Object.fromEntries(
          Object.entries(options.handlers).map(([k, fn]) => [
            k,
            `return (${fn.toString()})(args);`,
          ])
        )
      ),
      options.passthroughUnknown ?? true,
    ] as const
  );
}

export async function ipcLog(
  page: Page
): Promise<Array<{ cmd: string; args: unknown }>> {
  return await page.evaluate(
    () =>
      (window as unknown as Record<string, unknown>).__FOLIO_IPC_LOG__ as Array<{
        cmd: string;
        args: unknown;
      }>
  );
}
